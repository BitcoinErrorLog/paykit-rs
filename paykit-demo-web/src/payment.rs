//! Payment coordination module for WASM
//!
//! This module provides high-level payment coordination using WebSocket Noise channels.

use crate::identity::{Identity, WasmKeyProvider};
use crate::types::{PaykitNoiseMessage, PaykitReceipt};
use crate::websocket_transport::WebSocketNoiseChannel;
use paykit_lib::{MethodId, PublicKey};
use pubky_noise::NoiseClient;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

/// Receipt storage in browser localStorage
#[wasm_bindgen]
pub struct WasmReceiptStorage {
    storage_key: String,
}

impl Default for WasmReceiptStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmReceiptStorage {
    /// Create new receipt storage
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            storage_key: "paykit_receipts".to_string(),
        }
    }

    /// Save a receipt
    pub async fn save_receipt(&self, receipt_id: &str, receipt_json: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:{}", self.storage_key, receipt_id);
        storage
            .set_item(&key, receipt_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to save receipt: {:?}", e)))?;

        Ok(())
    }

    /// Get a receipt by ID
    pub async fn get_receipt(&self, receipt_id: &str) -> Result<Option<String>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:{}", self.storage_key, receipt_id);
        storage
            .get_item(&key)
            .map_err(|e| JsValue::from_str(&format!("Failed to get receipt: {:?}", e)))
    }

    /// List all receipts
    pub async fn list_receipts(&self) -> Result<Vec<JsValue>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let mut receipts = Vec::new();
        let prefix = format!("{}:", self.storage_key);
        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix) {
                    if let Ok(Some(json)) = storage.get_item(&key) {
                        receipts.push(JsValue::from_str(&json));
                    }
                }
            }
        }

        Ok(receipts)
    }

    /// Delete a receipt
    pub async fn delete_receipt(&self, receipt_id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:{}", self.storage_key, receipt_id);
        storage
            .remove_item(&key)
            .map_err(|e| JsValue::from_str(&format!("Failed to delete receipt: {:?}", e)))?;

        Ok(())
    }

    /// Filter receipts by direction (sent/received)
    ///
    /// # Arguments
    ///
    /// * `direction` - "sent" or "received"
    /// * `current_pubkey` - Current user's public key to determine direction
    ///
    /// # Examples
    ///
    /// ```
    /// let storage = WasmReceiptStorage::new();
    /// let sent = storage.filter_by_direction("sent", "8pin...").await?;
    /// ```
    pub async fn filter_by_direction(
        &self,
        direction: &str,
        current_pubkey: &str,
    ) -> Result<Vec<JsValue>, JsValue> {
        let all_receipts = self.list_receipts().await?;

        let filtered: Vec<JsValue> = all_receipts
            .into_iter()
            .filter(|receipt_js| {
                if let Some(json_str) = receipt_js.as_string() {
                    if let Ok(receipt_obj) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        let payer = receipt_obj
                            .get("payer")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        if direction == "sent" {
                            return payer == current_pubkey;
                        } else if direction == "received" {
                            return payer != current_pubkey;
                        }
                    }
                }
                false
            })
            .collect();

        Ok(filtered)
    }

    /// Filter receipts by method
    ///
    /// # Arguments
    ///
    /// * `method` - Payment method ID (e.g., "lightning", "onchain")
    ///
    /// # Examples
    ///
    /// ```
    /// let storage = WasmReceiptStorage::new();
    /// let lightning_receipts = storage.filter_by_method("lightning").await?;
    /// ```
    pub async fn filter_by_method(&self, method: &str) -> Result<Vec<JsValue>, JsValue> {
        let all_receipts = self.list_receipts().await?;

        let filtered: Vec<JsValue> = all_receipts
            .into_iter()
            .filter(|receipt_js| {
                if let Some(json_str) = receipt_js.as_string() {
                    if let Ok(receipt_obj) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        let receipt_method = receipt_obj
                            .get("method")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        return receipt_method == method;
                    }
                }
                false
            })
            .collect();

        Ok(filtered)
    }

    /// Filter receipts by contact public key
    ///
    /// # Arguments
    ///
    /// * `contact_pubkey` - Public key of the contact
    /// * `current_pubkey` - Current user's public key
    ///
    /// # Examples
    ///
    /// ```
    /// let storage = WasmReceiptStorage::new();
    /// let alice_receipts = storage.filter_by_contact("8pin...", "my_pubkey").await?;
    /// ```
    pub async fn filter_by_contact(
        &self,
        contact_pubkey: &str,
        current_pubkey: &str,
    ) -> Result<Vec<JsValue>, JsValue> {
        let all_receipts = self.list_receipts().await?;

        let filtered: Vec<JsValue> = all_receipts
            .into_iter()
            .filter(|receipt_js| {
                if let Some(json_str) = receipt_js.as_string() {
                    if let Ok(receipt_obj) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        let payer = receipt_obj
                            .get("payer")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let payee = receipt_obj
                            .get("payee")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        // Include if contact is either payer or payee (not current user)
                        return (payer == contact_pubkey && payee == current_pubkey)
                            || (payee == contact_pubkey && payer == current_pubkey);
                    }
                }
                false
            })
            .collect();

        Ok(filtered)
    }

    /// Export receipts as JSON array
    ///
    /// # Returns
    ///
    /// A JSON string containing array of all receipts
    ///
    /// # Examples
    ///
    /// ```
    /// let storage = WasmReceiptStorage::new();
    /// let json = storage.export_as_json().await?;
    /// // Download or process json
    /// ```
    pub async fn export_as_json(&self) -> Result<String, JsValue> {
        let receipts = self.list_receipts().await?;

        let receipt_objects: Vec<serde_json::Value> = receipts
            .into_iter()
            .filter_map(|r| {
                r.as_string()
                    .and_then(|json_str| serde_json::from_str(&json_str).ok())
            })
            .collect();

        serde_json::to_string_pretty(&receipt_objects)
            .map_err(|e| JsValue::from_str(&format!("Failed to export: {}", e)))
    }

    /// Get receipt statistics
    ///
    /// Returns an object with:
    /// - total: Total number of receipts
    /// - sent: Number of sent payments
    /// - received: Number of received payments
    ///
    /// # Arguments
    ///
    /// * `current_pubkey` - Current user's public key
    ///
    /// # Examples
    ///
    /// ```
    /// let storage = WasmReceiptStorage::new();
    /// let stats = storage.get_statistics("my_pubkey").await?;
    /// ```
    pub async fn get_statistics(&self, current_pubkey: &str) -> Result<JsValue, JsValue> {
        let all_receipts = self.list_receipts().await?;

        let mut sent = 0;
        let mut received = 0;

        for receipt_js in &all_receipts {
            if let Some(json_str) = receipt_js.as_string() {
                if let Ok(receipt_obj) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    let payer = receipt_obj
                        .get("payer")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if payer == current_pubkey {
                        sent += 1;
                    } else {
                        received += 1;
                    }
                }
            }
        }

        let stats = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&stats, &"total".into(), &all_receipts.len().into());
        let _ = js_sys::Reflect::set(&stats, &"sent".into(), &sent.into());
        let _ = js_sys::Reflect::set(&stats, &"received".into(), &received.into());

        Ok(stats.into())
    }

    /// Verify payment proof for a receipt
    ///
    /// # Arguments
    ///
    /// * `receipt_id` - The receipt ID to verify
    ///
    /// # Returns
    ///
    /// JS object with verification result:
    /// - `valid: boolean` - Whether proof is valid
    /// - `errors: string[]` - Array of error messages if invalid
    /// - `details: object` - Additional verification details
    ///
    /// # Examples
    ///
    /// ```
    /// let storage = WasmReceiptStorage::new();
    /// let result = storage.verify_proof("receipt_123").await?;
    /// ```
    pub async fn verify_proof(&self, receipt_id: &str) -> Result<JsValue, JsValue> {
        use paykit_interactive::proof::{PaymentProof, ProofType, ProofVerifier};
        use paykit_interactive::proof::verifiers::RealLightningProofVerifier;
        
        // Get receipt
        let receipt_json = self.get_receipt(receipt_id).await?
            .ok_or_else(|| utils::js_error(&format!("Receipt not found: {}", receipt_id)))?;
        
        let receipt: PaykitReceipt = serde_json::from_str(&receipt_json)
            .map_err(|e| utils::js_error(&format!("Failed to parse receipt: {}", e)))?;
        
        let proof_json = receipt.proof
            .ok_or_else(|| utils::js_error("Receipt has no proof"))?;
        
        // Parse proof
        let proof: PaymentProof = serde_json::from_value(proof_json.clone())
            .map_err(|e| utils::js_error(&format!("Failed to parse proof: {}", e)))?;
        
        // Verify based on proof type
        let verification_result = match proof.proof_type {
            ProofType::LightningPreimage { .. } => {
                let verifier = RealLightningProofVerifier::new();
                verifier.verify(&proof).await
            }
            ProofType::BitcoinTxid { .. } => {
                // Bitcoin verification requires Esplora API - not available in WASM
                return Err(utils::js_error("Bitcoin proof verification requires server-side Esplora API"));
            }
            ProofType::Custom { .. } => {
                return Err(utils::js_error("Custom proof verification not implemented"));
            }
        };
        
        // Build result object
        let result = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&result, &"valid".into(), &verification_result.valid.into());
        
        let errors_array = js_sys::Array::new();
        for error in &verification_result.errors {
            errors_array.push(&JsValue::from_str(error));
        }
        let _ = js_sys::Reflect::set(&result, &"errors".into(), &errors_array.into());
        
        if let Some(details) = verification_result.details {
            let details_js = serde_wasm_bindgen::to_value(&details)
                .map_err(|e| utils::js_error(&format!("Failed to serialize details: {}", e)))?;
            let _ = js_sys::Reflect::set(&result, &"details".into(), &details_js);
        }
        
        // Update receipt if verification succeeded
        if verification_result.valid {
            let mut updated_receipt = receipt;
            updated_receipt = updated_receipt.mark_proof_verified();
            let updated_json = serde_json::to_string(&updated_receipt)
                .map_err(|e| utils::js_error(&format!("Failed to serialize receipt: {}", e)))?;
            self.save_receipt(receipt_id, &updated_json).await?;
        }
        
        Ok(result.into())
    }

    /// Clear all receipts
    ///
    /// # Examples
    ///
    /// ```
    /// let storage = WasmReceiptStorage::new();
    /// storage.clear_all().await?;
    /// ```
    pub async fn clear_all(&self) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let mut keys_to_remove = Vec::new();
        let prefix = format!("{}:", self.storage_key);
        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

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
                .map_err(|e| JsValue::from_str(&format!("Failed to remove receipt: {:?}", e)))?;
        }

        Ok(())
    }
}

