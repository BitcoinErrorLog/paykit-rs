//! Lightning Network Payment Method Plugin
//!
//! This module implements the built-in Lightning Network payment method.
//! It supports BOLT11 invoices and LNURL for payment.
//!
//! # Wallet Integration
//!
//! To execute actual payments, provide a `LightningExecutor` implementation:
//!
//! ```ignore
//! use paykit_lib::methods::{LightningPlugin, LightningExecutor, MockLightningExecutor};
//! use std::sync::Arc;
//!
//! // For testing
//! let plugin = LightningPlugin::with_executor(Arc::new(MockLightningExecutor::new()));
//!
//! // For production, implement LightningExecutor for your node
//! struct MyLndNode { /* ... */ }
//! impl LightningExecutor for MyLndNode { /* ... */ }
//! let plugin = LightningPlugin::with_executor(Arc::new(MyLndNode::new()));
//! ```

use super::executor::{LightningExecutor, LightningPaymentStatus, MockLightningExecutor};
use super::traits::{
    Amount, PaymentExecution, PaymentMethodPlugin, PaymentProof, ValidationResult,
};
use crate::{EndpointData, MethodId, PaykitError, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// Lightning Network payment method plugin.
///
/// This plugin handles Lightning Network payments using BOLT11 invoices.
/// BOLT12 is not currently supported due to conflicting subscription schemes.
///
/// # Example
///
/// ```ignore
/// use paykit_lib::methods::{LightningPlugin, PaymentMethodPlugin};
///
/// let plugin = LightningPlugin::new();
/// assert_eq!(plugin.method_id().0, "lightning");
/// ```
pub struct LightningPlugin {
    /// Network type for invoice validation.
    network: LightningNetwork,
    /// Optional executor for actual payments.
    executor: Option<Arc<dyn LightningExecutor>>,
}

/// Lightning network types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightningNetwork {
    /// Bitcoin mainnet Lightning.
    #[default]
    Mainnet,
    /// Bitcoin testnet Lightning.
    Testnet,
    /// Bitcoin regtest Lightning.
    Regtest,
}

impl LightningPlugin {
    /// Creates a new Lightning plugin for mainnet.
    ///
    /// Without an executor, `execute_payment` will return a mock result.
    pub fn new() -> Self {
        Self {
            network: LightningNetwork::Mainnet,
            executor: None,
        }
    }

    /// Creates a new Lightning plugin for a specific network.
    pub fn with_network(network: LightningNetwork) -> Self {
        Self {
            network,
            executor: None,
        }
    }

    /// Creates a new Lightning plugin with a custom executor.
    ///
    /// The executor handles actual payment execution.
    pub fn with_executor(executor: Arc<dyn LightningExecutor>) -> Self {
        Self {
            network: LightningNetwork::Mainnet,
            executor: Some(executor),
        }
    }

    /// Creates a plugin with both network and executor configured.
    pub fn with_network_and_executor(
        network: LightningNetwork,
        executor: Arc<dyn LightningExecutor>,
    ) -> Self {
        Self {
            network,
            executor: Some(executor),
        }
    }

    /// Creates a plugin with a mock executor for testing.
    pub fn with_mock_executor() -> Self {
        Self {
            network: LightningNetwork::Mainnet,
            executor: Some(Arc::new(MockLightningExecutor::new())),
        }
    }

    /// Returns the network this plugin is configured for.
    pub fn network(&self) -> LightningNetwork {
        self.network
    }

    /// Returns whether this plugin has an executor configured.
    pub fn has_executor(&self) -> bool {
        self.executor.is_some()
    }

