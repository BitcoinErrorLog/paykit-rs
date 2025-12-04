//! WebSocket-based Noise protocol transport for WASM
//!
//! This module provides a browser-compatible implementation of the Noise
//! handshake protocols over WebSockets, enabling encrypted payment
//! coordination in the browser. In addition to the default IK flow it now
//! supports IK-raw (cold keys), N (anonymous client), and NN (fully anonymous)
//! patterns via dedicated helper methods.

use crate::types::PaykitNoiseMessage;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use js_sys::Uint8Array;
use pubky_noise::{datalink_adapter, NoiseClient, NoisePattern, NoiseSession, RingKeyProvider};
use std::{cell::RefCell, convert::TryInto, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{BinaryType, CloseEvent, ErrorEvent, MessageEvent, WebSocket};
use zeroize::Zeroizing;

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
    session: NoiseSession,
    rx: Rc<RefCell<UnboundedReceiver<Vec<u8>>>>,
    #[allow(dead_code)]
    tx: UnboundedSender<Vec<u8>>,
}

type WebSocketChannels = (
    WebSocket,
    Rc<RefCell<UnboundedReceiver<Vec<u8>>>>,
    UnboundedSender<Vec<u8>>,
);

impl WebSocketNoiseChannel {
    fn from_parts(
        ws: WebSocket,
        session: NoiseSession,
        rx: Rc<RefCell<UnboundedReceiver<Vec<u8>>>>,
        tx: UnboundedSender<Vec<u8>>,
    ) -> Self {
        Self {
            ws,
            session,
            rx,
            tx,
        }
    }

    fn init_websocket(ws_url: &str) -> Result<WebSocketChannels> {
        let ws = WebSocket::new(ws_url)
            .map_err(|e| TransportError::ConnectionFailed(format!("{:?}", e)))?;
        ws.set_binary_type(BinaryType::Arraybuffer);

        let (tx, rx) = unbounded::<Vec<u8>>();
        let rx = Rc::new(RefCell::new(rx));

        let tx_clone = tx.clone();
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let uint8_array = Uint8Array::new(&array_buffer);
                let mut data = vec![0u8; uint8_array.length() as usize];
                uint8_array.copy_to(&mut data);
                let _ = tx_clone.unbounded_send(data);
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

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

        Ok((ws, rx, tx))
    }

    fn send_frame(ws: &WebSocket, payload: &[u8], context: &str) -> Result<()> {
        let uint8_array = Uint8Array::from(payload);
        ws.send_with_array_buffer(&uint8_array.buffer())
            .map_err(|e| {
                TransportError::HandshakeFailed(format!("Failed to send {}: {:?}", context, e))
            })
    }

    fn send_pattern_byte(ws: &WebSocket, pattern: NoisePattern) -> Result<()> {
        Self::send_frame(ws, &[negotiation_byte(pattern)], "pattern byte")
    }

    /// Connect to a WebSocket endpoint and perform Noise handshake
    ///
    /// This performs the full IK handshake:
    /// 1. Client sends `-> e, es, s, ss` (includes identity payload)
    /// 2. Server responds with `<- e, ee, se` (completes handshake)
    /// 3. Channel is ready for encrypted transport
    pub async fn connect<R: RingKeyProvider>(
        ws_url: &str,
        client: &NoiseClient<R>,
        server_static_pub: &[u8; 32],
    ) -> Result<Self> {
        let (ws, rx, tx) = Self::init_websocket(ws_url)?;
        Self::wait_for_open(&ws).await?;

        let session = Self::perform_client_handshake(&ws, &rx, client, server_static_pub).await?;

        Ok(Self::from_parts(ws, session, rx, tx))
    }

    /// Connect using IK-raw pattern (cold key scenario).
    pub async fn connect_ik_raw(
        ws_url: &str,
        x25519_sk: &Zeroizing<[u8; 32]>,
        server_static_pub: &[u8; 32],
    ) -> Result<Self> {
        let (ws, rx, tx) = Self::init_websocket(ws_url)?;
        Self::wait_for_open(&ws).await?;
        Self::send_pattern_byte(&ws, NoisePattern::IKRaw)?;
        let session =
            Self::perform_client_handshake_ik_raw(&ws, &rx, x25519_sk, server_static_pub).await?;
        Ok(Self::from_parts(ws, session, rx, tx))
    }

    /// Connect using N pattern (anonymous client, authenticated server).
    pub async fn connect_anonymous(ws_url: &str, server_static_pub: &[u8; 32]) -> Result<Self> {
        let (ws, rx, tx) = Self::init_websocket(ws_url)?;
        Self::wait_for_open(&ws).await?;
        Self::send_pattern_byte(&ws, NoisePattern::N)?;
        let session = Self::perform_client_handshake_n(&ws, server_static_pub).await?;
        Ok(Self::from_parts(ws, session, rx, tx))
    }

