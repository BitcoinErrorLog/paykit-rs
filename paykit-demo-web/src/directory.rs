//! Directory operations for WASM
//!
//! This module provides directory operations for the web demo, including
//! both read (query) and write (publish) operations.
//!
//! # Publishing Modes
//!
//! Due to browser CORS restrictions, publishing to Pubky homeservers from
//! the browser may not work directly. This module supports multiple modes:
//!
//! 1. **Mock Mode** (default): Saves to localStorage only, does not publish to homeserver
//! 2. **Direct Mode**: Attempts direct HTTP PUT to homeserver (requires CORS headers)
//! 3. **Proxy Mode**: Routes requests through a CORS proxy
//!
//! # Example
//!
//! ```javascript
//! // Create client with direct homeserver access
//! const client = new DirectoryClient("https://homeserver.example.com");
//!
//! // Or with a CORS proxy
//! const proxyClient = DirectoryClient.withProxy(
//!     "https://homeserver.example.com",
//!     "https://cors-proxy.example.com"
//! );
//!
//! // Query methods (read-only, usually works without CORS issues)
//! const methods = await client.queryMethods(publicKey);
//!
//! // Publish a method endpoint
//! const result = await client.publishEndpoint(methodId, endpoint, authToken);
//! ```

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, RequestInit, RequestMode, Response};

use crate::utils;

/// Paykit directory path prefix
const PAYKIT_PATH_PREFIX: &str = "/pub/paykit.app/v0/";

/// Publishing mode for directory operations
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PublishMode {
    /// Mock mode - saves to localStorage only (for offline development)
    Mock,
    /// Direct mode - attempts direct HTTP PUT to homeserver (default)
    #[default]
    Direct,
    /// Proxy mode - routes through a CORS proxy
    Proxy,
}

/// Result of a publish operation
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct PublishResult {
    success: bool,
    message: String,
    mode: PublishMode,
}

#[wasm_bindgen]
impl PublishResult {
    /// Whether the publish operation succeeded
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.success
    }

    /// Message describing the result
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// The publishing mode that was used
    #[wasm_bindgen(getter)]
    pub fn mode(&self) -> PublishMode {
        self.mode
    }
}

/// Directory client for querying and publishing payment methods
///
/// Supports multiple modes for publishing to work around browser CORS restrictions.
#[wasm_bindgen]
pub struct DirectoryClient {
    homeserver: String,
    proxy_url: Option<String>,
    mode: PublishMode,
}

#[wasm_bindgen]
impl DirectoryClient {
    /// Create a new directory client with default settings (Direct mode)
    ///
    /// Direct mode attempts to publish directly to the homeserver.
    /// If CORS issues occur, use `withProxy()` or `withMockMode()` instead.
    ///
    /// # Arguments
    ///
    /// * `homeserver` - The Pubky homeserver URL (e.g., "https://demo.httprelay.io")
    #[wasm_bindgen(constructor)]
    pub fn new(homeserver: String) -> DirectoryClient {
        DirectoryClient {
            homeserver,
            proxy_url: None,
            mode: PublishMode::Direct,
        }
    }

    /// Create a directory client with mock mode (for offline development)
    ///
    /// Mock mode saves to localStorage only, without making network requests.
    ///
    /// # Arguments
    ///
    /// * `homeserver` - The Pubky homeserver URL (used for query operations)
    #[wasm_bindgen(js_name = withMockMode)]
    pub fn with_mock_mode(homeserver: String) -> DirectoryClient {
        DirectoryClient {
            homeserver,
            proxy_url: None,
            mode: PublishMode::Mock,
        }
    }

    /// Create a directory client with a CORS proxy
    ///
    /// # Arguments
    ///
    /// * `homeserver` - The Pubky homeserver URL
    /// * `proxy_url` - The CORS proxy URL that will forward requests
    #[wasm_bindgen(js_name = withProxy)]
    pub fn with_proxy(homeserver: String, proxy_url: String) -> DirectoryClient {
        DirectoryClient {
            homeserver,
            proxy_url: Some(proxy_url),
            mode: PublishMode::Proxy,
        }
    }

