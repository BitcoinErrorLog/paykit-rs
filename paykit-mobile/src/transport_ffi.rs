//! Transport FFI Wrappers
//!
//! This module provides FFI-safe wrappers for Paykit transport traits,
//! enabling mobile applications to perform directory operations.
//!
//! # Overview
//!
//! Mobile apps need to:
//! 1. Create a Pubky session (using Pubky SDK directly)
//! 2. Wrap it in `AuthenticatedTransportFFI` for write operations
//! 3. Wrap public storage in `UnauthenticatedTransportFFI` for read operations
//!
//! # Example
//!
//! ```ignore
//! // From Swift/Kotlin
//! let transport = AuthenticatedTransportFFI.fromSession(sessionData)
//! client.publishPaymentEndpoint(transport, "lightning", "lnbc1...")
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::{PaykitMobileError, PaymentMethod, Result};

// ============================================================================
// Mock Transport for Testing/Development
// ============================================================================

/// In-memory storage for mock transport operations.
/// Used for testing and development when a real Pubky connection is not available.
#[derive(Default)]
struct MockStorage {
    data: HashMap<String, String>,
}

// ============================================================================
// Authenticated Transport FFI
// ============================================================================

/// FFI wrapper for authenticated transport operations.
///
/// This wraps authenticated write access to Pubky homeservers.
/// Mobile apps should create this by wrapping their Pubky session.
#[derive(uniffi::Object)]
pub struct AuthenticatedTransportFFI {
    /// The owner's public key (z-base32 encoded).
    owner_pubkey: String,
    /// Mock storage for development (will be replaced with real Pubky transport).
    mock_storage: Arc<RwLock<MockStorage>>,
}

#[uniffi::export]
impl AuthenticatedTransportFFI {
    /// Create a new authenticated transport for testing/development.
    ///
    /// In production, use `from_pubky_session` instead.
    #[uniffi::constructor]
    pub fn new_mock(owner_pubkey: String) -> Arc<Self> {
        Arc::new(Self {
            owner_pubkey,
            mock_storage: Arc::new(RwLock::new(MockStorage::default())),
        })
    }

    /// Create authenticated transport from a Pubky session.
    ///
    /// # Arguments
    ///
    /// * `session_json` - JSON-serialized Pubky session data
    /// * `owner_pubkey` - The owner's public key (z-base32 encoded)
    ///
    /// # Note
    ///
    /// This currently creates a mock transport. In production, it will
    /// deserialize the session and create a real `PubkyAuthenticatedTransport`.
    #[uniffi::constructor]
    pub fn from_session_json(session_json: String, owner_pubkey: String) -> Result<Arc<Self>> {
        // Validate JSON
        serde_json::from_str::<serde_json::Value>(&session_json)
            .map_err(|e| PaykitMobileError::Serialization {
                message: format!("Invalid session JSON: {}", e),
            })?;

        // For now, create a mock transport
        // TODO: Deserialize PubkySession and create PubkyAuthenticatedTransport
        Ok(Arc::new(Self {
            owner_pubkey,
            mock_storage: Arc::new(RwLock::new(MockStorage::default())),
        }))
    }

    /// Get the owner's public key.
    pub fn owner_pubkey(&self) -> String {
        self.owner_pubkey.clone()
    }

    /// Put (create or update) a file at the given path.
    pub fn put(&self, path: String, content: String) -> Result<()> {
        let mut storage = self.mock_storage.write().map_err(|_| {
            PaykitMobileError::Internal {
                message: "Lock poisoned".to_string(),
            }
        })?;
        storage.data.insert(path, content);
        Ok(())
    }

    /// Get a file at the given path.
    pub fn get(&self, path: String) -> Result<Option<String>> {
        let storage = self.mock_storage.read().map_err(|_| {
            PaykitMobileError::Internal {
                message: "Lock poisoned".to_string(),
            }
        })?;
        Ok(storage.data.get(&path).cloned())
    }

    /// Delete a file at the given path.
    pub fn delete(&self, path: String) -> Result<()> {
        let mut storage = self.mock_storage.write().map_err(|_| {
            PaykitMobileError::Internal {
                message: "Lock poisoned".to_string(),
            }
        })?;
        storage.data.remove(&path);
        Ok(())
    }

    /// List files with a given prefix.
    pub fn list(&self, prefix: String) -> Result<Vec<String>> {
        let storage = self.mock_storage.read().map_err(|_| {
            PaykitMobileError::Internal {
                message: "Lock poisoned".to_string(),
            }
        })?;
        Ok(storage
            .data
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect())
    }
}

// ============================================================================
// Unauthenticated Transport FFI
// ============================================================================

/// FFI wrapper for unauthenticated (read-only) transport operations.
///
/// This wraps read-only access to public Pubky storage.
/// Mobile apps can use this to discover payment methods for other users.
#[derive(uniffi::Object)]
pub struct UnauthenticatedTransportFFI {
    /// Mock storage for development (will be replaced with real Pubky transport).
    mock_storage: Arc<RwLock<MockStorage>>,
}

