//! Subscription management using paykit-subscriptions
//!
//! Simplified subscription coordinator for demo applications

use anyhow::{Context, Result};
use paykit_lib::{MethodId, PublicKey};
use paykit_subscriptions::{
    AutoPayRule, FileSubscriptionStorage, PaymentFrequency, PaymentRequest, PeerSpendingLimit,
    Subscription, SubscriptionStorage, SubscriptionTerms,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// Demo wrapper around protocol subscription types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoSubscription {
    /// The underlying protocol subscription
    #[serde(flatten)]
    pub inner: Subscription,
    /// Human-readable description
    pub description: String,
    /// Whether auto-pay is enabled
    pub auto_pay_enabled: bool,
}

/// Demo wrapper around payment requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoPaymentRequest {
    /// The underlying protocol payment request
    #[serde(flatten)]
    pub inner: PaymentRequest,
    /// Human-readable description
    pub description: String,
}

/// Coordinates subscription flows using the subscription protocol
pub struct SubscriptionCoordinator {
    storage: Arc<Box<dyn SubscriptionStorage>>,
}

impl SubscriptionCoordinator {
    /// Create a new subscription coordinator
    ///
    /// # Arguments
    /// * `storage_path` - Path to store subscription data
    pub fn new(storage_path: impl AsRef<Path>) -> Result<Self> {
        let storage = FileSubscriptionStorage::new(storage_path.as_ref().to_path_buf())
            .context("Failed to create subscription storage")?;

        Ok(Self {
            storage: Arc::new(Box::new(storage)),
        })
    }

    /// Create a new subscription as the subscriber
    ///
    /// # Arguments
    /// * `subscriber` - The subscriber's public key
    /// * `provider` - The provider's public key
    /// * `amount_sats` - Amount in satoshis
    /// * `currency` - Currency code (e.g., "SAT", "BTC")
    /// * `frequency` - Payment frequency
    /// * `method` - Payment method to use
    /// * `description` - Human-readable description
    #[allow(clippy::too_many_arguments)]
    pub fn create_subscription(
        &self,
        subscriber: PublicKey,
        provider: PublicKey,
        amount_sats: i64,
        currency: String,
        frequency: PaymentFrequency,
        method: MethodId,
        description: String,
    ) -> Result<DemoSubscription> {
        let terms = SubscriptionTerms::new(
            paykit_subscriptions::Amount::from_sats(amount_sats),
            currency,
            frequency,
            method,
            description.clone(),
        );

        let subscription = Subscription::new(subscriber, provider, terms);

        Ok(DemoSubscription {
            inner: subscription,
            description,
            auto_pay_enabled: false,
        })
    }

    /// Save a subscription
    pub async fn save_subscription(&self, subscription: &Subscription) -> Result<()> {
        self.storage
            .save_subscription(subscription)
            .await
            .context("Failed to save subscription")?;
        Ok(())
    }

    /// Load all subscriptions
    pub async fn load_all_subscriptions(
        &self,
    ) -> Result<Vec<paykit_subscriptions::SignedSubscription>> {
        self.storage
            .list_active_subscriptions()
            .await
            .context("Failed to load subscriptions")
    }

    /// Create a payment request from a subscription
    ///
    /// # Arguments
    /// * `subscription` - The subscription to bill
    /// * `expiration_secs` - Seconds until the request expires (default: 3600 = 1 hour)
    pub fn create_payment_request(
        &self,
        subscription: &Subscription,
        expiration_secs: Option<i64>,
    ) -> Result<DemoPaymentRequest> {
        let expiration = expiration_secs.unwrap_or(3600);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let request = PaymentRequest::new(
            subscription.subscriber.clone(),
            subscription.provider.clone(),
            subscription.terms.amount,
            subscription.terms.currency.clone(),
            subscription.terms.method.clone(),
        )
        .with_description(subscription.terms.description.clone())
        .with_expiration(now + expiration);

        Ok(DemoPaymentRequest {
            inner: request,
            description: subscription.terms.description.clone(),
        })
    }

