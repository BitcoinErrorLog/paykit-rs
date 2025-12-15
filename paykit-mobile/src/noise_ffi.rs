//! Noise Protocol FFI Support
//!
//! This module provides FFI-safe wrappers for Noise protocol operations,
//! enabling mobile applications to perform encrypted peer-to-peer payments.
//!
//! # Architecture
//!
//! The noise FFI layer follows the "cold pkarr, hot noise" key model:
//! - Ed25519 (pkarr) keys are "cold" and managed by Pubky Ring (separate app)
//! - X25519 (Noise) keys are "hot" and cached locally in Paykit apps
//!
//! # Usage Flow
//!
//! ## Client Mode (Sending Payments)
//!
//! 1. Discover recipient's noise endpoint via `discover_noise_endpoint()`
//! 2. Connect to recipient's noise server
//! 3. Perform handshake using cached X25519 keys
//! 4. Send payment request over encrypted channel
//! 5. Receive receipt confirmation
//!
//! ## Server Mode (Receiving Payments)
//!
//! 1. Create a noise server manager via `create_noise_server()`
//! 2. Publish noise endpoint to directory
//! 3. Accept incoming connections
//! 4. Handle payment requests and send confirmations

use std::sync::Arc;

use crate::{PaykitMobileError, Result};

/// Get current unix timestamp
fn now_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ============================================================================
// Noise Endpoint Types
// ============================================================================

/// Information about a Noise protocol endpoint for receiving payments.
///
/// This is discovered from a recipient's public directory and contains
/// the connection information needed to establish a Noise session.
#[derive(Clone, Debug, uniffi::Record)]
pub struct NoiseEndpointInfo {
    /// The recipient's public key (z-base32 encoded).
    pub recipient_pubkey: String,
    /// Host address of the Noise server (IP or hostname).
    pub host: String,
    /// Port number of the Noise server.
    pub port: u16,
    /// The server's Noise public key (X25519, hex encoded).
    /// This is needed to verify the server during handshake.
    pub server_noise_pubkey: String,
    /// Optional metadata about the endpoint.
    pub metadata: Option<String>,
}

