//! Paykit Mobile FFI Bindings
//!
//! This crate provides UniFFI bindings for Paykit, enabling integration
//! with iOS (Swift) and Android (Kotlin) applications.
//!
//! # Architecture
//!
//! The FFI layer wraps the core Paykit functionality:
//! - Directory Protocol (endpoint management)
//! - Payment Method Selection
//! - Interactive Protocol (receipts, proofs)
//! - Subscription Management
//!
//! # Thread Safety
//!
//! All exposed types are thread-safe and can be used from any thread.
//! Async operations use the Tokio runtime.

pub mod async_bridge;
pub mod executor_ffi;
pub mod interactive_ffi;
pub mod keys;
pub mod noise_ffi;
pub mod scanner;
pub mod spending_ffi;
pub mod storage;
pub mod transport_ffi;

// Re-export transport types for easier access
pub use transport_ffi::{
    AuthenticatedTransportFFI, PubkyAuthenticatedStorageCallback,
    PubkyUnauthenticatedStorageCallback, StorageGetResult, StorageListResult,
    StorageOperationResult, UnauthenticatedTransportFFI,
};

// Re-export interactive types for easier access
pub use interactive_ffi::{
    ErrorMessage, ParsedMessage, PaykitInteractiveManagerFFI, PaykitMessageBuilder,
    PaykitMessageType, PrivateEndpointOffer, ReceiptGenerationResult, ReceiptGeneratorCallback,
    ReceiptRequest, ReceiptStore,
};

// Re-export key management types for easier access
pub use keys::{Ed25519Keypair, KeyBackup, X25519Keypair};

// Re-export noise FFI types for easier access
pub use noise_ffi::{
    NoiseConnectionStatus, NoiseEndpointInfo, NoiseHandshakeResult, NoisePaymentMessage,
    NoisePaymentMessageType, NoiseServerConfig, NoiseServerStatus, NoiseSessionInfo,
};

// Re-export executor FFI types for wallet integration (Bitkit, etc.)
pub use executor_ffi::{
    BitcoinExecutorBridge, BitcoinExecutorFFI, BitcoinNetworkFFI, BitcoinTxResultFFI,
    DecodedInvoiceFFI, LightningExecutorBridge, LightningExecutorFFI, LightningNetworkFFI,
    LightningPaymentResultFFI, LightningPaymentStatusFFI,
};

// Re-export spending FFI types for atomic spending limit operations
pub use spending_ffi::{
    PeerSpendingLimitFFI, SpendingCheckResultFFI, SpendingManagerFFI, SpendingReservationFFI,
};

use std::sync::{Arc, RwLock};

// UniFFI scaffolding
uniffi::setup_scaffolding!();

// ============================================================================
// Error Types
// ============================================================================

/// Mobile-friendly error type.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PaykitMobileError {
    /// Transport layer error (network, I/O).
    #[error("Transport error: {msg}")]
    Transport { msg: String },

    /// Validation error (invalid input, format).
    #[error("Validation error: {msg}")]
    Validation { msg: String },

    /// Resource not found.
    #[error("Not found: {msg}")]
    NotFound { msg: String },

    /// Serialization/deserialization error.
    #[error("Serialization error: {msg}")]
    Serialization { msg: String },

    /// Internal error (unexpected state).
    #[error("Internal error: {msg}")]
    Internal { msg: String },

    /// Network timeout error.
    #[error("Network timeout: {msg}")]
    NetworkTimeout { msg: String },

    /// Connection refused or failed.
    #[error("Connection error: {msg}")]
    ConnectionError { msg: String },

    /// Authentication failed.
    #[error("Authentication error: {msg}")]
    AuthenticationError { msg: String },

    /// Session expired or invalid.
    #[error("Session error: {msg}")]
    SessionError { msg: String },

    /// Rate limit exceeded.
    #[error("Rate limit exceeded: {msg}")]
    RateLimitError { msg: String },

    /// Permission denied.
    #[error("Permission denied: {msg}")]
    PermissionDenied { msg: String },
}

impl From<paykit_lib::PaykitError> for PaykitMobileError {
    fn from(e: paykit_lib::PaykitError) -> Self {
        match e {
            paykit_lib::PaykitError::Transport(msg) => Self::Transport { msg },
            paykit_lib::PaykitError::Unimplemented(msg) => Self::Internal {
                msg: msg.to_string(),
            },
            paykit_lib::PaykitError::ConnectionFailed { target, reason } => Self::ConnectionError {
                msg: format!("Connection to {} failed: {}", target, reason),
            },
            paykit_lib::PaykitError::ConnectionTimeout {
                operation,
                timeout_ms,
            } => Self::NetworkTimeout {
                msg: format!("{} timed out after {}ms", operation, timeout_ms),
            },
            paykit_lib::PaykitError::Auth(msg) => Self::AuthenticationError { msg },
            paykit_lib::PaykitError::SessionExpired => Self::SessionError {
                msg: "Session expired".to_string(),
            },
            paykit_lib::PaykitError::InvalidCredentials(msg) => {
                Self::AuthenticationError { msg }
            }
            paykit_lib::PaykitError::NotFound {
                resource_type,
                identifier,
            } => Self::NotFound {
                msg: format!("{} not found: {}", resource_type, identifier),
            },
            paykit_lib::PaykitError::MethodNotSupported(method) => Self::Validation {
                msg: format!("Payment method not supported: {}", method),
            },
            paykit_lib::PaykitError::InvalidData { field, reason } => Self::Validation {
                msg: format!("Invalid {}: {}", field, reason),
            },
            paykit_lib::PaykitError::ValidationFailed(msg) => Self::Validation { msg },
            paykit_lib::PaykitError::Serialization(msg) => Self::Serialization { msg },
            paykit_lib::PaykitError::Payment { payment_id, reason } => Self::Transport {
                msg: format!(
                    "Payment {} failed: {}",
                    payment_id.unwrap_or_default(),
                    reason
                ),
            },
            paykit_lib::PaykitError::InsufficientFunds {
                required,
                available,
                currency,
            } => Self::Transport {
                msg: format!(
                    "Insufficient funds: need {} {}, have {} {}",
                    required, currency, available, currency
                ),
            },
            paykit_lib::PaykitError::InvoiceExpired {
                invoice_id,
                expired_at,
            } => Self::Transport {
                msg: format!("Invoice {} expired at {}", invoice_id, expired_at),
            },
            paykit_lib::PaykitError::PaymentRejected { payment_id, reason } => Self::Transport {
                msg: format!("Payment {} rejected: {}", payment_id, reason),
            },
            paykit_lib::PaykitError::PaymentAlreadyCompleted { payment_id } => Self::Transport {
                msg: format!("Payment {} already completed", payment_id),
            },
            paykit_lib::PaykitError::Storage(msg) => Self::Internal { msg },
            paykit_lib::PaykitError::QuotaExceeded { used, limit } => Self::Internal {
                msg: format!("Quota exceeded: {} of {} used", used, limit),
            },
            paykit_lib::PaykitError::RateLimited { retry_after_ms } => Self::RateLimitError {
                msg: format!("Rate limited, retry after {}ms", retry_after_ms),
            },
            paykit_lib::PaykitError::Internal(msg) => Self::Internal { msg },
        }
    }
}

pub type Result<T> = std::result::Result<T, PaykitMobileError>;

// ============================================================================
// Core Types (FFI-safe wrappers)
// ============================================================================

/// A payment method identifier.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct MethodId {
    pub value: String,
}

impl From<paykit_lib::MethodId> for MethodId {
    fn from(id: paykit_lib::MethodId) -> Self {
        Self { value: id.0 }
    }
}

impl From<MethodId> for paykit_lib::MethodId {
    fn from(id: MethodId) -> Self {
        paykit_lib::MethodId(id.value)
    }
}

/// Endpoint data for a payment method.
#[derive(Clone, Debug, uniffi::Record)]
pub struct EndpointData {
    pub value: String,
}

impl From<paykit_lib::EndpointData> for EndpointData {
    fn from(data: paykit_lib::EndpointData) -> Self {
        Self { value: data.0 }
    }
}

impl From<EndpointData> for paykit_lib::EndpointData {
    fn from(data: EndpointData) -> Self {
        paykit_lib::EndpointData(data.value)
    }
}

/// A supported payment method with its endpoint.
#[derive(Clone, Debug, uniffi::Record)]
pub struct PaymentMethod {
    pub method_id: String,
    pub endpoint: String,
}

/// Payment amount.
#[derive(Clone, Debug, uniffi::Record)]
pub struct Amount {
    pub value: String,
    pub currency: String,
}

impl Amount {
    pub fn sats(value: u64) -> Self {
        Self {
            value: value.to_string(),
            currency: "SAT".to_string(),
        }
    }
}

// ============================================================================
// Selection Types
// ============================================================================

