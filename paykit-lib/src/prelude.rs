//! Prelude module for convenient imports.
//!
//! This module re-exports the most commonly used types and traits for
//! quick setup. Import everything with:
//!
//! ```rust,ignore
//! use paykit_lib::prelude::*;
//! ```
//!
//! ## What's Included
//!
//! - Core types: `MethodId`, `EndpointData`, `SupportedPayments`
//! - Error types: `PaykitError`, `PaykitErrorCode`, `Result`
//! - Transport traits: `AuthenticatedTransport`, `UnauthenticatedTransportRead`
//! - Payment methods: `PaymentMethodPlugin`, `PaymentMethodRegistry`, `Amount`
//! - URI parsing: `PaykitUri`, `parse_uri`

// Core types
pub use crate::{EndpointData, MethodId, SupportedPayments};

// Error handling
pub use crate::errors::{PaykitError, PaykitErrorCode};
pub use crate::Result;

// Transport traits
pub use crate::transport::{AuthenticatedTransport, UnauthenticatedTransportRead};

// URI parsing
pub use crate::uri::{parse_uri, PaykitUri};

// Payment methods
pub use crate::methods::{
    default_registry, testnet_registry, Amount, PaymentExecution, PaymentMethodPlugin,
    PaymentMethodRegistry, PaymentProof, ValidationResult,
};

// Built-in plugins
pub use crate::methods::{LightningPlugin, OnchainPlugin};

// Executor traits
pub use crate::methods::{BitcoinExecutor, LightningExecutor};

// Secure storage
pub use crate::secure_storage::SecureKeyStorage;

// Pubky transport (when available)
#[cfg(feature = "pubky")]
pub use crate::transport::{PubkyAuthenticatedTransport, PubkyUnauthenticatedTransport};

#[cfg(feature = "pubky")]
pub use crate::PublicKey;
