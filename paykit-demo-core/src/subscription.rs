//! Subscription management and auto-pay coordination
//!
//! This module provides the SubscriptionCoordinator for managing subscriptions,
//! auto-pay rules, and spending limits in demo applications.

use anyhow::{Context, Result};
use paykit_lib::{MethodId, PublicKey};
use paykit_subscriptions::{
    storage::{FileSubscriptionStorage, SubscriptionStorage},
    Amount, AutoPayRule, PaymentFrequency, PaymentRequest, PeerSpendingLimit, Subscription,
    SubscriptionTerms,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Demo subscription wrapper with additional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoSubscription {
    /// The underlying subscription
    pub inner: Subscription,
    /// Human-readable description
    pub description: String,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Demo payment request wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoPaymentRequest {
    /// The underlying payment request
    pub inner: paykit_subscriptions::PaymentRequest,
}

/// Coordinates subscription management, auto-pay rules, and spending limits
pub struct SubscriptionCoordinator {
    storage: FileSubscriptionStorage,
}

impl SubscriptionCoordinator {
    /// Create a new subscription coordinator
    pub fn new(storage_path: impl AsRef<Path>) -> Result<Self> {
        let storage = FileSubscriptionStorage::new(storage_path.as_ref().to_path_buf())
            .context("Failed to initialize subscription storage")?;
        Ok(Self { storage })
    }

    /// Create a new subscription
    pub fn create_subscription(
        &self,
        subscriber: PublicKey,
        provider: PublicKey,
        amount_sats: i64,
        currency: String,
        frequency: PaymentFrequency,
        method_id: MethodId,
        description: String,
    ) -> Result<DemoSubscription> {
        let terms = SubscriptionTerms::new(
            Amount::from_sats(amount_sats),
            currency,
            frequency,
            method_id,
            description.clone(),
        );

        let subscription = Subscription::new(subscriber, provider, terms);

        let demo_sub = DemoSubscription {
            inner: subscription,
            description,
            created_at: chrono::Utc::now(),
        };

        // Save subscription (blocking for simplicity in demo)
        tokio::runtime::Handle::try_current()
            .map(|handle| handle.block_on(self.storage.save_subscription(&demo_sub.inner)))
            .unwrap_or_else(|_| {
                // If no runtime, create a temporary one
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(self.storage.save_subscription(&demo_sub.inner))
            })
            .context("Failed to save subscription")?;

        Ok(demo_sub)
    }

    /// Create a payment request for a subscription
    pub fn create_payment_request(
        &self,
        subscription: &Subscription,
        expires_in_seconds: Option<u64>,
    ) -> Result<DemoPaymentRequest> {
        let mut request = PaymentRequest::new(
            subscription.provider.clone(),
            subscription.subscriber.clone(),
            subscription.terms.amount.clone(),
            subscription.terms.currency.clone(),
            subscription.terms.method.clone(),
        );

        if let Some(expires) = expires_in_seconds {
            let expires_at = chrono::Utc::now().timestamp() + expires as i64;
            request = request.with_expiration(expires_at);
        }

        Ok(DemoPaymentRequest { inner: request })
    }

    /// Configure auto-pay for a subscription
    pub fn configure_auto_pay(
        &self,
        subscription_id: String,
        peer: PublicKey,
        method_id: MethodId,
        max_amount_per_period: Option<i64>,
        period: String,
        enabled: bool,
    ) -> Result<AutoPayRule> {
        let mut rule = AutoPayRule::new(subscription_id, peer, method_id);
        rule.enabled = enabled;

        if let Some(max) = max_amount_per_period {
            rule = rule.with_max_period_amount(Amount::from_sats(max), period);
        }

        // Save rule
        tokio::runtime::Handle::try_current()
            .map(|handle| handle.block_on(self.storage.save_autopay_rule(&rule)))
            .unwrap_or_else(|_| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(self.storage.save_autopay_rule(&rule))
            })
            .context("Failed to save auto-pay rule")?;

        Ok(rule)
    }

    /// Set spending limit for a peer
    pub fn set_spending_limit(
        &self,
        peer: PublicKey,
        limit_sats: i64,
        period: String,
    ) -> Result<PeerSpendingLimit> {
        let limit = PeerSpendingLimit::new(peer, Amount::from_sats(limit_sats), period);

        // Save limit
        tokio::runtime::Handle::try_current()
            .map(|handle| handle.block_on(self.storage.save_peer_limit(&limit)))
            .unwrap_or_else(|_| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(self.storage.save_peer_limit(&limit))
            })
            .context("Failed to save spending limit")?;

        Ok(limit)
    }

    /// Get spending limit for a peer
    pub fn get_spending_limit(&self, peer: &PublicKey) -> Result<Option<PeerSpendingLimit>> {
        let result = tokio::runtime::Handle::try_current()
            .map(|handle| handle.block_on(self.storage.get_peer_limit(peer)))
            .unwrap_or_else(|_| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(self.storage.get_peer_limit(peer))
            })
            .context("Failed to get spending limit")?;

        Ok(result)
    }

    /// Check if an auto-pay would be approved for a peer/amount
    pub fn can_auto_pay(&self, peer: &PublicKey, amount: &Amount) -> Result<bool> {
        // Get the peer's spending limit
        let limit = self.get_spending_limit(peer)?;

        match limit {
            Some(limit) => {
                // Check if amount would exceed remaining limit
                Ok(!limit.would_exceed_limit(amount))
            }
            None => {
                // No limit set means auto-pay is not configured
                Ok(false)
            }
        }
    }

    /// Record an auto-payment (updates spending)
    pub fn record_auto_payment(&self, peer: &PublicKey, amount: Amount) -> Result<()> {
        let mut limit = self
            .get_spending_limit(peer)?
            .context("No spending limit set for peer")?;

        limit.add_spent(&amount).context("Failed to add spent")?;

        // Save updated limit
        tokio::runtime::Handle::try_current()
            .map(|handle| handle.block_on(self.storage.save_peer_limit(&limit)))
            .unwrap_or_else(|_| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(self.storage.save_peer_limit(&limit))
            })
            .context("Failed to save updated limit")?;

        Ok(())
    }

    /// Reset spending limit for a peer
    pub fn reset_spending_limit(&self, peer: &PublicKey) -> Result<()> {
        let mut limit = self
            .get_spending_limit(peer)?
            .context("No spending limit set for peer")?;

        limit.reset();

        // Save updated limit
        tokio::runtime::Handle::try_current()
            .map(|handle| handle.block_on(self.storage.save_peer_limit(&limit)))
            .unwrap_or_else(|_| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(self.storage.save_peer_limit(&limit))
            })
            .context("Failed to save reset limit")?;

        Ok(())
    }

    /// Get auto-pay rule for a subscription
    pub fn get_autopay_rule(&self, subscription_id: &str) -> Result<Option<AutoPayRule>> {
        let result = tokio::runtime::Handle::try_current()
            .map(|handle| handle.block_on(self.storage.get_autopay_rule(subscription_id)))
            .unwrap_or_else(|_| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(self.storage.get_autopay_rule(subscription_id))
            })
            .context("Failed to get auto-pay rule")?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pubky::Keypair;
    use tempfile::tempdir;

    #[test]
    fn test_coordinator_creation() {
        let temp_dir = tempdir().unwrap();
        let coordinator = SubscriptionCoordinator::new(temp_dir.path());
        assert!(coordinator.is_ok());
    }

    #[test]
    fn test_spending_limit_workflow() {
        let temp_dir = tempdir().unwrap();
        let coordinator = SubscriptionCoordinator::new(temp_dir.path()).unwrap();
        let peer = Keypair::random().public_key();

        // Set limit
        let limit = coordinator
            .set_spending_limit(peer.clone(), 10000, "daily".to_string())
            .unwrap();
        assert_eq!(limit.total_amount_limit.as_sats(), 10000);

        // Check auto-pay
        assert!(coordinator
            .can_auto_pay(&peer, &Amount::from_sats(5000))
            .unwrap());
        assert!(!coordinator
            .can_auto_pay(&peer, &Amount::from_sats(15000))
            .unwrap());

        // Record payment
        coordinator
            .record_auto_payment(&peer, Amount::from_sats(3000))
            .unwrap();

        // Check updated limit
        let updated = coordinator.get_spending_limit(&peer).unwrap().unwrap();
        assert_eq!(updated.current_spent.as_sats(), 3000);
    }
}
