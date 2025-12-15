//! Transport FFI Wrappers
//!
//! This module provides FFI-safe wrappers for Paykit transport traits,
//! enabling mobile applications to perform directory operations.
//!
//! # Overview
//!
//! Mobile apps have two options for transport:
//!
//! ## Option 1: Mock Transport (Testing/Development)
//!
//! Use `AuthenticatedTransportFFI::new_mock()` for testing without a real Pubky connection.
//!
//! ## Option 2: Callback-Based Transport (Production)
//!
//! Implement `PubkyStorageCallback` in Swift/Kotlin to wrap your Pubky SDK:
//!
//! ```swift
//! // Swift example
//! class MyPubkyStorage: PubkyStorageCallback {
//!     let session: PubkySession
//!     
//!     func put(path: String, content: String) -> StorageOperationResult {
//!         do {
//!             try session.storage.put(path, content)
//!             return StorageOperationResult.ok()
//!         } catch {
//!             return StorageOperationResult.err(error.localizedDescription)
//!         }
//!     }
//!     
//!     func get(path: String) -> StorageGetResult {
//!         // ... implement using Pubky SDK
//!     }
//!     
//!     // ... other methods
//! }
//!
//! let storage = MyPubkyStorage(session: pubkySession)
//! let transport = AuthenticatedTransportFFI.fromCallback(storage, ownerPubkey)
//! ```
//!
//! # Example Flow
//!
//! ```ignore
//! // 1. Create transport (choose one)
//! let transport = AuthenticatedTransportFFI.new_mock("pubkey") // For testing
//! // OR
//! let transport = AuthenticatedTransportFFI.from_callback(myStorage, "pubkey") // Production
//!
//! // 2. Use with PaykitClient
//! client.publishPaymentEndpoint(transport, "lightning", "lnbc1...")
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::{PaykitMobileError, PaymentMethod, Result};

// ============================================================================
// Storage Callback Interfaces
// ============================================================================

/// Result type for storage operations.
#[derive(Clone, Debug, uniffi::Record)]
pub struct StorageOperationResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl StorageOperationResult {
    /// Create a success result.
    pub fn ok() -> Self {
        Self {
            success: true,
            error: None,
        }
    }

    /// Create an error result.
    pub fn err(message: String) -> Self {
        Self {
            success: false,
            error: Some(message),
        }
    }
}

/// Result type for storage get operations.
#[derive(Clone, Debug, uniffi::Record)]
pub struct StorageGetResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// The content if found (None if not found but successful)
    pub content: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
}

impl StorageGetResult {
    /// Create a success result with content.
    pub fn ok(content: Option<String>) -> Self {
        Self {
            success: true,
            content,
            error: None,
        }
    }

    /// Create an error result.
    pub fn err(message: String) -> Self {
        Self {
            success: false,
            content: None,
            error: Some(message),
        }
    }
}

/// Result type for storage list operations.
#[derive(Clone, Debug, uniffi::Record)]
pub struct StorageListResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// List of file names/paths
    pub entries: Vec<String>,
    /// Error message if failed
    pub error: Option<String>,
}

impl StorageListResult {
    /// Create a success result with entries.
    pub fn ok(entries: Vec<String>) -> Self {
        Self {
            success: true,
            entries,
            error: None,
        }
    }

    /// Create an error result.
    pub fn err(message: String) -> Self {
        Self {
            success: false,
            entries: Vec::new(),
            error: Some(message),
        }
    }
}

/// Callback interface for authenticated Pubky storage operations.
///
/// Mobile apps implement this to wrap their Pubky SDK session.
/// All operations are performed on the owner's storage.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (Send + Sync).
#[uniffi::export(callback_interface)]
pub trait PubkyAuthenticatedStorageCallback: Send + Sync {
    /// Put (create or update) content at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Storage path (e.g., "/pub/paykit.app/v0/lightning")
    /// * `content` - Content to store
    fn put(&self, path: String, content: String) -> StorageOperationResult;

    /// Get content at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Storage path to read
    ///
    /// # Returns
    ///
    /// Content if found, None if path doesn't exist.
    fn get(&self, path: String) -> StorageGetResult;

    /// Delete content at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Storage path to delete
    fn delete(&self, path: String) -> StorageOperationResult;

    /// List files with the given prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Path prefix to list (e.g., "/pub/paykit.app/v0/")
    fn list(&self, prefix: String) -> StorageListResult;
}

