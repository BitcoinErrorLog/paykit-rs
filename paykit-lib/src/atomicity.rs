//! Atomicity Protocol Settlement Adapter for Paykit.
//!
//! This module provides integration between Paykit and the Atomicity decentralized
//! credit protocol, enabling Paykit to facilitate settlements for credit-based
//! payment networks.
//!
//! # Overview
//!
//! Atomicity is a decentralized P2P credit protocol that allows nodes to issue
//! IOUs and route payments through credit networks. Paykit serves as the
//! settlement layer, providing:
//!
//! 1. **Settlement Message Schemas** - Standardized formats for settlement requests
//! 2. **Proof Verification Hooks** - Integration with payment executors
//! 3. **Executor Integration** - Settlement execution via Lightning/on-chain
//!
//! # Atomicity Specification Reference
//!
//! This implementation follows the Atomicity Specification v1.0:
//! - Section 6: Settlement Protocol
//! - Section 7: Proof of Settlement
//! - Section 8: Replay Prevention
//!
//! # Example
//!
//! ```ignore
//! use paykit_lib::atomicity::{SettlementRequest, SettlementProof};
//!
//! // Create a settlement request
//! let request = SettlementRequest::new(
//!     "iou-123",
//!     1000, // amount in sats
//!     "ln...", // Lightning invoice
//! );
//!
//! // After settlement, create proof
//! let proof = SettlementProof::from_preimage(preimage);
//! ```

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Result;

/// Settlement method supported by Paykit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettlementMethod {
    /// Lightning Network settlement via BOLT11 invoice.
    Lightning,
    /// On-chain Bitcoin settlement.
    Onchain,
}

impl std::fmt::Display for SettlementMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettlementMethod::Lightning => write!(f, "lightning"),
            SettlementMethod::Onchain => write!(f, "onchain"),
        }
    }
}

/// Status of a settlement request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettlementStatus {
    /// Settlement request is pending execution.
    Pending,
    /// Settlement is in progress.
    InProgress,
    /// Settlement completed successfully.
    Completed,
    /// Settlement failed.
    Failed,
    /// Settlement was expired/timed out.
    Expired,
    /// Settlement was cancelled.
    Cancelled,
}

/// A settlement request from Atomicity to Paykit.
///
/// This represents a request to settle a credit IOU using a payment method
/// supported by Paykit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRequest {
    /// Unique identifier for this settlement request.
    pub request_id: String,
    /// Reference to the IOU being settled.
    pub iou_id: String,
    /// Amount in satoshis.
    pub amount_sats: u64,
    /// Settlement method to use.
    pub method: SettlementMethod,
    /// Payment details (invoice for Lightning, address for on-chain).
    pub payment_details: String,
    /// Unix timestamp (seconds) when the request was created.
    pub created_at: u64,
    /// Unix timestamp (seconds) when the request expires.
    pub expires_at: u64,
    /// Nonce for replay prevention (32 bytes, hex-encoded).
    pub nonce: String,
    /// Additional metadata (e.g., memo, routing hints).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl SettlementRequest {
    /// Create a new settlement request.
    pub fn new(
        iou_id: impl Into<String>,
        amount_sats: u64,
        payment_details: impl Into<String>,
        method: SettlementMethod,
    ) -> Self {
        use rand::RngCore;
        let mut nonce_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);

        let now = current_unix_timestamp();

        Self {
            request_id: generate_request_id(),
            iou_id: iou_id.into(),
            amount_sats,
            method,
            payment_details: payment_details.into(),
            created_at: now,
            expires_at: now + 3600, // 1 hour default expiry
            nonce: hex::encode(nonce_bytes),
            metadata: None,
        }
    }

    /// Set the expiry time.
    pub fn with_expiry(mut self, expires_at: u64) -> Self {
        self.expires_at = expires_at;
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Check if the request has expired.
    pub fn is_expired(&self) -> bool {
        current_unix_timestamp() >= self.expires_at
    }

    /// Compute a unique hash of this request for replay prevention.
    pub fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.iou_id.as_bytes());
        hasher.update(&self.amount_sats.to_le_bytes());
        hasher.update(self.payment_details.as_bytes());
        hasher.update(&self.nonce);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Serialize to JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| crate::PaykitError::InvalidData {
            field: "settlement_request".into(),
            reason: format!("Failed to serialize: {}", e),
        })
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data).map_err(|e| crate::PaykitError::InvalidData {
            field: "settlement_request".into(),
            reason: format!("Failed to deserialize: {}", e),
        })
    }
}

