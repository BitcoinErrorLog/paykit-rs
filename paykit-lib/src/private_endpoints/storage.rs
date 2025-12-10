//! Private endpoint storage implementations.

use super::types::PrivateEndpoint;
use crate::{MethodId, PublicKey};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

/// Error type for storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Storage I/O error: {0}")]
    Io(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Not found")]
    NotFound,
    #[error("Storage error: {0}")]
    Other(String),
}

impl From<serde_json::Error> for StorageError {
    fn from(e: serde_json::Error) -> Self {
        StorageError::Serialization(e.to_string())
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::Io(e.to_string())
    }
}

/// Result type for storage operations.
pub type StorageResult<T> = std::result::Result<T, StorageError>;

/// Trait for persisting private endpoints.
///
/// Implementations should ensure thread-safety and handle concurrent access.
#[async_trait]
pub trait PrivateEndpointStore: Send + Sync {
    /// Save a private endpoint.
    async fn save(&self, endpoint: PrivateEndpoint) -> StorageResult<()>;

    /// Get a private endpoint for a peer and method.
    async fn get(
        &self,
        peer: &PublicKey,
        method_id: &MethodId,
    ) -> StorageResult<Option<PrivateEndpoint>>;

    /// List all private endpoints for a peer.
    async fn list_for_peer(&self, peer: &PublicKey) -> StorageResult<Vec<PrivateEndpoint>>;

    /// List all peers that have private endpoints.
    async fn list_peers(&self) -> StorageResult<Vec<PublicKey>>;

    /// Remove a specific private endpoint.
    async fn remove(&self, peer: &PublicKey, method_id: &MethodId) -> StorageResult<()>;

    /// Remove all private endpoints for a peer.
    async fn remove_all_for_peer(&self, peer: &PublicKey) -> StorageResult<()>;

    /// Clean up all expired endpoints.
    ///
    /// Returns the number of endpoints removed.
    async fn cleanup_expired(&self) -> StorageResult<usize>;

    /// Update a private endpoint (e.g., to record usage).
    async fn update(&self, endpoint: PrivateEndpoint) -> StorageResult<()>;

    /// Count total number of endpoints.
    async fn count(&self) -> StorageResult<usize>;
}

/// In-memory storage for private endpoints.
///
/// This is useful for testing and short-lived processes.
/// Data is not persisted across restarts.
pub struct InMemoryStore {
    /// Map from (peer_string, method_id) to endpoint.
    endpoints: RwLock<HashMap<String, PrivateEndpoint>>,
}

impl InMemoryStore {
    /// Create a new in-memory store.
    pub fn new() -> Self {
        Self {
            endpoints: RwLock::new(HashMap::new()),
        }
    }