#[uniffi::export]
impl UnauthenticatedTransportFFI {
    /// Create a new unauthenticated transport for testing/development.
    #[uniffi::constructor]
    pub fn new_mock() -> Arc<Self> {
        Arc::new(Self {
            mock_storage: Arc::new(RwLock::new(MockStorage::default())),
        })
    }

    /// Create unauthenticated transport from Pubky SDK configuration.
    ///
    /// # Arguments
    ///
    /// * `config_json` - JSON configuration for Pubky SDK
    ///
    /// # Note
    ///
    /// This currently creates a mock transport. In production, it will
    /// initialize the Pubky SDK and create a real `PubkyUnauthenticatedTransport`.
    #[uniffi::constructor]
    pub fn from_config_json(config_json: String) -> Result<Arc<Self>> {
        // Validate JSON
        serde_json::from_str::<serde_json::Value>(&config_json)
            .map_err(|e| PaykitMobileError::Serialization {
                message: format!("Invalid config JSON: {}", e),
            })?;

        // For now, create a mock transport
        // TODO: Initialize Pubky SDK and create PubkyUnauthenticatedTransport
        Ok(Arc::new(Self {
            mock_storage: Arc::new(RwLock::new(MockStorage::default())),
        }))
    }

    /// Create unauthenticated transport that shares storage with an authenticated transport.
    ///
    /// Useful for testing when you want reads to see writes from the same session.
    #[uniffi::constructor]
    pub fn from_authenticated(auth: Arc<AuthenticatedTransportFFI>) -> Arc<Self> {
        Arc::new(Self {
            mock_storage: auth.mock_storage.clone(),
        })
    }

    /// Get a file at the given path from a public key's storage.
    pub fn get(&self, owner_pubkey: String, path: String) -> Result<Option<String>> {
        let storage = self.mock_storage.read().map_err(|_| {
            PaykitMobileError::Internal {
                message: "Lock poisoned".to_string(),
            }
        })?;
        // In mock mode, we ignore owner_pubkey since all data is in one storage
        // In production, this would query the owner's homeserver
        let _ = owner_pubkey;
        Ok(storage.data.get(&path).cloned())
    }

    /// List files with a given prefix from a public key's storage.
    pub fn list(&self, owner_pubkey: String, prefix: String) -> Result<Vec<String>> {
        let storage = self.mock_storage.read().map_err(|_| {
            PaykitMobileError::Internal {
                message: "Lock poisoned".to_string(),
            }
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
pub fn add_contact(
    transport: &AuthenticatedTransportFFI,
    contact_pubkey: &str,
) -> Result<()> {
    let path = format!("{}{}", PUBKY_FOLLOWS_PATH, contact_pubkey);
    transport.put(path, String::new())
}

/// Remove a contact from the follows list.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the owner
/// * `contact_pubkey` - The contact's public key to remove
pub fn remove_contact(
    transport: &AuthenticatedTransportFFI,
    contact_pubkey: &str,
) -> Result<()> {
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
        
        transport.put("/test/path".to_string(), "test_value".to_string()).unwrap();
        let result = transport.get("/test/path".to_string()).unwrap();
        
        assert_eq!(result, Some("test_value".to_string()));
    }

    #[test]
    fn test_mock_transport_delete() {
        let transport = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        
        transport.put("/test/path".to_string(), "test_value".to_string()).unwrap();
        transport.delete("/test/path".to_string()).unwrap();
        let result = transport.get("/test/path".to_string()).unwrap();
        
        assert!(result.is_none());
    }

    #[test]
    fn test_mock_transport_list() {
        let transport = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        
        transport.put("/pub/test/a".to_string(), "1".to_string()).unwrap();
        transport.put("/pub/test/b".to_string(), "2".to_string()).unwrap();
        transport.put("/other/c".to_string(), "3".to_string()).unwrap();
        
        let result = transport.list("/pub/test/".to_string()).unwrap();
        
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"/pub/test/a".to_string()));
        assert!(result.contains(&"/pub/test/b".to_string()));
    }

    #[test]
    fn test_unauthenticated_from_authenticated() {
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        auth.put("/test/path".to_string(), "shared_value".to_string()).unwrap();
        
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth);
        let result = unauth.get("test_owner".to_string(), "/test/path".to_string()).unwrap();
        
        assert_eq!(result, Some("shared_value".to_string()));
    }

    #[test]
    fn test_publish_and_fetch_payment_endpoint() {
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone());
        
        // Publish endpoint
        publish_payment_endpoint(&auth, "lightning", "lnbc1...").unwrap();
        
        // Fetch endpoint
        let result = fetch_payment_endpoint(&unauth, "test_owner", "lightning").unwrap();
        assert_eq!(result, Some("lnbc1...".to_string()));
    }

    #[test]
    fn test_fetch_supported_payments() {
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone());
        
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
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone());
        
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
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone());
        
        // Add contacts
        add_contact(&auth, "contact1").unwrap();
        add_contact(&auth, "contact2").unwrap();
        
        // Fetch via unauthenticated transport
        let contacts = fetch_known_contacts(&unauth, "test_owner").unwrap();
        assert_eq!(contacts.len(), 2);
    }
}
