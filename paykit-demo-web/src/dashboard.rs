//! Dashboard module for aggregating statistics across all Paykit features
//!
//! This module provides a unified view of the user's Paykit activity,
//! including contacts, payment methods, receipts, and subscriptions.

use wasm_bindgen::prelude::*;

use crate::contacts::WasmContactStorage;
use crate::payment::WasmReceiptStorage;
use crate::payment_methods::WasmPaymentMethodStorage;
use crate::subscriptions::WasmSubscriptionAgreementStorage;

/// Dashboard statistics aggregator
///
/// Collects statistics from all Paykit features and provides
/// a unified overview for the dashboard UI.
///
/// # Examples
///
/// ```
/// use paykit_demo_web::WasmDashboard;
///
/// let dashboard = WasmDashboard::new();
/// let stats = dashboard.get_overview_stats("my_pubkey").await?;
/// ```
#[wasm_bindgen]
pub struct WasmDashboard {
    contact_storage: WasmContactStorage,
    method_storage: WasmPaymentMethodStorage,
    receipt_storage: WasmReceiptStorage,
    subscription_storage: WasmSubscriptionAgreementStorage,
}

impl Default for WasmDashboard {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmDashboard {
    /// Create a new dashboard aggregator
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmDashboard;
    ///
    /// let dashboard = WasmDashboard::new();
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            contact_storage: WasmContactStorage::new(),
            method_storage: WasmPaymentMethodStorage::new(),
            receipt_storage: WasmReceiptStorage::new(),
            subscription_storage: WasmSubscriptionAgreementStorage::new(),
        }
    }

    /// Get comprehensive overview statistics
    ///
    /// Returns an object with statistics from all features:
    /// - contacts: Number of saved contacts
    /// - payment_methods: Number of configured methods
    /// - preferred_methods: Number of preferred methods
    /// - total_receipts: Total receipts
    /// - sent_receipts: Sent payments
    /// - received_receipts: Received payments
    /// - total_subscriptions: Total subscriptions
    /// - active_subscriptions: Currently active subscriptions
    ///
    /// # Arguments
    ///
    /// * `current_pubkey` - Current user's public key for receipt direction
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmDashboard;
    ///
    /// let dashboard = WasmDashboard::new();
    /// let stats = dashboard.get_overview_stats("my_pubkey").await?;
    /// ```
    pub async fn get_overview_stats(&self, current_pubkey: &str) -> Result<JsValue, JsValue> {
        // Get contact count
        let contacts = self.contact_storage.list_contacts().await?;
        let contact_count = contacts.len();

        // Get payment method counts
        let methods = self.method_storage.list_methods().await?;
        let method_count = methods.len();
        let preferred_methods = self.method_storage.get_preferred_methods().await?;
        let preferred_count = preferred_methods.len();

        // Get receipt statistics
        let receipt_stats = self.receipt_storage.get_statistics(current_pubkey).await?;
        let total_receipts = js_sys::Reflect::get(&receipt_stats, &"total".into())
            .unwrap_or_else(|_| JsValue::from_f64(0.0))
            .as_f64()
            .unwrap_or(0.0) as usize;
        let sent_receipts = js_sys::Reflect::get(&receipt_stats, &"sent".into())
            .unwrap_or_else(|_| JsValue::from_f64(0.0))
            .as_f64()
            .unwrap_or(0.0) as usize;
        let received_receipts = js_sys::Reflect::get(&receipt_stats, &"received".into())
            .unwrap_or_else(|_| JsValue::from_f64(0.0))
            .as_f64()
            .unwrap_or(0.0) as usize;

        // Get subscription counts
        let all_subscriptions = self.subscription_storage.list_all_subscriptions().await?;
        let subscription_count = all_subscriptions.len();
        let active_subscriptions = self
            .subscription_storage
            .list_active_subscriptions()
            .await?;
        let active_count = active_subscriptions.len();

        // Build stats object
        let stats = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&stats, &"contacts".into(), &contact_count.into());
        let _ = js_sys::Reflect::set(&stats, &"payment_methods".into(), &method_count.into());
        let _ = js_sys::Reflect::set(&stats, &"preferred_methods".into(), &preferred_count.into());
        let _ = js_sys::Reflect::set(&stats, &"total_receipts".into(), &total_receipts.into());
        let _ = js_sys::Reflect::set(&stats, &"sent_receipts".into(), &sent_receipts.into());
        let _ = js_sys::Reflect::set(
            &stats,
            &"received_receipts".into(),
            &received_receipts.into(),
        );
        let _ = js_sys::Reflect::set(
            &stats,
            &"total_subscriptions".into(),
            &subscription_count.into(),
        );
        let _ = js_sys::Reflect::set(&stats, &"active_subscriptions".into(), &active_count.into());

        Ok(stats.into())
    }

    /// Get recent activity summary
    ///
    /// Returns an array of recent activity items from receipts and subscriptions.
    /// Each item includes: type, timestamp, description.
    ///
    /// # Arguments
    ///
    /// * `current_pubkey` - Current user's public key
    /// * `limit` - Maximum number of items to return
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmDashboard;
    ///
    /// let dashboard = WasmDashboard::new();
    /// let activity = dashboard.get_recent_activity("my_pubkey", 10).await?;
    /// ```
    pub async fn get_recent_activity(
        &self,
        current_pubkey: &str,
        limit: usize,
    ) -> Result<Vec<JsValue>, JsValue> {
        let mut activities = Vec::new();

        // Get recent receipts
        let receipts = self.receipt_storage.list_receipts().await?;
        for receipt_js in receipts.iter().take(limit) {
            if let Some(json_str) = receipt_js.as_string() {
                if let Ok(receipt_obj) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    let payer = receipt_obj
                        .get("payer")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let amount = receipt_obj
                        .get("amount")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");
                    let currency = receipt_obj
                        .get("currency")
                        .and_then(|v| v.as_str())
                        .unwrap_or("SAT");
                    let timestamp = receipt_obj
                        .get("timestamp")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    let direction = if payer == current_pubkey {
                        "sent"
                    } else {
                        "received"
                    };

                    let activity = js_sys::Object::new();
                    let _ = js_sys::Reflect::set(&activity, &"type".into(), &"receipt".into());
                    let _ = js_sys::Reflect::set(&activity, &"timestamp".into(), &timestamp.into());
                    let _ = js_sys::Reflect::set(&activity, &"direction".into(), &direction.into());
                    let _ = js_sys::Reflect::set(&activity, &"amount".into(), &amount.into());
                    let _ = js_sys::Reflect::set(&activity, &"currency".into(), &currency.into());

                    activities.push(activity.into());
                }
            }
        }

        // Sort by timestamp (newest first)
        activities.sort_by(|a, b| {
            let ts_a = js_sys::Reflect::get(a, &"timestamp".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let ts_b = js_sys::Reflect::get(b, &"timestamp".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            ts_b.partial_cmp(&ts_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        activities.truncate(limit);

        Ok(activities)
    }

    /// Check if setup is complete
    ///
    /// Returns true if the user has:
    /// - At least one contact
    /// - At least one payment method configured
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmDashboard;
    ///
    /// let dashboard = WasmDashboard::new();
    /// let is_ready = dashboard.is_setup_complete().await?;
    /// ```
    pub async fn is_setup_complete(&self) -> Result<bool, JsValue> {
        let contacts = self.contact_storage.list_contacts().await?;
        let methods = self.method_storage.list_methods().await?;

        Ok(!contacts.is_empty() && !methods.is_empty())
    }

    /// Get setup checklist
    ///
    /// Returns an object with boolean flags for each setup step:
    /// - has_identity: Whether identity is set (checked by caller)
    /// - has_contacts: Whether user has any contacts
    /// - has_payment_methods: Whether user has configured methods
    /// - has_preferred_method: Whether user has a preferred method
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmDashboard;
    ///
    /// let dashboard = WasmDashboard::new();
    /// let checklist = dashboard.get_setup_checklist().await?;
    /// ```
    pub async fn get_setup_checklist(&self) -> Result<JsValue, JsValue> {
        let contacts = self.contact_storage.list_contacts().await?;
        let has_contacts = !contacts.is_empty();

        let methods = self.method_storage.list_methods().await?;
        let has_methods = !methods.is_empty();

        let preferred = self.method_storage.get_preferred_methods().await?;
        let has_preferred = !preferred.is_empty();

        let checklist = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&checklist, &"has_contacts".into(), &has_contacts.into());
        let _ = js_sys::Reflect::set(
            &checklist,
            &"has_payment_methods".into(),
            &has_methods.into(),
        );
        let _ = js_sys::Reflect::set(
            &checklist,
            &"has_preferred_method".into(),
            &has_preferred.into(),
        );

        Ok(checklist.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_dashboard_creation() {
        let _dashboard = WasmDashboard::new();
        // Just verify it creates without errors (no panic)
    }

    #[wasm_bindgen_test]
    async fn test_get_overview_stats() {
        let dashboard = WasmDashboard::new();
        let stats = dashboard.get_overview_stats("test_pubkey").await.unwrap();

        // Verify stats object has expected properties
        let contacts = js_sys::Reflect::get(&stats, &"contacts".into()).unwrap();
        assert!(contacts.as_f64().is_some());

        let methods = js_sys::Reflect::get(&stats, &"payment_methods".into()).unwrap();
        assert!(methods.as_f64().is_some());
    }

    #[wasm_bindgen_test]
    async fn test_setup_checklist() {
        let dashboard = WasmDashboard::new();
        let checklist = dashboard.get_setup_checklist().await.unwrap();

        // Verify checklist has expected properties
        let has_contacts = js_sys::Reflect::get(&checklist, &"has_contacts".into()).unwrap();
        assert!(has_contacts.as_bool().is_some());

        let has_methods = js_sys::Reflect::get(&checklist, &"has_payment_methods".into()).unwrap();
        assert!(has_methods.as_bool().is_some());
    }

    #[wasm_bindgen_test]
    async fn test_is_setup_complete() {
        let dashboard = WasmDashboard::new();
        let is_complete = dashboard.is_setup_complete().await.unwrap();

        // Should be bool
        let _is_bool: bool = is_complete;
    }

    #[wasm_bindgen_test]
    async fn test_get_recent_activity() {
        let dashboard = WasmDashboard::new();
        let activity = dashboard
            .get_recent_activity("test_pubkey", 10)
            .await
            .unwrap();

        // Should return an array (len() is always non-negative for Vec)
        let _len = activity.len();
    }
}
