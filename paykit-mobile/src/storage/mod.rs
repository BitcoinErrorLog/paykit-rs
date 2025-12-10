//! Secure Storage Abstraction
//!
//! This module provides a platform-agnostic secure storage trait that can be
//! implemented by platform-specific storage backends:
//!
//! - **iOS**: Keychain Services
//! - **Android**: EncryptedSharedPreferences or Keystore
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────┐
//! │                      Paykit Mobile                              │
//! │  ┌──────────────────────────────────────────────────────────┐  │
//! │  │                  SecureStorage Trait                      │  │
//! │  │  - store(key, value)                                      │  │
//! │  │  - retrieve(key) -> Option<value>                         │  │
//! │  │  - delete(key)                                            │  │
//! │  │  - list_keys() -> Vec<key>                                │  │
//! │  └──────────────────────────────────────────────────────────┘  │
//! │                            │                                    │
//! │         ┌──────────────────┼──────────────────┐                │
//! │         ▼                  ▼                  ▼                │
//! │  ┌─────────────┐   ┌─────────────┐   ┌─────────────────┐      │
//! │  │ iOS Keychain│   │ Android ESP │   │ In-Memory/File  │      │
//! │  │  (Swift)    │   │  (Kotlin)   │   │  (Testing)      │      │
//! │  └─────────────┘   └─────────────┘   └─────────────────┘      │
//! └────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! // Platform provides storage implementation
//! let storage = KeychainStorage::new("com.example.paykit");
//!
//! // Paykit uses it
//! storage.store("private_key", key_bytes)?;
//! let key = storage.retrieve("private_key")?;
//! ```

use std::sync::Arc;

/// Error type for storage operations.
#[derive(Debug, Clone)]
pub struct StorageError {
    pub code: StorageErrorCode,
    pub message: String,
}

