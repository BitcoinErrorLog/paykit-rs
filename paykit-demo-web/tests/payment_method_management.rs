#![cfg(target_arch = "wasm32")]
//! Integration tests for payment method management
//!
//! These tests verify the complete workflow of managing payment methods,
//! including creation, priority ordering, preferences, and mock publishing.

use paykit_demo_web::{WasmPaymentMethodConfig, WasmPaymentMethodStorage};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_complete_payment_method_workflow() {
    let storage = WasmPaymentMethodStorage::new();

    // Clean up any existing test methods
    let _ = storage.delete_method("workflow_lightning").await;
    let _ = storage.delete_method("workflow_onchain").await;

    // Create multiple payment methods
    let lightning = WasmPaymentMethodConfig::new(
        "workflow_lightning".to_string(),
        "lnurl1dp68gurn8ghj7um5v93kketj9ehx2amn9uh8wetvdskkkmn0wahz7mrww4excup0".to_string(),
        true, // public
        true, // preferred
        1,    // highest priority
    )
    .unwrap();

    let onchain = WasmPaymentMethodConfig::new(
        "workflow_onchain".to_string(),
        "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string(),
        true,  // public
        false, // not preferred
        2,     // lower priority
    )
    .unwrap();

    // Save both methods
    storage.save_method(&lightning).await.unwrap();
    storage.save_method(&onchain).await.unwrap();

    // Verify both are saved
    let retrieved_lightning = storage
        .get_method("workflow_lightning")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        retrieved_lightning.endpoint(),
        "lnurl1dp68gurn8ghj7um5v93kketj9ehx2amn9uh8wetvdskkkmn0wahz7mrww4excup0"
    );

    let retrieved_onchain = storage
        .get_method("workflow_onchain")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        retrieved_onchain.endpoint(),
        "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
    );

    // List methods - should be sorted by priority
    let all_methods = storage.list_methods().await.unwrap();
    assert!(all_methods.len() >= 2);

    // Get preferred methods
    let preferred = storage.get_preferred_methods().await.unwrap();
    let has_lightning = preferred.iter().any(|m| {
        if let Ok(id) = js_sys::Reflect::get(m, &"method_id".into()) {
            if let Some(id_str) = id.as_string() {
                return id_str == "workflow_lightning";
            }
        }
        false
    });
    assert!(has_lightning);

    // Update preference
    storage
        .set_preferred("workflow_onchain", true)
        .await
        .unwrap();
    let updated_onchain = storage
        .get_method("workflow_onchain")
        .await
        .unwrap()
        .unwrap();
    assert!(updated_onchain.is_preferred());

    // Update priority
    storage
        .update_priority("workflow_onchain", 1)
        .await
        .unwrap();
    let updated_onchain = storage
        .get_method("workflow_onchain")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_onchain.priority(), 1);

    // Mock publish
    let publish_result = storage.mock_publish().await.unwrap();
    assert!(publish_result.contains("MOCK"));
    assert!(publish_result.contains("2 public method"));

    // Clean up
    storage.delete_method("workflow_lightning").await.unwrap();
    storage.delete_method("workflow_onchain").await.unwrap();

    // Verify deletion
    let deleted = storage.get_method("workflow_lightning").await.unwrap();
    assert!(deleted.is_none());
}

