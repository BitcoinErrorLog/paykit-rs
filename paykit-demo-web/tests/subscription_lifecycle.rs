#![cfg(target_arch = "wasm32")]
//! Subscription lifecycle integration tests
//!
//! Tests the complete subscription workflow including creation, storage, and management.

use paykit_demo_web::{Identity, WasmSubscription, WasmSubscriptionAgreementStorage};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_subscription_creation() {
    let alice = Identity::with_nickname("alice").unwrap();
    let bob = Identity::with_nickname("bob").unwrap();

    let subscription = WasmSubscription::new(
        &alice.public_key(),
        &bob.public_key(),
        "1000",
        "SAT",
        "monthly:1",
        "Test subscription",
    );

    assert!(subscription.is_ok());
    let sub = subscription.unwrap();
    assert_eq!(sub.amount(), "1000");
    assert_eq!(sub.currency(), "SAT");
}

#[wasm_bindgen_test]
async fn test_subscription_storage() {
    let storage = WasmSubscriptionAgreementStorage::new();

    let alice = Identity::with_nickname("alice").unwrap();
    let bob = Identity::with_nickname("bob").unwrap();

    let subscription = WasmSubscription::new(
        &alice.public_key(),
        &bob.public_key(),
        "1000",
        "SAT",
        "monthly:1",
        "Test subscription",
    )
    .unwrap();

    let sub_id = subscription.subscription_id();

    // Save subscription
    let result = storage.save_subscription(&subscription).await;
    assert!(result.is_ok());

    // Retrieve subscription
    let retrieved = storage.get_subscription(&sub_id).await;
    assert!(retrieved.is_ok());
    assert!(retrieved.unwrap().is_some());
}

#[wasm_bindgen_test]
async fn test_subscription_deletion() {
    let storage = WasmSubscriptionAgreementStorage::new();

    let alice = Identity::with_nickname("alice").unwrap();
    let bob = Identity::with_nickname("bob").unwrap();

    let subscription = WasmSubscription::new(
        &alice.public_key(),
        &bob.public_key(),
        "1000",
        "SAT",
        "weekly",
        "Test subscription",
    )
    .unwrap();

    let sub_id = subscription.subscription_id();

    // Save and delete
    storage.save_subscription(&subscription).await.unwrap();
    let result = storage.delete_subscription(&sub_id).await;
    assert!(result.is_ok());

    // Should not be retrievable
    let retrieved = storage.get_subscription(&sub_id).await.unwrap();
    assert!(retrieved.is_none());
}

#[wasm_bindgen_test]
fn test_subscription_frequency_parsing() {
    let alice = Identity::with_nickname("alice").unwrap();
    let bob = Identity::with_nickname("bob").unwrap();

    // Test daily
    let daily = WasmSubscription::new(
        &alice.public_key(),
        &bob.public_key(),
        "100",
        "SAT",
        "daily",
        "Daily sub",
    );
    assert!(daily.is_ok());

    // Test weekly
    let weekly = WasmSubscription::new(
        &alice.public_key(),
        &bob.public_key(),
        "700",
        "SAT",
        "weekly",
        "Weekly sub",
    );
    assert!(weekly.is_ok());

    // Test monthly with day
    let monthly = WasmSubscription::new(
        &alice.public_key(),
        &bob.public_key(),
        "3000",
        "SAT",
        "monthly:15",
        "Monthly sub",
    );
    assert!(monthly.is_ok());
}

#[wasm_bindgen_test]
fn test_subscription_invalid_frequency() {
    let alice = Identity::with_nickname("alice").unwrap();
    let bob = Identity::with_nickname("bob").unwrap();

    let result = WasmSubscription::new(
        &alice.public_key(),
        &bob.public_key(),
        "1000",
        "SAT",
        "invalid_frequency",
        "Test",
    );

    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn test_list_subscriptions() {
    let storage = WasmSubscriptionAgreementStorage::new();

    // Clear any existing subscriptions
    let _ = storage.clear_all().await;

    let alice = Identity::with_nickname("alice").unwrap();
    let bob = Identity::with_nickname("bob").unwrap();

    // Create multiple subscriptions
    for i in 1..=3 {
        let subscription = WasmSubscription::new(
            &alice.public_key(),
            &bob.public_key(),
            &format!("{}000", i),
            "SAT",
            "monthly:1",
            &format!("Subscription {}", i),
        )
        .unwrap();

        storage.save_subscription(&subscription).await.unwrap();
    }

    let list = storage.list_all_subscriptions().await;
    assert!(list.is_ok());
    // Should have at least 3 subscriptions
    assert!(list.unwrap().len() >= 3);
}

#[wasm_bindgen_test]
fn test_subscription_validation() {
    let alice = Identity::with_nickname("alice").unwrap();
    let bob = Identity::with_nickname("bob").unwrap();

    // Negative amount should fail
    let result = WasmSubscription::new(
        &alice.public_key(),
        &bob.public_key(),
        "-1000",
        "SAT",
        "monthly:1",
        "Test",
    );
    assert!(result.is_err());

    // Zero amount should fail
    let result = WasmSubscription::new(
        &alice.public_key(),
        &bob.public_key(),
        "0",
        "SAT",
        "monthly:1",
        "Test",
    );
    assert!(result.is_err());
}
