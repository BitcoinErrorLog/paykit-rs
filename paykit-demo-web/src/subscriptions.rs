//! Subscriptions and payment requests WASM bindings

use crate::types::{
    Amount, PaymentFrequency, PaymentRequest, SignedSubscription, Subscription, SubscriptionTerms,
};
use paykit_lib::{MethodId, PublicKey};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

/// JavaScript-friendly payment request
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmPaymentRequest {
    inner: PaymentRequest,
}

#[wasm_bindgen]
impl WasmPaymentRequest {
    /// Create a new payment request
    #[wasm_bindgen(constructor)]
    pub fn new(
        from_pubkey: &str,
        to_pubkey: &str,
        amount: &str,
        currency: &str,
        method: &str,
    ) -> Result<WasmPaymentRequest, JsValue> {
        let from = PublicKey::from_str(from_pubkey)
            .map_err(|e| JsValue::from_str(&format!("Invalid from pubkey: {}", e)))?;
        let to = PublicKey::from_str(to_pubkey)
            .map_err(|e| JsValue::from_str(&format!("Invalid to pubkey: {}", e)))?;

        // Parse amount as satoshis
        let amount_sats: i64 = amount
            .parse()
            .map_err(|_| JsValue::from_str(&format!("Invalid amount: {}", amount)))?;

        let request = PaymentRequest::new(
            from,
            to,
            Amount::from_sats(amount_sats),
            currency.to_string(),
            MethodId(method.to_string()),
        );

        Ok(WasmPaymentRequest { inner: request })
    }

    /// Add description to the request
    pub fn with_description(mut self, description: &str) -> Self {
        self.inner = self.inner.with_description(description.to_string());
        self
    }

    /// Add expiration time (Unix timestamp)
    pub fn with_expiration(mut self, expires_at: i64) -> Self {
        self.inner = self.inner.with_expiration(expires_at);
        self
    }

    /// Get request ID
    #[wasm_bindgen(getter)]
    pub fn request_id(&self) -> String {
        self.inner.request_id.clone()
    }

    /// Get from public key
    #[wasm_bindgen(getter)]
    pub fn from(&self) -> String {
        self.inner.from.to_string()
    }

    /// Get to public key
    #[wasm_bindgen(getter)]
    pub fn to(&self) -> String {
        self.inner.to.to_string()
    }

    /// Get amount
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> String {
        self.inner.amount.to_string()
    }

    /// Get currency
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> String {
        self.inner.currency.clone()
    }

    /// Get description
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    /// Get created timestamp
    #[wasm_bindgen(getter)]
    pub fn created_at(&self) -> i64 {
        self.inner.created_at
    }

    /// Get expiration timestamp
    #[wasm_bindgen(getter)]
    pub fn expires_at(&self) -> Option<i64> {
        self.inner.expires_at
    }

    /// Check if expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.inner.expires_at {
            chrono::Utc::now().timestamp() > expires_at
        } else {
            false
        }
    }

    /// Convert to JSON
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Create from JSON
    pub fn from_json(json: &str) -> Result<WasmPaymentRequest, JsValue> {
        let request: PaymentRequest = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        Ok(WasmPaymentRequest { inner: request })
    }
}

/// Request-only storage manager for browser (simplified wrapper)
/// For full subscription storage, use WasmSubscriptionAgreementStorage
#[wasm_bindgen]
pub struct WasmRequestStorage {
    storage_key: String,
}

#[wasm_bindgen]
impl WasmRequestStorage {
    /// Create new storage manager
    #[wasm_bindgen(constructor)]
    pub fn new(storage_key: Option<String>) -> Self {
        Self {
            storage_key: storage_key.unwrap_or_else(|| "paykit_requests".to_string()),
        }
    }

    /// Save a payment request to browser localStorage
    pub async fn save_request(&self, request: &WasmPaymentRequest) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:request:{}", self.storage_key, request.request_id());
        let json = request.to_json()?;

        storage
            .set_item(&key, &json)
            .map_err(|e| JsValue::from_str(&format!("Failed to save: {:?}", e)))?;

        Ok(())
    }

    /// Get a payment request by ID
    pub async fn get_request(
        &self,
        request_id: &str,
    ) -> Result<Option<WasmPaymentRequest>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:request:{}", self.storage_key, request_id);

        match storage.get_item(&key) {
            Ok(Some(json)) => {
                let request = WasmPaymentRequest::from_json(&json)?;
                Ok(Some(request))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(JsValue::from_str(&format!("Failed to get: {:?}", e))),
        }
    }

    /// List all payment requests
    pub async fn list_requests(&self) -> Result<Vec<JsValue>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let mut requests = Vec::new();
        let prefix = format!("{}:request:", self.storage_key);

        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix) {
                    if let Ok(Some(json)) = storage.get_item(&key) {
                        if let Ok(request) = WasmPaymentRequest::from_json(&json) {
                            // Convert to JsValue
                            let js_obj = js_sys::Object::new();
                            js_sys::Reflect::set(
                                &js_obj,
                                &"request_id".into(),
                                &request.request_id().into(),
                            )?;
                            js_sys::Reflect::set(&js_obj, &"from".into(), &request.from().into())?;
                            js_sys::Reflect::set(&js_obj, &"to".into(), &request.to().into())?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"amount".into(),
                                &request.amount().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"currency".into(),
                                &request.currency().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"created_at".into(),
                                &request.created_at().into(),
                            )?;

                            if let Some(desc) = request.description() {
                                js_sys::Reflect::set(&js_obj, &"description".into(), &desc.into())?;
                            }
                            if let Some(exp) = request.expires_at() {
                                js_sys::Reflect::set(&js_obj, &"expires_at".into(), &exp.into())?;
                            }
                            js_sys::Reflect::set(
                                &js_obj,
                                &"is_expired".into(),
                                &request.is_expired().into(),
                            )?;

                            requests.push(js_obj.into());
                        }
                    }
                }
            }
        }

        Ok(requests)
    }

    /// Delete a payment request
    pub async fn delete_request(&self, request_id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:request:{}", self.storage_key, request_id);
        storage
            .remove_item(&key)
            .map_err(|e| JsValue::from_str(&format!("Failed to delete: {:?}", e)))?;

        Ok(())
    }

    /// Clear all payment requests
    pub async fn clear_all(&self) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let prefix = format!("{}:request:", self.storage_key);
        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        let mut keys_to_remove = Vec::new();
        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix) {
                    keys_to_remove.push(key);
                }
            }
        }

        for key in keys_to_remove {
            storage
                .remove_item(&key)
                .map_err(|e| JsValue::from_str(&format!("Failed to clear: {:?}", e)))?;
        }

        Ok(())
    }
}

