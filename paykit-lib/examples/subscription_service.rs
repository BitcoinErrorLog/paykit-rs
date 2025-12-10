//! Subscription Service Example
//!
//! This example demonstrates a subscription service provider:
//! - Provider setup and configuration
//! - Subscriber enrollment
//! - Auto-pay configuration
//! - Billing cycle execution
//! - Subscription modifications
//!
//! # Usage
//!
//! ```bash
//! cargo run --example subscription-service
//! ```

use paykit_subscriptions::{
    Amount, PaymentFrequency, SignedSubscription, Subscription, SubscriptionManager,
    SubscriptionTerms,
};
use paykit_lib::{MethodId, PublicKey};
use paykit_subscriptions::fallback::{FallbackHandler, SubscriptionFallbackPolicy};
use paykit_subscriptions::modifications::ModificationRequest;
use paykit_subscriptions::proration::ProrationCalculator;
use std::collections::HashMap;
use std::str::FromStr;

/// Subscription service provider.
struct SubscriptionProvider {
    provider_key: PublicKey,
    subscriptions: HashMap<String, SignedSubscription>,
}

impl SubscriptionProvider {
    fn new(provider_key: PublicKey) -> Self {
        Self {
            provider_key,
            subscriptions: HashMap::new(),
        }
    }

    /// Enroll a new subscriber.
    fn enroll_subscriber(
        &mut self,
        subscriber: PublicKey,
        terms: SubscriptionTerms,
    ) -> Result<Subscription, String> {
        let subscription = Subscription::new(subscriber, self.provider_key.clone(), terms);

        // In a real implementation, both parties would sign using sign_subscription_ed25519
        // For this example, we'll just store the unsigned subscription
        // and demonstrate the modification flow

        self.subscriptions
            .insert(subscription.subscription_id.clone(), 
                SignedSubscription::new(
                    subscription.clone(),
                    // Placeholder signatures - in real usage, these would be generated
                    paykit_subscriptions::Signature {
                        signature: vec![0; 64],
                        public_key: [0; 32],
                        nonce: [0; 32],
                        timestamp: chrono::Utc::now().timestamp(),
                        expires_at: chrono::Utc::now().timestamp() + 86400,
                    },
                    paykit_subscriptions::Signature {
                        signature: vec![0; 64],
                        public_key: [0; 32],
                        nonce: [0; 32],
                        timestamp: chrono::Utc::now().timestamp(),
                        expires_at: chrono::Utc::now().timestamp() + 86400,
                    },
                ));
        Ok(subscription)
    }

    /// Execute a billing cycle.
    async fn execute_billing_cycle(
        &self,
        subscription_id: &str,
    ) -> Result<(), String> {
        let signed_sub = self
            .subscriptions
            .get(subscription_id)
            .ok_or_else(|| "Subscription not found".to_string())?;

        println!("Executing billing for subscription: {}", subscription_id);
        println!("  Amount: {} {}", signed_sub.subscription.terms.amount, signed_sub.subscription.terms.currency);
        println!("  Method: {}", signed_sub.subscription.terms.method.0);

        // In a real implementation, this would:
        // 1. Use FallbackHandler to execute payment with fallback
        // 2. Handle payment failures
        // 3. Update subscription status

        Ok(())
    }

    /// Process a subscription modification.
    fn process_modification(
        &mut self,
        request: ModificationRequest,
    ) -> Result<Subscription, String> {
        let signed_sub = self
            .subscriptions
            .get(&request.subscription_id)
            .ok_or_else(|| "Subscription not found".to_string())?;

        let modified = request.apply(&signed_sub.subscription.clone())
            .map_err(|e| e.to_string())?;

        // Update stored subscription
        let signed = SignedSubscription::new(
            modified.clone(),
            signed_sub.subscriber_signature.clone(),
            signed_sub.provider_signature.clone(),
        );
        self.subscriptions
            .insert(modified.subscription_id.clone(), signed);

        Ok(modified)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Paykit Subscription Service Example ===\n");

    // Setup provider
    let provider_key = PublicKey::from_str("provider_pubkey_123").unwrap();
    let mut provider = SubscriptionProvider::new(provider_key.clone());

    // Enroll subscriber
    println!("Step 1: Enrolling subscriber");
    let subscriber_key = PublicKey::from_str("subscriber_pubkey_456").unwrap();
    let terms = SubscriptionTerms::new(
        Amount::from_sats(1000),
        "SAT".to_string(),
        PaymentFrequency::Monthly { day_of_month: 1 },
        MethodId("lightning".to_string()),
        "Monthly premium subscription".to_string(),
    );

    let subscription = provider.enroll_subscriber(subscriber_key.clone(), terms)?;
    println!("  Subscription ID: {}", subscription.subscription_id);
    println!("  Amount: {} {}", subscription.terms.amount, subscription.terms.currency);
    println!("  Frequency: {:?}", subscription.terms.frequency);
    println!();

    // Execute billing cycle
    println!("Step 2: Executing billing cycle");
    provider
        .execute_billing_cycle(&subscription.subscription_id)
        .await?;
    println!();

    // Setup fallback handler
    println!("Step 3: Setting up fallback handler");
    let fallback_policy = SubscriptionFallbackPolicy::default()
        .with_method(MethodId("lightning".to_string()), 1)
        .with_method(MethodId("onchain".to_string()), 2);
    let fallback_handler = FallbackHandler::with_defaults();
    println!("  Fallback methods configured");
    println!();

    // Process upgrade modification
    println!("Step 4: Processing subscription upgrade");
    let upgrade_request = ModificationRequest::upgrade(
        &subscription,
        Amount::from_sats(2000),
        chrono::Utc::now().timestamp(),
    );
    
    // Calculate proration
    let calculator = ProrationCalculator::new();
    let period_start = chrono::Utc::now().timestamp() - (15 * 86400);
    let period_end = chrono::Utc::now().timestamp() + (15 * 86400);
    let proration = calculator.calculate(
        &subscription.terms.amount,
        &Amount::from_sats(2000),
        period_start,
        period_end,
        chrono::Utc::now().timestamp(),
        "SAT",
    )?;
    println!("  Proration:");
    println!("    Credit: {} SAT", proration.credit);
    println!("    Charge: {} SAT", proration.charge);
    println!("    Net: {} SAT", proration.net_amount);

    let modified = provider.process_modification(upgrade_request)?;
    println!("  Upgraded subscription amount: {} SAT", modified.terms.amount);
    println!();

    println!("=== Example Complete ===");
    Ok(())
}