/// Callback interface for unauthenticated (read-only) Pubky storage operations.
///
/// Mobile apps implement this to wrap their Pubky SDK public storage.
///
/// # Thread Safety
///
/// Implementations must be thread-safe (Send + Sync).
#[uniffi::export(callback_interface)]
pub trait PubkyUnauthenticatedStorageCallback: Send + Sync {
    /// Get content at the given path from another user's public storage.
    ///
    /// # Arguments
    ///
    /// * `owner_pubkey` - The owner's public key (z-base32 encoded)
    /// * `path` - Storage path to read
    fn get(&self, owner_pubkey: String, path: String) -> StorageGetResult;

    /// List files with the given prefix from another user's public storage.
    ///
    /// # Arguments
    ///
    /// * `owner_pubkey` - The owner's public key (z-base32 encoded)
    /// * `prefix` - Path prefix to list
    fn list(&self, owner_pubkey: String, prefix: String) -> StorageListResult;
}

// ============================================================================
// Mock Transport for Testing/Development
// ============================================================================

/// In-memory storage for mock transport operations.
/// Used for testing and development when a real Pubky connection is not available.
#[derive(Default)]
struct MockStorage {
    data: HashMap<String, String>,
}

/// Internal storage backend enum.
enum StorageBackend {
    /// Mock in-memory storage for testing
    Mock(Arc<RwLock<MockStorage>>),
    /// Real callback-based storage for production
    Callback(Box<dyn PubkyAuthenticatedStorageCallback>),
}

// ============================================================================
// Authenticated Transport FFI
// ============================================================================

/// FFI wrapper for authenticated transport operations.
///
/// This wraps authenticated write access to Pubky homeservers.
/// Mobile apps can use either:
/// - `new_mock()` for testing
/// - `from_callback()` for production with real Pubky SDK
#[derive(uniffi::Object)]
pub struct AuthenticatedTransportFFI {
    /// The owner's public key (z-base32 encoded).
    owner_pubkey: String,
    /// Storage backend (mock or callback-based).
    backend: RwLock<StorageBackend>,
}

#[uniffi::export]
impl AuthenticatedTransportFFI {
    /// Create a new authenticated transport for testing/development.
    ///
    /// Uses in-memory storage - data is not persisted.
    ///
    /// # Arguments
    ///
    /// * `owner_pubkey` - The owner's public key (z-base32 encoded)
    #[uniffi::constructor]
    pub fn new_mock(owner_pubkey: String) -> Arc<Self> {
        Arc::new(Self {
            owner_pubkey,
            backend: RwLock::new(StorageBackend::Mock(Arc::new(RwLock::new(
                MockStorage::default(),
            )))),
        })
    }