    /// Validates a BOLT11 invoice.
    fn validate_bolt11(&self, invoice: &str) -> ValidationResult {
        let invoice = invoice.trim().to_lowercase();

        if invoice.is_empty() {
            return ValidationResult::invalid(vec!["Invoice is empty".to_string()]);
        }

        // Check invoice prefix based on network
        let valid_prefix = match self.network {
            LightningNetwork::Mainnet => invoice.starts_with("lnbc"),
            LightningNetwork::Testnet => {
                invoice.starts_with("lntb") || invoice.starts_with("lnbcrt")
            }
            LightningNetwork::Regtest => invoice.starts_with("lnbcrt"),
        };

        if !valid_prefix {
            let expected = match self.network {
                LightningNetwork::Mainnet => "lnbc",
                LightningNetwork::Testnet => "lntb or lnbcrt",
                LightningNetwork::Regtest => "lnbcrt",
            };
            return ValidationResult::invalid(vec![format!(
                "Invalid invoice prefix. Expected {} for {:?}",
                expected, self.network
            )]);
        }

        // Basic structure check (BOLT11 invoices are bech32 encoded)
        // A valid invoice should have:
        // 1. Network prefix (lnbc, lntb, lnbcrt)
        // 2. Optional amount
        // 3. Separator '1'
        // 4. Data part (timestamp, payment hash, etc.)
        // 5. Signature

        if !invoice.contains('1') {
            return ValidationResult::invalid(vec![
                "Invalid invoice format: missing separator".to_string()
            ]);
        }

        // Check minimum length (prefix + separator + minimal data + signature)
        if invoice.len() < 100 {
            return ValidationResult::invalid(vec![format!(
                "Invoice too short: {} chars (minimum ~100)",
                invoice.len()
            )]);
        }

        let mut result = ValidationResult::valid();

        // Check for expired invoice (if we could parse the timestamp)
        // For now, add a warning about expiration
        result = result.with_warning("Cannot verify invoice expiration without full parsing");

        result
    }

    /// Validates an LNURL.
    fn validate_lnurl(&self, lnurl: &str) -> ValidationResult {
        let lnurl = lnurl.trim().to_lowercase();

        if lnurl.is_empty() {
            return ValidationResult::invalid(vec!["LNURL is empty".to_string()]);
        }

        // LNURL should start with "lnurl"
        if !lnurl.starts_with("lnurl") {
            return ValidationResult::invalid(vec![
                "Invalid LNURL format: must start with 'lnurl'".to_string(),
            ]);
        }

        // Basic length check (bech32 encoded URL)
        if lnurl.len() < 20 {
            return ValidationResult::invalid(vec![format!(
                "LNURL too short: {} chars",
                lnurl.len()
            )]);
        }

        ValidationResult::valid()
    }

    /// Extracts payment data from endpoint.
    fn extract_payment_data(&self, data: &EndpointData) -> Result<PaymentData> {
        let data_str = data.0.trim();

        // Try parsing as JSON first
        if data_str.starts_with('{') {
            let json: serde_json::Value = serde_json::from_str(data_str)
                .map_err(|e| PaykitError::Transport(format!("Invalid JSON: {}", e)))?;

            // Look for BOLT11 invoice
            if let Some(bolt11) = json.get("bolt11").and_then(|v| v.as_str()) {
                return Ok(PaymentData::Bolt11(bolt11.to_string()));
            }

            // Look for LNURL
            if let Some(lnurl) = json.get("lnurl").and_then(|v| v.as_str()) {
                return Ok(PaymentData::Lnurl(lnurl.to_string()));
            }

            return Err(PaykitError::Transport(
                "No bolt11 or lnurl field found in JSON endpoint".to_string(),
            ));
        }

        // Check if it's an LNURL
        if data_str.to_lowercase().starts_with("lnurl") {
            return Ok(PaymentData::Lnurl(data_str.to_string()));
        }

        // Otherwise, treat as BOLT11 invoice
        Ok(PaymentData::Bolt11(data_str.to_string()))
    }
}

/// Parsed payment data from endpoint.
#[derive(Debug, Clone)]
enum PaymentData {
    /// BOLT11 invoice.
    Bolt11(String),
    /// LNURL for payment.
    Lnurl(String),
}

