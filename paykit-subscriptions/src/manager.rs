use crate::request::RequestNotification;
use crate::{
    signing::{self, Signature},
    NonceStore, PaymentRequest, PaymentRequestResponse, RequestStatus, Result, SignedSubscription,
    Subscription, SubscriptionStorage,
};
use paykit_interactive::{PaykitInteractiveManager, PaykitNoiseChannel, PaykitNoiseMessage};
use paykit_lib::PublicKey;
use std::sync::Arc;

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

pub struct SubscriptionManager {
    storage: Arc<Box<dyn SubscriptionStorage>>,
    interactive: Arc<PaykitInteractiveManager>,
    pubky_session: Option<pubky::PubkySession>,
    nonce_store: Arc<NonceStore>,
}

impl SubscriptionManager {
    pub fn new(
        storage: Arc<Box<dyn SubscriptionStorage>>,
        interactive: Arc<PaykitInteractiveManager>,
    ) -> Self {
        Self {
            storage,
            interactive,
            pubky_session: None,
            nonce_store: Arc::new(NonceStore::new()),
        }
    }

    pub fn with_pubky_session(mut self, session: pubky::PubkySession) -> Self {
        self.pubky_session = Some(session);
        self
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
        let _msg_json =
            serde_json::to_string(&SubscriptionMessage::PaymentRequest(Box::new(request.clone())))?;
        channel
            .send(PaykitNoiseMessage::Ack) // Placeholder - would extend enum
            .await?;

        // Also store notification in Pubky for async discovery
        if let Some(session) = &self.pubky_session {
            self.store_notification(session, &request).await?;
        }

        Ok(())
    }

    /// Store payment request notification in Pubky storage
    async fn store_notification(
        &self,
        session: &pubky::PubkySession,
        request: &PaymentRequest,
    ) -> Result<()> {
        let path = format!(
            "/pub/paykit.app/subscriptions/requests/{:?}/{}",
            request.to, request.request_id
        );

        let notification = RequestNotification {
            request_id: request.request_id.clone(),
            from: request.from.clone(),
            amount: request.amount,
            currency: request.currency.clone(),
            created_at: request.created_at,
        };

        let data = serde_json::to_vec(&notification)?;
        session
            .storage()
            .put(path, data)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store notification: {}", e))?;

        Ok(())
    }

