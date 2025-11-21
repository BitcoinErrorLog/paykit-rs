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
    peer_pubkey: String,
    max_amount: i64,
    period_seconds: u64,
    enabled: bool,
}

#[wasm_bindgen]
impl WasmAutoPayRule {
    /// Create a new auto-pay rule
    #[wasm_bindgen(constructor)]
    pub fn new(
        peer_pubkey: &str,
        max_amount: i64,
        period_seconds: u64,
    ) -> Result<WasmAutoPayRule, JsValue> {
        if max_amount <= 0 {
            return Err(JsValue::from_str("Max amount must be positive"));
        }
        if period_seconds == 0 {
            return Err(JsValue::from_str("Period must be non-zero"));
        }

        Ok(WasmAutoPayRule {
            id: format!("autopay_{}", uuid::Uuid::new_v4()),
            peer_pubkey: peer_pubkey.to_string(),
            max_amount,
            period_seconds,
            enabled: true,
        })
    }

    /// Get the rule ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_auto_pay_rule_creation() {
        let rule = WasmAutoPayRule::new("test_peer", 1000, 3600).unwrap();
        assert_eq!(rule.max_amount(), 1000);
        assert_eq!(rule.period_seconds(), 3600);
        assert!(rule.enabled());
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
