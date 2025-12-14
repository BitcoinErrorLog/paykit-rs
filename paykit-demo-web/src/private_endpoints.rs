//! Private endpoint storage for Web demo using IndexedDB

use paykit_lib::{
    private_endpoints::{PrivateEndpoint, PrivateEndpointStore, StorageError, StorageResult},
    MethodId, PublicKey,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    IdbDatabase, IdbFactory, IdbOpenDbRequest, IdbTransactionMode, IdbVersionChangeEvent, Window,
};

/// Web-based private endpoint storage using IndexedDB
#[wasm_bindgen]
pub struct WasmPrivateEndpointStorage {
    db_name: String,
    store_name: String,
}

#[derive(Serialize, Deserialize)]
struct StoredEndpoint {
    encrypted_data: Vec<u8>,
    created_at: i64,
}

impl WasmPrivateEndpointStorage {
    /// Create a new private endpoint storage
    pub fn new() -> Self {
        Self {
            db_name: "paykit_private_endpoints".to_string(),
            store_name: "endpoints".to_string(),
        }
    }

    /// Set password for encryption (for future use with WebCryptoStorage)
    #[allow(dead_code)]
    pub fn set_password(&self, _password: Vec<u8>) {
        // For now, encryption is handled at IndexedDB level
        // Future: integrate with WebCryptoStorage for additional encryption
    }

    /// Initialize the IndexedDB database
    async fn init_db(&self) -> Result<IdbDatabase, StorageError> {
        let window: Window = web_sys::window()
            .ok_or_else(|| StorageError::Other("No window".to_string()))?;

        let idb_factory: IdbFactory = window
            .indexed_db()
            .map_err(|_| StorageError::Other("IndexedDB not available".to_string()))?
            .ok_or_else(|| StorageError::Other("IndexedDB not available".to_string()))?;

        let open_request: IdbOpenDbRequest = idb_factory
            .open_with_u32(&self.db_name, 1)
            .map_err(|_| StorageError::Other("Failed to open database".to_string()))?;

        // Set up onupgradeneeded handler
        let store_name = self.store_name.clone();
        let closure = Closure::wrap(Box::new(move |event: IdbVersionChangeEvent| {
            let target = event.target().unwrap();
            let request: IdbOpenDbRequest = target.dyn_into().unwrap();
            if let Ok(db_js) = request.result() {
                if let Ok(db) = db_js.dyn_into::<IdbDatabase>() {
                    // Create object store if it doesn't exist
                    // Note: object_store_names() may not be available, so we'll try to create
                    // and ignore errors if it already exists
                    let _ = db.create_object_store(&store_name);
                }
            }
        }) as Box<dyn FnMut(IdbVersionChangeEvent)>);

        open_request.set_onupgradeneeded(Some(closure.as_ref().unchecked_ref()));
        closure.forget();

        // Convert IdbOpenDbRequest to JsFuture
        // IdbOpenDbRequest -> JsValue -> Promise -> JsFuture
        let js_value: JsValue = open_request.into();
        let promise: js_sys::Promise = js_value
            .dyn_into()
            .map_err(|_| StorageError::Other("Failed to convert to Promise".to_string()))?;
        let js_future = JsFuture::from(promise);
        let result = js_future
            .await
            .map_err(|_| StorageError::Other("Database open failed".to_string()))?;

        let db: IdbDatabase = result
            .dyn_into()
            .map_err(|_| StorageError::Other("Invalid database object".to_string()))?;

        Ok(db)
    }

    /// Create a storage key from peer and method
    fn make_key(peer: &PublicKey, method_id: &MethodId) -> String {
        format!("{}:{}", peer.to_string(), method_id.0)
    }

    /// Encrypt endpoint data
    /// For now, data is stored as JSON in IndexedDB
    /// Future: integrate with WebCryptoStorage for encryption
    async fn encrypt_endpoint(&self, endpoint: &PrivateEndpoint) -> Result<Vec<u8>, StorageError> {
        let json = serde_json::to_string(endpoint)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        Ok(json.as_bytes().to_vec())
    }

    /// Decrypt endpoint data
    async fn decrypt_endpoint(&self, encrypted: &[u8]) -> Result<PrivateEndpoint, StorageError> {
        // Decrypt - for now, data is stored as JSON
        // In full implementation, would decrypt using crypto storage
        let json = String::from_utf8(encrypted.to_vec())
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        let endpoint: PrivateEndpoint = serde_json::from_str(&json)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        Ok(endpoint)
    }
}

impl Default for WasmPrivateEndpointStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PrivateEndpointStore for WasmPrivateEndpointStorage {
    async fn save(&self, endpoint: PrivateEndpoint) -> StorageResult<()> {
        let key = Self::make_key(&endpoint.peer, &endpoint.method_id);
        
        // Encrypt the endpoint data
        let encrypted = self.encrypt_endpoint(&endpoint).await?;
        
        // Store in IndexedDB
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
            .map_err(|e| StorageError::Other(format!("Failed to create transaction: {:?}", e)))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| StorageError::Other(format!("Failed to get object store: {:?}", e)))?;

