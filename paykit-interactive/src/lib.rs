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
    /// @deprecated: Use OfferPrivateEndpoints for multiple methods
    OfferPrivateEndpoint {
        method_id: MethodId,
        endpoint: String,
    },
    /// Offer multiple private endpoints for different payment methods.
    /// Used during handshake negotiation to share available private endpoints.
    OfferPrivateEndpoints { methods: Vec<PrivateEndpointOffer> },
    /// Accept specific private endpoints from an offer.
    /// Used during handshake to indicate which endpoints are accepted.
    AcceptPrivateEndpoints { method_ids: Vec<MethodId> },
    /// Decline private endpoint offer with a reason.
    /// Used during handshake to reject endpoint offers.
    DeclinePrivateEndpoints { reason: String },
    /// Payer requests a receipt for a payment they intend to make or have made.
    RequestReceipt { provisional_receipt: PaykitReceipt },
    /// Payee confirms the receipt, potentially adding more metadata/signatures.
    ConfirmReceipt { receipt: PaykitReceipt },
    /// Acknowledge receipt of a message.
    Ack,
    /// Error reporting.
    Error { code: String, message: String },
}

/// Private endpoint offer with optional expiration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivateEndpointOffer {
    /// The payment method ID (e.g., "lightning", "onchain").
    pub method_id: MethodId,
    /// The endpoint data (e.g., Bitcoin address, Lightning invoice).
    pub endpoint: String,
    /// Optional expiration timestamp (unix epoch seconds).
    pub expires_at: Option<i64>,
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

pub mod connection_limit;
pub mod manager;
pub mod metadata;
pub mod metrics;
pub mod proof;
pub mod rate_limit;
pub mod status;
pub mod storage;
pub mod transport;

pub use manager::{PaykitInteractiveManager, ReceiptGenerator};
pub use metadata::{
    MetadataItem, MetadataValidator, OrderMetadata, PaymentMetadata, ShippingMetadata, TaxMetadata,
};
pub use proof::{
    PaymentProof, ProofType, ProofVerifier, ProofVerifierRegistry, VerificationResult,
};
pub use status::{PaymentStatus, PaymentStatusInfo, PaymentStatusTracker};
pub use storage::{
    smart_checkout, smart_checkout_all_methods, smart_checkout_detailed, CheckoutResult,
    PaykitStorage, StorageAdapter,
};

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