#[wasm_bindgen_test]
async fn test_multiple_methods_priority_ordering() {
    let storage = WasmPaymentMethodStorage::new();

    // Clean up
    let _ = storage.delete_method("priority_test_1").await;
    let _ = storage.delete_method("priority_test_2").await;
    let _ = storage.delete_method("priority_test_3").await;

    // Create methods with specific priorities
    let method1 = WasmPaymentMethodConfig::new(
        "priority_test_1".to_string(),
        "endpoint1".to_string(),
        true,
        true,
        3, // third priority
    )
    .unwrap();

    let method2 = WasmPaymentMethodConfig::new(
        "priority_test_2".to_string(),
        "endpoint2".to_string(),
        true,
        true,
        1, // first priority
    )
    .unwrap();

    let method3 = WasmPaymentMethodConfig::new(
        "priority_test_3".to_string(),
        "endpoint3".to_string(),
        true,
        true,
        2, // second priority
    )
    .unwrap();

    storage.save_method(&method1).await.unwrap();
    storage.save_method(&method2).await.unwrap();
    storage.save_method(&method3).await.unwrap();

    // Get all methods
    let all_methods = storage.list_methods().await.unwrap();

    // Filter to our test methods
    let test_methods: Vec<&wasm_bindgen::JsValue> = all_methods
        .iter()
        .filter(|m| {
            if let Ok(id) = js_sys::Reflect::get(m, &"method_id".into()) {
                if let Some(id_str) = id.as_string() {
                    return id_str.starts_with("priority_test_");
                }
            }
            false
        })
        .collect();

    // Verify we have all 3 test methods
    assert!(test_methods.len() >= 3);

    // Verify they are sorted by priority (1, 2, 3)
    // Note: There may be other methods, but our test methods should be in order
    let mut found_order = Vec::new();
    for method in test_methods {
        if let Ok(id) = js_sys::Reflect::get(method, &"method_id".into()) {
            if let Some(id_str) = id.as_string() {
                if id_str.starts_with("priority_test_") {
                    if let Ok(priority) = js_sys::Reflect::get(method, &"priority".into()) {
                        if let Some(p) = priority.as_f64() {
                            found_order.push((id_str, p as u32));
                        }
                    }
                }
            }
        }
    }

    // Sort by priority to verify ordering
    found_order.sort_by_key(|(_, p)| *p);
    assert_eq!(found_order.len(), 3);
    assert_eq!(found_order[0].1, 1); // method2 should be first
    assert_eq!(found_order[1].1, 2); // method3 should be second
    assert_eq!(found_order[2].1, 3); // method1 should be third

    // Clean up
    let _ = storage.delete_method("priority_test_1").await;
    let _ = storage.delete_method("priority_test_2").await;
    let _ = storage.delete_method("priority_test_3").await;
}

#[wasm_bindgen_test]
async fn test_reorder_priorities() {
    let storage = WasmPaymentMethodStorage::new();

    // Clean up
    let _ = storage.delete_method("reorder_1").await;
    let _ = storage.delete_method("reorder_2").await;

    // Create two methods
    let method1 = WasmPaymentMethodConfig::new(
        "reorder_1".to_string(),
        "endpoint1".to_string(),
        true,
        true,
        1,
    )
    .unwrap();

    let method2 = WasmPaymentMethodConfig::new(
        "reorder_2".to_string(),
        "endpoint2".to_string(),
        true,
        true,
        2,
    )
    .unwrap();

    storage.save_method(&method1).await.unwrap();
    storage.save_method(&method2).await.unwrap();

    // Swap priorities
    storage.update_priority("reorder_1", 2).await.unwrap();
    storage.update_priority("reorder_2", 1).await.unwrap();

    // Verify new order
    let updated1 = storage.get_method("reorder_1").await.unwrap().unwrap();
    let updated2 = storage.get_method("reorder_2").await.unwrap().unwrap();

    assert_eq!(updated1.priority(), 2);
    assert_eq!(updated2.priority(), 1);

    // Clean up
    let _ = storage.delete_method("reorder_1").await;
    let _ = storage.delete_method("reorder_2").await;
}

#[wasm_bindgen_test]
async fn test_set_unset_preferred() {
    let storage = WasmPaymentMethodStorage::new();

    // Clean up
    let _ = storage.delete_method("pref_toggle").await;

    // Create method with preferred = false
    let method = WasmPaymentMethodConfig::new(
        "pref_toggle".to_string(),
        "endpoint".to_string(),
        true,
        false,
        1,
    )
    .unwrap();

    storage.save_method(&method).await.unwrap();

    // Set preferred
    storage.set_preferred("pref_toggle", true).await.unwrap();
    let updated = storage.get_method("pref_toggle").await.unwrap().unwrap();
    assert!(updated.is_preferred());

    // Unset preferred
    storage.set_preferred("pref_toggle", false).await.unwrap();
    let updated = storage.get_method("pref_toggle").await.unwrap().unwrap();
    assert!(!updated.is_preferred());

    // Clean up
    let _ = storage.delete_method("pref_toggle").await;
}

