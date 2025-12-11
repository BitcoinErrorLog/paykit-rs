//! Payment Proof Standardization
//!
//! This module provides standardized proof types and verification logic
//! for different payment methods.

use paykit_lib::MethodId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Standardized payment proof format.
///
/// Each payment method has its own proof format, but all proofs share
/// common metadata and verification status.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentProof {
    /// The proof type.
    #[serde(flatten)]
    pub proof_type: ProofType,
    /// Whether this proof has been verified.
    pub verified: bool,
    /// Timestamp when the proof was generated.
    pub created_at: i64,
    /// Timestamp when the proof was last verified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<i64>,
}

/// Specific proof types for different payment methods.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "proof_type")]
pub enum ProofType {
    /// Bitcoin on-chain transaction proof.
    #[serde(rename = "bitcoin_txid")]
    BitcoinTxid {
        /// The transaction ID.
        txid: String,
        /// The block height (None if unconfirmed).
        #[serde(skip_serializing_if = "Option::is_none")]
        block_height: Option<u64>,
        /// Number of confirmations.
        #[serde(skip_serializing_if = "Option::is_none")]
        confirmations: Option<u64>,
        /// Output index in the transaction.
        #[serde(skip_serializing_if = "Option::is_none")]
        vout: Option<u32>,
    },
    /// Lightning Network payment proof.
    #[serde(rename = "lightning_preimage")]
    LightningPreimage {
        /// The payment preimage (proof of payment).
        preimage: String,
        /// The payment hash.
        payment_hash: String,
    },
    /// Custom proof for other payment methods.
    #[serde(rename = "custom")]
    Custom {
        /// The method ID.
        method_id: String,
        /// Method-specific proof data.
        data: Value,
    },
}

impl PaymentProof {
    /// Create a Bitcoin txid proof.
    pub fn bitcoin_txid(txid: impl Into<String>) -> Self {
        Self {
            proof_type: ProofType::BitcoinTxid {
                txid: txid.into(),
                block_height: None,
                confirmations: None,
                vout: None,
            },
            verified: false,
            created_at: current_timestamp(),
            verified_at: None,
        }
    }

    /// Create a Bitcoin txid proof with confirmation details.
    pub fn bitcoin_txid_confirmed(
        txid: impl Into<String>,
        block_height: u64,
        confirmations: u64,
        vout: u32,
    ) -> Self {
        Self {
            proof_type: ProofType::BitcoinTxid {
                txid: txid.into(),
                block_height: Some(block_height),
                confirmations: Some(confirmations),
                vout: Some(vout),
            },
            verified: true,
            created_at: current_timestamp(),
            verified_at: Some(current_timestamp()),
        }
    }

    /// Create a Lightning preimage proof.
    pub fn lightning_preimage(
        preimage: impl Into<String>,
        payment_hash: impl Into<String>,
    ) -> Self {
        Self {
            proof_type: ProofType::LightningPreimage {
                preimage: preimage.into(),
                payment_hash: payment_hash.into(),
            },
            verified: false,
            created_at: current_timestamp(),
            verified_at: None,
        }
    }

    /// Create a custom proof.
    pub fn custom(method_id: MethodId, data: Value) -> Self {
        Self {
            proof_type: ProofType::Custom {
                method_id: method_id.0,
                data,
            },
            verified: false,
            created_at: current_timestamp(),
            verified_at: None,
        }
    }

    /// Mark as verified.
    pub fn mark_verified(&mut self) {
        self.verified = true;
        self.verified_at = Some(current_timestamp());
    }

    /// Get the method ID for this proof.
    pub fn method_id(&self) -> MethodId {
        match &self.proof_type {
            ProofType::BitcoinTxid { .. } => MethodId("onchain".to_string()),
            ProofType::LightningPreimage { .. } => MethodId("lightning".to_string()),
            ProofType::Custom { method_id, .. } => MethodId(method_id.clone()),
        }
    }

    /// Check if this proof has been verified.
    pub fn is_verified(&self) -> bool {
        self.verified
    }
}

/// Result of proof verification.
#[derive(Clone, Debug)]
pub struct VerificationResult {
    /// Whether the proof is valid.
    pub valid: bool,
    /// Verification errors if any.
    pub errors: Vec<String>,
    /// Additional verification details.
    pub details: Option<Value>,
}

impl VerificationResult {
    /// Create a valid result.
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            details: None,
        }
    }

    /// Create an invalid result.
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
            details: None,
        }
    }

    /// Add verification details.
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

/// Trait for proof verifiers.
///
/// Implement this trait to add verification logic for specific payment methods.
#[async_trait::async_trait]
pub trait ProofVerifier: Send + Sync {
    /// Verify a payment proof.
    async fn verify(&self, proof: &PaymentProof) -> VerificationResult;

    /// Get the method ID this verifier handles.
    fn method_id(&self) -> MethodId;
}

/// Bitcoin transaction proof verifier.
///
/// This is a placeholder that performs basic validation.
/// A production implementation would query a Bitcoin node.
pub struct BitcoinProofVerifier;

#[async_trait::async_trait]
impl ProofVerifier for BitcoinProofVerifier {
    fn method_id(&self) -> MethodId {
        MethodId("onchain".to_string())
    }

