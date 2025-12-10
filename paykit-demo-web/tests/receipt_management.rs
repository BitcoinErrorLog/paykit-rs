//! Integration tests for receipt management
//!
//! These tests verify the complete workflow of managing payment receipts,
//! including storage, filtering, statistics, and export functionality.

use paykit_demo_web::WasmReceiptStorage;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Helper function to create a test receipt JSON
fn create_test_receipt(
    receipt_id: &str,
    payer: &str,
    payee: &str,
    amount: &str,
    currency: &str,
    method: &str,
    timestamp: i64,
) -> String {
    format!(
        r#"{{"receipt_id":"{}","payer":"{}","payee":"{}","amount":"{}","currency":"{}","method":"{}","timestamp":{}}}"#,
        receipt_id, payer, payee, amount, currency, method, timestamp
    )
}

#[wasm_bindgen_test]
async fn test_receipt_storage_workflow() {
    let storage = WasmReceiptStorage::new();

    // Clean up
    let _ = storage.delete_receipt("test_receipt_1").await;
    let _ = storage.delete_receipt("test_receipt_2").await;

    // Create test receipts
    let receipt1 = create_test_receipt(
        "test_receipt_1",
        "alice_pubkey",
        "bob_pubkey",
        "1000",
        "SAT",
        "lightning",
        1700000001,
    );

    let receipt2 = create_test_receipt(
        "test_receipt_2",
        "bob_pubkey",
        "alice_pubkey",
        "2000",
        "SAT",
        "onchain",
        1700000002,
    );

    // Save receipts
    storage
        .save_receipt("test_receipt_1", &receipt1)
        .await
        .unwrap();
    storage
        .save_receipt("test_receipt_2", &receipt2)
        .await
        .unwrap();

    // Retrieve individual receipt
    let retrieved = storage.get_receipt("test_receipt_1").await.unwrap();
    assert!(retrieved.is_some());
    assert!(retrieved.unwrap().contains("test_receipt_1"));

    // List all receipts
    let all_receipts = storage.list_receipts().await.unwrap();
    assert!(all_receipts.len() >= 2);

    // Delete receipt
    storage.delete_receipt("test_receipt_1").await.unwrap();
    let deleted = storage.get_receipt("test_receipt_1").await.unwrap();
    assert!(deleted.is_none());

    // Clean up
    let _ = storage.delete_receipt("test_receipt_2").await;
}

#[wasm_bindgen_test]
async fn test_filter_by_direction() {
    let storage = WasmReceiptStorage::new();

    // Clean up
    let _ = storage.delete_receipt("dir_test_1").await;
    let _ = storage.delete_receipt("dir_test_2").await;

    let my_pubkey = "my_test_pubkey";

    // Create sent receipt (I am payer)
    let sent_receipt = create_test_receipt(
        "dir_test_1",
        my_pubkey,
        "other_pubkey",
        "1000",
        "SAT",
        "lightning",
        1700000001,
    );

    // Create received receipt (I am payee)
    let received_receipt = create_test_receipt(
        "dir_test_2",
        "other_pubkey",
        my_pubkey,
        "2000",
        "SAT",
        "lightning",
        1700000002,
    );

    storage
        .save_receipt("dir_test_1", &sent_receipt)
        .await
        .unwrap();
    storage
        .save_receipt("dir_test_2", &received_receipt)
        .await
        .unwrap();

    // Filter sent
    let sent = storage
        .filter_by_direction("sent", my_pubkey)
        .await
        .unwrap();
    let has_sent = sent.iter().any(|r| {
        if let Some(json_str) = r.as_string() {
            return json_str.contains("dir_test_1");
        }
        false
    });
    assert!(has_sent);

    // Filter received
    let received = storage
        .filter_by_direction("received", my_pubkey)
        .await
        .unwrap();
    let has_received = received.iter().any(|r| {
        if let Some(json_str) = r.as_string() {
            return json_str.contains("dir_test_2");
        }
        false
    });
    assert!(has_received);

    // Clean up
    let _ = storage.delete_receipt("dir_test_1").await;
    let _ = storage.delete_receipt("dir_test_2").await;
}

