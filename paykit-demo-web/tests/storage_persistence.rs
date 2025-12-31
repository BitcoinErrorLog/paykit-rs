#![cfg(target_arch = "wasm32")]
//! Storage persistence tests
//!
//! Tests localStorage operations for identities, subscriptions, and receipts.

use paykit_demo_web::{
    BrowserStorage, Identity, WasmRequestStorage, WasmSubscriptionAgreementStorage,
};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_identity_storage() {
    let storage = BrowserStorage::new();

    let identity = Identity::with_nickname("test_user").unwrap();
    let name = "test_identity";

    // Save identity
    let result = storage.save_identity(name, &identity);
    assert!(result.is_ok());

    // Load identity
    let loaded = storage.load_identity(name);
    assert!(loaded.is_ok());
    assert_eq!(loaded.unwrap().public_key(), identity.public_key());
}

#[wasm_bindgen_test]
fn test_identity_list() {
    let storage = BrowserStorage::new();

    // Create and save multiple identities
    for i in 1..=3 {
        let identity = Identity::with_nickname(&format!("user_{}", i)).unwrap();
        let name = format!("identity_{}", i);
        storage.save_identity(&name, &identity).unwrap();
    }

    // List all identities
    let list = storage.list_identities().unwrap();
    assert!(list.len() >= 3);
}

#[wasm_bindgen_test]
fn test_identity_deletion() {
    let storage = BrowserStorage::new();

    let identity = Identity::with_nickname("temp_user").unwrap();
    let name = "temp_identity";

    // Save and delete
    storage.save_identity(name, &identity).unwrap();
    let result = storage.delete_identity(name);
    assert!(result.is_ok());

    // Should not be loadable
    let loaded = storage.load_identity(name);
    assert!(loaded.is_err());
}

#[wasm_bindgen_test]
fn test_current_identity() {
    let storage = BrowserStorage::new();

    let identity = Identity::with_nickname("current_user").unwrap();
    let name = "current_identity";

    // Save identity
    storage.save_identity(name, &identity).unwrap();

    // Set as current
    let result = storage.set_current_identity(name);
    assert!(result.is_ok());

    // Verify current
    let current = storage.get_current_identity();
    assert!(current.is_ok());
    assert_eq!(current.unwrap(), Some(name.to_string()));
}

#[wasm_bindgen_test]
async fn test_subscription_storage_persistence() {
    let storage = WasmSubscriptionAgreementStorage::new();

    let alice = Identity::with_nickname("alice").unwrap();
    let bob = Identity::with_nickname("bob").unwrap();

    let subscription = paykit_demo_web::WasmSubscription::new(
        &alice.public_key(),
        &bob.public_key(),
        "1000",
        "SAT",
        "monthly:1",
        "Persistence test",
    )
    .unwrap();

    let sub_id = subscription.subscription_id();

    // Save subscription
    storage.save_subscription(&subscription).await.unwrap();

    // Create new storage instance to verify persistence
    let new_storage = WasmSubscriptionAgreementStorage::new();
    let loaded = new_storage.get_subscription(&sub_id).await.unwrap();

    assert!(loaded.is_some());
}

#[wasm_bindgen_test]
async fn test_storage_clear_all() {
    let storage = WasmSubscriptionAgreementStorage::new();

    let alice = Identity::with_nickname("alice").unwrap();
    let bob = Identity::with_nickname("bob").unwrap();

    // Create multiple subscriptions
    for i in 1..=3 {
        let subscription = paykit_demo_web::WasmSubscription::new(
            &alice.public_key(),
            &bob.public_key(),
            &format!("{}000", i),
            "SAT",
            "monthly:1",
            &format!("Sub {}", i),
        )
        .unwrap();

        storage.save_subscription(&subscription).await.unwrap();
    }

    // Clear all
    let result = storage.clear_all().await;
    assert!(result.is_ok());

    // List should be empty
    let list = storage.list_all_subscriptions().await.unwrap();
    assert_eq!(list.len(), 0);
}

#[wasm_bindgen_test]
fn test_identity_json_serialization() {
    let identity = Identity::with_nickname("json_test").unwrap();

    // Export to JSON
    let json = identity.to_json();
    assert!(json.is_ok());

    // Import from JSON
    let restored = Identity::from_json(&json.unwrap());
    assert!(restored.is_ok());

    // Verify match
    assert_eq!(identity.public_key(), restored.unwrap().public_key());
}

#[wasm_bindgen_test]
fn test_identity_invalid_json() {
    let result = Identity::from_json("invalid json");
    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn test_payment_request_storage() {
    let storage = WasmRequestStorage::new(None);

    let alice = Identity::with_nickname("alice").unwrap();
    let bob = Identity::with_nickname("bob").unwrap();

    let request = paykit_demo_web::WasmPaymentRequest::new(
        &alice.public_key(),
        &bob.public_key(),
        "500",
        "SAT",
        "lightning",
    )
    .unwrap();

    // Save request
    let result = storage.save_request(&request).await;
    assert!(result.is_ok());

    // List requests
    let requests = storage.list_requests().await;
    assert!(requests.is_ok());
    assert!(!requests.unwrap().is_empty());
}
