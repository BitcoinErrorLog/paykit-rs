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

    /// Lists directory entries at the given path.
    /// Returns a vector of entry names (file/directory names without full path).
    async fn list_directory(&self, owner: &PublicKey, path: &str) -> Result<Vec<String>>;

    /// Fetches raw file content from the given path.
    /// Returns None if the file doesn't exist.
    async fn fetch_file(&self, owner: &PublicKey, path: &str) -> Result<Option<Vec<u8>>>;
}

/// Trait describing authenticated write (and optional read) access.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait AuthenticatedTransport {
    /// Writes or updates a payment endpoint document.
    async fn upsert_payment_endpoint(&self, method: &MethodId, data: &EndpointData) -> Result<()>;

    /// Removes an existing payment endpoint for the provided method.
    async fn remove_payment_endpoint(&self, method: &MethodId) -> Result<()>;
}