/// Selection strategy.
#[derive(Clone, Copy, Debug, uniffi::Enum)]
pub enum SelectionStrategy {
    Balanced,
    CostOptimized,
    SpeedOptimized,
    PrivacyOptimized,
}

/// Selection preferences.
#[derive(Clone, Debug, uniffi::Record)]
pub struct SelectionPreferences {
    pub strategy: SelectionStrategy,
    pub excluded_methods: Vec<String>,
    pub max_fee_sats: Option<u64>,
    pub max_confirmation_time_secs: Option<u64>,
}

impl Default for SelectionPreferences {
    fn default() -> Self {
        Self {
            strategy: SelectionStrategy::Balanced,
            excluded_methods: Vec::new(),
            max_fee_sats: None,
            max_confirmation_time_secs: None,
        }
    }
}

/// Result of payment method selection.
#[derive(Clone, Debug, uniffi::Record)]
pub struct SelectionResult {
    pub primary_method: String,
    pub fallback_methods: Vec<String>,
    pub reason: String,
}

// ============================================================================
// Status Types
// ============================================================================

/// Payment status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum PaymentStatus {
    Pending,
    Processing,
    Confirmed,
    Finalized,
    Failed,
    Cancelled,
    Expired,
}

/// Payment status information.
#[derive(Clone, Debug, uniffi::Record)]
pub struct PaymentStatusInfo {
    pub status: PaymentStatus,
    pub receipt_id: String,
    pub method_id: String,
    pub updated_at: i64,
    pub confirmations: Option<u64>,
    pub required_confirmations: Option<u64>,
    pub error: Option<String>,
}

// ============================================================================
// Payment Execution Types
// ============================================================================

/// Result of a payment execution.
///
/// Returned by `PaykitClient::execute_payment()` after attempting to
/// send a payment via the registered wallet executor.
#[derive(Clone, Debug, uniffi::Record)]
pub struct PaymentExecutionResult {
    /// Unique execution ID.
    pub execution_id: String,
    /// Payment method used.
    pub method_id: String,
    /// Payment destination.
    pub endpoint: String,
    /// Amount sent in satoshis.
    pub amount_sats: u64,
    /// Whether the payment succeeded.
    pub success: bool,
    /// Unix timestamp of execution.
    pub executed_at: i64,
    /// Execution details as JSON (contains txid, preimage, fees, etc.).
    pub execution_data_json: String,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Result of generating a payment proof.
///
/// Returned by `PaykitClient::generate_payment_proof()` after
/// extracting proof data from a successful payment execution.
#[derive(Clone, Debug, uniffi::Record)]
pub struct PaymentProofResult {
    /// Type of proof ("bitcoin_txid", "lightning_preimage", "custom").
    pub proof_type: String,
    /// Proof data as JSON.
    pub proof_data_json: String,
}

// ============================================================================
// Health Types
// ============================================================================

/// Health status of a payment method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unavailable,
    Unknown,
}

/// Health check result.
#[derive(Clone, Debug, uniffi::Record)]
pub struct HealthCheckResult {
    pub method_id: String,
    pub status: HealthStatus,
    pub checked_at: i64,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

// ============================================================================
// Subscription Types
// ============================================================================

/// Payment frequency for subscriptions.
#[derive(Clone, Debug, uniffi::Enum)]
pub enum PaymentFrequency {
    Daily,
    Weekly,
    Monthly { day_of_month: u8 },
    Yearly { month: u8, day: u8 },
    Custom { interval_seconds: u64 },
}

impl From<PaymentFrequency> for paykit_subscriptions::PaymentFrequency {
    fn from(f: PaymentFrequency) -> Self {
        match f {
            PaymentFrequency::Daily => paykit_subscriptions::PaymentFrequency::Daily,
            PaymentFrequency::Weekly => paykit_subscriptions::PaymentFrequency::Weekly,
            PaymentFrequency::Monthly { day_of_month } => {
                paykit_subscriptions::PaymentFrequency::Monthly { day_of_month }
            }
            PaymentFrequency::Yearly { month, day } => {
                paykit_subscriptions::PaymentFrequency::Yearly { month, day }
            }
            PaymentFrequency::Custom { interval_seconds } => {
                paykit_subscriptions::PaymentFrequency::Custom { interval_seconds }
            }
        }
    }
}

impl From<paykit_subscriptions::PaymentFrequency> for PaymentFrequency {
    fn from(f: paykit_subscriptions::PaymentFrequency) -> Self {
        match f {
            paykit_subscriptions::PaymentFrequency::Daily => PaymentFrequency::Daily,
            paykit_subscriptions::PaymentFrequency::Weekly => PaymentFrequency::Weekly,
            paykit_subscriptions::PaymentFrequency::Monthly { day_of_month } => {
                PaymentFrequency::Monthly { day_of_month }
            }
            paykit_subscriptions::PaymentFrequency::Yearly { month, day } => {
                PaymentFrequency::Yearly { month, day }
            }
            paykit_subscriptions::PaymentFrequency::Custom { interval_seconds } => {
                PaymentFrequency::Custom { interval_seconds }
            }
        }
    }
}

/// Subscription terms.
#[derive(Clone, Debug, uniffi::Record)]
pub struct SubscriptionTerms {
    pub amount_sats: i64,
    pub currency: String,
    pub frequency: PaymentFrequency,
    pub method_id: String,
    pub description: String,
}

/// Subscription information.
#[derive(Clone, Debug, uniffi::Record)]
pub struct Subscription {
    pub subscription_id: String,
    pub subscriber: String,
    pub provider: String,
    pub terms: SubscriptionTerms,
    pub created_at: i64,
    pub starts_at: i64,
    pub ends_at: Option<i64>,
    pub is_active: bool,
}

/// Modification type for subscriptions.
#[derive(Clone, Debug, uniffi::Enum)]
pub enum ModificationType {
    Upgrade {
        new_amount_sats: i64,
        effective_date: i64,
    },
    Downgrade {
        new_amount_sats: i64,
        effective_date: i64,
    },
    ChangeMethod {
        new_method_id: String,
    },
    ChangeBillingDate {
        new_day: u8,
    },
    Cancel {
        effective_date: i64,
        reason: Option<String>,
    },
    Pause {
        resume_date: i64,
    },
    Resume,
}

/// Proration result.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ProrationResult {
    pub credit_sats: i64,
    pub charge_sats: i64,
    pub net_sats: i64,
    pub is_refund: bool,
}

// ============================================================================
// Payment Request Types
// ============================================================================

/// Payment request.
#[derive(Clone, Debug, uniffi::Record)]
pub struct PaymentRequest {
    pub request_id: String,
    pub from_pubkey: String,
    pub to_pubkey: String,
    pub amount_sats: i64,
    pub currency: String,
    pub method_id: String,
    pub description: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

/// Payment request status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum RequestStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
    Paid,
}

// ============================================================================
// Receipt Types
// ============================================================================

/// Payment receipt.
#[derive(Clone, Debug, uniffi::Record)]
pub struct Receipt {
    pub receipt_id: String,
    pub payer: String,
    pub payee: String,
    pub method_id: String,
    pub amount: Option<String>,
    pub currency: Option<String>,
    pub created_at: i64,
    pub metadata_json: String,
}

// ============================================================================
// Private Endpoint Types
// ============================================================================

/// Private endpoint information.
#[derive(Clone, Debug, uniffi::Record)]
pub struct PrivateEndpoint {
    pub peer: String,
    pub method_id: String,
    pub endpoint: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

// ============================================================================
// Main Client
// ============================================================================

/// Main Paykit client for mobile applications.
#[derive(uniffi::Object)]
pub struct PaykitClient {
    /// Plugin registry (thread-safe for concurrent access).
    registry: Arc<std::sync::RwLock<paykit_lib::methods::PaymentMethodRegistry>>,
    /// Health monitor.
    health_monitor: Arc<paykit_lib::health::HealthMonitor>,
    /// Status tracker.
    status_tracker: Arc<paykit_interactive::PaymentStatusTracker>,
    /// Tokio runtime for async operations.
    runtime: tokio::runtime::Runtime,
    /// Configured Bitcoin network.
    bitcoin_network: executor_ffi::BitcoinNetworkFFI,
    /// Configured Lightning network.
    lightning_network: executor_ffi::LightningNetworkFFI,
}

#[uniffi::export]
impl PaykitClient {
    /// Create a new Paykit client with default (mainnet) network configuration.
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>> {
        Self::new_with_network(
            executor_ffi::BitcoinNetworkFFI::Mainnet,
            executor_ffi::LightningNetworkFFI::Mainnet,
        )
    }

