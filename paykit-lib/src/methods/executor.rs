//! Payment Executor Traits
//!
//! This module defines the executor traits that enable external wallet integration.
//! Payment plugins delegate actual payment execution to these executors, allowing
//! any wallet implementation to be plugged in.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    PaymentMethodPlugin                       │
//! │  (OnchainPlugin / LightningPlugin)                          │
//! │  ┌─────────────────────────────────────────────────────────┐ │
//! │  │                   PaymentExecutor                        │ │
//! │  │  - BitcoinExecutor (for on-chain)                       │ │
//! │  │  - LightningExecutor (for Lightning)                    │ │
//! │  │                                                          │ │
//! │  │  Implementations:                                        │ │
//! │  │  - MockExecutor (testing)                               │ │
//! │  │  - Your custom wallet integration                       │ │
//! │  └─────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use paykit_lib::methods::{BitcoinExecutor, OnchainPlugin};
//!
//! // Implement your wallet executor
//! struct MyWalletExecutor { /* ... */ }
//!
//! #[async_trait]
//! impl BitcoinExecutor for MyWalletExecutor {
//!     async fn send_to_address(
//!         &self,
//!         address: &str,
//!         amount_sats: u64,
//!         fee_rate: Option<f64>,
//!     ) -> Result<BitcoinTxResult> {
//!         // Your wallet implementation
//!     }
//!     // ...
//! }
//!
//! // Use it with the plugin
//! let executor = Arc::new(MyWalletExecutor::new());
//! let plugin = OnchainPlugin::with_executor(executor);
//! ```

use crate::{PaykitError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Result of a Bitcoin on-chain transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BitcoinTxResult {
    /// The transaction ID (hex-encoded).
    pub txid: String,
    /// The raw transaction hex (optional).
    pub raw_tx: Option<String>,
    /// The output index used for payment.
    pub vout: u32,
    /// The fee paid in satoshis.
    pub fee_sats: u64,
    /// The fee rate in sat/vB.
    pub fee_rate: f64,
    /// Block height if confirmed.
    pub block_height: Option<u64>,
    /// Number of confirmations.
    pub confirmations: u64,
}

impl BitcoinTxResult {
    /// Create a new transaction result.
    pub fn new(txid: impl Into<String>, vout: u32, fee_sats: u64, fee_rate: f64) -> Self {
        Self {
            txid: txid.into(),
            raw_tx: None,
            vout,
            fee_sats,
            fee_rate,
            block_height: None,
            confirmations: 0,
        }
    }

    /// Check if the transaction is confirmed.
    pub fn is_confirmed(&self) -> bool {
        self.confirmations > 0
    }
}

/// Result of a Lightning payment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LightningPaymentResult {
    /// The payment preimage (hex-encoded).
    pub preimage: String,
    /// The payment hash (hex-encoded).
    pub payment_hash: String,
    /// The amount paid in millisatoshis.
    pub amount_msat: u64,
    /// The fee paid in millisatoshis.
    pub fee_msat: u64,
    /// Number of hops in the route.
    pub hops: u32,
    /// Payment status.
    pub status: LightningPaymentStatus,
}

/// Status of a Lightning payment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightningPaymentStatus {
    /// Payment succeeded.
    Succeeded,
    /// Payment is pending.
    Pending,
    /// Payment failed.
    Failed,
}

impl LightningPaymentResult {
    /// Create a successful payment result.
    pub fn success(
        preimage: impl Into<String>,
        payment_hash: impl Into<String>,
        amount_msat: u64,
        fee_msat: u64,
    ) -> Self {
        Self {
            preimage: preimage.into(),
            payment_hash: payment_hash.into(),
            amount_msat,
            fee_msat,
            hops: 0,
            status: LightningPaymentStatus::Succeeded,
        }
    }
}

/// Executor trait for Bitcoin on-chain payments.
///
/// Implement this trait to integrate your Bitcoin wallet with Paykit.
#[async_trait]
pub trait BitcoinExecutor: Send + Sync {
    /// Send Bitcoin to an address.
    ///
    /// # Arguments
    ///
    /// * `address` - The destination Bitcoin address
    /// * `amount_sats` - The amount to send in satoshis
    /// * `fee_rate` - Optional fee rate in sat/vB (uses wallet default if None)
    ///
    /// # Returns
    ///
    /// Transaction result with txid and details.
    async fn send_to_address(
        &self,
        address: &str,
        amount_sats: u64,
        fee_rate: Option<f64>,
    ) -> Result<BitcoinTxResult>;

