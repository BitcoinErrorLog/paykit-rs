//! Executor FFI Bindings for Wallet Integration
//!
//! This module provides FFI callback interfaces that allow mobile wallets (like Bitkit)
//! to provide their wallet functionality as payment executors.
//!
//! # Architecture
//!
//! Mobile apps implement the callback interfaces (`BitcoinExecutorFFI`, `LightningExecutorFFI`)
//! in Swift/Kotlin. These implementations are registered with `PaykitClient` and bridged
//! to Rust executor traits via `BitcoinExecutorBridge` and `LightningExecutorBridge`.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     Mobile App (Swift/Kotlin)                    │
//! │  ┌─────────────────────────────────────────────────────────────┐ │
//! │  │  BitcoinExecutorFFI / LightningExecutorFFI Implementation   │ │
//! │  │  (Wraps actual wallet - e.g., Bitkit wallet)                │ │
//! │  └─────────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼ (UniFFI callback)
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      Rust FFI Layer                              │
//! │  ┌─────────────────────────────────────────────────────────────┐ │
//! │  │  BitcoinExecutorBridge / LightningExecutorBridge            │ │
//! │  │  (Implements paykit_lib::methods::BitcoinExecutor, etc.)    │ │
//! │  └─────────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Payment Method Plugin                         │
//! │  OnchainPlugin::with_executor() / LightningPlugin::with_executor│
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example (Swift)
//!
//! ```swift
//! class BitkitBitcoinExecutor: BitcoinExecutorFFI {
//!     let wallet: BitkitWallet
//!
//!     func sendToAddress(address: String, amountSats: UInt64, feeRate: Double?) throws -> BitcoinTxResultFFI {
//!         let tx = try wallet.send(to: address, amount: amountSats, feeRate: feeRate)
//!         return BitcoinTxResultFFI(
//!             txid: tx.txid,
//!             rawTx: tx.rawHex,
//!             vout: tx.outputIndex,
//!             feeSats: tx.fee,
//!             feeRate: tx.feeRate,
//!             blockHeight: tx.blockHeight,
//!             confirmations: tx.confirmations
//!         )
//!     }
//!     // ... other methods
//! }
//!
//! let client = PaykitClient.new()
//! let executor = BitkitBitcoinExecutor(wallet: myWallet)
//! try client.registerBitcoinExecutor(executor: executor)
//! ```

use std::sync::Arc;

use crate::PaykitMobileError;

// ============================================================================
// Result Types for FFI
// ============================================================================

/// Result of a Bitcoin on-chain transaction (FFI-compatible).
///
/// This type is returned by `BitcoinExecutorFFI::sendToAddress()` after
/// successfully broadcasting a transaction.
#[derive(Clone, Debug, uniffi::Record)]
pub struct BitcoinTxResultFFI {
    /// The transaction ID (hex-encoded, 64 characters).
    pub txid: String,

    /// The raw transaction hex (optional, for debugging/verification).
    pub raw_tx: Option<String>,

    /// The output index used for payment.
    pub vout: u32,

    /// The fee paid in satoshis.
    pub fee_sats: u64,

    /// The fee rate in sat/vB.
    pub fee_rate: f64,

    /// Block height if confirmed (None if unconfirmed).
    pub block_height: Option<u64>,

    /// Number of confirmations.
    pub confirmations: u64,
}