    /// Create authenticated transport from a storage callback.
    ///
    /// # Arguments
    ///
    /// * `callback` - Storage callback implementing PubkyAuthenticatedStorageCallback
    /// * `owner_pubkey` - The owner's public key (z-base32 encoded)
    ///
    /// # Example (Swift)
    ///
    /// ```swift
    /// class MyPubkyStorage: PubkyAuthenticatedStorageCallback {
    ///     let session: PubkySession
    ///     
    ///     func put(path: String, content: String) -> StorageOperationResult {
    ///         do {
    ///             try session.storage.put(path, content)
    ///             return StorageOperationResult.ok()
    ///         } catch {
    ///             return StorageOperationResult.err(error.localizedDescription)
    ///         }
    ///     }
    ///     // ... implement other methods
    /// }
    ///
    /// let transport = AuthenticatedTransportFFI.fromCallback(
    ///     MyPubkyStorage(session: session),
    ///     ownerPubkey: myPublicKey
    /// )
    /// ```
    #[uniffi::constructor]
    pub fn from_callback(
        callback: Box<dyn PubkyAuthenticatedStorageCallback>,
        owner_pubkey: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            owner_pubkey,
            backend: RwLock::new(StorageBackend::Callback(callback)),
        })
    }

    /// Create authenticated transport from a Pubky session JSON.
    ///
    /// # Deprecated
    ///
    /// This method creates a mock transport. Use `from_callback()` for production.
    ///
    /// # Arguments
    ///
    /// * `session_json` - JSON configuration (validated but not used)
    /// * `owner_pubkey` - The owner's public key (z-base32 encoded)
    #[uniffi::constructor]
    pub fn from_session_json(session_json: String, owner_pubkey: String) -> Result<Arc<Self>> {
        // Validate JSON
        serde_json::from_str::<serde_json::Value>(&session_json).map_err(|e| {
            PaykitMobileError::Serialization {
                msg: format!("Invalid session JSON: {}", e),
            }
        })?;

        // Create mock transport - use from_callback() for real implementation
        Ok(Self::new_mock(owner_pubkey))
    }

    /// Get the owner's public key.
    pub fn owner_pubkey(&self) -> String {
        self.owner_pubkey.clone()
    }

    /// Check if this transport uses a real callback (production) or mock storage.
    ///
    /// Returns `true` for mock transport, `false` for callback-based transport.
    /// Returns an error if the internal lock is poisoned.
    pub fn is_mock(&self) -> Result<bool> {
        let backend = self
            .backend
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Transport lock poisoned".to_string(),
            })?;
        Ok(matches!(*backend, StorageBackend::Mock(_)))
    }

    /// Put (create or update) a file at the given path.
    pub fn put(&self, path: String, content: String) -> Result<()> {
        let backend = self
            .backend
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
            })?;

        match &*backend {
            StorageBackend::Mock(storage) => {
                let mut storage = storage.write().map_err(|_| PaykitMobileError::Internal {
                    msg: "Lock poisoned".to_string(),
                })?;
                storage.data.insert(path, content);
                Ok(())
            }
            StorageBackend::Callback(callback) => {
                let result = callback.put(path, content);
                if result.success {
                    Ok(())
                } else {
                    Err(PaykitMobileError::Transport {
                        msg: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                    })
                }
            }
        }
    }

    /// Get a file at the given path.
    pub fn get(&self, path: String) -> Result<Option<String>> {
        let backend = self
            .backend
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
            })?;

        match &*backend {
            StorageBackend::Mock(storage) => {
                let storage = storage.read().map_err(|_| PaykitMobileError::Internal {
                    msg: "Lock poisoned".to_string(),
                })?;
                Ok(storage.data.get(&path).cloned())
            }
            StorageBackend::Callback(callback) => {
                let result = callback.get(path);
                if result.success {
                    Ok(result.content)
                } else {
                    Err(PaykitMobileError::Transport {
                        msg: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                    })
                }
            }
        }
    }

    /// Delete a file at the given path.
    pub fn delete(&self, path: String) -> Result<()> {
        let backend = self
            .backend
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
            })?;

        match &*backend {
            StorageBackend::Mock(storage) => {
                let mut storage = storage.write().map_err(|_| PaykitMobileError::Internal {
                    msg: "Lock poisoned".to_string(),
                })?;
                storage.data.remove(&path);
                Ok(())
            }
            StorageBackend::Callback(callback) => {
                let result = callback.delete(path);
                if result.success {
                    Ok(())
                } else {
                    Err(PaykitMobileError::Transport {
                        msg: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                    })
                }
            }
        }
    }

    /// List files with a given prefix.
    pub fn list(&self, prefix: String) -> Result<Vec<String>> {
        let backend = self
            .backend
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
            })?;

        match &*backend {
            StorageBackend::Mock(storage) => {
                let storage = storage.read().map_err(|_| PaykitMobileError::Internal {
                    msg: "Lock poisoned".to_string(),
                })?;
                Ok(storage
                    .data
                    .keys()
                    .filter(|k| k.starts_with(&prefix))
                    .cloned()
                    .collect())
            }
            StorageBackend::Callback(callback) => {
                let result = callback.list(prefix);
                if result.success {
                    Ok(result.entries)
                } else {
                    Err(PaykitMobileError::Transport {
                        msg: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                    })
                }
            }
        }
    }
}

// ============================================================================
// Unauthenticated Transport FFI
// ============================================================================

/// Internal storage backend for unauthenticated transport.
enum UnauthenticatedStorageBackend {
    /// Mock in-memory storage for testing
    Mock(Arc<RwLock<MockStorage>>),
    /// Real callback-based storage for production
    Callback(Box<dyn PubkyUnauthenticatedStorageCallback>),
}

