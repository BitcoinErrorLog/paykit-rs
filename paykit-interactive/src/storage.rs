use crate::{PaykitReceipt, Result};
use paykit_lib::{MethodId, PublicKey};

/// Trait for persisting Paykit data.
///
/// This should be implemented by the host application (e.g., using a local database).
#[async_trait::async_trait]
pub trait PaykitStorage: Send + Sync {
    /// Save a receipt.
    async fn save_receipt(&self, receipt: &PaykitReceipt) -> Result<()>;

    /// Retrieve a receipt by ID.
    async fn get_receipt(&self, receipt_id: &str) -> Result<Option<PaykitReceipt>>;

    /// Save a private endpoint offered by a peer.
    ///
    /// * `peer`: The public key of the peer who offered the endpoint.
    /// * `method`: The payment method ID (e.g., "lightning").
    /// * `endpoint`: The endpoint data string.
    async fn save_private_endpoint(
        &self,
        peer: &PublicKey,
        method: &MethodId,
        endpoint: &str,
    ) -> Result<()>;

    /// Get a private endpoint for a peer and method.
    async fn get_private_endpoint(
        &self,
        peer: &PublicKey,
        method: &MethodId,
    ) -> Result<Option<String>>;
}
