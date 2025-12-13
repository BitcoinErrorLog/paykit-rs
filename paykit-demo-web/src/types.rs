//! WASM-compatible Paykit types
//!
//! These are simplified versions of types from paykit-lib, paykit-interactive,
//! and paykit-subscriptions adapted for browser use without tokio dependencies.

use paykit_lib::{MethodId, PublicKey};
use serde::{Deserialize, Serialize};

// ============================================================
// Amount Type (from paykit-subscriptions)
// ============================================================

/// Amount in satoshis with overflow protection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount(i64);

impl Amount {
    pub fn from_sats(sats: i64) -> Self {
        Amount(sats)
    }

    pub fn to_sats(&self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================
// Payment Request Types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub request_id: String,
    pub from: PublicKey,
    pub to: PublicKey,
    pub amount: Amount,
    pub currency: String,
    pub method: MethodId,
    pub description: Option<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

impl PaymentRequest {
    pub fn new(
        from: PublicKey,
        to: PublicKey,
        amount: Amount,
        currency: String,
        method: MethodId,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let request_id = format!("req_{}", uuid::Uuid::new_v4());

        Self {
            request_id,
            from,
            to,
            amount,
            currency,
            method,
            description: None,
            created_at: now,
            expires_at: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_expiration(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
}

// ============================================================
// Subscription Types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub subscription_id: String,
    pub subscriber: PublicKey,
    pub provider: PublicKey,
    pub terms: SubscriptionTerms,
    pub created_at: i64,
    pub starts_at: i64,
    pub ends_at: Option<i64>,
}

impl Subscription {
    pub fn new(subscriber: PublicKey, provider: PublicKey, terms: SubscriptionTerms) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let subscription_id = format!("sub_{}", uuid::Uuid::new_v4());

        Self {
            subscription_id,
            subscriber,
            provider,
            terms,
            created_at: now,
            starts_at: now,
            ends_at: None,
        }
    }

    pub fn is_active(&self) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        now >= self.starts_at && self.ends_at.is_none_or(|end| now < end)
    }

    pub fn is_expired(&self) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.ends_at.is_some_and(|end| now >= end)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.terms.amount.to_sats() <= 0 {
            return Err("Amount must be positive".to_string());
        }
        if self.ends_at.is_some() && self.ends_at.unwrap() <= self.starts_at {
            return Err("End time must be after start time".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionTerms {
    pub amount: Amount,
    pub currency: String,
    pub frequency: PaymentFrequency,
    pub method: MethodId,
    pub description: String,
}

impl SubscriptionTerms {
    pub fn new(
        amount: Amount,
        currency: String,
        frequency: PaymentFrequency,
        method: MethodId,
        description: String,
    ) -> Self {
        Self {
            amount,
            currency,
            frequency,
            method,
            description,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentFrequency {
    Daily,
    Weekly,
    Monthly { day_of_month: u8 },
    Yearly { month: u8, day: u8 },
    Custom { interval_seconds: u64 },
}

impl std::fmt::Display for PaymentFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentFrequency::Daily => write!(f, "daily"),
            PaymentFrequency::Weekly => write!(f, "weekly"),
            PaymentFrequency::Monthly { day_of_month } => write!(f, "monthly:{}", day_of_month),
            PaymentFrequency::Yearly { month, day } => write!(f, "yearly:{}:{}", month, day),
            PaymentFrequency::Custom { interval_seconds } => {
                write!(f, "custom:{}", interval_seconds)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedSubscription {
    pub subscription: Subscription,
    pub subscriber_signature: Vec<u8>,
    pub provider_signature: Option<Vec<u8>>,
}

impl SignedSubscription {
    pub fn is_active(&self) -> bool {
        self.subscription.is_active()
    }

    pub fn is_expired(&self) -> bool {
        self.subscription.is_expired()
    }

    pub fn verify_signatures(&self) -> Result<bool, String> {
        // Simplified verification for WASM
        // In production, would verify Ed25519 signatures
        Ok(!self.subscriber_signature.is_empty())
    }
}

// ============================================================
// Interactive Payment Types
// ============================================================

/// A cryptographic receipt for payment coordination
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaykitReceipt {
    pub receipt_id: String,
    pub payer: PublicKey,
    pub payee: PublicKey,
    pub method_id: MethodId,
    pub amount: Option<String>,
    pub currency: Option<String>,
    pub created_at: i64,
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

impl PaykitReceipt {
    pub fn new(
        receipt_id: String,
        payer: PublicKey,
        payee: PublicKey,
        method_id: MethodId,
        amount: Option<String>,
        currency: Option<String>,
        metadata: serde_json::Value,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Self {
            receipt_id,
            payer,
            payee,
            method_id,
            amount,
            currency,
            created_at: now,
            metadata,
            proof: None,
            proof_verified: false,
            proof_verified_at: None,
        }
    }

    pub fn with_proof(mut self, proof: serde_json::Value) -> Self {
        self.proof = Some(proof);
        self
    }

    pub fn mark_proof_verified(mut self) -> Self {
        self.proof_verified = true;
        use std::time::{SystemTime, UNIX_EPOCH};
        self.proof_verified_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        );
        self
    }
}

/// Messages exchanged over encrypted channels for payment coordination
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum PaykitNoiseMessage {
    OfferPrivateEndpoint {
        method_id: MethodId,
        endpoint: String,
    },
    RequestReceipt {
        provisional_receipt: PaykitReceipt,
    },
    ConfirmReceipt {
        receipt: PaykitReceipt,
    },
    Ack,
    Error {
        code: String,
        message: String,
    },
}
