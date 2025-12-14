//! Private Endpoint Management
//!
//! This module provides management of private payment endpoints that are
//! exchanged via encrypted channels rather than published to the public directory.
//!
//! # Overview
//!
//! Private endpoints enable:
//! - **Enhanced Privacy**: Dedicated per-peer addresses avoid public address reuse
//! - **Custom Channels**: Per-peer dedicated payment addresses or Lightning channels
//! - **Expiration**: Endpoints can expire for security
//! - **Encryption**: Secure storage with AES-256-GCM (with `file-storage` feature)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  PrivateEndpointManager                      │
//! │  ┌─────────────────────────────────────────────────────────┐ │
//! │  │                 PrivateEndpointStore                     │ │
//! │  │  (trait - in-memory, file-based, or custom)              │ │
//! │  └─────────────────────────────────────────────────────────┘ │
//! │  ┌─────────────────────────────────────────────────────────┐ │
//! │  │              PrivateEndpoint Records                     │ │
//! │  │  - peer: PublicKey                                       │ │
//! │  │  - method_id: MethodId                                   │ │
//! │  │  - endpoint: EndpointData                                │ │
//! │  │  - created_at / expires_at                               │ │
//! │  └─────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use paykit_lib::private_endpoints::{PrivateEndpointManager, InMemoryStore};
//!
//! let store = InMemoryStore::new();
//! let manager = PrivateEndpointManager::new(store);
//!
//! // Store a private endpoint from a peer
//! manager.store_endpoint(peer_key, method_id, endpoint_data, None).await?;
//!
//! // Retrieve it later, preferring over public endpoints
//! if let Some(endpoint) = manager.get_endpoint(&peer_key, &method_id).await? {
//!     // Use the private endpoint
//! }
//! ```
//!
//! # Encrypted Storage
//!
//! With the `file-storage` feature, endpoints can be stored encrypted:
//!
//! ```ignore
//! use paykit_lib::private_endpoints::{PrivateEndpointManager, FileStore, encryption};
//!
//! // Generate a random encryption key (store this securely!)
//! let key = encryption::generate_key();
//!
//! // Create encrypted file store
//! let store = FileStore::new_encrypted("./private_endpoints", key)?;
//! let manager = PrivateEndpointManager::new(store);
//!
//! // Or use a passphrase
//! let store = FileStore::new_with_passphrase(
//!     "./private_endpoints",
//!     b"user_passphrase",
//!     b"app_unique_salt"
//! )?;
//! ```

mod storage;
mod types;

#[cfg(feature = "file-storage")]
pub mod encryption;

pub use storage::{InMemoryStore, PrivateEndpointStore, StorageError, StorageResult};
pub use types::{EndpointPolicy, ExpirationPolicy, PrivateEndpoint};

#[cfg(feature = "file-storage")]
pub use storage::FileStore;

use crate::{EndpointData, MethodId, PaykitError, PublicKey, Result};
use std::sync::Arc;

/// Manager for private payment endpoints.
///
/// This provides a high-level interface for storing, retrieving, and managing
/// private endpoints that are exchanged via encrypted channels.
pub struct PrivateEndpointManager<S: PrivateEndpointStore> {
    store: Arc<S>,
    default_policy: EndpointPolicy,
}

impl<S: PrivateEndpointStore> PrivateEndpointManager<S> {
    /// Create a new manager with the given storage backend.
    pub fn new(store: S) -> Self {
        Self {
            store: Arc::new(store),
            default_policy: EndpointPolicy::default(),
        }
    }

    /// Create a new manager with a custom default policy.
    pub fn with_policy(store: S, policy: EndpointPolicy) -> Self {
        Self {
            store: Arc::new(store),
            default_policy: policy,
        }
    }

