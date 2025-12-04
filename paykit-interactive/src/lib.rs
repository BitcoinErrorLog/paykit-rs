//! Paykit Interactive Layer
//!
//! This crate implements the interactive payment flows and receipt exchange for Paykit,
//! designed to run over encrypted channels (like Pubky Noise).

use paykit_lib::{MethodId, PublicKey};
use serde::{Deserialize, Serialize};

/// A cryptographic receipt shared between two peers for a payment.
///
/// This struct is designed to be forward-compatible with future features like
/// Locks and Atomicity.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaykitReceipt {
    /// Unique identifier for this receipt (e.g., monotonic timestamp + random).
    pub receipt_id: String,
    /// Public key of the entity making the payment.
    pub payer: PublicKey,
    /// Public key of the entity receiving the payment.
    pub payee: PublicKey,
    /// The payment method used (e.g., "lightning", "onchain").
    pub method_id: MethodId,
    /// Optional amount string (e.g., "1000", "0.005").
    /// Optional because some flows might not negotiate amount in the receipt explicitly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    /// Optional currency code (e.g., "SAT", "USD").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Timestamp of creation (unix epoch).
    pub created_at: i64,
    /// Arbitrary metadata (invoice numbers, order IDs, shipping info).
    pub metadata: serde_json::Value,
}

impl PaykitReceipt {
    /// Create a new provisional receipt.
    pub fn new(
        receipt_id: String,
        payer: PublicKey,
        payee: PublicKey,
        method_id: MethodId,
        amount: Option<String>,
        currency: Option<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            receipt_id,
            payer,
            payee,
            method_id,
            amount,
            currency,
            created_at: chrono_now(),
            metadata,
        }
    }
}

fn chrono_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Messages exchanged over the encrypted channel to negotiate payments and receipts.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum PaykitNoiseMessage {
    /// Payer offers a private endpoint to the payee (or vice versa).
    OfferPrivateEndpoint {
        method_id: MethodId,
        endpoint: String,
    },
    /// Payer requests a receipt for a payment they intend to make or have made.
    RequestReceipt { provisional_receipt: PaykitReceipt },
    /// Payee confirms the receipt, potentially adding more metadata/signatures.
    ConfirmReceipt { receipt: PaykitReceipt },
    /// Acknowledge receipt of a message.
    Ack,
    /// Error reporting.
    Error { code: String, message: String },
    /// NN pattern attestation - proves Ed25519 identity after ephemeral handshake.
    ///
    /// Used to authenticate after an NN (fully anonymous) Noise handshake.
    /// The signature binds the Ed25519 identity to the specific session by
    /// signing both ephemeral public keys from the handshake.
    ///
    /// # Security
    /// - Prevents MITM attacks on NN connections
    /// - Binds identity to this specific session (replay protection)
    /// - Should be exchanged immediately after NN handshake completes
    Attestation {
        /// The signer's Ed25519 public key (32 bytes, hex-encoded)
        ed25519_pk: String,
        /// Ed25519 signature over the attestation message (64 bytes, hex-encoded)
        signature: String,
    },
}

/// Abstraction for a secure channel to exchange Paykit messages.
///
/// In the future, this will be implemented by wrapping `pubky-noise`.
#[async_trait::async_trait]
pub trait PaykitNoiseChannel {
    /// Send a message.
    async fn send(&mut self, msg: PaykitNoiseMessage) -> Result<()>;
    /// Receive a message.
    async fn recv(&mut self) -> Result<PaykitNoiseMessage>;
}

pub mod manager;
pub mod storage;
pub mod transport;

pub use manager::{PaykitInteractiveManager, ReceiptGenerator};
pub use storage::PaykitStorage;
pub use transport::PubkyNoiseChannel;

// Re-export pubky-noise types for pattern selection
pub use pubky_noise::NoisePattern;
pub use zeroize::Zeroizing;

/// Result type for interactive operations.
pub type Result<T> = std::result::Result<T, InteractiveError>;

#[derive(thiserror::Error, Debug)]
pub enum InteractiveError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("not implemented")]
    Unimplemented,
    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<serde_json::Error> for InteractiveError {
    fn from(e: serde_json::Error) -> Self {
        InteractiveError::Serialization(e.to_string())
    }
}

// We need to check if paykit-lib errors need mapping, or just use strings.
// paykit-lib isn't used for errors here yet.