impl BitcoinTxResultFFI {
    /// Create a new transaction result.
    pub fn new(txid: String, vout: u32, fee_sats: u64, fee_rate: f64) -> Self {
        Self {
            txid,
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

/// Result of a Lightning payment (FFI-compatible).
///
/// This type is returned by `LightningExecutorFFI::payInvoice()` after
/// a successful Lightning payment.
#[derive(Clone, Debug, uniffi::Record)]
pub struct LightningPaymentResultFFI {
    /// The payment preimage (hex-encoded, 64 characters).
    /// This is the cryptographic proof of payment.
    pub preimage: String,

    /// The payment hash (hex-encoded, 64 characters).
    pub payment_hash: String,

    /// The amount paid in millisatoshis.
    pub amount_msat: u64,

    /// The fee paid in millisatoshis.
    pub fee_msat: u64,

    /// Number of hops in the payment route.
    pub hops: u32,

    /// Payment status.
    pub status: LightningPaymentStatusFFI,
}

impl LightningPaymentResultFFI {
    /// Create a successful payment result.
    pub fn success(
        preimage: String,
        payment_hash: String,
        amount_msat: u64,
        fee_msat: u64,
    ) -> Self {
        Self {
            preimage,
            payment_hash,
            amount_msat,
            fee_msat,
            hops: 0,
            status: LightningPaymentStatusFFI::Succeeded,
        }
    }
}

/// Status of a Lightning payment (FFI-compatible).
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum LightningPaymentStatusFFI {
    /// Payment succeeded.
    Succeeded,
    /// Payment is pending/in-flight.
    Pending,
    /// Payment failed.
    Failed,
}

/// Decoded BOLT11 invoice details (FFI-compatible).
///
/// This type is returned by `LightningExecutorFFI::decodeInvoice()`.
#[derive(Clone, Debug, uniffi::Record)]
pub struct DecodedInvoiceFFI {
    /// The payment hash (hex-encoded).
    pub payment_hash: String,

    /// Amount in millisatoshis (None for zero-amount invoices).
    pub amount_msat: Option<u64>,

    /// Invoice description.
    pub description: Option<String>,

    /// Description hash (for invoices with hashed descriptions).
    pub description_hash: Option<String>,

    /// Payee public key (hex-encoded).
    pub payee: String,

    /// Expiry time in seconds.
    pub expiry: u64,

    /// Creation timestamp (Unix epoch seconds).
    pub timestamp: u64,

    /// Whether the invoice has expired.
    pub expired: bool,
}

// ============================================================================
// Callback Interfaces
// ============================================================================

/// Bitcoin executor callback interface for mobile wallets.
///
/// Implement this interface in Swift/Kotlin to provide on-chain Bitcoin
/// payment capabilities to Paykit.
///
/// # Thread Safety
///
/// All methods may be called from any thread. Implementations must be
/// thread-safe.
///
/// # Error Handling
///
/// Return `PaykitMobileError` for failures. The error will be propagated
/// to the caller.
#[uniffi::export(callback_interface)]
pub trait BitcoinExecutorFFI: Send + Sync {
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
    /// Transaction result with txid and fee details.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The address is invalid
    /// - Insufficient funds
    /// - Network error
    /// - Wallet is locked
    fn send_to_address(
        &self,
        address: String,
        amount_sats: u64,
        fee_rate: Option<f64>,
    ) -> Result<BitcoinTxResultFFI, PaykitMobileError>;

    /// Estimate the fee for a transaction.
    ///
    /// # Arguments
    ///
    /// * `address` - The destination address
    /// * `amount_sats` - The amount to send in satoshis
    /// * `target_blocks` - Confirmation target in blocks (1, 3, 6, etc.)
    ///
    /// # Returns
    ///
    /// Estimated fee in satoshis.
    fn estimate_fee(
        &self,
        address: String,
        amount_sats: u64,
        target_blocks: u32,
    ) -> Result<u64, PaykitMobileError>;

    /// Get transaction details by txid.
    ///
    /// # Arguments
    ///
    /// * `txid` - The transaction ID (hex-encoded)
    ///
    /// # Returns
    ///
    /// Transaction details if found, None otherwise.
    fn get_transaction(
        &self,
        txid: String,
    ) -> Result<Option<BitcoinTxResultFFI>, PaykitMobileError>;

    /// Verify a transaction was sent to the expected address and amount.
    ///
    /// # Arguments
    ///
    /// * `txid` - The transaction ID
    /// * `address` - Expected destination address
    /// * `amount_sats` - Expected amount in satoshis
    ///
    /// # Returns
    ///
    /// True if the transaction matches, false otherwise.
    fn verify_transaction(
        &self,
        txid: String,
        address: String,
        amount_sats: u64,
    ) -> Result<bool, PaykitMobileError>;
}

/// Lightning executor callback interface for mobile wallets.
///
/// Implement this interface in Swift/Kotlin to provide Lightning Network
/// payment capabilities to Paykit.
///
/// # Thread Safety
///
/// All methods may be called from any thread. Implementations must be
/// thread-safe.
///
/// # Error Handling
///
/// Return `PaykitMobileError` for failures. The error will be propagated
/// to the caller.
#[uniffi::export(callback_interface)]
pub trait LightningExecutorFFI: Send + Sync {
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
    /// Payment result with preimage proof.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Invoice is invalid or expired
    /// - No route found
    /// - Insufficient funds
    /// - Payment failed
    fn pay_invoice(
        &self,
        invoice: String,
        amount_msat: Option<u64>,
        max_fee_msat: Option<u64>,
    ) -> Result<LightningPaymentResultFFI, PaykitMobileError>;

    /// Decode a BOLT11 invoice without paying.
    ///
    /// # Arguments
    ///
    /// * `invoice` - The BOLT11 invoice string
    ///
    /// # Returns
    ///
    /// Decoded invoice details.
    fn decode_invoice(&self, invoice: String) -> Result<DecodedInvoiceFFI, PaykitMobileError>;

    /// Estimate the fee for paying an invoice.
    ///
    /// # Arguments
    ///
    /// * `invoice` - The BOLT11 invoice
    ///
    /// # Returns
    ///
    /// Estimated fee in millisatoshis.
    fn estimate_fee(&self, invoice: String) -> Result<u64, PaykitMobileError>;

    /// Check the status of a payment by payment hash.
    ///
    /// # Arguments
    ///
    /// * `payment_hash` - The payment hash (hex-encoded)
    ///
    /// # Returns
    ///
    /// Payment result if found, None otherwise.
    fn get_payment(
        &self,
        payment_hash: String,
    ) -> Result<Option<LightningPaymentResultFFI>, PaykitMobileError>;

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
    fn verify_preimage(&self, preimage: String, payment_hash: String) -> bool;
}

// ============================================================================
// Executor Bridges
// ============================================================================

/// Bridge from FFI callback to Rust BitcoinExecutor trait.
///
/// This struct wraps a `BitcoinExecutorFFI` implementation and provides
/// the `paykit_lib::methods::BitcoinExecutor` trait.
pub struct BitcoinExecutorBridge {
    ffi: Arc<dyn BitcoinExecutorFFI>,
}

impl BitcoinExecutorBridge {
    /// Create a new bridge wrapping an FFI executor.
    pub fn new(ffi: Arc<dyn BitcoinExecutorFFI>) -> Self {
        Self { ffi }
    }
}

impl std::fmt::Debug for BitcoinExecutorBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BitcoinExecutorBridge")
            .field("ffi", &"<BitcoinExecutorFFI>")
            .finish()
    }
}

#[async_trait::async_trait]
impl paykit_lib::methods::BitcoinExecutor for BitcoinExecutorBridge {
    async fn send_to_address(
        &self,
        address: &str,
        amount_sats: u64,
        fee_rate: Option<f64>,
    ) -> paykit_lib::Result<paykit_lib::methods::BitcoinTxResult> {
        let result = self
            .ffi
            .send_to_address(address.to_string(), amount_sats, fee_rate)
            .map_err(|e| paykit_lib::PaykitError::Transport(e.to_string()))?;

        Ok(paykit_lib::methods::BitcoinTxResult {
            txid: result.txid,
            raw_tx: result.raw_tx,
            vout: result.vout,
            fee_sats: result.fee_sats,
            fee_rate: result.fee_rate,
            block_height: result.block_height,
            confirmations: result.confirmations,
        })
    }