#[wasm_bindgen_test]
async fn test_filter_by_method() {
    let storage = WasmReceiptStorage::new();

    // Clean up
    let _ = storage.delete_receipt("method_test_1").await;
    let _ = storage.delete_receipt("method_test_2").await;

    // Create lightning receipt
    let lightning_receipt = create_test_receipt(
        "method_test_1",
        "alice",
        "bob",
        "1000",
        "SAT",
        "lightning",
        1700000001,
    );

    // Create onchain receipt
    let onchain_receipt = create_test_receipt(
        "method_test_2",
        "alice",
        "bob",
        "2000",
        "SAT",
        "onchain",
        1700000002,
    );

    storage
        .save_receipt("method_test_1", &lightning_receipt)
        .await
        .unwrap();
    storage
        .save_receipt("method_test_2", &onchain_receipt)
        .await
        .unwrap();

    // Filter by lightning
    let lightning_results = storage.filter_by_method("lightning").await.unwrap();
    let has_lightning = lightning_results.iter().any(|r| {
        if let Some(json_str) = r.as_string() {
            return json_str.contains("method_test_1");
        }
        false
    });
    assert!(has_lightning);

    // Filter by onchain
    let onchain_results = storage.filter_by_method("onchain").await.unwrap();
    let has_onchain = onchain_results.iter().any(|r| {
        if let Some(json_str) = r.as_string() {
            return json_str.contains("method_test_2");
        }
        false
    });
    assert!(has_onchain);

    // Clean up
    let _ = storage.delete_receipt("method_test_1").await;
    let _ = storage.delete_receipt("method_test_2").await;
}

#[wasm_bindgen_test]
async fn test_filter_by_contact() {
    let storage = WasmReceiptStorage::new();

    // Clean up
    let _ = storage.delete_receipt("contact_test_1").await;
    let _ = storage.delete_receipt("contact_test_2").await;

    let my_pubkey = "my_pubkey";
    let contact_pubkey = "contact_pubkey";
    let other_pubkey = "other_pubkey";

    // Create receipt with contact (I sent)
    let receipt1 = create_test_receipt(
        "contact_test_1",
        my_pubkey,
        contact_pubkey,
        "1000",
        "SAT",
        "lightning",
        1700000001,
    );

    // Create receipt with contact (I received)
    let receipt2 = create_test_receipt(
        "contact_test_2",
        contact_pubkey,
        my_pubkey,
        "2000",
        "SAT",
        "lightning",
        1700000002,
    );

    // Create receipt with other contact
    let receipt3 = create_test_receipt(
        "contact_test_3",
        my_pubkey,
        other_pubkey,
        "3000",
        "SAT",
        "lightning",
        1700000003,
    );

    storage
        .save_receipt("contact_test_1", &receipt1)
        .await
        .unwrap();
    storage
        .save_receipt("contact_test_2", &receipt2)
        .await
        .unwrap();
    storage
        .save_receipt("contact_test_3", &receipt3)
        .await
        .unwrap();

    // Filter by contact
    let contact_receipts = storage
        .filter_by_contact(contact_pubkey, my_pubkey)
        .await
        .unwrap();

    // Should have both receipts with contact
    assert!(contact_receipts.len() >= 2);

    let has_receipt1 = contact_receipts.iter().any(|r| {
        if let Some(json_str) = r.as_string() {
            return json_str.contains("contact_test_1");
        }
        false
    });
    assert!(has_receipt1);

    let has_receipt2 = contact_receipts.iter().any(|r| {
        if let Some(json_str) = r.as_string() {
            return json_str.contains("contact_test_2");
        }
        false
    });
    assert!(has_receipt2);

    // Clean up
    let _ = storage.delete_receipt("contact_test_1").await;
    let _ = storage.delete_receipt("contact_test_2").await;
    let _ = storage.delete_receipt("contact_test_3").await;
}

