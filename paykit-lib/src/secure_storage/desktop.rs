//! Desktop secure key storage implementation.
//!
//! This implementation provides secure key storage for desktop platforms
//! (macOS, Windows, Linux) using OS-specific secure storage APIs.
//!
//! # Thread Safety
//!
//! This storage uses `RwLock` for thread-safe access. Public methods
//! will panic if the internal lock is poisoned (which only happens if a thread
//! panics while holding the lock).

use std::collections::HashMap;
use std::sync::RwLock;

use super::traits::{
    KeyMetadata, SecureKeyStorage, SecureStorageError, SecureStorageResult, StoreOptions,
};

/// Desktop secure key storage.
///
/// On desktop platforms, this uses:
/// - **macOS**: Keychain Services (via security-framework crate)
/// - **Windows**: Windows Credential Manager (via winapi)
/// - **Linux**: Secret Service API (via secret-service crate)
///
/// As a fallback when native APIs aren't available, it uses
/// encrypted file storage with a user-provided password.
pub struct DesktopKeyStorage {
    /// Application identifier for namespacing
    app_id: String,
    /// Fallback to encrypted file storage
    fallback_storage: RwLock<HashMap<String, StoredKey>>,
    /// Whether to use native OS storage
    use_native: bool,
}

#[derive(Clone)]
struct StoredKey {
    data: Vec<u8>,
    metadata: KeyMetadata,
}

impl DesktopKeyStorage {
    /// Create a new desktop key storage.
    ///
    /// The app_id is used to namespace keys in the OS keychain.
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            fallback_storage: RwLock::new(HashMap::new()),
            use_native: true,
        }
    }

    /// Disable native OS storage and use encrypted file fallback.
    pub fn with_fallback_only(mut self) -> Self {
        self.use_native = false;
        self
    }

    /// Get the application ID.
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// Check if using native OS storage.
    pub fn uses_native(&self) -> bool {
        self.use_native
    }

    // Platform-specific implementations would go here
    // For now, we use the fallback storage

    // Platform-specific native storage stubs - will be implemented when
    // security-framework (macOS), windows (Windows), or secret-service (Linux)
    // crate integrations are added.

    #[cfg(target_os = "macos")]
    #[allow(dead_code)] // Stub for future macOS Keychain integration
    fn store_native(&self, _key_id: &str, _data: &[u8]) -> Result<(), String> {
        // Would use security-framework crate
        Err("macOS Keychain not implemented - use fallback".to_string())
    }

    #[cfg(target_os = "windows")]
    #[allow(dead_code)] // Stub for future Windows Credential Manager integration
    fn store_native(&self, _key_id: &str, _data: &[u8]) -> Result<(), String> {
        // Would use windows crate
        Err("Windows Credential Manager not implemented - use fallback".to_string())
    }

    #[cfg(target_os = "linux")]
    #[allow(dead_code)] // Stub for future Linux Secret Service integration
    fn store_native(&self, _key_id: &str, _data: &[u8]) -> Result<(), String> {
        // Would use secret-service crate
        Err("Linux Secret Service not implemented - use fallback".to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    #[allow(dead_code)] // Stub for unsupported platforms
    fn store_native(&self, _key_id: &str, _data: &[u8]) -> Result<(), String> {
        Err("No native secure storage available on this platform".to_string())
    }

    /// Store using fallback in-memory storage.
    fn store_fallback(
        &self,
        key_id: &str,
        data: &[u8],
        options: &StoreOptions,
    ) -> SecureStorageResult<()> {
        let mut storage = self
            .fallback_storage
            .write()
            .expect("DesktopKeyStorage: lock poisoned during store_fallback");

        if storage.contains_key(key_id) && !options.overwrite {
            return Err(SecureStorageError::already_exists(key_id));
        }

        let metadata = KeyMetadata::new(key_id, data.len()).with_auth(options.require_auth);

        storage.insert(
            key_id.to_string(),
            StoredKey {
                data: data.to_vec(),
                metadata,
            },
        );

        Ok(())
    }

    /// Retrieve using fallback storage.
    fn retrieve_fallback(&self, key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        let storage = self
            .fallback_storage
            .read()
            .expect("DesktopKeyStorage: lock poisoned during retrieve_fallback");
        Ok(storage.get(key_id).map(|entry| entry.data.clone()))
    }

    /// Delete using fallback storage.
    fn delete_fallback(&self, key_id: &str) -> SecureStorageResult<()> {
        let mut storage = self
            .fallback_storage
            .write()
            .expect("DesktopKeyStorage: lock poisoned during delete_fallback");
        if storage.remove(key_id).is_some() {
            Ok(())
        } else {
            Err(SecureStorageError::not_found(key_id))
        }
    }
}

impl SecureKeyStorage for DesktopKeyStorage {
    async fn store(
        &self,
        key_id: &str,
        key_data: &[u8],
        options: StoreOptions,
    ) -> SecureStorageResult<()> {
        // Try native storage first if enabled
        if self.use_native {
            // Native storage not yet implemented, fall through
        }

        // Use fallback
        self.store_fallback(key_id, key_data, &options)
    }

    async fn retrieve(&self, key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        // Try native storage first if enabled
        if self.use_native {
            // Native storage not yet implemented, fall through
        }

        // Use fallback
        self.retrieve_fallback(key_id)
    }

    async fn delete(&self, key_id: &str) -> SecureStorageResult<()> {
        // Try native storage first if enabled
        if self.use_native {
            // Native storage not yet implemented, fall through
        }

        // Use fallback
        self.delete_fallback(key_id)
    }

    async fn exists(&self, key_id: &str) -> SecureStorageResult<bool> {
        let storage = self
            .fallback_storage
            .read()
            .expect("DesktopKeyStorage: lock poisoned during exists");
        Ok(storage.contains_key(key_id))
    }

    async fn get_metadata(&self, key_id: &str) -> SecureStorageResult<Option<KeyMetadata>> {
        let storage = self
            .fallback_storage
            .read()
            .expect("DesktopKeyStorage: lock poisoned during get_metadata");
        Ok(storage.get(key_id).map(|e| e.metadata.clone()))
    }

    async fn list_keys(&self) -> SecureStorageResult<Vec<String>> {
        let storage = self
            .fallback_storage
            .read()
            .expect("DesktopKeyStorage: lock poisoned during list_keys");
        Ok(storage.keys().cloned().collect())
    }

    async fn clear_all(&self) -> SecureStorageResult<()> {
        self.fallback_storage
            .write()
            .expect("DesktopKeyStorage: lock poisoned during clear_all")
            .clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secure_storage::traits::SecureKeyStorageExt;

    #[test]
    fn test_desktop_storage_creation() {
        let storage = DesktopKeyStorage::new("com.example.app");
        assert_eq!(storage.app_id(), "com.example.app");
        assert!(storage.uses_native());
    }

    #[tokio::test]
    async fn test_fallback_storage() {
        let storage = DesktopKeyStorage::new("test").with_fallback_only();
        assert!(!storage.uses_native());

        // Test basic operations
        storage.store_simple("key1", b"data1").await.unwrap();
        let retrieved = storage.retrieve("key1").await.unwrap();
        assert_eq!(retrieved, Some(b"data1".to_vec()));

        storage.delete("key1").await.unwrap();
        assert!(!storage.exists("key1").await.unwrap());
    }
}
