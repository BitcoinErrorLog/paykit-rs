//! Payment Request Discovery
//!
//! This module provides functionality to publish and discover payment requests
//! via Pubky homeservers, enabling async payment request delivery.
//!
//! ## Path Format (v0)
//!
//! Requests are stored on the **sender's** homeserver at:
//! `/pub/paykit.app/v0/requests/{recipient_scope}/{request_id}`
//!
//! Where `recipient_scope = hex(sha256(normalized_pubkey_z32))`.
//!
//! ## Security
//!
//! Payment requests are encrypted using Paykit Sealed Blob v1 before storage
//! to prevent public exposure of payment details. All requests MUST be encrypted.
//! Plaintext storage is REJECTED for security reasons.
//!
//! ## Discovery
//!
//! Recipients poll known contacts and list their `.../{my_scope}/` directory
//! to discover pending requests. Recipients cannot delete requests from sender
//! storage (deduplication is local-only).

use crate::{PaymentRequest, RequestNotification};
use paykit_lib::protocol::{
    payment_request_aad, payment_request_path, payment_requests_dir, subscription_proposal_aad,
    subscription_proposal_path, subscription_proposals_dir,
};
use paykit_lib::{AuthenticatedTransport, PublicKey, UnauthenticatedTransportRead};
use pubky_noise::sealed_blob::{is_sealed_blob, sealed_blob_decrypt, sealed_blob_encrypt};
use serde::{Deserialize, Serialize};

/// Path prefix for payment requests in Pubky storage (v0).
/// Use `payment_request_path()` or `payment_requests_dir()` for canonical paths.
pub const PAYKIT_REQUESTS_PATH: &str = "/pub/paykit.app/v0/requests/";

/// Path prefix for incoming request notifications (DEPRECATED).
/// Notifications require writing to recipient storage, which is not allowed
/// in the sender-storage model. Use polling discovery instead.
#[deprecated(
    since = "0.3.0",
    note = "Notifications require writing to recipient storage. Use polling discovery instead."
)]
pub const PAYKIT_NOTIFICATIONS_PATH: &str = "/pub/paykit.app/v0/notifications/requests/";

/// A discoverable payment request stored in Pubky.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedRequest {
    /// The full payment request.
    pub request: PaymentRequest,
    /// Timestamp when published.
    pub published_at: i64,
    /// Whether this request is still active.
    pub active: bool,
}

impl PublishedRequest {
    /// Create a new published request.
    pub fn new(request: PaymentRequest) -> Self {
        Self {
            request,
            published_at: chrono::Utc::now().timestamp(),
            active: true,
        }
    }