    async fn estimate_fee(
        &self,
        address: &str,
        amount_sats: u64,
        target_blocks: u32,
    ) -> paykit_lib::Result<u64> {
        self.ffi
            .estimate_fee(address.to_string(), amount_sats, target_blocks)
            .map_err(|e| paykit_lib::PaykitError::Transport(e.to_string()))
    }

    async fn get_transaction(
        &self,
        txid: &str,
    ) -> paykit_lib::Result<Option<paykit_lib::methods::BitcoinTxResult>> {
        let result = self
            .ffi
            .get_transaction(txid.to_string())
            .map_err(|e| paykit_lib::PaykitError::Transport(e.to_string()))?;

        Ok(result.map(|r| paykit_lib::methods::BitcoinTxResult {
            txid: r.txid,
            raw_tx: r.raw_tx,
            vout: r.vout,
            fee_sats: r.fee_sats,
            fee_rate: r.fee_rate,
            block_height: r.block_height,
            confirmations: r.confirmations,
        }))
    }

    async fn verify_transaction(
        &self,
        txid: &str,
        address: &str,
        amount_sats: u64,
    ) -> paykit_lib::Result<bool> {
        self.ffi
            .verify_transaction(txid.to_string(), address.to_string(), amount_sats)
            .map_err(|e| paykit_lib::PaykitError::Transport(e.to_string()))
    }
}

/// Bridge from FFI callback to Rust LightningExecutor trait.
///
/// This struct wraps a `LightningExecutorFFI` implementation and provides
/// the `paykit_lib::methods::LightningExecutor` trait.
pub struct LightningExecutorBridge {
    ffi: Arc<dyn LightningExecutorFFI>,
}

impl LightningExecutorBridge {
    /// Create a new bridge wrapping an FFI executor.
    pub fn new(ffi: Arc<dyn LightningExecutorFFI>) -> Self {
        Self { ffi }
    }
}

impl std::fmt::Debug for LightningExecutorBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LightningExecutorBridge")
            .field("ffi", &"<LightningExecutorFFI>")
            .finish()
    }
}