/// Utility functions for subscriptions
#[wasm_bindgen]
pub fn format_timestamp(timestamp: i64) -> String {
    if let Some(dt) = chrono::DateTime::from_timestamp(timestamp, 0) {
        dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    } else {
        "Invalid timestamp".to_string()
    }
}

#[wasm_bindgen]
pub fn is_valid_pubkey(pubkey: &str) -> bool {
    PublicKey::from_str(pubkey).is_ok()
}

// ============================================================
// Phase 2: Subscription Agreements
// ============================================================

/// JavaScript-friendly subscription
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmSubscription {
    inner: Subscription,
}

#[wasm_bindgen]
impl WasmSubscription {
    /// Create a new subscription
    #[wasm_bindgen(constructor)]
    pub fn new(
        subscriber_pubkey: &str,
        provider_pubkey: &str,
        amount: &str,
        currency: &str,
        frequency: &str,
        description: &str,
    ) -> Result<WasmSubscription, JsValue> {
        let subscriber = PublicKey::from_str(subscriber_pubkey)
            .map_err(|e| JsValue::from_str(&format!("Invalid subscriber pubkey: {}", e)))?;
        let provider = PublicKey::from_str(provider_pubkey)
            .map_err(|e| JsValue::from_str(&format!("Invalid provider pubkey: {}", e)))?;

        // Parse frequency
        let payment_frequency = parse_frequency_wasm(frequency)?;

        // Parse amount as satoshis
        let amount_sats: i64 = amount
            .parse()
            .map_err(|_| JsValue::from_str(&format!("Invalid amount: {}", amount)))?;

        let terms = SubscriptionTerms::new(
            Amount::from_sats(amount_sats),
            currency.to_string(),
            payment_frequency,
            MethodId("lightning".to_string()),
            description.to_string(),
        );

        let subscription = Subscription::new(subscriber, provider, terms);

        Ok(WasmSubscription {
            inner: subscription,
        })
    }

    /// Get subscription ID
    #[wasm_bindgen(getter)]
    pub fn subscription_id(&self) -> String {
        self.inner.subscription_id.clone()
    }

    /// Get subscriber public key
    #[wasm_bindgen(getter)]
    pub fn subscriber(&self) -> String {
        self.inner.subscriber.to_z32()
    }

    /// Get provider public key
    #[wasm_bindgen(getter)]
    pub fn provider(&self) -> String {
        self.inner.provider.to_z32()
    }

    /// Get amount
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> String {
        self.inner.terms.amount.to_string()
    }

    /// Get currency
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> String {
        self.inner.terms.currency.clone()
    }

    /// Get frequency
    #[wasm_bindgen(getter)]
    pub fn frequency(&self) -> String {
        self.inner.terms.frequency.to_string()
    }

    /// Get description
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.inner.terms.description.clone()
    }

    /// Get created timestamp
    #[wasm_bindgen(getter)]
    pub fn created_at(&self) -> i64 {
        self.inner.created_at
    }

    /// Get starts timestamp
    #[wasm_bindgen(getter)]
    pub fn starts_at(&self) -> i64 {
        self.inner.starts_at
    }

    /// Get ends timestamp (or null)
    #[wasm_bindgen(getter)]
    pub fn ends_at(&self) -> Option<i64> {
        self.inner.ends_at
    }

    /// Check if active
    pub fn is_active(&self) -> bool {
        self.inner.is_active()
    }

    /// Check if expired
    pub fn is_expired(&self) -> bool {
        self.inner.is_expired()
    }

    /// Validate subscription
    pub fn validate(&self) -> Result<(), JsValue> {
        self.inner
            .validate()
            .map_err(|e| JsValue::from_str(&format!("Validation error: {}", e)))
    }
}

/// JavaScript-friendly signed subscription
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmSignedSubscription {
    inner: SignedSubscription,
}

#[wasm_bindgen]
impl WasmSignedSubscription {
    /// Get subscription details
    pub fn subscription(&self) -> WasmSubscription {
        WasmSubscription {
            inner: self.inner.subscription.clone(),
        }
    }

    /// Check if signatures are valid
    pub fn verify_signatures(&self) -> Result<bool, JsValue> {
        self.inner
            .verify_signatures()
            .map_err(|e| JsValue::from_str(&format!("Verification error: {}", e)))
    }

    /// Check if active
    pub fn is_active(&self) -> bool {
        self.inner.is_active()
    }

    /// Check if expired
    pub fn is_expired(&self) -> bool {
        self.inner.is_expired()
    }
}

/// Storage for subscription agreements (WASM)
///
/// Full implementation using browser localStorage
#[wasm_bindgen]
pub struct WasmSubscriptionAgreementStorage {
    storage_key: String,
}

impl Default for WasmSubscriptionAgreementStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmSubscriptionAgreementStorage {
    /// Create new storage (uses browser localStorage)
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmSubscriptionAgreementStorage {
        WasmSubscriptionAgreementStorage {
            storage_key: "paykit_subscriptions".to_string(),
        }
    }

    /// Save a subscription
    pub async fn save_subscription(&self, subscription: &WasmSubscription) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!(
            "{}:sub:{}",
            self.storage_key,
            subscription.subscription_id()
        );
        let json = serde_json::to_string(&subscription.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

        storage
            .set_item(&key, &json)
            .map_err(|e| JsValue::from_str(&format!("Failed to save: {:?}", e)))?;

        Ok(())
    }

    /// Get a subscription by ID
    pub async fn get_subscription(&self, id: &str) -> Result<Option<WasmSubscription>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:sub:{}", self.storage_key, id);

        match storage.get_item(&key) {
            Ok(Some(json)) => {
                let inner: Subscription = serde_json::from_str(&json)
                    .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
                Ok(Some(WasmSubscription { inner }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(JsValue::from_str(&format!("Failed to get: {:?}", e))),
        }
    }

    /// Save a signed subscription
    pub async fn save_signed_subscription(
        &self,
        signed: &WasmSignedSubscription,
    ) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!(
            "{}:signed:{}",
            self.storage_key, signed.inner.subscription.subscription_id
        );
        let json = serde_json::to_string(&signed.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

        storage
            .set_item(&key, &json)
            .map_err(|e| JsValue::from_str(&format!("Failed to save: {:?}", e)))?;

        Ok(())
    }

    /// Get a signed subscription by ID
    pub async fn get_signed_subscription(
        &self,
        id: &str,
    ) -> Result<Option<WasmSignedSubscription>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:signed:{}", self.storage_key, id);

        match storage.get_item(&key) {
            Ok(Some(json)) => {
                let inner: SignedSubscription = serde_json::from_str(&json)
                    .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
                Ok(Some(WasmSignedSubscription { inner }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(JsValue::from_str(&format!("Failed to get: {:?}", e))),
        }
    }

    /// Delete a subscription by ID
    pub async fn delete_subscription(&self, id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:sub:{}", self.storage_key, id);
        storage
            .remove_item(&key)
            .map_err(|e| JsValue::from_str(&format!("Failed to delete: {:?}", e)))?;

        Ok(())
    }

    /// Delete a signed subscription by ID
    pub async fn delete_signed_subscription(&self, id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:signed:{}", self.storage_key, id);
        storage
            .remove_item(&key)
            .map_err(|e| JsValue::from_str(&format!("Failed to delete: {:?}", e)))?;

        Ok(())
    }

    /// List active subscriptions
    pub async fn list_active_subscriptions(&self) -> Result<Vec<JsValue>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let mut subscriptions = Vec::new();
        let prefix = format!("{}:signed:", self.storage_key);
        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix) {
                    if let Ok(Some(json)) = storage.get_item(&key) {
                        if let Ok(signed) = serde_json::from_str::<SignedSubscription>(&json) {
                            if signed.is_active() {
                                // Create JS object with subscription details
                                let js_obj = js_sys::Object::new();
                                js_sys::Reflect::set(
                                    &js_obj,
                                    &"subscription_id".into(),
                                    &signed.subscription.subscription_id.as_str().into(),
                                )?;
                                js_sys::Reflect::set(
                                    &js_obj,
                                    &"subscriber".into(),
                                    &signed.subscription.subscriber.to_z32().into(),
                                )?;
                                js_sys::Reflect::set(
                                    &js_obj,
                                    &"provider".into(),
                                    &signed.subscription.provider.to_z32().into(),
                                )?;
                                js_sys::Reflect::set(
                                    &js_obj,
                                    &"amount".into(),
                                    &signed.subscription.terms.amount.to_string().into(),
                                )?;
                                js_sys::Reflect::set(
                                    &js_obj,
                                    &"currency".into(),
                                    &signed.subscription.terms.currency.as_str().into(),
                                )?;
                                js_sys::Reflect::set(
                                    &js_obj,
                                    &"frequency".into(),
                                    &signed.subscription.terms.frequency.to_string().into(),
                                )?;
                                js_sys::Reflect::set(
                                    &js_obj,
                                    &"starts_at".into(),
                                    &signed.subscription.starts_at.into(),
                                )?;

                                subscriptions.push(js_obj.into());
                            }
                        }
                    }
                }
            }
        }

        Ok(subscriptions)
    }

    /// List all subscriptions (including inactive)
    pub async fn list_all_subscriptions(&self) -> Result<Vec<JsValue>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let mut subscriptions = Vec::new();
        let prefix = format!("{}:signed:", self.storage_key);
        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix) {
                    if let Ok(Some(json)) = storage.get_item(&key) {
                        if let Ok(signed) = serde_json::from_str::<SignedSubscription>(&json) {
                            let js_obj = js_sys::Object::new();
                            js_sys::Reflect::set(
                                &js_obj,
                                &"subscription_id".into(),
                                &signed.subscription.subscription_id.as_str().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"subscriber".into(),
                                &signed.subscription.subscriber.to_z32().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"provider".into(),
                                &signed.subscription.provider.to_z32().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"amount".into(),
                                &signed.subscription.terms.amount.to_string().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"currency".into(),
                                &signed.subscription.terms.currency.as_str().into(),
                            )?;
                            let is_active = signed.is_active();
                            let is_expired = signed.is_expired();
                            js_sys::Reflect::set(&js_obj, &"is_active".into(), &is_active.into())?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"is_expired".into(),
                                &is_expired.into(),
                            )?;

                            subscriptions.push(js_obj.into());
                        }
                    }
                }
            }
        }

        Ok(subscriptions)
    }

    /// Clear all subscriptions
    pub async fn clear_all(&self) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let prefix = self.storage_key.clone();
        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        let mut keys_to_remove = Vec::new();
        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix) {
                    keys_to_remove.push(key);
                }
            }
        }

        for key in keys_to_remove {
            storage
                .remove_item(&key)
                .map_err(|e| JsValue::from_str(&format!("Failed to clear: {:?}", e)))?;
        }

        Ok(())
    }
}

