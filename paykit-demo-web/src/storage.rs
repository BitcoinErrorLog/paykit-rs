//! Browser storage for identities and data

use wasm_bindgen::prelude::*;
use web_sys::{Storage, Window};

use crate::{utils, Identity};

/// Get browser's localStorage
fn get_local_storage() -> Result<Storage, JsValue> {
    let window: Window = web_sys::window().ok_or_else(|| utils::js_error("No window object"))?;
    window
        .local_storage()
        .map_err(|_| utils::js_error("Could not access localStorage"))?
        .ok_or_else(|| utils::js_error("localStorage is not available"))
}

/// Storage manager for browser localStorage
#[wasm_bindgen]
pub struct BrowserStorage;

impl Default for BrowserStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl BrowserStorage {
    /// Create a new browser storage manager
    #[wasm_bindgen(constructor)]
    pub fn new() -> BrowserStorage {
        BrowserStorage
    }

    /// Save an identity to localStorage
    #[wasm_bindgen(js_name = saveIdentity)]
    pub fn save_identity(&self, name: &str, identity: &Identity) -> Result<(), JsValue> {
        let storage = get_local_storage()?;
        let key = format!("paykit_identity_{}", name);
        let json = identity.to_json()?;
        storage
            .set_item(&key, &json)
            .map_err(|_| utils::js_error("Failed to save identity"))
    }

    /// Load an identity from localStorage
    #[wasm_bindgen(js_name = loadIdentity)]
    pub fn load_identity(&self, name: &str) -> Result<Identity, JsValue> {
        let storage = get_local_storage()?;
        let key = format!("paykit_identity_{}", name);
        let json = storage
            .get_item(&key)
            .map_err(|_| utils::js_error("Failed to read from storage"))?
            .ok_or_else(|| utils::js_error(&format!("Identity '{}' not found", name)))?;
        Identity::from_json(&json)
    }

    /// List all saved identity names
    #[wasm_bindgen(js_name = listIdentities)]
    pub fn list_identities(&self) -> Result<Vec<JsValue>, JsValue> {
        let storage = get_local_storage()?;
        let length = storage
            .length()
            .map_err(|_| utils::js_error("Failed to get storage length"))?;

        let mut names = Vec::new();
        let prefix = "paykit_identity_";

        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if let Some(name) = key.strip_prefix(prefix) {
                    names.push(JsValue::from_str(name));
                }
            }
        }

        Ok(names)
    }

    /// Delete an identity from localStorage
    #[wasm_bindgen(js_name = deleteIdentity)]
    pub fn delete_identity(&self, name: &str) -> Result<(), JsValue> {
        let storage = get_local_storage()?;
        let key = format!("paykit_identity_{}", name);
        storage
            .remove_item(&key)
            .map_err(|_| utils::js_error("Failed to delete identity"))
    }

    /// Get the current active identity name
    #[wasm_bindgen(js_name = getCurrentIdentity)]
    pub fn get_current_identity(&self) -> Result<Option<String>, JsValue> {
        let storage = get_local_storage()?;
        storage
            .get_item("paykit_current_identity")
            .map_err(|_| utils::js_error("Failed to read current identity"))
    }

    /// Set the current active identity
    #[wasm_bindgen(js_name = setCurrentIdentity)]
    pub fn set_current_identity(&self, name: &str) -> Result<(), JsValue> {
        let storage = get_local_storage()?;
        storage
            .set_item("paykit_current_identity", name)
            .map_err(|_| utils::js_error("Failed to set current identity"))
    }

    /// Clear all Paykit data from localStorage
    #[wasm_bindgen(js_name = clearAll)]
    pub fn clear_all(&self) -> Result<(), JsValue> {
        let storage = get_local_storage()?;
        let length = storage
            .length()
            .map_err(|_| utils::js_error("Failed to get storage length"))?;

        let mut keys_to_remove = Vec::new();
        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with("paykit_") {
                    keys_to_remove.push(key);
                }
            }
        }

        for key in keys_to_remove {
            storage
                .remove_item(&key)
                .map_err(|_| utils::js_error("Failed to clear storage"))?;
        }

        Ok(())
    }
}