impl StorageError {
    pub fn new(code: StorageErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn not_found(key: &str) -> Self {
        Self::new(StorageErrorCode::NotFound, format!("Key not found: {}", key))
    }

    pub fn access_denied(message: impl Into<String>) -> Self {
        Self::new(StorageErrorCode::AccessDenied, message)
    }

    pub fn encryption_error(message: impl Into<String>) -> Self {
        Self::new(StorageErrorCode::EncryptionError, message)
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for StorageError {}

/// Storage error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageErrorCode {
    /// Key not found in storage.
    NotFound,
    /// Access denied (biometrics failed, etc.).
    AccessDenied,
    /// Encryption or decryption failed.
    EncryptionError,
    /// Storage is full.
    StorageFull,
    /// Platform-specific error.
    PlatformError,
    /// Unknown error.
    Unknown,
}

/// Result type for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

/// Secure storage trait for platform-specific implementations.
///
/// This trait abstracts secure storage operations across platforms.
/// Implementations should use platform-native secure storage:
///
/// - iOS: Keychain Services
/// - Android: EncryptedSharedPreferences or Android Keystore
///
/// # Thread Safety
///
/// Implementations must be thread-safe (Send + Sync).
pub trait SecureStorage: Send + Sync {
    /// Store a value securely.
    ///
    /// # Arguments
    ///
    /// * `key` - Unique identifier for the value
    /// * `value` - Data to store (will be encrypted)
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails (full, access denied, etc.).
    fn store(&self, key: &str, value: &[u8]) -> StorageResult<()>;

    /// Retrieve a value from secure storage.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to retrieve
    ///
    /// # Returns
    ///
    /// The decrypted value, or None if not found.
    fn retrieve(&self, key: &str) -> StorageResult<Option<Vec<u8>>>;

    /// Delete a value from secure storage.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to delete
    ///
    /// # Errors
    ///
    /// Returns an error only on platform errors (not if key doesn't exist).
    fn delete(&self, key: &str) -> StorageResult<()>;

    /// List all keys in storage.
    ///
    /// # Returns
    ///
    /// All stored keys.
    fn list_keys(&self) -> StorageResult<Vec<String>>;

    /// Check if a key exists.
    fn contains(&self, key: &str) -> StorageResult<bool> {
        Ok(self.retrieve(key)?.is_some())
    }

    /// Clear all stored values.
    fn clear(&self) -> StorageResult<()> {
        for key in self.list_keys()? {
            self.delete(&key)?;
        }
        Ok(())
    }
}

/// In-memory storage for testing.
///
/// This implementation stores data in memory without encryption.
/// **Do not use in production!**
#[derive(Default)]
pub struct InMemoryStorage {
    data: std::sync::RwLock<std::collections::HashMap<String, Vec<u8>>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage.
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecureStorage for InMemoryStorage {
    fn store(&self, key: &str, value: &[u8]) -> StorageResult<()> {
        let mut data = self.data.write().map_err(|_| {
            StorageError::new(StorageErrorCode::Unknown, "Lock poisoned")
        })?;
        data.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn retrieve(&self, key: &str) -> StorageResult<Option<Vec<u8>>> {
        let data = self.data.read().map_err(|_| {
            StorageError::new(StorageErrorCode::Unknown, "Lock poisoned")
        })?;
        Ok(data.get(key).cloned())
    }

    fn delete(&self, key: &str) -> StorageResult<()> {
        let mut data = self.data.write().map_err(|_| {
            StorageError::new(StorageErrorCode::Unknown, "Lock poisoned")
        })?;
        data.remove(key);
        Ok(())
    }

    fn list_keys(&self) -> StorageResult<Vec<String>> {
        let data = self.data.read().map_err(|_| {
            StorageError::new(StorageErrorCode::Unknown, "Lock poisoned")
        })?;
        Ok(data.keys().cloned().collect())
    }
}

/// Storage wrapper that adds a prefix to all keys.
///
/// Useful for namespace isolation between different components.
pub struct PrefixedStorage<S: SecureStorage> {
    inner: S,
    prefix: String,
}

impl<S: SecureStorage> PrefixedStorage<S> {
    /// Create a new prefixed storage.
    pub fn new(inner: S, prefix: impl Into<String>) -> Self {
        Self {
            inner,
            prefix: prefix.into(),
        }
    }

    fn prefixed_key(&self, key: &str) -> String {
        format!("{}:{}", self.prefix, key)
    }

    fn strip_prefix<'a>(&self, key: &'a str) -> Option<&'a str> {
        key.strip_prefix(&format!("{}:", self.prefix))
    }
}

impl<S: SecureStorage> SecureStorage for PrefixedStorage<S> {
    fn store(&self, key: &str, value: &[u8]) -> StorageResult<()> {
        self.inner.store(&self.prefixed_key(key), value)
    }

    fn retrieve(&self, key: &str) -> StorageResult<Option<Vec<u8>>> {
        self.inner.retrieve(&self.prefixed_key(key))
    }

    fn delete(&self, key: &str) -> StorageResult<()> {
        self.inner.delete(&self.prefixed_key(key))
    }

    fn list_keys(&self) -> StorageResult<Vec<String>> {
        let all_keys = self.inner.list_keys()?;
        Ok(all_keys
            .into_iter()
            .filter_map(|k| self.strip_prefix(&k).map(String::from))
            .collect())
    }
}

/// Storage provider interface for FFI.
///
/// This trait is implemented by platform code (Swift/Kotlin) and passed
/// to Paykit for secure storage operations.
pub trait StorageProvider: Send + Sync {
    /// Get the secure storage instance.
    fn storage(&self) -> Arc<dyn SecureStorage>;
}

/// Convenience functions for storing specific types.
pub trait SecureStorageExt: SecureStorage {
    /// Store a string.
    fn store_string(&self, key: &str, value: &str) -> StorageResult<()> {
        self.store(key, value.as_bytes())
    }

    /// Retrieve a string.
    fn retrieve_string(&self, key: &str) -> StorageResult<Option<String>> {
        match self.retrieve(key)? {
            Some(bytes) => {
                let string = String::from_utf8(bytes)
                    .map_err(|e| StorageError::new(StorageErrorCode::Unknown, e.to_string()))?;
                Ok(Some(string))
            }
            None => Ok(None),
        }
    }

    /// Store JSON-serializable data.
    fn store_json<T: serde::Serialize>(&self, key: &str, value: &T) -> StorageResult<()> {
        let json = serde_json::to_vec(value)
            .map_err(|e| StorageError::new(StorageErrorCode::Unknown, e.to_string()))?;
        self.store(key, &json)
    }

    /// Retrieve JSON-deserializable data.
    fn retrieve_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> StorageResult<Option<T>> {
        match self.retrieve(key)? {
            Some(bytes) => {
                let value = serde_json::from_slice(&bytes)
                    .map_err(|e| StorageError::new(StorageErrorCode::Unknown, e.to_string()))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
}

// Implement extension trait for all SecureStorage implementations
impl<T: SecureStorage + ?Sized> SecureStorageExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_storage() {
        let storage = InMemoryStorage::new();

        // Store and retrieve
        storage.store("key1", b"value1").unwrap();
        let retrieved = storage.retrieve("key1").unwrap();
        assert_eq!(retrieved, Some(b"value1".to_vec()));

        // Not found
        let missing = storage.retrieve("nonexistent").unwrap();
        assert!(missing.is_none());

        // Delete
        storage.delete("key1").unwrap();
        let deleted = storage.retrieve("key1").unwrap();
        assert!(deleted.is_none());
    }

    #[test]
    fn test_list_keys() {
        let storage = InMemoryStorage::new();

        storage.store("a", b"1").unwrap();
        storage.store("b", b"2").unwrap();
        storage.store("c", b"3").unwrap();

        let keys = storage.list_keys().unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"a".to_string()));
        assert!(keys.contains(&"b".to_string()));
        assert!(keys.contains(&"c".to_string()));
    }

    #[test]
    fn test_contains() {
        let storage = InMemoryStorage::new();

        storage.store("exists", b"value").unwrap();

        assert!(storage.contains("exists").unwrap());
        assert!(!storage.contains("missing").unwrap());
    }

    #[test]
    fn test_clear() {
        let storage = InMemoryStorage::new();

        storage.store("a", b"1").unwrap();
        storage.store("b", b"2").unwrap();

        storage.clear().unwrap();

        let keys = storage.list_keys().unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_prefixed_storage() {
        let inner = InMemoryStorage::new();
        let prefixed = PrefixedStorage::new(inner, "myapp");

        prefixed.store("key", b"value").unwrap();

        // Should be stored with prefix
        let keys = prefixed.list_keys().unwrap();
        assert_eq!(keys, vec!["key".to_string()]);

        // Retrieve works
        let value = prefixed.retrieve("key").unwrap();
        assert_eq!(value, Some(b"value".to_vec()));
    }

    #[test]
    fn test_string_storage() {
        let storage = InMemoryStorage::new();

        storage.store_string("greeting", "Hello, World!").unwrap();
        let retrieved = storage.retrieve_string("greeting").unwrap();
        assert_eq!(retrieved, Some("Hello, World!".to_string()));
    }

    #[test]
    fn test_json_storage() {
        let storage = InMemoryStorage::new();

        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
        struct Config {
            enabled: bool,
            count: u32,
        }

        let config = Config {
            enabled: true,
            count: 42,
        };

        storage.store_json("config", &config).unwrap();
        let retrieved: Option<Config> = storage.retrieve_json("config").unwrap();
        assert_eq!(retrieved, Some(config));
    }
}