// Helper: parse frequency string for WASM
fn parse_frequency_wasm(freq: &str) -> Result<PaymentFrequency, JsValue> {
    match freq.to_lowercase().as_str() {
        "daily" => Ok(PaymentFrequency::Daily),
        "weekly" => Ok(PaymentFrequency::Weekly),
        freq_str if freq_str.starts_with("monthly") => {
            let day = if let Some(day_str) = freq_str.strip_prefix("monthly:") {
                day_str.parse::<u8>()
                    .map_err(|_| JsValue::from_str(&format!("Invalid day of month: {}", day_str)))?
            } else {
                1
            };
            if day == 0 || day > 31 {
                return Err(JsValue::from_str("Day of month must be between 1 and 31"));
            }
            Ok(PaymentFrequency::Monthly { day_of_month: day })
        }
        freq_str if freq_str.starts_with("yearly") => {
            let parts: Vec<&str> = freq_str.split(':').collect();
            if parts.len() != 3 {
                return Err(JsValue::from_str("Yearly frequency must be in format 'yearly:MONTH:DAY'"));
            }
            let month = parts[1].parse::<u8>()
                .map_err(|_| JsValue::from_str(&format!("Invalid month: {}", parts[1])))?;
            let day = parts[2].parse::<u8>()
                .map_err(|_| JsValue::from_str(&format!("Invalid day: {}", parts[2])))?;
            if month == 0 || month > 12 {
                return Err(JsValue::from_str("Month must be between 1 and 12"));
            }
            if day == 0 || day > 31 {
                return Err(JsValue::from_str("Day must be between 1 and 31"));
            }
            Ok(PaymentFrequency::Yearly { month, day })
        }
        freq_str if freq_str.starts_with("custom:") => {
            let interval_str = freq_str.strip_prefix("custom:")
                .ok_or_else(|| JsValue::from_str("Invalid custom frequency format"))?;
            let interval = interval_str.parse::<u64>()
                .map_err(|_| JsValue::from_str(&format!("Invalid interval: {}", interval_str)))?;
            Ok(PaymentFrequency::Custom { interval_seconds: interval })
        }
        _ => Err(JsValue::from_str(&format!(
            "Invalid frequency: {}. Use daily, weekly, monthly[:DAY], yearly:MONTH:DAY, or custom:SECONDS",
            freq
        ))),
    }
}

// ============================================================
// Phase 3: Auto-Pay WASM Bindings
// ============================================================

/// WASM-friendly auto-pay rule
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmAutoPayRule {
    id: String,
    subscription_id: String,
    peer_pubkey: String,
    max_amount: i64,
    period_seconds: u64,
    enabled: bool,
    require_confirmation: bool,
}

#[wasm_bindgen]
impl WasmAutoPayRule {
    /// Create a new auto-pay rule
    #[wasm_bindgen(constructor)]
    pub fn new(
        subscription_id: &str,
        peer_pubkey: &str,
        max_amount: i64,
        period_seconds: u64,
        require_confirmation: bool,
    ) -> Result<WasmAutoPayRule, JsValue> {
        if subscription_id.is_empty() {
            return Err(JsValue::from_str("Subscription ID cannot be empty"));
        }
        if max_amount <= 0 {
            return Err(JsValue::from_str("Max amount must be positive"));
        }
        if period_seconds == 0 {
            return Err(JsValue::from_str("Period must be non-zero"));
        }

        Ok(WasmAutoPayRule {
            id: format!("autopay_{}", uuid::Uuid::new_v4()),
            subscription_id: subscription_id.to_string(),
            peer_pubkey: peer_pubkey.to_string(),
            max_amount,
            period_seconds,
            enabled: true,
            require_confirmation,
        })
    }

    /// Get the rule ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Get the subscription ID
    #[wasm_bindgen(getter)]
    pub fn subscription_id(&self) -> String {
        self.subscription_id.clone()
    }

    /// Get the peer public key
    #[wasm_bindgen(getter)]
    pub fn peer_pubkey(&self) -> String {
        self.peer_pubkey.clone()
    }

    /// Get the maximum amount
    #[wasm_bindgen(getter)]
    pub fn max_amount(&self) -> i64 {
        self.max_amount
    }

    /// Get the period in seconds
    #[wasm_bindgen(getter)]
    pub fn period_seconds(&self) -> u64 {
        self.period_seconds
    }

    /// Check if the rule is enabled
    #[wasm_bindgen(getter)]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Enable the rule
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable the rule
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if manual confirmation is required
    #[wasm_bindgen(getter)]
    pub fn require_confirmation(&self) -> bool {
        self.require_confirmation
    }

    /// Set whether manual confirmation is required
    pub fn set_require_confirmation(&mut self, required: bool) {
        self.require_confirmation = required;
    }