impl NoiseEndpointInfo {
    /// Get the full connection address (host:port).
    pub fn connection_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Status of a Noise connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum NoiseConnectionStatus {
    /// Not connected.
    Disconnected,
    /// Connecting to server.
    Connecting,
    /// Handshake in progress.
    Handshaking,
    /// Connected and ready for communication.
    Connected,
    /// Connection failed.
    Failed,
}

/// Result of a Noise handshake operation.
#[derive(Clone, Debug, uniffi::Record)]
pub struct NoiseHandshakeResult {
    /// Whether the handshake succeeded.
    pub success: bool,
    /// Session ID for this connection (if successful).
    pub session_id: Option<String>,
    /// Remote peer's public key (z-base32 encoded, if successful).
    pub remote_pubkey: Option<String>,
    /// Error message (if failed).
    pub error: Option<String>,
}

// ============================================================================
// Noise Endpoint Discovery
// ============================================================================

/// Path prefix for Noise endpoints in the directory.
pub const NOISE_ENDPOINT_PATH: &str = "/pub/paykit.app/v0/noise";

/// Discover a Noise endpoint for a recipient.
///
/// Queries the recipient's public directory for their Noise server information.
///
/// # Arguments
///
/// * `transport` - Unauthenticated transport for reading
/// * `recipient_pubkey` - The recipient's public key (z-base32 encoded)
///
/// # Returns
///
/// The noise endpoint info if found, None otherwise.
///
/// # Example
///
/// ```ignore
/// let transport = UnauthenticatedTransportFFI::new_mock();
/// if let Some(endpoint) = discover_noise_endpoint(&transport, "8pinxxgqs41...")? {
///     println!("Connecting to {}:{}", endpoint.host, endpoint.port);
///     println!("Server pubkey: {}", endpoint.server_noise_pubkey);
/// }
/// ```
#[uniffi::export]
pub fn discover_noise_endpoint(
    transport: Arc<crate::UnauthenticatedTransportFFI>,
    recipient_pubkey: String,
) -> Result<Option<NoiseEndpointInfo>> {
    // Fetch the noise endpoint from the directory
    let path = NOISE_ENDPOINT_PATH.to_string();
    let content = transport.get(recipient_pubkey.clone(), path)?;

    match content {
        Some(json_str) => {
            // Parse the endpoint JSON
            let info: NoiseEndpointData =
                serde_json::from_str(&json_str).map_err(|e| PaykitMobileError::Serialization {
                    msg: format!("Invalid noise endpoint format: {}", e),
                })?;

            Ok(Some(NoiseEndpointInfo {
                recipient_pubkey,
                host: info.host,
                port: info.port,
                server_noise_pubkey: info.pubkey,
                metadata: info.metadata,
            }))
        }
        None => Ok(None),
    }
}

/// Publish a Noise endpoint to the directory.
///
/// Makes this device discoverable for receiving payments via Noise protocol.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for writing
/// * `host` - Host address where the Noise server is listening
/// * `port` - Port number where the Noise server is listening
/// * `noise_pubkey` - This server's Noise public key (X25519, hex encoded)
/// * `metadata` - Optional metadata about the endpoint
#[uniffi::export]
pub fn publish_noise_endpoint(
    transport: Arc<crate::AuthenticatedTransportFFI>,
    host: String,
    port: u16,
    noise_pubkey: String,
    metadata: Option<String>,
) -> Result<()> {
    let endpoint_data = NoiseEndpointData {
        host,
        port,
        pubkey: noise_pubkey,
        metadata,
    };

    let json =
        serde_json::to_string(&endpoint_data).map_err(|e| PaykitMobileError::Serialization {
            msg: format!("Failed to serialize noise endpoint: {}", e),
        })?;

    transport.put(NOISE_ENDPOINT_PATH.to_string(), json)
}

/// Remove the Noise endpoint from the directory.
///
/// Makes this device no longer discoverable for Noise payments.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for writing
#[uniffi::export]
pub fn remove_noise_endpoint(transport: Arc<crate::AuthenticatedTransportFFI>) -> Result<()> {
    transport.delete(NOISE_ENDPOINT_PATH.to_string())
}

/// Internal struct for serializing/deserializing noise endpoint data.
#[derive(serde::Serialize, serde::Deserialize)]
struct NoiseEndpointData {
    host: String,
    port: u16,
    pubkey: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<String>,
}

// ============================================================================
// Noise Server Configuration
// ============================================================================

/// Configuration for a Noise server (receiving payments).
#[derive(Clone, Debug, uniffi::Record)]
pub struct NoiseServerConfig {
    /// The port to listen on (0 for auto-assign).
    pub port: u16,
    /// Maximum number of concurrent connections.
    pub max_connections: u32,
    /// Connection timeout in seconds.
    pub connection_timeout_secs: u32,
    /// Whether to automatically publish endpoint to directory.
    pub auto_publish: bool,
}

impl Default for NoiseServerConfig {
    fn default() -> Self {
        Self {
            port: 0, // Auto-assign
            max_connections: 10,
            connection_timeout_secs: 30,
            auto_publish: false,
        }
    }
}

/// Create a default noise server configuration.
#[uniffi::export]
pub fn create_noise_server_config() -> NoiseServerConfig {
    NoiseServerConfig::default()
}

/// Create a noise server configuration with a specific port.
#[uniffi::export]
pub fn create_noise_server_config_with_port(port: u16) -> NoiseServerConfig {
    NoiseServerConfig {
        port,
        ..Default::default()
    }
}

// ============================================================================
// Noise Session Types
// ============================================================================

/// Information about an active Noise session.
#[derive(Clone, Debug, uniffi::Record)]
pub struct NoiseSessionInfo {
    /// Unique session identifier.
    pub session_id: String,
    /// Remote peer's public key (z-base32 encoded).
    pub remote_pubkey: String,
    /// When the session was established (unix timestamp).
    pub established_at: i64,
    /// Whether this is an incoming (server) or outgoing (client) session.
    pub is_incoming: bool,
    /// Number of messages sent in this session.
    pub messages_sent: u64,
    /// Number of messages received in this session.
    pub messages_received: u64,
}

/// Status of the Noise server.
#[derive(Clone, Debug, uniffi::Record)]
pub struct NoiseServerStatus {
    /// Whether the server is currently running.
    pub is_running: bool,
    /// The port the server is listening on (if running).
    pub port: Option<u16>,
    /// The server's Noise public key (X25519, hex encoded).
    pub noise_pubkey: String,
    /// Number of active sessions.
    pub active_sessions: u32,
    /// Total connections handled since start.
    pub total_connections: u64,
}

// ============================================================================
// Payment Message Types
// ============================================================================

/// Type of payment message exchanged over Noise channel.
#[derive(Clone, Debug, uniffi::Enum)]
pub enum NoisePaymentMessageType {
    /// Request a receipt for a payment.
    ReceiptRequest,
    /// Confirm receipt of payment.
    ReceiptConfirmation,
    /// Offer a private endpoint.
    PrivateEndpointOffer,
    /// Error response.
    Error,
    /// Ping for connection keep-alive.
    Ping,
    /// Pong response to ping.
    Pong,
}

/// A payment message to send over Noise channel.
#[derive(Clone, Debug, uniffi::Record)]
pub struct NoisePaymentMessage {
    /// Type of the message.
    pub message_type: NoisePaymentMessageType,
    /// JSON payload of the message.
    pub payload_json: String,
}

/// Create a receipt request message.
///
/// # Arguments
///
/// * `receipt_id` - Unique identifier for this receipt
/// * `payer_pubkey` - Payer's public key (z-base32)
/// * `payee_pubkey` - Payee's public key (z-base32)
/// * `method_id` - Payment method identifier
/// * `amount` - Optional payment amount
/// * `currency` - Optional currency code
#[uniffi::export]
pub fn create_receipt_request_message(
    receipt_id: String,
    payer_pubkey: String,
    payee_pubkey: String,
    method_id: String,
    amount: Option<String>,
    currency: Option<String>,
) -> Result<NoisePaymentMessage> {
    let payload = serde_json::json!({
        "type": "request_receipt",
        "receipt_id": receipt_id,
        "payer": payer_pubkey,
        "payee": payee_pubkey,
        "method_id": method_id,
        "amount": amount,
        "currency": currency,
        "created_at": now_timestamp()
    });

    Ok(NoisePaymentMessage {
        message_type: NoisePaymentMessageType::ReceiptRequest,
        payload_json: payload.to_string(),
    })
}

/// Create a receipt confirmation message.
///
/// # Arguments
///
/// * `receipt_id` - The receipt ID being confirmed
/// * `payer_pubkey` - Payer's public key
/// * `payee_pubkey` - Payee's public key
/// * `method_id` - Payment method used
/// * `amount` - Payment amount
/// * `currency` - Currency code
/// * `signature` - Optional signature from payee
#[uniffi::export]
pub fn create_receipt_confirmation_message(
    receipt_id: String,
    payer_pubkey: String,
    payee_pubkey: String,
    method_id: String,
    amount: Option<String>,
    currency: Option<String>,
    signature: Option<String>,
) -> Result<NoisePaymentMessage> {
    let payload = serde_json::json!({
        "type": "confirm_receipt",
        "receipt_id": receipt_id,
        "payer": payer_pubkey,
        "payee": payee_pubkey,
        "method_id": method_id,
        "amount": amount,
        "currency": currency,
        "confirmed_at": now_timestamp(),
        "signature": signature
    });

    Ok(NoisePaymentMessage {
        message_type: NoisePaymentMessageType::ReceiptConfirmation,
        payload_json: payload.to_string(),
    })
}

/// Create a private endpoint offer message.
///
/// # Arguments
///
/// * `method_id` - Payment method identifier
/// * `endpoint` - The private endpoint data
/// * `expires_in_secs` - Optional expiration time in seconds
#[uniffi::export]
pub fn create_private_endpoint_offer_message(
    method_id: String,
    endpoint: String,
    expires_in_secs: Option<u64>,
) -> Result<NoisePaymentMessage> {
    let expires_at = expires_in_secs.map(|secs| now_timestamp() + secs as i64);

    let payload = serde_json::json!({
        "type": "private_endpoint_offer",
        "method_id": method_id,
        "endpoint": endpoint,
        "created_at": now_timestamp(),
        "expires_at": expires_at
    });

    Ok(NoisePaymentMessage {
        message_type: NoisePaymentMessageType::PrivateEndpointOffer,
        payload_json: payload.to_string(),
    })
}

/// Create an error message.
///
/// # Arguments
///
/// * `code` - Error code
/// * `message` - Error description
#[uniffi::export]
pub fn create_error_message(code: String, message: String) -> Result<NoisePaymentMessage> {
    let payload = serde_json::json!({
        "type": "error",
        "code": code,
        "message": message,
        "timestamp": now_timestamp()
    });

    Ok(NoisePaymentMessage {
        message_type: NoisePaymentMessageType::Error,
        payload_json: payload.to_string(),
    })
}

/// Parse a payment message from JSON.
///
/// # Arguments
///
/// * `json` - The JSON string to parse
#[uniffi::export]
pub fn parse_payment_message(json: String) -> Result<NoisePaymentMessage> {
    let value: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| PaykitMobileError::Serialization {
            msg: format!("Invalid message JSON: {}", e),
        })?;