    /// Estimate the fee for a transaction.
    ///
    /// # Arguments
    ///
    /// * `address` - The destination address
    /// * `amount_sats` - The amount to send
    /// * `target_blocks` - Confirmation target in blocks (1, 3, 6, etc.)
    ///
    /// # Returns
    ///
    /// Estimated fee in satoshis.
    async fn estimate_fee(
        &self,
        address: &str,
        amount_sats: u64,
        target_blocks: u32,
    ) -> Result<u64>;

    /// Get transaction details by txid.
    ///
    /// # Arguments
    ///
    /// * `txid` - The transaction ID
    ///
    /// # Returns
    ///
    /// Transaction details if found.
    async fn get_transaction(&self, txid: &str) -> Result<Option<BitcoinTxResult>>;

    /// Verify a transaction was sent to the expected address and amount.
    ///
    /// # Arguments
    ///
    /// * `txid` - The transaction ID
    /// * `address` - Expected destination address
    /// * `amount_sats` - Expected amount
    ///
    /// # Returns
    ///
    /// True if the transaction matches.
    async fn verify_transaction(
        &self,
        txid: &str,
        address: &str,
        amount_sats: u64,
    ) -> Result<bool>;
}

/// Executor trait for Lightning Network payments.
///
/// Implement this trait to integrate your Lightning node/wallet with Paykit.
#[async_trait]
pub trait LightningExecutor: Send + Sync {
    /// Pay a BOLT11 invoice.
    ///
    /// # Arguments
    ///
    /// * `invoice` - The BOLT11 invoice string
    /// * `amount_msat` - Optional amount in millisatoshis (for zero-amount invoices)
    /// * `max_fee_msat` - Maximum fee willing to pay in millisatoshis
    ///
    /// # Returns
    ///
    /// Payment result with preimage.
    async fn pay_invoice(
        &self,
        invoice: &str,
        amount_msat: Option<u64>,
        max_fee_msat: Option<u64>,
    ) -> Result<LightningPaymentResult>;

    /// Decode a BOLT11 invoice without paying.
    ///
    /// # Arguments
    ///
    /// * `invoice` - The BOLT11 invoice string
    ///
    /// # Returns
    ///
    /// Decoded invoice details.
    async fn decode_invoice(&self, invoice: &str) -> Result<DecodedInvoice>;

    /// Estimate the fee for paying an invoice.
    ///
    /// # Arguments
    ///
    /// * `invoice` - The BOLT11 invoice
    ///
    /// # Returns
    ///
    /// Estimated fee in millisatoshis.
    async fn estimate_fee(&self, invoice: &str) -> Result<u64>;

    /// Check the status of a payment by payment hash.
    ///
    /// # Arguments
    ///
    /// * `payment_hash` - The payment hash (hex-encoded)
    ///
    /// # Returns
    ///
    /// Payment result if found.
    async fn get_payment(&self, payment_hash: &str) -> Result<Option<LightningPaymentResult>>;

    /// Verify a payment was made (check preimage matches hash).
    ///
    /// # Arguments
    ///
    /// * `preimage` - The payment preimage (hex-encoded)
    /// * `payment_hash` - The payment hash (hex-encoded)
    ///
    /// # Returns
    ///
    /// True if preimage matches hash.
    fn verify_preimage(&self, preimage: &str, payment_hash: &str) -> bool {
        // Default implementation using SHA256
        // Decode preimage from hex
        let preimage_bytes = match hex_decode(preimage) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        
        // Compute SHA256 hash
        let computed_hash = sha256(&preimage_bytes);
        
        // Compare with expected hash
        let expected_hash = match hex_decode(payment_hash) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        
        computed_hash == expected_hash
    }
}

/// Decoded BOLT11 invoice details.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecodedInvoice {
    /// The payment hash.
    pub payment_hash: String,
    /// Amount in millisatoshis (None for zero-amount invoices).
    pub amount_msat: Option<u64>,
    /// Invoice description.
    pub description: Option<String>,
    /// Description hash.
    pub description_hash: Option<String>,
    /// Payee public key.
    pub payee: String,
    /// Expiry time in seconds.
    pub expiry: u64,
    /// Creation timestamp.
    pub timestamp: u64,
    /// Whether the invoice has expired.
    pub expired: bool,
}