    /// Convert to JSON for storage
    pub fn to_json(&self) -> Result<String, JsValue> {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"id".into(), &self.id.clone().into())?;
        js_sys::Reflect::set(
            &obj,
            &"subscription_id".into(),
            &self.subscription_id.clone().into(),
        )?;
        js_sys::Reflect::set(
            &obj,
            &"peer_pubkey".into(),
            &self.peer_pubkey.clone().into(),
        )?;
        js_sys::Reflect::set(&obj, &"max_amount".into(), &self.max_amount.into())?;
        js_sys::Reflect::set(&obj, &"period_seconds".into(), &self.period_seconds.into())?;
        js_sys::Reflect::set(&obj, &"enabled".into(), &self.enabled.into())?;
        js_sys::Reflect::set(
            &obj,
            &"require_confirmation".into(),
            &self.require_confirmation.into(),
        )?;
        js_sys::JSON::stringify(&obj.into())
            .map_err(|_| JsValue::from_str("Failed to serialize"))?
            .as_string()
            .ok_or_else(|| JsValue::from_str("Failed to convert to string"))
    }

    /// Create from JSON
    pub fn from_json(json: &str) -> Result<WasmAutoPayRule, JsValue> {
        let obj = js_sys::JSON::parse(json).map_err(|_| JsValue::from_str("Invalid JSON"))?;

        let id = js_sys::Reflect::get(&obj, &"id".into())
            .ok()
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsValue::from_str("Missing id"))?;

        let subscription_id = js_sys::Reflect::get(&obj, &"subscription_id".into())
            .ok()
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsValue::from_str("Missing subscription_id"))?;

        let peer_pubkey = js_sys::Reflect::get(&obj, &"peer_pubkey".into())
            .ok()
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsValue::from_str("Missing peer_pubkey"))?;

        let max_amount = js_sys::Reflect::get(&obj, &"max_amount".into())
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as i64)
            .ok_or_else(|| JsValue::from_str("Missing or invalid max_amount"))?;

        let period_seconds = js_sys::Reflect::get(&obj, &"period_seconds".into())
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as u64)
            .ok_or_else(|| JsValue::from_str("Missing or invalid period_seconds"))?;

        let enabled = js_sys::Reflect::get(&obj, &"enabled".into())
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let require_confirmation = js_sys::Reflect::get(&obj, &"require_confirmation".into())
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(WasmAutoPayRule {
            id,
            subscription_id,
            peer_pubkey,
            max_amount,
            period_seconds,
            enabled,
            require_confirmation,
        })
    }
}

/// WASM-friendly peer spending limit
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmPeerSpendingLimit {
    peer_pubkey: String,
    total_limit: i64,
    current_spent: i64,
    period_seconds: u64,
    period_start: i64,
}

#[wasm_bindgen]
impl WasmPeerSpendingLimit {
    /// Create a new peer spending limit
    #[wasm_bindgen(constructor)]
    pub fn new(
        peer_pubkey: &str,
        total_limit: i64,
        period_seconds: u64,
    ) -> Result<WasmPeerSpendingLimit, JsValue> {
        if total_limit <= 0 {
            return Err(JsValue::from_str("Total limit must be positive"));
        }
        if period_seconds == 0 {
            return Err(JsValue::from_str("Period must be non-zero"));
        }

        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Ok(WasmPeerSpendingLimit {
            peer_pubkey: peer_pubkey.to_string(),
            total_limit,
            current_spent: 0,
            period_seconds,
            period_start: now,
        })
    }

    /// Get the peer public key
    #[wasm_bindgen(getter)]
    pub fn peer_pubkey(&self) -> String {
        self.peer_pubkey.clone()
    }

    /// Get the total limit
    #[wasm_bindgen(getter)]
    pub fn total_limit(&self) -> i64 {
        self.total_limit
    }

    /// Get the current spent amount
    #[wasm_bindgen(getter)]
    pub fn current_spent(&self) -> i64 {
        self.current_spent
    }

    /// Get the remaining limit
    #[wasm_bindgen(getter)]
    pub fn remaining_limit(&self) -> i64 {
        (self.total_limit - self.current_spent).max(0)
    }

    /// Get the period in seconds
    #[wasm_bindgen(getter)]
    pub fn period_seconds(&self) -> u64 {
        self.period_seconds
    }

    /// Get the period start timestamp
    #[wasm_bindgen(getter)]
    pub fn period_start(&self) -> i64 {
        self.period_start
    }

    /// Check if a payment amount is allowed
    pub fn can_spend(&self, amount: i64) -> bool {
        // Check if period has expired and should reset
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        if now - self.period_start > self.period_seconds as i64 {
            // Period expired, can spend up to limit
            amount <= self.total_limit
        } else {
            // Within period, check remaining
            amount <= self.remaining_limit()
        }
    }

    /// Record a payment
    pub fn record_payment(&mut self, amount: i64) -> Result<(), JsValue> {
        if !self.can_spend(amount) {
            return Err(JsValue::from_str("Payment exceeds spending limit"));
        }

        // Check if period needs to be reset
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        if now - self.period_start > self.period_seconds as i64 {
            // Reset for new period
            self.period_start = now;
            self.current_spent = amount;
        } else {
            // Add to current period
            self.current_spent += amount;
        }

        Ok(())
    }

    /// Reset the spending counter
    pub fn reset(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.period_start = now;
        self.current_spent = 0;
    }

