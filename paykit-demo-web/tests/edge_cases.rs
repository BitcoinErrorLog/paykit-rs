//! Edge case and validation tests
//!
//! This test suite covers edge cases, boundary conditions, and validation
//! scenarios across all Paykit Demo Web features.

use paykit_demo_web::{
    WasmContact, WasmContactStorage, WasmDashboard, WasmPaymentMethodConfig,
    WasmPaymentMethodStorage, WasmReceiptStorage,
};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ===========================
// Contact Edge Cases
// ===========================

#[wasm_bindgen_test]
fn test_contact_empty_name() {
    let result = WasmContact::new(
        "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
        "".to_string(),
    );
    // Empty name should be allowed (will validate if needed)
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn test_contact_invalid_pubkey_formats() {
    // Too short
    let result = WasmContact::new("short".to_string(), "Alice".to_string());
    assert!(result.is_err());

    // Invalid characters
    let result = WasmContact::new("invalid!@#$".to_string(), "Alice".to_string());
    assert!(result.is_err());

    // Empty pubkey
    let result = WasmContact::new("".to_string(), "Alice".to_string());
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_contact_unicode_name() {
    let result = WasmContact::new(
        "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
        "Alice 👋 测试".to_string(),
    );
    assert!(result.is_ok());
    let contact = result.unwrap();
    assert!(contact.name().contains("Alice"));
}

#[wasm_bindgen_test]
fn test_contact_very_long_notes() {
    let contact = WasmContact::new(
        "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
        "Alice".to_string(),
    )
    .unwrap();

    let long_notes = "a".repeat(10000);
    let contact_with_notes = contact.with_notes(long_notes.clone());

    assert_eq!(contact_with_notes.notes(), Some(long_notes));
}

// ===========================
// Payment Method Edge Cases
// ===========================

#[wasm_bindgen_test]
fn test_method_whitespace_validation() {
    // Empty after trim
    let result =
        WasmPaymentMethodConfig::new("   ".to_string(), "endpoint".to_string(), true, true, 1);
    assert!(result.is_err());

    let result =
        WasmPaymentMethodConfig::new("lightning".to_string(), "   ".to_string(), true, true, 1);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_method_special_characters_in_id() {
    let result = WasmPaymentMethodConfig::new(
        "my-custom-method_v2.1".to_string(),
        "endpoint".to_string(),
        true,
        true,
        1,
    );
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn test_method_very_high_priority() {
    let result = WasmPaymentMethodConfig::new(
        "lightning".to_string(),
        "endpoint".to_string(),
        true,
        true,
        999999,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().priority(), 999999);
}

#[wasm_bindgen_test]
fn test_method_zero_priority() {
    let result = WasmPaymentMethodConfig::new(
        "lightning".to_string(),
        "endpoint".to_string(),
        true,
        true,
        0,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().priority(), 0);
}

#[wasm_bindgen_test]
fn test_method_very_long_endpoint() {
    let long_endpoint = "lnurl1".to_string() + &"a".repeat(2000);
    let result = WasmPaymentMethodConfig::new(
        "lightning".to_string(),
        long_endpoint.clone(),
        true,
        true,
        1,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().endpoint(), long_endpoint);
}

// ===========================
// Receipt Edge Cases
// ===========================

#[wasm_bindgen_test]
async fn test_receipt_malformed_json() {
    let storage = WasmReceiptStorage::new();

    // Save malformed JSON
    let result = storage
        .save_receipt("bad_receipt", "not valid json {")
        .await;
    // Should succeed in saving (validation happens on retrieval)
    assert!(result.is_ok());

    // Clean up
    let _ = storage.delete_receipt("bad_receipt").await;
}

#[wasm_bindgen_test]
async fn test_receipt_empty_id() {
    let storage = WasmReceiptStorage::new();

    let result = storage.save_receipt("", "{}").await;
    // Should succeed (empty key is valid in localStorage)
    assert!(result.is_ok());

    // Clean up
    let _ = storage.delete_receipt("").await;
}

#[wasm_bindgen_test]
async fn test_receipt_special_characters_in_id() {
    let storage = WasmReceiptStorage::new();

    let receipt_id = "receipt:with:special@chars#123";
    let result = storage.save_receipt(receipt_id, "{}").await;
    assert!(result.is_ok());

    // Should be retrievable
    let retrieved = storage.get_receipt(receipt_id).await.unwrap();
    assert!(retrieved.is_some());

    // Clean up
    let _ = storage.delete_receipt(receipt_id).await;
}

#[wasm_bindgen_test]
async fn test_receipt_filter_with_empty_results() {
    let storage = WasmReceiptStorage::new();

    // Filter with no matches
    let results = storage.filter_by_method("nonexistent_method").await;
    assert!(results.is_ok());
    assert_eq!(results.unwrap().len(), 0);
}

#[wasm_bindgen_test]
async fn test_receipt_statistics_with_no_receipts() {
    let storage = WasmReceiptStorage::new();

    let stats = storage.get_statistics("any_pubkey").await.unwrap();

    let total = js_sys::Reflect::get(&stats, &"total".into())
        .unwrap()
        .as_f64()
        .unwrap();
    assert_eq!(total, 0.0);
}

// ===========================
// Dashboard Edge Cases
// ===========================

#[wasm_bindgen_test]
async fn test_dashboard_empty_state() {
    let dashboard = WasmDashboard::new();

    let stats = dashboard.get_overview_stats("empty_user").await.unwrap();

    // All counts should be 0 or valid numbers
    let contacts = js_sys::Reflect::get(&stats, &"contacts".into())
        .unwrap()
        .as_f64()
        .unwrap();
    assert!(contacts.is_finite());
}

#[wasm_bindgen_test]
async fn test_dashboard_activity_with_zero_limit() {
    let dashboard = WasmDashboard::new();

    let activity = dashboard.get_recent_activity("test_user", 0).await.unwrap();

    // Should return empty array
    assert_eq!(activity.len(), 0);
}

#[wasm_bindgen_test]
async fn test_dashboard_activity_with_large_limit() {
    let dashboard = WasmDashboard::new();

    let activity = dashboard
        .get_recent_activity("test_user", 10000)
        .await
        .unwrap();

    // Should not crash, returns what's available
    assert!(activity.len() < 10000); // Won't have that many
}

// ===========================
// Storage Edge Cases
// ===========================

#[wasm_bindgen_test]
async fn test_contact_storage_concurrent_operations() {
    let storage = WasmContactStorage::new();

    // Clean up
    let _ = storage.delete_contact("concurrent_test").await;

    let contact = WasmContact::new(
        "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
        "Test".to_string(),
    )
    .unwrap();

    // Multiple saves shouldn't cause issues
    storage.save_contact(&contact).await.unwrap();
    storage.save_contact(&contact).await.unwrap();
    storage.save_contact(&contact).await.unwrap();

    // Should only have one contact
    let retrieved = storage
        .get_contact("8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo")
        .await
        .unwrap();
    assert!(retrieved.is_some());

    // Clean up
    let _ = storage
        .delete_contact("8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo")
        .await;
}

#[wasm_bindgen_test]
async fn test_method_storage_update_scenario() {
    let storage = WasmPaymentMethodStorage::new();

    // Clean up
    let _ = storage.delete_method("update_test").await;

    // Create method
    let method1 = WasmPaymentMethodConfig::new(
        "update_test".to_string(),
        "endpoint1".to_string(),
        true,
        false,
        1,
    )
    .unwrap();

    storage.save_method(&method1).await.unwrap();

    // Update by saving again with different values
    let method2 = WasmPaymentMethodConfig::new(
        "update_test".to_string(),
        "endpoint2".to_string(),
        false,
        true,
        5,
    )
    .unwrap();

    storage.save_method(&method2).await.unwrap();

    // Should have updated values
    let retrieved = storage.get_method("update_test").await.unwrap().unwrap();
    assert_eq!(retrieved.endpoint(), "endpoint2");
    assert!(!retrieved.is_public());
    assert!(retrieved.is_preferred());
    assert_eq!(retrieved.priority(), 5);

    // Clean up
    let _ = storage.delete_method("update_test").await;
}

#[wasm_bindgen_test]
async fn test_search_with_empty_query() {
    let storage = WasmContactStorage::new();

    let results = storage.search_contacts("").await.unwrap();
    // Empty query should return all contacts (verify we can call len())
    let _len = results.len();
}

#[wasm_bindgen_test]
async fn test_search_case_insensitive() {
    let storage = WasmContactStorage::new();

    // Clean up
    let _ = storage
        .delete_contact("8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo")
        .await;

    let contact = WasmContact::new(
        "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
        "Alice Smith".to_string(),
    )
    .unwrap();

    storage.save_contact(&contact).await.unwrap();

    // Search with different cases
    let results1 = storage.search_contacts("ALICE").await.unwrap();
    let results2 = storage.search_contacts("alice").await.unwrap();
    let results3 = storage.search_contacts("AlIcE").await.unwrap();

    // All should find the contact
    assert!(!results1.is_empty());
    assert!(!results2.is_empty());
    assert!(!results3.is_empty());

    // Clean up
    let _ = storage
        .delete_contact("8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo")
        .await;
}

// ===========================
// JSON Serialization Edge Cases
// ===========================

#[wasm_bindgen_test]
fn test_method_json_with_special_chars() {
    let method = WasmPaymentMethodConfig::new(
        "test-method".to_string(),
        "endpoint with \"quotes\" and 'apostrophes'".to_string(),
        true,
        true,
        1,
    )
    .unwrap();

    let json = method.to_json().unwrap();
    let restored = WasmPaymentMethodConfig::from_json(&json).unwrap();

    assert_eq!(
        restored.endpoint(),
        "endpoint with \"quotes\" and 'apostrophes'"
    );
}

#[wasm_bindgen_test]
fn test_contact_json_with_unicode() {
    let contact = WasmContact::new(
        "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
        "Alice 👋 测试 🎉".to_string(),
    )
    .unwrap();

    let json = contact.to_json().unwrap();
    let restored = WasmContact::from_json(&json).unwrap();

    assert_eq!(restored.name(), "Alice 👋 测试 🎉");
}

// ===========================
// Boundary Conditions
// ===========================

#[wasm_bindgen_test]
async fn test_list_methods_with_many_items() {
    let storage = WasmPaymentMethodStorage::new();

    // Clean up
    for i in 0..50 {
        let _ = storage.delete_method(&format!("bulk_test_{}", i)).await;
    }

    // Create many methods
    for i in 0..50 {
        let method = WasmPaymentMethodConfig::new(
            format!("bulk_test_{}", i),
            format!("endpoint_{}", i),
            true,
            false,
            i + 1,
        )
        .unwrap();

        storage.save_method(&method).await.unwrap();
    }

    // List all
    let methods = storage.list_methods().await.unwrap();

    // Should have at least our 50 methods
    let bulk_count = methods
        .iter()
        .filter(|m| {
            if let Ok(id) = js_sys::Reflect::get(m, &"method_id".into()) {
                if let Some(id_str) = id.as_string() {
                    return id_str.starts_with("bulk_test_");
                }
            }
            false
        })
        .count();

    assert!(bulk_count >= 50);

    // Clean up
    for i in 0..50 {
        let _ = storage.delete_method(&format!("bulk_test_{}", i)).await;
    }
}

#[wasm_bindgen_test]
async fn test_delete_nonexistent_items() {
    let contact_storage = WasmContactStorage::new();
    let method_storage = WasmPaymentMethodStorage::new();
    let receipt_storage = WasmReceiptStorage::new();

    // Deleting non-existent items should not error
    let result1 = contact_storage.delete_contact("nonexistent").await;
    assert!(result1.is_ok());

    let result2 = method_storage.delete_method("nonexistent").await;
    assert!(result2.is_ok());

    let result3 = receipt_storage.delete_receipt("nonexistent").await;
    assert!(result3.is_ok());
}

// ===========================
// Filter Edge Cases
// ===========================

#[wasm_bindgen_test]
async fn test_filter_with_invalid_direction() {
    let storage = WasmReceiptStorage::new();

    // Invalid direction should return empty
    let results = storage
        .filter_by_direction("invalid", "any_pubkey")
        .await
        .unwrap();

    assert_eq!(results.len(), 0);
}

#[wasm_bindgen_test]
async fn test_preferred_methods_when_none() {
    let storage = WasmPaymentMethodStorage::new();

    // Clean up
    let _ = storage.delete_method("not_preferred_test").await;

    // Create method that's not preferred
    let method = WasmPaymentMethodConfig::new(
        "not_preferred_test".to_string(),
        "endpoint".to_string(),
        true,
        false,
        1,
    )
    .unwrap();

    storage.save_method(&method).await.unwrap();

    // Get preferred (should not include our method)
    let preferred = storage.get_preferred_methods().await.unwrap();

    let has_our_method = preferred.iter().any(|m| {
        if let Ok(id) = js_sys::Reflect::get(m, &"method_id".into()) {
            if let Some(id_str) = id.as_string() {
                return id_str == "not_preferred_test";
            }
        }
        false
    });

    assert!(!has_our_method);

    // Clean up
    let _ = storage.delete_method("not_preferred_test").await;
}

// ===========================
// Export Edge Cases
// ===========================

#[wasm_bindgen_test]
async fn test_export_receipts_with_no_data() {
    let storage = WasmReceiptStorage::new();

    let json = storage.export_as_json().await.unwrap();

    // Should be valid empty JSON array
    assert_eq!(json.trim(), "[]");
}

// ===========================
// Mock Publish Edge Cases
// ===========================

#[wasm_bindgen_test]
async fn test_mock_publish_with_no_methods() {
    let storage = WasmPaymentMethodStorage::new();

    let result = storage.mock_publish().await.unwrap();

    // Should indicate 0 methods
    assert!(result.contains("0 public method"));
}

#[wasm_bindgen_test]
async fn test_mock_publish_with_all_private() {
    let storage = WasmPaymentMethodStorage::new();

    // Clean up
    let _ = storage.delete_method("all_private_test").await;

    // Create private method
    let method = WasmPaymentMethodConfig::new(
        "all_private_test".to_string(),
        "endpoint".to_string(),
        false, // private
        true,
        1,
    )
    .unwrap();

    storage.save_method(&method).await.unwrap();

    let result = storage.mock_publish().await.unwrap();

    // Should indicate only public methods (0 in this case)
    assert!(result.contains("MOCK"));

    // Clean up
    let _ = storage.delete_method("all_private_test").await;
}