    /// Store a private endpoint received from a peer.
    ///
    /// # Arguments
    ///
    /// * `peer` - The public key of the peer who offered the endpoint
    /// * `method_id` - The payment method (e.g., "lightning", "onchain")
    /// * `endpoint` - The endpoint data
    /// * `expires_at` - Optional expiration timestamp (unix epoch seconds)
    pub async fn store_endpoint(
        &self,
        peer: PublicKey,
        method_id: MethodId,
        endpoint: EndpointData,
        expires_at: Option<i64>,
    ) -> Result<()> {
        let private_endpoint = PrivateEndpoint::new(peer, method_id, endpoint, expires_at);
        self.store
            .save(private_endpoint)
            .await
            .map_err(|e| PaykitError::Transport(e.to_string()))
    }

    /// Get a private endpoint for a peer and method.
    ///
    /// Returns `None` if no endpoint exists or if it has expired.
    pub async fn get_endpoint(
        &self,
        peer: &PublicKey,
        method_id: &MethodId,
    ) -> Result<Option<EndpointData>> {
        match self
            .store
            .get(peer, method_id)
            .await
            .map_err(|e| PaykitError::Transport(e.to_string()))?
        {
            Some(endpoint) if !endpoint.is_expired() => Ok(Some(endpoint.endpoint)),
            Some(_) => {
                // Endpoint is expired, clean it up
                let _ = self.store.remove(peer, method_id).await;
                Ok(None)
            }
            None => Ok(None),
        }
    }

    /// Get all private endpoints for a peer.
    ///
    /// Returns only non-expired endpoints.
    pub async fn get_endpoints_for_peer(&self, peer: &PublicKey) -> Result<Vec<PrivateEndpoint>> {
        let all = self
            .store
            .list_for_peer(peer)
            .await
            .map_err(|e| PaykitError::Transport(e.to_string()))?;

        Ok(all.into_iter().filter(|e| !e.is_expired()).collect())
    }

    /// Get all peers that have provided private endpoints.
    pub async fn list_peers(&self) -> Result<Vec<PublicKey>> {
        self.store
            .list_peers()
            .await
            .map_err(|e| PaykitError::Transport(e.to_string()))
    }

    /// Remove a specific private endpoint.
    pub async fn remove_endpoint(&self, peer: &PublicKey, method_id: &MethodId) -> Result<()> {
        self.store
            .remove(peer, method_id)
            .await
            .map_err(|e| PaykitError::Transport(e.to_string()))
    }

    /// Remove all private endpoints for a peer.
    pub async fn remove_all_for_peer(&self, peer: &PublicKey) -> Result<()> {
        self.store
            .remove_all_for_peer(peer)
            .await
            .map_err(|e| PaykitError::Transport(e.to_string()))
    }

    /// Clean up all expired endpoints.
    pub async fn cleanup_expired(&self) -> Result<usize> {
        self.store
            .cleanup_expired()
            .await
            .map_err(|e| PaykitError::Transport(e.to_string()))
    }

    /// Check if we have a valid (non-expired) private endpoint for a peer and method.
    pub async fn has_endpoint(&self, peer: &PublicKey, method_id: &MethodId) -> Result<bool> {
        Ok(self.get_endpoint(peer, method_id).await?.is_some())
    }

    /// Get the default endpoint policy.
    pub fn default_policy(&self) -> &EndpointPolicy {
        &self.default_policy
    }

    /// Get the underlying store (for advanced operations).
    pub fn store(&self) -> &S {
        &self.store
    }
}

