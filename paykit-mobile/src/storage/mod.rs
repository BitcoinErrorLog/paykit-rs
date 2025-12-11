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

// ============================================================================
// Local Contact Cache
// ============================================================================

/// Local contact cache for offline access.
///
/// Stores contacts locally for quick access without network requests.
/// Can be synced with remote contacts using transport operations.
pub struct LocalContactCache<S: SecureStorage> {
    storage: S,
    cache_key: String,
}

/// Contact entry with metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CachedContact {
    /// The contact's public key (z-base32 encoded).
    pub pubkey: String,
    /// Optional display name.
    pub name: Option<String>,
    /// When the contact was added (unix timestamp).
    pub added_at: i64,
    /// When the contact was last synced (unix timestamp).
    pub last_synced_at: Option<i64>,
}

impl CachedContact {
    /// Create a new cached contact.
    pub fn new(pubkey: impl Into<String>) -> Self {
        Self {
            pubkey: pubkey.into(),
            name: None,
            added_at: current_timestamp(),
            last_synced_at: None,
        }
    }

    /// Create with a display name.
    pub fn with_name(pubkey: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            pubkey: pubkey.into(),
            name: Some(name.into()),
            added_at: current_timestamp(),
            last_synced_at: None,
        }
    }
}

fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl<S: SecureStorage> LocalContactCache<S> {
    /// Create a new local contact cache.
    ///
    /// # Arguments
    ///
    /// * `storage` - The secure storage backend
    /// * `cache_key` - Key to use for storing the contact list
    pub fn new(storage: S, cache_key: impl Into<String>) -> Self {
        Self {
            storage,
            cache_key: cache_key.into(),
        }
    }

    /// Create with default cache key.
    pub fn with_default_key(storage: S) -> Self {
        Self::new(storage, "paykit:contacts:cache")
    }

    /// Get all cached contacts.
    pub fn get_all(&self) -> StorageResult<Vec<CachedContact>> {
        match self.storage.retrieve_json::<Vec<CachedContact>>(&self.cache_key)? {
            Some(contacts) => Ok(contacts),
            None => Ok(Vec::new()),
        }
    }

    /// Get a specific contact by public key.
    pub fn get(&self, pubkey: &str) -> StorageResult<Option<CachedContact>> {
        let contacts = self.get_all()?;
        Ok(contacts.into_iter().find(|c| c.pubkey == pubkey))
    }

    /// Add or update a contact.
    pub fn upsert(&self, contact: CachedContact) -> StorageResult<()> {
        let mut contacts = self.get_all()?;
        
        // Find and update existing, or add new
        if let Some(existing) = contacts.iter_mut().find(|c| c.pubkey == contact.pubkey) {
            *existing = contact;
        } else {
            contacts.push(contact);
        }
        
        self.storage.store_json(&self.cache_key, &contacts)
    }

    /// Add a contact by public key.
    pub fn add(&self, pubkey: impl Into<String>) -> StorageResult<()> {
        self.upsert(CachedContact::new(pubkey))
    }

    /// Add a contact with a display name.
    pub fn add_with_name(&self, pubkey: impl Into<String>, name: impl Into<String>) -> StorageResult<()> {
        self.upsert(CachedContact::with_name(pubkey, name))
    }

    /// Remove a contact by public key.
    pub fn remove(&self, pubkey: &str) -> StorageResult<()> {
        let mut contacts = self.get_all()?;
        contacts.retain(|c| c.pubkey != pubkey);
        self.storage.store_json(&self.cache_key, &contacts)
    }

    /// Check if a contact exists.
    pub fn contains(&self, pubkey: &str) -> StorageResult<bool> {
        Ok(self.get(pubkey)?.is_some())
    }

    /// Get the number of cached contacts.
    pub fn count(&self) -> StorageResult<usize> {
        Ok(self.get_all()?.len())
    }

    /// Clear all cached contacts.
    pub fn clear(&self) -> StorageResult<()> {
        self.storage.delete(&self.cache_key)
    }

    /// Sync with remote contacts.
    ///
    /// Merges remote contacts with local cache, preserving local metadata.
    ///
    /// # Arguments
    ///
    /// * `remote_pubkeys` - Public keys from the remote source
    ///
    /// # Returns
    ///
    /// A sync result indicating what changed.
    pub fn sync(&self, remote_pubkeys: &[String]) -> StorageResult<SyncResult> {
        let mut local = self.get_all()?;
        let now = current_timestamp();
        
        let mut added = 0;
        let removed = 0;
        
        // Add new remote contacts
        for pubkey in remote_pubkeys {
            if !local.iter().any(|c| &c.pubkey == pubkey) {
                local.push(CachedContact {
                    pubkey: pubkey.clone(),
                    name: None,
                    added_at: now,
                    last_synced_at: Some(now),
                });
                added += 1;
            } else {
                // Update sync time for existing
                if let Some(contact) = local.iter_mut().find(|c| &c.pubkey == pubkey) {
                    contact.last_synced_at = Some(now);
                }
            }
        }
        
        // Optionally remove contacts not in remote (commented out to preserve local-only contacts)
        // let remote_set: std::collections::HashSet<_> = remote_pubkeys.iter().collect();
        // local.retain(|c| remote_set.contains(&c.pubkey));
        
        self.storage.store_json(&self.cache_key, &local)?;
        
        Ok(SyncResult {
            total: local.len(),
            added,
            removed,
            synced_at: now,
        })
    }
}

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Total number of contacts after sync.
    pub total: usize,
    /// Number of contacts added.
    pub added: usize,
    /// Number of contacts removed.
    pub removed: usize,
    /// Timestamp of the sync.
    pub synced_at: i64,
}

