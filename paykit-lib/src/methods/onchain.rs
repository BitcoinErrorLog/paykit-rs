//! Bitcoin On-Chain Payment Method Plugin
//!
//! This module implements the built-in Bitcoin on-chain payment method.
//! It supports legacy, SegWit, and Bech32 address formats.
//!
//! # Wallet Integration
//!
//! To execute actual payments, provide a `BitcoinExecutor` implementation:
//!
//! ```ignore
//! use paykit_lib::methods::{OnchainPlugin, BitcoinExecutor, MockBitcoinExecutor};
//! use std::sync::Arc;
//!
//! // For testing
//! let plugin = OnchainPlugin::with_executor(Arc::new(MockBitcoinExecutor::new()));
//!
//! // For production, implement BitcoinExecutor for your wallet
//! struct MyWallet { /* ... */ }
//! impl BitcoinExecutor for MyWallet { /* ... */ }
//! let plugin = OnchainPlugin::with_executor(Arc::new(MyWallet::new()));
//! ```

use super::executor::{BitcoinExecutor, MockBitcoinExecutor};
use super::traits::{
    Amount, PaymentExecution, PaymentMethodPlugin, PaymentProof, ValidationResult,
};
use crate::{EndpointData, MethodId, PaykitError, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// Bitcoin on-chain payment method plugin.
///
/// This plugin handles Bitcoin on-chain payments using standard addresses.
/// It validates addresses in legacy, SegWit (P2SH-P2WPKH), and Bech32 formats.
///
/// # Example
///
/// ```ignore
/// use paykit_lib::methods::{OnchainPlugin, PaymentMethodPlugin};
///
/// let plugin = OnchainPlugin::new();
/// assert_eq!(plugin.method_id().0, "onchain");
/// ```
pub struct OnchainPlugin {
    /// Network type for address validation.
    network: BitcoinNetwork,
    /// Optional executor for actual payments.
    executor: Option<Arc<dyn BitcoinExecutor>>,
}

/// Bitcoin network types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BitcoinNetwork {
    /// Bitcoin mainnet.
    #[default]
    Mainnet,
    /// Bitcoin testnet.
    Testnet,
    /// Bitcoin regtest (local development).
    Regtest,
}

impl OnchainPlugin {
    /// Creates a new on-chain plugin for mainnet.
    ///
    /// Without an executor, `execute_payment` will return a mock result.
    pub fn new() -> Self {
        Self {
            network: BitcoinNetwork::Mainnet,
            executor: None,
        }
    }

    /// Creates a new on-chain plugin for a specific network.
    pub fn with_network(network: BitcoinNetwork) -> Self {
        Self {
            network,
            executor: None,
        }
    }

    /// Creates a new on-chain plugin with a custom executor.
    ///
    /// The executor handles actual payment execution.
    pub fn with_executor(executor: Arc<dyn BitcoinExecutor>) -> Self {
        Self {
            network: BitcoinNetwork::Mainnet,
            executor: Some(executor),
        }
    }

    /// Creates a plugin with both network and executor configured.
    pub fn with_network_and_executor(
        network: BitcoinNetwork,
        executor: Arc<dyn BitcoinExecutor>,
    ) -> Self {
        Self {
            network,
            executor: Some(executor),
        }
    }

    /// Creates a plugin with a mock executor for testing.
    pub fn with_mock_executor() -> Self {
        Self {
            network: BitcoinNetwork::Mainnet,
            executor: Some(Arc::new(MockBitcoinExecutor::new())),
        }
    }

    /// Returns the network this plugin is configured for.
    pub fn network(&self) -> BitcoinNetwork {
        self.network
    }

    /// Returns whether this plugin has an executor configured.
    pub fn has_executor(&self) -> bool {
        self.executor.is_some()
    }

    /// Validates a Bitcoin address.
    fn validate_address(&self, address: &str) -> ValidationResult {
        let address = address.trim();

        if address.is_empty() {
            return ValidationResult::invalid(vec!["Address is empty".to_string()]);
        }

        // Check address format based on network
        match self.network {
            BitcoinNetwork::Mainnet => self.validate_mainnet_address(address),
            BitcoinNetwork::Testnet => self.validate_testnet_address(address),
            BitcoinNetwork::Regtest => self.validate_regtest_address(address),
        }
    }

