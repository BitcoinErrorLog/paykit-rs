//! Mock transport implementations for testing without network dependencies.
//!
//! These mocks store data in-memory and simulate the behavior of real Pubky
//! transports without requiring a testnet or homeserver.

use async_trait::async_trait;
use paykit_lib::{
    AuthenticatedTransport, EndpointData, MethodId, PaykitError, PublicKey, Result,
    SupportedPayments, UnauthenticatedTransportRead,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

type OwnerData = HashMap<String, Vec<u8>>;
type StorageMap = HashMap<String, OwnerData>;

/// Path prefix for paykit data in mock storage.
const PAYKIT_PATH_PREFIX: &str = "/pub/paykit.app/v0/";
/// Path prefix for follows data in mock storage.
const PUBKY_FOLLOWS_PATH: &str = "/pub/pubky.app/follows/";

/// In-memory storage backend for mock transports.
///
/// Simulates the Pubky storage layer with thread-safe read/write access.
/// Data is organized by owner public key and path.
#[derive(Clone, Default)]
pub struct MockStorage {
    /// Map of owner -> path -> content
    data: Arc<RwLock<StorageMap>>,
}

impl MockStorage {
    /// Create a new empty mock storage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store data at the given owner/path.
    pub fn put(&self, owner: &str, path: &str, content: impl Into<Vec<u8>>) {
        let mut data = self.data.write().unwrap();
        let owner_data = data.entry(owner.to_string()).or_default();
        owner_data.insert(path.to_string(), content.into());
    }

    /// Get data at the given owner/path.
    pub fn get(&self, owner: &str, path: &str) -> Option<Vec<u8>> {
        let data = self.data.read().unwrap();
        data.get(owner)?.get(path).cloned()
    }

    /// Delete data at the given owner/path.
    pub fn delete(&self, owner: &str, path: &str) -> bool {
        let mut data = self.data.write().unwrap();
        if let Some(owner_data) = data.get_mut(owner) {
            owner_data.remove(path).is_some()
        } else {
            false
        }
    }

    /// List paths under the given owner/prefix.
    pub fn list(&self, owner: &str, prefix: &str) -> Vec<String> {
        let data = self.data.read().unwrap();
        data.get(owner)
            .map(|owner_data| {
                owner_data
                    .keys()
                    .filter(|p| p.starts_with(prefix))
                    .map(|p| {
                        // Extract the entry name after the prefix
                        let rest = &p[prefix.len()..];
                        // Get just the first component
                        rest.split('/').next().unwrap_or(rest).to_string()
                    })
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Clear all data (useful for test reset).
    pub fn clear(&self) {
        let mut data = self.data.write().unwrap();
        data.clear();
    }

    /// Get total number of entries for debugging.
    pub fn entry_count(&self) -> usize {
        let data = self.data.read().unwrap();
        data.values().map(|m| m.len()).sum()
    }
}

/// Mock authenticated transport for testing write operations.
///
/// Provides a test double for `AuthenticatedTransport` that stores data
/// in a `MockStorage` instance without making network calls.
pub struct MockAuthenticatedTransport {
    storage: MockStorage,
    owner: String,
}

impl MockAuthenticatedTransport {
    /// Create a new mock authenticated transport for the given owner.
    pub fn new(storage: MockStorage, owner: impl Into<String>) -> Self {
        Self {
            storage,
            owner: owner.into(),
        }
    }

    /// Create with a random owner ID for isolated tests.
    pub fn with_random_owner(storage: MockStorage) -> Self {
        let owner = format!("mock-owner-{}", rand_id());
        Self::new(storage, owner)
    }

    /// Get the owner public key string.
    pub fn owner(&self) -> &str {
        &self.owner
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AuthenticatedTransport for MockAuthenticatedTransport {
    async fn upsert_payment_endpoint(&self, method: &MethodId, data: &EndpointData) -> Result<()> {
        let path = format!("{}{}", PAYKIT_PATH_PREFIX, method.0);
        self.storage.put(&self.owner, &path, data.0.as_bytes());
        Ok(())
    }

    async fn remove_payment_endpoint(&self, method: &MethodId) -> Result<()> {
        let path = format!("{}{}", PAYKIT_PATH_PREFIX, method.0);
        if self.storage.delete(&self.owner, &path) {
            Ok(())
        } else {
            Err(PaykitError::Transport(format!(
                "endpoint {} not found",
                method.0
            )))
        }
    }
}

/// Mock unauthenticated transport for testing read operations.
///
/// Provides a test double for `UnauthenticatedTransportRead` that reads
/// from a `MockStorage` instance without making network calls.
pub struct MockUnauthenticatedTransport {
    storage: MockStorage,
}

impl MockUnauthenticatedTransport {
    /// Create a new mock unauthenticated transport.
    pub fn new(storage: MockStorage) -> Self {
        Self { storage }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl UnauthenticatedTransportRead for MockUnauthenticatedTransport {
    async fn fetch_supported_payments(&self, payee: &PublicKey) -> Result<SupportedPayments> {
        let owner = public_key_to_string(payee);
        let entries = self.storage.list(&owner, PAYKIT_PATH_PREFIX);

        let mut payments = SupportedPayments::default();
        for entry in entries {
            let path = format!("{}{}", PAYKIT_PATH_PREFIX, entry);
            if let Some(data) = self.storage.get(&owner, &path) {
                let method = MethodId(entry);
                let content = String::from_utf8_lossy(&data).to_string();
                payments.entries.insert(method, EndpointData(content));
            }
        }

        Ok(payments)
    }

    async fn fetch_payment_endpoint(
        &self,
        payee: &PublicKey,
        method: &MethodId,
    ) -> Result<Option<EndpointData>> {
        let owner = public_key_to_string(payee);
        let path = format!("{}{}", PAYKIT_PATH_PREFIX, method.0);

        Ok(self.storage.get(&owner, &path).map(|data| {
            let content = String::from_utf8_lossy(&data).to_string();
            EndpointData(content)
        }))
    }

    async fn fetch_known_contacts(&self, owner: &PublicKey) -> Result<Vec<PublicKey>> {
        let owner_str = public_key_to_string(owner);
        let entries = self.storage.list(&owner_str, PUBKY_FOLLOWS_PATH);

        let contacts = entries
            .into_iter()
            .filter_map(|entry| string_to_public_key(&entry))
            .collect();

        Ok(contacts)
    }

    async fn list_directory(&self, owner: &PublicKey, path: &str) -> Result<Vec<String>> {
        let owner_str = public_key_to_string(owner);
        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{}/", path)
        };

        Ok(self.storage.list(&owner_str, &prefix))
    }

    async fn fetch_file(&self, owner: &PublicKey, path: &str) -> Result<Option<Vec<u8>>> {
        let owner_str = public_key_to_string(owner);
        Ok(self.storage.get(&owner_str, path))
    }
}

/// Generate a random ID for isolated test owners.
fn rand_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

/// Convert PublicKey to string representation.
fn public_key_to_string(pk: &PublicKey) -> String {
    pk.to_string()
}

/// Convert string to PublicKey if valid.
fn string_to_public_key(s: &str) -> Option<PublicKey> {
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_storage_basic_operations() {
        let storage = MockStorage::new();

        // Put and get
        storage.put("owner1", "/path/a", "content a");
        assert_eq!(
            storage.get("owner1", "/path/a"),
            Some(b"content a".to_vec())
        );

        // Non-existent
        assert!(storage.get("owner1", "/path/b").is_none());
        assert!(storage.get("owner2", "/path/a").is_none());

        // Delete
        assert!(storage.delete("owner1", "/path/a"));
        assert!(storage.get("owner1", "/path/a").is_none());
        assert!(!storage.delete("owner1", "/path/a")); // Already deleted
    }

    #[test]
    fn test_mock_storage_list() {
        let storage = MockStorage::new();

        storage.put("owner1", "/pub/paykit.app/v0/lightning", "ln");
        storage.put("owner1", "/pub/paykit.app/v0/onchain", "btc");
        storage.put("owner1", "/pub/other/file", "other");

        let entries = storage.list("owner1", "/pub/paykit.app/v0/");
        assert_eq!(entries.len(), 2);
        assert!(entries.contains(&"lightning".to_string()));
        assert!(entries.contains(&"onchain".to_string()));
    }

    #[tokio::test]
    async fn test_mock_transport_endpoint_roundtrip() {
        let storage = MockStorage::new();
        let owner = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
        let auth = MockAuthenticatedTransport::new(storage.clone(), owner);
        let reader = MockUnauthenticatedTransport::new(storage);

        let method = MethodId("lightning".into());
        let data = EndpointData("{\"bolt11\":\"lnbc...\"}".into());

        // Write
        auth.upsert_payment_endpoint(&method, &data).await.unwrap();

        // Read back - use a valid pubky format or skip this test for non-pubky
        // For testing purposes, we use the owner string directly since fetch
        // will convert PublicKey to string anyway
        if let Ok(pk) = owner.parse::<PublicKey>() {
            let fetched = reader.fetch_payment_endpoint(&pk, &method).await.unwrap();
            assert_eq!(fetched, Some(data));
        }
    }

    #[tokio::test]
    async fn test_mock_transport_list_payments() {
        let storage = MockStorage::new();
        let owner = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
        let auth = MockAuthenticatedTransport::new(storage.clone(), owner);
        let reader = MockUnauthenticatedTransport::new(storage);

        // Add multiple endpoints
        auth.upsert_payment_endpoint(&MethodId("lightning".into()), &EndpointData("ln".into()))
            .await
            .unwrap();
        auth.upsert_payment_endpoint(&MethodId("onchain".into()), &EndpointData("btc".into()))
            .await
            .unwrap();

        if let Ok(pk) = owner.parse::<PublicKey>() {
            let payments = reader.fetch_supported_payments(&pk).await.unwrap();
            assert_eq!(payments.entries.len(), 2);
        }
    }

    #[tokio::test]
    async fn test_mock_transport_remove_endpoint() {
        let storage = MockStorage::new();
        let auth = MockAuthenticatedTransport::new(storage.clone(), "test-owner");

        let method = MethodId("lightning".into());
        let data = EndpointData("ln".into());

        auth.upsert_payment_endpoint(&method, &data).await.unwrap();
        auth.remove_payment_endpoint(&method).await.unwrap();

        // Removing again should error
        let result = auth.remove_payment_endpoint(&method).await;
        assert!(result.is_err());
    }
}