    /// Poll for new payment requests from Pubky storage
    pub async fn poll_requests(&self, peer: &PublicKey) -> Result<Vec<PaymentRequest>> {
        use paykit_lib::{PubkyUnauthenticatedTransport, UnauthenticatedTransportRead};

        // If no Pubky session available, return empty (can't poll)
        if self.pubky_session.is_none() {
            return Ok(Vec::new());
        }

        // Create unauthenticated transport for reading peer's storage
        let public_storage = pubky::PublicStorage::new()
            .map_err(|e| anyhow::anyhow!("Failed to create public storage: {}", e))?;
        let unauth_transport = PubkyUnauthenticatedTransport::new(public_storage);

        // List directory entries for this peer's payment requests
        let path = format!("/pub/paykit.app/subscriptions/requests/{:?}", peer);
        let entries = unauth_transport
            .list_directory(peer, &path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list directory: {}", e))?;

        let mut requests = Vec::new();

        // Fetch each notification and convert to PaymentRequest
        for entry in entries {
            let notification_path = format!("{}/{}", path, entry);
            if let Some(notification_json) = unauth_transport.get(peer, &notification_path).await? {
                match serde_json::from_str::<RequestNotification>(&notification_json) {
                    Ok(notification) => {
                        // Check if we've already seen this request
                        if self
                            .storage
                            .get_request(&notification.request_id)
                            .await?
                            .is_none()
                        {
                        // Convert notification to PaymentRequest
                        let request = PaymentRequest::new(
                            notification.from,
                            peer.clone(),
                            notification.amount,
                            notification.currency,
                            paykit_lib::MethodId("lightning".to_string()), // Default, should be in notification
                        );
                        // Override request_id and created_at from notification
                        let request = PaymentRequest {
                            request_id: notification.request_id,
                            created_at: notification.created_at,
                            ..request
                        };
                            requests.push(request);
                        }
                    }
                    Err(e) => {
                        // Log but continue processing other entries
                        eprintln!("Failed to parse notification {}: {}", entry, e);
                    }
                }
            }
        }

        Ok(requests)
    }

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

        // Send response via Noise channel
        // Placeholder - would use extended enum
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
            .nonce_store
            .as_ref()
            .check_and_mark(&nonce, signature.expires_at)?
        {
            return Err(anyhow::anyhow!("Nonce already used"));
        }

        // Store the signature temporarily (we'll use it when acceptance comes back)
        // For now, just send the proposal

        // Send proposal via Noise channel
        // Note: In production, this would extend PaykitNoiseMessage enum
        // For now, we use a placeholder
        channel.send(PaykitNoiseMessage::Ack).await?;

        // Also store in Pubky for async discovery
        if let Some(session) = &self.pubky_session {
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
            .nonce_store
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
            .nonce_store
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
        if !self.nonce_store.as_ref().check_and_mark(
            &signed.subscriber_signature.nonce,
            signed.subscriber_signature.expires_at,
        )? {
            return Err(anyhow::anyhow!(
                "Subscriber nonce already used (replay attack)"
            ));
        }
        if !self.nonce_store.as_ref().check_and_mark(
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

    async fn store_subscription_proposal(
        &self,
        session: &pubky::PubkySession,
        subscription: &Subscription,
    ) -> Result<()> {
        let path = format!(
            "/pub/paykit.app/subscriptions/proposals/{:?}/{}",
            subscription.provider, subscription.subscription_id
        );
        let data = serde_json::to_vec(&subscription)?;
        session
            .storage()
            .put(path, data)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store proposal: {}", e))?;
        Ok(())
    }

    async fn store_signed_subscription(
        &self,
        session: &pubky::PubkySession,
        signed: &SignedSubscription,
    ) -> Result<()> {
        // Store for both parties
        let path_subscriber = format!(
            "/pub/paykit.app/subscriptions/agreements/{:?}/{}",
            signed.subscription.subscriber, signed.subscription.subscription_id
        );
        let path_provider = format!(
            "/pub/paykit.app/subscriptions/agreements/{:?}/{}",
            signed.subscription.provider, signed.subscription.subscription_id
        );

        let data = serde_json::to_vec(&signed)?;

        session
            .storage()
            .put(path_subscriber.clone(), data.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store subscription for subscriber: {}", e))?;

        session
            .storage()
            .put(path_provider, data)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store subscription for provider: {}", e))?;

        Ok(())
    }

    async fn store_subscription_cancellation(
        &self,
        session: &pubky::PubkySession,
        subscription: &SignedSubscription,
        reason: Option<String>,
    ) -> Result<()> {
        let path = format!(
            "/pub/paykit.app/subscriptions/cancellations/{}",
            subscription.subscription.subscription_id
        );
        let data = serde_json::to_vec(&serde_json::json!({
            "subscription_id": subscription.subscription.subscription_id,
            "reason": reason,
            "cancelled_at": chrono::Utc::now().timestamp(),
        }))?;

        session
            .storage()
            .put(path, data)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store cancellation: {}", e))?;

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

    /// Execute auto-payment
    /// Execute auto-payment with atomic spending limit enforcement
    ///
    /// # Security
    ///
    /// Uses atomic check-and-reserve operations to prevent TOCTOU race conditions.
    /// If payment fails, the reservation is rolled back automatically.
    pub async fn execute_autopay<C: PaykitNoiseChannel>(
        &self,
        channel: &mut C,
        request: PaymentRequest,
        local_pk: &PublicKey,
    ) -> Result<paykit_interactive::PaykitReceipt> {
        // Phase 4: Atomic check-and-reserve spending limit
        let reservation = self
            .storage
            .try_reserve_spending(&request.from, &request.amount)
            .await?;

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
                self.storage.commit_spending(reservation).await?;
                self.storage
                    .update_request_status(&request.request_id, RequestStatus::Fulfilled)
                    .await?;
                Ok(receipt)
            }
            Err(e) => {
                // Payment failed - rollback the reservation
                self.storage.rollback_spending(reservation).await?;
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
    use crate::{storage::FileSubscriptionStorage, Amount};
    use paykit_interactive::{PaykitInteractiveManager, PaykitStorage, ReceiptGenerator};
    use paykit_lib::{MethodId, PublicKey};
    use std::str::FromStr;
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
        let manager = SubscriptionManager::new(storage.clone(), interactive);

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
        let manager = SubscriptionManager::new(storage.clone(), interactive);

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
        let manager = SubscriptionManager::new(storage, interactive);

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
}
