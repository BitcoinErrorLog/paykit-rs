//! Background monitoring for subscription payments (native only - not available in WASM)
//!
//! This module requires tokio::time::sleep which is not available in WASM environments.
//! For WASM applications, implement monitoring logic using JavaScript timers or
//! web workers.

use crate::{
    subscription::PaymentFrequency, PaymentRequest, Result, SignedSubscription, SubscriptionManager,
};
use chrono::Datelike;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Background monitor for subscription payments
///
/// Note: Only available on native platforms. WASM applications should use
/// JavaScript-based timers for monitoring.
pub struct SubscriptionMonitor {
    manager: Arc<SubscriptionManager>,
    check_interval: Duration,
}

impl SubscriptionMonitor {
    /// Create a new monitor
    pub fn new(manager: Arc<SubscriptionManager>, check_interval: Duration) -> Self {
        Self {
            manager,
            check_interval,
        }
    }

    /// Create with default check interval (1 hour)
    pub fn with_default_interval(manager: Arc<SubscriptionManager>) -> Self {
        Self::new(manager, Duration::from_secs(3600))
    }

    /// Start monitoring loop
    pub async fn start(&self) -> Result<()> {
        loop {
            if let Err(e) = self.check_due_payments().await {
                eprintln!("Error checking due payments: {}", e);
            }

            sleep(self.check_interval).await;
        }
    }

    /// Check for due payments once
    pub async fn check_due_payments(&self) -> Result<Vec<PaymentRequest>> {
        let now = chrono::Utc::now().timestamp();
        let subscriptions = self.manager.storage().list_active_subscriptions().await?;

        let mut due_requests = Vec::new();

        for sub in subscriptions {
            if self.is_payment_due(&sub, now) {
                // Generate payment request for this subscription
                let request = self.generate_payment_request(&sub)?;

                // Save locally (will be sent when peer connects)
                self.manager.storage().save_request(&request).await?;

                due_requests.push(request);
            }
        }

        Ok(due_requests)
    }

    /// Check if payment is due for a subscription
    fn is_payment_due(&self, subscription: &SignedSubscription, now: i64) -> bool {
        let terms = &subscription.subscription.terms;

        // Check if subscription is active
        if now < subscription.subscription.starts_at {
            return false;
        }

        if let Some(end) = subscription.subscription.ends_at {
            if now >= end {
                return false;
            }
        }

        // For now, simplified: check based on frequency
        // In production, this would track last payment time
        match terms.frequency {
            PaymentFrequency::Daily => {
                // Check if we're due for a daily payment
                // This is simplified - would need payment history
                true
            }
            PaymentFrequency::Weekly => {
                // Check if we're due for a weekly payment
                true
            }
            PaymentFrequency::Monthly { day_of_month } => {
                // Check if today is the payment day
                let date =
                    chrono::DateTime::from_timestamp(now, 0).unwrap_or_else(chrono::Utc::now);
                date.day() == day_of_month as u32
            }
            PaymentFrequency::Yearly { month, day } => {
                // Check if today is the annual payment day
                let date =
                    chrono::DateTime::from_timestamp(now, 0).unwrap_or_else(chrono::Utc::now);
                date.month() == month as u32 && date.day() == day as u32
            }
            PaymentFrequency::Custom { interval_seconds } => {
                // This would require tracking last payment time
                // For now, assume due
                let _ = interval_seconds;
                true
            }
        }
    }

    /// Generate a payment request from a subscription
    fn generate_payment_request(
        &self,
        subscription: &SignedSubscription,
    ) -> Result<PaymentRequest> {
        let sub = &subscription.subscription;
        let terms = &sub.terms;

        let request = PaymentRequest::new(
            sub.provider.clone(),
            sub.subscriber.clone(),
            terms.amount.clone(),
            terms.currency.clone(),
            terms.method.clone(),
        )
        .with_description(format!("Subscription payment: {}", terms.description));

        Ok(request)
    }

    /// Get manager reference
    pub fn manager(&self) -> &Arc<SubscriptionManager> {
        &self.manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        signing,
        storage::{FileSubscriptionStorage, SubscriptionStorage},
        subscription::{SignedSubscription, Subscription, SubscriptionTerms},
        Amount,
    };
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

    fn test_pubkey() -> PublicKey {
        let keypair = pkarr::Keypair::random();
        PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
    }

    fn create_test_subscription(monthly_day: u8) -> SignedSubscription {
        let subscriber = test_pubkey();
        let provider = test_pubkey();

        let terms = SubscriptionTerms::new(
            Amount::from_sats(100),
            "SAT".to_string(),
            PaymentFrequency::Monthly {
                day_of_month: monthly_day,
            },
            MethodId("lightning".to_string()),
            "Test subscription".to_string(),
        );

        let subscription = Subscription::new(subscriber, provider, terms);
        let keypair = pkarr::Keypair::random();
        let nonce1 = rand::random();
        let nonce2 = rand::random();

        let sig1 =
            signing::sign_subscription_ed25519(&subscription, &keypair, &nonce1, 3600).unwrap();
        let sig2 =
            signing::sign_subscription_ed25519(&subscription, &keypair, &nonce2, 3600).unwrap();

        SignedSubscription::new(subscription, sig1, sig2)
    }

    #[tokio::test]
    async fn test_monitor_creation() {
        let temp_dir = tempdir().unwrap();
        let storage: Arc<Box<dyn SubscriptionStorage>> = Arc::new(Box::new(
            FileSubscriptionStorage::new(temp_dir.path().to_path_buf()).unwrap(),
        ));

        let mock_storage: Arc<Box<dyn PaykitStorage>> = Arc::new(Box::new(MockStorage));
        let mock_generator: Arc<Box<dyn ReceiptGenerator>> = Arc::new(Box::new(MockGenerator));
        let interactive = Arc::new(PaykitInteractiveManager::new(mock_storage, mock_generator));

        let manager = Arc::new(SubscriptionManager::new(storage, interactive));
        let monitor = SubscriptionMonitor::with_default_interval(manager);

        assert_eq!(monitor.check_interval, Duration::from_secs(3600));
    }

    #[tokio::test]
    async fn test_payment_due_detection() {
        let temp_dir = tempdir().unwrap();
        let storage: Arc<Box<dyn SubscriptionStorage>> = Arc::new(Box::new(
            FileSubscriptionStorage::new(temp_dir.path().to_path_buf()).unwrap(),
        ));

        let mock_storage: Arc<Box<dyn PaykitStorage>> = Arc::new(Box::new(MockStorage));
        let mock_generator: Arc<Box<dyn ReceiptGenerator>> = Arc::new(Box::new(MockGenerator));
        let interactive = Arc::new(PaykitInteractiveManager::new(mock_storage, mock_generator));

        let manager = Arc::new(SubscriptionManager::new(storage.clone(), interactive));
        let monitor = SubscriptionMonitor::with_default_interval(manager);

        // Create subscription for today's day of month
        let now = chrono::Utc::now();
        let today_day = now.day() as u8;

        let subscription = create_test_subscription(today_day);

        // Save subscription
        storage
            .save_signed_subscription(&subscription)
            .await
            .unwrap();

        // Check due payments
        let due_requests = monitor.check_due_payments().await.unwrap();

        // Should have generated a request for today
        assert_eq!(due_requests.len(), 1);
        assert_eq!(due_requests[0].amount, Amount::from_sats(100));
        assert_eq!(due_requests[0].currency, "SAT");
    }
}
