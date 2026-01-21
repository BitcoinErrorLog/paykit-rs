//! Encrypted ACK protocol for Paykit.
//!
//! Per PUBKY_CRYPTO_SPEC v2.5, ACKs are encrypted messages sent by a recipient
//! back to the original sender to confirm message delivery.
//!
//! # ACK Flow
//!
//! 1. Alice sends an encrypted payment request to Bob (stored on Alice's homeserver)
//! 2. Bob reads the request and creates an encrypted ACK
//! 3. Bob stores the ACK on his own homeserver at the ACK path
//! 4. Alice polls Bob's homeserver for ACKs
//!
//! # Storage Paths
//!
//! ACKs are stored at: `/pub/paykit.app/v0/acks/{object_type}/{context_id}/{msg_id}`
//!
//! Where:
//! - `object_type`: The type of message being ACKed (e.g., "request", "subscription")
//! - `context_id`: Derived from the sender/recipient pair (legacy) or random ContextId
//! - `msg_id`: The original message's identifier
//!
//! # ACK Message Format
//!
//! The ACK payload is JSON-encoded before encryption:
//!
//! ```json
//! {
//!   "ack_id": "unique-ack-id",
//!   "object_type": "request",
//!   "msg_id": "original-message-id",
//!   "status": "received",
//!   "timestamp": 1700000000,
//!   "payload": null
//! }
//! ```
//!
//! # Encryption Target
//!
//! ACKs are encrypted to the **original sender's** InboxKey, allowing the sender
//! to decrypt and verify receipt.
//!
//! # Resend Defaults
//!
//! - ACKs are stored for 7 days (604800 seconds) by default
//! - Senders should poll for ACKs with exponential backoff
//! - If no ACK is received within the retry window, the message may be resent

use crate::Result;
use serde::{Deserialize, Serialize};

#[cfg(feature = "pubky")]
use super::sb2::{
    sb2_decrypt_verified, sb2_encrypt_signed, AppCertFetcher, Sb2EncryptParams, Sb2Signer,
    SignatureRequirement,
};

/// Default ACK retention time in seconds (7 days).
pub const DEFAULT_ACK_RETENTION_SECS: u64 = 604800;

/// Default initial retry delay in seconds.
pub const DEFAULT_RETRY_DELAY_SECS: u64 = 60;

/// Maximum retry attempts before giving up.
pub const MAX_RETRY_ATTEMPTS: u32 = 5;

/// ACK status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AckStatus {
    /// Message was received and processed successfully.
    Received,
    /// Message was received but processing failed.
    Failed,
    /// Message was received but rejected (e.g., policy violation).
    Rejected,
    /// Message is pending further action.
    Pending,
}

impl std::fmt::Display for AckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AckStatus::Received => write!(f, "received"),
            AckStatus::Failed => write!(f, "failed"),
            AckStatus::Rejected => write!(f, "rejected"),
            AckStatus::Pending => write!(f, "pending"),
        }
    }
}

/// Object types that can be ACKed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AckObjectType {
    /// Payment request.
    Request,
    /// Subscription proposal.
    Subscription,
    /// Settlement message (Atomicity).
    Settlement,
}

impl std::fmt::Display for AckObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AckObjectType::Request => write!(f, "request"),
            AckObjectType::Subscription => write!(f, "subscription"),
            AckObjectType::Settlement => write!(f, "settlement"),
        }
    }
}

impl AckObjectType {
    /// Convert from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "request" => Some(AckObjectType::Request),
            "subscription" => Some(AckObjectType::Subscription),
            "settlement" => Some(AckObjectType::Settlement),
            _ => None,
        }
    }

    /// Get the path component for this object type.
    pub fn as_path_component(&self) -> &'static str {
        match self {
            AckObjectType::Request => "request",
            AckObjectType::Subscription => "subscription",
            AckObjectType::Settlement => "settlement",
        }
    }
}

/// ACK message payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckMessage {
    /// Unique ACK identifier.
    pub ack_id: String,
    /// Type of object being ACKed.
    pub object_type: AckObjectType,
    /// Original message's identifier.
    pub msg_id: String,
    /// ACK status.
    pub status: AckStatus,
    /// Unix timestamp (seconds) when the ACK was created.
    pub timestamp: u64,
    /// Optional additional payload (e.g., error details).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

