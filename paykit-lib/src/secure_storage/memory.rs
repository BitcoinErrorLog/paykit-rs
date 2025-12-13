//! In-memory secure key storage implementation.
//!
//! This implementation is for testing and development only.
//! In production, use platform-specific implementations.
//!
//! # Thread Safety
//!
//! This storage uses `RwLock` for thread-safe access. Lock poisoning
//! is handled gracefully by returning an error rather than panicking.

use std::collections::HashMap;
use std::sync::RwLock;

use super::traits::{
    KeyMetadata, SecureKeyStorage, SecureStorageError, SecureStorageErrorCode, SecureStorageResult,
    StoreOptions,
};

/// In-memory key storage entry.
#[derive(Clone)]
struct StoredKey {
    data: Vec<u8>,
    metadata: KeyMetadata,
}

/// In-memory implementation of secure key storage.
///
/// **Warning**: This is for testing only. Keys are not encrypted
/// and will be lost when the process exits.
pub struct InMemoryKeyStorage {
    keys: RwLock<HashMap<String, StoredKey>>,
}

/// Helper function to handle lock poisoning gracefully.
fn lock_error(context: &str) -> SecureStorageError {
    SecureStorageError::new(
        SecureStorageErrorCode::Internal,
        format!("InMemoryKeyStorage: lock poisoned during {}", context),
    )
}

impl InMemoryKeyStorage {
    /// Create a new in-memory key storage.
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(HashMap::new()),
        }
    }

    /// Get the number of stored keys.
    ///
    /// Returns 0 if the lock is poisoned.
    pub fn len(&self) -> usize {
        self.keys.read().map(|k| k.len()).unwrap_or(0)
    }

    /// Check if storage is empty.
    ///
    /// Returns true if the lock is poisoned.
    pub fn is_empty(&self) -> bool {
        self.keys.read().map(|k| k.is_empty()).unwrap_or(true)
    }
}

impl Default for InMemoryKeyStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureKeyStorage for InMemoryKeyStorage {
    async fn store(
        &self,
        key_id: &str,
        key_data: &[u8],
        options: StoreOptions,
    ) -> SecureStorageResult<()> {
        let mut keys = self.keys.write().map_err(|_| lock_error("store"))?;

        if keys.contains_key(key_id) && !options.overwrite {
            return Err(SecureStorageError::already_exists(key_id));
        }

        let metadata = KeyMetadata::new(key_id, key_data.len()).with_auth(options.require_auth);

        keys.insert(
            key_id.to_string(),
            StoredKey {
                data: key_data.to_vec(),
                metadata,
            },
        );

        Ok(())
    }

    async fn retrieve(&self, key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        let mut keys = self.keys.write().map_err(|_| lock_error("retrieve"))?;

        if let Some(entry) = keys.get_mut(key_id) {
            // Update last accessed time
            entry.metadata.last_accessed = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
            );
            Ok(Some(entry.data.clone()))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, key_id: &str) -> SecureStorageResult<()> {
        let mut keys = self.keys.write().map_err(|_| lock_error("delete"))?;

        if keys.remove(key_id).is_some() {
            Ok(())
        } else {
            Err(SecureStorageError::not_found(key_id))
        }
    }

    async fn exists(&self, key_id: &str) -> SecureStorageResult<bool> {
        let keys = self.keys.read().map_err(|_| lock_error("exists"))?;
        Ok(keys.contains_key(key_id))
    }

    async fn get_metadata(&self, key_id: &str) -> SecureStorageResult<Option<KeyMetadata>> {
        let keys = self.keys.read().map_err(|_| lock_error("get_metadata"))?;
        Ok(keys.get(key_id).map(|e| e.metadata.clone()))
    }

    async fn list_keys(&self) -> SecureStorageResult<Vec<String>> {
        let keys = self.keys.read().map_err(|_| lock_error("list_keys"))?;
        Ok(keys.keys().cloned().collect())
    }