/// Payment coordinator for initiating payments
#[wasm_bindgen]
pub struct WasmPaymentCoordinator {
    receipt_storage: WasmReceiptStorage,
}

impl Default for WasmPaymentCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmPaymentCoordinator {
    /// Create new payment coordinator
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            receipt_storage: WasmReceiptStorage::new(),
        }
    }

    /// Initiate a payment to a payee
    ///
    /// This performs the full payment flow:
    /// 1. Connect to payee's WebSocket endpoint
    /// 2. Perform Noise handshake
    /// 3. Send payment request
    /// 4. Receive receipt confirmation
    /// 5. Store receipt
    ///
    /// Returns receipt JSON on success
    #[allow(clippy::too_many_arguments)]
    pub async fn initiate_payment(
        &self,
        payer_identity_json: &str,
        ws_url: &str,
        payee_pubkey: &str,
        server_static_key_hex: &str,
        amount: &str,
        currency: &str,
        method: &str,
    ) -> Result<String, JsValue> {
        // Parse payer identity
        let payer_identity = Identity::from_json(payer_identity_json)?;
        let payer = PublicKey::from_str(&payer_identity.public_key())
            .map_err(|e| JsValue::from_str(&format!("Invalid payer pubkey: {}", e)))?;

        // Parse payee
        let payee = PublicKey::from_str(payee_pubkey)
            .map_err(|e| JsValue::from_str(&format!("Invalid payee pubkey: {}", e)))?;

        // Parse server key
        let server_key_bytes = hex::decode(server_static_key_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid server key hex: {}", e)))?;
        let mut server_key = [0u8; 32];
        if server_key_bytes.len() != 32 {
            return Err(JsValue::from_str("Server key must be 32 bytes"));
        }
        server_key.copy_from_slice(&server_key_bytes);

        // Create provisional receipt
        let receipt_id = format!("pay_{}", uuid::Uuid::new_v4());
        let provisional_receipt = PaykitReceipt::new(
            receipt_id.clone(),
            payer.clone(),
            payee.clone(),
            MethodId(method.to_string()),
            Some(amount.to_string()),
            Some(currency.to_string()),
            serde_json::json!({
                "status": "requested",
                "timestamp": chrono::Utc::now().timestamp()
            }),
        );

        // Create Noise client with WASM key provider
        let key_provider = WasmKeyProvider::from_identity(&payer_identity);
        let device_id = b"wasm-browser-v1"; // Fixed device ID for browser
        let client = NoiseClient::new_direct(
            "paykit".to_string(),
            device_id,
            std::sync::Arc::new(key_provider),
        );

        // Connect and perform handshake
        let mut channel = WebSocketNoiseChannel::connect(ws_url, &client, &server_key)
            .await
            .map_err(|e| JsValue::from_str(&format!("Connection failed: {}", e)))?;

        // Send payment request
        channel
            .send(PaykitNoiseMessage::RequestReceipt {
                provisional_receipt: provisional_receipt.clone(),
            })
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to send request: {}", e)))?;

        // Receive confirmation
        let response = channel
            .recv()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to receive response: {}", e)))?;

        let confirmed_receipt = match response {
            PaykitNoiseMessage::ConfirmReceipt { receipt } => receipt,
            PaykitNoiseMessage::Error { code, message } => {
                return Err(JsValue::from_str(&format!(
                    "Payment error [{}]: {}",
                    code, message
                )));
            }
            _ => {
                return Err(JsValue::from_str("Unexpected response from payee"));
            }
        };

        // Serialize receipt
        let receipt_json = serde_json::to_string(&confirmed_receipt)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

        // Store receipt
        self.receipt_storage
            .save_receipt(&receipt_id, &receipt_json)
            .await?;

        Ok(receipt_json)
    }

    /// Get stored receipts
    pub async fn get_receipts(&self) -> Result<Vec<JsValue>, JsValue> {
        self.receipt_storage.list_receipts().await
    }
}

