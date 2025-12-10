//! Bitcoin On-Chain Payment Method Plugin
//!
//! This module implements the built-in Bitcoin on-chain payment method.
//! It supports legacy, SegWit, and Bech32 address formats.

use super::traits::{Amount, PaymentExecution, PaymentMethodPlugin, PaymentProof, ValidationResult};
use crate::{EndpointData, MethodId, PaykitError, Result};
use async_trait::async_trait;
use serde_json::Value;

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
}

/// Bitcoin network types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitcoinNetwork {
    /// Bitcoin mainnet.
    Mainnet,
    /// Bitcoin testnet.
    Testnet,
    /// Bitcoin regtest (local development).
    Regtest,
}

impl Default for BitcoinNetwork {
    fn default() -> Self {
        Self::Mainnet
    }
}

impl OnchainPlugin {
    /// Creates a new on-chain plugin for mainnet.
    pub fn new() -> Self {
        Self {
            network: BitcoinNetwork::Mainnet,
        }
    }

    /// Creates a new on-chain plugin for a specific network.
    pub fn with_network(network: BitcoinNetwork) -> Self {
        Self { network }
    }

    /// Returns the network this plugin is configured for.
    pub fn network(&self) -> BitcoinNetwork {
        self.network
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
            || address.starts_with("bc1p");    // Bech32m P2TR (Taproot)

        if !valid {
            return ValidationResult::invalid(vec![
                format!("Invalid mainnet address format: {}", address)
            ]);
        }

        // Basic length checks
        let mut result = ValidationResult::valid();
        
        if address.starts_with("1") || address.starts_with("3") {
            if address.len() < 26 || address.len() > 35 {
                return ValidationResult::invalid(vec![
                    format!("Invalid address length: {} (expected 26-35)", address.len())
                ]);
            }
        } else if address.starts_with("bc1q") {
            if address.len() != 42 && address.len() != 62 {
                return ValidationResult::invalid(vec![
                    format!("Invalid bech32 address length: {} (expected 42 or 62)", address.len())
                ]);
            }
        } else if address.starts_with("bc1p") {
            if address.len() != 62 {
                return ValidationResult::invalid(vec![
                    format!("Invalid taproot address length: {} (expected 62)", address.len())
                ]);
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
            || address.starts_with("tb1p");     // Bech32m testnet (Taproot)

        if !valid {
            return ValidationResult::invalid(vec![
                format!("Invalid testnet address format: {}", address)
            ]);
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
            return ValidationResult::invalid(vec![
                format!("Invalid regtest address format: {}", address)
            ]);
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
                "No address field found in JSON endpoint".to_string()
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
            return Err(PaykitError::Transport(
                validation.errors.join(", ")
            ));
        }

        // In a real implementation, this would:
        // 1. Connect to a Bitcoin node/wallet
        // 2. Create and sign a transaction
        // 3. Broadcast the transaction
        // 4. Return the txid
        
        // For now, return a placeholder execution
        // Real implementations should override this method
        Ok(PaymentExecution {
            method_id: self.method_id(),
            endpoint: endpoint.clone(),
            amount: amount.clone(),
            success: true,
            executed_at: current_timestamp(),
            execution_data: serde_json::json!({
                "address": address,
                "amount": amount.value,
                "currency": amount.currency,
                "metadata": metadata,
                // In real implementation, would include:
                // "txid": "actual_transaction_id",
                // "vout": 0,
                // "fee_rate": "1.5 sat/vB",
            }),
            error: None,
        })
    }

    fn generate_proof(&self, execution: &PaymentExecution) -> Result<PaymentProof> {
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

        Ok(PaymentProof::BitcoinTxid {
            txid: txid.to_string(),
            block_height,
            confirmations: execution.execution_data.get("confirmations").and_then(|v| v.as_u64()),
        })
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
        
        let data = EndpointData(r#"{"address":"bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"}"#.to_string());
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
}