/// Smart checkout helper that prefers private endpoints over public.
///
/// This function implements the checkout flow described in the BIP:
/// 1. Check for a private endpoint (preferred)
/// 2. Fall back to public directory endpoint
///
/// # Arguments
///
/// * `manager` - The private endpoint manager
/// * `public_reader` - Reader for public directory
/// * `peer` - The peer's public key
/// * `method_id` - The payment method to look up
pub async fn resolve_endpoint<S, R>(
    manager: &PrivateEndpointManager<S>,
    public_reader: &R,
    peer: &PublicKey,
    method_id: &MethodId,
) -> Result<Option<EndpointData>>
where
    S: PrivateEndpointStore,
    R: crate::UnauthenticatedTransportRead,
{
    // First, check for private endpoint
    if let Some(private) = manager.get_endpoint(peer, method_id).await? {
        return Ok(Some(private));
    }

    // Fall back to public endpoint
    crate::get_payment_endpoint(public_reader, peer, method_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pubkey() -> PublicKey {
        #[cfg(feature = "pubky")]
        {
            use pubky::Keypair;
            let keypair = Keypair::random();
            keypair.public_key()
        }
        #[cfg(not(feature = "pubky"))]
        {
            PublicKey("test_pubkey_123".to_string())
        }
    }

    #[tokio::test]
    async fn test_store_and_retrieve_endpoint() {
        let store = InMemoryStore::new();
        let manager = PrivateEndpointManager::new(store);

        let peer = test_pubkey();
        let method = MethodId("lightning".to_string());
        let endpoint = EndpointData("lnbc...".to_string());

        manager
            .store_endpoint(peer.clone(), method.clone(), endpoint.clone(), None)
            .await
            .unwrap();

        let retrieved = manager.get_endpoint(&peer, &method).await.unwrap();
        assert_eq!(retrieved, Some(endpoint));
    }

    #[tokio::test]
    async fn test_expired_endpoint_returns_none() {
        let store = InMemoryStore::new();
        let manager = PrivateEndpointManager::new(store);

        let peer = test_pubkey();
        let method = MethodId("lightning".to_string());
        let endpoint = EndpointData("lnbc...".to_string());

        // Expired 1 hour ago
        let expired_at = chrono::Utc::now().timestamp() - 3600;

        manager
            .store_endpoint(
                peer.clone(),
                method.clone(),
                endpoint.clone(),
                Some(expired_at),
            )
            .await
            .unwrap();

        let retrieved = manager.get_endpoint(&peer, &method).await.unwrap();
        assert_eq!(retrieved, None);
    }

    #[tokio::test]
    async fn test_list_peers() {
        let store = InMemoryStore::new();
        let manager = PrivateEndpointManager::new(store);

        let peer1 = test_pubkey();
        let peer2 = test_pubkey();
        let method = MethodId("onchain".to_string());
        let endpoint = EndpointData("bc1q...".to_string());

        manager
            .store_endpoint(peer1.clone(), method.clone(), endpoint.clone(), None)
            .await
            .unwrap();
        manager
            .store_endpoint(peer2.clone(), method.clone(), endpoint.clone(), None)
            .await
            .unwrap();

        let peers = manager.list_peers().await.unwrap();
        assert!(peers.len() >= 2);
    }

    #[tokio::test]
    async fn test_remove_endpoint() {
        let store = InMemoryStore::new();
        let manager = PrivateEndpointManager::new(store);

        let peer = test_pubkey();
        let method = MethodId("lightning".to_string());
        let endpoint = EndpointData("lnbc...".to_string());

        manager
            .store_endpoint(peer.clone(), method.clone(), endpoint.clone(), None)
            .await
            .unwrap();

        assert!(manager.has_endpoint(&peer, &method).await.unwrap());

        manager.remove_endpoint(&peer, &method).await.unwrap();

        assert!(!manager.has_endpoint(&peer, &method).await.unwrap());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let store = InMemoryStore::new();
        let manager = PrivateEndpointManager::new(store);

        let peer = test_pubkey();
        let method1 = MethodId("lightning".to_string());
        let method2 = MethodId("onchain".to_string());
        let endpoint = EndpointData("lnbc...".to_string());

        // One expired, one valid
        let expired_at = chrono::Utc::now().timestamp() - 3600;
        let valid_until = chrono::Utc::now().timestamp() + 3600;

        manager
            .store_endpoint(
                peer.clone(),
                method1.clone(),
                endpoint.clone(),
                Some(expired_at),
            )
            .await
            .unwrap();
        manager
            .store_endpoint(
                peer.clone(),
                method2.clone(),
                endpoint.clone(),
                Some(valid_until),
            )
            .await
            .unwrap();

        let cleaned = manager.cleanup_expired().await.unwrap();
        assert_eq!(cleaned, 1);

        // Only the valid one should remain
        assert!(!manager.has_endpoint(&peer, &method1).await.unwrap());
        assert!(manager.has_endpoint(&peer, &method2).await.unwrap());
    }
}