#[async_trait::async_trait]
impl paykit_lib::methods::LightningExecutor for LightningExecutorBridge {
    async fn pay_invoice(
        &self,
        invoice: &str,
        amount_msat: Option<u64>,
        max_fee_msat: Option<u64>,
    ) -> paykit_lib::Result<paykit_lib::methods::LightningPaymentResult> {
        let result = self
            .ffi
            .pay_invoice(invoice.to_string(), amount_msat, max_fee_msat)
            .map_err(|e| paykit_lib::PaykitError::Transport(e.to_string()))?;

        Ok(paykit_lib::methods::LightningPaymentResult {
            preimage: result.preimage,
            payment_hash: result.payment_hash,
            amount_msat: result.amount_msat,
            fee_msat: result.fee_msat,
            hops: result.hops,
            status: match result.status {
                LightningPaymentStatusFFI::Succeeded => {
                    paykit_lib::methods::LightningPaymentStatus::Succeeded
                }
                LightningPaymentStatusFFI::Pending => {
                    paykit_lib::methods::LightningPaymentStatus::Pending
                }
                LightningPaymentStatusFFI::Failed => {
                    paykit_lib::methods::LightningPaymentStatus::Failed
                }
            },
        })
    }

    async fn decode_invoice(
        &self,
        invoice: &str,
    ) -> paykit_lib::Result<paykit_lib::methods::DecodedInvoice> {
        let result = self
            .ffi
            .decode_invoice(invoice.to_string())
            .map_err(|e| paykit_lib::PaykitError::Transport(e.to_string()))?;

        Ok(paykit_lib::methods::DecodedInvoice {
            payment_hash: result.payment_hash,
            amount_msat: result.amount_msat,
            description: result.description,
            description_hash: result.description_hash,
            payee: result.payee,
            expiry: result.expiry,
            timestamp: result.timestamp,
            expired: result.expired,
        })
    }

    async fn estimate_fee(&self, invoice: &str) -> paykit_lib::Result<u64> {
        self.ffi
            .estimate_fee(invoice.to_string())
            .map_err(|e| paykit_lib::PaykitError::Transport(e.to_string()))
    }

    async fn get_payment(
        &self,
        payment_hash: &str,
    ) -> paykit_lib::Result<Option<paykit_lib::methods::LightningPaymentResult>> {
        let result = self
            .ffi
            .get_payment(payment_hash.to_string())
            .map_err(|e| paykit_lib::PaykitError::Transport(e.to_string()))?;

        Ok(result.map(|r| paykit_lib::methods::LightningPaymentResult {
            preimage: r.preimage,
            payment_hash: r.payment_hash,
            amount_msat: r.amount_msat,
            fee_msat: r.fee_msat,
            hops: r.hops,
            status: match r.status {
                LightningPaymentStatusFFI::Succeeded => {
                    paykit_lib::methods::LightningPaymentStatus::Succeeded
                }
                LightningPaymentStatusFFI::Pending => {
                    paykit_lib::methods::LightningPaymentStatus::Pending
                }
                LightningPaymentStatusFFI::Failed => {
                    paykit_lib::methods::LightningPaymentStatus::Failed
                }
            },
        }))
    }

    fn verify_preimage(&self, preimage: &str, payment_hash: &str) -> bool {
        self.ffi
            .verify_preimage(preimage.to_string(), payment_hash.to_string())
    }
}

// ============================================================================
// Network Configuration Types
// ============================================================================