    fn validate_mainnet_address(&self, address: &str) -> ValidationResult {
        // Mainnet address prefixes
        let valid = address.starts_with("1")  // Legacy P2PKH
            || address.starts_with("3")        // P2SH (includes P2SH-P2WPKH)
            || address.starts_with("bc1q")     // Bech32 P2WPKH
            || address.starts_with("bc1p"); // Bech32m P2TR (Taproot)

        if !valid {
            return ValidationResult::invalid(vec![format!(
                "Invalid mainnet address format: {}",
                address
            )]);
        }

        // Basic length checks
        let mut result = ValidationResult::valid();

        if address.starts_with("1") || address.starts_with("3") {
            if address.len() < 26 || address.len() > 35 {
                return ValidationResult::invalid(vec![format!(
                    "Invalid address length: {} (expected 26-35)",
                    address.len()
                )]);
            }
        } else if address.starts_with("bc1q") {
            if address.len() != 42 && address.len() != 62 {
                return ValidationResult::invalid(vec![format!(
                    "Invalid bech32 address length: {} (expected 42 or 62)",
                    address.len()
                )]);
            }
        } else if address.starts_with("bc1p") {
            if address.len() != 62 {
                return ValidationResult::invalid(vec![format!(
                    "Invalid taproot address length: {} (expected 62)",
                    address.len()
                )]);
            }
        }

        // Add warning for legacy addresses (privacy)
        if address.starts_with("1") {
            result = result.with_warning("Legacy addresses provide less privacy than SegWit");
        }

        result
    }

    fn validate_testnet_address(&self, address: &str) -> ValidationResult {
        // Testnet address prefixes
        let valid = address.starts_with("m")   // Legacy P2PKH testnet
            || address.starts_with("n")         // Legacy P2PKH testnet
            || address.starts_with("2")         // P2SH testnet
            || address.starts_with("tb1q")      // Bech32 testnet
            || address.starts_with("tb1p"); // Bech32m testnet (Taproot)

        if !valid {
            return ValidationResult::invalid(vec![format!(
                "Invalid testnet address format: {}",
                address
            )]);
        }

        ValidationResult::valid()
    }

    fn validate_regtest_address(&self, address: &str) -> ValidationResult {
        // Regtest address prefixes (same as testnet plus bcrt1)
        let valid = address.starts_with("m")
            || address.starts_with("n")
            || address.starts_with("2")
            || address.starts_with("bcrt1q")
            || address.starts_with("bcrt1p");

        if !valid {
            return ValidationResult::invalid(vec![format!(
                "Invalid regtest address format: {}",
                address
            )]);
        }

        ValidationResult::valid()
    }

    /// Extracts the address from endpoint data.
    fn extract_address(&self, data: &EndpointData) -> Result<String> {
        let data_str = data.0.trim();

        // Try parsing as JSON first
        if data_str.starts_with('{') {
            let json: serde_json::Value = serde_json::from_str(data_str)
                .map_err(|e| PaykitError::Transport(format!("Invalid JSON: {}", e)))?;

            // Look for common address field names
            for key in ["address", "p2wpkh", "p2tr", "p2sh", "p2pkh"] {
                if let Some(addr) = json.get(key).and_then(|v| v.as_str()) {
                    return Ok(addr.to_string());
                }
            }

            return Err(PaykitError::Transport(
                "No address field found in JSON endpoint".to_string(),
            ));
        }

        // Otherwise, treat as raw address
        Ok(data_str.to_string())
    }
}

impl Default for OnchainPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Verify a Bitcoin payment proof.
///
/// This function validates that a proof corresponds to a real transaction.
/// For full verification, use a `BitcoinExecutor` to query the blockchain.
///
/// # Arguments
///
/// * `proof` - The payment proof to verify
/// * `expected_address` - The expected recipient address
/// * `expected_amount` - The expected amount in satoshis
/// * `executor` - Optional executor for blockchain verification
///
/// # Returns
///
/// Result indicating whether the proof is valid.
pub async fn verify_bitcoin_proof(
    proof: &PaymentProof,
    expected_address: &str,
    expected_amount: u64,
    executor: Option<&dyn BitcoinExecutor>,
) -> Result<bool> {
    match proof {
        PaymentProof::BitcoinTxid { txid, .. } => {
            if txid == "pending" || txid.is_empty() {
                return Ok(false);
            }

            // If we have an executor, verify against the blockchain
            if let Some(exec) = executor {
                return exec
                    .verify_transaction(txid, expected_address, expected_amount)
                    .await;
            }

            // Without executor, we can only check that txid looks valid
            // A real txid is 64 hex characters
            Ok(txid.len() == 64 && txid.chars().all(|c| c.is_ascii_hexdigit()))
        }
        _ => Err(PaykitError::Transport(
            "Invalid proof type: expected BitcoinTxid".to_string(),
        )),
    }
}

#[async_trait]
impl PaymentMethodPlugin for OnchainPlugin {
    fn method_id(&self) -> MethodId {
        MethodId("onchain".to_string())
    }

    fn display_name(&self) -> &str {
        "Bitcoin On-Chain"
    }

    fn description(&self) -> &str {
        "Pay using standard Bitcoin on-chain transactions. Supports legacy, SegWit, and Taproot addresses."
    }