    /// Create a new Paykit client with specific network configuration.
    ///
    /// # Arguments
    ///
    /// * `bitcoin_network` - Bitcoin network to use (Mainnet, Testnet, or Regtest)
    /// * `lightning_network` - Lightning network to use (Mainnet, Testnet, or Regtest)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // For testnet development
    /// let client = PaykitClient::new_with_network(
    ///     BitcoinNetworkFFI::Testnet,
    ///     LightningNetworkFFI::Testnet,
    /// )?;
    /// ```
    #[uniffi::constructor]
    pub fn new_with_network(
        bitcoin_network: executor_ffi::BitcoinNetworkFFI,
        lightning_network: executor_ffi::LightningNetworkFFI,
    ) -> Result<Arc<Self>> {
        let runtime = tokio::runtime::Runtime::new().map_err(|e| PaykitMobileError::Internal {
            msg: e.to_string(),
        })?;

        Ok(Arc::new(Self {
            registry: Arc::new(RwLock::new(paykit_lib::methods::default_registry())),
            health_monitor: Arc::new(paykit_lib::health::HealthMonitor::with_defaults()),
            status_tracker: Arc::new(paykit_interactive::PaymentStatusTracker::new()),
            runtime,
            bitcoin_network,
            lightning_network,
        }))
    }

    /// Get the configured Bitcoin network.
    pub fn bitcoin_network(&self) -> executor_ffi::BitcoinNetworkFFI {
        self.bitcoin_network
    }

    /// Get the configured Lightning network.
    pub fn lightning_network(&self) -> executor_ffi::LightningNetworkFFI {
        self.lightning_network
    }

    /// Get the list of registered payment methods.
    pub fn list_methods(&self) -> Vec<String> {
        self.registry
            .read()
            .unwrap()
            .list_methods()
            .into_iter()
            .map(|m| m.0)
            .collect()
    }

    /// Validate an endpoint for a specific method.
    pub fn validate_endpoint(&self, method_id: String, endpoint: String) -> Result<bool> {
        let method = paykit_lib::MethodId(method_id);
        let data = paykit_lib::EndpointData(endpoint);

        let plugin = self
            .registry
            .read()
            .unwrap()
            .get(&method)
            .ok_or(PaykitMobileError::NotFound {
                msg: format!("Method not found: {}", method.0),
            })?;

        let result = plugin.validate_endpoint(&data);
        Ok(result.valid)
    }

    /// Select the best payment method from supported options.
    pub fn select_method(
        &self,
        supported_methods: Vec<PaymentMethod>,
        amount_sats: u64,
        preferences: Option<SelectionPreferences>,
    ) -> Result<SelectionResult> {
        use paykit_lib::selection::{PaymentMethodSelector, SelectionPreferences as LibPrefs};

        // Convert to internal types
        let mut entries = std::collections::HashMap::new();
        for method in supported_methods {
            entries.insert(
                paykit_lib::MethodId(method.method_id),
                paykit_lib::EndpointData(method.endpoint),
            );
        }
        let supported = paykit_lib::SupportedPayments { entries };

        let amount = paykit_lib::methods::Amount::sats(amount_sats);

        let prefs = preferences
            .map(|p| {
                let mut lib_prefs = match p.strategy {
                    SelectionStrategy::Balanced => LibPrefs::balanced(),
                    SelectionStrategy::CostOptimized => LibPrefs::cost_optimized(),
                    SelectionStrategy::SpeedOptimized => LibPrefs::speed_optimized(),
                    SelectionStrategy::PrivacyOptimized => LibPrefs::privacy_optimized(),
                };

                for excluded in p.excluded_methods {
                    lib_prefs = lib_prefs.exclude_method(paykit_lib::MethodId(excluded));
                }

                if let Some(max_fee) = p.max_fee_sats {
                    lib_prefs = lib_prefs.with_max_fee(max_fee);
                }

                if let Some(max_time) = p.max_confirmation_time_secs {
                    lib_prefs = lib_prefs.with_max_confirmation_time(max_time);
                }

                lib_prefs
            })
            .unwrap_or_default();

        let selector = PaymentMethodSelector::new(self.registry.read().unwrap_or_else(|e| e.into_inner()).clone());
        let result = selector.select(&supported, &amount, &prefs).map_err(|e| {
            PaykitMobileError::Validation {
                msg: e.to_string(),
            }
        })?;

        Ok(SelectionResult {
            primary_method: result.primary.0,
            fallback_methods: result.fallbacks.into_iter().map(|m| m.0).collect(),
            reason: result.reason,
        })
    }

    /// Check health of all payment methods.
    pub fn check_health(&self) -> Vec<HealthCheckResult> {
        self.runtime.block_on(async {
            let results = self.health_monitor.check_all().await;
            results
                .into_iter()
                .map(|r| HealthCheckResult {
                    method_id: r.method_id.0,
                    status: match r.status {
                        paykit_lib::health::HealthStatus::Healthy => HealthStatus::Healthy,
                        paykit_lib::health::HealthStatus::Degraded => HealthStatus::Degraded,
                        paykit_lib::health::HealthStatus::Unavailable => HealthStatus::Unavailable,
                        paykit_lib::health::HealthStatus::Unknown => HealthStatus::Unknown,
                    },
                    checked_at: r.checked_at,
                    latency_ms: r.latency_ms,
                    error: r.error,
                })
                .collect()
        })
    }

    /// Get health status of a specific method.
    pub fn get_health_status(&self, method_id: String) -> Option<HealthStatus> {
        self.health_monitor
            .get_status(&paykit_lib::MethodId(method_id))
            .map(|s| match s {
                paykit_lib::health::HealthStatus::Healthy => HealthStatus::Healthy,
                paykit_lib::health::HealthStatus::Degraded => HealthStatus::Degraded,
                paykit_lib::health::HealthStatus::Unavailable => HealthStatus::Unavailable,
                paykit_lib::health::HealthStatus::Unknown => HealthStatus::Unknown,
            })
    }

    /// Check if a method is usable (healthy or degraded).
    pub fn is_method_usable(&self, method_id: String) -> bool {
        self.health_monitor
            .is_usable(&paykit_lib::MethodId(method_id))
    }

    /// Get payment status for a receipt.
    pub fn get_payment_status(&self, receipt_id: String) -> Option<PaymentStatusInfo> {
        self.status_tracker
            .get(&receipt_id)
            .map(|info| PaymentStatusInfo {
                status: match info.status {
                    paykit_interactive::PaymentStatus::Pending => PaymentStatus::Pending,
                    paykit_interactive::PaymentStatus::Processing => PaymentStatus::Processing,
                    paykit_interactive::PaymentStatus::Confirmed => PaymentStatus::Confirmed,
                    paykit_interactive::PaymentStatus::Finalized => PaymentStatus::Finalized,
                    paykit_interactive::PaymentStatus::Failed => PaymentStatus::Failed,
                    paykit_interactive::PaymentStatus::Cancelled => PaymentStatus::Cancelled,
                    paykit_interactive::PaymentStatus::Expired => PaymentStatus::Expired,
                },
                receipt_id: info.receipt_id,
                method_id: info.method_id.0,
                updated_at: info.updated_at,
                confirmations: info.confirmations,
                required_confirmations: info.required_confirmations,
                error: info.error,
            })
    }

    /// Get all in-progress payments.
    pub fn get_in_progress_payments(&self) -> Vec<PaymentStatusInfo> {
        self.status_tracker
            .get_in_progress()
            .into_iter()
            .map(|info| PaymentStatusInfo {
                status: match info.status {
                    paykit_interactive::PaymentStatus::Pending => PaymentStatus::Pending,
                    paykit_interactive::PaymentStatus::Processing => PaymentStatus::Processing,
                    paykit_interactive::PaymentStatus::Confirmed => PaymentStatus::Confirmed,
                    paykit_interactive::PaymentStatus::Finalized => PaymentStatus::Finalized,
                    paykit_interactive::PaymentStatus::Failed => PaymentStatus::Failed,
                    paykit_interactive::PaymentStatus::Cancelled => PaymentStatus::Cancelled,
                    paykit_interactive::PaymentStatus::Expired => PaymentStatus::Expired,
                },
                receipt_id: info.receipt_id,
                method_id: info.method_id.0,
                updated_at: info.updated_at,
                confirmations: info.confirmations,
                required_confirmations: info.required_confirmations,
                error: info.error,
            })
            .collect()
    }

    // ========================================================================
    // Executor Registration Methods (Bitkit Integration)
    // ========================================================================

