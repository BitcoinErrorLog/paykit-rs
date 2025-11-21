//! Integration tests for subscription flows

use paykit_demo_core::{DemoSubscription, SubscriptionCoordinator};
use paykit_lib::MethodId;
use paykit_subscriptions::PaymentFrequency;
use pubky::Keypair;

#[test]
fn test_subscription_creation_and_storage() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let coordinator =
        SubscriptionCoordinator::new(temp_dir.path()).expect("Failed to create coordinator");

    let subscriber = Keypair::random().public_key();
    let provider = Keypair::random().public_key();

    // Create subscription
    let subscription = coordinator
        .create_subscription(
            subscriber.clone(),
            provider.clone(),
            1000,
            "SAT".to_string(),
            PaymentFrequency::Monthly { day_of_month: 1 },
            MethodId("lightning".to_string()),
            "Monthly premium subscription".to_string(),
        )
        .expect("Failed to create subscription");

    assert_eq!(subscription.inner.subscriber, subscriber);
    assert_eq!(subscription.inner.provider, provider);
    assert_eq!(subscription.inner.terms.amount.as_sats(), 1000);
    assert_eq!(subscription.description, "Monthly premium subscription");
}

#[test]
fn test_payment_request_from_subscription() {
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
            "Test subscription".to_string(),
        )
        .expect("Failed to create subscription");

    // Create payment request
    let request = coordinator
        .create_payment_request(&subscription.inner, Some(7200))
        .expect("Failed to create payment request");

    assert_eq!(request.inner.amount.as_sats(), 1000);
    assert_eq!(request.inner.currency, "SAT");
    assert!(request.inner.expires_at.is_some());
}

#[test]
fn test_auto_pay_configuration() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let coordinator =
        SubscriptionCoordinator::new(temp_dir.path()).expect("Failed to create coordinator");

    let provider = Keypair::random().public_key();

    // Configure auto-pay
    let rule = coordinator
        .configure_auto_pay(
            "sub_123".to_string(),
            provider.clone(),
            MethodId("lightning".to_string()),
            Some(5000),
            "daily".to_string(),
            true,
        )
        .expect("Failed to configure auto-pay");

    assert!(rule.enabled);
    assert_eq!(rule.subscription_id, "sub_123");
    assert_eq!(rule.peer, provider);
    assert_eq!(
        rule.max_total_amount_per_period
            .as_ref()
            .unwrap()
            .as_sats(),
        5000
    );
}

#[test]
fn test_spending_limits() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let coordinator =
        SubscriptionCoordinator::new(temp_dir.path()).expect("Failed to create coordinator");

    let peer = Keypair::random().public_key();

    // Set daily spending limit
    let limit = coordinator
        .set_spending_limit(peer.clone(), 10000, "daily".to_string())
        .expect("Failed to set spending limit");

    assert_eq!(limit.peer, peer);
    assert_eq!(limit.total_amount_limit.as_sats(), 10000);
    assert_eq!(limit.period, "daily");

    // Set weekly spending limit
    let weekly_limit = coordinator
        .set_spending_limit(peer.clone(), 50000, "weekly".to_string())
        .expect("Failed to set weekly limit");

    assert_eq!(weekly_limit.total_amount_limit.as_sats(), 50000);
    assert_eq!(weekly_limit.period, "weekly");
}
