//! WASM-compatible transport adapter for Paykit
//!
//! This module provides a WASM-compatible implementation of `UnauthenticatedTransportRead`
//! using the web demo's `DirectoryClient` for HTTP-based directory queries.

use paykit_lib::{
    UnauthenticatedTransportRead, EndpointData, MethodId, PaykitError, PublicKey,
    Result, SupportedPayments,
};
use js_sys::{Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};

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
        let mut methods = Vec::new();
        
        if methods_js.is_object() {
            let obj = Object::from(methods_js);
            let entries = Object::entries(&obj);
            
            for i in 0..entries.length() {
                if let Some(entry) = entries.get(i).dyn_ref::<js_sys::Array>() {
                    if entry.length() >= 2 {
                        let method_id_str = entry.get(0).as_string().unwrap_or_default();
                        let endpoint_str = entry.get(1).as_string().unwrap_or_default();
                        
                        if let Ok(method_id) = MethodId::new(&method_id_str) {
                            methods.push((method_id, EndpointData::new(endpoint_str)));
                        }
                    }
                }
            }
        }

        Ok(SupportedPayments { methods })
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
        
        let entries_js = self
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
        
        if let Ok(entries_array) = entries_js.dyn_into::<js_sys::Array>() {
            for i in 0..entries_array.length() {
                if let Some(entry_str) = entries_array.get(i).as_string() {
                    if let Ok(pubkey) = PublicKey::from_str(&entry_str) {
                        contacts.push(pubkey);
                    }
                }
            }
        }

        Ok(contacts)
    }

    async fn get(&self, _owner: &PublicKey, _path: &str) -> Result<Option<String>> {
        // TODO: Implement file retrieval using DirectoryClient
        // For now, return None as this is not critical for payment endpoint resolution
        Ok(None)
    }

    async fn list_directory(&self, owner: &PublicKey, path: &str) -> Result<Vec<String>> {
        let owner_str = owner.to_string();
        
        let entries_js = self
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
        
        if let Ok(entries_array) = entries_js.dyn_into::<js_sys::Array>() {
            for i in 0..entries_array.length() {
                let entry = entries_array.get(i);
                if let Some(entry_str) = entry.as_string() {
                    entries.push(entry_str);
                } else if entry.is_object() {
                    // Some directory listings return objects, extract name if available
                    if let Ok(obj) = entry.dyn_into::<Object>() {
                        if let Ok(name) = Reflect::get(&obj, &"name".into()) {
                            if let Some(name_str) = name.as_string() {
                                entries.push(name_str);
                            }
                        }
                    }
                }
            }
        }

        Ok(entries)
    }
}

use std::str::FromStr;