/// FFI wrapper for unauthenticated (read-only) transport operations.
///
/// This wraps read-only access to public Pubky storage.
/// Mobile apps can use this to discover payment methods for other users.
///
/// # Creating in Production
///
/// Implement `PubkyUnauthenticatedStorageCallback` in Swift/Kotlin:
///
/// ```swift
/// class MyPublicStorage: PubkyUnauthenticatedStorageCallback {
///     func get(ownerPubkey: String, path: String) -> StorageGetResult {
///         // Use Pubky SDK to read from public storage
///         let url = "pubky://\(ownerPubkey)\(path)"
///         if let content = try? pubkyClient.get(url) {
///             return StorageGetResult.ok(content: content)
///         }
///         return StorageGetResult.ok(content: nil) // Not found
///     }
///     
///     func list(ownerPubkey: String, prefix: String) -> StorageListResult {
///         // ... implement using Pubky SDK
///     }
/// }
///
/// let transport = UnauthenticatedTransportFFI.fromCallback(MyPublicStorage())
/// ```
#[derive(uniffi::Object)]
pub struct UnauthenticatedTransportFFI {
    /// Storage backend (mock or callback-based).
    backend: RwLock<UnauthenticatedStorageBackend>,
}

#[uniffi::export]
impl UnauthenticatedTransportFFI {
    /// Create a new unauthenticated transport for testing/development.
    ///
    /// Uses in-memory storage - no network calls are made.
    #[uniffi::constructor]
    pub fn new_mock() -> Arc<Self> {
        Arc::new(Self {
            backend: RwLock::new(UnauthenticatedStorageBackend::Mock(Arc::new(RwLock::new(
                MockStorage::default(),
            )))),
        })
    }

    /// Create unauthenticated transport from a storage callback.
    ///
    /// # Arguments
    ///
    /// * `callback` - Storage callback implementing PubkyUnauthenticatedStorageCallback
    ///
    /// # Example (Swift)
    ///
    /// ```swift
    /// class MyPublicStorage: PubkyUnauthenticatedStorageCallback {
    ///     func get(ownerPubkey: String, path: String) -> StorageGetResult {
    ///         // Use Pubky SDK to read from public storage
    ///         // ...
    ///     }
    /// }
    ///
    /// let transport = UnauthenticatedTransportFFI.fromCallback(MyPublicStorage())
    /// ```
    #[uniffi::constructor]
    pub fn from_callback(callback: Box<dyn PubkyUnauthenticatedStorageCallback>) -> Arc<Self> {
        Arc::new(Self {
            backend: RwLock::new(UnauthenticatedStorageBackend::Callback(callback)),
        })
    }

    /// Create unauthenticated transport from Pubky SDK configuration.
    ///
    /// # Deprecated
    ///
    /// This method creates a mock transport. Use `from_callback()` for production.
    ///
    /// # Arguments
    ///
    /// * `config_json` - JSON configuration (validated but not used)
    #[uniffi::constructor]
    pub fn from_config_json(config_json: String) -> Result<Arc<Self>> {
        // Validate JSON
        serde_json::from_str::<serde_json::Value>(&config_json).map_err(|e| {
            PaykitMobileError::Serialization {
                msg: format!("Invalid config JSON: {}", e),
            }
        })?;

        // Create mock transport - use from_callback() for real implementation
        Ok(Self::new_mock())
    }

