//! Directory operations for WASM

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

use crate::utils;

/// Directory client for querying payment methods
#[wasm_bindgen]
pub struct DirectoryClient {
    homeserver: String,
}

#[wasm_bindgen]
impl DirectoryClient {
    /// Create a new directory client
    #[wasm_bindgen(constructor)]
    pub fn new(homeserver: String) -> DirectoryClient {
        DirectoryClient { homeserver }
    }

    /// Query payment methods for a public key
    #[wasm_bindgen(js_name = queryMethods)]
    pub async fn query_methods(&self, public_key: &str) -> Result<JsValue, JsValue> {
        utils::log(&format!("Querying methods for: {}", public_key));

        // Construct the URL for the public directory
        let url = format!("{}/pub/{}/paykit.app/methods", self.homeserver, public_key);

        // Make the fetch call
        let window = web_sys::window().ok_or_else(|| utils::js_error("No window object"))?;
        let resp_value = JsFuture::from(window.fetch_with_str(&url))
            .await
            .map_err(|_| utils::js_error("Fetch failed"))?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| utils::js_error("Failed to cast to Response"))?;

        if !resp.ok() {
            return Err(utils::js_error(&format!("HTTP error: {}", resp.status())));
        }

        // Parse JSON response
        let json = JsFuture::from(resp.json().map_err(|_| utils::js_error("No JSON method"))?)
            .await
            .map_err(|_| utils::js_error("Failed to parse JSON"))?;

        Ok(json)
    }

    /// Publish payment methods (placeholder - requires authentication)
    #[wasm_bindgen(js_name = publishMethods)]
    pub async fn publish_methods(&self, _methods: JsValue) -> Result<(), JsValue> {
        // This would require a Pubky session which isn't yet implemented in WASM
        Err(utils::js_error(
            "Publishing requires authentication (not yet implemented in WASM)",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_directory_client_creation() {
        let client = DirectoryClient::new("https://demo.httprelay.io".to_string());
        assert_eq!(client.homeserver, "https://demo.httprelay.io");
    }
}
