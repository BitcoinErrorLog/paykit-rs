//! Cross-feature integration tests
//!
//! Tests that verify proper integration between different Paykit features,
//! including contacts + receipts, methods + dashboard, etc.

use paykit_demo_web::{
    WasmContact, WasmContactStorage, WasmDashboard, WasmPaymentMethodConfig,
    WasmPaymentMethodStorage, WasmReceiptStorage,
};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Helper to create test receipt JSON
fn create_receipt_json(receipt_id: &str, payer: &str, payee: &str, method: &str) -> String {
    format!(
        r#"{{"receipt_id":"{}","payer":"{}","payee":"{}","amount":"1000","currency":"SAT","method":"{}","timestamp":1700000000}}"#,
        receipt_id, payer, payee, method
    )
}

#[wasm_bindgen_test]
async fn test_contact_and_receipt_integration() {
    let contact_storage = WasmContactStorage::new();
    let receipt_storage = WasmReceiptStorage::new();

    let my_pubkey = "my_integration_test_key";
    let contact_pubkey = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

    // Clean up
    let _ = contact_storage.delete_contact(contact_pubkey).await;
    let _ = receipt_storage.delete_receipt("integ_receipt_1").await;

    // Create contact
    let contact = WasmContact::new(contact_pubkey.to_string(), "Alice".to_string()).unwrap();
    contact_storage.save_contact(&contact).await.unwrap();

    // Create receipt with this contact
    let receipt = create_receipt_json("integ_receipt_1", my_pubkey, contact_pubkey, "lightning");
    receipt_storage
        .save_receipt("integ_receipt_1", &receipt)
        .await
        .unwrap();

    // Filter receipts by contact
    let contact_receipts = receipt_storage
        .filter_by_contact(contact_pubkey, my_pubkey)
        .await
        .unwrap();

    // Should find the receipt
    let found = contact_receipts.iter().any(|r| {
        if let Some(json) = r.as_string() {
            return json.contains("integ_receipt_1");
        }
        false
    });

    assert!(found);

    // Clean up
    let _ = contact_storage.delete_contact(contact_pubkey).await;
    let _ = receipt_storage.delete_receipt("integ_receipt_1").await;
}

#[wasm_bindgen_test]
async fn test_method_and_receipt_integration() {
    let method_storage = WasmPaymentMethodStorage::new();
    let receipt_storage = WasmReceiptStorage::new();

    // Clean up
    let _ = method_storage.delete_method("integ_lightning").await;
    let _ = receipt_storage.delete_receipt("integ_receipt_2").await;

    // Create Lightning method
    let method = WasmPaymentMethodConfig::new(
        "integ_lightning".to_string(),
        "lnurl123".to_string(),
        true,
        true,
        1,
    )
    .unwrap();

    method_storage.save_method(&method).await.unwrap();

    // Create receipt using this method
    let receipt = create_receipt_json("integ_receipt_2", "alice", "bob", "integ_lightning");
    receipt_storage
        .save_receipt("integ_receipt_2", &receipt)
        .await
        .unwrap();

    // Filter receipts by method
    let method_receipts = receipt_storage
        .filter_by_method("integ_lightning")
        .await
        .unwrap();

    // Should find the receipt
    let found = method_receipts.iter().any(|r| {
        if let Some(json) = r.as_string() {
            return json.contains("integ_receipt_2");
        }
        false
    });

    assert!(found);

    // Clean up
    let _ = method_storage.delete_method("integ_lightning").await;
    let _ = receipt_storage.delete_receipt("integ_receipt_2").await;
}

#[wasm_bindgen_test]
async fn test_dashboard_aggregates_all_features() {
    let contact_storage = WasmContactStorage::new();
    let method_storage = WasmPaymentMethodStorage::new();
    let receipt_storage = WasmReceiptStorage::new();
    let dashboard = WasmDashboard::new();

    let my_pubkey = "dash_test_pubkey";
    let contact_pubkey = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

    // Clean up
    let _ = contact_storage.delete_contact(contact_pubkey).await;
    let _ = method_storage.delete_method("dash_test_method").await;
    let _ = receipt_storage.delete_receipt("dash_test_receipt").await;

    // Get initial stats
    let _before_stats = dashboard.get_overview_stats(my_pubkey).await.unwrap();
    let before_contacts = js_sys::Reflect::get(&_before_stats, &"contacts".into())
        .unwrap()
        .as_f64()
        .unwrap() as usize;

    // Add a contact
    let contact = WasmContact::new(contact_pubkey.to_string(), "Alice".to_string()).unwrap();
    contact_storage.save_contact(&contact).await.unwrap();

    // Add a payment method
    let method = WasmPaymentMethodConfig::new(
        "dash_test_method".to_string(),
        "ep".to_string(),
        true,
        true,
        1,
    )
    .unwrap();
    method_storage.save_method(&method).await.unwrap();

    // Add a receipt
    let receipt = create_receipt_json("dash_test_receipt", my_pubkey, contact_pubkey, "lightning");
    receipt_storage
        .save_receipt("dash_test_receipt", &receipt)
        .await
        .unwrap();

    // Get updated stats
    let after_stats = dashboard.get_overview_stats(my_pubkey).await.unwrap();
    let after_contacts = js_sys::Reflect::get(&after_stats, &"contacts".into())
        .unwrap()
        .as_f64()
        .unwrap() as usize;
    let after_methods = js_sys::Reflect::get(&after_stats, &"payment_methods".into())
        .unwrap()
        .as_f64()
        .unwrap() as usize;
    let after_receipts = js_sys::Reflect::get(&after_stats, &"total_receipts".into())
        .unwrap()
        .as_f64()
        .unwrap() as usize;

    // Stats should have increased
    assert!(after_contacts > before_contacts);
    assert!(after_methods >= 1);
    assert!(after_receipts >= 1);

    // Clean up
    let _ = contact_storage.delete_contact(contact_pubkey).await;
    let _ = method_storage.delete_method("dash_test_method").await;
    let _ = receipt_storage.delete_receipt("dash_test_receipt").await;
}

