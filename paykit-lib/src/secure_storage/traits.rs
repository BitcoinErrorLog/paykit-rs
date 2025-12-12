//! Core traits for secure key storage.

use std::fmt;
use std::future::Future;

/// Error codes for secure storage operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum SecureStorageErrorCode {
    /// Key not found
    NotFound = 1000,
    /// Access denied (auth required)
    AccessDenied = 2000,
    /// User cancelled authentication
    UserCancelled = 2001,
    /// Biometric authentication failed
    BiometricFailed = 2002,
    /// Storage is locked
    StorageLocked = 2003,
    /// Key already exists
    AlreadyExists = 3000,
    /// Invalid key format
    InvalidKey = 4000,
    /// Encryption failed
    EncryptionFailed = 5000,
    /// Decryption failed
    DecryptionFailed = 5001,
    /// Platform not supported
    Unsupported = 6000,
    /// Storage quota exceeded
    QuotaExceeded = 7000,
    /// Internal error
    Internal = 9999,
}

/// Error type for secure storage operations.
#[derive(Debug)]
pub struct SecureStorageError {
    /// Error code for FFI/mobile integration
    pub code: SecureStorageErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Key identifier related to error (if applicable)
    pub key_id: Option<String>,
}

impl SecureStorageError {
    /// Create a new error.
    pub fn new(code: SecureStorageErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            key_id: None,
        }
    }

    /// Create a new error with associated key ID.
    pub fn with_key(
        code: SecureStorageErrorCode,
        message: impl Into<String>,
        key_id: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            key_id: Some(key_id.into()),
        }
    }

    /// Create a "not found" error.
    pub fn not_found(key_id: impl Into<String>) -> Self {
        let key_id = key_id.into();
        Self {
            code: SecureStorageErrorCode::NotFound,
            message: format!("Key not found: {}", key_id),
            key_id: Some(key_id),
        }
    }

    /// Create an "already exists" error.
    pub fn already_exists(key_id: impl Into<String>) -> Self {
        let key_id = key_id.into();
        Self {
            code: SecureStorageErrorCode::AlreadyExists,
            message: format!("Key already exists: {}", key_id),
            key_id: Some(key_id),
        }
    }

    /// Create an "access denied" error.
    pub fn access_denied(reason: impl Into<String>) -> Self {
        Self::new(SecureStorageErrorCode::AccessDenied, reason)
    }

    /// Create an "unsupported" error.
    pub fn unsupported(feature: impl Into<String>) -> Self {
        Self::new(
            SecureStorageErrorCode::Unsupported,
            format!("Feature not supported: {}", feature.into()),
        )
    }

    /// Check if this error indicates the key wasn't found.
    pub fn is_not_found(&self) -> bool {
        self.code == SecureStorageErrorCode::NotFound
    }

    /// Check if this error requires user authentication.
    pub fn requires_auth(&self) -> bool {
        matches!(
            self.code,
            SecureStorageErrorCode::AccessDenied
                | SecureStorageErrorCode::StorageLocked
                | SecureStorageErrorCode::BiometricFailed
        )
    }

    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.code,
            SecureStorageErrorCode::UserCancelled
                | SecureStorageErrorCode::BiometricFailed
                | SecureStorageErrorCode::StorageLocked
        )
    }
}

impl fmt::Display for SecureStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(key_id) = &self.key_id {
            write!(f, "{} (key: {})", self.message, key_id)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for SecureStorageError {}

/// Result type for secure storage operations.
pub type SecureStorageResult<T> = Result<T, SecureStorageError>;

/// Metadata about a stored key.
#[derive(Debug, Clone)]
pub struct KeyMetadata {
    /// Key identifier
    pub key_id: String,
    /// Key size in bytes
    pub size_bytes: usize,
    /// When the key was created (Unix timestamp)
    pub created_at: i64,
    /// When the key was last accessed (Unix timestamp)
    pub last_accessed: Option<i64>,
    /// Whether the key requires authentication to access
    pub requires_auth: bool,
    /// Application-specific tags
    pub tags: Vec<String>,
}

impl KeyMetadata {
    /// Create metadata for a new key.
    pub fn new(key_id: impl Into<String>, size_bytes: usize) -> Self {
        Self {
            key_id: key_id.into(),
            size_bytes,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            last_accessed: None,
            requires_auth: false,
            tags: Vec::new(),
        }
    }