    /// Create unauthenticated transport that shares mock storage with an authenticated transport.
    ///
    /// Useful for testing when you want reads to see writes from the same session.
    ///
    /// # Note
    ///
    /// This only works with mock transports. For callback-based transports,
    /// create a new `UnauthenticatedTransportFFI::from_callback()` that shares
    /// the underlying Pubky client.
    #[uniffi::constructor]
    pub fn from_authenticated(auth: Arc<AuthenticatedTransportFFI>) -> Result<Arc<Self>> {
        let backend = auth
            .backend
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
            })?;

        match &*backend {
            StorageBackend::Mock(storage) => Ok(Arc::new(Self {
                backend: RwLock::new(UnauthenticatedStorageBackend::Mock(storage.clone())),
            })),
            StorageBackend::Callback(_) => Err(PaykitMobileError::Validation {
                msg: "Cannot create unauthenticated transport from callback-based authenticated transport. Use from_callback() instead.".to_string(),
            }),
        }
    }

    /// Check if this transport uses a real callback (production) or mock storage.
    ///
    /// Returns `true` for mock transport, `false` for callback-based transport.
    /// Returns an error if the internal lock is poisoned.
    pub fn is_mock(&self) -> Result<bool> {
        let backend = self
            .backend
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Transport lock poisoned".to_string(),
            })?;
        Ok(matches!(*backend, UnauthenticatedStorageBackend::Mock(_)))
    }

    /// Get a file at the given path from a public key's storage.
    pub fn get(&self, owner_pubkey: String, path: String) -> Result<Option<String>> {
        let backend = self
            .backend
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
            })?;

        match &*backend {
            UnauthenticatedStorageBackend::Mock(storage) => {
                let storage = storage.read().map_err(|_| PaykitMobileError::Internal {
                    msg: "Lock poisoned".to_string(),
                })?;
                // In mock mode, we ignore owner_pubkey since all data is in one storage
                let _ = owner_pubkey;
                Ok(storage.data.get(&path).cloned())
            }
            UnauthenticatedStorageBackend::Callback(callback) => {
                let result = callback.get(owner_pubkey, path);
                if result.success {
                    Ok(result.content)
                } else {
                    Err(PaykitMobileError::Transport {
                        msg: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                    })
                }
            }
        }
    }

    /// List files with a given prefix from a public key's storage.
    pub fn list(&self, owner_pubkey: String, prefix: String) -> Result<Vec<String>> {
        let backend = self
            .backend
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
            })?;

        match &*backend {
            UnauthenticatedStorageBackend::Mock(storage) => {
                let storage = storage.read().map_err(|_| PaykitMobileError::Internal {
                    msg: "Lock poisoned".to_string(),
                })?;
                // In mock mode, we ignore owner_pubkey
                let _ = owner_pubkey;
                Ok(storage
                    .data
                    .keys()
                    .filter(|k| k.starts_with(&prefix))
                    .cloned()
                    .collect())
            }
            UnauthenticatedStorageBackend::Callback(callback) => {
                let result = callback.list(owner_pubkey, prefix);
                if result.success {
                    Ok(result.entries)
                } else {
                    Err(PaykitMobileError::Transport {
                        msg: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                    })
                }
            }
        }
    }
}

// ============================================================================
// Directory Operations
// ============================================================================

/// Path prefix for Paykit payment endpoints.
pub const PAYKIT_PATH_PREFIX: &str = "/pub/paykit.app/v0/";

/// Path prefix for Pubky follows/contacts.
pub const PUBKY_FOLLOWS_PATH: &str = "/pub/pubky.app/follows/";

/// Publish a payment endpoint to the directory.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the owner
/// * `method_id` - Payment method identifier (e.g., "lightning", "onchain")
/// * `endpoint_data` - The endpoint data to publish
pub fn publish_payment_endpoint(
    transport: &AuthenticatedTransportFFI,
    method_id: &str,
    endpoint_data: &str,
) -> Result<()> {
    let path = format!("{}{}", PAYKIT_PATH_PREFIX, method_id);
    transport.put(path, endpoint_data.to_string())
}

/// Remove a payment endpoint from the directory.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the owner
/// * `method_id` - Payment method identifier to remove
pub fn remove_payment_endpoint(
    transport: &AuthenticatedTransportFFI,
    method_id: &str,
) -> Result<()> {
    let path = format!("{}{}", PAYKIT_PATH_PREFIX, method_id);
    transport.delete(path)
}

/// Fetch all supported payment methods for a public key.
///
/// # Arguments
///
/// * `transport` - Unauthenticated transport for reading
/// * `owner_pubkey` - The public key to query
///
/// # Returns
///
/// List of payment methods with their endpoints.
pub fn fetch_supported_payments(
    transport: &UnauthenticatedTransportFFI,
    owner_pubkey: &str,
) -> Result<Vec<PaymentMethod>> {
    let paths = transport.list(owner_pubkey.to_string(), PAYKIT_PATH_PREFIX.to_string())?;

    let mut methods = Vec::new();
    for path in paths {
        if let Some(method_id) = path.strip_prefix(PAYKIT_PATH_PREFIX) {
            if let Some(endpoint) = transport.get(owner_pubkey.to_string(), path.clone())? {
                methods.push(PaymentMethod {
                    method_id: method_id.to_string(),
                    endpoint,
                });
            }
        }
    }

    Ok(methods)
}