    /// Mark this request as inactive.
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

/// Publish a payment request to the sender's Pubky storage (encrypted).
///
/// The request is stored at `/pub/paykit.app/v0/requests/{recipient_scope}/{request_id}`
/// as a Paykit Sealed Blob v1 encrypted to the recipient's Noise endpoint public key.
///
/// # Path Format
///
/// `recipient_scope = hex(sha256(normalized_recipient_pubkey_z32))`
///
/// This creates a per-recipient directory on the sender's storage, allowing
/// recipients to poll known contacts and list `.../{my_scope}/` to discover requests.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the sender
/// * `request` - The payment request to publish
/// * `recipient_noise_pk` - Recipient's Noise endpoint X25519 public key (32 bytes)
///
/// # Example
///
/// ```ignore
/// use paykit_subscriptions::discovery::publish_payment_request;
///
/// let request = PaymentRequest::new(from, to, amount, currency, method);
/// publish_payment_request(&transport, &request, &recipient_noise_pk).await?;
/// ```
pub async fn publish_payment_request<T: AuthenticatedTransport>(
    transport: &T,
    request: &PaymentRequest,
    recipient_noise_pk: &[u8; 32],
) -> crate::Result<()> {
    let published = PublishedRequest::new(request.clone());
    let plaintext = serde_json::to_vec(&published)?;

    // Build canonical path using recipient scope
    let recipient_pubkey_z32 = request.to.to_string();
    let path = payment_request_path(&recipient_pubkey_z32, &request.request_id)
        .map_err(|e| anyhow::anyhow!("Invalid recipient pubkey: {}", e))?;

    // Build canonical AAD
    let aad = payment_request_aad(&recipient_pubkey_z32, &request.request_id)
        .map_err(|e| anyhow::anyhow!("Failed to build AAD: {}", e))?;

    // Encrypt using Sealed Blob v1
    let envelope = sealed_blob_encrypt(recipient_noise_pk, &plaintext, &aad, Some("request"))
        .map_err(|e| anyhow::anyhow!("Failed to encrypt payment request: {}", e))?;

    // Store encrypted blob on sender storage
    transport
        .put(&path, &envelope)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to publish request: {}", e))?;

    Ok(())
}

/// Publish a notification for a payment request to the recipient.
///
/// # DEPRECATED
///
/// This function is deprecated because it requires writing to the recipient's
/// storage, which is not allowed in the sender-storage model. The Paykit v0
/// protocol uses polling-based discovery instead:
///
/// 1. Sender publishes encrypted request to their own storage at
///    `/pub/paykit.app/v0/requests/{recipient_scope}/{request_id}`
/// 2. Recipient polls known contacts and lists `.../{my_scope}/` to discover requests
///
/// This function will fail in most configurations because the sender does not
/// have write access to the recipient's homeserver.
#[deprecated(
    since = "0.3.0",
    note = "Notifications require writing to recipient storage. Use polling discovery instead."
)]
pub async fn publish_request_notification<T: AuthenticatedTransport>(
    transport: &T,
    recipient: &PublicKey,
    request: &PaymentRequest,
) -> crate::Result<()> {
    let notification = RequestNotification {
        request_id: request.request_id.clone(),
        from: request.from.clone(),
        amount: request.amount,
        currency: request.currency.clone(),
        created_at: request.created_at,
    };

    let json = serde_json::to_string(&notification)?;

    // DEPRECATED: This requires write access to recipient's storage,
    // which is not allowed in the sender-storage model.
    #[allow(deprecated)]
    let path = format!(
        "pubky://{}{}{}",
        recipient, PAYKIT_NOTIFICATIONS_PATH, request.request_id
    );
    transport
        .put(&path, &json)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to publish notification: {}", e))?;

    Ok(())
}

