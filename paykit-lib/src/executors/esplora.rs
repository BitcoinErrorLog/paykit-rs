//! Esplora block explorer API executor implementation.
//!
//! Connects to Esplora-compatible APIs (Blockstream, mempool.space)
//! for on-chain Bitcoin operations.
//!
//! # Feature Flags
//!
//! This module requires the `http-executor` feature flag to be enabled for actual
//! HTTP requests. Without it, all requests return an `Unimplemented` error.
//!
//! ```toml
//! [dependencies]
//! paykit-lib = { version = "1.0", features = ["http-executor"] }
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use paykit_lib::executors::{EsploraConfig, EsploraExecutor};
//!
//! // Use a preset configuration
//! let executor = EsploraExecutor::blockstream_testnet();
//!
//! // Get fee estimates
//! let fees = executor.get_fee_estimates().await?;
//! println!("Fee for 1-block: {} sat/vB", fees.get_rate_for_blocks(1));
//!
//! // Get address balance
//! let info = executor.get_address_info("tb1q...").await?;
//! println!("Balance: {} sats", info.confirmed_balance());
//!
//! // Broadcast a signed transaction
//! let txid = executor.broadcast_tx("0200000001...").await?;
//! println!("Broadcast txid: {}", txid);
//! ```
//!
//! Note: This executor can query and verify transactions, but cannot
//! create transactions. For full send capability, pair with a wallet
//! that implements the BitcoinExecutor trait.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[cfg(feature = "http-executor")]
use std::time::Duration;

use super::config::EsploraConfig;
use crate::methods::{BitcoinExecutor, BitcoinTxResult};
use crate::{PaykitError, Result};

/// Esplora API executor for on-chain Bitcoin verification.
///
/// This executor provides read-only access to the blockchain via
/// Esplora-compatible APIs. It can verify transactions, estimate fees,
/// check balances, and broadcast pre-signed transactions.
///
/// For creating and signing transactions, use a wallet integration
/// and then broadcast via `broadcast_tx`.
///
/// # Supported APIs
///
/// - Blockstream.info
/// - mempool.space
/// - Any Esplora-compatible API
pub struct EsploraExecutor {
    config: EsploraConfig,
    #[cfg(feature = "http-executor")]
    client: reqwest::Client,
}

impl EsploraExecutor {
    /// Create a new Esplora executor with the given configuration.
    #[cfg(feature = "http-executor")]
    pub fn new(config: EsploraConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| PaykitError::Internal(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// Create a new Esplora executor with the given configuration (stub when feature disabled).
    #[cfg(not(feature = "http-executor"))]
    pub fn new(config: EsploraConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Create an executor for Blockstream mainnet.
    pub fn blockstream_mainnet() -> Result<Self> {
        Self::new(EsploraConfig::blockstream_mainnet())
    }

    /// Create an executor for Blockstream testnet.
    pub fn blockstream_testnet() -> Result<Self> {
        Self::new(EsploraConfig::blockstream_testnet())
    }

    /// Create an executor for mempool.space mainnet.
    pub fn mempool_mainnet() -> Result<Self> {
        Self::new(EsploraConfig::mempool_mainnet())
    }

    /// Create an executor for mempool.space testnet.
    pub fn mempool_testnet() -> Result<Self> {
        Self::new(EsploraConfig::mempool_testnet())
    }

    /// Get the configuration.
    pub fn config(&self) -> &EsploraConfig {
        &self.config
    }

    /// Build the full URL for an API endpoint.
    #[cfg(any(feature = "http-executor", test))]
    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.config.api_url.trim_end_matches('/'), path)
    }

    /// Make a GET request to the API.
    #[cfg(feature = "http-executor")]
    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = self.url(path);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| self.map_reqwest_error(e))?;