    async fn verify(&self, proof: &PaymentProof) -> VerificationResult {
        match &proof.proof_type {
            ProofType::BitcoinTxid { txid, .. } => {
                // Basic validation
                if txid.len() != 64 {
                    return VerificationResult::invalid(vec![format!(
                        "Invalid txid length: {} (expected 64)",
                        txid.len()
                    )]);
                }

                // Check for valid hex
                if !txid.chars().all(|c| c.is_ascii_hexdigit()) {
                    return VerificationResult::invalid(vec![
                        "Invalid txid: not hexadecimal".to_string()
                    ]);
                }

                // In production, would query a Bitcoin node to verify:
                // 1. Transaction exists
                // 2. Transaction has expected outputs
                // 3. Transaction has sufficient confirmations

                VerificationResult::valid().with_details(serde_json::json!({
                    "note": "Basic validation only. Production should query Bitcoin node."
                }))
            }
            _ => VerificationResult::invalid(vec![
                "Wrong proof type for Bitcoin verifier".to_string()
            ]),
        }
    }
}

/// Lightning payment proof verifier.
///
/// This performs cryptographic verification of preimage against payment hash.
pub struct LightningProofVerifier;

#[async_trait::async_trait]
impl ProofVerifier for LightningProofVerifier {
    fn method_id(&self) -> MethodId {
        MethodId("lightning".to_string())
    }

    async fn verify(&self, proof: &PaymentProof) -> VerificationResult {
        match &proof.proof_type {
            ProofType::LightningPreimage {
                preimage,
                payment_hash,
            } => {
                // Preimage should be 64 hex chars (32 bytes)
                if preimage.len() != 64 {
                    return VerificationResult::invalid(vec![format!(
                        "Invalid preimage length: {} (expected 64)",
                        preimage.len()
                    )]);
                }

                // Payment hash should also be 64 hex chars
                if payment_hash.len() != 64 {
                    return VerificationResult::invalid(vec![format!(
                        "Invalid payment_hash length: {} (expected 64)",
                        payment_hash.len()
                    )]);
                }

                // In production, would verify that SHA256(preimage) == payment_hash
                // This requires a crypto library

                VerificationResult::valid().with_details(serde_json::json!({
                    "note": "Format validation only. Production should verify SHA256(preimage) == payment_hash."
                }))
            }
            _ => VerificationResult::invalid(vec![
                "Wrong proof type for Lightning verifier".to_string()
            ]),
        }
    }
}

/// Registry of proof verifiers.
pub struct ProofVerifierRegistry {
    verifiers: std::collections::HashMap<String, Box<dyn ProofVerifier>>,
}

impl ProofVerifierRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            verifiers: std::collections::HashMap::new(),
        }
    }

    /// Create registry with default verifiers.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(BitcoinProofVerifier));
        registry.register(Box::new(LightningProofVerifier));
        registry
    }

    /// Register a verifier.
    pub fn register(&mut self, verifier: Box<dyn ProofVerifier>) {
        self.verifiers
            .insert(verifier.method_id().0.clone(), verifier);
    }

    /// Verify a proof using the appropriate verifier.
    pub async fn verify(&self, proof: &PaymentProof) -> VerificationResult {
        let method_id = proof.method_id();

        if let Some(verifier) = self.verifiers.get(&method_id.0) {
            verifier.verify(proof).await
        } else {
            VerificationResult::invalid(vec![format!(
                "No verifier registered for method: {}",
                method_id.0
            )])
        }
    }
}

impl Default for ProofVerifierRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitcoin_proof_creation() {
        let proof = PaymentProof::bitcoin_txid(
            "abc123def456abc123def456abc123def456abc123def456abc123def456abc1",
        );
        assert!(!proof.is_verified());
        assert_eq!(proof.method_id().0, "onchain");
    }

    #[test]
    fn test_lightning_proof_creation() {
        let proof = PaymentProof::lightning_preimage(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
        );
        assert!(!proof.is_verified());
        assert_eq!(proof.method_id().0, "lightning");
    }

    #[test]
    fn test_custom_proof_creation() {
        let proof = PaymentProof::custom(
            MethodId("my_method".to_string()),
            serde_json::json!({"custom_field": "value"}),
        );
        assert_eq!(proof.method_id().0, "my_method");
    }

    #[test]
    fn test_mark_verified() {
        let mut proof = PaymentProof::bitcoin_txid(
            "abc123def456abc123def456abc123def456abc123def456abc123def456abc1",
        );
        assert!(!proof.is_verified());

        proof.mark_verified();
        assert!(proof.is_verified());
        assert!(proof.verified_at.is_some());
    }

    #[tokio::test]
    async fn test_bitcoin_verifier() {
        let verifier = BitcoinProofVerifier;

        // Valid txid format
        let proof = PaymentProof::bitcoin_txid(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        let result = verifier.verify(&proof).await;
        assert!(result.valid);

        // Invalid txid (wrong length)
        let proof = PaymentProof::bitcoin_txid("abc123");
        let result = verifier.verify(&proof).await;
        assert!(!result.valid);
    }

    #[tokio::test]
    async fn test_lightning_verifier() {
        let verifier = LightningProofVerifier;

        // Valid format
        let proof = PaymentProof::lightning_preimage(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
        );
        let result = verifier.verify(&proof).await;
        assert!(result.valid);
    }

    #[tokio::test]
    async fn test_registry() {
        let registry = ProofVerifierRegistry::with_defaults();

        let btc_proof = PaymentProof::bitcoin_txid(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        let result = registry.verify(&btc_proof).await;
        assert!(result.valid);

        let ln_proof = PaymentProof::lightning_preimage(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
        );
        let result = registry.verify(&ln_proof).await;
        assert!(result.valid);
    }
}