    /// Set whether authentication is required.
    pub fn with_auth(mut self, requires_auth: bool) -> Self {
        self.requires_auth = requires_auth;
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Options for storing a key.
#[derive(Debug, Clone, Default)]
pub struct StoreOptions {
    /// Overwrite if key already exists
    pub overwrite: bool,
    /// Require biometric/device authentication to access
    pub require_auth: bool,
    /// Optional tags for organization
    pub tags: Vec<String>,
}

impl StoreOptions {
    /// Create default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allow overwriting existing keys.
    pub fn overwrite(mut self) -> Self {
        self.overwrite = true;
        self
    }

    /// Require authentication to access.
    pub fn require_auth(mut self) -> Self {
        self.require_auth = true;
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Platform-agnostic secure key storage trait.
///
/// Implementations should:
/// - Encrypt keys at rest using platform-specific secure storage
/// - Zeroize sensitive data from memory when possible
/// - Never log or expose key material
/// - Support access control via biometrics/device unlock where available
pub trait SecureKeyStorage: Send + Sync {
    /// Store a key with the given identifier.
    ///
    /// # Arguments
    /// * `key_id` - Unique identifier for the key
    /// * `key_data` - The secret key material to store
    /// * `options` - Storage options (overwrite, auth requirements, etc.)
    ///
    /// # Errors
    /// - `AlreadyExists` if key exists and overwrite is false
    /// - `QuotaExceeded` if storage is full
    /// - `EncryptionFailed` if platform encryption fails
    fn store(
        &self,
        key_id: &str,
        key_data: &[u8],
        options: StoreOptions,
    ) -> impl Future<Output = SecureStorageResult<()>> + Send;

    /// Retrieve a key by its identifier.
    ///
    /// # Arguments
    /// * `key_id` - Unique identifier for the key
    ///
    /// # Returns
    /// The key data, or None if not found.
    ///
    /// # Errors
    /// - `AccessDenied` if authentication fails
    /// - `DecryptionFailed` if platform decryption fails
    fn retrieve(
        &self,
        key_id: &str,
    ) -> impl Future<Output = SecureStorageResult<Option<Vec<u8>>>> + Send;

    /// Delete a key by its identifier.
    ///
    /// # Arguments
    /// * `key_id` - Unique identifier for the key
    ///
    /// # Errors
    /// - `NotFound` if key doesn't exist
    fn delete(&self, key_id: &str) -> impl Future<Output = SecureStorageResult<()>> + Send;

    /// Check if a key exists.
    ///
    /// # Arguments
    /// * `key_id` - Unique identifier for the key
    fn exists(&self, key_id: &str) -> impl Future<Output = SecureStorageResult<bool>> + Send;

    /// Get metadata for a key.
    ///
    /// # Arguments
    /// * `key_id` - Unique identifier for the key
    ///
    /// # Returns
    /// Key metadata, or None if not found.
    fn get_metadata(
        &self,
        key_id: &str,
    ) -> impl Future<Output = SecureStorageResult<Option<KeyMetadata>>> + Send;

    /// List all stored key IDs.
    ///
    /// # Returns
    /// Vector of key identifiers.
    fn list_keys(&self) -> impl Future<Output = SecureStorageResult<Vec<String>>> + Send;

    /// Clear all stored keys.
    ///
    /// Use with caution - this is irreversible.
    fn clear_all(&self) -> impl Future<Output = SecureStorageResult<()>> + Send;
}

/// Extension trait for convenience methods.
///
/// Provides common patterns like `store_simple` that don't require
/// specifying all options. Part of the public API for SDK consumers.
#[allow(dead_code)] // Public API for external consumers
pub trait SecureKeyStorageExt: SecureKeyStorage {
    /// Store a key with default options (no overwrite, no auth required).
    fn store_simple(
        &self,
        key_id: &str,
        key_data: &[u8],
    ) -> impl Future<Output = SecureStorageResult<()>> + Send {
        self.store(key_id, key_data, StoreOptions::default())
    }

    /// Store or update a key (overwrite if exists).
    fn upsert(
        &self,
        key_id: &str,
        key_data: &[u8],
    ) -> impl Future<Output = SecureStorageResult<()>> + Send {
        self.store(key_id, key_data, StoreOptions::new().overwrite())
    }

    /// Retrieve a key, returning error if not found.
    fn retrieve_required(
        &self,
        key_id: &str,
    ) -> impl Future<Output = SecureStorageResult<Vec<u8>>> + Send {
        async move {
            self.retrieve(key_id)
                .await?
                .ok_or_else(|| SecureStorageError::not_found(key_id))
        }
    }

    /// Delete a key if it exists (no error if missing).
    fn delete_if_exists(
        &self,
        key_id: &str,
    ) -> impl Future<Output = SecureStorageResult<()>> + Send {
        async move {
            match self.delete(key_id).await {
                Ok(()) => Ok(()),
                Err(e) if e.is_not_found() => Ok(()),
                Err(e) => Err(e),
            }
        }
    }
}

// Blanket implementation
impl<T: SecureKeyStorage> SecureKeyStorageExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = SecureStorageError::not_found("my-key");
        assert!(err.is_not_found());
        assert!(!err.requires_auth());
        assert_eq!(err.key_id, Some("my-key".to_string()));
    }

    #[test]
    fn test_metadata_creation() {
        let meta = KeyMetadata::new("test-key", 32)
            .with_auth(true)
            .with_tag("wallet");

        assert_eq!(meta.key_id, "test-key");
        assert_eq!(meta.size_bytes, 32);
        assert!(meta.requires_auth);
        assert_eq!(meta.tags, vec!["wallet".to_string()]);
    }

    #[test]
    fn test_store_options() {
        let opts = StoreOptions::new()
            .overwrite()
            .require_auth()
            .with_tag("important");

        assert!(opts.overwrite);
        assert!(opts.require_auth);
        assert!(opts.tags.contains(&"important".to_string()));
    }
}