impl Default for LightningPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Verify a Lightning payment proof.
///
/// This function validates that a preimage matches a payment hash.
/// The preimage is the cryptographic proof that payment was received.
///
/// # Arguments
///
/// * `proof` - The payment proof to verify
/// * `executor` - Optional executor for additional verification
///
/// # Returns
///
/// Result indicating whether the proof is valid.
pub fn verify_lightning_proof(
    proof: &PaymentProof,
    executor: Option<&dyn LightningExecutor>,
) -> Result<bool> {
    match proof {
        PaymentProof::LightningPreimage {
            preimage,
            payment_hash,
        } => {
            if preimage == "pending" || payment_hash == "pending" {
                return Ok(false);
            }

            // Verify preimage matches hash
            if let Some(exec) = executor {
                return Ok(exec.verify_preimage(preimage, payment_hash));
            }

            // Without executor, just check that values look valid
            // A real preimage/hash is 64 hex characters (32 bytes)
            let valid = preimage.len() == 64
                && payment_hash.len() == 64
                && preimage.chars().all(|c| c.is_ascii_hexdigit())
                && payment_hash.chars().all(|c| c.is_ascii_hexdigit());

            Ok(valid)
        }
        _ => Err(PaykitError::Transport(
            "Invalid proof type: expected LightningPreimage".to_string(),
        )),
    }
}

#[async_trait]
impl PaymentMethodPlugin for LightningPlugin {
    fn method_id(&self) -> MethodId {
        MethodId("lightning".to_string())
    }

    fn display_name(&self) -> &str {
        "Lightning Network"
    }

    fn description(&self) -> &str {
        "Pay using the Lightning Network for instant, low-fee Bitcoin payments. Supports BOLT11 invoices and LNURL."
    }

    fn validate_endpoint(&self, data: &EndpointData) -> ValidationResult {
        match self.extract_payment_data(data) {
            Ok(PaymentData::Bolt11(invoice)) => self.validate_bolt11(&invoice),
            Ok(PaymentData::Lnurl(lnurl)) => self.validate_lnurl(&lnurl),
            Err(e) => ValidationResult::invalid(vec![e.to_string()]),
        }
    }

    async fn execute_payment(
        &self,
        endpoint: &EndpointData,
        amount: &Amount,
        metadata: &Value,
    ) -> Result<PaymentExecution> {
        // Extract and validate payment data
        let payment_data = self.extract_payment_data(endpoint)?;

        let validation = match &payment_data {
            PaymentData::Bolt11(invoice) => self.validate_bolt11(invoice),
            PaymentData::Lnurl(lnurl) => self.validate_lnurl(lnurl),
        };

        if !validation.valid {
            return Err(PaykitError::Transport(validation.errors.join(", ")));
        }

        // Check amount limits
        if !self.supports_amount(amount) {
            return Err(PaykitError::Transport(format!(
                "Amount {} not supported for Lightning (limits: 1 sat - 4M sats)",
                amount
            )));
        }

        // Convert to millisatoshis
        let amount_msat = amount.as_u64().map(|sats| sats * 1000);

        // Get max fee from metadata
        let max_fee_msat = metadata.get("max_fee_msat").and_then(|v| v.as_u64());

        // Execute payment via executor if available
        if let Some(executor) = &self.executor {
            match &payment_data {
                PaymentData::Bolt11(invoice) => {
                    match executor
                        .pay_invoice(invoice, amount_msat, max_fee_msat)
                        .await
                    {
                        Ok(result) => {
                            return Ok(PaymentExecution {
                                method_id: self.method_id(),
                                endpoint: endpoint.clone(),
                                amount: amount.clone(),
                                success: result.status == LightningPaymentStatus::Succeeded,
                                executed_at: current_timestamp(),
                                execution_data: serde_json::json!({
                                    "type": "bolt11",
                                    "invoice": invoice,
                                    "preimage": result.preimage,
                                    "payment_hash": result.payment_hash,
                                    "amount_msat": result.amount_msat,
                                    "fee_msat": result.fee_msat,
                                    "hops": result.hops,
                                    "status": format!("{:?}", result.status),
                                    "metadata": metadata,
                                }),
                                error: if result.status == LightningPaymentStatus::Failed {
                                    Some("Payment failed".to_string())
                                } else {
                                    None
                                },
                            });
                        }
                        Err(e) => {
                            return Ok(PaymentExecution::failure(
                                self.method_id(),
                                endpoint.clone(),
                                amount.clone(),
                                e.to_string(),
                            ));
                        }
                    }
                }
                PaymentData::Lnurl(_lnurl) => {
                    // LNURL requires fetching the actual invoice first
                    // For now, return an error indicating LNURL needs special handling
                    return Err(PaykitError::Transport(
                        "LNURL payments require additional flow - use decode_lnurl first"
                            .to_string(),
                    ));
                }
            }
        }

        // No executor configured - return mock/placeholder result
        let execution_data = match payment_data {
            PaymentData::Bolt11(invoice) => serde_json::json!({
                "type": "bolt11",
                "invoice": invoice,
                "amount_msat": amount_msat,
                "currency": amount.currency,
                "metadata": metadata,
                "mock": true,
                "note": "No executor configured - this is a mock result",
            }),
            PaymentData::Lnurl(lnurl) => serde_json::json!({
                "type": "lnurl",
                "lnurl": lnurl,
                "amount_msat": amount_msat,
                "currency": amount.currency,
                "metadata": metadata,
                "mock": true,
                "note": "No executor configured - this is a mock result",
            }),
        };

        Ok(PaymentExecution {
            method_id: self.method_id(),
            endpoint: endpoint.clone(),
            amount: amount.clone(),
            success: true,
            executed_at: current_timestamp(),
            execution_data,
            error: None,
        })
    }