    /// Convert to JSON for storage
    pub fn to_json(&self) -> Result<String, JsValue> {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &"peer_pubkey".into(),
            &self.peer_pubkey.clone().into(),
        )?;
        js_sys::Reflect::set(&obj, &"total_limit".into(), &self.total_limit.into())?;
        js_sys::Reflect::set(&obj, &"current_spent".into(), &self.current_spent.into())?;
        js_sys::Reflect::set(&obj, &"period_seconds".into(), &self.period_seconds.into())?;
        js_sys::Reflect::set(&obj, &"period_start".into(), &self.period_start.into())?;
        js_sys::JSON::stringify(&obj.into())
            .map_err(|_| JsValue::from_str("Failed to serialize"))?
            .as_string()
            .ok_or_else(|| JsValue::from_str("Failed to convert to string"))
    }

    /// Create from JSON
    pub fn from_json(json: &str) -> Result<WasmPeerSpendingLimit, JsValue> {
        let obj = js_sys::JSON::parse(json).map_err(|_| JsValue::from_str("Invalid JSON"))?;

        let peer_pubkey = js_sys::Reflect::get(&obj, &"peer_pubkey".into())
            .ok()
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsValue::from_str("Missing peer_pubkey"))?;

        let total_limit = js_sys::Reflect::get(&obj, &"total_limit".into())
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as i64)
            .ok_or_else(|| JsValue::from_str("Missing or invalid total_limit"))?;

        let current_spent = js_sys::Reflect::get(&obj, &"current_spent".into())
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as i64)
            .unwrap_or(0);

        let period_seconds = js_sys::Reflect::get(&obj, &"period_seconds".into())
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as u64)
            .ok_or_else(|| JsValue::from_str("Missing or invalid period_seconds"))?;

        let period_start = js_sys::Reflect::get(&obj, &"period_start".into())
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as i64)
            .unwrap_or_else(|| {
                use std::time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64
            });

        Ok(WasmPeerSpendingLimit {
            peer_pubkey,
            total_limit,
            current_spent,
            period_seconds,
            period_start,
        })
    }
}

/// Storage for auto-pay rules in browser localStorage
#[wasm_bindgen]
pub struct WasmAutoPayRuleStorage {
    storage_key: String,
}

impl Default for WasmAutoPayRuleStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmAutoPayRuleStorage {
    /// Create new storage manager
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmAutoPayRuleStorage {
        WasmAutoPayRuleStorage {
            storage_key: "paykit_autopay".to_string(),
        }
    }

    /// Save an auto-pay rule
    pub async fn save_autopay_rule(&self, rule: &WasmAutoPayRule) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:rule:{}", self.storage_key, rule.subscription_id());
        let json = rule.to_json()?;

        storage
            .set_item(&key, &json)
            .map_err(|e| JsValue::from_str(&format!("Failed to save: {:?}", e)))?;

        Ok(())
    }

    /// Get an auto-pay rule by subscription ID
    pub async fn get_autopay_rule(
        &self,
        subscription_id: &str,
    ) -> Result<Option<WasmAutoPayRule>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:rule:{}", self.storage_key, subscription_id);

        match storage.get_item(&key) {
            Ok(Some(json)) => {
                let rule = WasmAutoPayRule::from_json(&json)?;
                Ok(Some(rule))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(JsValue::from_str(&format!("Failed to get: {:?}", e))),
        }
    }

    /// List all auto-pay rules
    pub async fn list_autopay_rules(&self) -> Result<Vec<JsValue>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let mut rules = Vec::new();
        let prefix = format!("{}:rule:", self.storage_key);
        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix) {
                    if let Ok(Some(json)) = storage.get_item(&key) {
                        if let Ok(rule) = WasmAutoPayRule::from_json(&json) {
                            let js_obj = js_sys::Object::new();
                            js_sys::Reflect::set(&js_obj, &"id".into(), &rule.id().into())?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"subscription_id".into(),
                                &rule.subscription_id().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"peer_pubkey".into(),
                                &rule.peer_pubkey().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"max_amount".into(),
                                &rule.max_amount().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"period_seconds".into(),
                                &rule.period_seconds().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"enabled".into(),
                                &rule.enabled().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"require_confirmation".into(),
                                &rule.require_confirmation().into(),
                            )?;

                            rules.push(js_obj.into());
                        }
                    }
                }
            }
        }

        Ok(rules)
    }

    /// Delete an auto-pay rule
    pub async fn delete_autopay_rule(&self, subscription_id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:rule:{}", self.storage_key, subscription_id);
        storage
            .remove_item(&key)
            .map_err(|e| JsValue::from_str(&format!("Failed to delete: {:?}", e)))?;

        Ok(())
    }

    /// Clear all auto-pay rules
    pub async fn clear_all(&self) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let prefix = format!("{}:rule:", self.storage_key);
        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        let mut keys_to_remove = Vec::new();
        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix) {
                    keys_to_remove.push(key);
                }
            }
        }

        for key in keys_to_remove {
            storage
                .remove_item(&key)
                .map_err(|e| JsValue::from_str(&format!("Failed to clear: {:?}", e)))?;
        }

        Ok(())
    }
}

/// Storage for peer spending limits in browser localStorage
#[wasm_bindgen]
pub struct WasmPeerSpendingLimitStorage {
    storage_key: String,
}

impl Default for WasmPeerSpendingLimitStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmPeerSpendingLimitStorage {
    /// Create new storage manager
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmPeerSpendingLimitStorage {
        WasmPeerSpendingLimitStorage {
            storage_key: "paykit_limits".to_string(),
        }
    }

    /// Save a peer spending limit
    pub async fn save_peer_limit(&self, limit: &WasmPeerSpendingLimit) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:peer:{}", self.storage_key, limit.peer_pubkey());
        let json = limit.to_json()?;

        storage
            .set_item(&key, &json)
            .map_err(|e| JsValue::from_str(&format!("Failed to save: {:?}", e)))?;