    /// Create a directory client for direct homeserver access
    ///
    /// Note: Direct mode requires the homeserver to have CORS headers configured.
    ///
    /// # Arguments
    ///
    /// * `homeserver` - The Pubky homeserver URL with CORS enabled
    #[wasm_bindgen(js_name = withDirectAccess)]
    pub fn with_direct_access(homeserver: String) -> DirectoryClient {
        DirectoryClient {
            homeserver,
            proxy_url: None,
            mode: PublishMode::Direct,
        }
    }

    /// Get the current publishing mode
    #[wasm_bindgen(getter, js_name = publishMode)]
    pub fn publish_mode(&self) -> PublishMode {
        self.mode
    }

    /// Set the publishing mode
    #[wasm_bindgen(setter, js_name = publishMode)]
    pub fn set_publish_mode(&mut self, mode: PublishMode) {
        self.mode = mode;
    }

    /// Get the homeserver URL
    #[wasm_bindgen(getter)]
    pub fn homeserver(&self) -> String {
        self.homeserver.clone()
    }

    /// Get the proxy URL if configured
    #[wasm_bindgen(getter, js_name = proxyUrl)]
    pub fn proxy_url(&self) -> Option<String> {
        self.proxy_url.clone()
    }

    /// Query payment methods for a public key
    ///
    /// This is a read-only operation that usually works without CORS issues.
    #[wasm_bindgen(js_name = queryMethods)]
    pub async fn query_methods(&self, public_key: &str) -> Result<JsValue, JsValue> {
        utils::log(&format!("Querying methods for: {}", public_key));

        // Construct the URL for the public directory
        let url = format!(
            "{}{}{}/",
            self.homeserver,
            PAYKIT_PATH_PREFIX.replace("/pub/", &format!("/pub/{}/", public_key)),
            ""
        );

        // Use proxy if configured for queries too
        let fetch_url = if let Some(ref proxy) = self.proxy_url {
            format!("{}/{}", proxy, url)
        } else {
            format!("{}/pub/{}/paykit.app/v0/", self.homeserver, public_key)
        };

        // Make the fetch call
        let window = web_sys::window().ok_or_else(|| utils::js_error("No window object"))?;
        let resp_value = JsFuture::from(window.fetch_with_str(&fetch_url))
            .await
            .map_err(|e| utils::js_error(&format!("Fetch failed: {:?}", e)))?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| utils::js_error("Failed to cast to Response"))?;

        if !resp.ok() {
            if resp.status() == 404 {
                // No methods published - return empty array
                return Ok(js_sys::Array::new().into());
            }
            return Err(utils::js_error(&format!("HTTP error: {}", resp.status())));
        }

        // Parse JSON response
        let json = JsFuture::from(resp.json().map_err(|_| utils::js_error("No JSON method"))?)
            .await
            .map_err(|_| utils::js_error("Failed to parse JSON"))?;

