//! WebSocket-based Noise protocol transport for WASM
//!
//! This module provides a browser-compatible implementation of the Noise IK
//! handshake protocol over WebSockets, enabling encrypted payment coordination
//! in the browser.

use crate::types::PaykitNoiseMessage;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use js_sys::Uint8Array;
use pubky_noise::{NoiseClient, NoiseLink, RingKeyProvider};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{BinaryType, CloseEvent, ErrorEvent, MessageEvent, WebSocket};

/// Errors that can occur during WebSocket Noise transport
#[derive(Debug, Clone)]
pub enum TransportError {
    ConnectionFailed(String),
    HandshakeFailed(String),
    EncryptionFailed(String),
    DecryptionFailed(String),
    SendFailed(String),
    ReceiveFailed(String),
    Serialization(String),
    WebSocketError(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            TransportError::HandshakeFailed(msg) => write!(f, "Handshake failed: {}", msg),
            TransportError::EncryptionFailed(msg) => write!(f, "Encryption failed: {}", msg),
            TransportError::DecryptionFailed(msg) => write!(f, "Decryption failed: {}", msg),
            TransportError::SendFailed(msg) => write!(f, "Send failed: {}", msg),
            TransportError::ReceiveFailed(msg) => write!(f, "Receive failed: {}", msg),
            TransportError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            TransportError::WebSocketError(msg) => write!(f, "WebSocket error: {}", msg),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<TransportError> for JsValue {
    fn from(err: TransportError) -> Self {
        JsValue::from_str(&err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, TransportError>;

/// WebSocket-based Noise protocol channel
///
/// This provides encrypted bidirectional communication over WebSockets
/// using the Noise_IK handshake pattern.
pub struct WebSocketNoiseChannel {
    ws: WebSocket,
    link: NoiseLink,
    rx: Rc<RefCell<UnboundedReceiver<Vec<u8>>>>,
    #[allow(dead_code)]
    tx: UnboundedSender<Vec<u8>>,
}

impl WebSocketNoiseChannel {
    /// Connect to a WebSocket endpoint and perform Noise handshake
    ///
    /// This performs the full IK handshake:
    /// 1. Client sends `-> e, es, s, ss` (includes identity payload)
    /// 2. Server responds with `<- e, ee, se` (completes handshake)
    /// 3. Channel is ready for encrypted transport
    pub async fn connect<R: RingKeyProvider>(
        ws_url: &str,
        client: &NoiseClient<R, ()>,
        server_static_pub: &[u8; 32],
    ) -> Result<Self> {
        // 1. Create WebSocket connection
        let ws = WebSocket::new(ws_url)
            .map_err(|e| TransportError::ConnectionFailed(format!("{:?}", e)))?;

        // Set binary type for raw bytes
        ws.set_binary_type(BinaryType::Arraybuffer);

        // 2. Set up message queue
        let (tx, rx) = unbounded::<Vec<u8>>();
        let rx = Rc::new(RefCell::new(rx));

        // 3. Set up WebSocket event handlers
        let tx_clone = tx.clone();
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let uint8_array = Uint8Array::new(&array_buffer);
                let mut data = vec![0u8; uint8_array.length() as usize];
                uint8_array.copy_to(&mut data);

                // Send to message queue
                let _ = tx_clone.unbounded_send(data);
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget(); // Keep closure alive

        let onerror = Closure::wrap(Box::new(move |e: ErrorEvent| {
            crate::utils::error(&format!("WebSocket error: {:?}", e.message()));
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        let onclose = Closure::wrap(Box::new(move |e: CloseEvent| {
            crate::utils::log(&format!(
                "WebSocket closed: code={}, reason={}",
                e.code(),
                e.reason()
            ));
        }) as Box<dyn FnMut(CloseEvent)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();

        // 4. Wait for WebSocket to be ready
        Self::wait_for_open(&ws).await?;

        // 5. Perform Noise handshake
        let link = Self::perform_client_handshake(&ws, &rx, client, server_static_pub).await?;

        Ok(Self { ws, link, rx, tx })
    }

    /// Wait for WebSocket to reach OPEN state
    async fn wait_for_open(ws: &WebSocket) -> Result<()> {
        if ws.ready_state() == WebSocket::OPEN {
            return Ok(());
        }

        let (tx, mut rx) = unbounded::<Result<()>>();

        let tx_clone = tx.clone();
        let onopen = Closure::once(Box::new(move || {
            let _ = tx_clone.unbounded_send(Ok(()));
        }) as Box<dyn FnOnce()>);

        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        let tx_clone = tx;
        let onerror = Closure::once(Box::new(move |e: ErrorEvent| {
            let _ = tx_clone.unbounded_send(Err(TransportError::ConnectionFailed(format!(
                "Connection failed: {:?}",
                e.message()
            ))));
        }) as Box<dyn FnOnce(ErrorEvent)>);

        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        // Wait for open or error
        rx.try_next()
            .map_err(|_| TransportError::ConnectionFailed("Channel closed".to_string()))?
            .ok_or_else(|| TransportError::ConnectionFailed("No response".to_string()))?
    }

    /// Perform client-side Noise IK handshake
    async fn perform_client_handshake<R: RingKeyProvider>(
        ws: &WebSocket,
        rx: &Rc<RefCell<UnboundedReceiver<Vec<u8>>>>,
        client: &NoiseClient<R, ()>,
        server_static_pub: &[u8; 32],
    ) -> Result<NoiseLink> {
        // 1. Build the IK handshake initiation message
        let (hs, first_msg) =
            pubky_noise::datalink_adapter::client_start_ik_direct(client, server_static_pub, None)
                .map_err(|e| {
                    TransportError::HandshakeFailed(format!("Handshake build failed: {}", e))
                })?;

        // 2. Send the handshake initiation message
        let uint8_array = Uint8Array::from(&first_msg[..]);
        ws.send_with_array_buffer(&uint8_array.buffer())
            .map_err(|e| {
                TransportError::HandshakeFailed(format!("Failed to send handshake: {:?}", e))
            })?;

        // 3. Receive server's response message
        let response = Self::receive_raw(rx).await.map_err(|e| {
            TransportError::HandshakeFailed(format!("Failed to read handshake response: {}", e))
        })?;

        // 4. Complete the handshake
        let link =
            pubky_noise::datalink_adapter::client_complete_ik(hs, &response).map_err(|e| {
                TransportError::HandshakeFailed(format!("Failed to complete handshake: {}", e))
            })?;

        Ok(link)
    }

    /// Receive raw bytes from the message queue
    #[allow(clippy::await_holding_refcell_ref)]
    async fn receive_raw(rx: &Rc<RefCell<UnboundedReceiver<Vec<u8>>>>) -> Result<Vec<u8>> {
        use futures::stream::StreamExt;

        let mut rx_mut = rx.borrow_mut();
        rx_mut
            .next()
            .await
            .ok_or_else(|| TransportError::ReceiveFailed("Channel closed".to_string()))
    }

    /// Send a Paykit message over the encrypted channel
    pub async fn send(&mut self, msg: PaykitNoiseMessage) -> Result<()> {
        // 1. Serialize message
        let json_bytes =
            serde_json::to_vec(&msg).map_err(|e| TransportError::Serialization(e.to_string()))?;

        // 2. Encrypt
        let ciphertext = self
            .link
            .encrypt(&json_bytes)
            .map_err(|e| TransportError::EncryptionFailed(format!("Encryption failed: {}", e)))?;

        // 3. Send length-prefixed
        let len = (ciphertext.len() as u32).to_be_bytes();
        let mut message = Vec::with_capacity(4 + ciphertext.len());
        message.extend_from_slice(&len);
        message.extend_from_slice(&ciphertext);

        let uint8_array = Uint8Array::from(&message[..]);
        self.ws
            .send_with_array_buffer(&uint8_array.buffer())
            .map_err(|e| TransportError::SendFailed(format!("Send failed: {:?}", e)))?;

        Ok(())
    }

    /// Receive a Paykit message from the encrypted channel
    pub async fn recv(&mut self) -> Result<PaykitNoiseMessage> {
        // 1. Read length prefix (4 bytes)
        let len_bytes = Self::receive_raw(&self.rx).await?;

        if len_bytes.len() < 4 {
            return Err(TransportError::ReceiveFailed(
                "Incomplete length prefix".to_string(),
            ));
        }

        let len =
            u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        // 2. Read ciphertext (remaining bytes or next message)
        let ciphertext = if len_bytes.len() > 4 {
            // Data was sent in one message
            len_bytes[4..].to_vec()
        } else {
            // Data comes in next message
            Self::receive_raw(&self.rx).await?
        };

        if ciphertext.len() != len {
            return Err(TransportError::ReceiveFailed(format!(
                "Expected {} bytes, got {}",
                len,
                ciphertext.len()
            )));
        }

        // 3. Decrypt
        let plaintext = self
            .link
            .decrypt(&ciphertext)
            .map_err(|e| TransportError::DecryptionFailed(format!("Decryption failed: {}", e)))?;

        // 4. Deserialize
        let msg = serde_json::from_slice(&plaintext)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;

        Ok(msg)
    }

    /// Close the WebSocket connection
    pub fn close(&self) -> Result<()> {
        self.ws
            .close()
            .map_err(|e| TransportError::WebSocketError(format!("Close failed: {:?}", e)))
    }
}

/// WASM-exposed client for initiating payments over WebSocket
#[wasm_bindgen]
pub struct WasmPaymentClient {
    coordinator: crate::payment::WasmPaymentCoordinator,
}

impl Default for WasmPaymentClient {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmPaymentClient {
    /// Create a new payment client
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            coordinator: crate::payment::WasmPaymentCoordinator::new(),
        }
    }

    /// Connect to a payee and initiate a payment
    ///
    /// # Arguments
    ///
    /// * `payer_identity_json` - JSON string of the payer's identity (from Identity.toJSON())
    /// * `homeserver` - Pubky homeserver URL for endpoint discovery
    /// * `payee_pubkey` - Recipient's public key (can be pubky:// URI or raw key)
    /// * `amount` - Payment amount as string
    /// * `currency` - Currency code (e.g., "SAT", "USD")
    /// * `method` - Payment method ID (e.g., "lightning", "onchain")
    ///
    /// # Returns
    ///
    /// A promise that resolves with the receipt JSON string
    ///
    /// # Example
    ///
    /// ```javascript
    /// const client = new WasmPaymentClient();
    /// const identity = Identity.fromJSON(localStorage.getItem('identity'));
    /// const receipt = await client.pay(
    ///     identity.toJSON(),
    ///     'https://homeserver.example.com',
    ///     'pubky://8pin...',
    ///     '1000',
    ///     'SAT',
    ///     'lightning'
    /// );
    /// ```
    pub async fn pay(
        &self,
        payer_identity_json: &str,
        homeserver: &str,
        payee_pubkey: &str,
        amount: &str,
        currency: &str,
        method: &str,
    ) -> std::result::Result<String, JsValue> {
        use crate::directory::DirectoryClient;
        use crate::payment::parse_noise_endpoint_wasm;
        use paykit_lib::PublicKey;
        use std::str::FromStr;

        // Extract payee public key (handle pubky:// URIs)
        let payee_pubkey_str = if payee_pubkey.starts_with("pubky://") {
            payee_pubkey.strip_prefix("pubky://").unwrap()
        } else {
            payee_pubkey
        };

        let payee = PublicKey::from_str(payee_pubkey_str)
            .map_err(|e| JsValue::from_str(&format!("Invalid payee pubkey: {}", e)))?;

        // Use smart checkout to resolve payment endpoint (private → public fallback)
        use crate::private_endpoints::WasmPrivateEndpointStorage;
        use crate::wasm_transport::WasmUnauthenticatedTransport;
        use paykit_interactive::smart_checkout_detailed;
        use paykit_lib::MethodId;
        
        let method_id = MethodId(method.to_string());
        
        // Create storage adapter for private endpoints
        let endpoint_storage = WasmPrivateEndpointStorage::new();
        
        struct WebPaykitStorage {
            endpoint_storage: WasmPrivateEndpointStorage,
        }
        
        #[async_trait::async_trait]
        impl paykit_interactive::PaykitStorage for WebPaykitStorage {
            async fn save_receipt(&self, _receipt: &paykit_interactive::PaykitReceipt) -> paykit_interactive::Result<()> {
                Ok(()) // Not used in this context
            }
            
            async fn get_receipt(&self, _receipt_id: &str) -> paykit_interactive::Result<Option<paykit_interactive::PaykitReceipt>> {
                Ok(None) // Not used in this context
            }
            
            async fn list_receipts(&self) -> paykit_interactive::Result<Vec<paykit_interactive::PaykitReceipt>> {
                Ok(vec![]) // Not used in this context
            }
            
            async fn save_private_endpoint(
                &self,
                _peer: &paykit_lib::PublicKey,
                _method: &paykit_lib::MethodId,
                _endpoint: &str,
            ) -> paykit_interactive::Result<()> {
                Ok(()) // Not used in this context
            }
            
            async fn get_private_endpoint(
                &self,
                peer: &paykit_lib::PublicKey,
                method: &paykit_lib::MethodId,
            ) -> paykit_interactive::Result<Option<String>> {
                match self.endpoint_storage.get(peer, method).await {
                    Ok(Some(endpoint)) => Ok(Some(endpoint.endpoint.0)),
                    Ok(None) => Ok(None),
                    Err(_) => Ok(None), // Return None on error
                }
            }
            
            async fn list_private_endpoints_for_peer(
                &self,
                peer: &paykit_lib::PublicKey,
            ) -> paykit_interactive::Result<Vec<(paykit_lib::MethodId, String)>> {
                match self.endpoint_storage.list_for_peer(peer).await {
                    Ok(endpoints) => {
                        Ok(endpoints.into_iter().map(|e| (e.method_id, e.endpoint.0)).collect())
                    }
                    Err(_) => Ok(vec![]), // Return empty on error
                }
            }
            
            async fn remove_private_endpoint(
                &self,
                _peer: &paykit_lib::PublicKey,
                _method: &paykit_lib::MethodId,
            ) -> paykit_interactive::Result<()> {
                Ok(()) // Not used in this context
            }
        }
        
        let storage_adapter = WebPaykitStorage {
            endpoint_storage,
        };
        
        // Create WASM-compatible public directory transport
        let public_transport = WasmUnauthenticatedTransport::new(homeserver.to_string());
        
        // Use smart checkout to resolve payment endpoint
        let checkout_result = smart_checkout_detailed(
            &storage_adapter,
            &public_transport,
            &payee,
            &method_id,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Smart checkout failed: {:?}", e)))?;
        
        // Log endpoint source if found
        if let Some(result) = &checkout_result {
            let source = if result.is_private { "private" } else { "public" };
            // Note: We still need the noise:// endpoint for WebSocket connection
            // The smart checkout gives us the payment endpoint, not the noise transport endpoint
        }
        
        // Discover payment endpoint from directory (for noise:// transport endpoint)
        let client = DirectoryClient::new(homeserver.to_string());
        let methods_js = client
            .query_methods(&payee.to_string())
            .await
            .map_err(|e| {
                let error_msg = if e.is_string() {
                    e.as_string().unwrap_or_else(|| format!("{:?}", e))
                } else {
                    format!("{:?}", e)
                };
                JsValue::from_str(&format!("Directory query failed: {}", error_msg))
            })?;

        // Convert JsValue to a Rust map to find noise:// endpoint
        // The methods object is a JavaScript object with method_id -> endpoint mappings
        let noise_endpoint = {
            use js_sys::{Object, Reflect};
            
            // Try to convert to a JavaScript object
            let obj = if methods_js.is_object() {
                Object::from(methods_js)
            } else {
                return Err(JsValue::from_str("Directory query returned invalid format"));
            };

            // Get all keys
            let keys = Object::keys(&obj);
            let mut found_endpoint: Option<String> = None;

            for i in 0..keys.length() {
                if let Some(key_js) = keys.get(i).as_string() {
                    if let Ok(endpoint_js) = Reflect::get(&obj, &key_js.into()) {
                        if let Some(endpoint_str) = endpoint_js.as_string() {
                            if endpoint_str.starts_with("noise://") {
                                found_endpoint = Some(endpoint_str);
                                break;
                            }
                        }
                    }
                }
            }

            found_endpoint.ok_or_else(|| {
                JsValue::from_str(
                    "No noise:// endpoint found for recipient. Recipient must publish a noise:// endpoint.",
                )
            })?
        };

        // Parse noise endpoint to get WebSocket URL and server key
        let endpoint_info = parse_noise_endpoint_wasm(&noise_endpoint)?;
        let endpoint_json: serde_json::Value = serde_json::from_str(
            &endpoint_info
                .as_string()
                .ok_or_else(|| JsValue::from_str("Failed to parse endpoint info"))?,
        )
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;

        let ws_url = endpoint_json
            .get("ws_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsValue::from_str("Missing ws_url in endpoint info"))?;
        let server_key_hex = endpoint_json
            .get("server_key_hex")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsValue::from_str("Missing server_key_hex in endpoint info"))?;

        // Initiate payment using coordinator
        self.coordinator
            .initiate_payment(
                payer_identity_json,
                ws_url,
                &payee.to_string(),
                server_key_hex,
                amount,
                currency,
                method,
            )
            .await
    }
}

/// WASM-exposed server for receiving payments over WebSocket
///
/// **Browser Limitation**: Browsers cannot directly accept incoming TCP/WebSocket connections.
/// To receive payments in a browser, you must use a WebSocket relay server that:
/// 1. Listens on a TCP port
/// 2. Accepts incoming Noise protocol connections
/// 3. Relays messages to/from browser clients via WebSocket
///
/// This struct provides utilities for handling payment requests once they arrive
/// via a relay server, but does not implement the listening functionality.
#[wasm_bindgen]
pub struct WasmPaymentServer {
    receiver: crate::payment::WasmPaymentReceiver,
}

impl Default for WasmPaymentServer {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmPaymentServer {
    /// Create a new payment server
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            receiver: crate::payment::WasmPaymentReceiver::new(),
        }
    }

    /// Accept a payment request (for use with relay server)
    ///
    /// This method processes a payment request that was received via a WebSocket relay server.
    /// The relay server should:
    /// 1. Accept incoming Noise protocol connections
    /// 2. Decrypt messages using the Noise channel
    /// 3. Forward decrypted payment requests to this method via WebSocket
    ///
    /// # Arguments
    ///
    /// * `request_json` - JSON string of the payment request (PaykitReceipt)
    ///
    /// # Returns
    ///
    /// Confirmed receipt JSON string
    ///
    /// # Example
    ///
    /// ```javascript
    /// // In relay server WebSocket handler:
    /// const server = new WasmPaymentServer();
    /// const receipt = await server.acceptPayment(requestJson);
    /// // Send receipt back via relay server
    /// ```
    pub async fn accept_payment(&self, request_json: &str) -> std::result::Result<String, JsValue> {
        self.receiver.accept_payment(request_json).await
    }

    /// Get stored receipts
    pub async fn get_receipts(&self) -> std::result::Result<Vec<JsValue>, JsValue> {
        self.receiver.get_receipts().await
    }

    /// Start listening for payment requests
    ///
    /// **Note**: This method is not fully implemented because browsers cannot directly
    /// listen on TCP ports. To receive payments in a browser:
    ///
    /// 1. **Use a WebSocket Relay Server**: Set up a server that:
    ///    - Listens on a TCP port (e.g., 8888)
    ///    - Accepts Noise protocol connections
    ///    - Relays messages to/from browser clients via WebSocket
    ///
    /// 2. **Connect from Browser**: Connect to the relay server's WebSocket endpoint
    ///    and handle incoming payment requests using `accept_payment()`
    ///
    /// # Example Relay Server Architecture
    ///
    /// ```
    /// [Noise Client] --TCP--> [Relay Server] --WebSocket--> [Browser]
    /// ```
    ///
    /// The relay server:
    /// - Accepts Noise connections on TCP port
    /// - Decrypts/encrypts messages
    /// - Forwards to browser via WebSocket
    /// - Returns responses back through Noise channel
    ///
    /// For a complete implementation, see the CLI demo's `receive` command which
    /// implements a full Noise protocol server.
    pub async fn listen(&self, _port: u16) -> std::result::Result<(), JsValue> {
        Err(JsValue::from_str(
            "Browser cannot listen directly on TCP ports. Use a WebSocket relay server. \
             See documentation for details on setting up a relay server architecture.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_transport_error_display() {
        let err = TransportError::ConnectionFailed("test".to_string());
        assert!(err.to_string().contains("Connection failed"));
    }

    // More tests will be added in Phase 5
}