/// Bitcoin network types (FFI-compatible).
///
/// Used to configure which Bitcoin network the wallet operates on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, uniffi::Enum)]
pub enum BitcoinNetworkFFI {
    /// Bitcoin mainnet (real money).
    #[default]
    Mainnet,
    /// Bitcoin testnet (test coins).
    Testnet,
    /// Bitcoin regtest (local development).
    Regtest,
}

impl From<BitcoinNetworkFFI> for paykit_lib::methods::BitcoinNetwork {
    fn from(network: BitcoinNetworkFFI) -> Self {
        match network {
            BitcoinNetworkFFI::Mainnet => paykit_lib::methods::BitcoinNetwork::Mainnet,
            BitcoinNetworkFFI::Testnet => paykit_lib::methods::BitcoinNetwork::Testnet,
            BitcoinNetworkFFI::Regtest => paykit_lib::methods::BitcoinNetwork::Regtest,
        }
    }
}

impl From<paykit_lib::methods::BitcoinNetwork> for BitcoinNetworkFFI {
    fn from(network: paykit_lib::methods::BitcoinNetwork) -> Self {
        match network {
            paykit_lib::methods::BitcoinNetwork::Mainnet => BitcoinNetworkFFI::Mainnet,
            paykit_lib::methods::BitcoinNetwork::Testnet => BitcoinNetworkFFI::Testnet,
            paykit_lib::methods::BitcoinNetwork::Regtest => BitcoinNetworkFFI::Regtest,
        }
    }
}

/// Lightning network types (FFI-compatible).
///
/// Used to configure which Lightning network the wallet operates on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, uniffi::Enum)]
pub enum LightningNetworkFFI {
    /// Lightning mainnet (real money).
    #[default]
    Mainnet,
    /// Lightning testnet (test coins).
    Testnet,
    /// Lightning regtest (local development).
    Regtest,
}

impl From<LightningNetworkFFI> for paykit_lib::methods::LightningNetwork {
    fn from(network: LightningNetworkFFI) -> Self {
        match network {
            LightningNetworkFFI::Mainnet => paykit_lib::methods::LightningNetwork::Mainnet,
            LightningNetworkFFI::Testnet => paykit_lib::methods::LightningNetwork::Testnet,
            LightningNetworkFFI::Regtest => paykit_lib::methods::LightningNetwork::Regtest,
        }
    }
}