#[wasm_bindgen_test]
async fn test_statistics() {
    let storage = WasmReceiptStorage::new();

    // Clean up
    let _ = storage.delete_receipt("stats_test_1").await;
    let _ = storage.delete_receipt("stats_test_2").await;
    let _ = storage.delete_receipt("stats_test_3").await;

    let my_pubkey = "stats_my_pubkey";

    // Create 2 sent receipts
    let sent1 = create_test_receipt(
        "stats_test_1",
        my_pubkey,
        "other1",
        "1000",
        "SAT",
        "lightning",
        1700000001,
    );
    let sent2 = create_test_receipt(
        "stats_test_2",
        my_pubkey,
        "other2",
        "2000",
        "SAT",
        "lightning",
        1700000002,
    );

    // Create 1 received receipt
    let received1 = create_test_receipt(
        "stats_test_3",
        "other3",
        my_pubkey,
        "3000",
        "SAT",
        "lightning",
        1700000003,
    );

    storage.save_receipt("stats_test_1", &sent1).await.unwrap();
    storage.save_receipt("stats_test_2", &sent2).await.unwrap();
    storage
        .save_receipt("stats_test_3", &received1)
        .await
        .unwrap();

    // Get statistics
    let stats = storage.get_statistics(my_pubkey).await.unwrap();

    let total = js_sys::Reflect::get(&stats, &"total".into())
        .unwrap()
        .as_f64()
        .unwrap() as usize;
    let sent = js_sys::Reflect::get(&stats, &"sent".into())
        .unwrap()
        .as_f64()
        .unwrap() as usize;
    let received = js_sys::Reflect::get(&stats, &"received".into())
        .unwrap()
        .as_f64()
        .unwrap() as usize;

    assert!(total >= 3);
    assert!(sent >= 2);
    assert!(received >= 1);

    // Clean up
    let _ = storage.delete_receipt("stats_test_1").await;
    let _ = storage.delete_receipt("stats_test_2").await;
    let _ = storage.delete_receipt("stats_test_3").await;
}

#[wasm_bindgen_test]
async fn test_export_json() {
    let storage = WasmReceiptStorage::new();

    // Clean up
    let _ = storage.delete_receipt("export_test_1").await;

    // Create test receipt
    let receipt = create_test_receipt(
        "export_test_1",
        "alice",
        "bob",
        "1000",
        "SAT",
        "lightning",
        1700000001,
    );

    storage
        .save_receipt("export_test_1", &receipt)
        .await
        .unwrap();

    // Export as JSON
    let json = storage.export_as_json().await.unwrap();

    // Verify it's valid JSON and contains our receipt
    assert!(json.contains("export_test_1"));
    assert!(json.starts_with('['));
    assert!(json.ends_with(']'));

    // Clean up
    let _ = storage.delete_receipt("export_test_1").await;
}

#[wasm_bindgen_test]
async fn test_clear_all() {
    let storage = WasmReceiptStorage::new();

    // Create test receipts
    let receipt1 = create_test_receipt(
        "clear_test_1",
        "alice",
        "bob",
        "1000",
        "SAT",
        "lightning",
        1700000001,
    );
    let receipt2 = create_test_receipt(
        "clear_test_2",
        "bob",
        "alice",
        "2000",
        "SAT",
        "onchain",
        1700000002,
    );

    storage
        .save_receipt("clear_test_1", &receipt1)
        .await
        .unwrap();
    storage
        .save_receipt("clear_test_2", &receipt2)
        .await
        .unwrap();

    // Verify receipts exist
    let before = storage.list_receipts().await.unwrap();
    assert!(before.len() >= 2);

    // Clear all
    storage.clear_all().await.unwrap();

    // Verify our test receipts are gone
    let after_1 = storage.get_receipt("clear_test_1").await.unwrap();
    let after_2 = storage.get_receipt("clear_test_2").await.unwrap();
    assert!(after_1.is_none());
    assert!(after_2.is_none());
}

#[wasm_bindgen_test]
async fn test_receipt_persistence() {
    let storage1 = WasmReceiptStorage::new();

    // Clean up
    let _ = storage1.delete_receipt("persist_test").await;

    // Save receipt with first storage instance
    let receipt = create_test_receipt(
        "persist_test",
        "alice",
        "bob",
        "1000",
        "SAT",
        "lightning",
        1700000001,
    );

    storage1
        .save_receipt("persist_test", &receipt)
        .await
        .unwrap();

    // Create new storage instance (simulates page reload)
    let storage2 = WasmReceiptStorage::new();

    // Retrieve with new instance
    let retrieved = storage2.get_receipt("persist_test").await.unwrap();
    assert!(retrieved.is_some());
    assert!(retrieved.unwrap().contains("persist_test"));

    // Clean up
    let _ = storage2.delete_receipt("persist_test").await;
}