// ============================================================================
// FFI Error Type for Storage
// ============================================================================

/// FFI-safe storage error type.
#[derive(Debug, Clone, thiserror::Error, uniffi::Error)]
pub enum StorageCacheError {
    #[error("Storage error: {message}")]
    Storage { message: String },
    
    #[error("Lock error: {message}")]
    Lock { message: String },
}

impl From<StorageError> for StorageCacheError {
    fn from(e: StorageError) -> Self {
        Self::Storage { message: e.to_string() }
    }
}

// ============================================================================
// FFI Wrapper for Contact Cache
// ============================================================================

/// FFI-safe wrapper for local contact cache.
#[derive(uniffi::Object)]
pub struct ContactCacheFFI {
    cache: std::sync::RwLock<LocalContactCache<InMemoryStorage>>,
}

#[uniffi::export]
impl ContactCacheFFI {
    /// Create a new contact cache (uses in-memory storage).
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            cache: std::sync::RwLock::new(LocalContactCache::with_default_key(InMemoryStorage::new())),
        })
    }

    /// Get all cached contacts.
    pub fn get_all(&self) -> Result<Vec<CachedContactFFI>, StorageCacheError> {
        let cache = self.cache.read().map_err(|_| StorageCacheError::Lock { message: "Lock poisoned".to_string() })?;
        let contacts = cache.get_all()?;
        Ok(contacts.into_iter().map(CachedContactFFI::from).collect())
    }

    /// Get a specific contact by public key.
    pub fn get(&self, pubkey: String) -> Result<Option<CachedContactFFI>, StorageCacheError> {
        let cache = self.cache.read().map_err(|_| StorageCacheError::Lock { message: "Lock poisoned".to_string() })?;
        let contact = cache.get(&pubkey)?;
        Ok(contact.map(CachedContactFFI::from))
    }

    /// Add a contact by public key.
    pub fn add(&self, pubkey: String) -> Result<(), StorageCacheError> {
        let cache = self.cache.write().map_err(|_| StorageCacheError::Lock { message: "Lock poisoned".to_string() })?;
        cache.add(pubkey)?;
        Ok(())
    }

    /// Add a contact with a display name.
    pub fn add_with_name(&self, pubkey: String, name: String) -> Result<(), StorageCacheError> {
        let cache = self.cache.write().map_err(|_| StorageCacheError::Lock { message: "Lock poisoned".to_string() })?;
        cache.add_with_name(pubkey, name)?;
        Ok(())
    }

    /// Remove a contact by public key.
    pub fn remove(&self, pubkey: String) -> Result<(), StorageCacheError> {
        let cache = self.cache.write().map_err(|_| StorageCacheError::Lock { message: "Lock poisoned".to_string() })?;
        cache.remove(&pubkey)?;
        Ok(())
    }

    /// Check if a contact exists.
    pub fn contains(&self, pubkey: String) -> Result<bool, StorageCacheError> {
        let cache = self.cache.read().map_err(|_| StorageCacheError::Lock { message: "Lock poisoned".to_string() })?;
        Ok(cache.contains(&pubkey)?)
    }

    /// Get the number of cached contacts.
    pub fn count(&self) -> Result<u32, StorageCacheError> {
        let cache = self.cache.read().map_err(|_| StorageCacheError::Lock { message: "Lock poisoned".to_string() })?;
        Ok(cache.count()? as u32)
    }

    /// Clear all cached contacts.
    pub fn clear(&self) -> Result<(), StorageCacheError> {
        let cache = self.cache.write().map_err(|_| StorageCacheError::Lock { message: "Lock poisoned".to_string() })?;
        cache.clear()?;
        Ok(())
    }

    /// Sync with remote contacts.
    pub fn sync(&self, remote_pubkeys: Vec<String>) -> Result<SyncResultFFI, StorageCacheError> {
        let cache = self.cache.write().map_err(|_| StorageCacheError::Lock { message: "Lock poisoned".to_string() })?;
        let result = cache.sync(&remote_pubkeys)?;
        Ok(SyncResultFFI::from(result))
    }
}

