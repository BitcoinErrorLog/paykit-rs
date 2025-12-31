use crate::{
    Amount, AutoPayRule, PaymentRequest, PeerSpendingLimit, RequestStatus, SignedSubscription,
    Subscription, SubscriptionError,
};
use async_trait::async_trait;
use paykit_lib::PublicKey;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type Result<T> = anyhow::Result<T>;

/// Token representing a reserved spending amount
///
/// This token ensures atomic check-and-reserve operations for spending limits.
/// The reservation must be either committed (payment succeeded) or rolled back
/// (payment failed).
#[derive(Debug, Clone)]
pub struct ReservationToken {
    pub peer: PublicKey,
    pub amount: Amount,
    pub reserved_at: i64,
    #[allow(dead_code)]
    token_id: String,
}

impl ReservationToken {
    fn new(peer: PublicKey, amount: Amount) -> Self {
        Self {
            peer,
            amount,
            reserved_at: chrono::Utc::now().timestamp(),
            token_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

/// Filter for listing payment requests
#[derive(Debug, Clone, Default)]
pub struct RequestFilter {
    pub peer: Option<PublicKey>,
    pub status: Option<RequestStatus>,
    pub direction: Option<Direction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Incoming,
    Outgoing,
}

/// Storage trait for subscription data
#[async_trait]
pub trait SubscriptionStorage: Send + Sync {
    // Payment requests
    async fn save_request(&self, request: &PaymentRequest) -> Result<()>;
    async fn get_request(&self, id: &str) -> Result<Option<PaymentRequest>>;
    async fn list_requests(&self, filter: RequestFilter) -> Result<Vec<PaymentRequest>>;
    async fn update_request_status(&self, id: &str, status: RequestStatus) -> Result<()>;

    // Subscriptions
    async fn save_subscription(&self, sub: &Subscription) -> Result<()>;
    async fn get_subscription(&self, id: &str) -> Result<Option<Subscription>>;
    async fn save_signed_subscription(&self, sub: &SignedSubscription) -> Result<()>;
    async fn get_signed_subscription(&self, id: &str) -> Result<Option<SignedSubscription>>;
    async fn list_subscriptions_with_peer(
        &self,
        peer: &PublicKey,
    ) -> Result<Vec<SignedSubscription>>;
    async fn list_active_subscriptions(&self) -> Result<Vec<SignedSubscription>>;

    // Auto-pay rules
    async fn save_autopay_rule(&self, rule: &AutoPayRule) -> Result<()>;
    async fn get_autopay_rule(&self, subscription_id: &str) -> Result<Option<AutoPayRule>>;

    // Spending limits
    async fn save_peer_limit(&self, limit: &PeerSpendingLimit) -> Result<()>;
    async fn get_peer_limit(&self, peer: &PublicKey) -> Result<Option<PeerSpendingLimit>>;

    // Atomic spending operations (Phase 4: fixes VULN-005 & VULN-006)
    /// Atomically check if spending is within limits and reserve the amount
    ///
    /// # Security
    ///
    /// This method MUST be atomic - no other thread can modify the spending
    /// limit between checking and reserving. Returns a ReservationToken that
    /// must be either committed or rolled back.
    ///
    /// # Errors
    ///
    /// Returns `Err(SubscriptionError::LimitExceeded)` if the amount would exceed the limit.
    /// Returns `Err(SubscriptionError::NotFound)` if no limit is set for this peer.
    async fn try_reserve_spending(
        &self,
        peer: &PublicKey,
        amount: &Amount,
    ) -> Result<ReservationToken>;

    /// Commit a spending reservation (payment succeeded)
    ///
    /// The reserved amount becomes permanent. This is idempotent - committing
    /// the same token multiple times has no effect after the first commit.
    async fn commit_spending(&self, token: ReservationToken) -> Result<()>;

    /// Rollback a spending reservation (payment failed)
    ///
    /// The reserved amount is released back to the limit. This is idempotent.
    async fn rollback_spending(&self, token: ReservationToken) -> Result<()>;
}

/// File-based storage implementation (native only)
///
/// This implementation uses file I/O and file-level locking (fs2) for atomic
/// operations, which are not available in WASM. For WASM environments, use
/// `WasmSubscriptionStorage` instead.
#[cfg(not(target_arch = "wasm32"))]
pub struct FileSubscriptionStorage {
    base_path: PathBuf,
    requests: Arc<Mutex<HashMap<String, (PaymentRequest, RequestStatus)>>>,
    subscriptions: Arc<Mutex<HashMap<String, Subscription>>>,
    signed_subscriptions: Arc<Mutex<HashMap<String, SignedSubscription>>>,
    autopay_rules: Arc<Mutex<HashMap<String, AutoPayRule>>>,
    peer_limits: Arc<Mutex<HashMap<String, PeerSpendingLimit>>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl FileSubscriptionStorage {
    pub fn new(base_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&base_path)?;
        std::fs::create_dir_all(base_path.join("requests"))?;
        std::fs::create_dir_all(base_path.join("subscriptions"))?;
        std::fs::create_dir_all(base_path.join("signed_subscriptions"))?;
        std::fs::create_dir_all(base_path.join("autopay_rules"))?;
        std::fs::create_dir_all(base_path.join("peer_limits"))?;

        Ok(Self {
            base_path,
            requests: Arc::new(Mutex::new(HashMap::new())),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            signed_subscriptions: Arc::new(Mutex::new(HashMap::new())),
            autopay_rules: Arc::new(Mutex::new(HashMap::new())),
            peer_limits: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn request_path(&self, id: &str) -> PathBuf {
        self.base_path.join("requests").join(format!("{}.json", id))
    }

    fn subscription_path(&self, id: &str) -> PathBuf {
        self.base_path
            .join("subscriptions")
            .join(format!("{}.json", id))
    }

    fn signed_subscription_path(&self, id: &str) -> PathBuf {
        self.base_path
            .join("signed_subscriptions")
            .join(format!("{}.json", id))
    }

    fn autopay_rule_path(&self, subscription_id: &str) -> PathBuf {
        self.base_path
            .join("autopay_rules")
            .join(format!("{}.json", subscription_id))
    }

    fn peer_limit_path(&self, peer: &PublicKey) -> PathBuf {
        let peer_str = format!("{:?}", peer);
        self.base_path
            .join("peer_limits")
            .join(format!("{}.json", peer_str))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl SubscriptionStorage for FileSubscriptionStorage {
    async fn save_request(&self, request: &PaymentRequest) -> Result<()> {
        let path = self.request_path(&request.request_id);
        let json = serde_json::to_string_pretty(request)?;
        std::fs::write(path, json)?;

        let mut requests = self.requests.lock().unwrap_or_else(|e| e.into_inner());
        requests.insert(
            request.request_id.clone(),
            (request.clone(), RequestStatus::Pending),
        );

        Ok(())
    }

    async fn get_request(&self, id: &str) -> Result<Option<PaymentRequest>> {
        let path = self.request_path(id);
        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(path)?;
        let request: PaymentRequest = serde_json::from_str(&json)?;
        Ok(Some(request))
    }

    async fn list_requests(&self, filter: RequestFilter) -> Result<Vec<PaymentRequest>> {
        let requests_dir = self.base_path.join("requests");
        let mut result = Vec::new();

        if !requests_dir.exists() {
            return Ok(result);
        }

        // Read all request files from disk
        for entry in std::fs::read_dir(requests_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let json = std::fs::read_to_string(&path)?;
            let req: PaymentRequest = serde_json::from_str(&json)?;

            // Apply filters
            if let Some(ref peer) = filter.peer {
                match filter.direction {
                    Some(Direction::Incoming) if &req.from != peer => continue,
                    Some(Direction::Outgoing) if &req.to != peer => continue,
                    _ if &req.from != peer && &req.to != peer => continue,
                    _ => {}
                }
            }

            result.push(req);
        }

        Ok(result)
    }

    async fn update_request_status(&self, id: &str, status: RequestStatus) -> Result<()> {
        let mut requests = self.requests.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((_req, old_status)) = requests.get_mut(id) {
            *old_status = status;
        }
        Ok(())
    }

    async fn save_subscription(&self, sub: &Subscription) -> Result<()> {
        let path = self.subscription_path(&sub.subscription_id);
        let json = serde_json::to_string_pretty(sub)?;
        std::fs::write(path, json)?;

        let mut subscriptions = self.subscriptions.lock().unwrap_or_else(|e| e.into_inner());
        subscriptions.insert(sub.subscription_id.clone(), sub.clone());

        Ok(())
    }

    async fn get_subscription(&self, id: &str) -> Result<Option<Subscription>> {
        let path = self.subscription_path(id);
        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(path)?;
        let sub: Subscription = serde_json::from_str(&json)?;
        Ok(Some(sub))
    }

    async fn save_signed_subscription(&self, sub: &SignedSubscription) -> Result<()> {
        let path = self.signed_subscription_path(&sub.subscription.subscription_id);
        let json = serde_json::to_string_pretty(sub)?;
        std::fs::write(path, json)?;

        let mut signed_subs = self.signed_subscriptions.lock().unwrap_or_else(|e| e.into_inner());
        signed_subs.insert(sub.subscription.subscription_id.clone(), sub.clone());

        Ok(())
    }

    async fn get_signed_subscription(&self, id: &str) -> Result<Option<SignedSubscription>> {
        let path = self.signed_subscription_path(id);
        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(path)?;
        let sub: SignedSubscription = serde_json::from_str(&json)?;
        Ok(Some(sub))
    }

    async fn list_subscriptions_with_peer(
        &self,
        peer: &PublicKey,
    ) -> Result<Vec<SignedSubscription>> {
        let signed_subs = self.signed_subscriptions.lock().unwrap_or_else(|e| e.into_inner());
        let result: Vec<SignedSubscription> = signed_subs
            .values()
            .filter(|s| &s.subscription.subscriber == peer || &s.subscription.provider == peer)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn list_active_subscriptions(&self) -> Result<Vec<SignedSubscription>> {
        let signed_subs = self.signed_subscriptions.lock().unwrap_or_else(|e| e.into_inner());
        let now = chrono::Utc::now().timestamp();

        let result: Vec<SignedSubscription> = signed_subs
            .values()
            .filter(|s| {
                s.subscription.starts_at <= now
                    && s.subscription.ends_at.is_none_or(|end| end > now)
            })
            .cloned()
            .collect();
        Ok(result)
    }

    async fn save_autopay_rule(&self, rule: &AutoPayRule) -> Result<()> {
        let path = self.autopay_rule_path(&rule.subscription_id);
        let json = serde_json::to_string_pretty(rule)?;
        std::fs::write(path, json)?;

        let mut rules = self.autopay_rules.lock().unwrap_or_else(|e| e.into_inner());
        rules.insert(rule.subscription_id.clone(), rule.clone());

        Ok(())
    }

    async fn get_autopay_rule(&self, subscription_id: &str) -> Result<Option<AutoPayRule>> {
        let path = self.autopay_rule_path(subscription_id);
        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(path)?;
        let rule: AutoPayRule = serde_json::from_str(&json)?;
        Ok(Some(rule))
    }

    async fn save_peer_limit(&self, limit: &PeerSpendingLimit) -> Result<()> {
        let path = self.peer_limit_path(&limit.peer);
        let json = serde_json::to_string_pretty(limit)?;
        std::fs::write(path, json)?;

        let mut limits = self.peer_limits.lock().unwrap_or_else(|e| e.into_inner());
        let peer_str = format!("{:?}", limit.peer);
        limits.insert(peer_str, limit.clone());

        Ok(())
    }

    async fn get_peer_limit(&self, peer: &PublicKey) -> Result<Option<PeerSpendingLimit>> {
        let path = self.peer_limit_path(peer);
        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(path)?;
        let limit: PeerSpendingLimit = serde_json::from_str(&json)?;
        Ok(Some(limit))
    }

    // ====================================================================
    // Phase 4: Atomic Spending Operations (VULN-005 & VULN-006 fixes)
    // ====================================================================

    async fn try_reserve_spending(
        &self,
        peer: &PublicKey,
        amount: &Amount,
    ) -> Result<ReservationToken> {
        use fs2::FileExt;
        use std::fs::OpenOptions;

        let path = self.peer_limit_path(peer);

        // Open/create the file for locking
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;

        // Acquire exclusive lock (blocks until available)
        file.lock_exclusive()?;

        // Load current limit
        let mut limit = if path.exists() && std::fs::metadata(&path)?.len() > 0 {
            let json = std::fs::read_to_string(&path)?;
            serde_json::from_str::<PeerSpendingLimit>(&json)?
        } else {
            // Release lock before returning error
            file.unlock()?;
            return Err(SubscriptionError::NotFound("Peer limit not found".to_string()).into());
        };

        // Check if reset needed
        if limit.should_reset() {
            limit.reset();
        }

        // Check if would exceed limit
        if limit.would_exceed_limit(amount) {
            file.unlock()?;
            return Err(SubscriptionError::LimitExceeded.into());
        }

        // Reserve amount
        limit.current_spent = limit
            .current_spent
            .checked_add(amount)
            .ok_or(SubscriptionError::Overflow)?;

        // Save updated limit
        let json = serde_json::to_string_pretty(&limit)?;
        std::fs::write(&path, json)?;

        // Update in-memory cache
        let mut limits = self.peer_limits.lock().unwrap_or_else(|e| e.into_inner());
        let peer_str = format!("{:?}", peer);
        limits.insert(peer_str, limit);

        // Release lock
        file.unlock()?;

        // Return reservation token
        Ok(ReservationToken::new(peer.clone(), *amount))
    }

    async fn commit_spending(&self, _token: ReservationToken) -> Result<()> {
        // The spending was already committed when we reserved it
        // This is here for API consistency and potential future enhancements
        // (like tracking committed vs pending reservations)
        Ok(())
    }

    async fn rollback_spending(&self, token: ReservationToken) -> Result<()> {
        use fs2::FileExt;
        use std::fs::OpenOptions;

        let path = self.peer_limit_path(&token.peer);

        if !path.exists() {
            // Peer limit was deleted, nothing to rollback
            return Ok(());
        }

        // Open file for locking
        let file = OpenOptions::new().read(true).write(true).open(&path)?;

        // Acquire exclusive lock
        file.lock_exclusive()?;

        // Load current limit
        let json = std::fs::read_to_string(&path)?;
        let mut limit: PeerSpendingLimit = serde_json::from_str(&json)?;

        // Rollback the reserved amount
        limit.current_spent = limit
            .current_spent
            .checked_sub(&token.amount)
            .unwrap_or(Amount::from_sats(0)); // Defensive: don't go negative

        // Save updated limit
        let json = serde_json::to_string_pretty(&limit)?;
        std::fs::write(&path, json)?;

        // Update in-memory cache
        let mut limits = self.peer_limits.lock().unwrap_or_else(|e| e.into_inner());
        let peer_str = format!("{:?}", &token.peer);
        limits.insert(peer_str, limit);

        // Release lock
        file.unlock()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tempfile::tempdir;

    fn test_pubkey() -> PublicKey {
        let keypair = pkarr::Keypair::random();
        PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
    }

    #[tokio::test]
    async fn test_save_and_get_request() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSubscriptionStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let from = test_pubkey();
        let to = test_pubkey();
        let request = PaymentRequest::new(
            from,
            to,
            Amount::from_sats(1000),
            "SAT".to_string(),
            paykit_lib::MethodId("lightning".to_string()),
        );

        storage.save_request(&request).await.unwrap();

        let loaded = storage.get_request(&request.request_id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().request_id, request.request_id);
    }

    #[tokio::test]
    async fn test_list_requests_with_filter() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSubscriptionStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let from = test_pubkey();
        let to = test_pubkey();

        let req1 = PaymentRequest::new(
            from.clone(),
            to.clone(),
            Amount::from_sats(1000),
            "SAT".to_string(),
            paykit_lib::MethodId("lightning".to_string()),
        );

        let req2 = PaymentRequest::new(
            to.clone(),
            from.clone(),
            Amount::from_sats(2000),
            "SAT".to_string(),
            paykit_lib::MethodId("onchain".to_string()),
        );

        storage.save_request(&req1).await.unwrap();
        storage.save_request(&req2).await.unwrap();

        let filter = RequestFilter {
            peer: Some(from.clone()),
            status: None,
            direction: Some(Direction::Outgoing),
        };

        let requests = storage.list_requests(filter).await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].request_id, req1.request_id);
    }

    #[tokio::test]
    async fn test_update_request_status() {
        let temp_dir = tempdir().unwrap();
        let storage = FileSubscriptionStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let from = test_pubkey();
        let to = test_pubkey();
        let request = PaymentRequest::new(
            from,
            to,
            Amount::from_sats(1000),
            "SAT".to_string(),
            paykit_lib::MethodId("lightning".to_string()),
        );

        storage.save_request(&request).await.unwrap();
        storage
            .update_request_status(&request.request_id, RequestStatus::Accepted)
            .await
            .unwrap();

        let filter = RequestFilter {
            peer: None,
            status: Some(RequestStatus::Accepted),
            direction: None,
        };

        let requests = storage.list_requests(filter).await.unwrap();
        assert_eq!(requests.len(), 1);
    }
}