        Ok(json)
    }

    /// Fetch a specific payment endpoint for a public key and method
    #[wasm_bindgen(js_name = fetchEndpoint)]
    pub async fn fetch_endpoint(
        &self,
        public_key: &str,
        method_id: &str,
    ) -> Result<JsValue, JsValue> {
        utils::log(&format!(
            "Fetching endpoint for {} method {}",
            public_key, method_id
        ));

        let url = format!(
            "{}/pub/{}/paykit.app/v0/{}",
            self.homeserver, public_key, method_id
        );

        let fetch_url = if let Some(ref proxy) = self.proxy_url {
            format!("{}/{}", proxy, url)
        } else {
            url
        };

        let window = web_sys::window().ok_or_else(|| utils::js_error("No window object"))?;
        let resp_value = JsFuture::from(window.fetch_with_str(&fetch_url))
            .await
            .map_err(|e| utils::js_error(&format!("Fetch failed: {:?}", e)))?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| utils::js_error("Failed to cast to Response"))?;

        if !resp.ok() {
            if resp.status() == 404 {
                return Ok(JsValue::NULL);
            }
            return Err(utils::js_error(&format!("HTTP error: {}", resp.status())));
        }

        let text = JsFuture::from(resp.text().map_err(|_| utils::js_error("No text method"))?)
            .await
            .map_err(|_| utils::js_error("Failed to get text"))?;

        Ok(text)
    }

    /// List contents of a directory path
    ///
    /// Returns an array of entry names in the directory.
    ///
    /// # Arguments
    ///
    /// * `public_key` - The public key to query
    /// * `path` - The directory path (e.g., "/pub/pubky.app/follows/")
    #[wasm_bindgen(js_name = listDirectory)]
    pub async fn list_directory(
        &self,
        public_key: &str,
        path: &str,
    ) -> Result<Vec<JsValue>, JsValue> {
        utils::log(&format!("Listing directory for {} at {}", public_key, path));

        let url = format!("{}/pub/{}{}", self.homeserver, public_key, path);

        let fetch_url = if let Some(ref proxy) = self.proxy_url {
            format!("{}/{}", proxy, url)
        } else {
            url
        };

        let window = web_sys::window().ok_or_else(|| utils::js_error("No window object"))?;
        let resp_value = JsFuture::from(window.fetch_with_str(&fetch_url))
            .await
            .map_err(|e| utils::js_error(&format!("Fetch failed: {:?}", e)))?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| utils::js_error("Failed to cast to Response"))?;

        if !resp.ok() {
            if resp.status() == 404 {
                return Ok(Vec::new());
            }
            return Err(utils::js_error(&format!("HTTP error: {}", resp.status())));
        }

        // Try to parse as JSON array
        let json = JsFuture::from(resp.json().map_err(|_| utils::js_error("No JSON method"))?)
            .await
            .map_err(|_| utils::js_error("Failed to parse JSON"))?;

        // Convert JSON array to Vec<JsValue>
        if js_sys::Array::is_array(&json) {
            let arr: js_sys::Array = json.into();
            Ok(arr.iter().collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Get payment methods list for a public key
    ///
    /// Returns an array of method entries with method_id and endpoint.
    ///
    /// # Arguments
    ///
    /// * `public_key` - The public key to query
    /// * `auth_token` - Optional authentication token (not used for reads)
    #[wasm_bindgen(js_name = getPaymentList)]
    pub async fn get_payment_list(
        &self,
        public_key: &str,
        _auth_token: Option<String>,
    ) -> Result<Vec<JsValue>, JsValue> {
        utils::log(&format!("Getting payment list for {}", public_key));

        // Query the paykit directory
        let methods = self.query_methods(public_key).await?;

        // If it's an array, return it directly
        if js_sys::Array::is_array(&methods) {
            let arr: js_sys::Array = methods.into();
            return Ok(arr.iter().collect());
        }

        // If it's an object, extract the entries
        if methods.is_object() {
            let mut result = Vec::new();
            let obj = js_sys::Object::from(methods);
            let entries = js_sys::Object::entries(&obj);

            for i in 0..entries.length() {
                if let Some(entry) = entries.get(i).dyn_ref::<js_sys::Array>() {
                    if entry.length() >= 2 {
                        let js_obj = js_sys::Object::new();
                        let _ = js_sys::Reflect::set(&js_obj, &"method_id".into(), &entry.get(0));
                        let _ = js_sys::Reflect::set(&js_obj, &"endpoint".into(), &entry.get(1));
                        result.push(js_obj.into());
                    }
                }
            }
            return Ok(result);
        }

        Ok(Vec::new())
    }

    /// Publish a payment endpoint to the directory
    ///
    /// # Arguments
    ///
    /// * `public_key` - Your public key (z-base32 encoded)
    /// * `method_id` - The payment method ID (e.g., "lightning", "onchain")
    /// * `endpoint` - The endpoint data (e.g., LNURL, Bitcoin address)
    /// * `auth_token` - Optional authentication token for the homeserver
    ///
    /// # Modes
    ///
    /// - **Mock**: Saves to localStorage, returns success without network call
    /// - **Direct**: HTTP PUT directly to homeserver (requires CORS headers)
    /// - **Proxy**: HTTP PUT through configured CORS proxy
    #[wasm_bindgen(js_name = publishEndpoint)]
    pub async fn publish_endpoint(
        &self,
        public_key: &str,
        method_id: &str,
        endpoint: &str,
        auth_token: Option<String>,
    ) -> Result<PublishResult, JsValue> {
        utils::log(&format!(
            "Publishing endpoint for {} method {} (mode: {:?})",
            public_key, method_id, self.mode
        ));

        match self.mode {
            PublishMode::Mock => self.mock_publish(public_key, method_id, endpoint).await,
            PublishMode::Direct => {
                self.direct_publish(public_key, method_id, endpoint, auth_token)
                    .await
            }
            PublishMode::Proxy => {
                self.proxy_publish(public_key, method_id, endpoint, auth_token)
                    .await
            }
        }
    }

    /// Remove a payment endpoint from the directory
    ///
    /// # Arguments
    ///
    /// * `public_key` - Your public key (z-base32 encoded)
    /// * `method_id` - The payment method ID to remove
    /// * `auth_token` - Optional authentication token for the homeserver
    #[wasm_bindgen(js_name = removeEndpoint)]
    pub async fn remove_endpoint(
        &self,
        public_key: &str,
        method_id: &str,
        auth_token: Option<String>,
    ) -> Result<PublishResult, JsValue> {
        utils::log(&format!(
            "Removing endpoint for {} method {} (mode: {:?})",
            public_key, method_id, self.mode
        ));

        match self.mode {
            PublishMode::Mock => self.mock_remove(public_key, method_id).await,
            PublishMode::Direct => self.direct_remove(public_key, method_id, auth_token).await,
            PublishMode::Proxy => self.proxy_remove(public_key, method_id, auth_token).await,
        }
    }

    // Private implementation methods

    async fn mock_publish(
        &self,
        public_key: &str,
        method_id: &str,
        endpoint: &str,
    ) -> Result<PublishResult, JsValue> {
        let window = web_sys::window().ok_or_else(|| utils::js_error("No window object"))?;
        let storage = window
            .local_storage()?
            .ok_or_else(|| utils::js_error("No localStorage"))?;

        let key = format!("paykit_published:{}:{}", public_key, method_id);
        storage
            .set_item(&key, endpoint)
            .map_err(|e| utils::js_error(&format!("localStorage error: {:?}", e)))?;

        // Also update the timestamp
        let timestamp_key = format!("paykit_published_at:{}:{}", public_key, method_id);
        let timestamp = js_sys::Date::now().to_string();
        let _ = storage.set_item(&timestamp_key, &timestamp);

        Ok(PublishResult {
            success: true,
            message: "MOCK: Endpoint saved to localStorage. Not published to real homeserver. \
                To enable real publishing, configure a proxy or use a CORS-enabled homeserver."
                .to_string(),
            mode: PublishMode::Mock,
        })
    }

    async fn mock_remove(
        &self,
        public_key: &str,
        method_id: &str,
    ) -> Result<PublishResult, JsValue> {
        let window = web_sys::window().ok_or_else(|| utils::js_error("No window object"))?;
        let storage = window
            .local_storage()?
            .ok_or_else(|| utils::js_error("No localStorage"))?;

        let key = format!("paykit_published:{}:{}", public_key, method_id);
        storage
            .remove_item(&key)
            .map_err(|e| utils::js_error(&format!("localStorage error: {:?}", e)))?;

        let timestamp_key = format!("paykit_published_at:{}:{}", public_key, method_id);
        let _ = storage.remove_item(&timestamp_key);

        Ok(PublishResult {
            success: true,
            message: "MOCK: Endpoint removed from localStorage.".to_string(),
            mode: PublishMode::Mock,
        })
    }

    async fn direct_publish(
        &self,
        public_key: &str,
        method_id: &str,
        endpoint: &str,
        auth_token: Option<String>,
    ) -> Result<PublishResult, JsValue> {
        let url = format!(
            "{}/pub/{}/paykit.app/v0/{}",
            self.homeserver, public_key, method_id
        );

        self.http_put(&url, endpoint, auth_token, PublishMode::Direct)
            .await
    }

    async fn proxy_publish(
        &self,
        public_key: &str,
        method_id: &str,
        endpoint: &str,
        auth_token: Option<String>,
    ) -> Result<PublishResult, JsValue> {
        let proxy = self
            .proxy_url
            .as_ref()
            .ok_or_else(|| utils::js_error("Proxy mode requires proxy_url to be set"))?;

        let target_url = format!(
            "{}/pub/{}/paykit.app/v0/{}",
            self.homeserver, public_key, method_id
        );
        let url = format!("{}/{}", proxy, target_url);

        self.http_put(&url, endpoint, auth_token, PublishMode::Proxy)
            .await
    }

    async fn direct_remove(
        &self,
        public_key: &str,
        method_id: &str,
        auth_token: Option<String>,
    ) -> Result<PublishResult, JsValue> {
        let url = format!(
            "{}/pub/{}/paykit.app/v0/{}",
            self.homeserver, public_key, method_id
        );

        self.http_delete(&url, auth_token, PublishMode::Direct)
            .await
    }

    async fn proxy_remove(
        &self,
        public_key: &str,
        method_id: &str,
        auth_token: Option<String>,
    ) -> Result<PublishResult, JsValue> {
        let proxy = self
            .proxy_url
            .as_ref()
            .ok_or_else(|| utils::js_error("Proxy mode requires proxy_url to be set"))?;

        let target_url = format!(
            "{}/pub/{}/paykit.app/v0/{}",
            self.homeserver, public_key, method_id
        );
        let url = format!("{}/{}", proxy, target_url);

        self.http_delete(&url, auth_token, PublishMode::Proxy).await
    }

    async fn http_put(
        &self,
        url: &str,
        body: &str,
        auth_token: Option<String>,
        mode: PublishMode,
    ) -> Result<PublishResult, JsValue> {
        let window = web_sys::window().ok_or_else(|| utils::js_error("No window object"))?;

        let headers = Headers::new().map_err(|e| utils::js_error(&format!("{:?}", e)))?;
        headers
            .set("Content-Type", "text/plain")
            .map_err(|e| utils::js_error(&format!("{:?}", e)))?;

        if let Some(token) = auth_token {
            headers
                .set("Authorization", &format!("Bearer {}", token))
                .map_err(|e| utils::js_error(&format!("{:?}", e)))?;
        }

        let opts = RequestInit::new();
        opts.set_method("PUT");
        opts.set_headers(&headers);
        opts.set_body(&JsValue::from_str(body));
        opts.set_mode(RequestMode::Cors);

        let request = web_sys::Request::new_with_str_and_init(url, &opts)
            .map_err(|e| utils::js_error(&format!("Request creation failed: {:?}", e)))?;

        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| {
                utils::js_error(&format!(
                    "Fetch failed (CORS issue?): {:?}. Consider using proxy mode.",
                    e
                ))
            })?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| utils::js_error("Failed to cast to Response"))?;

        if resp.ok() || resp.status() == 201 || resp.status() == 204 {
            Ok(PublishResult {
                success: true,
                message: format!("Endpoint published successfully to {}", self.homeserver),
                mode,
            })
        } else {
            Ok(PublishResult {
                success: false,
                message: format!("HTTP error {}: Failed to publish", resp.status()),
                mode,
            })
        }
    }

    /// Fetch a profile from the Pubky directory
    ///
    /// Returns the profile JSON if found, or null if not found.
    ///
    /// # Arguments
    ///
    /// * `public_key` - The public key to fetch the profile for
    #[wasm_bindgen(js_name = fetchProfile)]
    pub async fn fetch_profile(&self, public_key: &str) -> Result<JsValue, JsValue> {
        utils::log(&format!("Fetching profile for {}", public_key));

        let url = format!(
            "{}/pub/{}/pubky.app/profile.json",
            self.homeserver, public_key
        );

        let fetch_url = if let Some(ref proxy) = self.proxy_url {
            format!("{}/{}", proxy, url)
        } else {
            url
        };

        let window = web_sys::window().ok_or_else(|| utils::js_error("No window object"))?;
        let resp_value = JsFuture::from(window.fetch_with_str(&fetch_url))
            .await
            .map_err(|e| utils::js_error(&format!("Fetch failed: {:?}", e)))?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| utils::js_error("Failed to cast to Response"))?;

        if !resp.ok() {
            if resp.status() == 404 {
                return Ok(JsValue::NULL);
            }
            return Err(utils::js_error(&format!("HTTP error: {}", resp.status())));
        }

        let json = JsFuture::from(resp.json().map_err(|_| utils::js_error("No JSON method"))?)
            .await
            .map_err(|_| utils::js_error("Failed to parse JSON"))?;

        Ok(json)
    }

    /// Publish a profile to the Pubky directory
    ///
    /// # Arguments
    ///
    /// * `public_key` - Your public key
    /// * `profile_json` - The profile data as a JSON string
    /// * `auth_token` - Authentication token for the homeserver
    #[wasm_bindgen(js_name = publishProfile)]
    pub async fn publish_profile(
        &self,
        public_key: &str,
        profile_json: &str,
        auth_token: Option<String>,
    ) -> Result<PublishResult, JsValue> {
        utils::log(&format!(
            "Publishing profile for {} (mode: {:?})",
            public_key, self.mode
        ));

        match self.mode {
            PublishMode::Mock => self.mock_publish_profile(public_key, profile_json).await,
            PublishMode::Direct => {
                self.direct_publish_profile(public_key, profile_json, auth_token)
                    .await
            }
            PublishMode::Proxy => {
                self.proxy_publish_profile(public_key, profile_json, auth_token)
                    .await
            }
        }
    }

    async fn mock_publish_profile(
        &self,
        public_key: &str,
        profile_json: &str,
    ) -> Result<PublishResult, JsValue> {
        let window = web_sys::window().ok_or_else(|| utils::js_error("No window object"))?;
        let storage = window
            .local_storage()?
            .ok_or_else(|| utils::js_error("No localStorage"))?;

        let key = format!("paykit_profile:{}", public_key);
        storage
            .set_item(&key, profile_json)
            .map_err(|e| utils::js_error(&format!("localStorage error: {:?}", e)))?;

        Ok(PublishResult {
            success: true,
            message: "MOCK: Profile saved to localStorage. Not published to real homeserver."
                .to_string(),
            mode: PublishMode::Mock,
        })
    }

    async fn direct_publish_profile(
        &self,
        public_key: &str,
        profile_json: &str,
        auth_token: Option<String>,
    ) -> Result<PublishResult, JsValue> {
        let url = format!(
            "{}/pub/{}/pubky.app/profile.json",
            self.homeserver, public_key
        );

        self.http_put_json(&url, profile_json, auth_token, PublishMode::Direct)
            .await
    }

    async fn proxy_publish_profile(
        &self,
        public_key: &str,
        profile_json: &str,
        auth_token: Option<String>,
    ) -> Result<PublishResult, JsValue> {
        let proxy = self
            .proxy_url
            .as_ref()
            .ok_or_else(|| utils::js_error("Proxy mode requires proxy_url to be set"))?;

        let target_url = format!(
            "{}/pub/{}/pubky.app/profile.json",
            self.homeserver, public_key
        );
        let url = format!("{}/{}", proxy, target_url);

        self.http_put_json(&url, profile_json, auth_token, PublishMode::Proxy)
            .await
    }

    async fn http_put_json(
        &self,
        url: &str,
        body: &str,
        auth_token: Option<String>,
        mode: PublishMode,
    ) -> Result<PublishResult, JsValue> {
        let window = web_sys::window().ok_or_else(|| utils::js_error("No window object"))?;

        let headers = Headers::new().map_err(|e| utils::js_error(&format!("{:?}", e)))?;
        headers
            .set("Content-Type", "application/json")
            .map_err(|e| utils::js_error(&format!("{:?}", e)))?;

        if let Some(token) = auth_token {
            headers
                .set("Authorization", &format!("Bearer {}", token))
                .map_err(|e| utils::js_error(&format!("{:?}", e)))?;
        }

        let opts = RequestInit::new();
        opts.set_method("PUT");
        opts.set_headers(&headers);
        opts.set_body(&JsValue::from_str(body));
        opts.set_mode(RequestMode::Cors);

        let request = web_sys::Request::new_with_str_and_init(url, &opts)
            .map_err(|e| utils::js_error(&format!("Request creation failed: {:?}", e)))?;

        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| {
                utils::js_error(&format!(
                    "Fetch failed (CORS issue?): {:?}. Consider using proxy mode.",
                    e
                ))
            })?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| utils::js_error("Failed to cast to Response"))?;

        if resp.ok() || resp.status() == 201 || resp.status() == 204 {
            Ok(PublishResult {
                success: true,
                message: format!("Published successfully to {}", self.homeserver),
                mode,
            })
        } else {
            Ok(PublishResult {
                success: false,
                message: format!("HTTP error {}: Failed to publish", resp.status()),
                mode,
            })
        }
    }

    async fn http_delete(
        &self,
        url: &str,
        auth_token: Option<String>,
        mode: PublishMode,
    ) -> Result<PublishResult, JsValue> {
        let window = web_sys::window().ok_or_else(|| utils::js_error("No window object"))?;

        let headers = Headers::new().map_err(|e| utils::js_error(&format!("{:?}", e)))?;

        if let Some(token) = auth_token {
            headers
                .set("Authorization", &format!("Bearer {}", token))
                .map_err(|e| utils::js_error(&format!("{:?}", e)))?;
        }

        let opts = RequestInit::new();
        opts.set_method("DELETE");
        opts.set_headers(&headers);
        opts.set_mode(RequestMode::Cors);

        let request = web_sys::Request::new_with_str_and_init(url, &opts)
            .map_err(|e| utils::js_error(&format!("Request creation failed: {:?}", e)))?;

        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| {
                utils::js_error(&format!(
                    "Fetch failed (CORS issue?): {:?}. Consider using proxy mode.",
                    e
                ))
            })?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| utils::js_error("Failed to cast to Response"))?;

        if resp.ok() || resp.status() == 204 || resp.status() == 404 {
            Ok(PublishResult {
                success: true,
                message: "Endpoint removed successfully".to_string(),
                mode,
            })
        } else {
            Ok(PublishResult {
                success: false,
                message: format!("HTTP error {}: Failed to remove", resp.status()),
                mode,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_directory_client_creation() {
        let client = DirectoryClient::new("https://demo.httprelay.io".to_string());
        assert_eq!(client.homeserver, "https://demo.httprelay.io");
        assert_eq!(client.mode, PublishMode::Direct);
    }

    #[wasm_bindgen_test]
    fn test_directory_client_mock_mode() {
        let client = DirectoryClient::with_mock_mode("https://demo.httprelay.io".to_string());
        assert_eq!(client.homeserver, "https://demo.httprelay.io");
        assert_eq!(client.mode, PublishMode::Mock);
    }

    #[wasm_bindgen_test]
    fn test_directory_client_with_proxy() {
        let client = DirectoryClient::with_proxy(
            "https://homeserver.example.com".to_string(),
            "https://cors-proxy.example.com".to_string(),
        );
        assert_eq!(client.homeserver, "https://homeserver.example.com");
        assert_eq!(
            client.proxy_url,
            Some("https://cors-proxy.example.com".to_string())
        );
        assert_eq!(client.mode, PublishMode::Proxy);
    }

    #[wasm_bindgen_test]
    fn test_directory_client_direct_access() {
        let client =
            DirectoryClient::with_direct_access("https://cors-enabled.example.com".to_string());
        assert_eq!(client.homeserver, "https://cors-enabled.example.com");
        assert_eq!(client.proxy_url, None);
        assert_eq!(client.mode, PublishMode::Direct);
    }

    #[wasm_bindgen_test]
    fn test_publish_mode_default() {
        assert_eq!(PublishMode::default(), PublishMode::Mock);
    }

    #[wasm_bindgen_test]
    async fn test_mock_publish() {
        let client = DirectoryClient::with_mock_mode("https://demo.httprelay.io".to_string());
        let result = client
            .publish_endpoint("testpubkey123", "lightning", "lnurl1234", None)
            .await
            .unwrap();

        assert!(result.success());
        assert!(result.message().contains("MOCK"));
        assert_eq!(result.mode(), PublishMode::Mock);
    }

    #[wasm_bindgen_test]
    async fn test_mock_remove() {
        let client = DirectoryClient::with_mock_mode("https://demo.httprelay.io".to_string());

        // First publish
        client
            .publish_endpoint("testpubkey456", "lightning", "lnurl1234", None)
            .await
            .unwrap();

        // Then remove
        let result = client
            .remove_endpoint("testpubkey456", "lightning", None)
            .await
            .unwrap();

        assert!(result.success());
        assert!(result.message().contains("MOCK"));
    }
}