    /// Create a key for the internal map.
    fn make_key(peer: &PublicKey, method_id: &MethodId) -> String {
        format!("{}:{}", peer_to_string(peer), method_id.0)
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PrivateEndpointStore for InMemoryStore {
    async fn save(&self, endpoint: PrivateEndpoint) -> StorageResult<()> {
        let key = Self::make_key(&endpoint.peer, &endpoint.method_id);
        let mut endpoints = self
            .endpoints
            .write()
            .map_err(|e| StorageError::Other(e.to_string()))?;
        endpoints.insert(key, endpoint);
        Ok(())
    }

    async fn get(
        &self,
        peer: &PublicKey,
        method_id: &MethodId,
    ) -> StorageResult<Option<PrivateEndpoint>> {
        let key = Self::make_key(peer, method_id);
        let endpoints = self
            .endpoints
            .read()
            .map_err(|e| StorageError::Other(e.to_string()))?;
        Ok(endpoints.get(&key).cloned())
    }

    async fn list_for_peer(&self, peer: &PublicKey) -> StorageResult<Vec<PrivateEndpoint>> {
        let peer_str = peer_to_string(peer);
        let endpoints = self
            .endpoints
            .read()
            .map_err(|e| StorageError::Other(e.to_string()))?;

        Ok(endpoints
            .iter()
            .filter(|(key, _)| key.starts_with(&format!("{}:", peer_str)))
            .map(|(_, v)| v.clone())
            .collect())
    }

    async fn list_peers(&self) -> StorageResult<Vec<PublicKey>> {
        let endpoints = self
            .endpoints
            .read()
            .map_err(|e| StorageError::Other(e.to_string()))?;

        // Extract unique peer keys
        let peers: Vec<PublicKey> = endpoints
            .values()
            .map(|e| e.peer.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Ok(peers)
    }

    async fn remove(&self, peer: &PublicKey, method_id: &MethodId) -> StorageResult<()> {
        let key = Self::make_key(peer, method_id);
        let mut endpoints = self
            .endpoints
            .write()
            .map_err(|e| StorageError::Other(e.to_string()))?;
        endpoints.remove(&key);
        Ok(())
    }

    async fn remove_all_for_peer(&self, peer: &PublicKey) -> StorageResult<()> {
        let peer_str = peer_to_string(peer);
        let mut endpoints = self
            .endpoints
            .write()
            .map_err(|e| StorageError::Other(e.to_string()))?;

        endpoints.retain(|key, _| !key.starts_with(&format!("{}:", peer_str)));
        Ok(())
    }

    async fn cleanup_expired(&self) -> StorageResult<usize> {
        let mut endpoints = self
            .endpoints
            .write()
            .map_err(|e| StorageError::Other(e.to_string()))?;

        let before = endpoints.len();
        endpoints.retain(|_, v| !v.is_expired());
        let after = endpoints.len();

        Ok(before - after)
    }

    async fn update(&self, endpoint: PrivateEndpoint) -> StorageResult<()> {
        self.save(endpoint).await
    }

    async fn count(&self) -> StorageResult<usize> {
        let endpoints = self
            .endpoints
            .read()
            .map_err(|e| StorageError::Other(e.to_string()))?;
        Ok(endpoints.len())
    }
}

/// File-based encrypted storage for private endpoints.
///
/// This provides persistence across restarts with optional encryption.
/// Endpoints are stored as JSON files in a directory structure.
#[cfg(feature = "file-storage")]
pub struct FileStore {
    /// Base directory for storage.
    base_path: std::path::PathBuf,
    /// Encryption key (optional).
    encryption_key: Option<[u8; 32]>,
}

#[cfg(feature = "file-storage")]
impl FileStore {
    /// Create a new file store at the given path.
    pub fn new<P: AsRef<std::path::Path>>(base_path: P) -> std::io::Result<Self> {
        let path = base_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&path)?;
        Ok(Self {
            base_path: path,
            encryption_key: None,
        })
    }

    /// Create a new encrypted file store.
    pub fn new_encrypted<P: AsRef<std::path::Path>>(
        base_path: P,
        key: [u8; 32],
    ) -> std::io::Result<Self> {
        let path = base_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&path)?;
        Ok(Self {
            base_path: path,
            encryption_key: Some(key),
        })
    }

    /// Get the file path for an endpoint.
    fn endpoint_path(&self, peer: &PublicKey, method_id: &MethodId) -> std::path::PathBuf {
        let peer_str = peer_to_string(peer);
        // Use a hash of the peer key for the directory name to avoid filesystem issues
        let peer_hash = format!("{:x}", md5::compute(peer_str.as_bytes()));
        self.base_path
            .join(&peer_hash)
            .join(format!("{}.json", method_id.0))
    }

    /// Get the directory path for a peer.
    fn peer_path(&self, peer: &PublicKey) -> std::path::PathBuf {
        let peer_str = peer_to_string(peer);
        let peer_hash = format!("{:x}", md5::compute(peer_str.as_bytes()));
        self.base_path.join(&peer_hash)
    }

    /// Encrypt data if encryption is enabled.
    fn maybe_encrypt(&self, data: &[u8]) -> Vec<u8> {
        if let Some(_key) = &self.encryption_key {
            // In a real implementation, use ChaCha20-Poly1305 or similar
            // For now, just return the data (placeholder)
            // TODO: Implement actual encryption
            data.to_vec()
        } else {
            data.to_vec()
        }
    }

    /// Decrypt data if encryption is enabled.
    fn maybe_decrypt(&self, data: &[u8]) -> Vec<u8> {
        if let Some(_key) = &self.encryption_key {
            // TODO: Implement actual decryption
            data.to_vec()
        } else {
            data.to_vec()
        }
    }
}

#[cfg(feature = "file-storage")]
#[async_trait]
impl PrivateEndpointStore for FileStore {
    async fn save(&self, endpoint: PrivateEndpoint) -> StorageResult<()> {
        let path = self.endpoint_path(&endpoint.peer, &endpoint.method_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&endpoint)?;
        let data = self.maybe_encrypt(json.as_bytes());
        std::fs::write(&path, data)?;

        Ok(())
    }