    fn validate_endpoint(&self, data: &EndpointData) -> ValidationResult {
        match self.extract_address(data) {
            Ok(address) => self.validate_address(&address),
            Err(e) => ValidationResult::invalid(vec![e.to_string()]),
        }
    }

    async fn execute_payment(
        &self,
        endpoint: &EndpointData,
        amount: &Amount,
        metadata: &Value,
    ) -> Result<PaymentExecution> {
        // Extract and validate address
        let address = self.extract_address(endpoint)?;
        let validation = self.validate_address(&address);
        if !validation.valid {
            return Err(PaykitError::Transport(validation.errors.join(", ")));
        }

        // Parse amount in satoshis
        let amount_sats = amount
            .as_u64()
            .ok_or_else(|| PaykitError::Transport(format!("Invalid amount: {}", amount.value)))?;

        // Check dust limit
        if !self.supports_amount(amount) {
            return Err(PaykitError::Transport(format!(
                "Amount {} sats is below dust limit (546 sats)",
                amount_sats
            )));
        }

        // Execute payment via executor if available
        if let Some(executor) = &self.executor {
            let fee_rate = metadata.get("fee_rate").and_then(|v| v.as_f64());

            match executor
                .send_to_address(&address, amount_sats, fee_rate)
                .await
            {
                Ok(tx_result) => {
                    return Ok(PaymentExecution {
                        method_id: self.method_id(),
                        endpoint: endpoint.clone(),
                        amount: amount.clone(),
                        success: true,
                        executed_at: current_timestamp(),
                        execution_data: serde_json::json!({
                            "address": address,
                            "amount_sats": amount_sats,
                            "txid": tx_result.txid,
                            "vout": tx_result.vout,
                            "fee_sats": tx_result.fee_sats,
                            "fee_rate": tx_result.fee_rate,
                            "block_height": tx_result.block_height,
                            "confirmations": tx_result.confirmations,
                            "raw_tx": tx_result.raw_tx,
                            "metadata": metadata,
                        }),
                        error: None,
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

        // No executor configured - return mock/placeholder result
        // This is useful for validation and testing flows
        Ok(PaymentExecution {
            method_id: self.method_id(),
            endpoint: endpoint.clone(),
            amount: amount.clone(),
            success: true,
            executed_at: current_timestamp(),
            execution_data: serde_json::json!({
                "address": address,
                "amount_sats": amount_sats,
                "currency": amount.currency,
                "metadata": metadata,
                "mock": true,
                "note": "No executor configured - this is a mock result",
            }),
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

        // Extract txid from execution data
        let txid = execution
            .execution_data
            .get("txid")
            .and_then(|v| v.as_str())
            .unwrap_or("pending");

        let block_height = execution
            .execution_data
            .get("block_height")
            .and_then(|v| v.as_u64());

        let confirmations = execution
            .execution_data
            .get("confirmations")
            .and_then(|v| v.as_u64());

        Ok(PaymentProof::BitcoinTxid {
            txid: txid.to_string(),
            block_height,
            confirmations,
        })
    }

    async fn estimate_fee(&self, amount: &Amount) -> Option<Amount> {
        if let Some(executor) = &self.executor {
            let amount_sats = amount.as_u64()?;
            // Use 6 blocks as default target
            if let Ok(fee) = executor.estimate_fee("bc1qtest", amount_sats, 6).await {
                return Some(Amount::sats(fee));
            }
        }
        // Default estimate: ~1.5 sat/vB * 140 vB
        Some(Amount::sats(210))
    }

    fn format_receipt_metadata(&self, execution: &PaymentExecution) -> Value {
        let address = execution
            .execution_data
            .get("address")
            .cloned()
            .unwrap_or(Value::Null);

        let txid = execution
            .execution_data
            .get("txid")
            .cloned()
            .unwrap_or(Value::Null);

        serde_json::json!({
            "method": "onchain",
            "address": address,
            "txid": txid,
            "executed_at": execution.executed_at,
        })
    }

    fn supports_amount(&self, amount: &Amount) -> bool {
        // On-chain has dust limit (~546 sats for P2PKH, ~294 for P2WPKH)
        // Use a conservative minimum of 546 sats
        if amount.currency.to_uppercase() == "SAT" {
            if let Some(sats) = amount.as_u64() {
                return sats >= 546;
            }
        }
        true
    }

    fn estimated_confirmation_time(&self) -> Option<u64> {
        // ~10 minutes for 1 confirmation, ~60 minutes for 6 confirmations
        Some(600) // 10 minutes in seconds
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
        let plugin = OnchainPlugin::new();
        assert_eq!(plugin.method_id().0, "onchain");
    }

    #[test]
    fn test_plugin_with_executor() {
        let plugin = OnchainPlugin::with_mock_executor();
        assert!(plugin.has_executor());

        let plugin_no_exec = OnchainPlugin::new();
        assert!(!plugin_no_exec.has_executor());
    }

    #[test]
    fn test_validate_mainnet_addresses() {
        let plugin = OnchainPlugin::new();

        // Valid legacy P2PKH
        let result = plugin.validate_address("1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2");
        assert!(result.valid);
        assert!(!result.warnings.is_empty()); // Legacy warning

        // Valid P2SH
        let result = plugin.validate_address("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy");
        assert!(result.valid);

        // Valid Bech32 P2WPKH
        let result = plugin.validate_address("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq");
        assert!(result.valid);

        // Invalid - wrong prefix
        let result = plugin.validate_address("tb1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq");
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_testnet_addresses() {
        let plugin = OnchainPlugin::with_network(BitcoinNetwork::Testnet);

        // Valid testnet address
        let result = plugin.validate_address("tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx");
        assert!(result.valid);

        // Invalid - mainnet address
        let result = plugin.validate_address("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq");
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_endpoint_json() {
        let plugin = OnchainPlugin::new();

        let data =
            EndpointData(r#"{"address":"bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"}"#.to_string());
        let result = plugin.validate_endpoint(&data);
        assert!(result.valid);
    }

    #[test]
    fn test_validate_endpoint_raw() {
        let plugin = OnchainPlugin::new();

        let data = EndpointData("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".to_string());
        let result = plugin.validate_endpoint(&data);
        assert!(result.valid);
    }

    #[test]
    fn test_supports_amount() {
        let plugin = OnchainPlugin::new();

        // Above dust limit
        assert!(plugin.supports_amount(&Amount::sats(1000)));

        // Below dust limit
        assert!(!plugin.supports_amount(&Amount::sats(100)));

        // At dust limit
        assert!(plugin.supports_amount(&Amount::sats(546)));
    }

    #[test]
    fn test_confirmation_time() {
        let plugin = OnchainPlugin::new();
        assert_eq!(plugin.estimated_confirmation_time(), Some(600));
    }

    #[tokio::test]
    async fn test_execute_payment_with_mock_executor() {
        let plugin = OnchainPlugin::with_mock_executor();

        let endpoint = EndpointData("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".to_string());
        let amount = Amount::sats(10000);
        let metadata = serde_json::json!({});

        let result = plugin
            .execute_payment(&endpoint, &amount, &metadata)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.execution_data.get("txid").is_some());
        assert!(result.execution_data.get("fee_sats").is_some());
    }

    #[tokio::test]
    async fn test_execute_payment_without_executor() {
        let plugin = OnchainPlugin::new();

        let endpoint = EndpointData("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".to_string());
        let amount = Amount::sats(10000);
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
    async fn test_execute_payment_dust_rejected() {
        let plugin = OnchainPlugin::with_mock_executor();

        let endpoint = EndpointData("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".to_string());
        let amount = Amount::sats(100); // Below dust limit
        let metadata = serde_json::json!({});

        let result = plugin.execute_payment(&endpoint, &amount, &metadata).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_proof() {
        let plugin = OnchainPlugin::with_mock_executor();

        let endpoint = EndpointData("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".to_string());
        let amount = Amount::sats(10000);
        let metadata = serde_json::json!({});

        let execution = plugin
            .execute_payment(&endpoint, &amount, &metadata)
            .await
            .unwrap();
        let proof = plugin.generate_proof(&execution).unwrap();

        match proof {
            PaymentProof::BitcoinTxid { txid, .. } => {
                assert!(!txid.is_empty());
                assert_ne!(txid, "pending");
            }
            _ => panic!("Expected BitcoinTxid proof"),
        }
    }

    #[tokio::test]
    async fn test_estimate_fee() {
        let plugin = OnchainPlugin::with_mock_executor();

        let amount = Amount::sats(10000);
        let fee = plugin.estimate_fee(&amount).await;

        assert!(fee.is_some());
        assert!(fee.unwrap().as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_verify_bitcoin_proof() {
        let proof = PaymentProof::BitcoinTxid {
            txid: "abc123def456abc123def456abc123def456abc123def456abc123def456abcd".to_string(),
            block_height: Some(800000),
            confirmations: Some(6),
        };

        // Without executor, just validates txid format
        let result = verify_bitcoin_proof(&proof, "bc1qtest", 10000, None)
            .await
            .unwrap();

        assert!(result);

        // Pending txid should fail
        let pending_proof = PaymentProof::BitcoinTxid {
            txid: "pending".to_string(),
            block_height: None,
            confirmations: None,
        };

        let result = verify_bitcoin_proof(&pending_proof, "bc1qtest", 10000, None)
            .await
            .unwrap();
        assert!(!result);
    }
}