#[wasm_bindgen_test]
async fn test_setup_checklist_reflects_state() {
    let contact_storage = WasmContactStorage::new();
    let method_storage = WasmPaymentMethodStorage::new();
    let dashboard = WasmDashboard::new();

    let contact_pubkey = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

    // Clean up
    let _ = contact_storage.delete_contact(contact_pubkey).await;
    let _ = method_storage.delete_method("checklist_test").await;

    // Initial checklist (likely empty) - we'll check after adding items
    let _before = dashboard.get_setup_checklist().await.unwrap();

    // Add contact
    let contact = WasmContact::new(contact_pubkey.to_string(), "Test".to_string()).unwrap();
    contact_storage.save_contact(&contact).await.unwrap();

    // Add preferred method
    let method = WasmPaymentMethodConfig::new(
        "checklist_test".to_string(),
        "endpoint".to_string(),
        true,
        true, // preferred
        1,
    )
    .unwrap();
    method_storage.save_method(&method).await.unwrap();

    // Updated checklist
    let after = dashboard.get_setup_checklist().await.unwrap();

    let has_contacts = js_sys::Reflect::get(&after, &"has_contacts".into())
        .unwrap()
        .as_bool()
        .unwrap();
    let has_methods = js_sys::Reflect::get(&after, &"has_payment_methods".into())
        .unwrap()
        .as_bool()
        .unwrap();
    let has_preferred = js_sys::Reflect::get(&after, &"has_preferred_method".into())
        .unwrap()
        .as_bool()
        .unwrap();

    assert!(has_contacts);
    assert!(has_methods);
    assert!(has_preferred);

    // Clean up
    let _ = contact_storage.delete_contact(contact_pubkey).await;
    let _ = method_storage.delete_method("checklist_test").await;
}

#[wasm_bindgen_test]
async fn test_full_user_workflow() {
    let contact_storage = WasmContactStorage::new();
    let method_storage = WasmPaymentMethodStorage::new();
    let receipt_storage = WasmReceiptStorage::new();
    let dashboard = WasmDashboard::new();

    let my_pubkey = "workflow_me";
    let contact_pubkey = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

    // Clean up
    let _ = contact_storage.delete_contact(contact_pubkey).await;
    let _ = method_storage.delete_method("workflow_lightning").await;
    let _ = receipt_storage.delete_receipt("workflow_receipt").await;

    // 1. Add contact
    let contact = WasmContact::new(contact_pubkey.to_string(), "Bob".to_string()).unwrap();
    contact_storage.save_contact(&contact).await.unwrap();

    // 2. Configure payment method
    let method = WasmPaymentMethodConfig::new(
        "workflow_lightning".to_string(),
        "bob@lightning.com".to_string(),
        true,
        true,
        1,
    )
    .unwrap();
    method_storage.save_method(&method).await.unwrap();

    // 3. Create receipt
    let receipt = create_receipt_json(
        "workflow_receipt",
        my_pubkey,
        contact_pubkey,
        "workflow_lightning",
    );
    receipt_storage
        .save_receipt("workflow_receipt", &receipt)
        .await
        .unwrap();

    // 4. Verify dashboard shows everything
    let stats = dashboard.get_overview_stats(my_pubkey).await.unwrap();

    let contacts = js_sys::Reflect::get(&stats, &"contacts".into())
        .unwrap()
        .as_f64()
        .unwrap() as usize;
    let methods = js_sys::Reflect::get(&stats, &"payment_methods".into())
        .unwrap()
        .as_f64()
        .unwrap() as usize;
    let receipts = js_sys::Reflect::get(&stats, &"total_receipts".into())
        .unwrap()
        .as_f64()
        .unwrap() as usize;

    assert!(contacts >= 1);
    assert!(methods >= 1);
    assert!(receipts >= 1);

    // 5. Verify setup complete
    let is_complete = dashboard.is_setup_complete().await.unwrap();
    assert!(is_complete);

    // 6. Verify can filter receipts by contact
    let contact_receipts = receipt_storage
        .filter_by_contact(contact_pubkey, my_pubkey)
        .await
        .unwrap();

    assert!(!contact_receipts.is_empty());

    // Clean up
    let _ = contact_storage.delete_contact(contact_pubkey).await;
    let _ = method_storage.delete_method("workflow_lightning").await;
    let _ = receipt_storage.delete_receipt("workflow_receipt").await;
}