    let msg_type = value
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");

    let message_type = match msg_type {
        "request_receipt" => NoisePaymentMessageType::ReceiptRequest,
        "confirm_receipt" => NoisePaymentMessageType::ReceiptConfirmation,
        "private_endpoint_offer" => NoisePaymentMessageType::PrivateEndpointOffer,
        "error" => NoisePaymentMessageType::Error,
        "ping" => NoisePaymentMessageType::Ping,
        "pong" => NoisePaymentMessageType::Pong,
        _ => {
            return Err(PaykitMobileError::Validation {
                msg: format!("Unknown message type: {}", msg_type),
            })
        }
    };

    Ok(NoisePaymentMessage {
        message_type,
        payload_json: json,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport_ffi::{AuthenticatedTransportFFI, UnauthenticatedTransportFFI};

    #[test]
    fn test_noise_endpoint_info() {
        let info = NoiseEndpointInfo {
            recipient_pubkey: "8pinxxgqs41...".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8888,
            server_noise_pubkey: "abcd1234...".to_string(),
            metadata: None,
        };

        assert_eq!(info.connection_address(), "127.0.0.1:8888");
    }

    #[test]
    fn test_publish_and_discover_noise_endpoint() {
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

        // Publish endpoint
        publish_noise_endpoint(
            auth.clone(),
            "127.0.0.1".to_string(),
            8888,
            "abcd1234".to_string(),
            Some("Test endpoint".to_string()),
        )
        .unwrap();

        // Discover endpoint
        let result = discover_noise_endpoint(unauth, "test_owner".to_string()).unwrap();

        assert!(result.is_some());
        let endpoint = result.unwrap();
        assert_eq!(endpoint.host, "127.0.0.1");
        assert_eq!(endpoint.port, 8888);
        assert_eq!(endpoint.server_noise_pubkey, "abcd1234");
        assert_eq!(endpoint.metadata, Some("Test endpoint".to_string()));
    }

    #[test]
    fn test_remove_noise_endpoint() {
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

        // Publish and remove
        publish_noise_endpoint(
            auth.clone(),
            "127.0.0.1".to_string(),
            8888,
            "abcd1234".to_string(),
            None,
        )
        .unwrap();

        remove_noise_endpoint(auth).unwrap();

        // Verify removed
        let result = discover_noise_endpoint(unauth, "test_owner".to_string()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_noise_server_config() {
        let config = create_noise_server_config();
        assert_eq!(config.port, 0);
        assert_eq!(config.max_connections, 10);
        assert!(!config.auto_publish);

        let config_with_port = create_noise_server_config_with_port(9999);
        assert_eq!(config_with_port.port, 9999);
    }

    #[test]
    fn test_create_receipt_request_message() {
        let msg = create_receipt_request_message(
            "rcpt_123".to_string(),
            "payer_pk".to_string(),
            "payee_pk".to_string(),
            "lightning".to_string(),
            Some("1000".to_string()),
            Some("SAT".to_string()),
        )
        .unwrap();

        assert!(matches!(
            msg.message_type,
            NoisePaymentMessageType::ReceiptRequest
        ));
        assert!(msg.payload_json.contains("request_receipt"));
        assert!(msg.payload_json.contains("rcpt_123"));
    }

    #[test]
    fn test_create_receipt_confirmation_message() {
        let msg = create_receipt_confirmation_message(
            "rcpt_123".to_string(),
            "payer_pk".to_string(),
            "payee_pk".to_string(),
            "lightning".to_string(),
            Some("1000".to_string()),
            Some("SAT".to_string()),
            None,
        )
        .unwrap();

        assert!(matches!(
            msg.message_type,
            NoisePaymentMessageType::ReceiptConfirmation
        ));
        assert!(msg.payload_json.contains("confirm_receipt"));
    }

    #[test]
    fn test_create_private_endpoint_offer_message() {
        let msg = create_private_endpoint_offer_message(
            "lightning".to_string(),
            "lnbc1...".to_string(),
            Some(3600),
        )
        .unwrap();

        assert!(matches!(
            msg.message_type,
            NoisePaymentMessageType::PrivateEndpointOffer
        ));
        assert!(msg.payload_json.contains("private_endpoint_offer"));
        assert!(msg.payload_json.contains("lnbc1..."));
    }

    #[test]
    fn test_create_error_message() {
        let msg = create_error_message(
            "invalid_amount".to_string(),
            "Amount is invalid".to_string(),
        )
        .unwrap();

        assert!(matches!(msg.message_type, NoisePaymentMessageType::Error));
        assert!(msg.payload_json.contains("error"));
        assert!(msg.payload_json.contains("invalid_amount"));
    }

    #[test]
    fn test_parse_payment_message() {
        let json = r#"{"type":"request_receipt","receipt_id":"rcpt_123"}"#;
        let msg = parse_payment_message(json.to_string()).unwrap();

        assert!(matches!(
            msg.message_type,
            NoisePaymentMessageType::ReceiptRequest
        ));
    }

    #[test]
    fn test_parse_unknown_message_type() {
        let json = r#"{"type":"unknown_type"}"#;
        let result = parse_payment_message(json.to_string());

        assert!(result.is_err());
    }
}
