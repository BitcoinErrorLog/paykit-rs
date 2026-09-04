#[cfg(test)]
use crate::NonceStore;
use crate::{
    signing::{self, Signature},
    NonceStorage, PaymentRequest, PaymentRequestResponse, RequestStatus, Result,
    SignedSubscription, Subscription, SubscriptionStorage,
};
use paykit_interactive::{PaykitInteractiveManager, PaykitNoiseChannel, PaykitNoiseMessage};
use paykit_lib::protocol::drop_transport::{
    receive_bonded, send_protocol_message, BondSession, DropClient, DropHttp, ProtocolMessageKind,
};
use paykit_lib::protocol::{
    owner_peerid_bytes_from_z32, subscription_proposal_path, PURPOSE_SUBSCRIPTION_PROPOSAL,
};
use paykit_lib::{HomeserverSessionStorage, PublicKey};
use pubky_crypto::molt::{Authenticity, Header, PurposeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Messages for subscription protocol
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum SubscriptionMessage {
    PaymentRequest(Box<PaymentRequest>),
    PaymentRequestResponse(Box<PaymentRequestResponse>),
    SubscriptionProposal(Box<Subscription>),
    SubscriptionAcceptance(Box<SignedSubscription>),
    SubscriptionCancellation {
        subscription_id: String,
        reason: Option<String>,
    },
}

/// A registered bonded route to one peer (W2b): the Molt `BondSession` and
/// the Drop relay client used to reach that peer's channels.
struct BondedRoute {
    session: BondSession,
    client: DropClient<Box<dyn DropHttp>>,
}

pub struct SubscriptionManager {
    storage: Arc<Box<dyn SubscriptionStorage>>,
    interactive: Arc<PaykitInteractiveManager>,
    pubky_session: Option<pubky::PubkySession>,
    nonce_storage: Arc<dyn NonceStorage>,
    /// Our Noise secret key for encryption/decryption
    my_noise_sk: Option<[u8; 32]>,
    /// Cache of peer Noise public keys (pubkey -> noise_pk)
    noise_pk_cache: Arc<RwLock<HashMap<String, [u8; 32]>>>,
    /// Bonded outbound routes per peer z32 (W2b). A peer present here gets
    /// protocol traffic over the Drop channel instead of the public outbox.
    bonded_outbounds: RwLock<HashMap<String, BondedRoute>>,
}

impl SubscriptionManager {
    /// Create a new subscription manager.
    ///
    /// # Arguments
    ///
    /// * `storage` - Storage backend for subscriptions and spending limits
    /// * `interactive` - Interactive payment manager
    /// * `nonce_storage` - Persistent nonce storage for replay attack prevention
    ///
    /// # Security
    ///
    /// The `nonce_storage` MUST be persistent across app restarts to prevent replay attacks.
    /// Use [`NonceStore`](crate::NonceStore) only for testing; production apps should use
    /// [`FileNonceStorage`](crate::FileNonceStorage) or a platform-specific
    /// implementation (SharedPreferences, UserDefaults, etc.).
    pub fn new(
        storage: Arc<Box<dyn SubscriptionStorage>>,
        interactive: Arc<PaykitInteractiveManager>,
        nonce_storage: Arc<dyn NonceStorage>,
    ) -> Self {
        Self {
            storage,
            interactive,
            pubky_session: None,
            nonce_storage,
            my_noise_sk: None,
            noise_pk_cache: Arc::new(RwLock::new(HashMap::new())),
            bonded_outbounds: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new subscription manager with in-memory nonce storage (for testing only).
    ///
    /// # Security Warning
    ///
    /// This uses in-memory nonce storage that resets on restart, making the app
    /// vulnerable to replay attacks. Only use for testing.
    #[cfg(test)]
    pub fn new_for_testing(
        storage: Arc<Box<dyn SubscriptionStorage>>,
        interactive: Arc<PaykitInteractiveManager>,
    ) -> Self {
        Self::new(storage, interactive, Arc::new(NonceStore::new()))
    }

    pub fn with_pubky_session(mut self, session: pubky::PubkySession) -> Self {
        self.pubky_session = Some(session);
        self
    }

    /// Configure the manager with a Noise secret key for encryption/decryption
    pub fn with_noise_keypair(mut self, noise_sk: [u8; 32]) -> Self {
        self.my_noise_sk = Some(noise_sk);
        self
    }

    /// Get the Noise secret key if configured
    pub fn noise_sk(&self) -> Option<&[u8; 32]> {
        self.my_noise_sk.as_ref()
    }

    /// Register a bonded outbound route for a peer (W2b).
    ///
    /// Once registered, protocol messages to `peer` (e.g. subscription
    /// proposals) are delivered over the peer's Drop channel with purpose
    /// `pubky.molt.paykit.v1` instead of the public homeserver outbox, and a
    /// failed bonded send fails closed (no silent public fallback). The
    /// caller owns session establishment and capability scope; `session`
    /// must be the `BondSession` for this exact peer.
    pub async fn add_bonded_outbound(
        &self,
        peer: &PublicKey,
        session: BondSession,
        client: DropClient<Box<dyn DropHttp>>,
    ) {
        self.bonded_outbounds
            .write()
            .await
            .insert(peer.to_string(), BondedRoute { session, client });
    }

    /// Remove a previously registered bonded route; returns `true` if one
    /// existed.
    pub async fn remove_bonded_outbound(&self, peer: &PublicKey) -> bool {
        self.bonded_outbounds
            .write()
            .await
            .remove(&peer.to_string())
            .is_some()
    }

    /// `true` when a bonded route is registered for `peer`.
    pub async fn has_bonded_outbound(&self, peer: &PublicKey) -> bool {
        self.bonded_outbounds
            .read()
            .await
            .contains_key(&peer.to_string())
    }

    /// Deliver an already-serialized protocol payload to `peer_z32` over its
    /// registered bonded route (W2b).
    ///
    /// One registry write guard spans the membership check and the dispatch,
    /// so a route removed after the caller selected bonded delivery surfaces
    /// as a clean error here — never a panic and never a silent fallback to
    /// the public outbox (fail closed). The guard is held across the send so
    /// registration state cannot change mid-dispatch.
    async fn deliver_bonded(
        &self,
        peer_z32: &str,
        kind: ProtocolMessageKind,
        body: &[u8],
    ) -> Result<()> {
        let mut routes = self.bonded_outbounds.write().await;
        let route = routes.get_mut(peer_z32).ok_or_else(|| {
            anyhow::anyhow!(
                "bonded route to {} is no longer registered; message not delivered (no public fallback)",
                peer_z32
            )
        })?;
        send_protocol_message(&mut route.session, kind, body, &route.client)
            .await
            .map_err(|e| anyhow::anyhow!("bonded send failed (no public fallback): {}", e))
    }

    /// Publish a payment request for async discovery (W2b).
    ///
    /// When a bonded route is registered for the recipient (see
    /// [`add_bonded_outbound`](Self::add_bonded_outbound)), the encrypted
    /// request is delivered over the recipient's Drop channel with purpose
    /// `pubky.molt.paykit.v1` and nothing is written to any `/pub/` path; a
    /// failed bonded send returns an error and never falls back to the
    /// public outbox. When no route is registered, the behavior is
    /// byte-identical to [`crate::discovery::publish_payment_request`].
    ///
    /// The payload (Sealed Blob, canonical path in the AAD) is identical on
    /// both routes, so the recipient decrypts it with the unchanged schema
    /// regardless of how it arrived.
    ///
    /// # Arguments
    ///
    /// * `transport` - Authenticated transport for the sender (used only
    ///   when no bonded route is registered)
    /// * `sender_pubkey_z32` - Sender's z-base-32 pubkey (storage owner)
    /// * `request` - The payment request to publish
    /// * `recipient_noise_pk` - Recipient's Noise endpoint X25519 public key
    pub async fn publish_payment_request<T: HomeserverSessionStorage>(
        &self,
        transport: &T,
        sender_pubkey_z32: &str,
        request: &PaymentRequest,
        recipient_noise_pk: &[u8; 32],
    ) -> Result<()> {
        let recipient_z32 = request.to.to_string();
        if self
            .bonded_outbounds
            .read()
            .await
            .contains_key(&recipient_z32)
        {
            let (_path, envelope) = crate::discovery::seal_payment_request(
                sender_pubkey_z32,
                request,
                recipient_noise_pk,
            )?;
            self.deliver_bonded(
                &recipient_z32,
                ProtocolMessageKind::Request,
                envelope.as_bytes(),
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to deliver bonded payment request (no public fallback): {}",
                    e
                )
            })
        } else {
            crate::discovery::publish_payment_request(
                transport,
                sender_pubkey_z32,
                request,
                recipient_noise_pk,
            )
            .await
        }
    }

    /// Store an encrypted ACK for `peer` (W2b): over the bonded Drop
    /// channel when a route is registered for `peer`, otherwise via the
    /// caller's legacy public-outbox write.
    ///
    /// - Bonded: the ACK bytes are delivered with
    ///   [`ProtocolMessageKind::Ack`] (declared `ExternallyAuthenticated`,
    ///   so the signed receipt inside the body stays independently
    ///   verifiable). `public_write` is never invoked, and a failed bonded
    ///   send returns an error — no public fallback.
    /// - No route: `public_write()` runs exactly as the caller would have
    ///   invoked it (byte-identical legacy behavior).
    ///
    /// # Arguments
    ///
    /// * `peer` - The counterparty the ACK is addressed to
    /// * `encrypted_ack` - The encrypted ACK as produced by
    ///   `paykit_lib::protocol::ack::encrypt_ack`
    /// * `public_write` - The caller's existing public-outbox write
    ///   (`/pub/paykit.app/v0/acks/…`), invoked only when no bonded route
    ///   is registered
    pub async fn store_encrypted_ack<F, Fut>(
        &self,
        peer: &PublicKey,
        encrypted_ack: &[u8],
        public_write: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let peer_z32 = peer.to_string();
        if self.bonded_outbounds.read().await.contains_key(&peer_z32) {
            self.deliver_bonded(&peer_z32, ProtocolMessageKind::Ack, encrypted_ack)
                .await
                .map_err(|e| {
                    anyhow::anyhow!("Failed to deliver bonded ACK (no public fallback): {}", e)
                })
        } else {
            public_write().await
        }
    }

    /// Poll every registered bonded route's receive channels and open every
    /// envelope that authenticates (W2b receive helper; the library-level
    /// caller of [`receive_bonded`]).
    ///
    /// Returns one `(peer_z32, header, authenticity, body)` tuple per opened
    /// envelope across all registered routes, keyed by the registry's peer
    /// z32. Bodies are returned untouched and the authenticity is exactly
    /// the AAD-bound declaration; successfully opened messages are
    /// ack-deleted by [`receive_bonded`], unopenable ones stay on the relay
    /// for TTL expiry.
    ///
    /// # Errors
    ///
    /// Mirroring [`receive_bonded`], a route whose every channel poll failed
    /// does not discard messages already collected from other routes; an
    /// error is returned only when **every** registered route failed to
    /// poll. An empty registry yields an empty result.
    pub async fn poll_bonded(
        &self,
        purposes: &[PurposeId],
    ) -> Result<Vec<(String, Header, Authenticity, Vec<u8>)>> {
        let mut routes = self.bonded_outbounds.write().await;
        let mut out = Vec::new();
        let mut first_error: Option<anyhow::Error> = None;
        let mut failed = 0usize;
        for (peer_z32, route) in routes.iter_mut() {
            match receive_bonded(
                std::slice::from_mut(&mut route.session),
                purposes,
                &route.client,
            )
            .await
            {
                Ok(messages) => {
                    out.extend(
                        messages
                            .into_iter()
                            .map(|(_peer, hdr, authenticity, body)| {
                                (peer_z32.clone(), hdr, authenticity, body)
                            }),
                    );
                }
                Err(e) => {
                    failed += 1;
                    if first_error.is_none() {
                        first_error = Some(anyhow::Error::new(e));
                    }
                }
            }
        }
        if !routes.is_empty() && failed == routes.len() {
            return Err(
                first_error.unwrap_or_else(|| anyhow::anyhow!("every bonded route poll failed"))
            );
        }
        Ok(out)
    }

    /// Validate payment request
    fn validate_request(&self, request: &PaymentRequest) -> Result<()> {
        if request.currency.is_empty() {
            anyhow::bail!("Currency cannot be empty");
        }
        if request.is_expired() {
            anyhow::bail!("Request has already expired");
        }
        Ok(())
    }

    /// Send payment request to peer (real-time via Noise if connected)
    pub async fn send_request(
        &self,
        channel: &mut dyn PaykitNoiseChannel,
        request: PaymentRequest,
    ) -> Result<()> {
        // Validate request
        self.validate_request(&request)?;

        // Save locally
        self.storage.save_request(&request).await?;

        // Send via Noise channel
        // For now, we'll send as a special PaykitNoiseMessage
        // In a full implementation, we'd extend PaykitNoiseMessage enum
        let _msg_json = serde_json::to_string(&SubscriptionMessage::PaymentRequest(Box::new(
            request.clone(),
        )))?;
        // Note: Real-time Noise delivery uses Ack as confirmation.
        // The actual payment request is delivered via stored encrypted blobs
        // on the sender's homeserver (see discovery.rs::publish_payment_request).
        channel.send(PaykitNoiseMessage::Ack).await?;

        // DEPRECATED: Plaintext notification storage has been removed for security.
        // Use `publish_payment_request` from discovery.rs for encrypted async storage.
        // The Noise channel above handles real-time delivery.

        Ok(())
    }

    // Deprecated plaintext notification and polling methods removed for pre-production hardening.

    /// Handle incoming payment request
    pub async fn handle_request(
        &self,
        request: PaymentRequest,
    ) -> Result<Option<PaymentRequestResponse>> {
        // Save request
        self.storage.save_request(&request).await?;

        // For Phase 1, always return Pending to require manual approval
        Ok(Some(PaymentRequestResponse::Pending {
            request_id: request.request_id,
            estimated_payment_time: None,
        }))
    }

    /// Manually respond to payment request
    pub async fn respond_to_request(
        &self,
        channel: &mut dyn PaykitNoiseChannel,
        request_id: &str,
        response: PaymentRequestResponse,
    ) -> Result<()> {
        // Update local status
        match &response {
            PaymentRequestResponse::Accepted { .. } => {
                self.storage
                    .update_request_status(request_id, RequestStatus::Accepted)
                    .await?;
            }
            PaymentRequestResponse::Declined { .. } => {
                self.storage
                    .update_request_status(request_id, RequestStatus::Declined)
                    .await?;
            }
            _ => {}
        }

        // Send response via Noise channel.
        // Ack confirms the response was processed; actual content is in the
        // stored encrypted blobs on the recipient's homeserver.
        channel.send(PaykitNoiseMessage::Ack).await?;

        Ok(())
    }

    /// Get storage reference (for testing and CLI integration)
    pub fn storage(&self) -> &Arc<Box<dyn SubscriptionStorage>> {
        &self.storage
    }

    // ============================================================
    // Phase 2: Subscription Agreements
    // ============================================================

    /// Propose a subscription to a peer
    pub async fn propose_subscription(
        &self,
        channel: &mut dyn PaykitNoiseChannel,
        subscription: Subscription,
        keypair: &pubky::Keypair,
    ) -> Result<()> {
        // Validate subscription
        subscription.validate()?;

        // Save locally as pending
        self.storage.save_subscription(&subscription).await?;

        // Generate unique nonce and sign the subscription as proposer
        let nonce = rand::random::<[u8; 32]>();
        let signature = signing::sign_subscription_ed25519(
            &subscription,
            keypair,
            &nonce,
            3600 * 24 * 7, // 7 days
        )?;

        // Record nonce
        if !self
            .nonce_storage
            .as_ref()
            .check_and_mark(&nonce, signature.expires_at)?
        {
            return Err(anyhow::anyhow!("Nonce already used"));
        }

        // Store the signature temporarily (we'll use it when acceptance comes back)
        // For now, just send the proposal

        // Send Noise Ack to signal proposal notification.
        // The actual encrypted proposal is stored on the provider's homeserver
        // and discovered via polling (see store_subscription_proposal below).
        channel.send(PaykitNoiseMessage::Ack).await?;

        // Deliver the proposal for async discovery: over the bonded Drop
        // channel when a BondSession is registered for the subscriber (W2b),
        // otherwise to the public homeserver outbox as before. Fail closed:
        // a failed bonded send returns an error and never falls back to the
        // public outbox. `deliver_bonded` holds one registry write guard
        // across the membership check and the dispatch, so a route removed
        // between the check here and the send surfaces as a clean error,
        // never a panic.
        let subscriber_z32 = subscription.subscriber.to_string();
        let is_bonded = self
            .bonded_outbounds
            .read()
            .await
            .contains_key(&subscriber_z32);
        if is_bonded {
            let (_path, envelope) = self.seal_subscription_proposal(&subscription).await?;
            self.deliver_bonded(
                &subscriber_z32,
                ProtocolMessageKind::Proposal,
                envelope.as_bytes(),
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to deliver bonded subscription proposal (no public fallback): {}",
                    e
                )
            })?;
        } else if let Some(session) = &self.pubky_session {
            self.store_subscription_proposal(session, &subscription)
                .await?;
        }

        Ok(())
    }

    /// Accept a subscription proposal
    pub async fn accept_subscription(
        &self,
        channel: &mut dyn PaykitNoiseChannel,
        subscription_id: &str,
        keypair: &pubky::Keypair,
        proposer_signature: Signature,
    ) -> Result<SignedSubscription> {
        // Load proposal
        let subscription = self
            .storage
            .get_subscription(subscription_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Subscription {} not found", subscription_id))?;

        // Validate subscription
        subscription.validate()?;

        // Verify proposer signature and check nonce
        if !signing::verify_signature_ed25519(&subscription, &proposer_signature)? {
            return Err(anyhow::anyhow!("Invalid proposer signature"));
        }
        if !self
            .nonce_storage
            .as_ref()
            .check_and_mark(&proposer_signature.nonce, proposer_signature.expires_at)?
        {
            return Err(anyhow::anyhow!(
                "Nonce already used (replay attack detected)"
            ));
        }

        // Generate unique nonce and sign as acceptor
        let nonce = rand::random::<[u8; 32]>();
        let acceptor_signature = signing::sign_subscription_ed25519(
            &subscription,
            keypair,
            &nonce,
            3600 * 24 * 7, // 7 days
        )?;
        if !self
            .nonce_storage
            .as_ref()
            .check_and_mark(&nonce, acceptor_signature.expires_at)?
        {
            return Err(anyhow::anyhow!("Nonce already used"));
        }

        // Create signed subscription
        let signed =
            SignedSubscription::new(subscription.clone(), proposer_signature, acceptor_signature);

        // Verify both signatures
        if !signed.verify_signatures()? {
            return Err(anyhow::anyhow!("Signature verification failed"));
        }

        // Save signed subscription
        self.storage.save_signed_subscription(&signed).await?;

        // Store in Pubky for persistence
        if let Some(session) = &self.pubky_session {
            self.store_signed_subscription(session, &signed).await?;
        }

        // Send acceptance via Noise channel
        channel.send(PaykitNoiseMessage::Ack).await?;

        Ok(signed)
    }

    /// Handle incoming subscription proposal
    pub async fn handle_subscription_proposal(&self, subscription: Subscription) -> Result<()> {
        // Validate
        subscription.validate()?;

        // Save for manual review/acceptance
        self.storage.save_subscription(&subscription).await?;

        Ok(())
    }

    /// Handle incoming subscription acceptance
    pub async fn handle_subscription_acceptance(&self, signed: SignedSubscription) -> Result<()> {
        // Verify signatures
        if !signed.verify_signatures()? {
            return Err(anyhow::anyhow!(
                "Invalid signatures on subscription acceptance"
            ));
        }

        // Check and record nonces
        if !self.nonce_storage.as_ref().check_and_mark(
            &signed.subscriber_signature.nonce,
            signed.subscriber_signature.expires_at,
        )? {
            return Err(anyhow::anyhow!(
                "Subscriber nonce already used (replay attack)"
            ));
        }
        if !self.nonce_storage.as_ref().check_and_mark(
            &signed.provider_signature.nonce,
            signed.provider_signature.expires_at,
        )? {
            return Err(anyhow::anyhow!(
                "Provider nonce already used (replay attack)"
            ));
        }

        // Save signed subscription
        self.storage.save_signed_subscription(&signed).await?;

        Ok(())
    }

    /// Cancel a subscription
    pub async fn cancel_subscription(
        &self,
        channel: &mut dyn PaykitNoiseChannel,
        subscription_id: &str,
        reason: Option<String>,
    ) -> Result<()> {
        // Load subscription
        let subscription = self
            .storage
            .get_signed_subscription(subscription_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Subscription {} not found", subscription_id))?;

        // Mark as cancelled locally (we could add status field)
        // For now, we just don't return it from list_active_subscriptions

        // Send cancellation message
        channel.send(PaykitNoiseMessage::Ack).await?;

        // Store cancellation in Pubky
        if let Some(session) = &self.pubky_session {
            self.store_subscription_cancellation(session, &subscription, reason)
                .await?;
        }

        Ok(())
    }

    /// List active subscriptions with a peer
    pub async fn list_subscriptions_with_peer(
        &self,
        peer: &PublicKey,
    ) -> Result<Vec<SignedSubscription>> {
        self.storage.list_subscriptions_with_peer(peer).await
    }

    /// List all active subscriptions
    pub async fn list_active_subscriptions(&self) -> Result<Vec<SignedSubscription>> {
        self.storage.list_active_subscriptions().await
    }

    // ============================================================
    // Private helper methods for Pubky storage
    // ============================================================

    /// Discover the Noise public key for a peer from their `/pub/paykit.app/v0/noise` endpoint.
    ///
    /// Uses caching to avoid repeated network requests.
    async fn discover_noise_pk(&self, pubkey: &PublicKey) -> Result<[u8; 32]> {
        let pubkey_str = pubkey.to_string();

        // Check cache first
        {
            let cache = self.noise_pk_cache.read().await;
            if let Some(pk) = cache.get(&pubkey_str) {
                return Ok(*pk);
            }
        }

        // Create public storage reader for fetching peer's noise endpoint
        let public_storage = pubky::PublicStorage::new()
            .map_err(|e| anyhow::anyhow!("Failed to create public storage: {}", e))?;

        let path = format!("pubky://{}/pub/paykit.app/v0/noise", pubkey_str);
        let response = public_storage
            .get(&path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch noise endpoint: {}", e))?;

        let content = response
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read noise endpoint response: {}", e))?;

        if content.is_empty() {
            anyhow::bail!("No noise endpoint found for {}", pubkey_str);
        }

        // Parse the noise endpoint JSON to extract public key
        let endpoint: serde_json::Value = serde_json::from_slice(&content)
            .map_err(|e| anyhow::anyhow!("Invalid noise endpoint JSON: {}", e))?;

        // Accept both "pubkey" (PaykitMobile FFI schema) and "public_key" (legacy)
        let pk_hex = endpoint
            .get("pubkey")
            .or_else(|| endpoint.get("public_key"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing pubkey in noise endpoint"))?;

        let pk_bytes = hex::decode(pk_hex)
            .map_err(|e| anyhow::anyhow!("Invalid noise public key hex: {}", e))?;

        if pk_bytes.len() != 32 {
            anyhow::bail!(
                "Invalid noise public key length: expected 32, got {}",
                pk_bytes.len()
            );
        }

        let mut pk_arr = [0u8; 32];
        pk_arr.copy_from_slice(&pk_bytes);

        // Cache the result
        {
            let mut cache = self.noise_pk_cache.write().await;
            cache.insert(pubkey_str, pk_arr);
        }

        Ok(pk_arr)
    }

    /// Store a subscription proposal encrypted to the subscriber.
    ///
    /// The proposal is encrypted using Paykit Sealed Blob v2 with spec-compliant
    /// binary AAD so only the intended subscriber can decrypt and read it.
    ///
    /// # Path Format (v0)
    ///
    /// Proposals are stored at: `/pub/paykit.app/v0/subscriptions/proposals/{context_id}/{proposal_id}`
    /// where `context_id = hex(sha256("paykit:v0:context:" || sorted(provider_z32, subscriber_z32)))`.
    /// Build the canonical path and encrypted blob for a subscription
    /// proposal (shared by the public-outbox and bonded delivery paths).
    ///
    /// Returns `(path, envelope)`: the public-outbox path
    /// (`/pub/paykit.app/v0/subscriptions/proposals/{context_id}/{proposal_id}`,
    /// also the AAD's canonical path) and the Sealed Blob v2 encrypted to
    /// the subscriber's Noise endpoint public key.
    async fn seal_subscription_proposal(
        &self,
        subscription: &Subscription,
    ) -> Result<(String, String)> {
        // Build canonical path using context_id
        let provider_pubkey_z32 = subscription.provider.to_string();
        let subscriber_pubkey_z32 = subscription.subscriber.to_string();
        let path = subscription_proposal_path(
            &provider_pubkey_z32,
            &subscriber_pubkey_z32,
            &subscription.subscription_id,
        )
        .map_err(|e| anyhow::anyhow!("Invalid pubkey: {}", e))?;

        // Discover subscriber's Noise public key for encryption
        let subscriber_noise_pk = self.discover_noise_pk(&subscription.subscriber).await?;

        // Serialize subscription
        let plaintext = serde_json::to_vec(&subscription)?;

        // Convert owner z32 to bytes for binary AAD (owner = provider)
        let owner_peerid_bytes = owner_peerid_bytes_from_z32(&provider_pubkey_z32)
            .map_err(|e| anyhow::anyhow!("Invalid owner pubkey: {}", e))?;

        // Encrypt using Sealed Blob v2 with spec-compliant binary AAD
        let envelope = pubky_crypto::sealed_blob::sealed_blob_encrypt_with_context(
            &subscriber_noise_pk,
            &plaintext,
            &owner_peerid_bytes,
            &path,
            Some(PURPOSE_SUBSCRIPTION_PROPOSAL),
        )
        .map_err(|e| anyhow::anyhow!("Failed to encrypt subscription proposal: {}", e))?;

        Ok((path, envelope))
    }

    async fn store_subscription_proposal(
        &self,
        session: &pubky::PubkySession,
        subscription: &Subscription,
    ) -> Result<()> {
        let (path, envelope) = self.seal_subscription_proposal(subscription).await?;

        // Store encrypted envelope on provider storage
        session
            .storage()
            .put(path, envelope.as_bytes().to_vec())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store proposal: {}", e))?;

        Ok(())
    }

    /// Store a signed subscription encrypted for both parties.
    ///
    /// Each party gets their own encrypted copy that only they can decrypt.
    ///
    /// Note: Agreements use a simpler path format as they're stored on both
    /// parties' own storage (no scope-based discovery needed).
    async fn store_signed_subscription(
        &self,
        session: &pubky::PubkySession,
        signed: &SignedSubscription,
    ) -> Result<()> {
        let plaintext = serde_json::to_vec(&signed)?;

        // Store for subscriber (encrypted to subscriber's Noise PK)
        let path_subscriber = format!(
            "/pub/paykit.app/v0/subscriptions/agreements/{}/{}",
            signed.subscription.subscriber, signed.subscription.subscription_id
        );
        let subscriber_noise_pk = self
            .discover_noise_pk(&signed.subscription.subscriber)
            .await?;
        let aad_subscriber = format!(
            "paykit:v0:subscription_agreement:{}:{}",
            path_subscriber, signed.subscription.subscription_id
        );
        let envelope_subscriber = pubky_crypto::sealed_blob::sealed_blob_encrypt(
            &subscriber_noise_pk,
            &plaintext,
            &aad_subscriber,
            Some("subscription_agreement"),
        )
        .map_err(|e| anyhow::anyhow!("Failed to encrypt subscription for subscriber: {}", e))?;

        session
            .storage()
            .put(path_subscriber, envelope_subscriber.as_bytes().to_vec())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store subscription for subscriber: {}", e))?;

        // Store for provider (encrypted to provider's Noise PK)
        let path_provider = format!(
            "/pub/paykit.app/v0/subscriptions/agreements/{}/{}",
            signed.subscription.provider, signed.subscription.subscription_id
        );
        let provider_noise_pk = self
            .discover_noise_pk(&signed.subscription.provider)
            .await?;
        let aad_provider = format!(
            "paykit:v0:subscription_agreement:{}:{}",
            path_provider, signed.subscription.subscription_id
        );
        let envelope_provider = pubky_crypto::sealed_blob::sealed_blob_encrypt(
            &provider_noise_pk,
            &plaintext,
            &aad_provider,
            Some("subscription_agreement"),
        )
        .map_err(|e| anyhow::anyhow!("Failed to encrypt subscription for provider: {}", e))?;

        session
            .storage()
            .put(path_provider, envelope_provider.as_bytes().to_vec())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store subscription for provider: {}", e))?;

        Ok(())
    }

    /// Store a subscription cancellation encrypted for both parties.
    ///
    /// Each party gets their own encrypted copy that only they can decrypt.
    async fn store_subscription_cancellation(
        &self,
        session: &pubky::PubkySession,
        subscription: &SignedSubscription,
        reason: Option<String>,
    ) -> Result<()> {
        let cancellation = serde_json::json!({
            "subscription_id": subscription.subscription.subscription_id,
            "reason": reason,
            "cancelled_at": chrono::Utc::now().timestamp(),
        });
        let plaintext = serde_json::to_vec(&cancellation)?;

        // Store for subscriber (encrypted to subscriber's Noise PK)
        let path_subscriber = format!(
            "/pub/paykit.app/v0/subscriptions/cancellations/{}/{}",
            subscription.subscription.subscriber, subscription.subscription.subscription_id
        );
        let subscriber_noise_pk = self
            .discover_noise_pk(&subscription.subscription.subscriber)
            .await?;
        let aad_subscriber = format!(
            "paykit:v0:subscription_cancellation:{}:{}",
            path_subscriber, subscription.subscription.subscription_id
        );
        let envelope_subscriber = pubky_crypto::sealed_blob::sealed_blob_encrypt(
            &subscriber_noise_pk,
            &plaintext,
            &aad_subscriber,
            Some("subscription_cancellation"),
        )
        .map_err(|e| anyhow::anyhow!("Failed to encrypt cancellation for subscriber: {}", e))?;

        session
            .storage()
            .put(path_subscriber, envelope_subscriber.as_bytes().to_vec())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store cancellation for subscriber: {}", e))?;

        // Store for provider (encrypted to provider's Noise PK)
        let path_provider = format!(
            "/pub/paykit.app/v0/subscriptions/cancellations/{}/{}",
            subscription.subscription.provider, subscription.subscription.subscription_id
        );
        let provider_noise_pk = self
            .discover_noise_pk(&subscription.subscription.provider)
            .await?;
        let aad_provider = format!(
            "paykit:v0:subscription_cancellation:{}:{}",
            path_provider, subscription.subscription.subscription_id
        );
        let envelope_provider = pubky_crypto::sealed_blob::sealed_blob_encrypt(
            &provider_noise_pk,
            &plaintext,
            &aad_provider,
            Some("subscription_cancellation"),
        )
        .map_err(|e| anyhow::anyhow!("Failed to encrypt cancellation for provider: {}", e))?;

        session
            .storage()
            .put(path_provider, envelope_provider.as_bytes().to_vec())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store cancellation for provider: {}", e))?;

        Ok(())
    }

    // ============================================================
    // Phase 3: Auto-Pay Automation
    // ============================================================

    /// Check if payment request should be auto-paid
    pub async fn should_autopay(&self, request: &PaymentRequest) -> Result<bool> {
        // Check if request is from a valid subscription
        let subscription = self.find_matching_subscription(request).await?;

        if let Some(sub) = subscription {
            // Check auto-pay rule
            let rule = self
                .storage
                .get_autopay_rule(&sub.subscription.subscription_id)
                .await?;

            if let Some(rule) = rule {
                if !rule.enabled {
                    return Ok(false);
                }

                // Check amount limits
                if !rule.is_amount_within_limit(&request.amount) {
                    return Ok(false);
                }

                // Check peer spending limits
                if !self.check_peer_limits(request).await? {
                    return Ok(false);
                }

                // Check if manual confirmation required
                if rule.require_confirmation {
                    return Ok(false);
                }

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Find subscription matching payment request
    async fn find_matching_subscription(
        &self,
        request: &PaymentRequest,
    ) -> Result<Option<SignedSubscription>> {
        // Load all subscriptions with this peer
        let subs = self
            .storage
            .list_subscriptions_with_peer(&request.from)
            .await?;

        // Find active subscription matching terms
        for sub in subs {
            if sub.is_active() && self.matches_subscription_terms(&sub, request) {
                return Ok(Some(sub));
            }
        }

        Ok(None)
    }

    /// Check if request matches subscription terms
    fn matches_subscription_terms(
        &self,
        subscription: &SignedSubscription,
        request: &PaymentRequest,
    ) -> bool {
        let terms = &subscription.subscription.terms;

        // Method must match
        if request.method != terms.method {
            return false;
        }

        // Currency must match
        if request.currency != terms.currency {
            return false;
        }

        // Amount must match (or be less than max)
        if request.amount != terms.amount {
            // Check if within max_amount_per_period
            if let Some(ref max) = terms.max_amount_per_period {
                if !request.amount.is_within_limit(max) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check if payment is due according to frequency
        self.is_payment_due(subscription, request.created_at)
    }

    /// Check if payment is due based on subscription frequency
    fn is_payment_due(&self, _subscription: &SignedSubscription, _now: i64) -> bool {
        // For Phase 3, we'll assume payment is due if the request matches
        // In a full implementation, this would check the last payment time
        // and compare against the subscription frequency
        true // Simplified for now
    }

    /// Check peer spending limits
    async fn check_peer_limits(&self, request: &PaymentRequest) -> Result<bool> {
        // Get peer spending limit
        if let Some(mut limit) = self.storage.get_peer_limit(&request.from).await? {
            // Check if limit needs reset
            if limit.should_reset() {
                limit.reset();
                self.storage.save_peer_limit(&limit).await?;
            }

            // Check if payment would exceed limit
            if limit.would_exceed_limit(&request.amount) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Execute auto-payment with atomic spending limit enforcement
    ///
    /// # Security
    ///
    /// Uses [`SpendingGuard`](crate::SpendingGuard) for panic-safe spending limit enforcement.
    /// The guard automatically rolls back the reservation if:
    /// - The payment fails and we return early
    /// - A panic occurs during payment execution
    /// - The future is dropped
    ///
    /// This prevents TOCTOU race conditions and spending limit leaks.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn execute_autopay<C: PaykitNoiseChannel>(
        &self,
        channel: &mut C,
        request: PaymentRequest,
        local_pk: &PublicKey,
    ) -> Result<paykit_interactive::PaykitReceipt> {
        use crate::SpendingGuard;

        // Atomic check-and-reserve spending limit
        let reservation = self
            .storage
            .try_reserve_spending(&request.from, &request.amount)
            .await?;

        // Wrap in SpendingGuard for panic-safe rollback
        let guard = SpendingGuard::new(self.storage.clone(), reservation);

        // Create provisional receipt
        let provisional_receipt = paykit_interactive::PaykitReceipt::new(
            format!("autopay_{}", request.request_id),
            local_pk.clone(),
            request.from.clone(),
            request.method.clone(),
            Some(request.amount.to_string()),
            Some(request.currency.clone()),
            request.metadata.clone(),
        );

        // Try to execute payment
        let payment_result = self
            .interactive
            .initiate_payment(channel, provisional_receipt)
            .await;

        // Commit or rollback based on result
        match payment_result {
            Ok(receipt) => {
                // Payment succeeded - commit the reservation
                guard.commit().await?;
                self.storage
                    .update_request_status(&request.request_id, RequestStatus::Fulfilled)
                    .await?;
                Ok(receipt)
            }
            Err(e) => {
                // Payment failed - guard auto-rolls-back on drop
                // We can also explicitly rollback for clarity:
                let _ = guard.rollback().await;
                Err(e.into())
            }
        }
    }

    /// Update spending limits after payment
    #[allow(dead_code)]
    async fn update_spending_limits(&self, request: &PaymentRequest) -> Result<()> {
        if let Some(mut limit) = self.storage.get_peer_limit(&request.from).await? {
            limit.add_spent(&request.amount)?;
            self.storage.save_peer_limit(&limit).await?;
        }
        Ok(())
    }

    /// Get or create auto-pay rule for subscription
    pub async fn get_or_create_autopay_rule(
        &self,
        subscription_id: &str,
    ) -> Result<crate::AutoPayRule> {
        // Try to get existing rule
        if let Some(rule) = self.storage.get_autopay_rule(subscription_id).await? {
            return Ok(rule);
        }

        // Create default rule
        let subscription = self
            .storage
            .get_signed_subscription(subscription_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Subscription not found"))?;

        let rule = crate::AutoPayRule::new(
            subscription_id.to_string(),
            subscription.subscription.provider.clone(),
            subscription.subscription.terms.method.clone(),
        );

        // Don't save yet - let user configure it first
        Ok(rule)
    }

    /// Enable auto-pay for a subscription
    pub async fn enable_autopay(
        &self,
        subscription_id: &str,
        rule: crate::AutoPayRule,
    ) -> Result<()> {
        // Validate subscription exists
        let _subscription = self
            .storage
            .get_signed_subscription(subscription_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Subscription not found"))?;

        // Validate rule
        rule.validate()?;

        // Save rule
        self.storage.save_autopay_rule(&rule).await?;

        Ok(())
    }

    /// Disable auto-pay for a subscription
    pub async fn disable_autopay(&self, subscription_id: &str) -> Result<()> {
        if let Some(mut rule) = self.storage.get_autopay_rule(subscription_id).await? {
            rule.enabled = false;
            self.storage.save_autopay_rule(&rule).await?;
        }
        Ok(())
    }

    /// Set spending limit for a peer
    pub async fn set_peer_spending_limit(
        &self,
        _peer: &PublicKey,
        limit: crate::PeerSpendingLimit,
    ) -> Result<()> {
        self.storage.save_peer_limit(&limit).await?;
        Ok(())
    }

    /// Get spending limit for a peer
    pub async fn get_peer_spending_limit(
        &self,
        peer: &PublicKey,
    ) -> Result<Option<crate::PeerSpendingLimit>> {
        self.storage.get_peer_limit(peer).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{storage::FileSubscriptionStorage, Amount, PaymentFrequency, SubscriptionTerms};
    use paykit_interactive::{PaykitInteractiveManager, PaykitStorage, ReceiptGenerator};
    use paykit_lib::protocol::drop_transport::{receive_bonded, DropHttp};
    use paykit_lib::{HomeserverSessionStorage, MethodId, PublicKey};
    use pubky_crypto::molt::{
        derive_bond, derive_pair_secret, pair_public, Authenticity, Bond, BondRecord, PairPublic,
        PeerId, PurposeId,
    };
    use std::str::FromStr;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Mutex;
    use tempfile::tempdir;

    // Mock implementations
    struct MockStorage;
    struct MockGenerator;

    #[async_trait::async_trait]
    impl PaykitStorage for MockStorage {
        async fn save_receipt(
            &self,
            _receipt: &paykit_interactive::PaykitReceipt,
        ) -> paykit_interactive::Result<()> {
            Ok(())
        }
        async fn get_receipt(
            &self,
            _id: &str,
        ) -> paykit_interactive::Result<Option<paykit_interactive::PaykitReceipt>> {
            Ok(None)
        }
        async fn save_private_endpoint(
            &self,
            _peer: &PublicKey,
            _method: &MethodId,
            _endpoint: &str,
        ) -> paykit_interactive::Result<()> {
            Ok(())
        }
        async fn get_private_endpoint(
            &self,
            _peer: &PublicKey,
            _method: &MethodId,
        ) -> paykit_interactive::Result<Option<String>> {
            Ok(None)
        }
        async fn list_receipts(
            &self,
        ) -> paykit_interactive::Result<Vec<paykit_interactive::PaykitReceipt>> {
            Ok(Vec::new())
        }
        async fn list_private_endpoints_for_peer(
            &self,
            _peer: &PublicKey,
        ) -> paykit_interactive::Result<Vec<(MethodId, String)>> {
            Ok(Vec::new())
        }
        async fn remove_private_endpoint(
            &self,
            _peer: &PublicKey,
            _method: &MethodId,
        ) -> paykit_interactive::Result<()> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl ReceiptGenerator for MockGenerator {
        async fn generate_receipt(
            &self,
            request: &paykit_interactive::PaykitReceipt,
        ) -> paykit_interactive::Result<paykit_interactive::PaykitReceipt> {
            Ok(request.clone())
        }
    }

    struct MockChannel;

    #[async_trait::async_trait]
    impl PaykitNoiseChannel for MockChannel {
        async fn send(&mut self, _msg: PaykitNoiseMessage) -> paykit_interactive::Result<()> {
            Ok(())
        }
        async fn recv(&mut self) -> paykit_interactive::Result<PaykitNoiseMessage> {
            Ok(PaykitNoiseMessage::Ack)
        }
    }

    fn test_pubkey() -> PublicKey {
        let keypair = pkarr::Keypair::random();
        PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
    }

    #[tokio::test]
    async fn test_send_request() {
        let temp_dir = tempdir().unwrap();
        let storage: Arc<Box<dyn SubscriptionStorage>> = Arc::new(Box::new(
            FileSubscriptionStorage::new(temp_dir.path().to_path_buf()).unwrap(),
        ));

        let mock_storage: Arc<Box<dyn PaykitStorage>> = Arc::new(Box::new(MockStorage));
        let mock_generator: Arc<Box<dyn ReceiptGenerator>> = Arc::new(Box::new(MockGenerator));

        let interactive = Arc::new(PaykitInteractiveManager::new(mock_storage, mock_generator));
        let manager = SubscriptionManager::new_for_testing(storage.clone(), interactive);

        let from = test_pubkey();
        let to = test_pubkey();
        let request = PaymentRequest::new(
            from,
            to,
            Amount::from_sats(1000),
            "SAT".to_string(),
            MethodId("lightning".to_string()),
        );

        let mut channel = MockChannel;
        manager
            .send_request(&mut channel, request.clone())
            .await
            .unwrap();

        // Verify request was saved
        let saved = storage.get_request(&request.request_id).await.unwrap();
        assert!(saved.is_some());
    }

    #[tokio::test]
    async fn test_handle_request() {
        let temp_dir = tempdir().unwrap();
        let storage: Arc<Box<dyn SubscriptionStorage>> = Arc::new(Box::new(
            FileSubscriptionStorage::new(temp_dir.path().to_path_buf()).unwrap(),
        ));

        let mock_storage: Arc<Box<dyn PaykitStorage>> = Arc::new(Box::new(MockStorage));
        let mock_generator: Arc<Box<dyn ReceiptGenerator>> = Arc::new(Box::new(MockGenerator));

        let interactive = Arc::new(PaykitInteractiveManager::new(mock_storage, mock_generator));
        let manager = SubscriptionManager::new_for_testing(storage.clone(), interactive);

        let from = test_pubkey();
        let to = test_pubkey();
        let request = PaymentRequest::new(
            from,
            to,
            Amount::from_sats(1000),
            "SAT".to_string(),
            MethodId("lightning".to_string()),
        );

        let response = manager.handle_request(request.clone()).await.unwrap();
        assert!(response.is_some());

        // Verify request was saved
        let saved = storage.get_request(&request.request_id).await.unwrap();
        assert!(saved.is_some());
    }

    #[tokio::test]
    async fn test_validate_request() {
        let temp_dir = tempdir().unwrap();
        let storage: Arc<Box<dyn SubscriptionStorage>> = Arc::new(Box::new(
            FileSubscriptionStorage::new(temp_dir.path().to_path_buf()).unwrap(),
        ));

        let mock_storage: Arc<Box<dyn PaykitStorage>> = Arc::new(Box::new(MockStorage));
        let mock_generator: Arc<Box<dyn ReceiptGenerator>> = Arc::new(Box::new(MockGenerator));

        let interactive = Arc::new(PaykitInteractiveManager::new(mock_storage, mock_generator));
        let manager = SubscriptionManager::new_for_testing(storage, interactive);

        let from = test_pubkey();
        let to = test_pubkey();

        // Test empty currency
        let mut request = PaymentRequest::new(
            from.clone(),
            to.clone(),
            Amount::from_sats(1000),
            "".to_string(),
            MethodId("lightning".to_string()),
        );
        assert!(manager.validate_request(&request).is_err());

        // Test expired request
        request.currency = "SAT".to_string();
        request.expires_at = Some(chrono::Utc::now().timestamp() - 3600);
        assert!(manager.validate_request(&request).is_err());

        // Test valid request
        request.expires_at = None;
        assert!(manager.validate_request(&request).is_ok());
    }

    // ============================================================
    // W2b: bonded proposal delivery
    // ============================================================

    /// In-process mock of the S8 Drop relay (test-only).
    /// One stored relay message: (cursor, timestamp, body).
    type StoredMessage = (u64, u64, Vec<u8>);

    struct StubDropRelay {
        channels: Mutex<HashMap<String, Vec<StoredMessage>>>,
        next_cursor: AtomicU64,
        fail_writes: AtomicBool,
        fail_reads: AtomicBool,
    }

    impl StubDropRelay {
        fn new() -> Self {
            StubDropRelay {
                channels: Mutex::new(HashMap::new()),
                next_cursor: AtomicU64::new(1),
                fail_writes: AtomicBool::new(false),
                fail_reads: AtomicBool::new(false),
            }
        }

        fn channel_key(url: &str) -> paykit_lib::Result<String> {
            let path =
                url.split("/drop/")
                    .nth(1)
                    .ok_or_else(|| paykit_lib::PaykitError::InvalidData {
                        field: "url".into(),
                        reason: "missing /drop/ prefix".into(),
                    })?;
            let channel = path.split(['?', '/']).next().unwrap_or("");
            use base64::Engine;
            let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(channel)
                .map_err(|_| paykit_lib::PaykitError::InvalidData {
                    field: "channel".into(),
                    reason: "invalid base64url".into(),
                })?;
            if decoded.len() != 32 {
                return Err(paykit_lib::PaykitError::InvalidData {
                    field: "channel".into(),
                    reason: "must decode to 32 bytes".into(),
                });
            }
            Ok(channel.to_string())
        }

        fn message_count(&self) -> usize {
            self.channels
                .lock()
                .expect("lock")
                .values()
                .map(Vec::len)
                .sum()
        }
    }

    /// Local `Arc` wrapper so `DropHttp` can be implemented (orphan rule).
    #[derive(Clone)]
    struct SharedRelay(Arc<StubDropRelay>);

    #[async_trait::async_trait]
    impl DropHttp for SharedRelay {
        async fn http_put(&self, url: &str, body: Vec<u8>) -> paykit_lib::Result<u64> {
            if self.0.fail_writes.load(Ordering::SeqCst) {
                return Err(paykit_lib::PaykitError::Transport(
                    "relay write failure".into(),
                ));
            }
            let key = StubDropRelay::channel_key(url)?;
            let cursor = self.0.next_cursor.fetch_add(1, Ordering::SeqCst);
            self.0
                .channels
                .lock()
                .expect("lock")
                .entry(key)
                .or_default()
                .push((cursor, 1_700_000_000, body));
            Ok(cursor)
        }

        async fn http_get(
            &self,
            url: &str,
            _max_response_bytes: usize,
        ) -> paykit_lib::Result<Vec<u8>> {
            if self.0.fail_reads.load(Ordering::SeqCst) {
                return Err(paykit_lib::PaykitError::Transport(
                    "relay read failure".into(),
                ));
            }
            let key = StubDropRelay::channel_key(url)?;
            let messages = self
                .0
                .channels
                .lock()
                .expect("lock")
                .get(&key)
                .cloned()
                .unwrap_or_default();
            let items: Vec<serde_cbor::Value> = messages
                .into_iter()
                .map(|(cursor, ts, body)| {
                    serde_cbor::Value::Map(
                        [
                            (
                                serde_cbor::Value::Integer(0.into()),
                                serde_cbor::Value::Integer(cursor as i128),
                            ),
                            (
                                serde_cbor::Value::Integer(1.into()),
                                serde_cbor::Value::Integer(ts as i128),
                            ),
                            (
                                serde_cbor::Value::Integer(2.into()),
                                serde_cbor::Value::Bytes(body),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    )
                })
                .collect();
            serde_cbor::to_vec(&serde_cbor::Value::Array(items))
                .map_err(|e| paykit_lib::PaykitError::Serialization(e.to_string()))
        }

        async fn http_delete(&self, url: &str) -> paykit_lib::Result<()> {
            let key = StubDropRelay::channel_key(url)?;
            let cursor_str = url.rsplit('/').next().unwrap_or("");
            let cursor: u64 =
                cursor_str
                    .parse()
                    .map_err(|_| paykit_lib::PaykitError::InvalidData {
                        field: "cursor".into(),
                        reason: "must be an unsigned integer".into(),
                    })?;
            let mut channels = self.0.channels.lock().expect("lock");
            if let Some(messages) = channels.get_mut(&key) {
                messages.retain(|(c, _, _)| *c != cursor);
            }
            Ok(())
        }
    }

    fn bonded_pair() -> (PeerId, PeerId, BondSession, BondSession) {
        let provider = PeerId([0x01; 32]);
        let subscriber = PeerId([0x02; 32]);
        let sk_p = derive_pair_secret(&[0x11; 32], &subscriber).expect("pair secret");
        let sk_s = derive_pair_secret(&[0x22; 32], &provider).expect("pair secret");
        let pk_p = pair_public(&sk_p);
        let pk_s = pair_public(&sk_s);
        let bond_p: Bond = derive_bond(&provider, &sk_p, &subscriber, &pk_s).expect("bond");
        let bond_s: Bond = derive_bond(&subscriber, &sk_s, &provider, &pk_p).expect("bond");
        let record = |peer: PeerId, pair_pk_peer: PairPublic| BondRecord {
            peer,
            pair_pk_peer,
            epoch_secs: 86_400,
            relays: vec!["http://relay.test".into()],
        };
        (
            provider,
            subscriber,
            BondSession::new(&provider, subscriber, bond_p, record(subscriber, pk_s)),
            BondSession::new(&subscriber, provider, bond_s, record(provider, pk_p)),
        )
    }

    fn test_subscription(subscriber: &PublicKey, provider: &PublicKey) -> Subscription {
        Subscription::new(
            subscriber.clone(),
            provider.clone(),
            SubscriptionTerms::new(
                Amount::from_sats(1000),
                "SAT".to_string(),
                PaymentFrequency::Monthly { day_of_month: 1 },
                MethodId("lightning".to_string()),
                "test subscription".to_string(),
            ),
        )
    }

    fn test_manager(
        temp_dir: &tempfile::TempDir,
    ) -> (SubscriptionManager, Arc<Box<dyn SubscriptionStorage>>) {
        let storage: Arc<Box<dyn SubscriptionStorage>> = Arc::new(Box::new(
            FileSubscriptionStorage::new(temp_dir.path().to_path_buf()).unwrap(),
        ));
        let mock_storage: Arc<Box<dyn PaykitStorage>> = Arc::new(Box::new(MockStorage));
        let mock_generator: Arc<Box<dyn ReceiptGenerator>> = Arc::new(Box::new(MockGenerator));
        let interactive = Arc::new(PaykitInteractiveManager::new(mock_storage, mock_generator));
        let manager = SubscriptionManager::new_for_testing(storage.clone(), interactive);
        (manager, storage)
    }

    #[tokio::test]
    async fn test_propose_subscription_bonded_skips_public_outbox() {
        let temp_dir = tempdir().unwrap();
        let (manager, _storage) = test_manager(&temp_dir);

        let provider = test_pubkey();
        let subscriber = test_pubkey();
        let (_provider_id, _subscriber_id, provider_session, mut subscriber_session) =
            bonded_pair();

        let relay = SharedRelay(Arc::new(StubDropRelay::new()));
        let client: DropClient<Box<dyn DropHttp>> = DropClient::new(
            "http://relay.test",
            Box::new(relay.clone()) as Box<dyn DropHttp>,
        )
        .unwrap();
        manager
            .add_bonded_outbound(&subscriber, provider_session, client)
            .await;
        assert!(manager.has_bonded_outbound(&subscriber).await);

        // The proposal is encrypted to the subscriber's Noise key; seed the
        // cache so no network lookup is attempted.
        let (noise_sk, noise_pk) = pubky_crypto::sealed_blob::x25519_generate_keypair();
        manager
            .noise_pk_cache
            .write()
            .await
            .insert(subscriber.to_string(), noise_pk);

        let subscription = test_subscription(&subscriber, &provider);
        let keypair = pkarr::Keypair::random();
        let mut channel = MockChannel;
        manager
            .propose_subscription(&mut channel, subscription.clone(), &keypair)
            .await
            .expect("bonded propose");

        // The manager holds no pubky session at all, so no `/pub/` write is
        // even possible; the proposal reached the Drop relay instead.
        assert!(manager.pubky_session.is_none());
        assert_eq!(relay.0.message_count(), 1);

        // The subscriber opens it over its own receive channel; the body is
        // the same encrypted blob the public path would have stored.
        let sub_client: DropClient<SharedRelay> =
            DropClient::new("http://relay.test", relay.clone()).unwrap();
        let received = receive_bonded(
            std::slice::from_mut(&mut subscriber_session),
            &[PurposeId::paykit()],
            &sub_client,
        )
        .await
        .expect("subscriber receive");
        assert_eq!(received.len(), 1);
        let body = String::from_utf8(received[0].3.clone()).expect("blob is text");
        assert!(pubky_crypto::sealed_blob::is_sealed_blob(&body));

        // ...and it decrypts to exactly the proposed subscription.
        let provider_z32 = provider.to_string();
        let subscriber_z32 = subscriber.to_string();
        let path = subscription_proposal_path(
            &provider_z32,
            &subscriber_z32,
            &subscription.subscription_id,
        )
        .expect("path");
        let owner_bytes = owner_peerid_bytes_from_z32(&provider_z32).expect("owner bytes");
        let plaintext = pubky_crypto::sealed_blob::sealed_blob_decrypt_with_context(
            &noise_sk,
            &body,
            &owner_bytes,
            &path,
        )
        .expect("decrypt proposal");
        let decoded: Subscription = serde_json::from_slice(&plaintext).expect("decode");
        assert_eq!(decoded, subscription);
    }

    #[tokio::test]
    async fn test_propose_subscription_bonded_failure_fails_closed() {
        let temp_dir = tempdir().unwrap();
        let (manager, _storage) = test_manager(&temp_dir);

        let provider = test_pubkey();
        let subscriber = test_pubkey();
        let (_p, _s, provider_session, _subscriber_session) = bonded_pair();

        let relay = SharedRelay(Arc::new(StubDropRelay::new()));
        relay.0.fail_writes.store(true, Ordering::SeqCst);
        let client: DropClient<Box<dyn DropHttp>> = DropClient::new(
            "http://relay.test",
            Box::new(relay.clone()) as Box<dyn DropHttp>,
        )
        .unwrap();
        manager
            .add_bonded_outbound(&subscriber, provider_session, client)
            .await;

        let (_noise_sk, noise_pk) = pubky_crypto::sealed_blob::x25519_generate_keypair();
        manager
            .noise_pk_cache
            .write()
            .await
            .insert(subscriber.to_string(), noise_pk);

        let subscription = test_subscription(&subscriber, &provider);
        let keypair = pkarr::Keypair::random();
        let mut channel = MockChannel;
        let err = manager
            .propose_subscription(&mut channel, subscription, &keypair)
            .await
            .expect_err("bonded send must fail");
        let msg = format!("{err}");
        assert!(
            msg.contains("bonded subscription proposal"),
            "unexpected error: {msg}"
        );
        // Fail closed: nothing on the relay, and with no pubky session there
        // was no public-outbox fallback either.
        assert_eq!(relay.0.message_count(), 0);
        assert!(manager.pubky_session.is_none());
    }

    #[tokio::test]
    async fn test_bonded_outbound_registry() {
        let temp_dir = tempdir().unwrap();
        let (manager, _storage) = test_manager(&temp_dir);
        let peer = test_pubkey();
        assert!(!manager.has_bonded_outbound(&peer).await);
        assert!(!manager.remove_bonded_outbound(&peer).await);

        let (_p, _s, provider_session, _sub) = bonded_pair();
        let relay = SharedRelay(Arc::new(StubDropRelay::new()));
        let client: DropClient<Box<dyn DropHttp>> =
            DropClient::new("http://relay.test", Box::new(relay) as Box<dyn DropHttp>).unwrap();
        manager
            .add_bonded_outbound(&peer, provider_session, client)
            .await;
        assert!(manager.has_bonded_outbound(&peer).await);
        assert!(manager.remove_bonded_outbound(&peer).await);
        assert!(!manager.has_bonded_outbound(&peer).await);
    }

    // ============================================================
    // W4 audit: SF-2 (mid-flight route removal) and SF-3 (bonded
    // requests/ACKs + poll_bonded through the registry)
    // ============================================================

    /// Homeserver session storage mock recording every `put` (path, content).
    struct RecordingSessionStorage {
        puts: Mutex<Vec<(String, String)>>,
    }

    impl RecordingSessionStorage {
        fn new() -> Self {
            RecordingSessionStorage {
                puts: Mutex::new(Vec::new()),
            }
        }

        fn recorded(&self) -> Vec<(String, String)> {
            self.puts.lock().expect("lock").clone()
        }
    }

    #[async_trait::async_trait]
    impl HomeserverSessionStorage for RecordingSessionStorage {
        async fn upsert_payment_endpoint(
            &self,
            _method: &MethodId,
            _data: &paykit_lib::EndpointData,
        ) -> paykit_lib::Result<()> {
            Ok(())
        }

        async fn remove_payment_endpoint(&self, _method: &MethodId) -> paykit_lib::Result<()> {
            Ok(())
        }

        async fn put(&self, path: &str, content: &str) -> paykit_lib::Result<()> {
            self.puts
                .lock()
                .expect("lock")
                .push((path.to_string(), content.to_string()));
            Ok(())
        }

        async fn get(&self, _path: &str) -> paykit_lib::Result<Option<String>> {
            Ok(None)
        }

        async fn delete(&self, _path: &str) -> paykit_lib::Result<()> {
            Ok(())
        }
    }

    /// Register a bonded route to `peer` on `manager` over a fresh stub
    /// relay; returns the relay and the counterparty's session.
    fn bonded_route_setup() -> (PeerId, BondSession, BondSession) {
        let (sender_id, _recipient_id, sender_session, recipient_session) = bonded_pair();
        (sender_id, sender_session, recipient_session)
    }

    #[tokio::test]
    async fn test_bonded_delivery_route_removed_midflight_fails_closed() {
        // SF-2 regression: the bonded dispatch used to re-acquire the
        // registry lock and `expect("presence checked above")`; a route
        // removed between the caller's presence check and the dispatch
        // turned that into a reachable panic. `deliver_bonded` now holds one
        // write guard across check-and-dispatch and returns a clean error —
        // for every message kind (SF-3 funnels all three through it).
        let temp_dir = tempdir().unwrap();
        let (manager, _storage) = test_manager(&temp_dir);
        let peer = test_pubkey();

        let (_peer_id, sender_session, _recipient_session) = bonded_route_setup();
        let relay = SharedRelay(Arc::new(StubDropRelay::new()));
        let client: DropClient<Box<dyn DropHttp>> = DropClient::new(
            "http://relay.test",
            Box::new(relay.clone()) as Box<dyn DropHttp>,
        )
        .unwrap();
        manager
            .add_bonded_outbound(&peer, sender_session, client)
            .await;
        // The route vanishes between the caller's presence check and the
        // dispatch (the interleaving that previously panicked).
        assert!(manager.remove_bonded_outbound(&peer).await);

        for kind in [
            ProtocolMessageKind::Request,
            ProtocolMessageKind::Proposal,
            ProtocolMessageKind::Ack,
        ] {
            let err = manager
                .deliver_bonded(&peer.to_string(), kind, b"payload")
                .await
                .expect_err("removed route must be a clean error, not a panic");
            let msg = format!("{err}");
            assert!(
                msg.contains("no longer registered"),
                "unexpected error: {msg}"
            );
            assert!(
                msg.contains("no public fallback"),
                "unexpected error: {msg}"
            );
        }
        // Fail closed in every case: nothing reached the relay.
        assert_eq!(relay.0.message_count(), 0);
    }

    #[tokio::test]
    async fn test_publish_payment_request_bonded_arrives_via_drop() {
        let temp_dir = tempdir().unwrap();
        let (manager, _storage) = test_manager(&temp_dir);

        let sender = test_pubkey();
        let recipient = test_pubkey();
        let (_peer_id, sender_session, mut recipient_session) = bonded_route_setup();

        let relay = SharedRelay(Arc::new(StubDropRelay::new()));
        let client: DropClient<Box<dyn DropHttp>> = DropClient::new(
            "http://relay.test",
            Box::new(relay.clone()) as Box<dyn DropHttp>,
        )
        .unwrap();
        manager
            .add_bonded_outbound(&recipient, sender_session, client)
            .await;

        let storage = RecordingSessionStorage::new();
        let request = PaymentRequest::new(
            sender.clone(),
            recipient.clone(),
            Amount::from_sats(1000),
            "SAT".to_string(),
            MethodId("lightning".to_string()),
        );
        let (noise_sk, noise_pk) = pubky_crypto::sealed_blob::x25519_generate_keypair();
        manager
            .publish_payment_request(&storage, &sender.to_string(), &request, &noise_pk)
            .await
            .expect("bonded publish");

        // SF-3: with a registered route the storage mock records ZERO
        // writes under /pub/; the request traveled over the Drop channel.
        assert!(storage.recorded().is_empty());
        assert_eq!(relay.0.message_count(), 1);

        // The recipient opens it over its own receive channel; the body is
        // the same encrypted blob the public path would have stored.
        let sub_client: DropClient<SharedRelay> =
            DropClient::new("http://relay.test", relay.clone()).unwrap();
        let received = receive_bonded(
            std::slice::from_mut(&mut recipient_session),
            &[PurposeId::paykit()],
            &sub_client,
        )
        .await
        .expect("recipient receive");
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].2, Authenticity::SessionAuthenticated);
        let body = String::from_utf8(received[0].3.clone()).expect("blob is text");
        assert!(pubky_crypto::sealed_blob::is_sealed_blob(&body));

        let path = paykit_lib::protocol::payment_request_path(
            &sender.to_string(),
            &recipient.to_string(),
            &request.request_id,
        )
        .expect("path");
        let owner_bytes = owner_peerid_bytes_from_z32(&sender.to_string()).expect("owner bytes");
        let plaintext = pubky_crypto::sealed_blob::sealed_blob_decrypt_with_context(
            &noise_sk,
            &body,
            &owner_bytes,
            &path,
        )
        .expect("decrypt request");
        let published: crate::discovery::PublishedRequest =
            serde_json::from_slice(&plaintext).expect("decode");
        assert_eq!(published.request.request_id, request.request_id);
        assert!(published.active);
    }

    #[tokio::test]
    async fn test_publish_payment_request_unbonded_is_byte_identical_legacy() {
        let temp_dir = tempdir().unwrap();
        let (manager, _storage) = test_manager(&temp_dir);

        let sender = test_pubkey();
        let recipient = test_pubkey();
        let request = PaymentRequest::new(
            sender.clone(),
            recipient.clone(),
            Amount::from_sats(1000),
            "SAT".to_string(),
            MethodId("lightning".to_string()),
        );
        let (_noise_sk, noise_pk) = pubky_crypto::sealed_blob::x25519_generate_keypair();

        // Manager path with no registered route...
        let via_manager = RecordingSessionStorage::new();
        manager
            .publish_payment_request(&via_manager, &sender.to_string(), &request, &noise_pk)
            .await
            .expect("legacy publish via manager");

        // ...must behave byte-identically to the legacy entry point.
        let direct = RecordingSessionStorage::new();
        crate::discovery::publish_payment_request(
            &direct,
            &sender.to_string(),
            &request,
            &noise_pk,
        )
        .await
        .expect("legacy publish direct");

        let via_manager = via_manager.recorded();
        let direct = direct.recorded();
        assert_eq!(via_manager.len(), 1);
        assert_eq!(direct.len(), 1);
        // Same canonical /pub/ path; bodies differ only by random nonce.
        assert_eq!(via_manager[0].0, direct[0].0);
        assert!(via_manager[0].0.starts_with("/pub/paykit.app/v0/requests/"));
        assert!(pubky_crypto::sealed_blob::is_sealed_blob(&via_manager[0].1));
    }

    #[tokio::test]
    async fn test_store_encrypted_ack_bonded_arrives_via_drop() {
        let temp_dir = tempdir().unwrap();
        let (manager, _storage) = test_manager(&temp_dir);

        let recipient = test_pubkey();
        let (_peer_id, acker_session, mut recipient_session) = bonded_route_setup();

        let relay = SharedRelay(Arc::new(StubDropRelay::new()));
        let client: DropClient<Box<dyn DropHttp>> = DropClient::new(
            "http://relay.test",
            Box::new(relay.clone()) as Box<dyn DropHttp>,
        )
        .unwrap();
        manager
            .add_bonded_outbound(&recipient, acker_session, client)
            .await;

        let public_calls = Arc::new(Mutex::new(0usize));
        let calls = public_calls.clone();
        let ack_bytes = b"signed-sb2-ack-bytes".to_vec();
        manager
            .store_encrypted_ack(&recipient, &ack_bytes, move || {
                let calls = calls.clone();
                async move {
                    *calls.lock().expect("lock") += 1;
                    Ok(())
                }
            })
            .await
            .expect("bonded ack");

        // SF-3, fail closed: the public write never ran; the ACK traveled
        // over the Drop channel.
        assert_eq!(*public_calls.lock().expect("lock"), 0);
        assert_eq!(relay.0.message_count(), 1);

        // The recipient receives it declared ExternallyAuthenticated
        // (receipt semantics, S9).
        let sub_client: DropClient<SharedRelay> =
            DropClient::new("http://relay.test", relay.clone()).unwrap();
        let received = receive_bonded(
            std::slice::from_mut(&mut recipient_session),
            &[PurposeId::paykit()],
            &sub_client,
        )
        .await
        .expect("receive");
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].2, Authenticity::ExternallyAuthenticated);
        assert_eq!(received[0].3, ack_bytes);
    }

    #[tokio::test]
    async fn test_store_encrypted_ack_unbonded_runs_public_write() {
        let temp_dir = tempdir().unwrap();
        let (manager, _storage) = test_manager(&temp_dir);
        let peer = test_pubkey();

        // No route registered: the caller's legacy public write runs,
        // unchanged.
        let public_calls = Arc::new(Mutex::new(Vec::new()));
        let calls = public_calls.clone();
        manager
            .store_encrypted_ack(&peer, b"ack-bytes", move || {
                let calls = calls.clone();
                async move {
                    calls.lock().expect("lock").push(b"ack-bytes".to_vec());
                    Ok(())
                }
            })
            .await
            .expect("public ack write");
        assert_eq!(public_calls.lock().expect("lock").len(), 1);
    }

    #[tokio::test]
    async fn test_poll_bonded_receives_across_registered_routes() {
        let temp_dir = tempdir().unwrap();
        let (manager, _storage) = test_manager(&temp_dir);

        let peer = test_pubkey();
        let (_peer_id, my_session, mut peer_session) = bonded_route_setup();

        let relay = SharedRelay(Arc::new(StubDropRelay::new()));
        let client: DropClient<Box<dyn DropHttp>> = DropClient::new(
            "http://relay.test",
            Box::new(relay.clone()) as Box<dyn DropHttp>,
        )
        .unwrap();
        manager.add_bonded_outbound(&peer, my_session, client).await;

        // The counterparty sends a protocol message back over the same
        // relay; `poll_bonded` is the in-repo receive-side caller of
        // `receive_bonded`.
        let peer_client: DropClient<SharedRelay> =
            DropClient::new("http://relay.test", relay.clone()).unwrap();
        send_protocol_message(
            &mut peer_session,
            ProtocolMessageKind::Request,
            b"inbound-request",
            &peer_client,
        )
        .await
        .expect("peer send");

        let received = manager
            .poll_bonded(&[PurposeId::paykit()])
            .await
            .expect("poll");
        assert_eq!(received.len(), 1);
        let (peer_z32, hdr, authenticity, body) = &received[0];
        assert_eq!(peer_z32, &peer.to_string());
        assert_eq!(hdr.purpose, PurposeId::paykit());
        assert_eq!(authenticity, &Authenticity::SessionAuthenticated);
        assert_eq!(body, b"inbound-request");

        // Opened messages were ack-deleted: a second poll is empty.
        let again = manager
            .poll_bonded(&[PurposeId::paykit()])
            .await
            .expect("second poll");
        assert!(again.is_empty());
        assert_eq!(relay.0.message_count(), 0);

        // An empty registry polls clean.
        assert!(manager.remove_bonded_outbound(&peer).await);
        let none = manager
            .poll_bonded(&[PurposeId::paykit()])
            .await
            .expect("empty registry");
        assert!(none.is_empty());
    }

    #[tokio::test]
    async fn test_poll_bonded_errors_when_every_route_fails() {
        let temp_dir = tempdir().unwrap();
        let (manager, _storage) = test_manager(&temp_dir);

        let peer = test_pubkey();
        let (_peer_id, my_session, _peer_session) = bonded_route_setup();
        let relay = SharedRelay(Arc::new(StubDropRelay::new()));
        relay.0.fail_reads.store(true, Ordering::SeqCst);
        let client: DropClient<Box<dyn DropHttp>> =
            DropClient::new("http://relay.test", Box::new(relay) as Box<dyn DropHttp>).unwrap();
        manager.add_bonded_outbound(&peer, my_session, client).await;

        let err = manager
            .poll_bonded(&[PurposeId::paykit()])
            .await
            .expect_err("every route failing must error");
        let msg = format!("{err}");
        assert!(
            msg.contains("relay read failure"),
            "unexpected error: {msg}"
        );
    }
}
