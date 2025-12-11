//! Payment Method Plugin Traits
//!
//! This module defines the core traits for payment method plugins.
//! Any payment method (onchain, lightning, ethereum, etc.) can implement
//! these traits to integrate with Paykit.

use crate::{EndpointData, MethodId, PaykitError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Amount for payment operations with currency support.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    /// The amount value as a string to preserve precision.
    pub value: String,
    /// The currency code (e.g., "SAT", "BTC", "USD").
    pub currency: String,
}

impl Amount {
    /// Create a new amount.
    pub fn new(value: impl Into<String>, currency: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            currency: currency.into(),
        }
    }

    /// Create an amount in satoshis.
    pub fn sats(value: u64) -> Self {
        Self {
            value: value.to_string(),
            currency: "SAT".to_string(),
        }
    }

    /// Parse the value as u64 (for satoshi amounts).
    pub fn as_u64(&self) -> Option<u64> {
        self.value.parse().ok()
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.value, self.currency)
    }
}

/// Result of executing a payment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentExecution {
    /// The payment method used.
    pub method_id: MethodId,
    /// The endpoint used for payment.
    pub endpoint: EndpointData,
    /// The amount paid.
    pub amount: Amount,
    /// Whether the payment was successful.
    pub success: bool,
    /// Timestamp of execution (unix epoch seconds).
    pub executed_at: i64,
    /// Method-specific execution data (e.g., txid, preimage).
    pub execution_data: Value,
    /// Error message if payment failed.
    pub error: Option<String>,
}

impl PaymentExecution {
    /// Create a successful payment execution.
    pub fn success(
        method_id: MethodId,
        endpoint: EndpointData,
        amount: Amount,
        execution_data: Value,
    ) -> Self {
        Self {
            method_id,
            endpoint,
            amount,
            success: true,
            executed_at: current_timestamp(),
            execution_data,
            error: None,
        }
    }

    /// Create a failed payment execution.
    pub fn failure(
        method_id: MethodId,
        endpoint: EndpointData,
        amount: Amount,
        error: String,
    ) -> Self {
        Self {
            method_id,
            endpoint,
            amount,
            success: false,
            executed_at: current_timestamp(),
            execution_data: Value::Null,
            error: Some(error),
        }
    }
}

/// Standardized payment proof format.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PaymentProof {
    /// Bitcoin on-chain transaction proof.
    BitcoinTxid {
        /// The transaction ID.
        txid: String,
        /// The block height (None if unconfirmed).
        block_height: Option<u64>,
        /// Number of confirmations.
        confirmations: Option<u64>,
    },
    /// Lightning Network payment proof.
    LightningPreimage {
        /// The payment preimage.
        preimage: String,
        /// The payment hash.
        payment_hash: String,
    },
    /// Custom proof for other payment methods.
    Custom {
        /// The method ID.
        method: MethodId,
        /// Method-specific proof data.
        data: Value,
    },
}

impl PaymentProof {
    /// Create a Bitcoin txid proof.
    pub fn bitcoin_txid(txid: impl Into<String>, block_height: Option<u64>) -> Self {
        Self::BitcoinTxid {
            txid: txid.into(),
            block_height,
            confirmations: None,
        }
    }

    /// Create a Lightning preimage proof.
    pub fn lightning_preimage(
        preimage: impl Into<String>,
        payment_hash: impl Into<String>,
    ) -> Self {
        Self::LightningPreimage {
            preimage: preimage.into(),
            payment_hash: payment_hash.into(),
        }
    }

    /// Create a custom proof.
    pub fn custom(method: MethodId, data: Value) -> Self {
        Self::Custom { method, data }
    }
}