/// Mock Bitcoin executor for testing.
///
/// This executor simulates successful payments without actually sending transactions.
#[derive(Default)]
pub struct MockBitcoinExecutor {
    /// Whether to simulate failures.
    pub simulate_failure: bool,
    /// Fixed txid to return.
    pub mock_txid: Option<String>,
}

impl MockBitcoinExecutor {
    /// Create a new mock executor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a mock that simulates failures.
    pub fn failing() -> Self {
        Self {
            simulate_failure: true,
            mock_txid: None,
        }
    }

    /// Set a fixed txid to return.
    pub fn with_txid(txid: impl Into<String>) -> Self {
        Self {
            simulate_failure: false,
            mock_txid: Some(txid.into()),
        }
    }
}

#[async_trait]
impl BitcoinExecutor for MockBitcoinExecutor {
    async fn send_to_address(
        &self,
        address: &str,
        amount_sats: u64,
        fee_rate: Option<f64>,
    ) -> Result<BitcoinTxResult> {
        if self.simulate_failure {
            return Err(PaykitError::Transport("Simulated failure".to_string()));
        }

        let txid = self.mock_txid.clone().unwrap_or_else(|| {
            // Generate a mock txid based on inputs
            format!(
                "{:064x}",
                simple_hash(&format!("{}:{}:{}", address, amount_sats, current_timestamp()))
            )
        });

        let fee_rate = fee_rate.unwrap_or(1.5);
        let fee_sats = (fee_rate * 140.0) as u64; // Assume ~140 vB transaction

        Ok(BitcoinTxResult::new(txid, 0, fee_sats, fee_rate))
    }

    async fn estimate_fee(
        &self,
        _address: &str,
        _amount_sats: u64,
        target_blocks: u32,
    ) -> Result<u64> {
        // Mock fee estimation based on target blocks
        let sat_per_vb = match target_blocks {
            1 => 10.0,
            2..=3 => 5.0,
            4..=6 => 2.0,
            _ => 1.0,
        };
        Ok((sat_per_vb * 140.0) as u64)
    }

    async fn get_transaction(&self, txid: &str) -> Result<Option<BitcoinTxResult>> {
        // Return a mock confirmed transaction
        Ok(Some(BitcoinTxResult {
            txid: txid.to_string(),
            raw_tx: None,
            vout: 0,
            fee_sats: 210,
            fee_rate: 1.5,
            block_height: Some(800000),
            confirmations: 6,
        }))
    }

    async fn verify_transaction(
        &self,
        _txid: &str,
        _address: &str,
        _amount_sats: u64,
    ) -> Result<bool> {
        Ok(!self.simulate_failure)
    }
}

/// Mock Lightning executor for testing.
#[derive(Default)]
pub struct MockLightningExecutor {
    /// Whether to simulate failures.
    pub simulate_failure: bool,
    /// Fixed preimage to return.
    pub mock_preimage: Option<String>,
}

impl MockLightningExecutor {
    /// Create a new mock executor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a mock that simulates failures.
    pub fn failing() -> Self {
        Self {
            simulate_failure: true,
            mock_preimage: None,
        }
    }

    /// Set a fixed preimage to return.
    pub fn with_preimage(preimage: impl Into<String>) -> Self {
        Self {
            simulate_failure: false,
            mock_preimage: Some(preimage.into()),
        }
    }
}

#[async_trait]
impl LightningExecutor for MockLightningExecutor {
    async fn pay_invoice(
        &self,
        invoice: &str,
        amount_msat: Option<u64>,
        _max_fee_msat: Option<u64>,
    ) -> Result<LightningPaymentResult> {
        if self.simulate_failure {
            return Err(PaykitError::Transport("Simulated failure".to_string()));
        }

        let preimage = self.mock_preimage.clone().unwrap_or_else(|| {
            format!("{:064x}", simple_hash(&format!("preimage:{}", invoice)))
        });

        let payment_hash = format!("{:064x}", simple_hash(&preimage));
        let amount = amount_msat.unwrap_or(1000);

        Ok(LightningPaymentResult::success(
            preimage,
            payment_hash,
            amount,
            (amount as f64 * 0.001) as u64, // 0.1% fee
        ))
    }