/// Fetch a specific payment endpoint for a public key.
///
/// # Arguments
///
/// * `transport` - Unauthenticated transport for reading
/// * `owner_pubkey` - The public key to query
/// * `method_id` - The payment method to fetch
///
/// # Returns
///
/// The endpoint data if found, None otherwise.
pub fn fetch_payment_endpoint(
    transport: &UnauthenticatedTransportFFI,
    owner_pubkey: &str,
    method_id: &str,
) -> Result<Option<String>> {
    let path = format!("{}{}", PAYKIT_PATH_PREFIX, method_id);
    transport.get(owner_pubkey.to_string(), path)
}

/// Fetch known contacts for a public key.
///
/// # Arguments
///
/// * `transport` - Unauthenticated transport for reading
/// * `owner_pubkey` - The public key to query
///
/// # Returns
///
/// List of contact public keys.
pub fn fetch_known_contacts(
    transport: &UnauthenticatedTransportFFI,
    owner_pubkey: &str,
) -> Result<Vec<String>> {
    let paths = transport.list(owner_pubkey.to_string(), PUBKY_FOLLOWS_PATH.to_string())?;

    let contacts: Vec<String> = paths
        .iter()
        .filter_map(|path| path.strip_prefix(PUBKY_FOLLOWS_PATH).map(String::from))
        .collect();

    Ok(contacts)
}

/// Add a contact to the follows list.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the owner
/// * `contact_pubkey` - The contact's public key to add
pub fn add_contact(transport: &AuthenticatedTransportFFI, contact_pubkey: &str) -> Result<()> {
    let path = format!("{}{}", PUBKY_FOLLOWS_PATH, contact_pubkey);
    transport.put(path, String::new())
}

/// Remove a contact from the follows list.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the owner
/// * `contact_pubkey` - The contact's public key to remove
pub fn remove_contact(transport: &AuthenticatedTransportFFI, contact_pubkey: &str) -> Result<()> {
    let path = format!("{}{}", PUBKY_FOLLOWS_PATH, contact_pubkey);
    transport.delete(path)
}

