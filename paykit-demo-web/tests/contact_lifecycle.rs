//! Integration tests for contact management
//!
//! These tests verify the full contact lifecycle and workflows.

use paykit_demo_web::{Identity, WasmContact, WasmContactStorage};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Generate a valid test public key using Identity
fn generate_test_pubkey() -> String {
    Identity::new().unwrap().public_key()
}

/// Test full lifecycle: create → save → retrieve → update → delete
#[wasm_bindgen_test]
async fn test_contact_full_lifecycle() {
    let storage = WasmContactStorage::new();
    let test_pubkey = generate_test_pubkey();

    // Clean up any existing test data
    let _ = storage.delete_contact(&test_pubkey).await;

    // Create a contact
    let contact = WasmContact::new(test_pubkey.clone(), "Alice".to_string())
        .unwrap()
        .with_notes("Test contact".to_string());

    // Save it
    storage.save_contact(&contact).await.unwrap();

    // Retrieve it
    let retrieved = storage.get_contact(&test_pubkey).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name(), "Alice");
    assert_eq!(retrieved.notes(), Some("Test contact".to_string()));

    // Update it (save with new data)
    let updated = WasmContact::new(test_pubkey.clone(), "Alice Updated".to_string())
        .unwrap()
        .with_notes("Updated notes".to_string());
    storage.save_contact(&updated).await.unwrap();

    // Verify update
    let retrieved = storage.get_contact(&test_pubkey).await.unwrap().unwrap();
    assert_eq!(retrieved.name(), "Alice Updated");
    assert_eq!(retrieved.notes(), Some("Updated notes".to_string()));

    // Delete it
    storage.delete_contact(&test_pubkey).await.unwrap();

    // Verify deletion
    let retrieved = storage.get_contact(&test_pubkey).await.unwrap();
    assert!(retrieved.is_none());
}

/// Test managing multiple contacts
#[wasm_bindgen_test]
async fn test_multiple_contacts_management() {
    let storage = WasmContactStorage::new();
    let pubkey1 = generate_test_pubkey();
    let pubkey2 = generate_test_pubkey();
    let pubkey3 = generate_test_pubkey();

    // Clean up
    let _ = storage.delete_contact(&pubkey1).await;
    let _ = storage.delete_contact(&pubkey2).await;
    let _ = storage.delete_contact(&pubkey3).await;

    // Create three contacts
    let contact1 = WasmContact::new(pubkey1.clone(), "Alice".to_string()).unwrap();
    let contact2 = WasmContact::new(pubkey2.clone(), "Bob".to_string()).unwrap();
    let contact3 = WasmContact::new(pubkey3.clone(), "Charlie".to_string()).unwrap();

    storage.save_contact(&contact1).await.unwrap();
    storage.save_contact(&contact2).await.unwrap();
    storage.save_contact(&contact3).await.unwrap();

    // List all contacts
    let contacts = storage.list_contacts().await.unwrap();
    assert!(contacts.len() >= 3);

    // Verify sorting (Alice, Bob, Charlie alphabetically)
    // Note: Other tests may have created contacts, so we just verify our contacts exist

    // Search for specific contacts
    let alice_results = storage.search_contacts("alice").await.unwrap();
    assert!(!alice_results.is_empty());

    let bob_results = storage.search_contacts("bob").await.unwrap();
    assert!(!bob_results.is_empty());

    // Clean up
    storage.delete_contact(&pubkey1).await.unwrap();
    storage.delete_contact(&pubkey2).await.unwrap();
    storage.delete_contact(&pubkey3).await.unwrap();
}

/// Test payment history tracking
#[wasm_bindgen_test]
async fn test_payment_history_tracking() {
    let storage = WasmContactStorage::new();
    let test_pubkey = generate_test_pubkey();

    // Clean up
    let _ = storage.delete_contact(&test_pubkey).await;

    // Create a contact
    let contact = WasmContact::new(test_pubkey.clone(), "Alice".to_string()).unwrap();
    storage.save_contact(&contact).await.unwrap();

    // Add payment history
    storage
        .update_payment_history(&test_pubkey, "receipt_001")
        .await
        .unwrap();
    storage
        .update_payment_history(&test_pubkey, "receipt_002")
        .await
        .unwrap();
    storage
        .update_payment_history(&test_pubkey, "receipt_003")
        .await
        .unwrap();

    // Retrieve and verify history
    let retrieved = storage.get_contact(&test_pubkey).await.unwrap().unwrap();
    let history = retrieved.payment_history();
    assert_eq!(history.len(), 3);

    // Clean up
    storage.delete_contact(&test_pubkey).await.unwrap();
}

/// Test empty storage edge cases
#[wasm_bindgen_test]
async fn test_empty_storage_edge_cases() {
    let storage = WasmContactStorage::new();

    // Getting non-existent contact should return None
    let result = storage.get_contact("nonexistent_pubkey").await.unwrap();
    assert!(result.is_none());

    // Searching with no contacts should return empty vec
    let results = storage.search_contacts("nonexistent").await.unwrap();
    assert!(results.is_empty());

    // Deleting non-existent contact should not error
    let result = storage.delete_contact("nonexistent_pubkey").await;
    assert!(result.is_ok());
}

/// Test search functionality with various queries
#[wasm_bindgen_test]
async fn test_search_functionality() {
    let storage = WasmContactStorage::new();
    let pubkey1 = generate_test_pubkey();
    let pubkey2 = generate_test_pubkey();

    // Clean up
    let _ = storage.delete_contact(&pubkey1).await;
    let _ = storage.delete_contact(&pubkey2).await;

    // Create contacts with specific names for searching
    let contact1 = WasmContact::new(pubkey1.clone(), "Alice Smith".to_string()).unwrap();
    let contact2 = WasmContact::new(pubkey2.clone(), "Bob Anderson".to_string()).unwrap();

    storage.save_contact(&contact1).await.unwrap();
    storage.save_contact(&contact2).await.unwrap();

    // Test case-insensitive search
    let results = storage.search_contacts("alice").await.unwrap();
    assert!(!results.is_empty());

    let results = storage.search_contacts("ALICE").await.unwrap();
    assert!(!results.is_empty());

    // Test partial match
    let results = storage.search_contacts("smith").await.unwrap();
    assert!(!results.is_empty());

    let results = storage.search_contacts("anderson").await.unwrap();
    assert!(!results.is_empty());

    // Test non-matching query
    let results = storage.search_contacts("xyz123").await.unwrap();
    assert!(results.is_empty());

    // Clean up
    storage.delete_contact(&pubkey1).await.unwrap();
    storage.delete_contact(&pubkey2).await.unwrap();
}

/// Test contact persistence across storage instances
#[wasm_bindgen_test]
async fn test_contact_persistence() {
    let storage1 = WasmContactStorage::new();
    let test_pubkey = generate_test_pubkey();

    // Clean up
    let _ = storage1.delete_contact(&test_pubkey).await;

    // Save with first storage instance
    let contact = WasmContact::new(test_pubkey.clone(), "Alice".to_string()).unwrap();
    storage1.save_contact(&contact).await.unwrap();

    // Create new storage instance
    let storage2 = WasmContactStorage::new();

    // Retrieve with second instance
    let retrieved = storage2.get_contact(&test_pubkey).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name(), "Alice");

    // Clean up
    storage1.delete_contact(&test_pubkey).await.unwrap();
}
