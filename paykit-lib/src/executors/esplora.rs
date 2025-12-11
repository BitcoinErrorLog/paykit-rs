//! Esplora block explorer API executor implementation.
//!
//! Connects to Esplora-compatible APIs (Blockstream, mempool.space)
//! for on-chain Bitcoin operations.
//!
//! Note: This executor can query and verify transactions, but cannot
//! create transactions. For full send capability, pair with a wallet
//! that implements the BitcoinExecutor trait.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::config::EsploraConfig;
use crate::methods::{BitcoinExecutor, BitcoinTxResult};
use crate::{PaykitError, Result};

/// Esplora API executor for on-chain Bitcoin verification.
///
/// This executor provides read-only access to the blockchain via
/// Esplora-compatible APIs. It can verify transactions and estimate
/// fees, but cannot send transactions directly.
///
/// For sending transactions, use this in combination with a wallet
/// integration or use the `broadcast_tx` method to broadcast a
/// pre-signed transaction.
pub struct EsploraExecutor {
    config: EsploraConfig,
}

impl EsploraExecutor {
    /// Create a new Esplora executor with the given configuration.
    pub fn new(config: EsploraConfig) -> Self {
        Self { config }
    }

    /// Create an executor for Blockstream mainnet.
    pub fn blockstream_mainnet() -> Self {
        Self::new(EsploraConfig::blockstream_mainnet())
    }

    /// Create an executor for Blockstream testnet.
    pub fn blockstream_testnet() -> Self {
        Self::new(EsploraConfig::blockstream_testnet())
    }

    /// Create an executor for mempool.space mainnet.
    pub fn mempool_mainnet() -> Self {
        Self::new(EsploraConfig::mempool_mainnet())
    }

    /// Create an executor for mempool.space testnet.
    pub fn mempool_testnet() -> Self {
        Self::new(EsploraConfig::mempool_testnet())
    }

    /// Get the configuration.
    pub fn config(&self) -> &EsploraConfig {
        &self.config
    }

    /// Build the full URL for an API endpoint.
    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.config.api_url.trim_end_matches('/'), path)
    }

    /// Make a GET request to the API.
    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        // Stub implementation - would use reqwest in full version
        let _ = path;
        Err(PaykitError::Unimplemented(
            "Esplora HTTP client not compiled - add reqwest dependency",
        ))
    }

    /// Make a POST request to the API.
    async fn post_text(&self, path: &str, body: &str) -> Result<String> {
        // Stub implementation - would use reqwest in full version
        let _ = (path, body);
        Err(PaykitError::Unimplemented(
            "Esplora HTTP client not compiled - add reqwest dependency",
        ))
    }

    /// Broadcast a signed transaction.
    ///
    /// # Arguments
    /// * `tx_hex` - The signed transaction in hexadecimal format
    ///
    /// # Returns
    /// The transaction ID if broadcast successfully.
    pub async fn broadcast_tx(&self, tx_hex: &str) -> Result<String> {
        let txid = self.post_text("tx", tx_hex).await?;
        Ok(txid.trim().to_string())
    }

    /// Get fee estimates for different confirmation targets.
    ///
    /// Returns a map of target blocks to sat/vB fee rates.
    pub async fn get_fee_estimates(&self) -> Result<FeeEstimates> {
        self.get("fee-estimates").await
    }

    /// Get the current blockchain tip height.
    pub async fn get_block_height(&self) -> Result<u64> {
        self.get("blocks/tip/height").await
    }

    /// Get address information including balance and tx count.
    pub async fn get_address_info(&self, address: &str) -> Result<AddressInfo> {
        self.get(&format!("address/{}", address)).await
    }

    /// Get UTXOs for an address.
    pub async fn get_address_utxos(&self, address: &str) -> Result<Vec<Utxo>> {
        self.get(&format!("address/{}/utxo", address)).await
    }
}