impl AckMessage {
    /// Create a new ACK message.
    pub fn new(object_type: AckObjectType, msg_id: impl Into<String>, status: AckStatus) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            ack_id: generate_ack_id(),
            object_type,
            msg_id: msg_id.into(),
            status,
            timestamp,
            payload: None,
        }
    }

    /// Create a "received" ACK.
    pub fn received(object_type: AckObjectType, msg_id: impl Into<String>) -> Self {
        Self::new(object_type, msg_id, AckStatus::Received)
    }

    /// Create a "failed" ACK with an error message.
    pub fn failed(
        object_type: AckObjectType,
        msg_id: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        let mut ack = Self::new(object_type, msg_id, AckStatus::Failed);
        ack.payload = Some(serde_json::json!({ "error": error.into() }));
        ack
    }

    /// Create a "rejected" ACK with a reason.
    pub fn rejected(
        object_type: AckObjectType,
        msg_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let mut ack = Self::new(object_type, msg_id, AckStatus::Rejected);
        ack.payload = Some(serde_json::json!({ "reason": reason.into() }));
        ack
    }

    /// Serialize to JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| crate::PaykitError::InvalidData {
            field: "ack_message".into(),
            reason: format!("Failed to serialize ACK: {}", e),
        })
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data).map_err(|e| crate::PaykitError::InvalidData {
            field: "ack_message".into(),
            reason: format!("Failed to deserialize ACK: {}", e),
        })
    }
}

/// Generate a unique ACK identifier.
fn generate_ack_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("ack_{}", hex::encode(bytes))
}

/// Retry configuration for ACK polling.
#[derive(Debug, Clone)]
pub struct AckRetryConfig {
    /// Initial delay between retries in seconds.
    pub initial_delay_secs: u64,
    /// Maximum delay between retries in seconds.
    pub max_delay_secs: u64,
    /// Maximum number of retry attempts.
    pub max_attempts: u32,
    /// Exponential backoff multiplier.
    pub backoff_multiplier: f64,
}

impl Default for AckRetryConfig {
    fn default() -> Self {
        Self {
            initial_delay_secs: DEFAULT_RETRY_DELAY_SECS,
            max_delay_secs: 3600, // 1 hour max
            max_attempts: MAX_RETRY_ATTEMPTS,
            backoff_multiplier: 2.0,
        }
    }
}

impl AckRetryConfig {
    /// Calculate the delay for a given attempt number (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay = (self.initial_delay_secs as f64 * self.backoff_multiplier.powi(attempt as i32))
            as u64;
        delay.min(self.max_delay_secs)
    }

    /// Check if we should retry based on attempt count.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

/// Parameters for creating an encrypted ACK.
#[cfg(feature = "pubky")]
#[derive(Debug, Clone)]
pub struct EncryptedAckParams {
    /// Recipient's InboxKey public key (the original sender who should decrypt)
    pub recipient_inbox_pk: [u8; 32],
    /// Owner's PKARR peer ID (32 bytes)
    pub owner_peerid: [u8; 32],
    /// Sender's PKARR peer ID (this ACK creator)
    pub sender_peerid: [u8; 32],
    /// Recipient's PKARR peer ID (original message sender)
    pub recipient_peerid: [u8; 32],
    /// Context ID (32 bytes) - from original message or random
    pub context_id: [u8; 32],
}

/// Create an encrypted ACK message using SB2 format with signature.
///
/// The ACK is encrypted to the original sender's InboxKey, allowing them
/// to verify receipt of their message. Per PUBKY_CRYPTO_SPEC v2.5, all
/// Paykit protocol messages (including ACKs) MUST be signed.
///
/// # Arguments
///
/// * `ack` - The ACK message to encrypt
/// * `params` - Encryption parameters including keys and context
/// * `signer` - Signing capability (RootKey or AppKey via `Sb2Signer` trait)
///
/// # Returns
///
/// Signed SB2-encrypted binary blob suitable for storage
#[cfg(feature = "pubky")]
pub fn encrypt_ack(
    ack: &AckMessage,
    params: &EncryptedAckParams,
    signer: &dyn Sb2Signer,
) -> Result<Vec<u8>> {
    use super::paths::ack_path_with_context_id;

    let plaintext = ack.to_bytes()?;
    let context_id_hex = hex::encode(params.context_id);
    let canonical_path = ack_path_with_context_id(
        ack.object_type.as_path_component(),
        &context_id_hex,
        &ack.msg_id,
    );

    let sb2_params = Sb2EncryptParams {
        recipient_inbox_pk: params.recipient_inbox_pk,
        owner_peerid: params.owner_peerid,
        sender_peerid: params.sender_peerid,
        recipient_peerid: params.recipient_peerid,
        context_id: params.context_id,
        canonical_path,
        msg_id: ack.ack_id.clone(),
        purpose: Some("ack".to_string()),
        expires_at: Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                + DEFAULT_ACK_RETENTION_SECS,
        ),
    };

    sb2_encrypt_signed(&plaintext, &sb2_params, signer)
}