    async fn decode_invoice(&self, invoice: &str) -> Result<DecodedInvoice> {
        // Return mock decoded invoice
        Ok(DecodedInvoice {
            payment_hash: format!("{:064x}", simple_hash(invoice)),
            amount_msat: Some(1000000), // 1000 sats
            description: Some("Mock invoice".to_string()),
            description_hash: None,
            payee: format!("{:066x}", simple_hash(&format!("payee:{}", invoice))),
            expiry: 3600,
            timestamp: current_timestamp() as u64,
            expired: false,
        })
    }

    async fn estimate_fee(&self, _invoice: &str) -> Result<u64> {
        // Return 0.1% of assumed 1000 sat payment
        Ok(1000)
    }

    async fn get_payment(&self, payment_hash: &str) -> Result<Option<LightningPaymentResult>> {
        let preimage = self.mock_preimage.clone().unwrap_or_else(|| {
            format!("{:064x}", simple_hash(&format!("preimage_for:{}", payment_hash)))
        });

        Ok(Some(LightningPaymentResult::success(
            preimage,
            payment_hash.to_string(),
            1000000,
            1000,
        )))
    }
}

/// Simple hash function for mock data generation.
fn simple_hash(data: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in data.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

/// Get current timestamp.
fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Decode hex string to bytes.
fn hex_decode(hex: &str) -> std::result::Result<Vec<u8>, &'static str> {
    if hex.len() % 2 != 0 {
        return Err("Invalid hex length");
    }
    
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| "Invalid hex character")
        })
        .collect()
}

/// Simple SHA256 implementation for preimage verification.
/// In production, use a proper crypto library.
fn sha256(data: &[u8]) -> Vec<u8> {
    // This is a placeholder. In a real implementation, use:
    // use sha2::{Sha256, Digest};
    // Sha256::digest(data).to_vec()
    
    // For now, return a mock hash based on data
    let mut hash = [0u8; 32];
    for (i, byte) in data.iter().enumerate() {
        hash[i % 32] ^= byte;
    }
    hash.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_bitcoin_executor() {
        let executor = MockBitcoinExecutor::new();
        
        let result = executor
            .send_to_address("bc1qtest...", 10000, None)
            .await
            .unwrap();
        
        assert!(!result.txid.is_empty());
        assert_eq!(result.vout, 0);
        assert!(result.fee_sats > 0);
    }

    #[tokio::test]
    async fn test_mock_bitcoin_executor_with_txid() {
        let executor = MockBitcoinExecutor::with_txid("abc123");
        
        let result = executor
            .send_to_address("bc1qtest...", 10000, None)
            .await
            .unwrap();
        
        assert_eq!(result.txid, "abc123");
    }

    #[tokio::test]
    async fn test_mock_bitcoin_executor_failure() {
        let executor = MockBitcoinExecutor::failing();
        
        let result = executor.send_to_address("bc1qtest...", 10000, None).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_lightning_executor() {
        let executor = MockLightningExecutor::new();
        
        let result = executor
            .pay_invoice("lnbc1...", None, None)
            .await
            .unwrap();
        
        assert!(!result.preimage.is_empty());
        assert!(!result.payment_hash.is_empty());
        assert_eq!(result.status, LightningPaymentStatus::Succeeded);
    }

    #[tokio::test]
    async fn test_mock_lightning_decode_invoice() {
        let executor = MockLightningExecutor::new();
        
        let decoded = executor.decode_invoice("lnbc1...").await.unwrap();
        
        assert!(!decoded.payment_hash.is_empty());
        assert!(decoded.amount_msat.is_some());
        assert!(!decoded.expired);
    }

    #[test]
    fn test_bitcoin_tx_result() {
        let result = BitcoinTxResult::new("abc123", 0, 210, 1.5);
        
        assert_eq!(result.txid, "abc123");
        assert!(!result.is_confirmed());
    }

    #[test]
    fn test_lightning_payment_result() {
        let result = LightningPaymentResult::success("preimage", "hash", 1000, 10);
        
        assert_eq!(result.status, LightningPaymentStatus::Succeeded);
        assert_eq!(result.amount_msat, 1000);
    }
}