/// Payment receiver for accepting payments
#[wasm_bindgen]
pub struct WasmPaymentReceiver {
    receipt_storage: WasmReceiptStorage,
}

impl Default for WasmPaymentReceiver {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmPaymentReceiver {
    /// Create new payment receiver
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            receipt_storage: WasmReceiptStorage::new(),
        }
    }

    /// Accept a payment request
    ///
    /// Note: In browser, this typically requires a WebSocket relay server
    /// since browsers cannot directly accept incoming connections.
    pub async fn accept_payment(&self, request_json: &str) -> Result<String, JsValue> {
        // Parse request
        let request: PaykitReceipt = serde_json::from_str(request_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid request: {}", e)))?;

        // Generate confirmed receipt
        let confirmed_receipt = PaykitReceipt::new(
            request.receipt_id.clone(),
            request.payer.clone(),
            request.payee.clone(),
            request.method_id.clone(),
            request.amount.clone(),
            request.currency.clone(),
            serde_json::json!({
                "status": "accepted",
                "timestamp": chrono::Utc::now().timestamp()
            }),
        );

        // Serialize
        let receipt_json = serde_json::to_string(&confirmed_receipt)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

        // Store
        self.receipt_storage
            .save_receipt(&confirmed_receipt.receipt_id, &receipt_json)
            .await?;

        Ok(receipt_json)
    }

    /// Get stored receipts
    pub async fn get_receipts(&self) -> Result<Vec<JsValue>, JsValue> {
        self.receipt_storage.list_receipts().await
    }
}

