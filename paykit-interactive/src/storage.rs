use crate::{PaykitReceipt, Result};
use paykit_lib::{EndpointData, MethodId, PublicKey};
use paykit_lib::private_endpoints::PrivateEndpoint;

/// Trait for persisting Paykit data.
///
/// This should be implemented by the host application (e.g., using a local database).
#[async_trait::async_trait]
pub trait PaykitStorage: Send + Sync {
    /// Save a receipt.
    async fn save_receipt(&self, receipt: &PaykitReceipt) -> Result<()>;

    /// Retrieve a receipt by ID.
    async fn get_receipt(&self, receipt_id: &str) -> Result<Option<PaykitReceipt>>;

    /// List all receipts.
    async fn list_receipts(&self) -> Result<Vec<PaykitReceipt>>;

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

    /// List all private endpoints for a peer.
    async fn list_private_endpoints_for_peer(
        &self,
        peer: &PublicKey,
    ) -> Result<Vec<(MethodId, String)>>;

    /// Remove a private endpoint.
    async fn remove_private_endpoint(
        &self,
        peer: &PublicKey,
        method: &MethodId,
    ) -> Result<()>;
}

/// Adapter that bridges PaykitStorage to the new PrivateEndpointStore trait.
///
/// This allows existing PaykitStorage implementations to work with the
/// new private endpoint management system in paykit-lib.
pub struct StorageAdapter<S: PaykitStorage> {
    storage: S,
}

impl<S: PaykitStorage> StorageAdapter<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Save a private endpoint using the underlying storage.
    pub async fn save_endpoint(&self, endpoint: &PrivateEndpoint) -> Result<()> {
        self.storage
            .save_private_endpoint(&endpoint.peer, &endpoint.method_id, &endpoint.endpoint.0)
            .await
    }

    /// Get a private endpoint as EndpointData.
    pub async fn get_endpoint(
        &self,
        peer: &PublicKey,
        method: &MethodId,
    ) -> Result<Option<EndpointData>> {
        self.storage
            .get_private_endpoint(peer, method)
            .await
            .map(|opt| opt.map(EndpointData))
    }
}

/// Smart checkout helper that resolves the best endpoint for a payment.
///
/// This implements the checkout flow from the BIP specification:
/// 1. Check for a private endpoint (preferred for privacy)
/// 2. Fall back to public directory endpoint
///
/// # Arguments
///
/// * `storage` - Storage for private endpoints
/// * `public_reader` - Reader for public directory
/// * `peer` - The peer's public key
/// * `method_id` - The payment method to look up
///
/// # Returns
///
/// The endpoint data if found, or None if no endpoint is available.
pub async fn smart_checkout<S, R>(
    storage: &S,
    public_reader: &R,
    peer: &PublicKey,
    method_id: &MethodId,
) -> Result<Option<EndpointData>>
where
    S: PaykitStorage,
    R: paykit_lib::UnauthenticatedTransportRead,
{
    // Step 1: Check for private endpoint (preferred)
    if let Some(private) = storage.get_private_endpoint(peer, method_id).await? {
        return Ok(Some(EndpointData(private)));
    }

    // Step 2: Fall back to public endpoint
    paykit_lib::get_payment_endpoint(public_reader, peer, method_id)
        .await
        .map_err(|e| crate::InteractiveError::Transport(e.to_string()))
}

/// Result of smart checkout with source information.
#[derive(Debug, Clone)]
pub struct CheckoutResult {
    /// The endpoint data.
    pub endpoint: EndpointData,
    /// Whether this came from a private endpoint.
    pub is_private: bool,
    /// The payment method.
    pub method_id: MethodId,
}

/// Smart checkout with detailed result including source information.
pub async fn smart_checkout_detailed<S, R>(
    storage: &S,
    public_reader: &R,
    peer: &PublicKey,
    method_id: &MethodId,
) -> Result<Option<CheckoutResult>>
where
    S: PaykitStorage,
    R: paykit_lib::UnauthenticatedTransportRead,
{
    // Step 1: Check for private endpoint (preferred)
    if let Some(private) = storage.get_private_endpoint(peer, method_id).await? {
        return Ok(Some(CheckoutResult {
            endpoint: EndpointData(private),
            is_private: true,
            method_id: method_id.clone(),
        }));
    }

    // Step 2: Fall back to public endpoint
    match paykit_lib::get_payment_endpoint(public_reader, peer, method_id).await {
        Ok(Some(endpoint)) => Ok(Some(CheckoutResult {
            endpoint,
            is_private: false,
            method_id: method_id.clone(),
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(crate::InteractiveError::Transport(e.to_string())),
    }
}

/// Checkout across all available methods for a peer.
///
/// Returns all available endpoints (private and public) for the peer,
/// with private endpoints taking precedence for each method.
pub async fn smart_checkout_all_methods<S, R>(
    storage: &S,
    public_reader: &R,
    peer: &PublicKey,
) -> Result<Vec<CheckoutResult>>
where
    S: PaykitStorage,
    R: paykit_lib::UnauthenticatedTransportRead,
{
    let mut results = Vec::new();
    let mut seen_methods = std::collections::HashSet::new();

    // First, add all private endpoints
    let private_endpoints = storage.list_private_endpoints_for_peer(peer).await?;
    for (method_id, endpoint) in private_endpoints {
        seen_methods.insert(method_id.0.clone());
        results.push(CheckoutResult {
            endpoint: EndpointData(endpoint),
            is_private: true,
            method_id,
        });
    }

    // Then, add public endpoints for methods we don't have private endpoints for
    match paykit_lib::get_payment_list(public_reader, peer).await {
        Ok(public) => {
            for (method_id, endpoint) in public.entries {
                if !seen_methods.contains(&method_id.0) {
                    results.push(CheckoutResult {
                        endpoint,
                        is_private: false,
                        method_id,
                    });
                }
            }
        }
        Err(e) => {
            // Log but don't fail - we may still have private endpoints
            tracing_warn(&format!("Failed to fetch public endpoints: {}", e));
        }
    }

    Ok(results)
}

// Helper to avoid requiring tracing feature
fn tracing_warn(_msg: &str) {
    #[cfg(feature = "tracing")]
    tracing::warn!("{}", _msg);
}