        let stored = StoredEndpoint {
            encrypted_data: encrypted,
            created_at: endpoint.created_at,
        };

        let value = serde_wasm_bindgen::to_value(&stored)
            .map_err(|e| StorageError::Serialization(format!("Serialization failed: {}", e)))?;

        store
            .put_with_key(&value, &JsValue::from_str(&key))
            .map_err(|e| StorageError::Other(format!("Failed to store endpoint: {:?}", e)))?;

        let js_value: JsValue = transaction.into();
        let promise: js_sys::Promise = js_value.dyn_into().map_err(|_| StorageError::Other("Failed to convert transaction to Promise".to_string()))?;
        let promise: JsFuture = JsFuture::from(promise);
        promise
            .await
            .map_err(|e| StorageError::Other(format!("Transaction failed: {:?}", e)))?;

        Ok(())
    }

    async fn get(
        &self,
        peer: &PublicKey,
        method_id: &MethodId,
    ) -> StorageResult<Option<PrivateEndpoint>> {
        let key = Self::make_key(peer, method_id);
        
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str(&self.store_name)
            .map_err(|e| StorageError::Other(format!("Failed to create transaction: {:?}", e)))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| StorageError::Other(format!("Failed to get object store: {:?}", e)))?;

        let request = store
            .get(&JsValue::from_str(&key))
            .map_err(|e| StorageError::Other(format!("Failed to get endpoint: {:?}", e)))?;

        let js_value: JsValue = request.into();
        let promise_obj: js_sys::Promise = js_value.dyn_into().map_err(|_| StorageError::Other("Failed to convert request to Promise".to_string()))?;
        let promise: JsFuture = JsFuture::from(promise_obj);
        let result = promise
            .await
            .map_err(|e| StorageError::Other(format!("Get request failed: {:?}", e)))?;

        if result.is_undefined() || result.is_null() {
            return Ok(None);
        }

        let stored: StoredEndpoint = serde_wasm_bindgen::from_value(result)
            .map_err(|e| StorageError::Serialization(format!("Deserialization failed: {}", e)))?;

        let endpoint = self.decrypt_endpoint(&stored.encrypted_data).await?;
        Ok(Some(endpoint))
    }

    async fn list_for_peer(&self, peer: &PublicKey) -> StorageResult<Vec<PrivateEndpoint>> {
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str(&self.store_name)
            .map_err(|e| StorageError::Other(format!("Failed to create transaction: {:?}", e)))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| StorageError::Other(format!("Failed to get object store: {:?}", e)))?;

        let request = store
            .get_all()
            .map_err(|e| StorageError::Other(format!("Failed to get all: {:?}", e)))?;

        let js_value: JsValue = request.into();
        let promise_obj: js_sys::Promise = js_value.dyn_into().map_err(|_| StorageError::Other("Failed to convert request to Promise".to_string()))?;
        let promise: JsFuture = JsFuture::from(promise_obj);
        let result = promise
            .await
            .map_err(|e| StorageError::Other(format!("Get all failed: {:?}", e)))?;

        let js_array = js_sys::Array::from(&result);
        let mut endpoints = Vec::new();
        let peer_str = peer.to_string();

        for i in 0..js_array.length() {
            let item = js_array.get(i);
            let stored: StoredEndpoint = serde_wasm_bindgen::from_value(item)
                .map_err(|e| StorageError::Serialization(format!("Deserialization failed: {}", e)))?;

            let endpoint = self.decrypt_endpoint(&stored.encrypted_data).await?;
            if endpoint.peer.to_string() == peer_str {
                endpoints.push(endpoint);
            }
        }

        Ok(endpoints)
    }

    async fn list_peers(&self) -> StorageResult<Vec<PublicKey>> {
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str(&self.store_name)
            .map_err(|e| StorageError::Other(format!("Failed to create transaction: {:?}", e)))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| StorageError::Other(format!("Failed to get object store: {:?}", e)))?;

        let request = store
            .get_all()
            .map_err(|e| StorageError::Other(format!("Failed to get all: {:?}", e)))?;

        let js_value: JsValue = request.into();
        let promise_obj: js_sys::Promise = js_value.dyn_into().map_err(|_| StorageError::Other("Failed to convert request to Promise".to_string()))?;
        let promise: JsFuture = JsFuture::from(promise_obj);
        let result = promise
            .await
            .map_err(|e| StorageError::Other(format!("Get all failed: {:?}", e)))?;

        let js_array = js_sys::Array::from(&result);
        let mut peers = std::collections::HashSet::new();

        for i in 0..js_array.length() {
            let item = js_array.get(i);
            let stored: StoredEndpoint = serde_wasm_bindgen::from_value(item)
                .map_err(|e| StorageError::Serialization(format!("Deserialization failed: {}", e)))?;

            let endpoint = self.decrypt_endpoint(&stored.encrypted_data).await?;
            peers.insert(endpoint.peer);
        }

        Ok(peers.into_iter().collect())
    }

    async fn remove(&self, peer: &PublicKey, method_id: &MethodId) -> StorageResult<()> {
        let key = Self::make_key(peer, method_id);
        
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
            .map_err(|e| StorageError::Other(format!("Failed to create transaction: {:?}", e)))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| StorageError::Other(format!("Failed to get object store: {:?}", e)))?;

        store
            .delete(&JsValue::from_str(&key))
            .map_err(|e| StorageError::Other(format!("Failed to delete endpoint: {:?}", e)))?;

        let js_value: JsValue = transaction.into();
        let promise: js_sys::Promise = js_value.dyn_into().map_err(|_| StorageError::Other("Failed to convert transaction to Promise".to_string()))?;
        let promise: JsFuture = JsFuture::from(promise);
        promise
            .await
            .map_err(|e| StorageError::Other(format!("Transaction failed: {:?}", e)))?;

        Ok(())
    }

    async fn remove_all_for_peer(&self, peer: &PublicKey) -> StorageResult<()> {
        let endpoints = self.list_for_peer(peer).await?;
        for endpoint in endpoints {
            self.remove(&endpoint.peer, &endpoint.method_id).await?;
        }
        Ok(())
    }

    async fn cleanup_expired(&self) -> StorageResult<usize> {
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str(&self.store_name)
            .map_err(|e| StorageError::Other(format!("Failed to create transaction: {:?}", e)))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| StorageError::Other(format!("Failed to get object store: {:?}", e)))?;

        let request = store
            .get_all()
            .map_err(|e| StorageError::Other(format!("Failed to get all: {:?}", e)))?;

        let js_value: JsValue = request.into();
        let promise_obj: js_sys::Promise = js_value.dyn_into().map_err(|_| StorageError::Other("Failed to convert request to Promise".to_string()))?;
        let promise: JsFuture = JsFuture::from(promise_obj);
        let result = promise
            .await
            .map_err(|e| StorageError::Other(format!("Get all failed: {:?}", e)))?;

        let js_array = js_sys::Array::from(&result);
        let mut expired_keys = Vec::new();

        for i in 0..js_array.length() {
            let item = js_array.get(i);
            let stored: StoredEndpoint = serde_wasm_bindgen::from_value(item)
                .map_err(|e| StorageError::Serialization(format!("Deserialization failed: {}", e)))?;

            let endpoint = self.decrypt_endpoint(&stored.encrypted_data).await?;
            if endpoint.is_expired() {
                expired_keys.push(Self::make_key(&endpoint.peer, &endpoint.method_id));
            }
        }

        // Delete expired endpoints
        for key in &expired_keys {
            let db = self.init_db().await?;
            let transaction = db
                .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
                .map_err(|e| StorageError::Other(format!("Failed to create transaction: {:?}", e)))?;
            let store = transaction
                .object_store(&self.store_name)
                .map_err(|e| StorageError::Other(format!("Failed to get object store: {:?}", e)))?;
            store
                .delete(&JsValue::from_str(key))
                .map_err(|e| StorageError::Other(format!("Failed to delete: {:?}", e)))?;
            let js_value: JsValue = transaction.into();
        let promise: js_sys::Promise = js_value.dyn_into().map_err(|_| StorageError::Other("Failed to convert transaction to Promise".to_string()))?;
        let promise: JsFuture = JsFuture::from(promise);
            promise.await.map_err(|e| StorageError::Other(format!("Transaction failed: {:?}", e)))?;
        }

        Ok(expired_keys.len())
    }

    async fn update(&self, endpoint: PrivateEndpoint) -> StorageResult<()> {
        self.save(endpoint).await
    }

    async fn count(&self) -> StorageResult<usize> {
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str(&self.store_name)
            .map_err(|e| StorageError::Other(format!("Failed to create transaction: {:?}", e)))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| StorageError::Other(format!("Failed to get object store: {:?}", e)))?;

        let request = store
            .count()
            .map_err(|e| StorageError::Other(format!("Failed to count: {:?}", e)))?;

        let js_value: JsValue = request.into();
        let promise_obj: js_sys::Promise = js_value.dyn_into().map_err(|_| StorageError::Other("Failed to convert request to Promise".to_string()))?;
        let promise: JsFuture = JsFuture::from(promise_obj);
        let result = promise
            .await
            .map_err(|e| StorageError::Other(format!("Count failed: {:?}", e)))?;

        let count = js_sys::Reflect::get(&result, &JsValue::from_str("result"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as usize;

        Ok(count)
    }
}