#[async_trait]
impl BitcoinExecutor for EsploraExecutor {
    async fn send_to_address(
        &self,
        _address: &str,
        _amount_sats: u64,
        _fee_rate: Option<f64>,
    ) -> Result<BitcoinTxResult> {
        // Esplora is read-only - cannot create transactions
        // Must use a wallet to sign and then broadcast via broadcast_tx
        Err(PaykitError::Unimplemented(
            "Esplora executor is read-only. Use a wallet to create and sign transactions, \
             then broadcast via EsploraExecutor::broadcast_tx()",
        ))
    }

    async fn estimate_fee(
        &self,
        _address: &str,
        _amount_sats: u64,
        target_blocks: u32,
    ) -> Result<u64> {
        let estimates = self.get_fee_estimates().await?;

        // Find the appropriate fee rate for target blocks
        let fee_rate = estimates.get_rate_for_blocks(target_blocks);

        // Assume ~140 vB for a simple P2WPKH transaction
        let estimated_vsize = 140u64;
        let fee_sats = (fee_rate * estimated_vsize as f64).ceil() as u64;

        Ok(fee_sats)
    }

    async fn get_transaction(&self, txid: &str) -> Result<Option<BitcoinTxResult>> {
        let tx: EsploraTx = match self.get(&format!("tx/{}", txid)).await {
            Ok(tx) => tx,
            Err(e) => {
                // Check if it's a 404 (not found)
                if e.to_string().contains("404") {
                    return Ok(None);
                }
                return Err(e);
            }
        };

        // Get current block height to calculate confirmations
        let current_height = self.get_block_height().await.unwrap_or(0);
        let confirmations = tx
            .status
            .block_height
            .map(|h| current_height.saturating_sub(h) + 1)
            .unwrap_or(0);

        Ok(Some(BitcoinTxResult {
            txid: tx.txid,
            raw_tx: None,
            vout: 0, // Would need to be determined from context
            fee_sats: tx.fee,
            fee_rate: if tx.size > 0 {
                tx.fee as f64 / tx.size as f64
            } else {
                0.0
            },
            block_height: tx.status.block_height,
            confirmations,
        }))
    }