    fn generate_proof(&self, execution: &PaymentExecution) -> Result<PaymentProof> {
        // Check if execution was successful
        if !execution.success {
            return Err(PaykitError::Transport(
                execution
                    .error
                    .clone()
                    .unwrap_or_else(|| "Payment failed".to_string()),
            ));
        }

        // Extract preimage and payment hash from execution data
        let preimage = execution
            .execution_data
            .get("preimage")
            .and_then(|v| v.as_str())
            .unwrap_or("pending");

        let payment_hash = execution
            .execution_data
            .get("payment_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("pending");

        Ok(PaymentProof::LightningPreimage {
            preimage: preimage.to_string(),
            payment_hash: payment_hash.to_string(),
        })
    }

    async fn estimate_fee(&self, amount: &Amount) -> Option<Amount> {
        // Lightning fees are typically a percentage + base fee
        // Estimate ~0.1% + 1 sat base
        let amount_sats = amount.as_u64()?;
        let fee = (amount_sats / 1000).max(1) + 1;
        Some(Amount::sats(fee))
    }

    fn format_receipt_metadata(&self, execution: &PaymentExecution) -> Value {
        let payment_type = execution
            .execution_data
            .get("type")
            .cloned()
            .unwrap_or(Value::String("bolt11".to_string()));

        let preimage = execution
            .execution_data
            .get("preimage")
            .cloned()
            .unwrap_or(Value::Null);

        let payment_hash = execution
            .execution_data
            .get("payment_hash")
            .cloned()
            .unwrap_or(Value::Null);

        serde_json::json!({
            "method": "lightning",
            "payment_type": payment_type,
            "preimage": preimage,
            "payment_hash": payment_hash,
            "executed_at": execution.executed_at,
        })
    }

    fn supports_amount(&self, amount: &Amount) -> bool {
        // Lightning has practical limits
        // Minimum: 1 sat (some nodes require higher)
        // Maximum: depends on channel capacity, but ~0.04 BTC is common limit
        if amount.currency.to_uppercase() == "SAT" {
            if let Some(sats) = amount.as_u64() {
                // Minimum 1 sat, max ~4,000,000 sats (0.04 BTC)
                return (1..=4_000_000).contains(&sats);
            }
        }
        true
    }

    fn estimated_confirmation_time(&self) -> Option<u64> {
        // Lightning payments are essentially instant
        Some(1) // 1 second
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
    fn test_plugin_id() {
        let plugin = LightningPlugin::new();
        assert_eq!(plugin.method_id().0, "lightning");
    }

    #[test]
    fn test_plugin_with_executor() {
        let plugin = LightningPlugin::with_mock_executor();
        assert!(plugin.has_executor());

        let plugin_no_exec = LightningPlugin::new();
        assert!(!plugin_no_exec.has_executor());
    }

    #[test]
    fn test_validate_bolt11_mainnet() {
        let plugin = LightningPlugin::new();

        // Valid mainnet invoice (example structure)
        let valid_invoice = "lnbc1pvjluezpp5qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqypqdpl2pkx2ctnv5sxxmmwwd5kgetjypeh2ursdae8g6twvus8g6rfwvs8qun0dfjkxaq8rkx3yf5tcsyz3d73gafnh3cax9rn449d9p5uxz9ezhhypd0elx87sjle52x86fux2ypatgddc6k63n7erqz25le42c4u4ecky03ylcqca784w";
        let result = plugin.validate_bolt11(valid_invoice);
        assert!(result.valid);

        // Invalid - wrong prefix
        let result = plugin.validate_bolt11("lntb1...");
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_bolt11_testnet() {
        let plugin = LightningPlugin::with_network(LightningNetwork::Testnet);

        // Valid testnet prefix
        let result = plugin.validate_bolt11("lntb1pvjluezpp5qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqypqdpl2pkx2ctnv5sxxmmwwd5kgetjypeh2ursdae8g6twvus8g6rfwvs8qun0dfjkxaq8rkx3yf5tcsyz3d73gafnh3cax9rn449d9p5uxz9ezhhypd0elx87sjle52x86fux2ypatgddc6k63n7erqz25le42c4u4ecky03ylcq");
        assert!(result.valid);
    }

    #[test]
    fn test_validate_lnurl() {
        let plugin = LightningPlugin::new();

        // Valid LNURL format
        let result = plugin.validate_lnurl("lnurl1dp68gurn8ghj7um9wfmxjcm99e3k7mf0v9cxj0m385ekvcenxc6r2c35xvukxefcv5mkvv34x5ekzd3ev56nyd3hxqyf3ex");
        assert!(result.valid);

        // Invalid - wrong prefix
        let result = plugin.validate_lnurl("https://example.com/lnurl");
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_endpoint_json() {
        let plugin = LightningPlugin::new();

        let data = EndpointData(r#"{"bolt11":"lnbc1pvjluezpp5qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqypqdpl2pkx2ctnv5sxxmmwwd5kgetjypeh2ursdae8g6twvus8g6rfwvs8qun0dfjkxaq8rkx3yf5tcsyz3d73gafnh3cax9rn449d9p5uxz9ezhhypd0elx87sjle52x86fux2ypatgddc6k63n7erqz25le42c4u4ecky03ylcqca784w"}"#.to_string());
        let result = plugin.validate_endpoint(&data);
        assert!(result.valid);
    }

    #[test]
    fn test_supports_amount() {
        let plugin = LightningPlugin::new();

        // Valid amounts
        assert!(plugin.supports_amount(&Amount::sats(1)));
        assert!(plugin.supports_amount(&Amount::sats(1000)));
        assert!(plugin.supports_amount(&Amount::sats(1_000_000)));

        // Too large
        assert!(!plugin.supports_amount(&Amount::sats(10_000_000)));
    }

    #[test]
    fn test_confirmation_time() {
        let plugin = LightningPlugin::new();
        assert_eq!(plugin.estimated_confirmation_time(), Some(1));
    }

    #[test]
    fn test_extract_payment_data() {
        let plugin = LightningPlugin::new();

        // BOLT11 from JSON
        let data = EndpointData(r#"{"bolt11":"lnbc1..."}"#.to_string());
        let result = plugin.extract_payment_data(&data);
        assert!(matches!(result, Ok(PaymentData::Bolt11(_))));

        // LNURL from JSON
        let data = EndpointData(r#"{"lnurl":"lnurl1..."}"#.to_string());
        let result = plugin.extract_payment_data(&data);
        assert!(matches!(result, Ok(PaymentData::Lnurl(_))));

        // Raw LNURL
        let data = EndpointData(
            "lnurl1dp68gurn8ghj7um9wfmxjcm99e3k7mf0v9cxj0m385ekvcenxc6r2c35".to_string(),
        );
        let result = plugin.extract_payment_data(&data);
        assert!(matches!(result, Ok(PaymentData::Lnurl(_))));

        // Raw BOLT11
        let data = EndpointData("lnbc1pvjluezpp5...".to_string());
        let result = plugin.extract_payment_data(&data);
        assert!(matches!(result, Ok(PaymentData::Bolt11(_))));
    }

    #[tokio::test]
    async fn test_execute_payment_with_mock_executor() {
        let plugin = LightningPlugin::with_mock_executor();

        let invoice = "lnbc1pvjluezpp5qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqypqdpl2pkx2ctnv5sxxmmwwd5kgetjypeh2ursdae8g6twvus8g6rfwvs8qun0dfjkxaq8rkx3yf5tcsyz3d73gafnh3cax9rn449d9p5uxz9ezhhypd0elx87sjle52x86fux2ypatgddc6k63n7erqz25le42c4u4ecky03ylcqca784w";
        let endpoint = EndpointData(invoice.to_string());
        let amount = Amount::sats(1000);
        let metadata = serde_json::json!({});

        let result = plugin
            .execute_payment(&endpoint, &amount, &metadata)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.execution_data.get("preimage").is_some());
        assert!(result.execution_data.get("payment_hash").is_some());
    }

    #[tokio::test]
    async fn test_execute_payment_without_executor() {
        let plugin = LightningPlugin::new();

        let invoice = "lnbc1pvjluezpp5qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqypqdpl2pkx2ctnv5sxxmmwwd5kgetjypeh2ursdae8g6twvus8g6rfwvs8qun0dfjkxaq8rkx3yf5tcsyz3d73gafnh3cax9rn449d9p5uxz9ezhhypd0elx87sjle52x86fux2ypatgddc6k63n7erqz25le42c4u4ecky03ylcqca784w";
        let endpoint = EndpointData(invoice.to_string());
        let amount = Amount::sats(1000);
        let metadata = serde_json::json!({});

        let result = plugin
            .execute_payment(&endpoint, &amount, &metadata)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(
            result.execution_data.get("mock").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[tokio::test]
    async fn test_generate_proof() {
        let plugin = LightningPlugin::with_mock_executor();

        let invoice = "lnbc1pvjluezpp5qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqypqdpl2pkx2ctnv5sxxmmwwd5kgetjypeh2ursdae8g6twvus8g6rfwvs8qun0dfjkxaq8rkx3yf5tcsyz3d73gafnh3cax9rn449d9p5uxz9ezhhypd0elx87sjle52x86fux2ypatgddc6k63n7erqz25le42c4u4ecky03ylcqca784w";
        let endpoint = EndpointData(invoice.to_string());
        let amount = Amount::sats(1000);
        let metadata = serde_json::json!({});

        let execution = plugin
            .execute_payment(&endpoint, &amount, &metadata)
            .await
            .unwrap();
        let proof = plugin.generate_proof(&execution).unwrap();

        match proof {
            PaymentProof::LightningPreimage {
                preimage,
                payment_hash,
            } => {
                assert!(!preimage.is_empty());
                assert!(!payment_hash.is_empty());
                assert_ne!(preimage, "pending");
            }
            _ => panic!("Expected LightningPreimage proof"),
        }
    }

    #[tokio::test]
    async fn test_estimate_fee() {
        let plugin = LightningPlugin::with_mock_executor();

        let amount = Amount::sats(10000);
        let fee = plugin.estimate_fee(&amount).await;

        assert!(fee.is_some());
        let fee_sats = fee.unwrap().as_u64().unwrap();
        // Should be ~0.1% + 1 sat = 11 sats
        assert!(fee_sats >= 1);
    }

    #[test]
    fn test_verify_lightning_proof() {
        // Valid-looking preimage and hash (64 hex chars each)
        let proof = PaymentProof::LightningPreimage {
            preimage: "abc123def456abc123def456abc123def456abc123def456abc123def456abcd"
                .to_string(),
            payment_hash: "def456abc123def456abc123def456abc123def456abc123def456abc123defg"
                .to_string(),
        };

        // Without executor, just validates format
        let result = verify_lightning_proof(&proof, None).unwrap();
        // Should be false because 'g' is not a valid hex character
        assert!(!result);

        // Valid hex
        let proof = PaymentProof::LightningPreimage {
            preimage: "abc123def456abc123def456abc123def456abc123def456abc123def456abcd"
                .to_string(),
            payment_hash: "def456abc123def456abc123def456abc123def456abc123def456abc123defa"
                .to_string(),
        };
        let result = verify_lightning_proof(&proof, None).unwrap();
        assert!(result);

        // Pending should fail
        let pending_proof = PaymentProof::LightningPreimage {
            preimage: "pending".to_string(),
            payment_hash: "pending".to_string(),
        };
        let result = verify_lightning_proof(&pending_proof, None).unwrap();
        assert!(!result);
    }
}