        Ok(())
    }

    /// Get a peer spending limit by peer pubkey
    pub async fn get_peer_limit(
        &self,
        peer_pubkey: &str,
    ) -> Result<Option<WasmPeerSpendingLimit>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:peer:{}", self.storage_key, peer_pubkey);

        match storage.get_item(&key) {
            Ok(Some(json)) => {
                let mut limit = WasmPeerSpendingLimit::from_json(&json)?;
                // Check if period needs reset
                use std::time::{SystemTime, UNIX_EPOCH};
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;

                if now - limit.period_start > limit.period_seconds() as i64 {
                    limit.reset();
                    // Save the reset limit
                    self.save_peer_limit(&limit).await?;
                }
                Ok(Some(limit))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(JsValue::from_str(&format!("Failed to get: {:?}", e))),
        }
    }

    /// List all peer spending limits
    pub async fn list_peer_limits(&self) -> Result<Vec<JsValue>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let mut limits = Vec::new();
        let prefix = format!("{}:peer:", self.storage_key);
        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix) {
                    if let Ok(Some(json)) = storage.get_item(&key) {
                        if let Ok(mut limit) = WasmPeerSpendingLimit::from_json(&json) {
                            // Check if period needs reset
                            use std::time::{SystemTime, UNIX_EPOCH};
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64;

                            if now - limit.period_start > limit.period_seconds() as i64 {
                                limit.reset();
                                // Save the reset limit
                                self.save_peer_limit(&limit).await?;
                            }

                            let js_obj = js_sys::Object::new();
                            js_sys::Reflect::set(
                                &js_obj,
                                &"peer_pubkey".into(),
                                &limit.peer_pubkey().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"total_limit".into(),
                                &limit.total_limit().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"current_spent".into(),
                                &limit.current_spent().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"remaining_limit".into(),
                                &limit.remaining_limit().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"period_seconds".into(),
                                &limit.period_seconds().into(),
                            )?;
                            js_sys::Reflect::set(
                                &js_obj,
                                &"period_start".into(),
                                &limit.period_start().into(),
                            )?;

                            limits.push(js_obj.into());
                        }
                    }
                }
            }
        }

        Ok(limits)
    }

    /// Delete a peer spending limit
    pub async fn delete_peer_limit(&self, peer_pubkey: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:peer:{}", self.storage_key, peer_pubkey);
        storage
            .remove_item(&key)
            .map_err(|e| JsValue::from_str(&format!("Failed to delete: {:?}", e)))?;

        Ok(())
    }

    /// Clear all peer spending limits
    pub async fn clear_all(&self) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let prefix = format!("{}:peer:", self.storage_key);
        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        let mut keys_to_remove = Vec::new();
        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix) {
                    keys_to_remove.push(key);
                }
            }
        }

        for key in keys_to_remove {
            storage
                .remove_item(&key)
                .map_err(|e| JsValue::from_str(&format!("Failed to clear: {:?}", e)))?;
        }

        Ok(())
    }
}

// ============================================================
// Phase 4: Proration Calculator WASM Bindings
// ============================================================

/// Proration calculation result
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmProrationResult {
    credit: i64,
    charge: i64,
    net_adjustment: i64,
    days_at_old_rate: u32,
    days_at_new_rate: u32,
    days_remaining: u32,
}

#[wasm_bindgen]
impl WasmProrationResult {
    /// Credit for unused time on old plan (positive = refund owed to subscriber)
    #[wasm_bindgen(getter)]
    pub fn credit(&self) -> i64 {
        self.credit
    }

    /// Charge for time on new plan
    #[wasm_bindgen(getter)]
    pub fn charge(&self) -> i64 {
        self.charge
    }

    /// Net adjustment (positive = subscriber owes more, negative = refund)
    #[wasm_bindgen(getter)]
    pub fn net_adjustment(&self) -> i64 {
        self.net_adjustment
    }

    /// Days at the old rate before change
    #[wasm_bindgen(getter)]
    pub fn days_at_old_rate(&self) -> u32 {
        self.days_at_old_rate
    }

    /// Days at the new rate after change
    #[wasm_bindgen(getter)]
    pub fn days_at_new_rate(&self) -> u32 {
        self.days_at_new_rate
    }

    /// Total days remaining in period from change date
    #[wasm_bindgen(getter)]
    pub fn days_remaining(&self) -> u32 {
        self.days_remaining
    }

    /// Convert to JavaScript object
    pub fn to_object(&self) -> JsValue {
        let obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&obj, &"credit".into(), &self.credit.into());
        let _ = js_sys::Reflect::set(&obj, &"charge".into(), &self.charge.into());
        let _ = js_sys::Reflect::set(&obj, &"net_adjustment".into(), &self.net_adjustment.into());
        let _ = js_sys::Reflect::set(
            &obj,
            &"days_at_old_rate".into(),
            &self.days_at_old_rate.into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"days_at_new_rate".into(),
            &self.days_at_new_rate.into(),
        );
        let _ = js_sys::Reflect::set(&obj, &"days_remaining".into(), &self.days_remaining.into());
        obj.into()
    }
}