/// Validation result for endpoint data.
#[derive(Clone, Debug)]
pub struct ValidationResult {
    /// Whether the endpoint is valid.
    pub valid: bool,
    /// Validation errors if any.
    pub errors: Vec<String>,
    /// Warnings (non-fatal issues).
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a valid result.
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create an invalid result with errors.
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add a warning.
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

/// Core trait for payment method plugins.
///
/// Implement this trait to add support for a new payment method.
/// Each plugin handles validation, payment execution, and proof generation
/// for its specific payment protocol.
#[async_trait]
pub trait PaymentMethodPlugin: Send + Sync {
    /// Returns the unique identifier for this payment method.
    fn method_id(&self) -> MethodId;

    /// Returns a human-readable name for this payment method.
    fn display_name(&self) -> &str;

    /// Returns a description of this payment method.
    fn description(&self) -> &str;

    /// Validates endpoint data for this payment method.
    ///
    /// Returns a `ValidationResult` indicating whether the endpoint is valid.
    fn validate_endpoint(&self, data: &EndpointData) -> ValidationResult;

    /// Executes a payment using the given endpoint and amount.
    ///
    /// This is an async operation that performs the actual payment.
    /// The implementation should handle all method-specific logic.
    async fn execute_payment(
        &self,
        endpoint: &EndpointData,
        amount: &Amount,
        metadata: &Value,
    ) -> Result<PaymentExecution>;

    /// Generates a payment proof from an execution result.
    ///
    /// The proof can be used to verify that payment was completed.
    fn generate_proof(&self, execution: &PaymentExecution) -> Result<PaymentProof>;

    /// Formats receipt metadata for this payment method.
    ///
    /// Returns method-specific metadata to include in the receipt.
    fn format_receipt_metadata(&self, execution: &PaymentExecution) -> Value;

    /// Checks if this method supports the given amount.
    ///
    /// Some methods may have minimum/maximum amount limits.
    fn supports_amount(&self, amount: &Amount) -> bool {
        // Default: support all amounts
        let _ = amount;
        true
    }

    /// Returns the estimated fee for a payment.
    ///
    /// Returns None if fee estimation is not available.
    async fn estimate_fee(&self, _amount: &Amount) -> Option<Amount> {
        None
    }

    /// Returns the estimated confirmation time in seconds.
    ///
    /// Returns None if time estimation is not available.
    fn estimated_confirmation_time(&self) -> Option<u64> {
        None
    }

    /// Generates a new endpoint for receiving payments.
    ///
    /// This is used for endpoint rotation and private endpoint generation.
    async fn generate_endpoint(&self) -> Result<EndpointData> {
        Err(PaykitError::Unimplemented("generate_endpoint"))
    }
}

/// Helper function to get current timestamp.
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
    fn test_amount_creation() {
        let amt = Amount::sats(1000);
        assert_eq!(amt.value, "1000");
        assert_eq!(amt.currency, "SAT");
        assert_eq!(amt.as_u64(), Some(1000));
    }

    #[test]
    fn test_amount_display() {
        let amt = Amount::new("0.001", "BTC");
        assert_eq!(format!("{}", amt), "0.001 BTC");
    }

    #[test]
    fn test_validation_result() {
        let valid = ValidationResult::valid();
        assert!(valid.valid);
        assert!(valid.errors.is_empty());

        let invalid = ValidationResult::invalid(vec!["Invalid address".to_string()]);
        assert!(!invalid.valid);
        assert_eq!(invalid.errors.len(), 1);
    }

    #[test]
    fn test_payment_proof_creation() {
        let btc_proof = PaymentProof::bitcoin_txid("abc123", Some(700000));
        match btc_proof {
            PaymentProof::BitcoinTxid {
                txid, block_height, ..
            } => {
                assert_eq!(txid, "abc123");
                assert_eq!(block_height, Some(700000));
            }
            _ => panic!("Expected BitcoinTxid"),
        }

        let ln_proof = PaymentProof::lightning_preimage("preimage123", "hash456");
        match ln_proof {
            PaymentProof::LightningPreimage {
                preimage,
                payment_hash,
            } => {
                assert_eq!(preimage, "preimage123");
                assert_eq!(payment_hash, "hash456");
            }
            _ => panic!("Expected LightningPreimage"),
        }
    }
}