    async fn verify_transaction(
        &self,
        txid: &str,
        address: &str,
        amount_sats: u64,
    ) -> Result<bool> {
        let tx: EsploraTx = match self.get(&format!("tx/{}", txid)).await {
            Ok(tx) => tx,
            Err(_) => return Ok(false),
        };

        // Check if any output matches the address and amount
        for output in tx.vout {
            if let Some(ref script_pubkey) = output.scriptpubkey_address {
                if script_pubkey == address && output.value == amount_sats {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

/// Fee estimates from Esplora API.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeeEstimates {
    /// Map of confirmation target (blocks) to fee rate (sat/vB).
    #[serde(flatten)]
    pub estimates: std::collections::HashMap<String, f64>,
}

impl FeeEstimates {
    /// Get the fee rate for a target number of blocks.
    pub fn get_rate_for_blocks(&self, target_blocks: u32) -> f64 {
        // Try exact match first
        if let Some(&rate) = self.estimates.get(&target_blocks.to_string()) {
            return rate;
        }

        // Find closest available target
        let mut closest_key = 1;
        let mut closest_diff = u32::MAX;

        for key in self.estimates.keys() {
            if let Ok(k) = key.parse::<u32>() {
                let diff = (k as i32 - target_blocks as i32).unsigned_abs();
                // Prefer lower block counts on ties (faster confirmation, safer)
                if diff < closest_diff || (diff == closest_diff && k < closest_key) {
                    closest_diff = diff;
                    closest_key = k;
                }
            }
        }

        self.estimates
            .get(&closest_key.to_string())
            .copied()
            .unwrap_or(1.0)
    }
}

/// Address information from Esplora API.
#[derive(Clone, Debug, Deserialize)]
pub struct AddressInfo {
    /// The address.
    pub address: String,
    /// Chain stats (confirmed).
    pub chain_stats: AddressStats,
    /// Mempool stats (unconfirmed).
    pub mempool_stats: AddressStats,
}

impl AddressInfo {
    /// Get confirmed balance in satoshis.
    pub fn confirmed_balance(&self) -> i64 {
        self.chain_stats.funded_txo_sum as i64 - self.chain_stats.spent_txo_sum as i64
    }

    /// Get unconfirmed balance in satoshis.
    pub fn unconfirmed_balance(&self) -> i64 {
        self.mempool_stats.funded_txo_sum as i64 - self.mempool_stats.spent_txo_sum as i64
    }

    /// Get total balance (confirmed + unconfirmed) in satoshis.
    pub fn total_balance(&self) -> i64 {
        self.confirmed_balance() + self.unconfirmed_balance()
    }
}

/// Address statistics.
#[derive(Clone, Debug, Deserialize)]
pub struct AddressStats {
    /// Number of funded transaction outputs.
    pub funded_txo_count: u64,
    /// Sum of funded transaction outputs in satoshis.
    pub funded_txo_sum: u64,
    /// Number of spent transaction outputs.
    pub spent_txo_count: u64,
    /// Sum of spent transaction outputs in satoshis.
    pub spent_txo_sum: u64,
    /// Number of transactions.
    pub tx_count: u64,
}

/// UTXO information.
#[derive(Clone, Debug, Deserialize)]
pub struct Utxo {
    /// Transaction ID.
    pub txid: String,
    /// Output index.
    pub vout: u32,
    /// Value in satoshis.
    pub value: u64,
    /// Confirmation status.
    pub status: TxStatus,
}

/// Transaction status.
#[derive(Clone, Debug, Deserialize)]
pub struct TxStatus {
    /// Whether the transaction is confirmed.
    pub confirmed: bool,
    /// Block height if confirmed.
    pub block_height: Option<u64>,
    /// Block hash if confirmed.
    pub block_hash: Option<String>,
    /// Block time if confirmed.
    pub block_time: Option<u64>,
}

/// Transaction from Esplora API.
#[derive(Clone, Debug, Deserialize)]
struct EsploraTx {
    txid: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    fee: u64,
    status: TxStatus,
    #[serde(default)]
    vout: Vec<EsploraTxOutput>,
}

#[derive(Clone, Debug, Deserialize)]
struct EsploraTxOutput {
    value: u64,
    scriptpubkey_address: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_esplora_executor_creation() {
        let executor = EsploraExecutor::blockstream_mainnet();
        assert!(executor.config().api_url.contains("blockstream.info"));
    }

    #[test]
    fn test_url_building() {
        let executor = EsploraExecutor::new(EsploraConfig::new("https://api.example.com/"));
        assert_eq!(
            executor.url("tx/abc123"),
            "https://api.example.com/tx/abc123"
        );
    }

    #[test]
    fn test_fee_estimates() {
        let mut estimates = std::collections::HashMap::new();
        estimates.insert("1".to_string(), 50.0);
        estimates.insert("3".to_string(), 25.0);
        estimates.insert("6".to_string(), 10.0);
        estimates.insert("144".to_string(), 1.0);

        let fee_estimates = FeeEstimates { estimates };

        assert_eq!(fee_estimates.get_rate_for_blocks(1), 50.0);
        assert_eq!(fee_estimates.get_rate_for_blocks(3), 25.0);
        // 2 is equidistant from 1 and 3, prefer lower (1) for safety
        assert_eq!(fee_estimates.get_rate_for_blocks(2), 50.0);
        // 4 is closer to 3 than to 6
        assert_eq!(fee_estimates.get_rate_for_blocks(4), 25.0);
        // 100 blocks is closest to 144, so should get its rate
        assert_eq!(fee_estimates.get_rate_for_blocks(100), 1.0);
    }

    #[test]
    fn test_address_balance() {
        let info = AddressInfo {
            address: "bc1q...".to_string(),
            chain_stats: AddressStats {
                funded_txo_count: 5,
                funded_txo_sum: 100000,
                spent_txo_count: 3,
                spent_txo_sum: 50000,
                tx_count: 8,
            },
            mempool_stats: AddressStats {
                funded_txo_count: 1,
                funded_txo_sum: 10000,
                spent_txo_count: 0,
                spent_txo_sum: 0,
                tx_count: 1,
            },
        };

        assert_eq!(info.confirmed_balance(), 50000);
        assert_eq!(info.unconfirmed_balance(), 10000);
        assert_eq!(info.total_balance(), 60000);
    }
}