/// Discover payment requests from a sender addressed to me.
///
/// Lists the sender's `.../{my_scope}/` directory and decrypts Sealed Blob v1
/// encrypted requests using the recipient's Noise secret key.
///
/// # Path Format
///
/// Requests are stored at: `/pub/paykit.app/v0/requests/{recipient_scope}/{request_id}`
/// This function lists the `{recipient_scope}` directory on the sender's storage.
///
/// # Arguments
///
/// * `reader` - Unauthenticated reader for Pubky storage
/// * `sender` - The sender's public key
/// * `my_pubkey_z32` - The recipient's own z-base-32 pubkey (to compute scope)
/// * `my_noise_sk` - Recipient's Noise endpoint X25519 secret key (32 bytes)
///
/// # Returns
///
/// A list of published payment requests from the sender addressed to me.
pub async fn discover_requests<R: UnauthenticatedTransportRead>(
    reader: &R,
    sender: &PublicKey,
    my_pubkey_z32: &str,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Vec<PublishedRequest>> {
    // Compute my scope directory
    let my_scope_dir = payment_requests_dir(my_pubkey_z32)
        .map_err(|e| anyhow::anyhow!("Invalid pubkey: {}", e))?;

    let entries = reader
        .list_directory(sender, &my_scope_dir)
        .await
        .unwrap_or_else(|_| vec![]); // Empty if directory doesn't exist

    let mut requests = Vec::new();

    for entry in entries {
        // Build full path for this request
        let path = payment_request_path(my_pubkey_z32, &entry)
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;

        if let Ok(Some(content)) = reader.get(sender, &path).await {
            // Decrypt sealed blob only (no plaintext fallback)
            if let Some(published) =
                try_decrypt_request(&content, my_pubkey_z32, &entry, my_noise_sk)
            {
                if published.active {
                    requests.push(published);
                }
            }
        }
    }

    Ok(requests)
}

/// Discover a specific payment request by ID.
///
/// Decrypts Sealed Blob v1 encrypted requests using the recipient's Noise secret key.
///
/// # Arguments
///
/// * `reader` - Unauthenticated reader for Pubky storage
/// * `sender` - The sender's public key
/// * `my_pubkey_z32` - The recipient's own z-base-32 pubkey (to compute scope)
/// * `request_id` - The request ID
/// * `my_noise_sk` - Recipient's Noise endpoint X25519 secret key (32 bytes)
///
/// # Returns
///
/// The published request if found.
pub async fn discover_request<R: UnauthenticatedTransportRead>(
    reader: &R,
    sender: &PublicKey,
    my_pubkey_z32: &str,
    request_id: &str,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Option<PublishedRequest>> {
    // Build canonical path
    let path = payment_request_path(my_pubkey_z32, request_id)
        .map_err(|e| anyhow::anyhow!("Invalid pubkey: {}", e))?;

    match reader.get(sender, &path).await {
        Ok(Some(content)) => {
            let published = try_decrypt_request(&content, my_pubkey_z32, request_id, my_noise_sk)
                .ok_or_else(|| {
                anyhow::anyhow!("Failed to decrypt request (sealed blob only)")
            })?;
            Ok(Some(published))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to fetch request: {}", e)),
    }
}

/// Decrypt a sealed blob payment request.
///
/// SECURITY: Only encrypted Sealed Blob v1 format is accepted.
/// Plaintext storage is REJECTED for security reasons.
fn try_decrypt_request(
    content: &str,
    recipient_pubkey_z32: &str,
    request_id: &str,
    my_noise_sk: &[u8; 32],
) -> Option<PublishedRequest> {
    // SECURITY: Only accept encrypted sealed blobs
    if !is_sealed_blob(content) {
        tracing::warn!(
            "SECURITY: Rejected plaintext payment request for {}. Only encrypted blobs accepted.",
            request_id
        );
        return None;
    }

    // Build canonical AAD (must match encryption)
    let aad = match payment_request_aad(recipient_pubkey_z32, request_id) {
        Ok(aad) => aad,
        Err(e) => {
            tracing::warn!("Failed to build AAD for request {}: {}", request_id, e);
            return None;
        }
    };

    // Decrypt
    match sealed_blob_decrypt(my_noise_sk, content, &aad) {
        Ok(plaintext) => serde_json::from_slice(&plaintext).ok(),
        Err(e) => {
            tracing::warn!("Failed to decrypt payment request {}: {}", request_id, e);
            None
        }
    }
}

/// Cancel a published payment request.
///
/// This removes the request from storage.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the sender
/// * `recipient_pubkey_z32` - The recipient's z-base-32 pubkey
/// * `request_id` - The request ID to cancel
pub async fn cancel_payment_request<T: AuthenticatedTransport>(
    transport: &T,
    recipient_pubkey_z32: &str,
    request_id: &str,
) -> crate::Result<()> {
    let path = payment_request_path(recipient_pubkey_z32, request_id)
        .map_err(|e| anyhow::anyhow!("Invalid pubkey: {}", e))?;

    // Delete the request from storage
    transport
        .delete(&path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to cancel request: {}", e))?;

    Ok(())
}

/// Discovery poller for incoming payment requests.
///
/// This struct provides polling-based discovery of payment requests
/// from known contacts or specific peers. Supports decryption of
/// Sealed Blob v1 encrypted requests using the provided Noise secret key.
///
/// # Discovery Model
///
/// The poller iterates over known peers (contacts) and lists each peer's
/// `.../{my_scope}/` directory to find requests addressed to me.
pub struct RequestDiscoveryPoller<R: UnauthenticatedTransportRead> {
    reader: R,
    known_peers: Vec<PublicKey>,
    last_poll: i64,
    poll_interval_secs: u64,
    /// My z-base-32 pubkey (to compute my scope directory)
    my_pubkey_z32: String,
    /// Noise endpoint secret key for decrypting encrypted requests
    noise_sk: [u8; 32],
}

impl<R: UnauthenticatedTransportRead> RequestDiscoveryPoller<R> {
    /// Create a new poller.
    ///
    /// # Arguments
    ///
    /// * `reader` - Unauthenticated reader for Pubky storage
    /// * `poll_interval_secs` - Polling interval in seconds
    /// * `my_pubkey_z32` - My z-base-32 encoded pubkey (to compute my scope)
    /// * `noise_sk` - Noise endpoint X25519 secret key for decrypting requests
    pub fn new(
        reader: R,
        poll_interval_secs: u64,
        my_pubkey_z32: String,
        noise_sk: [u8; 32],
    ) -> Self {
        Self {
            reader,
            known_peers: Vec::new(),
            last_poll: 0,
            poll_interval_secs,
            my_pubkey_z32,
            noise_sk,
        }
    }

    /// Add a peer to monitor for payment requests.
    pub fn add_peer(&mut self, peer: PublicKey) {
        if !self.known_peers.contains(&peer) {
            self.known_peers.push(peer);
        }
    }

    /// Remove a peer from monitoring.
    pub fn remove_peer(&mut self, peer: &PublicKey) {
        self.known_peers.retain(|p| p != peer);
    }

    /// Check if a poll is due.
    pub fn should_poll(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        (now - self.last_poll) as u64 >= self.poll_interval_secs
    }

    /// Poll for new payment requests from all monitored peers.
    ///
    /// Returns a list of (sender, requests) tuples for peers with active requests.
    pub async fn poll(&mut self) -> crate::Result<Vec<(PublicKey, Vec<PublishedRequest>)>> {
        let mut results = Vec::new();

        for peer in &self.known_peers {
            match discover_requests(&self.reader, peer, &self.my_pubkey_z32, &self.noise_sk).await {
                Ok(requests) if !requests.is_empty() => {
                    results.push((peer.clone(), requests));
                }
                Ok(_) => {} // Empty, skip
                Err(e) => {
                    tracing::debug!("Failed to poll peer {}: {}", peer, e);
                }
            }
        }

        self.last_poll = chrono::Utc::now().timestamp();
        Ok(results)
    }

    /// Poll for new payment requests and filter by creation time.
    ///
    /// Only returns requests created after `after_timestamp`.
    pub async fn poll_new(
        &mut self,
        after_timestamp: i64,
    ) -> crate::Result<Vec<(PublicKey, Vec<PublishedRequest>)>> {
        let all = self.poll().await?;

        let filtered: Vec<(PublicKey, Vec<PublishedRequest>)> = all
            .into_iter()
            .map(|(peer, requests)| {
                let new_requests: Vec<PublishedRequest> = requests
                    .into_iter()
                    .filter(|r| r.request.created_at > after_timestamp)
                    .collect();
                (peer, new_requests)
            })
            .filter(|(_, requests)| !requests.is_empty())
            .collect();

        Ok(filtered)
    }
}

// ============================================================
// Subscription Discovery (with encryption support)
// ============================================================

/// Path prefix for subscription proposals (v0).
/// Use `subscription_proposal_path()` or `subscription_proposals_dir()` for canonical paths.
pub const PAYKIT_PROPOSALS_PATH: &str = "/pub/paykit.app/v0/subscriptions/proposals/";
/// Path prefix for subscription agreements (v0).
pub const PAYKIT_AGREEMENTS_PATH: &str = "/pub/paykit.app/v0/subscriptions/agreements/";
/// Path prefix for subscription cancellations (v0).
pub const PAYKIT_CANCELLATIONS_PATH: &str = "/pub/paykit.app/v0/subscriptions/cancellations/";

/// Discover subscription proposals from a provider addressed to me.
///
/// Lists the provider's `.../{my_scope}/` directory and decrypts Sealed Blob v1
/// encrypted proposals using the subscriber's Noise secret key.
///
/// # Path Format
///
/// Proposals are stored at: `/pub/paykit.app/v0/subscriptions/proposals/{subscriber_scope}/{proposal_id}`
/// This function lists the `{subscriber_scope}` directory on the provider's storage.
///
/// # Arguments
///
/// * `reader` - Unauthenticated transport for reading
/// * `provider` - The provider's public key
/// * `my_pubkey_z32` - My z-base-32 encoded pubkey (to compute my scope)
/// * `my_noise_sk` - My Noise secret key for decryption
pub async fn discover_subscription_proposals<R: UnauthenticatedTransportRead>(
    reader: &R,
    provider: &PublicKey,
    my_pubkey_z32: &str,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Vec<crate::Subscription>> {
    // Compute my scope directory on provider storage
    let my_scope_dir = subscription_proposals_dir(my_pubkey_z32)
        .map_err(|e| anyhow::anyhow!("Invalid pubkey: {}", e))?;

    let entries: Vec<String> = reader
        .list_directory(provider, &my_scope_dir)
        .await
        .unwrap_or_default();

    let mut proposals = Vec::new();
    for entry in entries {
        // Build full path for this proposal
        let full_path = subscription_proposal_path(my_pubkey_z32, &entry)
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;

        if let Ok(Some(content)) = reader.get(provider, &full_path).await {
            if let Some(subscription) =
                try_decrypt_subscription_proposal(&content, my_pubkey_z32, &entry, my_noise_sk)
            {
                proposals.push(subscription);
            }
        }
    }

    Ok(proposals)
}

/// Discover a specific subscription proposal by ID.
///
/// Decrypts Sealed Blob v1 encrypted proposals using the subscriber's Noise secret key.
///
/// # Arguments
///
/// * `reader` - Unauthenticated transport for reading
/// * `provider` - The provider's public key
/// * `my_pubkey_z32` - My z-base-32 encoded pubkey (to compute my scope)
/// * `proposal_id` - The proposal ID
/// * `my_noise_sk` - My Noise secret key for decryption
pub async fn discover_subscription_proposal<R: UnauthenticatedTransportRead>(
    reader: &R,
    provider: &PublicKey,
    my_pubkey_z32: &str,
    proposal_id: &str,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Option<crate::Subscription>> {
    // Build canonical path
    let path = subscription_proposal_path(my_pubkey_z32, proposal_id)
        .map_err(|e| anyhow::anyhow!("Invalid pubkey: {}", e))?;

    match reader.get(provider, &path).await {
        Ok(Some(content)) => {
            let subscription = try_decrypt_subscription_proposal(
                &content,
                my_pubkey_z32,
                proposal_id,
                my_noise_sk,
            );
            Ok(subscription)
        }
        Ok(None) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to fetch proposal: {}", e)),
    }
}

/// Discover subscription agreements for a party.
///
/// Decrypts Sealed Blob v1 encrypted agreements using the party's Noise secret key.
pub async fn discover_subscription_agreements<R: UnauthenticatedTransportRead>(
    reader: &R,
    party: &PublicKey,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Vec<crate::SignedSubscription>> {
    let path = format!("{}{}/", PAYKIT_AGREEMENTS_PATH, party);

    let entries: Vec<String> = reader
        .list_directory(party, &path)
        .await
        .unwrap_or_default();

    let mut agreements = Vec::new();
    for entry in entries {
        let full_path = format!("{}{}", path, entry);
        if let Ok(Some(content)) = reader.get(party, &full_path).await {
            if let Some(signed) =
                try_decrypt_signed_subscription(&content, &full_path, &entry, my_noise_sk)
            {
                agreements.push(signed);
            }
        }
    }

    Ok(agreements)
}

/// Discover a specific subscription agreement by ID.
pub async fn discover_subscription_agreement<R: UnauthenticatedTransportRead>(
    reader: &R,
    party: &PublicKey,
    subscription_id: &str,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Option<crate::SignedSubscription>> {
    let path = format!("{}{}/{}", PAYKIT_AGREEMENTS_PATH, party, subscription_id);

    match reader.get(party, &path).await {
        Ok(Some(content)) => {
            let signed =
                try_decrypt_signed_subscription(&content, &path, subscription_id, my_noise_sk);
            Ok(signed)
        }
        Ok(None) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to fetch agreement: {}", e)),
    }
}

/// Discover subscription cancellations for a party.
pub async fn discover_subscription_cancellations<R: UnauthenticatedTransportRead>(
    reader: &R,
    party: &PublicKey,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Vec<serde_json::Value>> {
    let path = format!("{}{}/", PAYKIT_CANCELLATIONS_PATH, party);

    let entries: Vec<String> = reader
        .list_directory(party, &path)
        .await
        .unwrap_or_default();

    let mut cancellations = Vec::new();
    for entry in entries {
        let full_path = format!("{}{}", path, entry);
        if let Ok(Some(content)) = reader.get(party, &full_path).await {
            if let Some(cancellation) =
                try_decrypt_cancellation(&content, &full_path, &entry, my_noise_sk)
            {
                cancellations.push(cancellation);
            }
        }
    }

    Ok(cancellations)
}

/// Decrypt an encrypted subscription proposal.
///
/// SECURITY: Only encrypted Sealed Blob v1 format is accepted.
fn try_decrypt_subscription_proposal(
    content: &str,
    subscriber_pubkey_z32: &str,
    proposal_id: &str,
    my_noise_sk: &[u8; 32],
) -> Option<crate::Subscription> {
    if !is_sealed_blob(content) {
        tracing::warn!(
            "SECURITY: Rejected plaintext subscription proposal {}. Only encrypted blobs accepted.",
            proposal_id
        );
        return None;
    }

    // Build canonical AAD
    let aad = match subscription_proposal_aad(subscriber_pubkey_z32, proposal_id) {
        Ok(aad) => aad,
        Err(e) => {
            tracing::warn!("Failed to build AAD for proposal {}: {}", proposal_id, e);
            return None;
        }
    };

    match sealed_blob_decrypt(my_noise_sk, content, &aad) {
        Ok(plaintext) => serde_json::from_slice(&plaintext).ok(),
        Err(e) => {
            tracing::warn!(
                "Failed to decrypt subscription proposal {}: {}",
                proposal_id,
                e
            );
            None
        }
    }
}

/// Decrypt an encrypted signed subscription agreement.
///
/// SECURITY: Only encrypted Sealed Blob v1 format is accepted.
/// AAD format: `paykit:v0:subscription_agreement:{path}:{subscription_id}` (matches manager.rs storage)
fn try_decrypt_signed_subscription(
    content: &str,
    path: &str,
    subscription_id: &str,
    my_noise_sk: &[u8; 32],
) -> Option<crate::SignedSubscription> {
    if !is_sealed_blob(content) {
        tracing::warn!(
            "SECURITY: Rejected plaintext agreement at {}. Only encrypted blobs accepted.",
            path
        );
        return None;
    }

    // AAD format matches store_signed_subscription in manager.rs
    let aad = format!("paykit:v0:subscription_agreement:{}:{}", path, subscription_id);
    match sealed_blob_decrypt(my_noise_sk, content, &aad) {
        Ok(plaintext) => serde_json::from_slice(&plaintext).ok(),
        Err(e) => {
            tracing::warn!("Failed to decrypt agreement at {}: {}", path, e);
            None
        }
    }
}

/// Decrypt an encrypted cancellation.
///
/// SECURITY: Only encrypted Sealed Blob v1 format is accepted.
/// AAD format: `paykit:v0:subscription_cancellation:{path}:{subscription_id}` (matches manager.rs storage)
fn try_decrypt_cancellation(
    content: &str,
    path: &str,
    subscription_id: &str,
    my_noise_sk: &[u8; 32],
) -> Option<serde_json::Value> {
    if !is_sealed_blob(content) {
        tracing::warn!(
            "SECURITY: Rejected plaintext cancellation at {}. Only encrypted blobs accepted.",
            path
        );
        return None;
    }

    // AAD format matches store_subscription_cancellation in manager.rs
    let aad = format!("paykit:v0:subscription_cancellation:{}:{}", path, subscription_id);
    match sealed_blob_decrypt(my_noise_sk, content, &aad) {
        Ok(plaintext) => serde_json::from_slice(&plaintext).ok(),
        Err(e) => {
            tracing::warn!("Failed to decrypt cancellation at {}: {}", path, e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Amount;
    use paykit_lib::MethodId;
    use std::str::FromStr;

    fn test_pubkey() -> PublicKey {
        let keypair = pkarr::Keypair::random();
        PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
    }

    #[test]
    fn test_published_request_creation() {
        let from = test_pubkey();
        let to = test_pubkey();
        let request = PaymentRequest::new(
            from,
            to,
            Amount::from_sats(1000),
            "SAT".to_string(),
            MethodId("lightning".to_string()),
        );

        let published = PublishedRequest::new(request);
        assert!(published.active);
        assert!(published.published_at > 0);
    }

    #[test]
    fn test_published_request_deactivate() {
        let from = test_pubkey();
        let to = test_pubkey();
        let request = PaymentRequest::new(
            from,
            to,
            Amount::from_sats(1000),
            "SAT".to_string(),
            MethodId("lightning".to_string()),
        );

        let mut published = PublishedRequest::new(request);
        assert!(published.active);

        published.deactivate();
        assert!(!published.active);
    }

    #[test]
    fn test_path_constants() {
        assert!(PAYKIT_REQUESTS_PATH.starts_with("/pub/paykit.app/"));
        assert!(PAYKIT_NOTIFICATIONS_PATH.starts_with("/pub/paykit.app/"));
    }
}