    async fn get(
        &self,
        peer: &PublicKey,
        method_id: &MethodId,
    ) -> StorageResult<Option<PrivateEndpoint>> {
        let path = self.endpoint_path(peer, method_id);
        if !path.exists() {
            return Ok(None);
        }

        let data = std::fs::read(&path)?;
        let decrypted = self.maybe_decrypt(&data);
        let json = String::from_utf8(decrypted)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let endpoint: PrivateEndpoint = serde_json::from_str(&json)?;

        Ok(Some(endpoint))
    }

    async fn list_for_peer(&self, peer: &PublicKey) -> StorageResult<Vec<PrivateEndpoint>> {
        let peer_dir = self.peer_path(peer);
        if !peer_dir.exists() {
            return Ok(Vec::new());
        }

        let mut endpoints = Vec::new();
        for entry in std::fs::read_dir(&peer_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let data = std::fs::read(&path)?;
                let decrypted = self.maybe_decrypt(&data);
                let json = String::from_utf8(decrypted)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                if let Ok(endpoint) = serde_json::from_str::<PrivateEndpoint>(&json) {
                    endpoints.push(endpoint);
                }
            }
        }

        Ok(endpoints)
    }

    async fn list_peers(&self) -> StorageResult<Vec<PublicKey>> {
        let mut peers = Vec::new();

        if !self.base_path.exists() {
            return Ok(peers);
        }

        for entry in std::fs::read_dir(&self.base_path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                // Read any endpoint file to get the peer key
                let dir_path = entry.path();
                for file_entry in std::fs::read_dir(&dir_path)? {
                    let file_entry = file_entry?;
                    let path = file_entry.path();
                    if path.extension().map(|e| e == "json").unwrap_or(false) {
                        let data = std::fs::read(&path)?;
                        let decrypted = self.maybe_decrypt(&data);
                        let json = String::from_utf8(decrypted)
                            .map_err(|e| StorageError::Serialization(e.to_string()))?;
                        if let Ok(endpoint) = serde_json::from_str::<PrivateEndpoint>(&json) {
                            peers.push(endpoint.peer);
                            break; // Only need one per peer directory
                        }
                    }
                }
            }
        }

