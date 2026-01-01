//! Web SubtleCrypto implementation of secure key storage.
//!
//! This implementation uses the Web Crypto API (SubtleCrypto)
//! combined with IndexedDB for secure key storage in browsers.
//!
//! ## Status: Work in Progress
//!
//! This is a stub implementation. Full Web Crypto API support requires
//! significant work with the web-sys bindings.
//!
//! ## Security Model (planned)
//!
//! - Random master key generated on first use
//! - Master key encrypted with password-derived key (PBKDF2)
//! - All data encrypted with master key using AES-256-GCM
//! - Encrypted master key stored in IndexedDB
//! - User must provide password to unlock storage

#[cfg(target_arch = "wasm32")]
use super::traits::{
    KeyMetadata, SecureKeyStorage, SecureStorageError, SecureStorageResult, StoreOptions,
};

#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

/// Web Crypto-backed secure key storage.
///
/// Note: This is currently a stub implementation using in-memory storage.
/// A full implementation would use SubtleCrypto for encryption and IndexedDB
/// for persistence.
#[cfg(target_arch = "wasm32")]
pub struct WebCryptoStorage {
    /// Database name for IndexedDB (reserved for future use)
    #[allow(dead_code)]
    db_name: String,
    /// In-memory storage for keys (temporary - would use IndexedDB in full impl)
    keys: RefCell<HashMap<String, Vec<u8>>>,
    /// In-memory metadata storage
    metadata: RefCell<HashMap<String, KeyMetadata>>,
}

#[cfg(target_arch = "wasm32")]
impl WebCryptoStorage {
    /// Create a new Web Crypto storage.
    pub fn new(db_name: impl Into<String>) -> Self {
        Self {
            db_name: db_name.into(),
            keys: RefCell::new(HashMap::new()),
            metadata: RefCell::new(HashMap::new()),
        }
    }

    /// Set a custom object store name.
    pub fn with_store_name(self, _name: impl Into<String>) -> Self {
        // Store name is ignored in stub implementation
        self
    }

    /// Set password for unlocking storage.
    /// Note: Password is ignored in stub implementation.
    pub fn set_password(&self, _password: Vec<u8>) {
        // Password is ignored in stub implementation
    }

    /// Clear the password from memory.
    pub fn clear_password(&self) {
        // No-op in stub implementation
    }

    /// Check if storage is initialized.
    pub fn is_initialized(&self) -> bool {
        true
    }
}

#[cfg(target_arch = "wasm32")]
impl SecureKeyStorage for WebCryptoStorage {
    fn store(
        &self,
        key_id: &str,
        key_data: &[u8],
        options: StoreOptions,
    ) -> impl std::future::Future<Output = SecureStorageResult<()>> {
        async move {
            let mut keys = self.keys.borrow_mut();
            
            if keys.contains_key(key_id) && !options.overwrite {
                return Err(SecureStorageError::already_exists(key_id));
            }
            
            keys.insert(key_id.to_string(), key_data.to_vec());
            
            // Store metadata
            let mut metadata = self.metadata.borrow_mut();
            let meta = KeyMetadata::new(key_id, key_data.len())
                .with_auth(options.require_auth);
            metadata.insert(key_id.to_string(), meta);
            
            Ok(())
        }
    }

    fn retrieve(
        &self,
        key_id: &str,
    ) -> impl std::future::Future<Output = SecureStorageResult<Option<Vec<u8>>>> {
        async move {
            let keys = self.keys.borrow();
            Ok(keys.get(key_id).cloned())
        }
    }

    fn delete(&self, key_id: &str) -> impl std::future::Future<Output = SecureStorageResult<()>> {
        async move {
            let mut keys = self.keys.borrow_mut();
            keys.remove(key_id)
                .ok_or_else(|| SecureStorageError::not_found(key_id))?;
            
            let mut metadata = self.metadata.borrow_mut();
            metadata.remove(key_id);
            
            Ok(())
        }
    }

    fn exists(&self, key_id: &str) -> impl std::future::Future<Output = SecureStorageResult<bool>> {
        async move {
            let keys = self.keys.borrow();
            Ok(keys.contains_key(key_id))
        }
    }

    fn get_metadata(
        &self,
        key_id: &str,
    ) -> impl std::future::Future<Output = SecureStorageResult<Option<KeyMetadata>>> {
        async move {
            let metadata = self.metadata.borrow();
            Ok(metadata.get(key_id).cloned())
        }
    }

    fn list_keys(&self) -> impl std::future::Future<Output = SecureStorageResult<Vec<String>>> {
        async move {
            let keys = self.keys.borrow();
            Ok(keys.keys().cloned().collect())
        }
    }

    fn clear_all(&self) -> impl std::future::Future<Output = SecureStorageResult<()>> {
        async move {
            let mut keys = self.keys.borrow_mut();
            keys.clear();
            
            let mut metadata = self.metadata.borrow_mut();
            metadata.clear();
            
            Ok(())
        }
    }
}
