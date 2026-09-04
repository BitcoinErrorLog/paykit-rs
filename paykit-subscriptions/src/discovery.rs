//! Payment Request Discovery
//!
//! This module provides functionality to publish and discover payment requests
//! via Pubky homeservers, enabling async payment request delivery.
//!
//! ## Path Format (v0)
//!
//! Requests are stored on the **sender's** homeserver at:
//! `/pub/paykit.app/v0/requests/{context_id}/{request_id}`
//!
//! Where `context_id = hex(sha256("paykit:v0:context:" + first_z32 + ":" + second_z32))`.
//!
//! ## Security
//!
//! Payment requests are encrypted using Paykit Sealed Blob v2 before storage
//! to prevent public exposure of payment details. All requests MUST be encrypted.
//! Plaintext storage is REJECTED for security reasons.
//!
//! ## Discovery
//!
//! Recipients poll known contacts and list their `.../{context_id}/` directory
//! to discover pending requests. Recipients cannot delete requests from sender
//! storage (deduplication is local-only).

use crate::PaymentRequest;
use paykit_lib::protocol::drop_transport::{DropHttp, OutboundTransport, ProtocolMessageKind};
use paykit_lib::protocol::{
    owner_peerid_bytes_from_z32, payment_request_aad, payment_request_path, payment_requests_dir,
    subscription_proposal_aad, subscription_proposal_path, subscription_proposals_dir,
    PURPOSE_REQUEST,
};
use paykit_lib::{HomeserverPublicStorageRead, HomeserverSessionStorage, PublicKey};
use pubky_crypto::sealed_blob::{
    is_sealed_blob, sealed_blob_decrypt, sealed_blob_decrypt_with_context,
    sealed_blob_encrypt_with_context,
};
use serde::{Deserialize, Serialize};

/// Path prefix for payment requests in Pubky storage (v0).
/// Use `payment_request_path()` or `payment_requests_dir()` for canonical paths.
pub const PAYKIT_REQUESTS_PATH: &str = "/pub/paykit.app/v0/requests/";

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
/// The request is stored at `/pub/paykit.app/v0/requests/{context_id}/{request_id}`
/// as a Paykit Sealed Blob v2 encrypted to the recipient's Noise endpoint public key.
///
/// # Path Format
///
/// `context_id = hex(sha256("paykit:v0:context:" + first_z32 + ":" + second_z32))`
///
/// This creates a per-peer-pair directory on the sender's storage, allowing
/// recipients to poll known contacts and list `.../{context_id}/` to discover requests.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the sender
/// * `sender_pubkey_z32` - Sender's z-base-32 pubkey (storage owner)
/// * `request` - The payment request to publish
/// * `recipient_noise_pk` - Recipient's Noise endpoint X25519 public key (32 bytes)
///
/// # Example
///
/// ```ignore
/// use paykit_subscriptions::discovery::publish_payment_request;
///
/// let request = PaymentRequest::new(from, to, amount, currency, method);
/// publish_payment_request(&transport, &sender_z32, &request, &recipient_noise_pk).await?;
/// ```
pub async fn publish_payment_request<T: HomeserverSessionStorage>(
    transport: &T,
    sender_pubkey_z32: &str,
    request: &PaymentRequest,
    recipient_noise_pk: &[u8; 32],
) -> crate::Result<()> {
    let (path, envelope) = seal_payment_request(sender_pubkey_z32, request, recipient_noise_pk)?;

    // Store encrypted blob on sender storage
    transport
        .put(&path, &envelope)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to publish request: {}", e))?;

    Ok(())
}