    /// Register a Bitcoin executor for on-chain payments.
    ///
    /// This allows Bitkit or other wallets to provide their wallet implementation
    /// as the executor for on-chain Bitcoin payments. The executor handles:
    /// - Sending payments to addresses
    /// - Estimating fees
    /// - Verifying transactions
    ///
    /// # Arguments
    ///
    /// * `executor` - Implementation of `BitcoinExecutorFFI` from the wallet
    ///
    /// # Example (Swift)
    ///
    /// ```swift
    /// class BitkitBitcoinExecutor: BitcoinExecutorFFI {
    ///     func sendToAddress(address: String, amountSats: UInt64, feeRate: Double?) throws -> BitcoinTxResultFFI {
    ///         // Implement using Bitkit wallet
    ///     }
    ///     // ... other methods
    /// }
    ///
    /// let client = PaykitClient.newWithNetwork(
    ///     bitcoinNetwork: .mainnet,
    ///     lightningNetwork: .mainnet
    /// )
    /// try client.registerBitcoinExecutor(executor: BitkitBitcoinExecutor())
    /// ```
    pub fn register_bitcoin_executor(
        &self,
        executor: Box<dyn executor_ffi::BitcoinExecutorFFI>,
    ) -> Result<()> {
        // Create a bridge that wraps the FFI executor
        let bridge = executor_ffi::BitcoinExecutorBridge::new(Arc::from(executor));

        // Create a new OnchainPlugin with the executor and network
        let plugin = paykit_lib::methods::OnchainPlugin::with_network_and_executor(
            self.bitcoin_network.into(),
            Arc::new(bridge),
        );

        // Register the plugin (replaces the default one)
        self.registry.write().unwrap_or_else(|e| e.into_inner()).register(Box::new(plugin));

        Ok(())
    }

    /// Register a Lightning executor for Lightning Network payments.
    ///
    /// This allows Bitkit or other wallets to provide their Lightning node
    /// implementation as the executor for Lightning payments. The executor handles:
    /// - Paying BOLT11 invoices
    /// - Decoding invoices
    /// - Estimating routing fees
    /// - Verifying payments via preimage
    ///
    /// # Arguments
    ///
    /// * `executor` - Implementation of `LightningExecutorFFI` from the wallet
    ///
    /// # Example (Swift)
    ///
    /// ```swift
    /// class BitkitLightningExecutor: LightningExecutorFFI {
    ///     func payInvoice(invoice: String, amountMsat: UInt64?, maxFeeMsat: UInt64?) throws -> LightningPaymentResultFFI {
    ///         // Implement using Bitkit Lightning node
    ///     }
    ///     // ... other methods
    /// }
    ///
    /// try client.registerLightningExecutor(executor: BitkitLightningExecutor())
    /// ```
    pub fn register_lightning_executor(
        &self,
        executor: Box<dyn executor_ffi::LightningExecutorFFI>,
    ) -> Result<()> {
        // Create a bridge that wraps the FFI executor
        let bridge = executor_ffi::LightningExecutorBridge::new(Arc::from(executor));

        // Create a new LightningPlugin with the executor and network
        let plugin = paykit_lib::methods::LightningPlugin::with_network_and_executor(
            self.lightning_network.into(),
            Arc::new(bridge),
        );

        // Register the plugin (replaces the default one)
        self.registry.write().unwrap_or_else(|e| e.into_inner()).register(Box::new(plugin));

        Ok(())
    }

    /// Check if a Bitcoin executor has been registered.
    ///
    /// Note: This checks if the onchain method is registered. After calling
    /// `register_bitcoin_executor`, this will return true.
    pub fn has_bitcoin_executor(&self) -> bool {
        self.registry
            .read()
            .unwrap()
            .get(&paykit_lib::MethodId("onchain".to_string()))
            .is_some()
    }

    /// Check if a Lightning executor has been registered.
    ///
    /// Note: This checks if the lightning method is registered. After calling
    /// `register_lightning_executor`, this will return true.
    pub fn has_lightning_executor(&self) -> bool {
        self.registry
            .read()
            .unwrap()
            .get(&paykit_lib::MethodId("lightning".to_string()))
            .is_some()
    }

    // ========================================================================
    // Payment Execution Methods
    // ========================================================================

    /// Execute a payment using the registered executor.
    ///
    /// This method executes a real payment using the wallet executor that was
    /// registered via `register_bitcoin_executor` or `register_lightning_executor`.
    ///
    /// # Arguments
    ///
    /// * `method_id` - Payment method ("onchain" or "lightning")
    /// * `endpoint` - Payment destination (Bitcoin address or Lightning invoice)
    /// * `amount_sats` - Amount to send in satoshis
    /// * `metadata_json` - Optional JSON metadata (e.g., fee rate preferences)
    ///
    /// # Returns
    ///
    /// `PaymentExecutionResult` with success/failure status and execution details.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // After registering executors
    /// let result = client.execute_payment(
    ///     "lightning",
    ///     "lnbc1000n1...",
    ///     1000,
    ///     None
    /// )?;
    ///
    /// if result.success {
    ///     println!("Payment succeeded: {}", result.execution_id);
    /// }
    /// ```
    pub fn execute_payment(
        &self,
        method_id: String,
        endpoint: String,
        amount_sats: u64,
        metadata_json: Option<String>,
    ) -> Result<PaymentExecutionResult> {
        let plugin = self
            .registry
            .read()
            .unwrap()
            .get(&paykit_lib::MethodId(method_id.clone()))
            .ok_or(PaykitMobileError::NotFound {
                msg: format!("Payment method not registered: {}", method_id),
            })?;

        let endpoint_data = paykit_lib::EndpointData(endpoint.clone());
        let amount = paykit_lib::methods::Amount::sats(amount_sats);

        let metadata: serde_json::Value = metadata_json
            .as_ref()
            .map(|s| serde_json::from_str(s).unwrap_or(serde_json::json!({})))
            .unwrap_or(serde_json::json!({}));

        // Execute payment asynchronously
        let execution = self.runtime.block_on(async {
            plugin
                .execute_payment(&endpoint_data, &amount, &metadata)
                .await
        })?;

        Ok(PaymentExecutionResult {
            execution_id: format!("exec_{}", rand_suffix()),
            method_id,
            endpoint,
            amount_sats,
            success: execution.success,
            executed_at: execution.executed_at,
            execution_data_json: serde_json::to_string(&execution.execution_data)
                .unwrap_or_default(),
            error: execution.error,
        })
    }

