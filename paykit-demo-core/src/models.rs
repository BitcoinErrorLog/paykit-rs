//! Data models for Paykit demo applications
//!
//! This module defines the core data structures used across demo applications.
//! These are simplified wrappers around protocol types for ease of use.
//!
//! # Models
//!
//! - [`Contact`] - Address book entry with public key and metadata
//! - [`PaymentMethod`] - Payment endpoint (Lightning, onchain, etc.)
//! - [`Receipt`] - Payment proof/record
//!
//! # Examples
//!
//! ## Creating a contact
//!
//! ```
//! use paykit_demo_core::Contact;
//! use pubky::Keypair;
//!
//! let keypair = Keypair::random();
//! let contact = Contact::new(keypair.public_key(), "Alice".to_string())
//!     .with_notes("Met at Bitcoin conference".to_string());
//!
//! println!("Contact: {} ({})", contact.name, contact.pubky_uri());
//! ```
//!
//! ## Creating a payment method
//!
//! ```
//! use paykit_demo_core::PaymentMethod;
//!
//! let method = PaymentMethod::new(
//!     "lightning".to_string(),
//!     "lnbc1...invoice".to_string(),
//!     true  // public
//! );
//! ```
//!
//! ## Creating a receipt
//!
//! ```
//! use paykit_demo_core::Receipt;
//! use pubky::Keypair;
//!
//! let payer = Keypair::random().public_key();
//! let payee = Keypair::random().public_key();
//!
//! let receipt = Receipt::new(
//!     "receipt_001".to_string(),
//!     payer,
//!     payee,
//!     "lightning".to_string(),
//! )
//! .with_amount("1000".to_string(), "SAT".to_string())
//! .with_metadata(serde_json::json!({
//!     "order_id": "ORDER-123",
//!     "item": "Digital download"
//! }));
//! ```

use pubky::PublicKey;
use serde::{Deserialize, Serialize};

/// A contact in the address book
///
/// Represents a person or service that you may send payments to.
///
/// # Examples
///
/// ```
/// use paykit_demo_core::Contact;
/// use pubky::Keypair;
///
/// let keypair = Keypair::random();
/// let contact = Contact::new(keypair.public_key(), "Bob's Coffee Shop".to_string())
///     .with_notes("Lightning payments only".to_string());
///
/// assert_eq!(contact.name, "Bob's Coffee Shop");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// Contact's public key
    pub public_key: PublicKey,
    /// Human-readable name
    pub name: String,
    /// Optional notes
    pub notes: Option<String>,
    /// Timestamp when added
    pub added_at: i64,
}

impl Contact {
    pub fn new(public_key: PublicKey, name: String) -> Self {
        Self {
            public_key,
            name,
            notes: None,
            added_at: current_timestamp(),
        }
    }

    pub fn with_notes(mut self, notes: String) -> Self {
        self.notes = Some(notes);
        self
    }

    pub fn pubky_uri(&self) -> String {
        format!("pubky://{}", self.public_key)
    }
}

/// A payment method (onchain, lightning, etc.)
///
/// Represents a way to receive payments - could be a Bitcoin address,
/// Lightning invoice, or other payment endpoint.
///
/// # Examples
///
/// ```
/// use paykit_demo_core::PaymentMethod;
///
/// // Public Lightning method
/// let lightning = PaymentMethod::new(
///     "lightning".to_string(),
///     "lnbc1...".to_string(),
///     true
/// );
///
/// // Private onchain method (shared only in Noise channels)
/// let onchain = PaymentMethod::new(
///     "onchain".to_string(),
///     "bc1q...".to_string(),
///     false
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethod {
    /// Method ID (e.g., "onchain", "lightning")
    pub method_id: String,
    /// Endpoint data (address, invoice, etc.)
    pub endpoint: String,
    /// Whether this is public or private
    pub is_public: bool,
    /// Timestamp when created
    pub created_at: i64,
}

impl PaymentMethod {
    pub fn new(method_id: String, endpoint: String, is_public: bool) -> Self {
        Self {
            method_id,
            endpoint,
            is_public,
            created_at: current_timestamp(),
        }
    }
}

/// A payment receipt
///
/// Cryptographic proof that a payment was coordinated between payer and payee.
/// Note that this is proof of coordination, not proof of on-chain settlement.
///
/// # Examples
///
/// ```
/// use paykit_demo_core::Receipt;
/// use pubky::Keypair;
///
/// let payer = Keypair::random().public_key();
/// let payee = Keypair::random().public_key();
///
/// let receipt = Receipt::new(
///     "receipt_abc123".to_string(),
///     payer,
///     payee,
///     "lightning".to_string(),
/// )
/// .with_amount("1000".to_string(), "SAT".to_string());
///
/// println!("Receipt {} for {} SAT", receipt.id, receipt.amount.unwrap());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// Unique receipt ID
    pub id: String,
    /// Payer's public key
    pub payer: PublicKey,
    /// Payee's public key
    pub payee: PublicKey,
    /// Payment method used
    pub method: String,
    /// Amount (optional)
    pub amount: Option<String>,
    /// Currency (optional)
    pub currency: Option<String>,
    /// Timestamp
    pub timestamp: i64,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl Receipt {
    pub fn new(id: String, payer: PublicKey, payee: PublicKey, method: String) -> Self {
        Self {
            id,
            payer,
            payee,
            method,
            amount: None,
            currency: None,
            timestamp: current_timestamp(),
            metadata: serde_json::json!({}),
        }
    }

    pub fn with_amount(mut self, amount: String, currency: String) -> Self {
        self.amount = Some(amount);
        self.currency = Some(currency);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Get current Unix timestamp
///
/// Returns the number of seconds since the Unix epoch.
/// In the extremely unlikely case of system time being before epoch,
/// returns 0 as a safe fallback.
pub fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0) // Safe fallback if system time is before epoch (extremely unlikely)
}
