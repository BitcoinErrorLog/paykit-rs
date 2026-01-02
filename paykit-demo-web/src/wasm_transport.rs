//! WASM-compatible transport adapter for Paykit
//!
//! This module provides a WASM-compatible implementation of `UnauthenticatedTransportRead`
//! using the web demo's `DirectoryClient` for HTTP-based directory queries.

use js_sys::{Object, Reflect};
use paykit_lib::{
    EndpointData, MethodId, PaykitError, PublicKey, Result, SupportedPayments,
    UnauthenticatedTransportRead,
};
use wasm_bindgen::JsCast;

use crate::directory::DirectoryClient;

/// WASM-compatible transport adapter that uses HTTP fetch for directory queries
pub struct WasmUnauthenticatedTransport {
    directory_client: DirectoryClient,
}

impl WasmUnauthenticatedTransport {
    /// Create a new WASM transport adapter
    pub fn new(homeserver: String) -> Self {
        Self {
            directory_client: DirectoryClient::new(homeserver),
        }
    }

    /// Create with a CORS proxy
    pub fn with_proxy(homeserver: String, proxy_url: String) -> Self {
        Self {
            directory_client: DirectoryClient::with_proxy(homeserver, proxy_url),
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl UnauthenticatedTransportRead for WasmUnauthenticatedTransport {
    async fn fetch_supported_payments(&self, payee: &PublicKey) -> Result<SupportedPayments> {
        let payee_str = payee.to_string();
        let methods_js = self
            .directory_client
            .query_methods(&payee_str)
            .await
            .map_err(|e| {
                let error_msg = if e.is_string() {
                    e.as_string().unwrap_or_else(|| format!("{:?}", e))
                } else {
                    format!("{:?}", e)
                };
                PaykitError::Transport(format!("Directory query failed: {}", error_msg))
            })?;

        // Convert JsValue to SupportedPayments
        let mut entries = std::collections::HashMap::new();

        if methods_js.is_object() {
            let obj = Object::from(methods_js);
            let obj_entries = Object::entries(&obj);

            for i in 0..obj_entries.length() {
                if let Some(entry) = obj_entries.get(i).dyn_ref::<js_sys::Array>() {
                    if entry.length() >= 2 {
                        let method_id_str = entry.get(0).as_string().unwrap_or_default();
                        let endpoint_str = entry.get(1).as_string().unwrap_or_default();

                        let method_id = MethodId::new(&method_id_str);
                        entries.insert(method_id, EndpointData::new(endpoint_str));
                    }
                }
            }
        }

        Ok(SupportedPayments { entries })
    }

    async fn fetch_payment_endpoint(
        &self,
        payee: &PublicKey,
        method: &MethodId,
    ) -> Result<Option<EndpointData>> {
        let payee_str = payee.to_string();
        let method_str = method.to_string();

        let endpoint_js = self
            .directory_client
            .fetch_endpoint(&payee_str, &method_str)
            .await
            .map_err(|e| {
                let error_msg = if e.is_string() {
                    e.as_string().unwrap_or_else(|| format!("{:?}", e))
                } else {
                    format!("{:?}", e)
                };
                PaykitError::Transport(format!("Endpoint fetch failed: {}", error_msg))
            })?;

        if endpoint_js.is_null() {
            return Ok(None);
        }

        let endpoint_str = endpoint_js
            .as_string()
            .ok_or_else(|| PaykitError::Transport("Invalid endpoint format".to_string()))?;

        Ok(Some(EndpointData::new(endpoint_str)))
    }

    async fn fetch_known_contacts(&self, owner: &PublicKey) -> Result<Vec<PublicKey>> {
        // Use the directory client's list_directory to query the follows path
        let owner_str = owner.to_string();
        let follows_path = "/pub/pubky.app/follows/";

        let entries_vec = self
            .directory_client
            .list_directory(&owner_str, follows_path)
            .await
            .map_err(|e| {
                let error_msg = if e.is_string() {
                    e.as_string().unwrap_or_else(|| format!("{:?}", e))
                } else {
                    format!("{:?}", e)
                };
                PaykitError::Transport(format!("Contact discovery failed: {}", error_msg))
            })?;

        let mut contacts = Vec::new();

        // entries_vec is already a Vec<JsValue>
        for entry_js in entries_vec {
            if let Some(entry_str) = entry_js.as_string() {
                if let Ok(pubkey) = PublicKey::from_str(&entry_str) {
                    contacts.push(pubkey);
                }
            }
        }

        Ok(contacts)
    }

    async fn get(&self, owner: &PublicKey, path: &str) -> Result<Option<String>> {
        let owner_str = owner.to_string();

        // Use the directory client's homeserver to construct URL
        let homeserver = self.directory_client.homeserver();
        let proxy_url = self.directory_client.proxy_url();

        // Construct the URL for the file
        let url = format!("{}/pub/{}{}", homeserver, owner_str, path);
        let fetch_url = if let Some(proxy) = proxy_url {
            format!("{}/{}", proxy, url)
        } else {
            url
        };

        // Make the fetch call
        let window = web_sys::window()
            .ok_or_else(|| PaykitError::Transport("No window object".to_string()))?;

        let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(&fetch_url))
            .await
            .map_err(|e| PaykitError::Transport(format!("Fetch failed: {:?}", e)))?;

        let resp: web_sys::Response = resp_value
            .dyn_into()
            .map_err(|_| PaykitError::Transport("Failed to cast to Response".to_string()))?;

        if resp.status() == 404 {
            return Ok(None);
        }

        if !resp.ok() {
            return Err(PaykitError::Transport(format!(
                "HTTP error: {}",
                resp.status()
            )));
        }

        // Get text content
        let text_promise = resp
            .text()
            .map_err(|_| PaykitError::Transport("No text method".to_string()))?;
        let text = wasm_bindgen_futures::JsFuture::from(text_promise)
            .await
            .map_err(|_| PaykitError::Transport("Failed to get text".to_string()))?;

        let content = text
            .as_string()
            .ok_or_else(|| PaykitError::Transport("Response is not a string".to_string()))?;

        if content.is_empty() {
            return Ok(None);
        }

        Ok(Some(content))
    }

    async fn list_directory(&self, owner: &PublicKey, path: &str) -> Result<Vec<String>> {
        let owner_str = owner.to_string();

        let entries_vec = self
            .directory_client
            .list_directory(&owner_str, path)
            .await
            .map_err(|e| {
                let error_msg = if e.is_string() {
                    e.as_string().unwrap_or_else(|| format!("{:?}", e))
                } else {
                    format!("{:?}", e)
                };
                PaykitError::Transport(format!("Directory listing failed: {}", error_msg))
            })?;

        let mut entries = Vec::new();

        // entries_vec is already a Vec<JsValue>
        for entry_js in entries_vec {
            if let Some(entry_str) = entry_js.as_string() {
                entries.push(entry_str);
            } else if entry_js.is_object() {
                // Some directory listings return objects, extract name if available
                if let Ok(name) = Reflect::get(&entry_js.into(), &"name".into()) {
                    if let Some(name_str) = name.as_string() {
                        entries.push(name_str);
                    }
                }
            }
        }

        Ok(entries)
    }
}

use std::str::FromStr;