/// Proof that a settlement was completed.
///
/// This is returned after successful settlement and can be verified
/// by Atomicity nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementProof {
    /// Reference to the settlement request.
    pub request_id: String,
    /// Reference to the IOU being settled.
    pub iou_id: String,
    /// Settlement method used.
    pub method: SettlementMethod,
    /// Proof data (method-specific).
    pub proof_data: SettlementProofData,
    /// Unix timestamp (seconds) when settlement completed.
    pub settled_at: u64,
}

/// Method-specific settlement proof data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SettlementProofData {
    /// Lightning preimage proof.
    Lightning {
        /// Preimage in hex format.
        preimage: String,
        /// Payment hash in hex format.
        payment_hash: String,
    },
    /// Bitcoin transaction proof.
    Onchain {
        /// Transaction ID.
        txid: String,
        /// Output index.
        vout: u32,
        /// Block height (if confirmed).
        block_height: Option<u64>,
    },
}

impl SettlementProof {
    /// Create a Lightning settlement proof from a preimage.
    pub fn from_lightning_preimage(
        request_id: impl Into<String>,
        iou_id: impl Into<String>,
        preimage_hex: impl Into<String>,
    ) -> Self {
        let preimage = preimage_hex.into();
        let preimage_bytes =
            hex::decode(&preimage).unwrap_or_default();
        let payment_hash = Sha256::digest(&preimage_bytes);

        Self {
            request_id: request_id.into(),
            iou_id: iou_id.into(),
            method: SettlementMethod::Lightning,
            proof_data: SettlementProofData::Lightning {
                preimage,
                payment_hash: hex::encode(payment_hash),
            },
            settled_at: current_unix_timestamp(),
        }
    }

    /// Create an on-chain settlement proof from a transaction.
    pub fn from_bitcoin_tx(
        request_id: impl Into<String>,
        iou_id: impl Into<String>,
        txid: impl Into<String>,
        vout: u32,
        block_height: Option<u64>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            iou_id: iou_id.into(),
            method: SettlementMethod::Onchain,
            proof_data: SettlementProofData::Onchain {
                txid: txid.into(),
                vout,
                block_height,
            },
            settled_at: current_unix_timestamp(),
        }
    }

    /// Verify the proof is valid.
    ///
    /// For Lightning: verifies SHA256(preimage) == payment_hash
    /// For on-chain: performs format validation only
    pub fn verify(&self) -> Result<bool> {
        match &self.proof_data {
            SettlementProofData::Lightning {
                preimage,
                payment_hash,
            } => {
                let preimage_bytes =
                    hex::decode(preimage).map_err(|e| crate::PaykitError::InvalidData {
                        field: "preimage".into(),
                        reason: format!("Invalid hex: {}", e),
                    })?;
                let computed = Sha256::digest(&preimage_bytes);
                let expected = hex::decode(payment_hash).map_err(|e| {
                    crate::PaykitError::InvalidData {
                        field: "payment_hash".into(),
                        reason: format!("Invalid hex: {}", e),
                    }
                })?;
                Ok(computed.as_slice() == expected.as_slice())
            }
            SettlementProofData::Onchain { txid, .. } => {
                // Basic format validation
                if txid.len() != 64 || hex::decode(txid).is_err() {
                    return Ok(false);
                }
                Ok(true)
            }
        }
    }

    /// Serialize to JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| crate::PaykitError::InvalidData {
            field: "settlement_proof".into(),
            reason: format!("Failed to serialize: {}", e),
        })
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data).map_err(|e| crate::PaykitError::InvalidData {
            field: "settlement_proof".into(),
            reason: format!("Failed to deserialize: {}", e),
        })
    }
}