/// Parse a Noise endpoint string and return WebSocket URL and server key
///
/// Format: noise://host:port@pubkey_hex
/// Returns JSON: { ws_url: string, server_key_hex: string, host: string, port: number }
#[wasm_bindgen]
pub fn parse_noise_endpoint_wasm(endpoint: &str) -> Result<JsValue, JsValue> {
    if !endpoint.starts_with("noise://") {
        return Err(JsValue::from_str("Endpoint must start with 'noise://'"));
    }

    let without_prefix = endpoint.strip_prefix("noise://").unwrap();
    let parts: Vec<&str> = without_prefix.split('@').collect();

    if parts.len() != 2 {
        return Err(JsValue::from_str(
            "Invalid Noise endpoint format. Expected: noise://host:port@pubkey_hex",
        ));
    }

    let host_port = parts[0];
    let server_key_hex = parts[1];

    // Validate server key is 64 hex characters (32 bytes)
    if server_key_hex.len() != 64 || !server_key_hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(JsValue::from_str(
            "Server key must be 64 hex characters (32 bytes)",
        ));
    }

    // Parse host and port
    let colon_index = host_port
        .rfind(':')
        .ok_or_else(|| JsValue::from_str("Invalid host:port format"))?;

    let host = host_port[..colon_index].to_string();
    let port_str = &host_port[colon_index + 1..];
    let port: u16 = port_str
        .parse()
        .map_err(|_| JsValue::from_str("Invalid port number"))?;

    if port == 0 {
        return Err(JsValue::from_str("Port must be between 1 and 65535"));
    }

    // Convert to WebSocket URL
    // Use wss:// for non-localhost, ws:// for localhost
    let protocol = if host == "localhost"
        || host == "127.0.0.1"
        || host.starts_with("192.168.")
        || host.starts_with("10.")
        || host.starts_with("172.")
    {
        "ws"
    } else {
        "wss"
    };
    let ws_url = format!("{}://{}:{}", protocol, host, port);

    // Build JSON response
    let result = serde_json::json!({
        "ws_url": ws_url,
        "server_key_hex": server_key_hex,
        "host": host,
        "port": port
    });

    Ok(JsValue::from_str(&serde_json::to_string(&result).map_err(
        |e| JsValue::from_str(&format!("Serialization error: {}", e)),
    )?))
}