/// Publish a payment request through an explicit outbound route (W2b).
///
/// The payload (Sealed Blob, canonical path) is built exactly as for
/// [`publish_payment_request`]. When `outbound` is
/// [`OutboundTransport::Bonded`], the encrypted blob is sent over the
/// counterparty's Drop channel with purpose `pubky.molt.paykit.v1` and
/// nothing is written to any `/pub/` path; a failed bonded send is returned
/// as an error and never falls back to the public outbox. When `outbound`
/// is [`OutboundTransport::PublicOutbox`], the behavior is identical to
/// [`publish_payment_request`].
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the sender (used only on the
///   public route)
/// * `outbound` - The selected outbound route for this peer
/// * `sender_pubkey_z32` - Sender's z-base-32 pubkey (storage owner)
/// * `request` - The payment request to publish
/// * `recipient_noise_pk` - Recipient's Noise endpoint X25519 public key
pub async fn publish_payment_request_routed<T, H>(
    transport: &T,
    outbound: &mut OutboundTransport<'_, H>,
    sender_pubkey_z32: &str,
    request: &PaymentRequest,
    recipient_noise_pk: &[u8; 32],
) -> crate::Result<()>
where
    T: HomeserverSessionStorage,
    H: DropHttp,
{
    let (path, envelope) = seal_payment_request(sender_pubkey_z32, request, recipient_noise_pk)?;
    outbound
        .deliver(
            ProtocolMessageKind::Request,
            envelope.as_bytes(),
            || async {
                transport
                    .put(&path, &envelope)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to publish request: {}", e))
            },
        )
        .await
}

/// Build the canonical path and encrypted blob for a payment request.
///
/// Returns `(path, envelope)`: the public-outbox path
/// (`/pub/paykit.app/v0/requests/{context_id}/{request_id}`, also the AAD's
/// canonical path) and the Sealed Blob v2 encrypted to the recipient's
/// Noise endpoint public key.
fn seal_payment_request(
    sender_pubkey_z32: &str,
    request: &PaymentRequest,
    recipient_noise_pk: &[u8; 32],
) -> crate::Result<(String, String)> {
    let published = PublishedRequest::new(request.clone());
    let plaintext = serde_json::to_vec(&published)?;

    // Build canonical path using context_id
    let recipient_pubkey_z32 = request.to.to_string();
    let path = payment_request_path(
        sender_pubkey_z32,
        &recipient_pubkey_z32,
        &request.request_id,
    )
    .map_err(|e| anyhow::anyhow!("Invalid pubkey: {}", e))?;

    // Convert owner z32 to bytes for binary AAD (owner = sender for requests)
    let owner_peerid_bytes = owner_peerid_bytes_from_z32(sender_pubkey_z32)
        .map_err(|e| anyhow::anyhow!("Invalid owner pubkey: {}", e))?;

    // Encrypt using Sealed Blob v2 with spec-compliant binary AAD
    let envelope = sealed_blob_encrypt_with_context(
        recipient_noise_pk,
        &plaintext,
        &owner_peerid_bytes,
        &path,
        Some(PURPOSE_REQUEST),
    )
    .map_err(|e| anyhow::anyhow!("Failed to encrypt payment request: {}", e))?;

    Ok((path, envelope))
}

/// Discover payment requests from a sender addressed to me.
///
/// Lists the sender's `.../{context_id}/` directory and decrypts Sealed Blob v2
/// encrypted requests using the recipient's Noise secret key.
///
/// # Path Format
///
/// Requests are stored at: `/pub/paykit.app/v0/requests/{context_id}/{request_id}`
/// This function lists the `{context_id}` directory on the sender's storage.
///
/// # Arguments
///
/// * `reader` - Unauthenticated reader for Pubky storage
/// * `sender` - The sender's public key
/// * `sender_pubkey_z32` - The sender's z-base-32 pubkey (to compute context_id)
/// * `my_pubkey_z32` - The recipient's own z-base-32 pubkey (to compute context_id)
/// * `my_noise_sk` - Recipient's Noise endpoint X25519 secret key (32 bytes)
///
/// # Returns
///
/// A list of published payment requests from the sender addressed to me.
pub async fn discover_requests<R: HomeserverPublicStorageRead>(
    reader: &R,
    sender: &PublicKey,
    sender_pubkey_z32: &str,
    my_pubkey_z32: &str,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Vec<PublishedRequest>> {
    // Compute context_id directory
    let ctx_dir = payment_requests_dir(sender_pubkey_z32, my_pubkey_z32)
        .map_err(|e| anyhow::anyhow!("Invalid pubkey: {}", e))?;

    let entries = reader
        .list_directory(sender, &ctx_dir)
        .await
        .unwrap_or_else(|_| vec![]); // Empty if directory doesn't exist

    let mut requests = Vec::new();

    for entry in entries {
        // Build full path for this request
        let path = payment_request_path(sender_pubkey_z32, my_pubkey_z32, &entry)
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;

        if let Ok(Some(content)) = reader.get(sender, &path).await {
            // Decrypt sealed blob only (no plaintext fallback)
            if let Some(published) = try_decrypt_request(
                &content,
                sender_pubkey_z32,
                my_pubkey_z32,
                &entry,
                my_noise_sk,
            ) {
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
/// Decrypts Sealed Blob v2 encrypted requests using the recipient's Noise secret key.
///
/// # Arguments
///
/// * `reader` - Unauthenticated reader for Pubky storage
/// * `sender` - The sender's public key
/// * `sender_pubkey_z32` - The sender's z-base-32 pubkey (to compute context_id)
/// * `my_pubkey_z32` - The recipient's own z-base-32 pubkey (to compute context_id)
/// * `request_id` - The request ID
/// * `my_noise_sk` - Recipient's Noise endpoint X25519 secret key (32 bytes)
///
/// # Returns
///
/// The published request if found.
pub async fn discover_request<R: HomeserverPublicStorageRead>(
    reader: &R,
    sender: &PublicKey,
    sender_pubkey_z32: &str,
    my_pubkey_z32: &str,
    request_id: &str,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Option<PublishedRequest>> {
    // Build canonical path
    let path = payment_request_path(sender_pubkey_z32, my_pubkey_z32, request_id)
        .map_err(|e| anyhow::anyhow!("Invalid pubkey: {}", e))?;

    match reader.get(sender, &path).await {
        Ok(Some(content)) => {
            let published = try_decrypt_request(
                &content,
                sender_pubkey_z32,
                my_pubkey_z32,
                request_id,
                my_noise_sk,
            )
            .ok_or_else(|| anyhow::anyhow!("Failed to decrypt request (sealed blob only)"))?;
            Ok(Some(published))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to fetch request: {}", e)),
    }
}

/// Decrypt a sealed blob payment request.
///
/// Implements dual decryption per PUBKY_CRYPTO_SPEC v2.5 migration strategy:
/// 1. First try binary AAD (spec-compliant, new writes)
/// 2. Fallback to legacy string AAD for backward compatibility
///
/// SECURITY: Only encrypted Sealed Blob v2 format is accepted.
/// Plaintext storage is REJECTED for security reasons.
fn try_decrypt_request(
    content: &str,
    sender_pubkey_z32: &str,
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

    // Build canonical path for binary AAD
    let path = match payment_request_path(sender_pubkey_z32, recipient_pubkey_z32, request_id) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Failed to build path for request {}: {}", request_id, e);
            return None;
        }
    };

    // Convert owner z32 to bytes (owner = sender for requests)
    let owner_peerid_bytes = match owner_peerid_bytes_from_z32(sender_pubkey_z32) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!(
                "Failed to convert owner z32 for request {}: {}",
                request_id,
                e
            );
            // Cannot try binary AAD, but can still try legacy
            return try_decrypt_request_legacy(
                content,
                sender_pubkey_z32,
                recipient_pubkey_z32,
                request_id,
                my_noise_sk,
            );
        }
    };

    // Try binary AAD first (spec-compliant, new writes)
    match sealed_blob_decrypt_with_context(my_noise_sk, content, &owner_peerid_bytes, &path) {
        Ok(plaintext) => {
            return serde_json::from_slice(&plaintext).ok();
        }
        Err(_) => {
            // Fallback to legacy string AAD for backward compatibility
            tracing::debug!(
                "Binary AAD decryption failed for request {}, trying legacy",
                request_id
            );
        }
    }

    // Fallback to legacy string AAD
    try_decrypt_request_legacy(
        content,
        sender_pubkey_z32,
        recipient_pubkey_z32,
        request_id,
        my_noise_sk,
    )
}

/// Legacy decryption with string AAD format.
fn try_decrypt_request_legacy(
    content: &str,
    sender_pubkey_z32: &str,
    recipient_pubkey_z32: &str,
    request_id: &str,
    my_noise_sk: &[u8; 32],
) -> Option<PublishedRequest> {
    // Build legacy string AAD
    let aad = match payment_request_aad(
        sender_pubkey_z32,
        sender_pubkey_z32,
        recipient_pubkey_z32,
        request_id,
    ) {
        Ok(aad) => aad,
        Err(e) => {
            tracing::warn!(
                "Failed to build legacy AAD for request {}: {}",
                request_id,
                e
            );
            return None;
        }
    };

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
/// * `sender_pubkey_z32` - The sender's z-base-32 pubkey
/// * `recipient_pubkey_z32` - The recipient's z-base-32 pubkey
/// * `request_id` - The request ID to cancel
pub async fn cancel_payment_request<T: HomeserverSessionStorage>(
    transport: &T,
    sender_pubkey_z32: &str,
    recipient_pubkey_z32: &str,
    request_id: &str,
) -> crate::Result<()> {
    let path = payment_request_path(sender_pubkey_z32, recipient_pubkey_z32, request_id)
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
/// Sealed Blob v2 encrypted requests using the provided Noise secret key.
///
/// # Discovery Model
///
/// The poller iterates over known peers (contacts) and lists each peer's
/// `.../{context_id}/` directory to find requests addressed to me.
pub struct RequestDiscoveryPoller<R: HomeserverPublicStorageRead> {
    reader: R,
    known_peers: Vec<PublicKey>,
    last_poll: i64,
    poll_interval_secs: u64,
    /// My z-base-32 pubkey (to compute my scope directory)
    my_pubkey_z32: String,
    /// Noise endpoint secret key for decrypting encrypted requests
    noise_sk: [u8; 32],
}

impl<R: HomeserverPublicStorageRead> RequestDiscoveryPoller<R> {
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
            // Convert peer PublicKey to z32 for context_id computation
            let peer_z32 = peer.to_string();
            match discover_requests(
                &self.reader,
                peer,
                &peer_z32,
                &self.my_pubkey_z32,
                &self.noise_sk,
            )
            .await
            {
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
/// Lists the provider's `.../{context_id}/` directory and decrypts Sealed Blob v2
/// encrypted proposals using the subscriber's Noise secret key.
///
/// # Path Format
///
/// Proposals are stored at: `/pub/paykit.app/v0/subscriptions/proposals/{context_id}/{proposal_id}`
/// This function lists the `{context_id}` directory on the provider's storage.
///
/// # Arguments
///
/// * `reader` - Unauthenticated transport for reading
/// * `provider` - The provider's public key
/// * `provider_pubkey_z32` - The provider's z-base-32 encoded pubkey (to compute context_id)
/// * `my_pubkey_z32` - My z-base-32 encoded pubkey (to compute context_id)
/// * `my_noise_sk` - My Noise secret key for decryption
pub async fn discover_subscription_proposals<R: HomeserverPublicStorageRead>(
    reader: &R,
    provider: &PublicKey,
    provider_pubkey_z32: &str,
    my_pubkey_z32: &str,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Vec<crate::Subscription>> {
    // Compute context_id directory on provider storage
    let ctx_dir = subscription_proposals_dir(provider_pubkey_z32, my_pubkey_z32)
        .map_err(|e| anyhow::anyhow!("Invalid pubkey: {}", e))?;

    let entries: Vec<String> = reader
        .list_directory(provider, &ctx_dir)
        .await
        .unwrap_or_default();

    let mut proposals = Vec::new();
    for entry in entries {
        // Build full path for this proposal
        let full_path = subscription_proposal_path(provider_pubkey_z32, my_pubkey_z32, &entry)
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;

        if let Ok(Some(content)) = reader.get(provider, &full_path).await {
            if let Some(subscription) = try_decrypt_subscription_proposal(
                &content,
                provider_pubkey_z32,
                my_pubkey_z32,
                &entry,
                my_noise_sk,
            ) {
                proposals.push(subscription);
            }
        }
    }

    Ok(proposals)
}

/// Discover a specific subscription proposal by ID.
///
/// Decrypts Sealed Blob v2 encrypted proposals using the subscriber's Noise secret key.
///
/// # Arguments
///
/// * `reader` - Unauthenticated transport for reading
/// * `provider` - The provider's public key
/// * `provider_pubkey_z32` - The provider's z-base-32 encoded pubkey (to compute context_id)
/// * `my_pubkey_z32` - My z-base-32 encoded pubkey (to compute context_id)
/// * `proposal_id` - The proposal ID
/// * `my_noise_sk` - My Noise secret key for decryption
pub async fn discover_subscription_proposal<R: HomeserverPublicStorageRead>(
    reader: &R,
    provider: &PublicKey,
    provider_pubkey_z32: &str,
    my_pubkey_z32: &str,
    proposal_id: &str,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Option<crate::Subscription>> {
    // Build canonical path using context_id
    let path = subscription_proposal_path(provider_pubkey_z32, my_pubkey_z32, proposal_id)
        .map_err(|e| anyhow::anyhow!("Invalid pubkey: {}", e))?;

    match reader.get(provider, &path).await {
        Ok(Some(content)) => {
            let subscription = try_decrypt_subscription_proposal(
                &content,
                provider_pubkey_z32,
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
/// Decrypts Sealed Blob v2 encrypted agreements using the party's Noise secret key.
pub async fn discover_subscription_agreements<R: HomeserverPublicStorageRead>(
    reader: &R,
    party: &PublicKey,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Vec<crate::SignedSubscription>> {
    let path = format!("{}{}/", PAYKIT_AGREEMENTS_PATH, party);

    let entries: Vec<String> = reader
        .list_directory(party, &path)
        .await
        .unwrap_or_default();

    let party_z32 = party.to_z32();
    let mut agreements = Vec::new();
    for entry in entries {
        let full_path = format!("{}{}", path, entry);
        if let Ok(Some(content)) = reader.get(party, &full_path).await {
            if let Some(signed) = try_decrypt_signed_subscription(
                &content,
                &full_path,
                &entry,
                &party_z32,
                my_noise_sk,
            ) {
                agreements.push(signed);
            }
        }
    }

    Ok(agreements)
}

/// Discover a specific subscription agreement by ID.
pub async fn discover_subscription_agreement<R: HomeserverPublicStorageRead>(
    reader: &R,
    party: &PublicKey,
    subscription_id: &str,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Option<crate::SignedSubscription>> {
    let path = format!("{}{}/{}", PAYKIT_AGREEMENTS_PATH, party, subscription_id);
    let party_z32 = party.to_z32();

    match reader.get(party, &path).await {
        Ok(Some(content)) => {
            let signed = try_decrypt_signed_subscription(
                &content,
                &path,
                subscription_id,
                &party_z32,
                my_noise_sk,
            );
            Ok(signed)
        }
        Ok(None) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to fetch agreement: {}", e)),
    }
}

/// Discover subscription cancellations for a party.
pub async fn discover_subscription_cancellations<R: HomeserverPublicStorageRead>(
    reader: &R,
    party: &PublicKey,
    my_noise_sk: &[u8; 32],
) -> crate::Result<Vec<serde_json::Value>> {
    let path = format!("{}{}/", PAYKIT_CANCELLATIONS_PATH, party);
    let party_z32 = party.to_z32();

    let entries: Vec<String> = reader
        .list_directory(party, &path)
        .await
        .unwrap_or_default();

    let mut cancellations = Vec::new();
    for entry in entries {
        let full_path = format!("{}{}", path, entry);
        if let Ok(Some(content)) = reader.get(party, &full_path).await {
            if let Some(cancellation) =
                try_decrypt_cancellation(&content, &full_path, &entry, &party_z32, my_noise_sk)
            {
                cancellations.push(cancellation);
            }
        }
    }

    Ok(cancellations)
}

/// Decrypt an encrypted subscription proposal.
///
/// Implements dual decryption per PUBKY_CRYPTO_SPEC v2.5 migration strategy:
/// 1. First try binary AAD (spec-compliant, new writes)
/// 2. Fallback to legacy string AAD for backward compatibility
///
/// SECURITY: Only encrypted Sealed Blob v2 format is accepted.
fn try_decrypt_subscription_proposal(
    content: &str,
    provider_pubkey_z32: &str,
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

    // Build canonical path for binary AAD
    let path =
        match subscription_proposal_path(provider_pubkey_z32, subscriber_pubkey_z32, proposal_id) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to build path for proposal {}: {}", proposal_id, e);
                return None;
            }
        };

    // Convert owner z32 to bytes (owner = provider for proposals)
    let owner_peerid_bytes = match owner_peerid_bytes_from_z32(provider_pubkey_z32) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!(
                "Failed to convert owner z32 for proposal {}: {}",
                proposal_id,
                e
            );
            // Cannot try binary AAD, but can still try legacy
            return try_decrypt_subscription_proposal_legacy(
                content,
                provider_pubkey_z32,
                subscriber_pubkey_z32,
                proposal_id,
                my_noise_sk,
            );
        }
    };

    // Try binary AAD first (spec-compliant, new writes)
    match sealed_blob_decrypt_with_context(my_noise_sk, content, &owner_peerid_bytes, &path) {
        Ok(plaintext) => {
            return serde_json::from_slice(&plaintext).ok();
        }
        Err(_) => {
            // Fallback to legacy string AAD for backward compatibility
            tracing::debug!(
                "Binary AAD decryption failed for proposal {}, trying legacy",
                proposal_id
            );
        }
    }

    // Fallback to legacy string AAD
    try_decrypt_subscription_proposal_legacy(
        content,
        provider_pubkey_z32,
        subscriber_pubkey_z32,
        proposal_id,
        my_noise_sk,
    )
}

/// Legacy decryption with string AAD format for subscription proposals.
fn try_decrypt_subscription_proposal_legacy(
    content: &str,
    provider_pubkey_z32: &str,
    subscriber_pubkey_z32: &str,
    proposal_id: &str,
    my_noise_sk: &[u8; 32],
) -> Option<crate::Subscription> {
    // Build legacy string AAD with owner binding (owner = provider for proposals)
    let aad = match subscription_proposal_aad(
        provider_pubkey_z32, // owner = provider (stores on their homeserver)
        provider_pubkey_z32,
        subscriber_pubkey_z32,
        proposal_id,
    ) {
        Ok(aad) => aad,
        Err(e) => {
            tracing::warn!(
                "Failed to build legacy AAD for proposal {}: {}",
                proposal_id,
                e
            );
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
/// SECURITY: Only encrypted Sealed Blob v2 format is accepted.
/// Supports both binary AAD (PUBKY_CRYPTO_SPEC v2.5) and legacy string AAD formats.
fn try_decrypt_signed_subscription(
    content: &str,
    path: &str,
    subscription_id: &str,
    owner_pubkey_z32: &str,
    my_noise_sk: &[u8; 32],
) -> Option<crate::SignedSubscription> {
    if !is_sealed_blob(content) {
        tracing::warn!(
            "SECURITY: Rejected plaintext agreement at {}. Only encrypted blobs accepted.",
            path
        );
        return None;
    }

    // Try binary AAD first (spec-compliant)
    if let Ok(owner_bytes) = owner_peerid_bytes_from_z32(owner_pubkey_z32) {
        if let Ok(plaintext) =
            sealed_blob_decrypt_with_context(my_noise_sk, content, &owner_bytes, path)
        {
            return serde_json::from_slice(&plaintext).ok();
        }
        tracing::debug!(
            "Binary AAD decryption failed for agreement at {}, trying legacy",
            path
        );
    }

    // Fallback to legacy string AAD format
    let aad = format!(
        "paykit:v0:subscription_agreement:{}:{}",
        path, subscription_id
    );
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
/// SECURITY: Only encrypted Sealed Blob v2 format is accepted.
/// Supports both binary AAD (PUBKY_CRYPTO_SPEC v2.5) and legacy string AAD formats.
fn try_decrypt_cancellation(
    content: &str,
    path: &str,
    subscription_id: &str,
    owner_pubkey_z32: &str,
    my_noise_sk: &[u8; 32],
) -> Option<serde_json::Value> {
    if !is_sealed_blob(content) {
        tracing::warn!(
            "SECURITY: Rejected plaintext cancellation at {}. Only encrypted blobs accepted.",
            path
        );
        return None;
    }

    // Try binary AAD first (spec-compliant)
    if let Ok(owner_bytes) = owner_peerid_bytes_from_z32(owner_pubkey_z32) {
        if let Ok(plaintext) =
            sealed_blob_decrypt_with_context(my_noise_sk, content, &owner_bytes, path)
        {
            return serde_json::from_slice(&plaintext).ok();
        }
        tracing::debug!(
            "Binary AAD decryption failed for cancellation at {}, trying legacy",
            path
        );
    }

    // Fallback to legacy string AAD format
    let aad = format!(
        "paykit:v0:subscription_cancellation:{}:{}",
        path, subscription_id
    );
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
        assert!(PAYKIT_PROPOSALS_PATH.starts_with("/pub/paykit.app/"));
        assert!(PAYKIT_AGREEMENTS_PATH.starts_with("/pub/paykit.app/"));
        assert!(PAYKIT_CANCELLATIONS_PATH.starts_with("/pub/paykit.app/"));
    }
}