/// Replay prevention for settlement requests.
///
/// Tracks nonces to prevent the same settlement request from being
/// processed multiple times.
pub trait SettlementNonceStorage: Send + Sync {
    /// Check if a nonce has been used. If not, mark it as used.
    ///
    /// Returns `Ok(true)` if the nonce was new and is now marked.
    /// Returns `Ok(false)` if the nonce was already used.
    fn check_and_mark(&self, nonce: &[u8; 32], expires_at: u64) -> Result<bool>;

    /// Check if a nonce has been used (without marking).
    fn is_used(&self, nonce: &[u8; 32]) -> Result<bool>;

    /// Clean up expired nonces.
    fn cleanup_expired(&self, before: u64) -> Result<()>;
}

/// Settlement executor trait for integrating with Atomicity.
///
/// Implement this trait to handle settlement execution.
#[allow(async_fn_in_trait)]
pub trait SettlementExecutor: Send + Sync {
    /// Execute a settlement request.
    ///
    /// Returns a settlement proof on success.
    async fn execute(&self, request: &SettlementRequest) -> Result<SettlementProof>;

    /// Check the status of a pending settlement.
    async fn check_status(&self, request_id: &str) -> Result<SettlementStatus>;

    /// Cancel a pending settlement (if possible).
    async fn cancel(&self, request_id: &str) -> Result<()>;
}

// ============================================================================
// Executor Adapters
// ============================================================================

/// Adapter that wraps an LND executor to implement SettlementExecutor.
///
/// This enables using Paykit's existing Lightning infrastructure for
/// Atomicity settlement.
#[cfg(feature = "lnd")]
pub struct LndSettlementAdapter<E> {
    inner: E,
    pending_settlements: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, SettlementRequest>>>,
}

#[cfg(feature = "lnd")]
impl<E> LndSettlementAdapter<E> {
    /// Create a new LND settlement adapter.
    pub fn new(executor: E) -> Self {
        Self {
            inner: executor,
            pending_settlements: std::sync::Arc::new(std::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
        }
    }
}

/// Mock settlement executor for testing.
///
/// This executor immediately returns success with a generated proof.
pub struct MockSettlementExecutor {
    pub delay_ms: u64,
    pub should_fail: bool,
}

impl MockSettlementExecutor {
    /// Create a new mock executor that succeeds.
    pub fn new() -> Self {
        Self {
            delay_ms: 0,
            should_fail: false,
        }
    }

    /// Create a mock executor that fails.
    pub fn failing() -> Self {
        Self {
            delay_ms: 0,
            should_fail: true,
        }
    }
}

impl Default for MockSettlementExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl SettlementExecutor for MockSettlementExecutor {
    async fn execute(&self, request: &SettlementRequest) -> Result<SettlementProof> {
        if self.delay_ms > 0 {
            #[cfg(feature = "tokio")]
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        }

        if self.should_fail {
            return Err(crate::PaykitError::Internal(
                "Mock settlement failed".into(),
            ));
        }

        // Generate a mock proof
        let preimage = "0001020304050607080910111213141516171819202122232425262728293031";
        Ok(SettlementProof::from_lightning_preimage(
            &request.request_id,
            &request.iou_id,
            preimage,
        ))
    }

    async fn check_status(&self, _request_id: &str) -> Result<SettlementStatus> {
        if self.should_fail {
            Ok(SettlementStatus::Failed)
        } else {
            Ok(SettlementStatus::Completed)
        }
    }