/// Extract public key from pubky:// URI or raw public key
///
/// Returns public key string
#[wasm_bindgen]
pub fn extract_pubkey_from_uri_wasm(uri: &str) -> Result<String, JsValue> {
    // If it's a pubky:// URI, extract the key
    if uri.starts_with("pubky://") {
        return Ok(uri.strip_prefix("pubky://").unwrap().to_string());
    }

    // Otherwise, assume it's a raw public key
    // Validation should be done by the caller using is_valid_pubkey
    Ok(uri.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_receipt_storage_creation() {
        let storage = WasmReceiptStorage::new();
        assert_eq!(storage.storage_key, "paykit_receipts");
    }

    #[wasm_bindgen_test]
    fn test_payment_coordinator_creation() {
        let coordinator = WasmPaymentCoordinator::new();
        assert_eq!(coordinator.receipt_storage.storage_key, "paykit_receipts");
    }

    #[wasm_bindgen_test]
    fn test_parse_noise_endpoint_wasm() {
        let endpoint = "noise://127.0.0.1:9735@0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let result = parse_noise_endpoint_wasm(endpoint);
        assert!(result.is_ok());

        let json_str = result.unwrap().as_string().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["ws_url"], "ws://127.0.0.1:9735");
        assert_eq!(parsed["host"], "127.0.0.1");
        assert_eq!(parsed["port"], 9735);
        assert_eq!(
            parsed["server_key_hex"],
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
    }

    #[wasm_bindgen_test]
    fn test_parse_noise_endpoint_wasm_remote() {
        let endpoint = "noise://example.com:9735@0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let result = parse_noise_endpoint_wasm(endpoint);
        assert!(result.is_ok());

        let json_str = result.unwrap().as_string().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["ws_url"], "wss://example.com:9735");
    }

    #[wasm_bindgen_test]
    fn test_parse_noise_endpoint_wasm_invalid() {
        let endpoint = "noise://127.0.0.1:9735"; // Missing @ and pubkey
        let result = parse_noise_endpoint_wasm(endpoint);
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_extract_pubkey_from_uri_wasm() {
        let uri = "pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
        let result = extract_pubkey_from_uri_wasm(uri);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
        );
    }

    #[wasm_bindgen_test]
    fn test_extract_pubkey_from_uri_wasm_raw() {
        let pubkey = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
        let result = extract_pubkey_from_uri_wasm(pubkey);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), pubkey);
    }
}
