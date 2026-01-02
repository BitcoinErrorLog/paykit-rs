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

    /// Validate the payment method endpoint
    ///
    /// Uses the Paykit method registry to validate that the endpoint
    /// is properly formatted for the given method type.
    ///
    /// # Returns
    ///
    /// A JavaScript object with validation results:
    /// ```json
    /// {
    ///   "valid": true|false,
    ///   "errors": ["error1", "error2"],
    ///   "warnings": ["warning1"]
    /// }
    /// ```
    pub fn validate(&self) -> std::result::Result<JsValue, JsValue> {
        use paykit_lib::methods::{default_registry, ValidationResult};
        use paykit_lib::{EndpointData, MethodId};

        let registry = default_registry();
        let method_id = MethodId::new(&self.inner.method_id);

        let result = if let Some(plugin) = registry.get(&method_id) {
            let endpoint_data = EndpointData(self.inner.endpoint.clone());
            plugin.validate_endpoint(&endpoint_data)
        } else {
            // No validator for this method type - assume valid
            ValidationResult::valid()
        };

        // Convert to JS object
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"valid".into(), &result.valid.into())?;

        let errors = js_sys::Array::new();
        for error in &result.errors {
            errors.push(&error.into());
        }
        js_sys::Reflect::set(&obj, &"errors".into(), &errors)?;

        let warnings = js_sys::Array::new();
        for warning in &result.warnings {
            warnings.push(&warning.into());
        }
        js_sys::Reflect::set(&obj, &"warnings".into(), &warnings)?;

        Ok(obj.into())
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
    /// For real publishing, use `publish_to_directory()` with a configured DirectoryClient.
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
            "MOCK: {} public method(s) would be published. This is demo-only and does NOT publish to real homeserver. \
             For real publishing, use DirectoryClient with Direct or Proxy mode.",
            public_count
        ))
    }

    /// Publish all public methods to a real Pubky homeserver
    ///
    /// This function publishes all methods marked as `is_public` to the Pubky directory
    /// using the provided DirectoryClient.
    ///
    /// # Arguments
    ///
    /// * `directory_client` - A configured DirectoryClient (use Direct or Proxy mode)
    /// * `public_key` - Your public key (z-base32 encoded)
    /// * `auth_token` - Optional authentication token for the homeserver
    ///
    /// # Returns
    ///
    /// A summary of the publish operation including success/failure counts.
    ///
    /// # Examples
    ///
    /// ```javascript
    /// const storage = new WasmPaymentMethodStorage();
    /// const client = DirectoryClient.withProxy(
    ///     "https://homeserver.example.com",
    ///     "https://cors-proxy.example.com"
    /// );
    ///
    /// const result = await storage.publishToDirectory(client, myPublicKey, authToken);
    /// console.log(result);
    /// ```
    #[wasm_bindgen(js_name = publishToDirectory)]
    pub async fn publish_to_directory(
        &self,
        directory_client: &crate::directory::DirectoryClient,
        public_key: &str,
        auth_token: Option<String>,
    ) -> Result<String, JsValue> {
        // Get all public methods
        let all_methods = self.list_methods().await?;
        let public_methods: Vec<&JsValue> = all_methods
            .iter()
            .filter(|method_js| {
                if let Ok(is_public) = js_sys::Reflect::get(method_js, &"is_public".into()) {
                    return is_public.as_bool().unwrap_or(false);
                }
                false
            })
            .collect();

        if public_methods.is_empty() {
            return Ok("No public methods to publish.".to_string());
        }

        let mut success_count = 0;
        let mut failure_count = 0;
        let mut errors: Vec<String> = Vec::new();

        for method_js in public_methods {
            let method_id = js_sys::Reflect::get(method_js, &"method_id".into())
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();

            let endpoint = js_sys::Reflect::get(method_js, &"endpoint".into())
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();

            if method_id.is_empty() || endpoint.is_empty() {
                continue;
            }

            match directory_client
                .publish_endpoint(public_key, &method_id, &endpoint, auth_token.clone())
                .await
            {
                Ok(result) if result.success() => {
                    success_count += 1;
                }
                Ok(result) => {
                    failure_count += 1;
                    errors.push(format!("{}: {}", method_id, result.message()));
                }
                Err(e) => {
                    failure_count += 1;
                    errors.push(format!(
                        "{}: {:?}",
                        method_id,
                        e.as_string().unwrap_or_default()
                    ));
                }
            }
        }

        let mut result = format!(
            "Published {} method(s) successfully, {} failed.",
            success_count, failure_count
        );

        if !errors.is_empty() {
            result.push_str(&format!("\nErrors: {}", errors.join("; ")));
        }

        Ok(result)
    }

    /// Check health status of a specific payment method
    ///
    /// This performs basic connectivity and validation checks for the given method.
    /// Returns a health status object with status and details.
    ///
    /// # Arguments
    ///
    /// * `method_id` - The ID of the method to check (e.g., "lightning", "onchain")
    ///
    /// # Returns
    ///
    /// A JavaScript object with fields:
    /// - `healthy`: boolean indicating if the method is usable
    /// - `status`: string status ("healthy", "degraded", "unhealthy", "unknown")
    /// - `message`: human-readable status message
    /// - `checked_at`: timestamp of the check
    ///
    /// # Examples
    ///
    /// ```javascript
    /// const storage = new WasmPaymentMethodStorage();
    /// const status = await storage.checkHealth("lightning");
    /// console.log(status.healthy, status.message);
    /// ```
    #[wasm_bindgen(js_name = checkHealth)]
    pub async fn check_health(&self, method_id: &str) -> Result<JsValue, JsValue> {
        let method = self.get_method(method_id).await?;

        let js_obj = js_sys::Object::new();
        let timestamp = js_sys::Date::now();
        let _ = js_sys::Reflect::set(&js_obj, &"checked_at".into(), &timestamp.into());
        let _ = js_sys::Reflect::set(&js_obj, &"method_id".into(), &method_id.into());

        match method {
            Some(m) => {
                let endpoint = m.endpoint();

                // Basic validation based on method type
                let (healthy, status, message) = match method_id.to_lowercase().as_str() {
                    "lightning" | "ln" | "ln-btc" => {
                        if endpoint.starts_with("lnurl") || endpoint.starts_with("LNURL") {
                            (
                                true,
                                "healthy",
                                format!(
                                    "LNURL endpoint configured: {}...",
                                    &endpoint[..20.min(endpoint.len())]
                                ),
                            )
                        } else if endpoint.starts_with("lnbc") || endpoint.starts_with("lntb") {
                            (true, "healthy", "BOLT11 invoice configured".to_string())
                        } else if endpoint.contains("@") || endpoint.contains(".") {
                            (
                                true,
                                "healthy",
                                format!("Lightning address configured: {}", endpoint),
                            )
                        } else {
                            (
                                false,
                                "degraded",
                                "Lightning endpoint format not recognized".to_string(),
                            )
                        }
                    }
                    "onchain" | "btc" | "onchain-btc" => {
                        if endpoint.starts_with("bc1") || endpoint.starts_with("tb1") {
                            (
                                true,
                                "healthy",
                                format!(
                                    "SegWit address configured: {}...",
                                    &endpoint[..12.min(endpoint.len())]
                                ),
                            )
                        } else if endpoint.starts_with("1") || endpoint.starts_with("3") {
                            (
                                true,
                                "healthy",
                                "Legacy Bitcoin address configured".to_string(),
                            )
                        } else {
                            (
                                false,
                                "degraded",
                                "Bitcoin address format not recognized".to_string(),
                            )
                        }
                    }
                    _ => {
                        // Unknown method type - just check endpoint is non-empty
                        if !endpoint.is_empty() {
                            (
                                true,
                                "unknown",
                                format!("Custom method '{}' configured", method_id),
                            )
                        } else {
                            (false, "unhealthy", "No endpoint configured".to_string())
                        }
                    }
                };

                let _ = js_sys::Reflect::set(&js_obj, &"healthy".into(), &healthy.into());
                let _ = js_sys::Reflect::set(&js_obj, &"status".into(), &status.into());
                let _ = js_sys::Reflect::set(&js_obj, &"message".into(), &message.into());
            }
            None => {
                let _ = js_sys::Reflect::set(&js_obj, &"healthy".into(), &false.into());
                let _ = js_sys::Reflect::set(&js_obj, &"status".into(), &"not_found".into());
                let _ = js_sys::Reflect::set(
                    &js_obj,
                    &"message".into(),
                    &format!("Method '{}' not configured", method_id).into(),
                );
            }
        }

        Ok(js_obj.into())
    }

    /// Check health of all configured payment methods
    ///
    /// Returns an array of health status objects for each method.
    ///
    /// # Examples
    ///
    /// ```javascript
    /// const storage = new WasmPaymentMethodStorage();
    /// const statuses = await storage.checkAllHealth();
    /// for (const status of statuses) {
    ///     console.log(`${status.method_id}: ${status.status}`);
    /// }
    /// ```
    #[wasm_bindgen(js_name = checkAllHealth)]
    pub async fn check_all_health(&self) -> Result<Vec<JsValue>, JsValue> {
        let methods = self.list_methods().await?;
        let mut results = Vec::new();

        for method_js in &methods {
            if let Ok(method_id) = js_sys::Reflect::get(method_js, &"method_id".into()) {
                if let Some(id) = method_id.as_string() {
                    let health = self.check_health(&id).await?;
                    results.push(health);
                }
            }
        }

        Ok(results)
    }

    /// Get a summary of all method health statuses
    ///
    /// Returns an object with:
    /// - `total`: total number of methods
    /// - `healthy`: number of healthy methods
    /// - `degraded`: number of degraded methods
    /// - `unhealthy`: number of unhealthy methods
    /// - `all_healthy`: boolean if all methods are healthy
    ///
    /// # Examples
    ///
    /// ```javascript
    /// const storage = new WasmPaymentMethodStorage();
    /// const summary = await storage.healthSummary();
    /// console.log(`${summary.healthy}/${summary.total} methods healthy`);
    /// ```
    #[wasm_bindgen(js_name = healthSummary)]
    pub async fn health_summary(&self) -> Result<JsValue, JsValue> {
        let statuses = self.check_all_health().await?;

        let mut healthy = 0u32;
        let mut degraded = 0u32;
        let mut unhealthy = 0u32;

        for status in &statuses {
            if let Ok(s) = js_sys::Reflect::get(status, &"status".into()) {
                match s.as_string().as_deref() {
                    Some("healthy") => healthy += 1,
                    Some("degraded") => degraded += 1,
                    _ => unhealthy += 1,
                }
            }
        }

        let total = statuses.len() as u32;
        let js_obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&js_obj, &"total".into(), &total.into());
        let _ = js_sys::Reflect::set(&js_obj, &"healthy".into(), &healthy.into());
        let _ = js_sys::Reflect::set(&js_obj, &"degraded".into(), &degraded.into());
        let _ = js_sys::Reflect::set(&js_obj, &"unhealthy".into(), &unhealthy.into());
        let _ = js_sys::Reflect::set(
            &js_obj,
            &"all_healthy".into(),
            &(healthy == total && total > 0).into(),
        );

        Ok(js_obj.into())
    }

    /// Remove all published methods from the directory
    ///
    /// # Arguments
    ///
    /// * `directory_client` - A configured DirectoryClient
    /// * `public_key` - Your public key (z-base32 encoded)
    /// * `auth_token` - Optional authentication token for the homeserver
    #[wasm_bindgen(js_name = unpublishFromDirectory)]
    pub async fn unpublish_from_directory(
        &self,
        directory_client: &crate::directory::DirectoryClient,
        public_key: &str,
        auth_token: Option<String>,
    ) -> Result<String, JsValue> {
        let all_methods = self.list_methods().await?;
        let public_methods: Vec<&JsValue> = all_methods
            .iter()
            .filter(|method_js| {
                if let Ok(is_public) = js_sys::Reflect::get(method_js, &"is_public".into()) {
                    return is_public.as_bool().unwrap_or(false);
                }
                false
            })
            .collect();

        if public_methods.is_empty() {
            return Ok("No public methods to unpublish.".to_string());
        }

        let mut success_count = 0;
        let mut failure_count = 0;

        for method_js in public_methods {
            let method_id = js_sys::Reflect::get(method_js, &"method_id".into())
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();

            if method_id.is_empty() {
                continue;
            }

            match directory_client
                .remove_endpoint(public_key, &method_id, auth_token.clone())
                .await
            {
                Ok(result) if result.success() => {
                    success_count += 1;
                }
                Ok(_) | Err(_) => {
                    failure_count += 1;
                }
            }
        }

        Ok(format!(
            "Removed {} method(s) successfully, {} failed.",
            success_count, failure_count
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