    /// Generate a payment proof from an execution result.
    ///
    /// After a successful payment, call this to generate cryptographic proof
    /// of payment (e.g., transaction ID for on-chain, preimage for Lightning).
    ///
    /// # Arguments
    ///
    /// * `method_id` - Payment method used
    /// * `execution_data_json` - The execution data from `execute_payment` result
    ///
    /// # Returns
    ///
    /// `PaymentProofResult` containing the proof type and data.
    pub fn generate_payment_proof(
        &self,
        method_id: String,
        execution_data_json: String,
    ) -> Result<PaymentProofResult> {
        let plugin = self
            .registry
            .read()
            .unwrap()
            .get(&paykit_lib::MethodId(method_id.clone()))
            .ok_or(PaykitMobileError::NotFound {
                msg: format!("Payment method not registered: {}", method_id),
            })?;

        // Parse execution data
        let execution_data: serde_json::Value = serde_json::from_str(&execution_data_json)
            .map_err(|e| PaykitMobileError::Serialization {
                msg: e.to_string(),
            })?;

        // Extract amount - try amount_sats first, then convert from amount_msat
        let amount_sats = execution_data
            .get("amount_sats")
            .and_then(|v| v.as_u64())
            .or_else(|| {
                execution_data
                    .get("amount_msat")
                    .and_then(|v| v.as_u64())
                    .map(|msat| msat / 1000)
            })
            .unwrap_or(0);

        // Create a PaymentExecution from the data
        let execution = paykit_lib::methods::PaymentExecution {
            method_id: paykit_lib::MethodId(method_id.clone()),
            endpoint: paykit_lib::EndpointData(
                execution_data
                    .get("address")
                    .or_else(|| execution_data.get("invoice"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            ),
            amount: paykit_lib::methods::Amount::sats(amount_sats),
            success: true,
            executed_at: execution_data
                .get("executed_at")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            execution_data: execution_data.clone(),
            error: None,
        };

        let proof = plugin.generate_proof(&execution)?;

        Ok(PaymentProofResult {
            proof_type: match &proof {
                paykit_lib::methods::PaymentProof::BitcoinTxid { .. } => "bitcoin_txid".to_string(),
                paykit_lib::methods::PaymentProof::LightningPreimage { .. } => {
                    "lightning_preimage".to_string()
                }
                paykit_lib::methods::PaymentProof::Custom { .. } => "custom".to_string(),
            },
            proof_data_json: serde_json::to_string(&proof).unwrap_or_default(),
        })
    }

    // ========================================================================
    // Subscription Methods
    // ========================================================================

    /// Create a new subscription.
    pub fn create_subscription(
        &self,
        subscriber: String,
        provider: String,
        terms: SubscriptionTerms,
    ) -> Result<Subscription> {
        use std::str::FromStr;

        let subscriber_key = paykit_lib::PublicKey::from_str(&subscriber).map_err(|e| {
            PaykitMobileError::Validation {
                msg: format!("Invalid subscriber key: {}", e),
            }
        })?;
        let provider_key = paykit_lib::PublicKey::from_str(&provider).map_err(|e| {
            PaykitMobileError::Validation {
                msg: format!("Invalid provider key: {}", e),
            }
        })?;

        // Convert frequency first to avoid partial move
        let lib_frequency = match &terms.frequency {
            PaymentFrequency::Daily => paykit_subscriptions::PaymentFrequency::Daily,
            PaymentFrequency::Weekly => paykit_subscriptions::PaymentFrequency::Weekly,
            PaymentFrequency::Monthly { day_of_month } => {
                paykit_subscriptions::PaymentFrequency::Monthly {
                    day_of_month: *day_of_month,
                }
            }
            PaymentFrequency::Yearly { month, day } => {
                paykit_subscriptions::PaymentFrequency::Yearly {
                    month: *month,
                    day: *day,
                }
            }
            PaymentFrequency::Custom { interval_seconds } => {
                paykit_subscriptions::PaymentFrequency::Custom {
                    interval_seconds: *interval_seconds,
                }
            }
        };

        let lib_terms = paykit_subscriptions::SubscriptionTerms::new(
            paykit_subscriptions::Amount::from_sats(terms.amount_sats),
            terms.currency.clone(),
            lib_frequency,
            paykit_lib::MethodId(terms.method_id.clone()),
            terms.description.clone(),
        );

        let sub = paykit_subscriptions::Subscription::new(subscriber_key, provider_key, lib_terms);

        Ok(Subscription {
            subscription_id: sub.subscription_id.clone(),
            subscriber,
            provider,
            terms,
            created_at: sub.created_at,
            starts_at: sub.starts_at,
            ends_at: sub.ends_at,
            is_active: sub.is_active(),
        })
    }

    /// Calculate proration for a subscription modification.
    pub fn calculate_proration(
        &self,
        current_amount_sats: i64,
        new_amount_sats: i64,
        period_start: i64,
        period_end: i64,
        change_date: i64,
    ) -> Result<ProrationResult> {
        let calculator = paykit_subscriptions::ProrationCalculator::new();

        let result = calculator
            .calculate(
                &paykit_subscriptions::Amount::from_sats(current_amount_sats),
                &paykit_subscriptions::Amount::from_sats(new_amount_sats),
                period_start,
                period_end,
                change_date,
                "SAT",
            )
            .map_err(|e| PaykitMobileError::Validation {
                msg: e.to_string(),
            })?;

        Ok(ProrationResult {
            credit_sats: result.credit.as_sats(),
            charge_sats: result.charge.as_sats(),
            net_sats: result.net_amount.as_sats(),
            is_refund: result.is_refund(),
        })
    }

    /// Get days remaining in current billing period.
    pub fn days_remaining_in_period(&self, period_end: i64) -> u32 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let remaining_seconds = period_end - now;
        if remaining_seconds <= 0 {
            return 0;
        }
        (remaining_seconds / 86400) as u32
    }

    // ========================================================================
    // Payment Request Methods
    // ========================================================================

    /// Create a payment request.
    #[allow(clippy::too_many_arguments)]
    pub fn create_payment_request(
        &self,
        from_pubkey: String,
        to_pubkey: String,
        amount_sats: i64,
        currency: String,
        method_id: String,
        description: String,
        expires_in_secs: Option<u64>,
    ) -> Result<PaymentRequest> {
        use std::str::FromStr;

        let from_key = paykit_lib::PublicKey::from_str(&from_pubkey).map_err(|e| {
            PaykitMobileError::Validation {
                msg: format!("Invalid from key: {}", e),
            }
        })?;
        let to_key = paykit_lib::PublicKey::from_str(&to_pubkey).map_err(|e| {
            PaykitMobileError::Validation {
                msg: format!("Invalid to key: {}", e),
            }
        })?;

        let request = paykit_subscriptions::PaymentRequest::new(
            from_key,
            to_key,
            paykit_subscriptions::Amount::from_sats(amount_sats),
            currency.clone(),
            paykit_lib::MethodId(method_id.clone()),
        )
        .with_description(description.clone());

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let expires_at = expires_in_secs.map(|secs| now + secs as i64);

        Ok(PaymentRequest {
            request_id: request.request_id.clone(),
            from_pubkey,
            to_pubkey,
            amount_sats,
            currency,
            method_id,
            description,
            created_at: request.created_at,
            expires_at,
        })
    }

    // ========================================================================
    // Receipt Methods
    // ========================================================================

    /// Create a new receipt.
    pub fn create_receipt(
        &self,
        payer: String,
        payee: String,
        method_id: String,
        amount: Option<String>,
        currency: Option<String>,
    ) -> Result<Receipt> {
        use std::str::FromStr;

        let payer_key =
            paykit_lib::PublicKey::from_str(&payer).map_err(|e| PaykitMobileError::Validation {
                msg: format!("Invalid payer key: {}", e),
            })?;
        let payee_key =
            paykit_lib::PublicKey::from_str(&payee).map_err(|e| PaykitMobileError::Validation {
                msg: format!("Invalid payee key: {}", e),
            })?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let receipt_id = format!("rcpt_{}_{}", now, rand_suffix());

        let receipt = paykit_interactive::PaykitReceipt::new(
            receipt_id.clone(),
            payer_key,
            payee_key,
            paykit_lib::MethodId(method_id.clone()),
            amount.clone(),
            currency.clone(),
            serde_json::json!({}),
        );

        Ok(Receipt {
            receipt_id,
            payer,
            payee,
            method_id,
            amount,
            currency,
            created_at: receipt.created_at,
            metadata_json: "{}".to_string(),
        })
    }

    /// Parse receipt metadata as JSON.
    pub fn parse_receipt_metadata(&self, metadata_json: String) -> Result<String> {
        // Validate JSON
        serde_json::from_str::<serde_json::Value>(&metadata_json).map_err(|e| {
            PaykitMobileError::Serialization {
                msg: e.to_string(),
            }
        })?;
        Ok(metadata_json)
    }

    // ========================================================================
    // Scanner Methods
    // ========================================================================

    /// Parse scanned QR code data as a Paykit URI.
    pub fn parse_scanned_qr(&self, scanned_data: String) -> Result<scanner::ScannedUri> {
        scanner::parse_scanned_uri(scanned_data)
            .map_err(|e| PaykitMobileError::Validation { msg: e })
    }

    /// Check if scanned data looks like a Paykit URI.
    pub fn is_paykit_qr(&self, scanned_data: String) -> bool {
        scanner::is_paykit_uri(scanned_data)
    }

    /// Extract public key from scanned QR code.
    pub fn extract_key_from_qr(&self, scanned_data: String) -> Option<String> {
        scanner::extract_public_key(scanned_data)
    }

    /// Extract payment method from scanned QR code.
    pub fn extract_method_from_qr(&self, scanned_data: String) -> Option<String> {
        scanner::extract_payment_method(scanned_data)
    }

    // ========================================================================
    // Directory Operations
    // ========================================================================

    /// Publish a payment endpoint to the directory.
    ///
    /// # Arguments
    ///
    /// * `transport` - Authenticated transport for the owner
    /// * `method_id` - Payment method identifier (e.g., "lightning", "onchain")
    /// * `endpoint_data` - The endpoint data to publish
    ///
    /// # Example
    ///
    /// ```ignore
    /// let transport = AuthenticatedTransportFFI::from_session_json(session, pubkey)?;
    /// client.publish_payment_endpoint(transport, "lightning", "lnbc1...")?;
    /// ```
    pub fn publish_payment_endpoint(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
        method_id: String,
        endpoint_data: String,
    ) -> Result<()> {
        transport_ffi::publish_payment_endpoint(&transport, &method_id, &endpoint_data)
    }

    /// Remove a payment endpoint from the directory.
    ///
    /// # Arguments
    ///
    /// * `transport` - Authenticated transport for the owner
    /// * `method_id` - Payment method identifier to remove
    pub fn remove_payment_endpoint_from_directory(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
        method_id: String,
    ) -> Result<()> {
        transport_ffi::remove_payment_endpoint(&transport, &method_id)
    }

    /// Fetch all supported payment methods for a public key.
    ///
    /// # Arguments
    ///
    /// * `transport` - Unauthenticated transport for reading
    /// * `owner_pubkey` - The public key to query (z-base32 encoded)
    ///
    /// # Returns
    ///
    /// List of payment methods with their endpoints.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let transport = UnauthenticatedTransportFFI::new_mock();
    /// let methods = client.fetch_supported_payments(transport, "8pinxxgqs41...")?;
    /// for method in methods {
    ///     println!("{}: {}", method.method_id, method.endpoint);
    /// }
    /// ```
    pub fn fetch_supported_payments(
        &self,
        transport: Arc<UnauthenticatedTransportFFI>,
        owner_pubkey: String,
    ) -> Result<Vec<PaymentMethod>> {
        transport_ffi::fetch_supported_payments(&transport, &owner_pubkey)
    }

    /// Fetch a specific payment endpoint for a public key.
    ///
    /// # Arguments
    ///
    /// * `transport` - Unauthenticated transport for reading
    /// * `owner_pubkey` - The public key to query
    /// * `method_id` - The payment method to fetch
    ///
    /// # Returns
    ///
    /// The endpoint data if found, None otherwise.
    pub fn fetch_payment_endpoint(
        &self,
        transport: Arc<UnauthenticatedTransportFFI>,
        owner_pubkey: String,
        method_id: String,
    ) -> Result<Option<String>> {
        transport_ffi::fetch_payment_endpoint(&transport, &owner_pubkey, &method_id)
    }

    /// Fetch known contacts for a public key.
    ///
    /// # Arguments
    ///
    /// * `transport` - Unauthenticated transport for reading
    /// * `owner_pubkey` - The public key to query
    ///
    /// # Returns
    ///
    /// List of contact public keys.
    pub fn fetch_known_contacts(
        &self,
        transport: Arc<UnauthenticatedTransportFFI>,
        owner_pubkey: String,
    ) -> Result<Vec<String>> {
        transport_ffi::fetch_known_contacts(&transport, &owner_pubkey)
    }

    // ========================================================================
    // Contact Management
    // ========================================================================

    /// Add a contact to the follows list.
    ///
    /// # Arguments
    ///
    /// * `transport` - Authenticated transport for the owner
    /// * `contact_pubkey` - The contact's public key to add
    pub fn add_contact(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
        contact_pubkey: String,
    ) -> Result<()> {
        transport_ffi::add_contact(&transport, &contact_pubkey)
    }

    /// Remove a contact from the follows list.
    ///
    /// # Arguments
    ///
    /// * `transport` - Authenticated transport for the owner
    /// * `contact_pubkey` - The contact's public key to remove
    pub fn remove_contact(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
        contact_pubkey: String,
    ) -> Result<()> {
        transport_ffi::remove_contact(&transport, &contact_pubkey)
    }

    /// List all contacts.
    ///
    /// # Arguments
    ///
    /// * `transport` - Authenticated transport for the owner
    ///
    /// # Returns
    ///
    /// List of contact public keys.
    pub fn list_contacts(&self, transport: Arc<AuthenticatedTransportFFI>) -> Result<Vec<String>> {
        transport_ffi::list_contacts(&transport)
    }

    // ========================================================================
    // Noise Protocol Operations
    // ========================================================================

    /// Discover a Noise endpoint for a recipient.
    ///
    /// Queries the recipient's public directory for their Noise server information.
    ///
    /// # Arguments
    ///
    /// * `transport` - Unauthenticated transport for reading
    /// * `recipient_pubkey` - The recipient's public key (z-base32 encoded)
    ///
    /// # Returns
    ///
    /// The noise endpoint info if found, None otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let transport = UnauthenticatedTransportFFI::new_mock();
    /// if let Some(endpoint) = client.discover_noise_endpoint(transport, "8pinxxgqs41...")? {
    ///     println!("Connecting to {}:{}", endpoint.host, endpoint.port);
    /// }
    /// ```
    pub fn discover_noise_endpoint(
        &self,
        transport: Arc<UnauthenticatedTransportFFI>,
        recipient_pubkey: String,
    ) -> Result<Option<NoiseEndpointInfo>> {
        noise_ffi::discover_noise_endpoint(transport, recipient_pubkey)
    }

    /// Publish a Noise endpoint to the directory.
    ///
    /// Makes this device discoverable for receiving payments via Noise protocol.
    ///
    /// # Arguments
    ///
    /// * `transport` - Authenticated transport for writing
    /// * `host` - Host address where the Noise server is listening
    /// * `port` - Port number where the Noise server is listening
    /// * `noise_pubkey` - This server's Noise public key (X25519, hex encoded)
    /// * `metadata` - Optional metadata about the endpoint
    pub fn publish_noise_endpoint(
        &self,
        transport: Arc<AuthenticatedTransportFFI>,
        host: String,
        port: u16,
        noise_pubkey: String,
        metadata: Option<String>,
    ) -> Result<()> {
        noise_ffi::publish_noise_endpoint(transport, host, port, noise_pubkey, metadata)
    }

    /// Remove the Noise endpoint from the directory.
    ///
    /// Makes this device no longer discoverable for Noise payments.
    ///
    /// # Arguments
    ///
    /// * `transport` - Authenticated transport for writing
    pub fn remove_noise_endpoint(&self, transport: Arc<AuthenticatedTransportFFI>) -> Result<()> {
        noise_ffi::remove_noise_endpoint(transport)
    }

    /// Create a receipt request message for Noise channel.
    ///
    /// # Arguments
    ///
    /// * `receipt_id` - Unique identifier for this receipt
    /// * `payer_pubkey` - Payer's public key (z-base32)
    /// * `payee_pubkey` - Payee's public key (z-base32)
    /// * `method_id` - Payment method identifier
    /// * `amount` - Optional payment amount
    /// * `currency` - Optional currency code
    pub fn create_receipt_request_message(
        &self,
        receipt_id: String,
        payer_pubkey: String,
        payee_pubkey: String,
        method_id: String,
        amount: Option<String>,
        currency: Option<String>,
    ) -> Result<NoisePaymentMessage> {
        noise_ffi::create_receipt_request_message(
            receipt_id,
            payer_pubkey,
            payee_pubkey,
            method_id,
            amount,
            currency,
        )
    }

    /// Create a receipt confirmation message for Noise channel.
    ///
    /// # Arguments
    ///
    /// * `receipt_id` - The receipt ID being confirmed
    /// * `payer_pubkey` - Payer's public key
    /// * `payee_pubkey` - Payee's public key
    /// * `method_id` - Payment method used
    /// * `amount` - Payment amount
    /// * `currency` - Currency code
    /// * `signature` - Optional signature from payee
    #[allow(clippy::too_many_arguments)]
    pub fn create_receipt_confirmation_message(
        &self,
        receipt_id: String,
        payer_pubkey: String,
        payee_pubkey: String,
        method_id: String,
        amount: Option<String>,
        currency: Option<String>,
        signature: Option<String>,
    ) -> Result<NoisePaymentMessage> {
        noise_ffi::create_receipt_confirmation_message(
            receipt_id,
            payer_pubkey,
            payee_pubkey,
            method_id,
            amount,
            currency,
            signature,
        )
    }

    /// Create an error message for Noise channel.
    ///
    /// # Arguments
    ///
    /// * `code` - Error code
    /// * `message` - Error description
    pub fn create_noise_error_message(
        &self,
        code: String,
        message: String,
    ) -> Result<NoisePaymentMessage> {
        noise_ffi::create_error_message(code, message)
    }

    /// Parse a payment message from JSON.
    ///
    /// # Arguments
    ///
    /// * `json` - The JSON string to parse
    pub fn parse_noise_payment_message(&self, json: String) -> Result<NoisePaymentMessage> {
        noise_ffi::parse_payment_message(json)
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Create a new Paykit client.
#[uniffi::export]
pub fn create_paykit_client() -> Result<Arc<PaykitClient>> {
    PaykitClient::new()
}

/// Get the library version.
#[uniffi::export]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Generate a random suffix for IDs.
fn rand_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{:08x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_client() {
        let client = PaykitClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_list_methods() {
        let client = PaykitClient::new().unwrap();
        let methods = client.list_methods();
        assert!(methods.contains(&"onchain".to_string()));
        assert!(methods.contains(&"lightning".to_string()));
    }

    #[test]
    fn test_validate_endpoint() {
        let client = PaykitClient::new().unwrap();

        // Valid Bitcoin address
        let result = client.validate_endpoint(
            "onchain".to_string(),
            "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".to_string(),
        );
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_select_method() {
        let client = PaykitClient::new().unwrap();

        let methods = vec![
            PaymentMethod {
                method_id: "lightning".to_string(),
                endpoint: "lnbc...".to_string(),
            },
            PaymentMethod {
                method_id: "onchain".to_string(),
                endpoint: "bc1q...".to_string(),
            },
        ];

        let result = client.select_method(methods, 10000, None);
        assert!(result.is_ok());

        let selection = result.unwrap();
        assert_eq!(selection.primary_method, "lightning");
    }

    #[test]
    fn test_check_health() {
        let client = PaykitClient::new().unwrap();
        let results = client.check_health();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_calculate_proration() {
        let client = PaykitClient::new().unwrap();

        // 30-day period, upgrade on day 10
        let period_start = 0;
        let period_end = 30 * 86400;
        let change_date = 10 * 86400;

        let result = client.calculate_proration(
            3000, // current
            6000, // new
            period_start,
            period_end,
            change_date,
        );

        assert!(result.is_ok());
        let proration = result.unwrap();
        assert!(proration.net_sats > 0); // Should be a charge
        assert!(!proration.is_refund);
    }

    #[test]
    fn test_days_remaining() {
        let client = PaykitClient::new().unwrap();

        // Period ends in 10 days
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let period_end = now + (10 * 86400);

        let remaining = client.days_remaining_in_period(period_end);
        assert!((9..=10).contains(&remaining));
    }

    #[test]
    fn test_payment_frequency_conversion() {
        let monthly = PaymentFrequency::Monthly { day_of_month: 15 };
        let lib_freq: paykit_subscriptions::PaymentFrequency = monthly.into();

        match lib_freq {
            paykit_subscriptions::PaymentFrequency::Monthly { day_of_month } => {
                assert_eq!(day_of_month, 15);
            }
            _ => panic!("Expected Monthly frequency"),
        }
    }

    // ========================================================================
    // Directory Operation Tests
    // ========================================================================

    #[test]
    fn test_publish_and_fetch_payment_endpoint() {
        let client = PaykitClient::new().unwrap();
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

        // Publish endpoint
        client
            .publish_payment_endpoint(
                auth.clone(),
                "lightning".to_string(),
                "lnbc1...".to_string(),
            )
            .unwrap();

        // Fetch endpoint
        let result = client
            .fetch_payment_endpoint(
                unauth.clone(),
                "test_owner".to_string(),
                "lightning".to_string(),
            )
            .unwrap();

        assert_eq!(result, Some("lnbc1...".to_string()));
    }

    #[test]
    fn test_fetch_supported_payments() {
        let client = PaykitClient::new().unwrap();
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

        // Publish multiple endpoints
        client
            .publish_payment_endpoint(
                auth.clone(),
                "lightning".to_string(),
                "lnbc1...".to_string(),
            )
            .unwrap();
        client
            .publish_payment_endpoint(auth.clone(), "onchain".to_string(), "bc1q...".to_string())
            .unwrap();

        // Fetch all
        let methods = client
            .fetch_supported_payments(unauth, "test_owner".to_string())
            .unwrap();

        assert_eq!(methods.len(), 2);
        assert!(methods.iter().any(|m| m.method_id == "lightning"));
        assert!(methods.iter().any(|m| m.method_id == "onchain"));
    }

    #[test]
    fn test_remove_payment_endpoint_from_directory() {
        let client = PaykitClient::new().unwrap();
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

        // Publish and remove
        client
            .publish_payment_endpoint(
                auth.clone(),
                "lightning".to_string(),
                "lnbc1...".to_string(),
            )
            .unwrap();
        client
            .remove_payment_endpoint_from_directory(auth.clone(), "lightning".to_string())
            .unwrap();

        // Verify removed
        let result = client
            .fetch_payment_endpoint(unauth, "test_owner".to_string(), "lightning".to_string())
            .unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_fetch_known_contacts() {
        let client = PaykitClient::new().unwrap();
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

        // Add contacts
        client
            .add_contact(auth.clone(), "contact1".to_string())
            .unwrap();
        client
            .add_contact(auth.clone(), "contact2".to_string())
            .unwrap();

        // Fetch via unauthenticated transport
        let contacts = client
            .fetch_known_contacts(unauth, "test_owner".to_string())
            .unwrap();

        assert_eq!(contacts.len(), 2);
        assert!(contacts.contains(&"contact1".to_string()));
        assert!(contacts.contains(&"contact2".to_string()));
    }

    #[test]
    fn test_contact_management() {
        let client = PaykitClient::new().unwrap();
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());

        // Add contacts
        client
            .add_contact(auth.clone(), "contact1".to_string())
            .unwrap();
        client
            .add_contact(auth.clone(), "contact2".to_string())
            .unwrap();

        // List contacts
        let contacts = client.list_contacts(auth.clone()).unwrap();
        assert_eq!(contacts.len(), 2);

        // Remove contact
        client
            .remove_contact(auth.clone(), "contact1".to_string())
            .unwrap();
        let contacts = client.list_contacts(auth.clone()).unwrap();
        assert_eq!(contacts.len(), 1);
        assert!(contacts.contains(&"contact2".to_string()));
    }

    // ========================================================================
    // Noise Protocol Tests
    // ========================================================================

    #[test]
    fn test_publish_and_discover_noise_endpoint() {
        let client = PaykitClient::new().unwrap();
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

        // Publish noise endpoint
        client
            .publish_noise_endpoint(
                auth.clone(),
                "127.0.0.1".to_string(),
                8888,
                "abcd1234".to_string(),
                Some("Test server".to_string()),
            )
            .unwrap();

        // Discover noise endpoint
        let result = client
            .discover_noise_endpoint(unauth.clone(), "test_owner".to_string())
            .unwrap();

        assert!(result.is_some());
        let endpoint = result.unwrap();
        assert_eq!(endpoint.host, "127.0.0.1");
        assert_eq!(endpoint.port, 8888);
        assert_eq!(endpoint.server_noise_pubkey, "abcd1234");
    }

    #[test]
    fn test_remove_noise_endpoint() {
        let client = PaykitClient::new().unwrap();
        let auth = AuthenticatedTransportFFI::new_mock("test_owner".to_string());
        let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

        // Publish and remove
        client
            .publish_noise_endpoint(
                auth.clone(),
                "127.0.0.1".to_string(),
                8888,
                "abcd1234".to_string(),
                None,
            )
            .unwrap();

        client.remove_noise_endpoint(auth.clone()).unwrap();

        // Verify removed
        let result = client
            .discover_noise_endpoint(unauth, "test_owner".to_string())
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_create_receipt_request_message() {
        let client = PaykitClient::new().unwrap();

        let msg = client
            .create_receipt_request_message(
                "rcpt_123".to_string(),
                "payer_pk".to_string(),
                "payee_pk".to_string(),
                "lightning".to_string(),
                Some("1000".to_string()),
                Some("SAT".to_string()),
            )
            .unwrap();

        assert!(matches!(
            msg.message_type,
            NoisePaymentMessageType::ReceiptRequest
        ));
        assert!(msg.payload_json.contains("rcpt_123"));
    }

    #[test]
    fn test_create_receipt_confirmation_message() {
        let client = PaykitClient::new().unwrap();

        let msg = client
            .create_receipt_confirmation_message(
                "rcpt_123".to_string(),
                "payer_pk".to_string(),
                "payee_pk".to_string(),
                "lightning".to_string(),
                Some("1000".to_string()),
                Some("SAT".to_string()),
                None,
            )
            .unwrap();

        assert!(matches!(
            msg.message_type,
            NoisePaymentMessageType::ReceiptConfirmation
        ));
    }

    #[test]
    fn test_parse_noise_payment_message() {
        let client = PaykitClient::new().unwrap();

        let json = r#"{"type":"request_receipt","receipt_id":"rcpt_123"}"#;
        let msg = client
            .parse_noise_payment_message(json.to_string())
            .unwrap();

        assert!(matches!(
            msg.message_type,
            NoisePaymentMessageType::ReceiptRequest
        ));
    }

    #[test]
    fn test_create_noise_error_message() {
        let client = PaykitClient::new().unwrap();

        let msg = client
            .create_noise_error_message(
                "payment_rejected".to_string(),
                "Insufficient funds".to_string(),
            )
            .unwrap();

        assert!(matches!(msg.message_type, NoisePaymentMessageType::Error));
        assert!(msg.payload_json.contains("payment_rejected"));
    }

    // ========================================================================
    // Phase 2: Executor Registration and Payment Execution Tests
    // ========================================================================

    #[test]
    fn test_new_with_network() {
        let client = PaykitClient::new_with_network(
            executor_ffi::BitcoinNetworkFFI::Testnet,
            executor_ffi::LightningNetworkFFI::Testnet,
        )
        .unwrap();

        assert_eq!(
            client.bitcoin_network(),
            executor_ffi::BitcoinNetworkFFI::Testnet
        );
        assert_eq!(
            client.lightning_network(),
            executor_ffi::LightningNetworkFFI::Testnet
        );
    }

    #[test]
    fn test_new_defaults_to_mainnet() {
        let client = PaykitClient::new().unwrap();

        assert_eq!(
            client.bitcoin_network(),
            executor_ffi::BitcoinNetworkFFI::Mainnet
        );
        assert_eq!(
            client.lightning_network(),
            executor_ffi::LightningNetworkFFI::Mainnet
        );
    }

    #[test]
    fn test_has_executors_default() {
        let client = PaykitClient::new().unwrap();

        // Default client has onchain and lightning methods registered
        assert!(client.has_bitcoin_executor());
        assert!(client.has_lightning_executor());
    }

    #[test]
    fn test_register_bitcoin_executor() {
        use std::sync::atomic::{AtomicU32, Ordering};

        struct MockBitcoinExecutor {
            call_count: AtomicU32,
        }

        impl executor_ffi::BitcoinExecutorFFI for MockBitcoinExecutor {
            fn send_to_address(
                &self,
                _address: String,
                _amount_sats: u64,
                _fee_rate: Option<f64>,
            ) -> Result<executor_ffi::BitcoinTxResultFFI> {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                Ok(executor_ffi::BitcoinTxResultFFI::new(
                    "test_txid_123".to_string(),
                    0,
                    210,
                    1.5,
                ))
            }

            fn estimate_fee(
                &self,
                _address: String,
                _amount_sats: u64,
                _target_blocks: u32,
            ) -> Result<u64> {
                Ok(210)
            }

            fn get_transaction(
                &self,
                _txid: String,
            ) -> Result<Option<executor_ffi::BitcoinTxResultFFI>> {
                Ok(None)
            }

            fn verify_transaction(
                &self,
                _txid: String,
                _address: String,
                _amount_sats: u64,
            ) -> Result<bool> {
                Ok(true)
            }
        }

        let client = PaykitClient::new_with_network(
            executor_ffi::BitcoinNetworkFFI::Testnet,
            executor_ffi::LightningNetworkFFI::Testnet,
        )
        .unwrap();

        let executor = Box::new(MockBitcoinExecutor {
            call_count: AtomicU32::new(0),
        });

        // Register the executor
        client.register_bitcoin_executor(executor).unwrap();

        // Verify it's registered
        assert!(client.has_bitcoin_executor());
    }

    #[test]
    fn test_register_lightning_executor() {
        use std::sync::atomic::{AtomicU32, Ordering};

        struct MockLightningExecutor {
            call_count: AtomicU32,
        }

        impl executor_ffi::LightningExecutorFFI for MockLightningExecutor {
            fn pay_invoice(
                &self,
                _invoice: String,
                amount_msat: Option<u64>,
                _max_fee_msat: Option<u64>,
            ) -> Result<executor_ffi::LightningPaymentResultFFI> {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                Ok(executor_ffi::LightningPaymentResultFFI::success(
                    "test_preimage".to_string(),
                    "test_hash".to_string(),
                    amount_msat.unwrap_or(1000000),
                    100,
                ))
            }

            fn decode_invoice(&self, _invoice: String) -> Result<executor_ffi::DecodedInvoiceFFI> {
                Ok(executor_ffi::DecodedInvoiceFFI {
                    payment_hash: "test_hash".to_string(),
                    amount_msat: Some(1000000),
                    description: Some("Test".to_string()),
                    description_hash: None,
                    payee: "test_payee".to_string(),
                    expiry: 3600,
                    timestamp: 1700000000,
                    expired: false,
                })
            }

            fn estimate_fee(&self, _invoice: String) -> Result<u64> {
                Ok(100)
            }

            fn get_payment(
                &self,
                _payment_hash: String,
            ) -> Result<Option<executor_ffi::LightningPaymentResultFFI>> {
                Ok(None)
            }

            fn verify_preimage(&self, _preimage: String, _payment_hash: String) -> bool {
                true
            }
        }

        let client = PaykitClient::new_with_network(
            executor_ffi::BitcoinNetworkFFI::Testnet,
            executor_ffi::LightningNetworkFFI::Testnet,
        )
        .unwrap();

        let executor = Box::new(MockLightningExecutor {
            call_count: AtomicU32::new(0),
        });

        // Register the executor
        client.register_lightning_executor(executor).unwrap();

        // Verify it's registered
        assert!(client.has_lightning_executor());
    }

    #[test]
    fn test_execute_payment_onchain() {
        struct MockBitcoinExecutor;

        impl executor_ffi::BitcoinExecutorFFI for MockBitcoinExecutor {
            fn send_to_address(
                &self,
                _address: String,
                _amount_sats: u64,
                _fee_rate: Option<f64>,
            ) -> Result<executor_ffi::BitcoinTxResultFFI> {
                Ok(executor_ffi::BitcoinTxResultFFI {
                    txid: "abc123def456".to_string(),
                    raw_tx: None,
                    vout: 0,
                    fee_sats: 210,
                    fee_rate: 1.5,
                    block_height: None,
                    confirmations: 0,
                })
            }

            fn estimate_fee(
                &self,
                _address: String,
                _amount_sats: u64,
                _target_blocks: u32,
            ) -> Result<u64> {
                Ok(210)
            }

            fn get_transaction(
                &self,
                _txid: String,
            ) -> Result<Option<executor_ffi::BitcoinTxResultFFI>> {
                Ok(None)
            }

            fn verify_transaction(
                &self,
                _txid: String,
                _address: String,
                _amount_sats: u64,
            ) -> Result<bool> {
                Ok(true)
            }
        }

        let client = PaykitClient::new_with_network(
            executor_ffi::BitcoinNetworkFFI::Testnet,
            executor_ffi::LightningNetworkFFI::Testnet,
        )
        .unwrap();

        client
            .register_bitcoin_executor(Box::new(MockBitcoinExecutor))
            .unwrap();

        // Execute a payment
        let result = client
            .execute_payment(
                "onchain".to_string(),
                "tb1qtest123".to_string(),
                10000,
                None,
            )
            .unwrap();

        assert!(result.success);
        assert_eq!(result.method_id, "onchain");
        assert_eq!(result.amount_sats, 10000);
        assert!(result.execution_data_json.contains("abc123def456"));
    }

    #[test]
    fn test_execute_payment_lightning() {
        struct MockLightningExecutor;

        impl executor_ffi::LightningExecutorFFI for MockLightningExecutor {
            fn pay_invoice(
                &self,
                _invoice: String,
                _amount_msat: Option<u64>,
                _max_fee_msat: Option<u64>,
            ) -> Result<executor_ffi::LightningPaymentResultFFI> {
                Ok(executor_ffi::LightningPaymentResultFFI::success(
                    "preimage_abc123".to_string(),
                    "hash_def456".to_string(),
                    1000000,
                    100,
                ))
            }

            fn decode_invoice(&self, _invoice: String) -> Result<executor_ffi::DecodedInvoiceFFI> {
                Ok(executor_ffi::DecodedInvoiceFFI {
                    payment_hash: "hash_def456".to_string(),
                    amount_msat: Some(1000000),
                    description: Some("Test payment".to_string()),
                    description_hash: None,
                    payee: "test_payee".to_string(),
                    expiry: 3600,
                    timestamp: 1700000000,
                    expired: false,
                })
            }

            fn estimate_fee(&self, _invoice: String) -> Result<u64> {
                Ok(100)
            }

            fn get_payment(
                &self,
                _payment_hash: String,
            ) -> Result<Option<executor_ffi::LightningPaymentResultFFI>> {
                Ok(None)
            }

            fn verify_preimage(&self, _preimage: String, _payment_hash: String) -> bool {
                true
            }
        }

        let client = PaykitClient::new_with_network(
            executor_ffi::BitcoinNetworkFFI::Testnet,
            executor_ffi::LightningNetworkFFI::Testnet,
        )
        .unwrap();

        client
            .register_lightning_executor(Box::new(MockLightningExecutor))
            .unwrap();

        // Execute a payment with a realistic-length invoice
        // Real BOLT11 invoices are typically 200+ characters
        let mock_invoice = format!(
            "lntb1000n1p{}",
            "0".repeat(200) // Pad to make it look like a real invoice
        );

        let result = client
            .execute_payment("lightning".to_string(), mock_invoice, 1000, None)
            .unwrap();

        assert!(result.success);
        assert_eq!(result.method_id, "lightning");
        assert!(result.execution_data_json.contains("preimage_abc123"));
    }

    #[test]
    fn test_execute_payment_method_not_found() {
        let client = PaykitClient::new().unwrap();

        let result = client.execute_payment(
            "unknown_method".to_string(),
            "some_endpoint".to_string(),
            1000,
            None,
        );

        assert!(result.is_err());
        match result {
            Err(PaykitMobileError::NotFound { msg }) => {
                assert!(msg.contains("unknown_method"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_generate_payment_proof() {
        let client = PaykitClient::new().unwrap();

        // Mock execution data for on-chain
        let execution_data = serde_json::json!({
            "txid": "abc123def456",
            "address": "bc1qtest",
            "amount_sats": 10000,
            "vout": 0,
            "fee_sats": 210
        });

        let result = client
            .generate_payment_proof(
                "onchain".to_string(),
                serde_json::to_string(&execution_data).unwrap(),
            )
            .unwrap();

        assert_eq!(result.proof_type, "bitcoin_txid");
        assert!(result.proof_data_json.contains("abc123def456"));
    }

    #[test]
    fn test_payment_execution_result_fields() {
        let result = PaymentExecutionResult {
            execution_id: "exec_123".to_string(),
            method_id: "lightning".to_string(),
            endpoint: "lnbc...".to_string(),
            amount_sats: 1000,
            success: true,
            executed_at: 1700000000,
            execution_data_json: "{}".to_string(),
            error: None,
        };

        assert_eq!(result.execution_id, "exec_123");
        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_payment_proof_result_fields() {
        let result = PaymentProofResult {
            proof_type: "lightning_preimage".to_string(),
            proof_data_json: r#"{"preimage":"abc123"}"#.to_string(),
        };

        assert_eq!(result.proof_type, "lightning_preimage");
        assert!(result.proof_data_json.contains("abc123"));
    }
}