    async fn cancel(&self, _request_id: &str) -> Result<()> {
        Ok(())
    }
}

/// In-memory nonce storage for testing.
pub struct InMemoryNonceStorage {
    used: std::sync::Mutex<std::collections::HashMap<[u8; 32], u64>>,
}

impl InMemoryNonceStorage {
    /// Create a new in-memory nonce storage.
    pub fn new() -> Self {
        Self {
            used: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryNonceStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SettlementNonceStorage for InMemoryNonceStorage {
    fn check_and_mark(&self, nonce: &[u8; 32], expires_at: u64) -> Result<bool> {
        let mut storage = self.used.lock().map_err(|_| {
            crate::PaykitError::Internal("Failed to lock nonce storage".into())
        })?;

        if storage.contains_key(nonce) {
            return Ok(false);
        }

        storage.insert(*nonce, expires_at);
        Ok(true)
    }

    fn is_used(&self, nonce: &[u8; 32]) -> Result<bool> {
        let storage = self.used.lock().map_err(|_| {
            crate::PaykitError::Internal("Failed to lock nonce storage".into())
        })?;
        Ok(storage.contains_key(nonce))
    }

    fn cleanup_expired(&self, before: u64) -> Result<()> {
        let mut storage = self.used.lock().map_err(|_| {
            crate::PaykitError::Internal("Failed to lock nonce storage".into())
        })?;
        storage.retain(|_, expires_at| *expires_at > before);
        Ok(())
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn generate_request_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("settle_{}", hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settlement_request_serialization() {
        let request = SettlementRequest::new(
            "iou-123",
            50000,
            "lnbc500u1...",
            SettlementMethod::Lightning,
        );

        let bytes = request.to_bytes().unwrap();
        let parsed = SettlementRequest::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.iou_id, "iou-123");
        assert_eq!(parsed.amount_sats, 50000);
        assert_eq!(parsed.method, SettlementMethod::Lightning);
    }

    #[test]
    fn settlement_request_expiry() {
        let now = current_unix_timestamp();
        let request = SettlementRequest::new(
            "iou-123",
            1000,
            "addr",
            SettlementMethod::Onchain,
        )
        .with_expiry(now - 100); // Already expired

        assert!(request.is_expired());
    }

    #[test]
    fn settlement_request_hash_is_deterministic() {
        let mut request = SettlementRequest::new(
            "iou-123",
            1000,
            "lnbc...",
            SettlementMethod::Lightning,
        );
        request.nonce = "test_nonce".to_string();

        let hash1 = request.compute_hash();
        let hash2 = request.compute_hash();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn settlement_proof_lightning_verification() {
        // Known test vector: preimage that hashes to expected hash
        let preimage = "0001020304050607080910111213141516171819202122232425262728293031";
        let proof = SettlementProof::from_lightning_preimage("req-1", "iou-1", preimage);

        assert!(proof.verify().unwrap());
    }

    #[test]
    fn settlement_proof_onchain_validation() {
        let proof = SettlementProof::from_bitcoin_tx(
            "req-1",
            "iou-1",
            "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234",
            0,
            Some(800000),
        );

        assert!(proof.verify().unwrap());

        // Invalid txid length
        let invalid = SettlementProof::from_bitcoin_tx("req-2", "iou-2", "invalid", 0, None);
        assert!(!invalid.verify().unwrap());
    }

    #[test]
    fn settlement_method_display() {
        assert_eq!(SettlementMethod::Lightning.to_string(), "lightning");
        assert_eq!(SettlementMethod::Onchain.to_string(), "onchain");
    }

    #[test]
    fn request_id_is_unique() {
        let req1 = SettlementRequest::new("iou-1", 1000, "ln...", SettlementMethod::Lightning);
        let req2 = SettlementRequest::new("iou-2", 1000, "ln...", SettlementMethod::Lightning);

        assert_ne!(req1.request_id, req2.request_id);
        assert!(req1.request_id.starts_with("settle_"));
    }
}
