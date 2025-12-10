use async_trait::async_trait;

use crate::{EndpointData, MethodId, PublicKey, Result};

/// Trait describing read-only access to public Paykit transport.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait UnauthenticatedTransportRead {
    /// Fetches the raw Supported Payments List for the provided `payee`.
    async fn fetch_supported_payments(&self, payee: &PublicKey)
        -> Result<crate::SupportedPayments>;

    /// Fetches an individual payment endpoint document if it exists.
    async fn fetch_payment_endpoint(
        &self,
        payee: &PublicKey,
        method: &MethodId,
    ) -> Result<Option<EndpointData>>;

    /// Returns the set of known contacts (public keys) reachable to the caller.
    async fn fetch_known_contacts(&self, owner: &PublicKey) -> Result<Vec<PublicKey>>;

    /// Get a file at the given path from a public key's storage.
    ///
    /// Returns the content as a string if found, None if the file doesn't exist.
    async fn get(&self, owner: &PublicKey, path: &str) -> Result<Option<String>>;

    /// List entries in a directory from a public key's storage.
    ///
    /// Returns a list of file/directory names (not full paths).
    async fn list_directory(&self, owner: &PublicKey, path: &str) -> Result<Vec<String>>;
}

/// Trait describing authenticated write (and optional read) access.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait AuthenticatedTransport {
    /// Writes or updates a payment endpoint document.
    async fn upsert_payment_endpoint(&self, method: &MethodId, data: &EndpointData) -> Result<()>;

    /// Removes an existing payment endpoint for the provided method.
    async fn remove_payment_endpoint(&self, method: &MethodId) -> Result<()>;

    /// Put (create or update) a file at the given path.
    async fn put(&self, path: &str, content: &str) -> Result<()>;

    /// Get a file at the given path.
    ///
    /// Returns the content as a string if found, None if the file doesn't exist.
    async fn get(&self, path: &str) -> Result<Option<String>>;

    /// Delete a file at the given path.
    async fn delete(&self, path: &str) -> Result<()>;
}