    /// Configure auto-pay rule for a subscription
    ///
    /// # Arguments
    /// * `subscription_id` - The subscription ID
    /// * `provider` - Provider's public key
    /// * `method` - Payment method
    /// * `max_amount_sats` - Maximum amount to auto-pay per period (optional)
    /// * `period` - Time period (e.g., "daily", "weekly", "monthly")
    /// * `enabled` - Whether to enable the rule
    pub fn configure_auto_pay(
        &self,
        subscription_id: String,
        provider: PublicKey,
        method: MethodId,
        max_amount_sats: Option<i64>,
        period: String,
        enabled: bool,
    ) -> Result<AutoPayRule> {
        let mut rule = AutoPayRule::new(subscription_id, provider, method);
        rule.enabled = enabled;

        if let Some(max_amount) = max_amount_sats {
            rule = rule.with_max_period_amount(
                paykit_subscriptions::Amount::from_sats(max_amount),
                period,
            );
        }

        Ok(rule)
    }

    /// Set spending limit for a peer
    ///
    /// # Arguments
    /// * `peer` - Peer's public key
    /// * `max_amount_sats` - Maximum amount to spend in the period
    /// * `period` - Time period (e.g., "daily", "weekly", "monthly")
    pub fn set_spending_limit(
        &self,
        peer: PublicKey,
        max_amount_sats: i64,
        period: String,
    ) -> Result<PeerSpendingLimit> {
        let limit = PeerSpendingLimit::new(
            peer,
            paykit_subscriptions::Amount::from_sats(max_amount_sats),
            period,
        );

        Ok(limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pubky::Keypair;

    #[test]
    fn test_create_subscription() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let coordinator =
            SubscriptionCoordinator::new(temp_dir.path()).expect("Failed to create coordinator");

        let subscriber = Keypair::random().public_key();
        let provider = Keypair::random().public_key();

        let subscription = coordinator
            .create_subscription(
                subscriber,
                provider,
                1000,
                "SAT".to_string(),
                PaymentFrequency::Monthly { day_of_month: 1 },
                MethodId("lightning".to_string()),
                "Monthly service".to_string(),
            )
            .expect("Failed to create subscription");

        assert_eq!(subscription.inner.terms.amount.as_sats(), 1000);
        assert_eq!(subscription.description, "Monthly service");
    }

    #[test]
    fn test_create_payment_request() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let coordinator =
            SubscriptionCoordinator::new(temp_dir.path()).expect("Failed to create coordinator");

        let subscriber = Keypair::random().public_key();
        let provider = Keypair::random().public_key();

        let demo_sub = coordinator
            .create_subscription(
                subscriber,
                provider,
                1000,
                "SAT".to_string(),
                PaymentFrequency::Monthly { day_of_month: 1 },
                MethodId("lightning".to_string()),
                "Monthly service".to_string(),
            )
            .expect("Failed to create subscription");

        let request = coordinator
            .create_payment_request(&demo_sub.inner, Some(3600))
            .expect("Failed to create payment request");

        assert_eq!(request.inner.amount.as_sats(), 1000);
        assert_eq!(request.description, "Monthly service");
    }

    #[test]
    fn test_configure_auto_pay() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let coordinator =
            SubscriptionCoordinator::new(temp_dir.path()).expect("Failed to create coordinator");

        let provider = Keypair::random().public_key();

        let rule = coordinator
            .configure_auto_pay(
                "sub_001".to_string(),
                provider,
                MethodId("lightning".to_string()),
                Some(5000),
                "daily".to_string(),
                true,
            )
            .expect("Failed to configure auto-pay");

        assert!(rule.enabled);
        assert_eq!(
            rule.max_total_amount_per_period.as_ref().unwrap().as_sats(),
            5000
        );
    }

    #[test]
    fn test_set_spending_limit() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let coordinator =
            SubscriptionCoordinator::new(temp_dir.path()).expect("Failed to create coordinator");

        let peer = Keypair::random().public_key();

        let limit = coordinator
            .set_spending_limit(peer.clone(), 10000, "daily".to_string())
            .expect("Failed to set spending limit");

        assert_eq!(limit.peer, peer);
        assert_eq!(limit.total_amount_limit.as_sats(), 10000);
        assert_eq!(limit.period, "daily");
    }
}