    async fn clear_all(&self) -> SecureStorageResult<()> {
        let mut keys = self.keys.write().map_err(|_| lock_error("clear_all"))?;
        keys.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secure_storage::traits::SecureKeyStorageExt;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let storage = InMemoryKeyStorage::new();
        let key_data = b"secret-key-data";

        storage.store_simple("test-key", key_data).await.unwrap();

        let retrieved = storage.retrieve("test-key").await.unwrap();
        assert_eq!(retrieved, Some(key_data.to_vec()));
    }

    #[tokio::test]
    async fn test_retrieve_missing() {
        let storage = InMemoryKeyStorage::new();

        let result = storage.retrieve("nonexistent").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_retrieve_required_missing() {
        let storage = InMemoryKeyStorage::new();

        let result = storage.retrieve_required("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().is_not_found());
    }

    #[tokio::test]
    async fn test_no_overwrite() {
        let storage = InMemoryKeyStorage::new();

        storage.store_simple("test-key", b"first").await.unwrap();

        let result = storage.store_simple("test-key", b"second").await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code,
            SecureStorageErrorCode::AlreadyExists
        );
    }

    #[tokio::test]
    async fn test_upsert() {
        let storage = InMemoryKeyStorage::new();

        storage.upsert("test-key", b"first").await.unwrap();
        storage.upsert("test-key", b"second").await.unwrap();

        let retrieved = storage.retrieve("test-key").await.unwrap();
        assert_eq!(retrieved, Some(b"second".to_vec()));
    }

    #[tokio::test]
    async fn test_delete() {
        let storage = InMemoryKeyStorage::new();

        storage.store_simple("test-key", b"data").await.unwrap();
        assert!(storage.exists("test-key").await.unwrap());

        storage.delete("test-key").await.unwrap();
        assert!(!storage.exists("test-key").await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_missing() {
        let storage = InMemoryKeyStorage::new();

        let result = storage.delete("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_if_exists() {
        let storage = InMemoryKeyStorage::new();

        // Should not error on missing key
        storage.delete_if_exists("nonexistent").await.unwrap();

        // Should delete existing key
        storage.store_simple("test-key", b"data").await.unwrap();
        storage.delete_if_exists("test-key").await.unwrap();
        assert!(!storage.exists("test-key").await.unwrap());
    }

    #[tokio::test]
    async fn test_list_keys() {
        let storage = InMemoryKeyStorage::new();

        storage.store_simple("key1", b"data1").await.unwrap();
        storage.store_simple("key2", b"data2").await.unwrap();
        storage.store_simple("key3", b"data3").await.unwrap();

        let mut keys = storage.list_keys().await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["key1", "key2", "key3"]);
    }

    #[tokio::test]
    async fn test_clear_all() {
        let storage = InMemoryKeyStorage::new();

        storage.store_simple("key1", b"data1").await.unwrap();
        storage.store_simple("key2", b"data2").await.unwrap();

        storage.clear_all().await.unwrap();

        assert!(storage.is_empty());
    }

    #[tokio::test]
    async fn test_metadata() {
        let storage = InMemoryKeyStorage::new();

        storage
            .store(
                "test-key",
                b"data",
                StoreOptions::new().require_auth().with_tag("wallet"),
            )
            .await
            .unwrap();

        let metadata = storage.get_metadata("test-key").await.unwrap().unwrap();
        assert_eq!(metadata.key_id, "test-key");
        assert_eq!(metadata.size_bytes, 4);
        assert!(metadata.requires_auth);
    }

    #[tokio::test]
    async fn test_last_accessed_updated() {
        let storage = InMemoryKeyStorage::new();

        storage.store_simple("test-key", b"data").await.unwrap();

        // First access
        storage.retrieve("test-key").await.unwrap();
        let meta1 = storage.get_metadata("test-key").await.unwrap().unwrap();
        assert!(meta1.last_accessed.is_some());

        // Second access after brief delay
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        storage.retrieve("test-key").await.unwrap();
        let meta2 = storage.get_metadata("test-key").await.unwrap().unwrap();

        // last_accessed should be >= first access
        assert!(meta2.last_accessed.unwrap() >= meta1.last_accessed.unwrap());
    }
}
