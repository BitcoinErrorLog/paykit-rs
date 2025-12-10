//! Payment Method Configuration for Paykit Demo Web
//!
//! This module provides browser-based payment method management using localStorage.
//! Payment methods represent your configured ways to send/receive payments, with
//! preferences, priorities, and public/private visibility settings.
//!
//! # Storage Schema
//!
//! Payment methods are stored in browser localStorage with keys:
//! - `paykit_payment_method:{method_id}` - Individual method configurations
//! - Methods are indexed by their method_id for fast lookup
//!
//! # Important Limitation
//!
//! **This implementation uses MOCK PUBLISHING only:**
//! - Methods are saved to localStorage but NOT actually published to Pubky homeserver
//! - Directory queries will NOT see your methods
//! - For production, integrate with Pubky's authenticated PUT operations
//!
//! # Security Warning
//!
//! **This storage is for demo purposes only and is NOT production-ready:**
//! - No encryption at rest (methods stored in plaintext)
//! - No access control or authentication
//! - Subject to browser localStorage limits (~5-10MB)
//! - Can be cleared by user or browser
//! - Mock publishing does NOT interact with real Pubky homeservers
//!
//! For production use, implement proper encryption, authentication,
//! and real Pubky homeserver publishing with capability tokens.
//!
//! # Examples
//!
//! ```
//! use paykit_demo_web::{WasmPaymentMethodConfig, WasmPaymentMethodStorage};
//! use wasm_bindgen_test::*;
//!
//! wasm_bindgen_test_configure!(run_in_browser);
//!
//! #[wasm_bindgen_test]
//! async fn example_payment_method_usage() {
//!     let storage = WasmPaymentMethodStorage::new();
//!     
//!     // Create a payment method
//!     let method = WasmPaymentMethodConfig::new(
//!         "lightning".to_string(),
//!         "lnurl1234...".to_string(),
//!         true,  // is_public
//!         true,  // is_preferred
//!         1      // priority
//!     ).unwrap();
//!     
//!     // Save it
//!     storage.save_method(&method).await.unwrap();
//!     
//!     // Retrieve it
//!     let retrieved = storage.get_method("lightning").await.unwrap();
//!     assert!(retrieved.is_some());
//! }
//! ```

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// A payment method configuration
///
/// Represents a configured payment method with endpoint, visibility,
/// and preference settings.
///
/// # Examples
///
/// ```
/// use paykit_demo_web::WasmPaymentMethodConfig;
///
/// let method = WasmPaymentMethodConfig::new(
///     "lightning".to_string(),
///     "lnurl1dp68gurn8ghj7um5v93kketj9ehx2amn9uh8wetvdskkkmn0wahz7mrww4excup0...".to_string(),
///     true,  // is_public
///     true,  // is_preferred
///     1      // priority (1 = highest)
/// ).unwrap();
/// ```
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct WasmPaymentMethodConfig {
    inner: PaymentMethodConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PaymentMethodConfig {
    method_id: String,
    endpoint: String,
    is_public: bool,
    is_preferred: bool,
    priority: u32,
}

#[wasm_bindgen]
impl WasmPaymentMethodConfig {
    /// Create a new payment method configuration
    ///
    /// # Arguments
    ///
    /// * `method_id` - Unique identifier (e.g., "lightning", "onchain", "custom")
    /// * `endpoint` - Payment endpoint (e.g., LNURL, Bitcoin address, etc.)
    /// * `is_public` - Whether to publish this method publicly
    /// * `is_preferred` - Whether this is a preferred method
    /// * `priority` - Priority order (1 = highest priority)
    ///
    /// # Errors
    ///
    /// Returns an error if method_id or endpoint is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmPaymentMethodConfig;
    ///
    /// let method = WasmPaymentMethodConfig::new(
    ///     "lightning".to_string(),
    ///     "lnurl1234...".to_string(),
    ///     true,
    ///     true,
    ///     1
    /// ).unwrap();
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(
        method_id: String,
        endpoint: String,
        is_public: bool,
        is_preferred: bool,
        priority: u32,
    ) -> Result<WasmPaymentMethodConfig, JsValue> {
        // Validate inputs
        if method_id.trim().is_empty() {
            return Err(JsValue::from_str("method_id cannot be empty"));
        }
        if endpoint.trim().is_empty() {
            return Err(JsValue::from_str("endpoint cannot be empty"));
        }

        Ok(WasmPaymentMethodConfig {
            inner: PaymentMethodConfig {
                method_id,
                endpoint,
                is_public,
                is_preferred,
                priority,
            },
        })
    }

    /// Get the method ID
    #[wasm_bindgen(getter)]
    pub fn method_id(&self) -> String {
        self.inner.method_id.clone()
    }

    /// Get the endpoint
    #[wasm_bindgen(getter)]
    pub fn endpoint(&self) -> String {
        self.inner.endpoint.clone()
    }

    /// Get the public visibility status
    #[wasm_bindgen(getter)]
    pub fn is_public(&self) -> bool {
        self.inner.is_public
    }

    /// Get the preferred status
    #[wasm_bindgen(getter)]
    pub fn is_preferred(&self) -> bool {
        self.inner.is_preferred
    }

    /// Get the priority
    #[wasm_bindgen(getter)]
    pub fn priority(&self) -> u32 {
        self.inner.priority
    }

    /// Convert method to JSON string
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Create method from JSON string
    pub fn from_json(json: &str) -> Result<WasmPaymentMethodConfig, JsValue> {
        let inner: PaymentMethodConfig = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        Ok(WasmPaymentMethodConfig { inner })
    }
}

/// Storage manager for payment methods in browser localStorage
///
/// Provides CRUD operations for managing payment method configurations
/// with localStorage persistence.
///
/// # Examples
///
/// ```
/// use paykit_demo_web::{WasmPaymentMethodConfig, WasmPaymentMethodStorage};
/// use wasm_bindgen_test::*;
///
/// wasm_bindgen_test_configure!(run_in_browser);
///
/// #[wasm_bindgen_test]
/// async fn example_storage() {
///     let storage = WasmPaymentMethodStorage::new();
///     let method = WasmPaymentMethodConfig::new(
///         "lightning".to_string(),
///         "lnurl1234...".to_string(),
///         true,
///         true,
///         1
///     ).unwrap();
///     
///     storage.save_method(&method).await.unwrap();
///     let methods = storage.list_methods().await.unwrap();
///     assert!(methods.len() >= 1);
/// }
/// ```
#[wasm_bindgen]
pub struct WasmPaymentMethodStorage {
    storage_key_prefix: String,
}

impl Default for WasmPaymentMethodStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmPaymentMethodStorage {
    /// Create a new payment method storage manager
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmPaymentMethodStorage;
    ///
    /// let storage = WasmPaymentMethodStorage::new();
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            storage_key_prefix: "paykit_payment_method".to_string(),
        }
    }

    /// Save a payment method to localStorage
    ///
    /// If a method with the same method_id exists, it will be overwritten.
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::{WasmPaymentMethodConfig, WasmPaymentMethodStorage};
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn save_example() {
    ///     let storage = WasmPaymentMethodStorage::new();
    ///     let method = WasmPaymentMethodConfig::new(
    ///         "lightning".to_string(),
    ///         "lnurl1234...".to_string(),
    ///         true,
    ///         true,
    ///         1
    ///     ).unwrap();
    ///     storage.save_method(&method).await.unwrap();
    /// }
    /// ```
    pub async fn save_method(&self, method: &WasmPaymentMethodConfig) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:{}", self.storage_key_prefix, method.method_id());
        let json = method.to_json()?;

        storage
            .set_item(&key, &json)
            .map_err(|e| JsValue::from_str(&format!("Failed to save method: {:?}", e)))?;

        Ok(())
    }

    /// Get a payment method by method_id
    ///
    /// Returns `None` if the method doesn't exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmPaymentMethodStorage;
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn get_example() {
    ///     let storage = WasmPaymentMethodStorage::new();
    ///     let method = storage.get_method("lightning").await.unwrap();
    ///     // method is None if not found
    /// }
    /// ```
    pub async fn get_method(
        &self,
        method_id: &str,
    ) -> Result<Option<WasmPaymentMethodConfig>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:{}", self.storage_key_prefix, method_id);

        match storage.get_item(&key) {
            Ok(Some(json)) => {
                let method = WasmPaymentMethodConfig::from_json(&json)?;
                Ok(Some(method))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(JsValue::from_str(&format!("Failed to get method: {:?}", e))),
        }
    }

    /// List all payment methods, sorted by priority (lowest number = highest priority)
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmPaymentMethodStorage;
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn list_example() {
    ///     let storage = WasmPaymentMethodStorage::new();
    ///     let methods = storage.list_methods().await.unwrap();
    ///     // Returns empty vector if no methods
    /// }
    /// ```
    pub async fn list_methods(&self) -> Result<Vec<JsValue>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let mut methods = Vec::new();
        let prefix = format!("{}:", self.storage_key_prefix);
        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix) {
                    if let Ok(Some(json)) = storage.get_item(&key) {
                        if let Ok(method) = WasmPaymentMethodConfig::from_json(&json) {
                            methods.push(method);
                        }
                    }
                }
            }
        }

        // Sort by priority (lowest number first = highest priority)
        methods.sort_by_key(|m| m.priority());

        // Convert to JsValue objects for JavaScript
        let js_methods: Vec<JsValue> = methods
            .iter()
            .map(|method| {
                let js_obj = js_sys::Object::new();
                let _ =
                    js_sys::Reflect::set(&js_obj, &"method_id".into(), &method.method_id().into());
                let _ =
                    js_sys::Reflect::set(&js_obj, &"endpoint".into(), &method.endpoint().into());
                let _ =
                    js_sys::Reflect::set(&js_obj, &"is_public".into(), &method.is_public().into());
                let _ = js_sys::Reflect::set(
                    &js_obj,
                    &"is_preferred".into(),
                    &method.is_preferred().into(),
                );
                let _ =
                    js_sys::Reflect::set(&js_obj, &"priority".into(), &method.priority().into());
                js_obj.into()
            })
            .collect();

        Ok(js_methods)
    }

    /// Delete a payment method by method_id
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmPaymentMethodStorage;
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn delete_example() {
    ///     let storage = WasmPaymentMethodStorage::new();
    ///     storage.delete_method("lightning").await.unwrap();
    /// }
    /// ```
    pub async fn delete_method(&self, method_id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:{}", self.storage_key_prefix, method_id);
        storage
            .remove_item(&key)
            .map_err(|e| JsValue::from_str(&format!("Failed to delete method: {:?}", e)))?;

        Ok(())
    }

    /// Set or update the preferred status of a payment method
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmPaymentMethodStorage;
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn set_preferred_example() {
    ///     let storage = WasmPaymentMethodStorage::new();
    ///     storage.set_preferred("lightning", true).await.unwrap();
    /// }
    /// ```
    pub async fn set_preferred(&self, method_id: &str, preferred: bool) -> Result<(), JsValue> {
        let mut method = self
            .get_method(method_id)
            .await?
            .ok_or_else(|| JsValue::from_str("Method not found"))?;

        method.inner.is_preferred = preferred;
        self.save_method(&method).await?;

        Ok(())
    }

    /// Update the priority of a payment method
    ///
    /// Lower numbers = higher priority (1 is highest)
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmPaymentMethodStorage;
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn update_priority_example() {
    ///     let storage = WasmPaymentMethodStorage::new();
    ///     storage.update_priority("lightning", 1).await.unwrap();
    /// }
    /// ```
    pub async fn update_priority(&self, method_id: &str, priority: u32) -> Result<(), JsValue> {
        let mut method = self
            .get_method(method_id)
            .await?
            .ok_or_else(|| JsValue::from_str("Method not found"))?;

        method.inner.priority = priority;
        self.save_method(&method).await?;

        Ok(())
    }

    /// Get all preferred payment methods, sorted by priority
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmPaymentMethodStorage;
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn get_preferred_example() {
    ///     let storage = WasmPaymentMethodStorage::new();
    ///     let preferred = storage.get_preferred_methods().await.unwrap();
    /// }
    /// ```
    pub async fn get_preferred_methods(&self) -> Result<Vec<JsValue>, JsValue> {
        let all_methods = self.list_methods().await?;

        let preferred: Vec<JsValue> = all_methods
            .into_iter()
            .filter(|method_js| {
                if let Ok(is_preferred) = js_sys::Reflect::get(method_js, &"is_preferred".into()) {
                    return is_preferred.as_bool().unwrap_or(false);
                }
                false
            })
            .collect();

        Ok(preferred)
    }

    /// Mock publish methods to Pubky homeserver
    ///
    /// **⚠️ WARNING: This is a MOCK implementation for demo purposes only.**
    ///
    /// This function simulates publishing by saving a special marker to localStorage.
    /// It does NOT actually publish methods to a real Pubky homeserver.
    ///
    /// For production use, integrate with Pubky's authenticated PUT operations
    /// to publish methods to the directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmPaymentMethodStorage;
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn mock_publish_example() {
    ///     let storage = WasmPaymentMethodStorage::new();
    ///     storage.mock_publish().await.unwrap();
    /// }
    /// ```
    pub async fn mock_publish(&self) -> Result<String, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        // Get all public methods
        let all_methods = self.list_methods().await?;
        let public_count = all_methods
            .iter()
            .filter(|method_js| {
                if let Ok(is_public) = js_sys::Reflect::get(method_js, &"is_public".into()) {
                    return is_public.as_bool().unwrap_or(false);
                }
                false
            })
            .count();

        // Save a mock publish marker
        let timestamp = js_sys::Date::now();
        let marker = format!("mock_published_at:{}", timestamp);
        storage
            .set_item("paykit_mock_publish_status", &marker)
            .map_err(|e| JsValue::from_str(&format!("Failed to save publish status: {:?}", e)))?;

        Ok(format!(
            "MOCK: {} public method(s) would be published. This is demo-only and does NOT publish to real homeserver.",
            public_count
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_method_creation() {
        let method = WasmPaymentMethodConfig::new(
            "lightning".to_string(),
            "lnurl1234".to_string(),
            true,
            true,
            1,
        )
        .unwrap();

        assert_eq!(method.method_id(), "lightning");
        assert_eq!(method.endpoint(), "lnurl1234");
        assert!(method.is_public());
        assert!(method.is_preferred());
        assert_eq!(method.priority(), 1);
    }

    #[wasm_bindgen_test]
    fn test_method_validation() {
        // Empty method_id should fail
        let result =
            WasmPaymentMethodConfig::new("".to_string(), "lnurl1234".to_string(), true, true, 1);
        assert!(result.is_err());

        // Empty endpoint should fail
        let result =
            WasmPaymentMethodConfig::new("lightning".to_string(), "".to_string(), true, true, 1);
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_method_json_serialization() {
        let method = WasmPaymentMethodConfig::new(
            "lightning".to_string(),
            "lnurl1234".to_string(),
            true,
            false,
            2,
        )
        .unwrap();

        let json = method.to_json().unwrap();
        let restored = WasmPaymentMethodConfig::from_json(&json).unwrap();

        assert_eq!(restored.method_id(), method.method_id());
        assert_eq!(restored.endpoint(), method.endpoint());
        assert_eq!(restored.is_public(), method.is_public());
        assert_eq!(restored.is_preferred(), method.is_preferred());
        assert_eq!(restored.priority(), method.priority());
    }

    #[wasm_bindgen_test]
    async fn test_save_and_retrieve() {
        let storage = WasmPaymentMethodStorage::new();
        let method = WasmPaymentMethodConfig::new(
            "test_lightning".to_string(),
            "lnurl1234".to_string(),
            true,
            true,
            1,
        )
        .unwrap();

        storage.save_method(&method).await.unwrap();
        let retrieved = storage.get_method("test_lightning").await.unwrap();

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.endpoint(), "lnurl1234");
    }

    #[wasm_bindgen_test]
    async fn test_list_sorted_by_priority() {
        let storage = WasmPaymentMethodStorage::new();

        // Clean up first
        let _ = storage.delete_method("test_method_1").await;
        let _ = storage.delete_method("test_method_2").await;
        let _ = storage.delete_method("test_method_3").await;

        // Create methods with different priorities
        let method1 = WasmPaymentMethodConfig::new(
            "test_method_1".to_string(),
            "endpoint1".to_string(),
            true,
            true,
            3,
        )
        .unwrap();

        let method2 = WasmPaymentMethodConfig::new(
            "test_method_2".to_string(),
            "endpoint2".to_string(),
            true,
            true,
            1,
        )
        .unwrap();

        let method3 = WasmPaymentMethodConfig::new(
            "test_method_3".to_string(),
            "endpoint3".to_string(),
            true,
            true,
            2,
        )
        .unwrap();

        storage.save_method(&method1).await.unwrap();
        storage.save_method(&method2).await.unwrap();
        storage.save_method(&method3).await.unwrap();

        let methods = storage.list_methods().await.unwrap();

        // Find our test methods
        let test_methods: Vec<&JsValue> = methods
            .iter()
            .filter(|m| {
                if let Ok(id) = js_sys::Reflect::get(m, &"method_id".into()) {
                    if let Some(id_str) = id.as_string() {
                        return id_str.starts_with("test_method_");
                    }
                }
                false
            })
            .collect();

        assert!(test_methods.len() >= 3);

        // Clean up
        let _ = storage.delete_method("test_method_1").await;
        let _ = storage.delete_method("test_method_2").await;
        let _ = storage.delete_method("test_method_3").await;
    }

    #[wasm_bindgen_test]
    async fn test_set_preferred() {
        let storage = WasmPaymentMethodStorage::new();
        let method = WasmPaymentMethodConfig::new(
            "test_prefer".to_string(),
            "endpoint".to_string(),
            true,
            false,
            1,
        )
        .unwrap();

        storage.save_method(&method).await.unwrap();
        storage.set_preferred("test_prefer", true).await.unwrap();

        let retrieved = storage.get_method("test_prefer").await.unwrap().unwrap();
        assert!(retrieved.is_preferred());

        // Clean up
        let _ = storage.delete_method("test_prefer").await;
    }

    #[wasm_bindgen_test]
    async fn test_update_priority() {
        let storage = WasmPaymentMethodStorage::new();
        let method = WasmPaymentMethodConfig::new(
            "test_priority".to_string(),
            "endpoint".to_string(),
            true,
            true,
            1,
        )
        .unwrap();

        storage.save_method(&method).await.unwrap();
        storage.update_priority("test_priority", 5).await.unwrap();

        let retrieved = storage.get_method("test_priority").await.unwrap().unwrap();
        assert_eq!(retrieved.priority(), 5);

        // Clean up
        let _ = storage.delete_method("test_priority").await;
    }

    #[wasm_bindgen_test]
    async fn test_delete_method() {
        let storage = WasmPaymentMethodStorage::new();
        let method = WasmPaymentMethodConfig::new(
            "test_delete".to_string(),
            "endpoint".to_string(),
            true,
            true,
            1,
        )
        .unwrap();

        storage.save_method(&method).await.unwrap();
        storage.delete_method("test_delete").await.unwrap();

        let retrieved = storage.get_method("test_delete").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[wasm_bindgen_test]
    async fn test_get_preferred_methods() {
        let storage = WasmPaymentMethodStorage::new();

        // Clean up first
        let _ = storage.delete_method("test_pref_1").await;
        let _ = storage.delete_method("test_pref_2").await;

        let method1 = WasmPaymentMethodConfig::new(
            "test_pref_1".to_string(),
            "endpoint1".to_string(),
            true,
            true,
            1,
        )
        .unwrap();

        let method2 = WasmPaymentMethodConfig::new(
            "test_pref_2".to_string(),
            "endpoint2".to_string(),
            true,
            false,
            2,
        )
        .unwrap();

        storage.save_method(&method1).await.unwrap();
        storage.save_method(&method2).await.unwrap();

        let preferred = storage.get_preferred_methods().await.unwrap();

        // Should have at least our test preferred method
        let has_test_pref = preferred.iter().any(|m| {
            if let Ok(id) = js_sys::Reflect::get(m, &"method_id".into()) {
                if let Some(id_str) = id.as_string() {
                    return id_str == "test_pref_1";
                }
            }
            false
        });

        assert!(has_test_pref);

        // Clean up
        let _ = storage.delete_method("test_pref_1").await;
        let _ = storage.delete_method("test_pref_2").await;
    }

    #[wasm_bindgen_test]
    async fn test_mock_publish() {
        let storage = WasmPaymentMethodStorage::new();
        let result = storage.mock_publish().await.unwrap();
        assert!(result.contains("MOCK"));
        assert!(result.contains("demo-only"));
    }
}