/// FFI-safe cached contact.
#[derive(Clone, Debug, uniffi::Record)]
pub struct CachedContactFFI {
    pub pubkey: String,
    pub name: Option<String>,
    pub added_at: i64,
    pub last_synced_at: Option<i64>,
}

impl From<CachedContact> for CachedContactFFI {
    fn from(c: CachedContact) -> Self {
        Self {
            pubkey: c.pubkey,
            name: c.name,
            added_at: c.added_at,
            last_synced_at: c.last_synced_at,
        }
    }
}

/// FFI-safe sync result.
#[derive(Clone, Debug, uniffi::Record)]
pub struct SyncResultFFI {
    pub total: u32,
    pub added: u32,
    pub removed: u32,
    pub synced_at: i64,
}

impl From<SyncResult> for SyncResultFFI {
    fn from(r: SyncResult) -> Self {
        Self {
            total: r.total as u32,
            added: r.added as u32,
            removed: r.removed as u32,
            synced_at: r.synced_at,
        }
    }
}

/// Create a new contact cache.
#[uniffi::export]
pub fn create_contact_cache() -> Arc<ContactCacheFFI> {
    ContactCacheFFI::new()
}

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

    // ========================================================================
    // Contact Cache Tests
    // ========================================================================

    #[test]
    fn test_contact_cache_add_and_get() {
        let storage = InMemoryStorage::new();
        let cache = LocalContactCache::with_default_key(storage);

        cache.add("pubkey1").unwrap();
        cache.add_with_name("pubkey2", "Alice").unwrap();

        let contact1 = cache.get("pubkey1").unwrap();
        assert!(contact1.is_some());
        assert_eq!(contact1.unwrap().pubkey, "pubkey1");

        let contact2 = cache.get("pubkey2").unwrap();
        assert!(contact2.is_some());
        assert_eq!(contact2.as_ref().unwrap().name, Some("Alice".to_string()));
    }

    #[test]
    fn test_contact_cache_get_all() {
        let storage = InMemoryStorage::new();
        let cache = LocalContactCache::with_default_key(storage);

        cache.add("pubkey1").unwrap();
        cache.add("pubkey2").unwrap();
        cache.add("pubkey3").unwrap();

        let all = cache.get_all().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_contact_cache_remove() {
        let storage = InMemoryStorage::new();
        let cache = LocalContactCache::with_default_key(storage);

        cache.add("pubkey1").unwrap();
        cache.add("pubkey2").unwrap();

        cache.remove("pubkey1").unwrap();

        assert!(!cache.contains("pubkey1").unwrap());
        assert!(cache.contains("pubkey2").unwrap());
        assert_eq!(cache.count().unwrap(), 1);
    }

    #[test]
    fn test_contact_cache_clear() {
        let storage = InMemoryStorage::new();
        let cache = LocalContactCache::with_default_key(storage);

        cache.add("pubkey1").unwrap();
        cache.add("pubkey2").unwrap();

        cache.clear().unwrap();

        assert_eq!(cache.count().unwrap(), 0);
    }

    #[test]
    fn test_contact_cache_sync() {
        let storage = InMemoryStorage::new();
        let cache = LocalContactCache::with_default_key(storage);

        // Add local contact
        cache.add_with_name("local_only", "Local").unwrap();

        // Sync with remote
        let remote = vec![
            "remote1".to_string(),
            "remote2".to_string(),
        ];
        let result = cache.sync(&remote).unwrap();

        // Should add 2 remote contacts
        assert_eq!(result.added, 2);
        
        // Local contact should be preserved
        assert!(cache.contains("local_only").unwrap());
        assert!(cache.contains("remote1").unwrap());
        assert!(cache.contains("remote2").unwrap());
        assert_eq!(cache.count().unwrap(), 3);
    }

    #[test]
    fn test_contact_cache_upsert() {
        let storage = InMemoryStorage::new();
        let cache = LocalContactCache::with_default_key(storage);

        // Add contact
        cache.add("pubkey1").unwrap();
        assert!(cache.get("pubkey1").unwrap().unwrap().name.is_none());

        // Update with name
        cache.add_with_name("pubkey1", "Updated Name").unwrap();
        assert_eq!(
            cache.get("pubkey1").unwrap().unwrap().name,
            Some("Updated Name".to_string())
        );

        // Should still be only one contact
        assert_eq!(cache.count().unwrap(), 1);
    }
}