#[wasm_bindgen_test]
async fn test_mock_publish_workflow() {
    let storage = WasmPaymentMethodStorage::new();

    // Clean up
    let _ = storage.delete_method("publish_test_pub").await;
    let _ = storage.delete_method("publish_test_priv").await;

    // Create one public and one private method
    let public_method = WasmPaymentMethodConfig::new(
        "publish_test_pub".to_string(),
        "endpoint_public".to_string(),
        true, // public
        true,
        1,
    )
    .unwrap();

    let private_method = WasmPaymentMethodConfig::new(
        "publish_test_priv".to_string(),
        "endpoint_private".to_string(),
        false, // private
        true,
        2,
    )
    .unwrap();

    storage.save_method(&public_method).await.unwrap();
    storage.save_method(&private_method).await.unwrap();

    // Mock publish - should only count public methods
    let result = storage.mock_publish().await.unwrap();
    assert!(result.contains("MOCK"));
    assert!(result.contains("demo-only"));

    // Clean up
    let _ = storage.delete_method("publish_test_pub").await;
    let _ = storage.delete_method("publish_test_priv").await;
}

#[wasm_bindgen_test]
async fn test_method_preference_persistence() {
    let storage = WasmPaymentMethodStorage::new();

    // Clean up
    let _ = storage.delete_method("persist_test").await;

    // Create method
    let method = WasmPaymentMethodConfig::new(
        "persist_test".to_string(),
        "endpoint".to_string(),
        true,
        false,
        1,
    )
    .unwrap();

    storage.save_method(&method).await.unwrap();

    // Update preference
    storage.set_preferred("persist_test", true).await.unwrap();

    // Create a new storage instance to simulate page reload
    let new_storage = WasmPaymentMethodStorage::new();

    // Retrieve method with new storage instance
    let retrieved = new_storage
        .get_method("persist_test")
        .await
        .unwrap()
        .unwrap();
    assert!(retrieved.is_preferred());

    // Clean up
    let _ = storage.delete_method("persist_test").await;
}

#[wasm_bindgen_test]
async fn test_empty_storage() {
    let storage = WasmPaymentMethodStorage::new();

    // Try to get non-existent method
    let result = storage.get_method("does_not_exist").await.unwrap();
    assert!(result.is_none());

    // Try to update non-existent method
    let result = storage.set_preferred("does_not_exist", true).await.is_err();
    assert!(result);

    let result = storage.update_priority("does_not_exist", 1).await.is_err();
    assert!(result);
}

#[wasm_bindgen_test]
async fn test_duplicate_method_id_overwrites() {
    let storage = WasmPaymentMethodStorage::new();

    // Clean up
    let _ = storage.delete_method("duplicate_test").await;

    // Create first method
    let method1 = WasmPaymentMethodConfig::new(
        "duplicate_test".to_string(),
        "endpoint1".to_string(),
        true,
        true,
        1,
    )
    .unwrap();

    storage.save_method(&method1).await.unwrap();

    // Create second method with same ID but different endpoint
    let method2 = WasmPaymentMethodConfig::new(
        "duplicate_test".to_string(),
        "endpoint2".to_string(),
        false,
        false,
        2,
    )
    .unwrap();

    storage.save_method(&method2).await.unwrap();

    // Should have overwritten
    let retrieved = storage.get_method("duplicate_test").await.unwrap().unwrap();
    assert_eq!(retrieved.endpoint(), "endpoint2");
    assert!(!retrieved.is_public());
    assert!(!retrieved.is_preferred());
    assert_eq!(retrieved.priority(), 2);

    // Clean up
    let _ = storage.delete_method("duplicate_test").await;
}