impl From<paykit_lib::methods::LightningNetwork> for LightningNetworkFFI {
    fn from(network: paykit_lib::methods::LightningNetwork) -> Self {
        match network {
            paykit_lib::methods::LightningNetwork::Mainnet => LightningNetworkFFI::Mainnet,
            paykit_lib::methods::LightningNetwork::Testnet => LightningNetworkFFI::Testnet,
            paykit_lib::methods::LightningNetwork::Regtest => LightningNetworkFFI::Regtest,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use paykit_lib::methods::{BitcoinExecutor, LightningExecutor};
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Mock Bitcoin executor for testing.
    struct MockBitcoinExecutorFFI {
        send_count: AtomicU32,
        should_fail: bool,
    }

    impl MockBitcoinExecutorFFI {
        fn new() -> Self {
            Self {
                send_count: AtomicU32::new(0),
                should_fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                send_count: AtomicU32::new(0),
                should_fail: true,
            }
        }
    }

    impl BitcoinExecutorFFI for MockBitcoinExecutorFFI {
        fn send_to_address(
            &self,
            _address: String,
            _amount_sats: u64,
            fee_rate: Option<f64>,
        ) -> Result<BitcoinTxResultFFI, PaykitMobileError> {
            if self.should_fail {
                return Err(PaykitMobileError::Transport {
                    message: "Mock failure".to_string(),
                });
            }
            self.send_count.fetch_add(1, Ordering::SeqCst);
            Ok(BitcoinTxResultFFI {
                txid: format!("mock_txid_{}", self.send_count.load(Ordering::SeqCst)),
                raw_tx: None,
                vout: 0,
                fee_sats: (fee_rate.unwrap_or(1.0) * 140.0) as u64,
                fee_rate: fee_rate.unwrap_or(1.0),
                block_height: None,
                confirmations: 0,
            })
        }

        fn estimate_fee(
            &self,
            _address: String,
            _amount_sats: u64,
            target_blocks: u32,
        ) -> Result<u64, PaykitMobileError> {
            if self.should_fail {
                return Err(PaykitMobileError::Transport {
                    message: "Mock failure".to_string(),
                });
            }
            // Simple fee estimation: lower target = higher fee
            Ok(210 * (10 - target_blocks.min(9)) as u64)
        }

        fn get_transaction(
            &self,
            txid: String,
        ) -> Result<Option<BitcoinTxResultFFI>, PaykitMobileError> {
            if self.should_fail {
                return Err(PaykitMobileError::Transport {
                    message: "Mock failure".to_string(),
                });
            }
            if txid.starts_with("mock_txid") {
                Ok(Some(BitcoinTxResultFFI {
                    txid,
                    raw_tx: None,
                    vout: 0,
                    fee_sats: 210,
                    fee_rate: 1.5,
                    block_height: Some(800000),
                    confirmations: 6,
                }))
            } else {
                Ok(None)
            }
        }

        fn verify_transaction(
            &self,
            _txid: String,
            _address: String,
            _amount_sats: u64,
        ) -> Result<bool, PaykitMobileError> {
            if self.should_fail {
                return Err(PaykitMobileError::Transport {
                    message: "Mock failure".to_string(),
                });
            }
            Ok(true)
        }
    }

    /// Mock Lightning executor for testing.
    struct MockLightningExecutorFFI {
        pay_count: AtomicU32,
        should_fail: bool,
    }

    impl MockLightningExecutorFFI {
        fn new() -> Self {
            Self {
                pay_count: AtomicU32::new(0),
                should_fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                pay_count: AtomicU32::new(0),
                should_fail: true,
            }
        }
    }

    impl LightningExecutorFFI for MockLightningExecutorFFI {
        fn pay_invoice(
            &self,
            _invoice: String,
            amount_msat: Option<u64>,
            _max_fee_msat: Option<u64>,
        ) -> Result<LightningPaymentResultFFI, PaykitMobileError> {
            if self.should_fail {
                return Err(PaykitMobileError::Transport {
                    message: "Mock failure".to_string(),
                });
            }
            self.pay_count.fetch_add(1, Ordering::SeqCst);
            Ok(LightningPaymentResultFFI {
                preimage: format!(
                    "mock_preimage_{:064x}",
                    self.pay_count.load(Ordering::SeqCst)
                ),
                payment_hash: format!("mock_hash_{:064x}", self.pay_count.load(Ordering::SeqCst)),
                amount_msat: amount_msat.unwrap_or(1000000),
                fee_msat: 100,
                hops: 3,
                status: LightningPaymentStatusFFI::Succeeded,
            })
        }

        fn decode_invoice(&self, _invoice: String) -> Result<DecodedInvoiceFFI, PaykitMobileError> {
            if self.should_fail {
                return Err(PaykitMobileError::Transport {
                    message: "Mock failure".to_string(),
                });
            }
            Ok(DecodedInvoiceFFI {
                payment_hash: "mock_hash".to_string(),
                amount_msat: Some(1000000),
                description: Some("Test invoice".to_string()),
                description_hash: None,
                payee: "mock_payee".to_string(),
                expiry: 3600,
                timestamp: 1700000000,
                expired: false,
            })
        }

        fn estimate_fee(&self, _invoice: String) -> Result<u64, PaykitMobileError> {
            if self.should_fail {
                return Err(PaykitMobileError::Transport {
                    message: "Mock failure".to_string(),
                });
            }
            Ok(100)
        }

        fn get_payment(
            &self,
            payment_hash: String,
        ) -> Result<Option<LightningPaymentResultFFI>, PaykitMobileError> {
            if self.should_fail {
                return Err(PaykitMobileError::Transport {
                    message: "Mock failure".to_string(),
                });
            }
            if payment_hash.starts_with("mock_hash") {
                Ok(Some(LightningPaymentResultFFI {
                    preimage: "mock_preimage".to_string(),
                    payment_hash,
                    amount_msat: 1000000,
                    fee_msat: 100,
                    hops: 3,
                    status: LightningPaymentStatusFFI::Succeeded,
                }))
            } else {
                Ok(None)
            }
        }

        fn verify_preimage(&self, preimage: String, payment_hash: String) -> bool {
            // Simple mock verification
            preimage.contains("mock") && payment_hash.contains("mock")
        }
    }

    // ========================================================================
    // Bitcoin Executor Tests
    // ========================================================================

    #[test]
    fn test_bitcoin_tx_result_new() {
        let result = BitcoinTxResultFFI::new("abc123".to_string(), 0, 210, 1.5);
        assert_eq!(result.txid, "abc123");
        assert_eq!(result.vout, 0);
        assert_eq!(result.fee_sats, 210);
        assert!(!result.is_confirmed());
    }

    #[test]
    fn test_bitcoin_tx_result_confirmed() {
        let mut result = BitcoinTxResultFFI::new("abc123".to_string(), 0, 210, 1.5);
        result.confirmations = 6;
        result.block_height = Some(800000);
        assert!(result.is_confirmed());
    }

    #[test]
    fn test_mock_bitcoin_executor_send() {
        let executor = MockBitcoinExecutorFFI::new();
        let result = executor
            .send_to_address("bc1qtest".to_string(), 10000, Some(2.0))
            .unwrap();
        assert!(result.txid.starts_with("mock_txid"));
        assert_eq!(result.fee_sats, 280); // 2.0 * 140
    }

    #[test]
    fn test_mock_bitcoin_executor_failure() {
        let executor = MockBitcoinExecutorFFI::failing();
        let result = executor.send_to_address("bc1qtest".to_string(), 10000, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_bitcoin_executor_estimate_fee() {
        let executor = MockBitcoinExecutorFFI::new();
        let fee = executor
            .estimate_fee("bc1qtest".to_string(), 10000, 1)
            .unwrap();
        assert_eq!(fee, 210 * 9); // (10 - 1) * 210
    }

    #[test]
    fn test_mock_bitcoin_executor_get_transaction() {
        let executor = MockBitcoinExecutorFFI::new();

        // Known transaction
        let result = executor.get_transaction("mock_txid_1".to_string()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().confirmations, 6);

        // Unknown transaction
        let result = executor.get_transaction("unknown".to_string()).unwrap();
        assert!(result.is_none());
    }

    // ========================================================================
    // Lightning Executor Tests
    // ========================================================================

    #[test]
    fn test_lightning_payment_result_success() {
        let result = LightningPaymentResultFFI::success(
            "preimage".to_string(),
            "hash".to_string(),
            1000000,
            100,
        );
        assert_eq!(result.preimage, "preimage");
        assert_eq!(result.status, LightningPaymentStatusFFI::Succeeded);
    }

    #[test]
    fn test_mock_lightning_executor_pay() {
        let executor = MockLightningExecutorFFI::new();
        let result = executor
            .pay_invoice("lnbc1000n1...".to_string(), Some(1000000), Some(1000))
            .unwrap();
        assert!(result.preimage.starts_with("mock_preimage"));
        assert_eq!(result.status, LightningPaymentStatusFFI::Succeeded);
    }

    #[test]
    fn test_mock_lightning_executor_failure() {
        let executor = MockLightningExecutorFFI::failing();
        let result = executor.pay_invoice("lnbc1000n1...".to_string(), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_lightning_executor_decode() {
        let executor = MockLightningExecutorFFI::new();
        let decoded = executor
            .decode_invoice("lnbc1000n1...".to_string())
            .unwrap();
        assert_eq!(decoded.amount_msat, Some(1000000));
        assert!(!decoded.expired);
    }

    #[test]
    fn test_mock_lightning_executor_verify_preimage() {
        let executor = MockLightningExecutorFFI::new();
        assert!(executor.verify_preimage("mock_preimage".to_string(), "mock_hash".to_string()));
        assert!(!executor.verify_preimage("invalid".to_string(), "invalid".to_string()));
    }

    // ========================================================================
    // Bridge Tests
    // ========================================================================

    #[tokio::test]
    async fn test_bitcoin_executor_bridge() {
        let mock = Arc::new(MockBitcoinExecutorFFI::new());
        let bridge = BitcoinExecutorBridge::new(mock);

        // Test send
        let result = bridge
            .send_to_address("bc1qtest", 10000, Some(1.5))
            .await
            .unwrap();
        assert!(result.txid.starts_with("mock_txid"));

        // Test estimate
        let fee = bridge.estimate_fee("bc1qtest", 10000, 6).await.unwrap();
        assert!(fee > 0);

        // Test get transaction
        let tx = bridge.get_transaction("mock_txid_1").await.unwrap();
        assert!(tx.is_some());

        // Test verify
        let verified = bridge
            .verify_transaction("mock_txid_1", "bc1qtest", 10000)
            .await
            .unwrap();
        assert!(verified);
    }

    #[tokio::test]
    async fn test_bitcoin_executor_bridge_error() {
        let mock = Arc::new(MockBitcoinExecutorFFI::failing());
        let bridge = BitcoinExecutorBridge::new(mock);

        let result = bridge.send_to_address("bc1qtest", 10000, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_lightning_executor_bridge() {
        let mock = Arc::new(MockLightningExecutorFFI::new());
        let bridge = LightningExecutorBridge::new(mock);

        // Test pay
        let result = bridge
            .pay_invoice("lnbc1000n1...", Some(1000000), Some(1000))
            .await
            .unwrap();
        assert!(result.preimage.starts_with("mock_preimage"));

        // Test decode
        let decoded = bridge.decode_invoice("lnbc1000n1...").await.unwrap();
        assert_eq!(decoded.amount_msat, Some(1000000));

        // Test estimate
        let fee = bridge.estimate_fee("lnbc1000n1...").await.unwrap();
        assert_eq!(fee, 100);

        // Test get payment
        let payment = bridge.get_payment("mock_hash_1").await.unwrap();
        assert!(payment.is_some());

        // Test verify
        let verified = bridge.verify_preimage("mock_preimage", "mock_hash");
        assert!(verified);
    }

    #[tokio::test]
    async fn test_lightning_executor_bridge_error() {
        let mock = Arc::new(MockLightningExecutorFFI::failing());
        let bridge = LightningExecutorBridge::new(mock);

        let result = bridge.pay_invoice("lnbc1000n1...", None, None).await;
        assert!(result.is_err());
    }

    // ========================================================================
    // Network Configuration Tests
    // ========================================================================

    #[test]
    fn test_bitcoin_network_conversion() {
        assert_eq!(
            paykit_lib::methods::BitcoinNetwork::from(BitcoinNetworkFFI::Mainnet),
            paykit_lib::methods::BitcoinNetwork::Mainnet
        );
        assert_eq!(
            paykit_lib::methods::BitcoinNetwork::from(BitcoinNetworkFFI::Testnet),
            paykit_lib::methods::BitcoinNetwork::Testnet
        );
        assert_eq!(
            paykit_lib::methods::BitcoinNetwork::from(BitcoinNetworkFFI::Regtest),
            paykit_lib::methods::BitcoinNetwork::Regtest
        );
    }

    #[test]
    fn test_bitcoin_network_reverse_conversion() {
        assert_eq!(
            BitcoinNetworkFFI::from(paykit_lib::methods::BitcoinNetwork::Mainnet),
            BitcoinNetworkFFI::Mainnet
        );
        assert_eq!(
            BitcoinNetworkFFI::from(paykit_lib::methods::BitcoinNetwork::Testnet),
            BitcoinNetworkFFI::Testnet
        );
        assert_eq!(
            BitcoinNetworkFFI::from(paykit_lib::methods::BitcoinNetwork::Regtest),
            BitcoinNetworkFFI::Regtest
        );
    }

    #[test]
    fn test_lightning_network_conversion() {
        assert_eq!(
            paykit_lib::methods::LightningNetwork::from(LightningNetworkFFI::Mainnet),
            paykit_lib::methods::LightningNetwork::Mainnet
        );
        assert_eq!(
            paykit_lib::methods::LightningNetwork::from(LightningNetworkFFI::Testnet),
            paykit_lib::methods::LightningNetwork::Testnet
        );
        assert_eq!(
            paykit_lib::methods::LightningNetwork::from(LightningNetworkFFI::Regtest),
            paykit_lib::methods::LightningNetwork::Regtest
        );
    }

    #[test]
    fn test_lightning_network_reverse_conversion() {
        assert_eq!(
            LightningNetworkFFI::from(paykit_lib::methods::LightningNetwork::Mainnet),
            LightningNetworkFFI::Mainnet
        );
        assert_eq!(
            LightningNetworkFFI::from(paykit_lib::methods::LightningNetwork::Testnet),
            LightningNetworkFFI::Testnet
        );
        assert_eq!(
            LightningNetworkFFI::from(paykit_lib::methods::LightningNetwork::Regtest),
            LightningNetworkFFI::Regtest
        );
    }

    #[test]
    fn test_network_defaults() {
        assert_eq!(BitcoinNetworkFFI::default(), BitcoinNetworkFFI::Mainnet);
        assert_eq!(LightningNetworkFFI::default(), LightningNetworkFFI::Mainnet);
    }
}
