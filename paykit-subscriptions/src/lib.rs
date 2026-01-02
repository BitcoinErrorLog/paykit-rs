//! # Paykit Subscriptions Protocol
//!
//! ## Security Model (v0.2.0)
//!
//! This crate handles cryptographic signatures and financial transactions.
//! Key security properties:
//! - Deterministic Ed25519 signatures using postcard serialization
//! - Replay protection via nonces and expiration
//! - Atomic spending limit enforcement
//! - Fixed-point decimal arithmetic for financial accuracy
//!
//! ## Breaking Changes in v0.2.0
//! - All amount fields now use `Amount` type instead of `String`
//! - Signature format changed (includes nonce, timestamp, expires_at)
//! - X25519 signing removed (Ed25519 only)
//! - Storage operations now atomic

pub mod amount;
pub mod autopay;
pub mod discovery;
pub mod fallback;
pub mod invoice;
pub mod manager;
pub mod modifications;
pub mod nonce_store;
pub mod proration;
pub mod request;
pub mod signing;
pub mod storage;
pub mod subscription;

// Platform-specific modules
#[cfg(not(target_arch = "wasm32"))]
pub mod monitor;

// WASM storage module removed - future work (see FINAL_SWEEP_REPORT.md)
// #[cfg(target_arch = "wasm32")]
// pub mod storage_wasm;

pub use amount::Amount;
pub use invoice::{
    Invoice, InvoiceFormat, InvoiceItem, ShippingAddress, ShippingInfo, ShippingMethod, TaxInfo,
};
pub use nonce_store::NonceStore;
pub use request::{PaymentRequest, PaymentRequestResponse, RequestNotification, RequestStatus};
pub use storage::{Direction, RequestFilter, ReservationToken, SubscriptionStorage};

// Platform-specific storage implementations
#[cfg(not(target_arch = "wasm32"))]
pub use storage::FileSubscriptionStorage;

// WASM storage implementation (WasmSubscriptionStorage) is future work
// See FINAL_SWEEP_REPORT.md for details
pub use autopay::{AutoPayRule, PeerSpendingLimit};
pub use fallback::{FallbackHandler, FallbackRecord, FallbackStatus, SubscriptionFallbackPolicy};
pub use manager::SubscriptionManager;
pub use modifications::{
    ModificationHistory, ModificationRecord, ModificationRequest, ModificationType, RequestedBy,
    SubscriptionVersion,
};
pub use proration::{ProratedAmount, ProrationCalculator, ProrationDetails, RoundingMode};
pub use signing::{sign_subscription_ed25519, verify_signature_ed25519, Signature};
pub use subscription::{PaymentFrequency, SignedSubscription, Subscription, SubscriptionTerms};

// Re-export subscription discovery functions
pub use discovery::{
    discover_subscription_agreement, discover_subscription_agreements,
    discover_subscription_cancellations, discover_subscription_proposal,
    discover_subscription_proposals, PAYKIT_AGREEMENTS_PATH, PAYKIT_CANCELLATIONS_PATH,
    PAYKIT_PROPOSALS_PATH,
};

// Monitor only available on native platforms
#[cfg(not(target_arch = "wasm32"))]
pub use monitor::SubscriptionMonitor;

pub type Result<T> = anyhow::Result<T>;

#[derive(thiserror::Error, Debug)]
pub enum SubscriptionError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("cryptographic error: {0}")]
    Crypto(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("limit exceeded")]
    LimitExceeded,
    #[error("arithmetic overflow")]
    Overflow,
    #[error("other error: {0}")]
    Other(String),
}