/// Decrypt an encrypted ACK message with signature verification.
///
/// Per PUBKY_CRYPTO_SPEC v2.5, all Paykit protocol messages (including ACKs)
/// MUST have valid signatures. This function rejects ACKs with missing or
/// invalid signatures.
///
/// # Arguments
///
/// * `data` - The encrypted SB2 blob
/// * `inbox_secret_key` - The recipient's InboxKey secret key
/// * `owner_peerid` - The owner's PKARR peer ID
/// * `canonical_path` - The storage path for AAD verification
/// * `cert_fetcher` - Optional AppCert fetcher for delegated signatures.
///   Pass `None` to reject delegated signatures (cert_id in header).
///
/// # Returns
///
/// The decrypted ACK message (only if signature is valid)
#[cfg(feature = "pubky")]
pub fn decrypt_ack(
    data: &[u8],
    inbox_secret_key: &[u8; 32],
    owner_peerid: &[u8; 32],
    canonical_path: &str,
    cert_fetcher: Option<&dyn AppCertFetcher>,
) -> Result<AckMessage> {
    let (plaintext, _metadata) = sb2_decrypt_verified(
        data,
        inbox_secret_key,
        owner_peerid,
        canonical_path,
        SignatureRequirement::Required,
        cert_fetcher,
    )?;
    AckMessage::from_bytes(&plaintext)
}

/// Pending ACK entry for tracking unacknowledged messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAck {
    /// The message ID we're waiting for an ACK on.
    pub msg_id: String,
    /// Object type of the original message.
    pub object_type: AckObjectType,
    /// Context ID (hex-encoded).
    pub context_id_hex: String,
    /// Recipient's PKARR peer ID (hex-encoded).
    pub recipient_peerid_hex: String,
    /// Unix timestamp when the message was sent.
    pub sent_at: u64,
    /// Number of retry attempts made.
    pub retry_count: u32,
    /// Unix timestamp of next retry.
    pub next_retry_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack_message_serialization() {
        let ack = AckMessage::received(AckObjectType::Request, "req-123");

        let bytes = ack.to_bytes().unwrap();
        let parsed = AckMessage::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.msg_id, "req-123");
        assert_eq!(parsed.status, AckStatus::Received);
        assert_eq!(parsed.object_type, AckObjectType::Request);
    }

    #[test]
    fn ack_message_with_payload() {
        let ack = AckMessage::failed(AckObjectType::Request, "req-456", "Invalid amount");

        let bytes = ack.to_bytes().unwrap();
        let parsed = AckMessage::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.status, AckStatus::Failed);
        assert!(parsed.payload.is_some());
        let payload = parsed.payload.unwrap();
        assert_eq!(payload["error"], "Invalid amount");
    }

    #[test]
    fn ack_id_is_unique() {
        let ack1 = AckMessage::received(AckObjectType::Request, "req-1");
        let ack2 = AckMessage::received(AckObjectType::Request, "req-2");

        assert_ne!(ack1.ack_id, ack2.ack_id);
        assert!(ack1.ack_id.starts_with("ack_"));
    }

    #[test]
    fn retry_config_backoff() {
        let config = AckRetryConfig::default();

        assert_eq!(config.delay_for_attempt(0), 60);
        assert_eq!(config.delay_for_attempt(1), 120);
        assert_eq!(config.delay_for_attempt(2), 240);
        assert_eq!(config.delay_for_attempt(3), 480);
    }

    #[test]
    fn retry_config_max_delay() {
        let config = AckRetryConfig {
            max_delay_secs: 300,
            ..Default::default()
        };

        // Should cap at 300
        assert_eq!(config.delay_for_attempt(10), 300);
    }

    #[test]
    fn retry_config_should_retry() {
        let config = AckRetryConfig {
            max_attempts: 3,
            ..Default::default()
        };

        assert!(config.should_retry(0));
        assert!(config.should_retry(1));
        assert!(config.should_retry(2));
        assert!(!config.should_retry(3));
    }

    #[test]
    fn ack_object_type_conversion() {
        assert_eq!(
            AckObjectType::from_str("request"),
            Some(AckObjectType::Request)
        );
        assert_eq!(
            AckObjectType::from_str("subscription"),
            Some(AckObjectType::Subscription)
        );
        assert_eq!(
            AckObjectType::from_str("settlement"),
            Some(AckObjectType::Settlement)
        );
        assert_eq!(AckObjectType::from_str("unknown"), None);
    }

    #[test]
    fn ack_status_display() {
        assert_eq!(AckStatus::Received.to_string(), "received");
        assert_eq!(AckStatus::Failed.to_string(), "failed");
        assert_eq!(AckStatus::Rejected.to_string(), "rejected");
        assert_eq!(AckStatus::Pending.to_string(), "pending");
    }
}