        Ok(peers)
    }

    async fn remove(&self, peer: &PublicKey, method_id: &MethodId) -> StorageResult<()> {
        let path = self.endpoint_path(peer, method_id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    async fn remove_all_for_peer(&self, peer: &PublicKey) -> StorageResult<()> {
        let peer_dir = self.peer_path(peer);
        if peer_dir.exists() {
            std::fs::remove_dir_all(&peer_dir)?;
        }
        Ok(())
    }

    async fn cleanup_expired(&self) -> StorageResult<usize> {
        let peers = self.list_peers().await?;
        let mut count = 0;

        for peer in peers {
            let endpoints = self.list_for_peer(&peer).await?;
            for endpoint in endpoints {
                if endpoint.is_expired() {
                    self.remove(&peer, &endpoint.method_id).await?;
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    async fn update(&self, endpoint: PrivateEndpoint) -> StorageResult<()> {
        self.save(endpoint).await
    }

    async fn count(&self) -> StorageResult<usize> {
        let peers = self.list_peers().await?;
        let mut total = 0;
        for peer in peers {
            total += self.list_for_peer(&peer).await?.len();
        }
        Ok(total)
    }
}

/// Helper function to convert PublicKey to string.
#[cfg(feature = "pubky")]
fn peer_to_string(peer: &PublicKey) -> String {
    peer.to_string()
}

#[cfg(not(feature = "pubky"))]
fn peer_to_string(peer: &PublicKey) -> String {
    peer.0.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EndpointData;

    fn test_pubkey() -> PublicKey {
        #[cfg(feature = "pubky")]
        {
            use pubky::Keypair;
            let keypair = Keypair::random();
            keypair.public_key()
        }
        #[cfg(not(feature = "pubky"))]
        {
            PublicKey(format!("test_key_{}", rand::random::<u32>()))
        }
    }

    #[tokio::test]
    async fn test_in_memory_store_basic_operations() {
        let store = InMemoryStore::new();

        let peer = test_pubkey();
        let method = MethodId("lightning".to_string());
        let endpoint = EndpointData("lnbc...".to_string());

        let private = PrivateEndpoint::new(peer.clone(), method.clone(), endpoint.clone(), None);

        // Save
        store.save(private.clone()).await.unwrap();

        // Get
        let retrieved = store.get(&peer, &method).await.unwrap().unwrap();
        assert_eq!(retrieved.endpoint, endpoint);

        // Count
        assert_eq!(store.count().await.unwrap(), 1);

        // Remove
        store.remove(&peer, &method).await.unwrap();
        assert!(store.get(&peer, &method).await.unwrap().is_none());
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_in_memory_store_list_operations() {
        let store = InMemoryStore::new();

        let peer = test_pubkey();
        let method1 = MethodId("lightning".to_string());
        let method2 = MethodId("onchain".to_string());
        let endpoint = EndpointData("data".to_string());

        store
            .save(PrivateEndpoint::new(
                peer.clone(),
                method1.clone(),
                endpoint.clone(),
                None,
            ))
            .await
            .unwrap();
        store
            .save(PrivateEndpoint::new(
                peer.clone(),
                method2.clone(),
                endpoint.clone(),
                None,
            ))
            .await
            .unwrap();

        // List for peer
        let list = store.list_for_peer(&peer).await.unwrap();
        assert_eq!(list.len(), 2);

        // List peers
        let peers = store.list_peers().await.unwrap();
        assert!(peers.contains(&peer));

        // Remove all for peer
        store.remove_all_for_peer(&peer).await.unwrap();
        assert!(store.list_for_peer(&peer).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_store_cleanup_expired() {
        let store = InMemoryStore::new();

        let peer = test_pubkey();
        let method1 = MethodId("lightning".to_string());
        let method2 = MethodId("onchain".to_string());
        let endpoint = EndpointData("data".to_string());

        // One expired, one valid
        let expired_at = chrono::Utc::now().timestamp() - 3600;
        let valid_until = chrono::Utc::now().timestamp() + 3600;

        store
            .save(PrivateEndpoint::new(
                peer.clone(),
                method1.clone(),
                endpoint.clone(),
                Some(expired_at),
            ))
            .await
            .unwrap();
        store
            .save(PrivateEndpoint::new(
                peer.clone(),
                method2.clone(),
                endpoint.clone(),
                Some(valid_until),
            ))
            .await
            .unwrap();

        assert_eq!(store.count().await.unwrap(), 2);

        let cleaned = store.cleanup_expired().await.unwrap();
        assert_eq!(cleaned, 1);
        assert_eq!(store.count().await.unwrap(), 1);

        // The valid one should remain
        assert!(store.get(&peer, &method2).await.unwrap().is_some());
        assert!(store.get(&peer, &method1).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_in_memory_store_update() {
        let store = InMemoryStore::new();

        let peer = test_pubkey();
        let method = MethodId("lightning".to_string());
        let endpoint = EndpointData("lnbc...".to_string());

        let mut private =
            PrivateEndpoint::new(peer.clone(), method.clone(), endpoint.clone(), None);

        store.save(private.clone()).await.unwrap();

        // Update with usage
        private.record_use();
        store.update(private.clone()).await.unwrap();

        let retrieved = store.get(&peer, &method).await.unwrap().unwrap();
        assert_eq!(retrieved.use_count, 1);
    }
}