/// Calculate proration for a subscription plan change
///
/// # Arguments
///
/// * `current_amount` - Current plan amount in satoshis
/// * `new_amount` - New plan amount in satoshis
/// * `period_start` - Period start timestamp (Unix seconds)
/// * `period_end` - Period end timestamp (Unix seconds)
/// * `change_date` - Date of the plan change (Unix seconds)
///
/// # Returns
///
/// A proration result with credit, charge, and net adjustment
///
/// # Examples
///
/// ```javascript
/// import { calculateProration } from 'paykit-demo-web';
///
/// // Upgrading from 3000 SAT/month to 6000 SAT/month mid-period
/// const result = calculateProration(3000, 6000, 1700000000, 1702600000, 1701300000);
/// console.log(`Credit: ${result.credit} SAT`);
/// console.log(`Charge: ${result.charge} SAT`);
/// console.log(`Net: ${result.net_adjustment} SAT`);
/// ```
#[wasm_bindgen(js_name = calculateProration)]
pub fn calculate_proration(
    current_amount: i64,
    new_amount: i64,
    period_start: i64,
    period_end: i64,
    change_date: i64,
) -> Result<WasmProrationResult, JsValue> {
    // Validate inputs
    if period_end <= period_start {
        return Err(JsValue::from_str("Period end must be after period start"));
    }
    if change_date < period_start || change_date > period_end {
        return Err(JsValue::from_str("Change date must be within the period"));
    }
    if current_amount < 0 || new_amount < 0 {
        return Err(JsValue::from_str("Amounts must be non-negative"));
    }

    // Calculate time proportions
    let total_period_seconds = period_end - period_start;
    let used_seconds = change_date - period_start;
    let remaining_seconds = period_end - change_date;

    // Calculate days (approximate, for display purposes)
    let days_at_old_rate = (used_seconds / 86400) as u32;
    let days_at_new_rate = (remaining_seconds / 86400) as u32;
    let days_remaining = days_at_new_rate;

    // Calculate prorated amounts
    // Credit: unused portion of old plan
    let credit = if total_period_seconds > 0 {
        (current_amount * remaining_seconds) / total_period_seconds
    } else {
        0
    };

    // Charge: remaining portion at new rate
    let charge = if total_period_seconds > 0 {
        (new_amount * remaining_seconds) / total_period_seconds
    } else {
        0
    };

    // Net adjustment: charge - credit
    // Positive = subscriber owes more (upgrade)
    // Negative = subscriber gets refund (downgrade)
    let net_adjustment = charge - credit;

    Ok(WasmProrationResult {
        credit,
        charge,
        net_adjustment,
        days_at_old_rate,
        days_at_new_rate,
        days_remaining,
    })
}

/// Calculate proration for upgrading a subscription
///
/// Convenience function that returns just the amount owed for an upgrade.
///
/// # Arguments
///
/// * `current_amount` - Current plan amount in satoshis
/// * `new_amount` - New plan amount in satoshis (must be higher)
/// * `period_start` - Period start timestamp (Unix seconds)
/// * `period_end` - Period end timestamp (Unix seconds)
/// * `change_date` - Date of the upgrade (Unix seconds)
///
/// # Returns
///
/// The additional amount owed for the upgrade
#[wasm_bindgen(js_name = calculateUpgradeAmount)]
pub fn calculate_upgrade_amount(
    current_amount: i64,
    new_amount: i64,
    period_start: i64,
    period_end: i64,
    change_date: i64,
) -> Result<i64, JsValue> {
    if new_amount <= current_amount {
        return Err(JsValue::from_str(
            "New amount must be higher than current for upgrade",
        ));
    }

    let result = calculate_proration(
        current_amount,
        new_amount,
        period_start,
        period_end,
        change_date,
    )?;
    Ok(result.net_adjustment.max(0))
}

/// Calculate proration for downgrading a subscription
///
/// Convenience function that returns the refund amount for a downgrade.
///
/// # Arguments
///
/// * `current_amount` - Current plan amount in satoshis
/// * `new_amount` - New plan amount in satoshis (must be lower)
/// * `period_start` - Period start timestamp (Unix seconds)
/// * `period_end` - Period end timestamp (Unix seconds)
/// * `change_date` - Date of the downgrade (Unix seconds)
///
/// # Returns
///
/// The refund amount for the downgrade (as positive number)
#[wasm_bindgen(js_name = calculateDowngradeRefund)]
pub fn calculate_downgrade_refund(
    current_amount: i64,
    new_amount: i64,
    period_start: i64,
    period_end: i64,
    change_date: i64,
) -> Result<i64, JsValue> {
    if new_amount >= current_amount {
        return Err(JsValue::from_str(
            "New amount must be lower than current for downgrade",
        ));
    }

    let result = calculate_proration(
        current_amount,
        new_amount,
        period_start,
        period_end,
        change_date,
    )?;
    Ok((-result.net_adjustment).max(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_proration_upgrade() {
        // Upgrading from 3000 to 6000 mid-period
        let result = calculate_proration(3000, 6000, 0, 30 * 86400, 15 * 86400).unwrap();
        // Should owe extra (roughly half of 3000 difference)
        assert!(result.net_adjustment > 0);
        assert_eq!(result.days_at_old_rate, 15);
        assert_eq!(result.days_at_new_rate, 15);
    }

    #[wasm_bindgen_test]
    fn test_proration_downgrade() {
        // Downgrading from 6000 to 3000 mid-period
        let result = calculate_proration(6000, 3000, 0, 30 * 86400, 15 * 86400).unwrap();
        // Should get refund (roughly half of 3000 difference)
        assert!(result.net_adjustment < 0);
    }

    #[wasm_bindgen_test]
    fn test_proration_no_change() {
        // Same amount = no adjustment
        let result = calculate_proration(5000, 5000, 0, 30 * 86400, 15 * 86400).unwrap();
        assert_eq!(result.net_adjustment, 0);
    }

    #[wasm_bindgen_test]
    fn test_auto_pay_rule_creation() {
        let rule = WasmAutoPayRule::new("sub_123", "test_peer", 1000, 3600, false).unwrap();
        assert_eq!(rule.max_amount(), 1000);
        assert_eq!(rule.period_seconds(), 3600);
        assert!(rule.enabled());
        assert_eq!(rule.subscription_id(), "sub_123");
        assert!(!rule.require_confirmation());
    }

    #[wasm_bindgen_test]
    fn test_spending_limit_check() {
        let limit = WasmPeerSpendingLimit::new("test_peer", 1000, 3600).unwrap();
        assert!(limit.can_spend(500));
        assert!(limit.can_spend(1000));
        assert!(!limit.can_spend(1001));
    }

    #[wasm_bindgen_test]
    fn test_spending_limit_record() {
        let mut limit = WasmPeerSpendingLimit::new("test_peer", 1000, 3600).unwrap();
        assert!(limit.record_payment(300).is_ok());
        assert_eq!(limit.current_spent(), 300);
        assert_eq!(limit.remaining_limit(), 700);
    }
}
