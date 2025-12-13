//! Data models for Paykit demo applications

use pubky::PublicKey;
use serde::{Deserialize, Serialize};

/// A contact in the address book
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
    /// Payment proof (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<serde_json::Value>,
    /// Whether proof has been verified
    #[serde(default)]
    pub proof_verified: bool,
    /// Timestamp when proof was verified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_verified_at: Option<i64>,
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
            proof: None,
            proof_verified: false,
            proof_verified_at: None,
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

    pub fn with_proof(mut self, proof: serde_json::Value) -> Self {
        self.proof = Some(proof);
        self
    }

    pub fn mark_proof_verified(mut self) -> Self {
        self.proof_verified = true;
        self.proof_verified_at = Some(current_timestamp());
        self
    }
}

/// Get current Unix timestamp
pub fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