    /// Connect using NN pattern (fully anonymous).
    ///
    /// Returns the channel and the server's ephemeral public key for
    /// post-handshake attestation.
    pub async fn connect_ephemeral(ws_url: &str) -> Result<(Self, [u8; 32], [u8; 32])> {
        let (ws, rx, tx) = Self::init_websocket(ws_url)?;
        Self::wait_for_open(&ws).await?;
        Self::send_pattern_byte(&ws, NoisePattern::NN)?;
        let (session, server_ephemeral, client_ephemeral) =
            Self::perform_client_handshake_nn(&ws, &rx).await?;
        Ok((
            Self::from_parts(ws, session, rx, tx),
            server_ephemeral,
            client_ephemeral,
        ))
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
        client: &NoiseClient<R>,
        server_static_pub: &[u8; 32],
    ) -> Result<NoiseSession> {
        // 1. Build the IK handshake initiation message
        let (hs, first_msg) =
            pubky_noise::datalink_adapter::client_start_ik_direct(client, server_static_pub)
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
        let session =
            pubky_noise::datalink_adapter::client_complete_ik(hs, &response).map_err(|e| {
                TransportError::HandshakeFailed(format!("Failed to complete handshake: {}", e))
            })?;

        Ok(session)
    }

    async fn perform_client_handshake_ik_raw(
        ws: &WebSocket,
        rx: &Rc<RefCell<UnboundedReceiver<Vec<u8>>>>,
        x25519_sk: &Zeroizing<[u8; 32]>,
        server_static_pub: &[u8; 32],
    ) -> Result<NoiseSession> {
        let (hs, first_msg) = datalink_adapter::start_ik_raw(x25519_sk, server_static_pub)
            .map_err(|e| {
                TransportError::HandshakeFailed(format!("Handshake build failed: {}", e))
            })?;

        Self::send_frame(ws, &first_msg, "handshake")?;

        let response = Self::receive_raw(rx).await.map_err(|e| {
            TransportError::HandshakeFailed(format!("Failed to read handshake response: {}", e))
        })?;

        datalink_adapter::complete_raw(hs, &response).map_err(|e| {
            TransportError::HandshakeFailed(format!("Failed to complete handshake: {}", e))
        })
    }

    async fn perform_client_handshake_n(
        ws: &WebSocket,
        server_static_pub: &[u8; 32],
    ) -> Result<NoiseSession> {
        let (hs, first_msg) = datalink_adapter::start_n(server_static_pub).map_err(|e| {
            TransportError::HandshakeFailed(format!("Handshake build failed: {}", e))
        })?;

        Self::send_frame(ws, &first_msg, "handshake")?;

        datalink_adapter::complete_n(hs).map_err(|e| {
            TransportError::HandshakeFailed(format!("Failed to complete handshake: {}", e))
        })
    }

    async fn perform_client_handshake_nn(
        ws: &WebSocket,
        rx: &Rc<RefCell<UnboundedReceiver<Vec<u8>>>>,
    ) -> Result<(NoiseSession, [u8; 32], [u8; 32])> {
        let (hs, first_msg) = datalink_adapter::start_nn().map_err(|e| {
            TransportError::HandshakeFailed(format!("Handshake build failed: {}", e))
        })?;
        let client_ephemeral: [u8; 32] = first_msg
            .get(..32)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| {
                TransportError::HandshakeFailed("Invalid NN first message length".into())
            })?;

        Self::send_frame(ws, &first_msg, "handshake")?;

        let response = Self::receive_raw(rx).await.map_err(|e| {
            TransportError::HandshakeFailed(format!("Failed to read handshake response: {}", e))
        })?;

        let server_ephemeral: [u8; 32] = response
            .get(..32)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| {
                TransportError::HandshakeFailed(
                    "Invalid response length for NN pattern".to_string(),
                )
            })?;

        let session = datalink_adapter::complete_raw(hs, &response).map_err(|e| {
            TransportError::HandshakeFailed(format!("Failed to complete handshake: {}", e))
        })?;

        Ok((session, server_ephemeral, client_ephemeral))
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
            .session
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
            .session
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

fn negotiation_byte(pattern: NoisePattern) -> u8 {
    match pattern {
        NoisePattern::IK => 0,
        NoisePattern::IKRaw => 1,
        NoisePattern::N => 2,
        NoisePattern::NN => 3,
        NoisePattern::XX => 4,
    }
}

/// WASM-exposed client for initiating payments over WebSocket
#[wasm_bindgen]
pub struct WasmPaymentClient {
    // Will be implemented in Phase 4
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
        Self {}
    }

    /// Connect to a payee and initiate a payment
    /// Returns a promise that resolves with the receipt
    pub async fn pay(
        &self,
        _ws_url: &str,
        _payee_pubkey: &str,
        _amount: &str,
        _currency: &str,
        _method: &str,
    ) -> std::result::Result<JsValue, JsValue> {
        // Will be implemented in Phase 4 - Interactive Payment Integration
        Err(JsValue::from_str(
            "Payment client not yet implemented - Phase 4",
        ))
    }
}

/// WASM-exposed server for receiving payments over WebSocket
#[wasm_bindgen]
pub struct WasmPaymentServer {
    // Will be implemented in Phase 4
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
        Self {}
    }

    /// Start listening for payment requests
    /// Note: In browser, this requires a WebSocket relay server
    pub async fn listen(&self, _port: u16) -> std::result::Result<(), JsValue> {
        // Will be implemented in Phase 4 - Interactive Payment Integration
        Err(JsValue::from_str("Payment server not yet implemented - Phase 4. Browser cannot listen directly; requires WebSocket relay server."))
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
