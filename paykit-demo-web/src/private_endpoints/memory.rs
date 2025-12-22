#![cfg(not(target_arch = "wasm32"))]

use paykit_lib::private_endpoints::{
    InMemoryStore, PrivateEndpoint, PrivateEndpointStore, StorageResult,
};
use paykit_lib::{MethodId, PublicKey};

pub struct WasmPrivateEndpointStorage {
    inner: InMemoryStore,
}

impl WasmPrivateEndpointStorage {
    pub fn new() -> Self {
        Self {
            inner: InMemoryStore::new(),
        }
    }
}

impl Default for WasmPrivateEndpointStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PrivateEndpointStore for WasmPrivateEndpointStorage {
    async fn save(&self, endpoint: PrivateEndpoint) -> StorageResult<()> {
        self.inner.save(endpoint).await
    }

    async fn get(
        &self,
        peer: &PublicKey,
        method_id: &MethodId,
    ) -> StorageResult<Option<PrivateEndpoint>> {
        self.inner.get(peer, method_id).await
    }

    async fn list_for_peer(&self, peer: &PublicKey) -> StorageResult<Vec<PrivateEndpoint>> {
        self.inner.list_for_peer(peer).await
    }

    async fn list_peers(&self) -> StorageResult<Vec<PublicKey>> {
        self.inner.list_peers().await
    }

    async fn remove(&self, peer: &PublicKey, method_id: &MethodId) -> StorageResult<()> {
        self.inner.remove(peer, method_id).await
    }

    async fn remove_all_for_peer(&self, peer: &PublicKey) -> StorageResult<()> {
        self.inner.remove_all_for_peer(peer).await
    }

    async fn cleanup_expired(&self) -> StorageResult<usize> {
        self.inner.cleanup_expired().await
    }

    async fn update(&self, endpoint: PrivateEndpoint) -> StorageResult<()> {
        self.inner.update(endpoint).await
    }

    async fn count(&self) -> StorageResult<usize> {
        self.inner.count().await
    }
}