/// List all contacts.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the owner
///
/// # Returns
///
/// List of contact public keys.
pub fn list_contacts(transport: &AuthenticatedTransportFFI) -> Result<Vec<String>> {
    let paths = transport.list(PUBKY_FOLLOWS_PATH.to_string())?;

    let contacts: Vec<String> = paths
        .iter()
        .filter_map(|path| path.strip_prefix(PUBKY_FOLLOWS_PATH).map(String::from))
        .collect();

    Ok(contacts)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_transport_put_get() {
        let transport = AuthenticatedTransportFFI::new_mock("test_owner".to_string());

        transport
            .put("/test/path".to_string(), "test_value".to_string())
            .unwrap();
        let result = transport.get("/test/path".to_string()).unwrap();

        assert_eq!(result, Some("test_value".to_string()));
    }

    #[test]
    fn test_mock_transport_delete() {
        let transport = AuthenticatedTransportFFI::new_mock("test_owner".to_string());

        transport
            .put("/test/path".to_string(), "test_value".to_string())
            .unwrap();
        transport.delete("/test/path".to_string()).unwrap();
        let result = transport.get("/test/path".to_string()).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_mock_transport_list() {
        let transport = AuthenticatedTransportFFI::new_mock("test_owner".to_string());

        transport
            .put("/pub/test/a".to_string(), "1".to_string())
            .unwrap();
        transport
            .put("/pub/test/b".to_string(), "2".to_string())
            .unwrap();
        transport
            .put("/other/c".to_string(), "3".to_string())
            .unwrap();

        let result = transport.list("/pub/test/".to_string()).unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.contains(&"/pub/test/a".to_string()));
        assert!(result.contains(&"/pub/test/b".to_string()));
    }

    #[test]
    fn test_unauthenticated_from_authenticated() {
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        auth.put("/test/path".to_string(), "shared_value".to_string())
            .unwrap();

        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth).unwrap();
        let result = unauth
            .get("test_owner".to_string(), "/test/path".to_string())
            .unwrap();

        assert_eq!(result, Some("shared_value".to_string()));
    }

    #[test]
    fn test_publish_and_fetch_payment_endpoint() {
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

        // Publish endpoint
        publish_payment_endpoint(&auth, "lightning", "lnbc1...").unwrap();

        // Fetch endpoint
        let result = fetch_payment_endpoint(&unauth, "test_owner", "lightning").unwrap();
        assert_eq!(result, Some("lnbc1...".to_string()));
    }

    #[test]
    fn test_fetch_supported_payments() {
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

        // Publish multiple endpoints
        publish_payment_endpoint(&auth, "lightning", "lnbc1...").unwrap();
        publish_payment_endpoint(&auth, "onchain", "bc1q...").unwrap();

        // Fetch all
        let methods = fetch_supported_payments(&unauth, "test_owner").unwrap();
        assert_eq!(methods.len(), 2);
    }

    #[test]
    fn test_remove_payment_endpoint() {
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

        // Publish and remove
        publish_payment_endpoint(&auth, "lightning", "lnbc1...").unwrap();
        remove_payment_endpoint(&auth, "lightning").unwrap();

        // Verify removed
        let result = fetch_payment_endpoint(&unauth, "test_owner", "lightning").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_contact_management() {
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());

        // Add contacts
        add_contact(&auth, "contact1").unwrap();
        add_contact(&auth, "contact2").unwrap();

        // List contacts
        let contacts = list_contacts(&auth).unwrap();
        assert_eq!(contacts.len(), 2);
        assert!(contacts.contains(&"contact1".to_string()));
        assert!(contacts.contains(&"contact2".to_string()));

        // Remove contact
        remove_contact(&auth, "contact1").unwrap();
        let contacts = list_contacts(&auth).unwrap();
        assert_eq!(contacts.len(), 1);
        assert!(contacts.contains(&"contact2".to_string()));
    }

    #[test]
    fn test_fetch_known_contacts() {
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

        // Add contacts
        add_contact(&auth, "contact1").unwrap();
        add_contact(&auth, "contact2").unwrap();

        // Fetch via unauthenticated transport
        let contacts = fetch_known_contacts(&unauth, "test_owner").unwrap();
        assert_eq!(contacts.len(), 2);
    }

    #[test]
    fn test_callback_based_authenticated_transport() {
        // Test callback transport implementation
        struct TestCallback;
        impl PubkyAuthenticatedStorageCallback for TestCallback {
            fn put(&self, path: String, content: String) -> StorageOperationResult {
                // Just verify the call was made
                if path.is_empty() || content.is_empty() {
                    StorageOperationResult::err("Empty path or content".to_string())
                } else {
                    StorageOperationResult::ok()
                }
            }

            fn get(&self, path: String) -> StorageGetResult {
                if path == "/test/exists" {
                    StorageGetResult::ok(Some("test_content".to_string()))
                } else {
                    StorageGetResult::ok(None)
                }
            }

            fn delete(&self, _path: String) -> StorageOperationResult {
                StorageOperationResult::ok()
            }

            fn list(&self, _prefix: String) -> StorageListResult {
                StorageListResult::ok(vec!["/test/a".to_string(), "/test/b".to_string()])
            }
        }

        let transport = AuthenticatedTransportFFI::from_callback(
            Box::new(TestCallback),
            "test_owner".to_string(),
        );

        assert!(!transport.is_mock().unwrap());

        // Test get
        let result = transport.get("/test/exists".to_string()).unwrap();
        assert_eq!(result, Some("test_content".to_string()));

        let result = transport.get("/test/missing".to_string()).unwrap();
        assert!(result.is_none());

        // Test list
        let result = transport.list("/test/".to_string()).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_callback_based_unauthenticated_transport() {
        // Test callback transport implementation
        struct TestCallback;
        impl PubkyUnauthenticatedStorageCallback for TestCallback {
            fn get(&self, owner_pubkey: String, path: String) -> StorageGetResult {
                if owner_pubkey == "user1" && path == "/test/data" {
                    StorageGetResult::ok(Some("user1_data".to_string()))
                } else {
                    StorageGetResult::ok(None)
                }
            }

            fn list(&self, _owner_pubkey: String, _prefix: String) -> StorageListResult {
                StorageListResult::ok(vec!["/pub/a".to_string()])
            }
        }

        let transport = UnauthenticatedTransportFFI::from_callback(Box::new(TestCallback));

        assert!(!transport.is_mock().unwrap());

        // Test get
        let result = transport
            .get("user1".to_string(), "/test/data".to_string())
            .unwrap();
        assert_eq!(result, Some("user1_data".to_string()));

        let result = transport
            .get("user2".to_string(), "/test/data".to_string())
            .unwrap();
        assert!(result.is_none());
    }
}