        self.handle_response(response).await
    }

    /// Make a GET request to the API (stub when feature disabled).
    #[cfg(not(feature = "http-executor"))]
    async fn get<T: for<'de> Deserialize<'de>>(&self, _path: &str) -> Result<T> {
        Err(PaykitError::Unimplemented(
            "Esplora HTTP client not compiled - enable the 'http-executor' feature",
        ))
    }

    /// Make a POST request with text body.
    #[cfg(feature = "http-executor")]
    async fn post_text(&self, path: &str, body: &str) -> Result<String> {
        let url = self.url(path);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "text/plain")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| self.map_reqwest_error(e))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| PaykitError::Serialization(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(self.map_status_error(status.as_u16(), &text));
        }

        Ok(text)
    }

    /// Make a POST request with text body (stub when feature disabled).
    #[cfg(not(feature = "http-executor"))]
    async fn post_text(&self, _path: &str, _body: &str) -> Result<String> {
        Err(PaykitError::Unimplemented(
            "Esplora HTTP client not compiled - enable the 'http-executor' feature",
        ))
    }

    /// Handle an HTTP response, parsing JSON or returning an error.
    #[cfg(feature = "http-executor")]
    async fn handle_response<T: for<'de> Deserialize<'de>>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(self.map_status_error(status.as_u16(), &error_text));
        }

        response.json::<T>().await.map_err(|e| {
            PaykitError::Serialization(format!("Failed to parse Esplora response: {}", e))
        })
    }

    /// Map HTTP status codes to PaykitError.
    #[cfg(feature = "http-executor")]
    fn map_status_error(&self, status: u16, error_text: &str) -> PaykitError {
        match status {
            400 => PaykitError::InvalidData {
                field: "request".to_string(),
                reason: error_text.to_string(),
            },
            404 => PaykitError::NotFound {
                resource_type: "Esplora resource".to_string(),
                identifier: error_text.to_string(),
            },
            429 => PaykitError::RateLimited {
                retry_after_ms: 5000,
            },
            500..=599 => {
                PaykitError::Internal(format!("Esplora server error ({}): {}", status, error_text))
            }
            _ => PaykitError::Transport(format!(
                "Esplora request failed ({}): {}",
                status, error_text
            )),
        }
    }

    /// Map reqwest errors to PaykitError.
    #[cfg(feature = "http-executor")]
    fn map_reqwest_error(&self, e: reqwest::Error) -> PaykitError {
        if e.is_timeout() {
            PaykitError::ConnectionTimeout {
                operation: "Esplora request".to_string(),
                timeout_ms: self.config.timeout_secs * 1000,
            }
        } else if e.is_connect() {
            PaykitError::ConnectionFailed {
                target: self.config.api_url.clone(),
                reason: e.to_string(),
            }
        } else {
            PaykitError::Transport(format!("Esplora request failed: {}", e))
        }
    }

    // ========================================================================
    // Public API Methods
    // ========================================================================

    /// Broadcast a signed transaction.
    ///
    /// # Arguments
    ///
    /// * `tx_hex` - The signed transaction in hexadecimal format
    ///
    /// # Returns
    ///
    /// The transaction ID if broadcast successfully.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let txid = executor.broadcast_tx("0200000001...").await?;
    /// println!("Broadcast: {}", txid);
    /// ```
    pub async fn broadcast_tx(&self, tx_hex: &str) -> Result<String> {
        let txid = self.post_text("tx", tx_hex).await?;
        Ok(txid.trim().to_string())
    }

    /// Get fee estimates for different confirmation targets.
    ///
    /// Returns a map of target blocks to sat/vB fee rates.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let fees = executor.get_fee_estimates().await?;
    /// let fast = fees.get_rate_for_blocks(1);    // Next block
    /// let medium = fees.get_rate_for_blocks(6);  // ~1 hour
    /// let slow = fees.get_rate_for_blocks(144);  // ~1 day
    /// ```
    pub async fn get_fee_estimates(&self) -> Result<FeeEstimates> {
        self.get("fee-estimates").await
    }

    /// Get the current blockchain tip height.
    pub async fn get_block_height(&self) -> Result<u64> {
        self.get("blocks/tip/height").await
    }

    /// Get the current blockchain tip hash.
    pub async fn get_block_hash(&self) -> Result<String> {
        self.get("blocks/tip/hash").await
    }

    /// Get address information including balance and transaction count.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let info = executor.get_address_info("bc1q...").await?;
    /// println!("Confirmed: {} sats", info.confirmed_balance());
    /// println!("Pending: {} sats", info.unconfirmed_balance());
    /// println!("Tx count: {}", info.chain_stats.tx_count);
    /// ```
    pub async fn get_address_info(&self, address: &str) -> Result<AddressInfo> {
        self.get(&format!("address/{}", address)).await
    }

    /// Get UTXOs for an address.
    ///
    /// Returns all unspent transaction outputs for the address,
    /// including both confirmed and unconfirmed.
    pub async fn get_address_utxos(&self, address: &str) -> Result<Vec<Utxo>> {
        self.get(&format!("address/{}/utxo", address)).await
    }

    /// Get transaction details by txid.
    pub async fn get_tx(&self, txid: &str) -> Result<EsploraTx> {
        self.get(&format!("tx/{}", txid)).await
    }

    /// Get raw transaction hex.
    pub async fn get_tx_hex(&self, txid: &str) -> Result<String> {
        self.get(&format!("tx/{}/hex", txid)).await
    }

    /// Get transaction status.
    pub async fn get_tx_status(&self, txid: &str) -> Result<TxStatus> {
        self.get(&format!("tx/{}/status", txid)).await
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
        let tx = match self.get_tx(txid).await {
            Ok(tx) => tx,
            Err(PaykitError::NotFound { .. }) => return Ok(None),
            Err(e) => return Err(e),
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
        let tx = match self.get_tx(txid).await {
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

// ============================================================================
// API Response Types
// ============================================================================

/// Fee estimates from Esplora API.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeeEstimates {
    /// Map of confirmation target (blocks) to fee rate (sat/vB).
    #[serde(flatten)]
    pub estimates: std::collections::HashMap<String, f64>,
}

impl FeeEstimates {
    /// Get the fee rate for a target number of blocks.
    ///
    /// If the exact target isn't available, returns the closest available estimate,
    /// preferring faster (lower block count) targets for safety.
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

    /// Get all available confirmation targets, sorted.
    pub fn targets(&self) -> Vec<u32> {
        let mut targets: Vec<u32> = self
            .estimates
            .keys()
            .filter_map(|k| k.parse().ok())
            .collect();
        targets.sort();
        targets
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
pub struct EsploraTx {
    /// Transaction ID.
    pub txid: String,
    /// Transaction version.
    #[serde(default)]
    pub version: u32,
    /// Lock time.
    #[serde(default)]
    pub locktime: u32,
    /// Transaction size in bytes.
    #[serde(default)]
    pub size: u64,
    /// Transaction weight.
    #[serde(default)]
    pub weight: u64,
    /// Fee in satoshis.
    #[serde(default)]
    pub fee: u64,
    /// Confirmation status.
    pub status: TxStatus,
    /// Transaction inputs.
    #[serde(default)]
    pub vin: Vec<EsploraTxInput>,
    /// Transaction outputs.
    #[serde(default)]
    pub vout: Vec<EsploraTxOutput>,
}

/// Transaction input.
#[derive(Clone, Debug, Deserialize)]
pub struct EsploraTxInput {
    /// Previous transaction ID.
    pub txid: String,
    /// Previous output index.
    pub vout: u32,
    /// Previous output value.
    #[serde(default)]
    pub prevout: Option<EsploraTxOutput>,
    /// Scriptsig.
    #[serde(default)]
    pub scriptsig: String,
    /// Witness data.
    #[serde(default)]
    pub witness: Vec<String>,
    /// Sequence number.
    #[serde(default)]
    pub sequence: u32,
    /// Is coinbase.
    #[serde(default)]
    pub is_coinbase: bool,
}

/// Transaction output.
#[derive(Clone, Debug, Deserialize)]
pub struct EsploraTxOutput {
    /// Value in satoshis.
    pub value: u64,
    /// Scriptpubkey hex.
    #[serde(default)]
    pub scriptpubkey: String,
    /// Scriptpubkey ASM.
    #[serde(default)]
    pub scriptpubkey_asm: String,
    /// Scriptpubkey type.
    #[serde(default)]
    pub scriptpubkey_type: String,
    /// Scriptpubkey address (if applicable).
    pub scriptpubkey_address: Option<String>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_esplora_executor_creation() {
        let executor = EsploraExecutor::blockstream_mainnet().unwrap();
        assert!(executor.config().api_url.contains("blockstream.info"));
    }

    #[test]
    fn test_url_building() {
        let executor =
            EsploraExecutor::new(EsploraConfig::new("https://api.example.com/")).unwrap();
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
    fn test_fee_estimates_targets() {
        let mut estimates = std::collections::HashMap::new();
        estimates.insert("1".to_string(), 50.0);
        estimates.insert("6".to_string(), 10.0);
        estimates.insert("144".to_string(), 1.0);

        let fee_estimates = FeeEstimates { estimates };
        let targets = fee_estimates.targets();

        assert_eq!(targets, vec![1, 6, 144]);
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

    #[test]
    fn test_preset_configs() {
        let mainnet = EsploraExecutor::blockstream_mainnet().unwrap();
        assert!(mainnet.config().api_url.contains("blockstream.info"));
        assert!(!mainnet.config().api_url.contains("testnet"));

        let testnet = EsploraExecutor::blockstream_testnet().unwrap();
        assert!(testnet.config().api_url.contains("testnet"));

        let mempool = EsploraExecutor::mempool_mainnet().unwrap();
        assert!(mempool.config().api_url.contains("mempool.space"));
    }
}
