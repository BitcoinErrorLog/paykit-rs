//! Integration tests for dashboard functionality
//!
//! These tests verify the dashboard statistics aggregation and overview features.

use paykit_demo_web::WasmDashboard;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_dashboard_creation() {
    let _dashboard = WasmDashboard::new();
    // Verify creation succeeds (just check it doesn't panic)
}

#[wasm_bindgen_test]
async fn test_get_overview_stats() {
    let dashboard = WasmDashboard::new();

    let stats = dashboard.get_overview_stats("test_pubkey").await.unwrap();

    // Verify all expected fields exist
    let contacts = js_sys::Reflect::get(&stats, &"contacts".into()).unwrap();
    assert!(contacts.as_f64().is_some());

    let methods = js_sys::Reflect::get(&stats, &"payment_methods".into()).unwrap();
    assert!(methods.as_f64().is_some());

    let receipts = js_sys::Reflect::get(&stats, &"total_receipts".into()).unwrap();
    assert!(receipts.as_f64().is_some());

    let sent = js_sys::Reflect::get(&stats, &"sent_receipts".into()).unwrap();
    assert!(sent.as_f64().is_some());

    let received = js_sys::Reflect::get(&stats, &"received_receipts".into()).unwrap();
    assert!(received.as_f64().is_some());

    let subs = js_sys::Reflect::get(&stats, &"total_subscriptions".into()).unwrap();
    assert!(subs.as_f64().is_some());
}

#[wasm_bindgen_test]
async fn test_get_setup_checklist() {
    let dashboard = WasmDashboard::new();

    let checklist = dashboard.get_setup_checklist().await.unwrap();

    // Verify all checklist items exist
    let has_contacts = js_sys::Reflect::get(&checklist, &"has_contacts".into()).unwrap();
    assert!(has_contacts.as_bool().is_some());

    let has_methods = js_sys::Reflect::get(&checklist, &"has_payment_methods".into()).unwrap();
    assert!(has_methods.as_bool().is_some());

    let has_preferred = js_sys::Reflect::get(&checklist, &"has_preferred_method".into()).unwrap();
    assert!(has_preferred.as_bool().is_some());
}

#[wasm_bindgen_test]
async fn test_is_setup_complete() {
    let dashboard = WasmDashboard::new();

    let is_complete = dashboard.is_setup_complete().await.unwrap();

    // Should return a boolean (verify it's actually a bool type)
    let _is_bool: bool = is_complete;
}

#[wasm_bindgen_test]
async fn test_get_recent_activity() {
    let dashboard = WasmDashboard::new();

    let activity = dashboard
        .get_recent_activity("test_pubkey", 10)
        .await
        .unwrap();

    // Should return an array (verify we can call len())
    let _len = activity.len();
}

#[wasm_bindgen_test]
async fn test_activity_limit() {
    let dashboard = WasmDashboard::new();

    // Request only 5 items
    let activity = dashboard
        .get_recent_activity("test_pubkey", 5)
        .await
        .unwrap();

    // Should respect the limit
    assert!(activity.len() <= 5);
}

#[wasm_bindgen_test]
async fn test_dashboard_with_no_data() {
    let dashboard = WasmDashboard::new();

    // Should handle empty state gracefully
    let stats = dashboard.get_overview_stats("new_user").await.unwrap();

    let contacts = js_sys::Reflect::get(&stats, &"contacts".into())
        .unwrap()
        .as_f64()
        .unwrap();

    // Should return valid number for new user
    assert!(contacts.is_finite());
}
