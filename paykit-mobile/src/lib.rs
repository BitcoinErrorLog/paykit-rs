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
pub mod interactive_ffi;
pub mod keys;
pub mod scanner;
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

use std::sync::Arc;

// UniFFI scaffolding
uniffi::setup_scaffolding!();

// ============================================================================
// Error Types
// ============================================================================

/// Mobile-friendly error type.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum PaykitMobileError {
    /// Transport layer error (network, I/O).
    #[error("Transport error: {message}")]
    Transport { message: String },

    /// Validation error (invalid input, format).
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Resource not found.
    #[error("Not found: {message}")]
    NotFound { message: String },

    /// Serialization/deserialization error.
    #[error("Serialization error: {message}")]
    Serialization { message: String },

    /// Internal error (unexpected state).
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// Network timeout error.
    #[error("Network timeout: {message}")]
    NetworkTimeout { message: String },

    /// Connection refused or failed.
    #[error("Connection error: {message}")]
    ConnectionError { message: String },

    /// Authentication failed.
    #[error("Authentication error: {message}")]
    AuthenticationError { message: String },

    /// Session expired or invalid.
    #[error("Session error: {message}")]
    SessionError { message: String },

    /// Rate limit exceeded.
    #[error("Rate limit exceeded: {message}")]
    RateLimitError { message: String },

    /// Permission denied.
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },
}

impl From<paykit_lib::PaykitError> for PaykitMobileError {
    fn from(e: paykit_lib::PaykitError) -> Self {
        match e {
            paykit_lib::PaykitError::Transport(msg) => Self::Transport { message: msg },
            paykit_lib::PaykitError::Unimplemented(msg) => Self::Internal {
                message: msg.to_string(),
            },
            paykit_lib::PaykitError::ConnectionFailed { target, reason } => Self::ConnectionError {
                message: format!("Connection to {} failed: {}", target, reason),
            },
            paykit_lib::PaykitError::ConnectionTimeout {
                operation,
                timeout_ms,
            } => Self::NetworkTimeout {
                message: format!("{} timed out after {}ms", operation, timeout_ms),
            },
            paykit_lib::PaykitError::Auth(msg) => Self::AuthenticationError { message: msg },
            paykit_lib::PaykitError::SessionExpired => Self::SessionError {
                message: "Session expired".to_string(),
            },
            paykit_lib::PaykitError::InvalidCredentials(msg) => {
                Self::AuthenticationError { message: msg }
            }
            paykit_lib::PaykitError::NotFound {
                resource_type,
                identifier,
            } => Self::NotFound {
                message: format!("{} not found: {}", resource_type, identifier),
            },
            paykit_lib::PaykitError::MethodNotSupported(method) => Self::Validation {
                message: format!("Payment method not supported: {}", method),
            },
            paykit_lib::PaykitError::InvalidData { field, reason } => Self::Validation {
                message: format!("Invalid {}: {}", field, reason),
            },
            paykit_lib::PaykitError::ValidationFailed(msg) => Self::Validation { message: msg },
            paykit_lib::PaykitError::Serialization(msg) => Self::Serialization { message: msg },
            paykit_lib::PaykitError::Payment { payment_id, reason } => Self::Transport {
                message: format!(
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
                message: format!(
                    "Insufficient funds: need {} {}, have {} {}",
                    required, currency, available, currency
                ),
            },
            paykit_lib::PaykitError::InvoiceExpired {
                invoice_id,
                expired_at,
            } => Self::Transport {
                message: format!("Invoice {} expired at {}", invoice_id, expired_at),
            },
            paykit_lib::PaykitError::PaymentRejected { payment_id, reason } => Self::Transport {
                message: format!("Payment {} rejected: {}", payment_id, reason),
            },
            paykit_lib::PaykitError::PaymentAlreadyCompleted { payment_id } => Self::Transport {
                message: format!("Payment {} already completed", payment_id),
            },
            paykit_lib::PaykitError::Storage(msg) => Self::Internal { message: msg },
            paykit_lib::PaykitError::QuotaExceeded { used, limit } => Self::Internal {
                message: format!("Quota exceeded: {} of {} used", used, limit),
            },
            paykit_lib::PaykitError::RateLimited { retry_after_ms } => Self::RateLimitError {
                message: format!("Rate limited, retry after {}ms", retry_after_ms),
            },
            paykit_lib::PaykitError::Internal(msg) => Self::Internal { message: msg },
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
    /// Plugin registry.
    registry: paykit_lib::methods::PaymentMethodRegistry,
    /// Health monitor.
    health_monitor: Arc<paykit_lib::health::HealthMonitor>,
    /// Status tracker.
    status_tracker: Arc<paykit_interactive::PaymentStatusTracker>,
    /// Tokio runtime for async operations.
    runtime: tokio::runtime::Runtime,
}

#[uniffi::export]
impl PaykitClient {
    /// Create a new Paykit client.
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>> {
        let runtime = tokio::runtime::Runtime::new().map_err(|e| PaykitMobileError::Internal {
            message: e.to_string(),
        })?;

        Ok(Arc::new(Self {
            registry: paykit_lib::methods::default_registry(),
            health_monitor: Arc::new(paykit_lib::health::HealthMonitor::with_defaults()),
            status_tracker: Arc::new(paykit_interactive::PaymentStatusTracker::new()),
            runtime,
        }))
    }

    /// Get the list of registered payment methods.
    pub fn list_methods(&self) -> Vec<String> {
        self.registry
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
            .get(&method)
            .ok_or(PaykitMobileError::NotFound {
                message: format!("Method not found: {}", method.0),
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

        let selector = PaymentMethodSelector::new(self.registry.clone());
        let result = selector.select(&supported, &amount, &prefs).map_err(|e| {
            PaykitMobileError::Validation {
                message: e.to_string(),
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
                message: format!("Invalid subscriber key: {}", e),
            }
        })?;
        let provider_key = paykit_lib::PublicKey::from_str(&provider).map_err(|e| {
            PaykitMobileError::Validation {
                message: format!("Invalid provider key: {}", e),
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
                message: e.to_string(),
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
                message: format!("Invalid from key: {}", e),
            }
        })?;
        let to_key = paykit_lib::PublicKey::from_str(&to_pubkey).map_err(|e| {
            PaykitMobileError::Validation {
                message: format!("Invalid to key: {}", e),
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
                message: format!("Invalid payer key: {}", e),
            })?;
        let payee_key =
            paykit_lib::PublicKey::from_str(&payee).map_err(|e| PaykitMobileError::Validation {
                message: format!("Invalid payee key: {}", e),
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
                message: e.to_string(),
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
            .map_err(|e| PaykitMobileError::Validation { message: e })
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
        assert!(remaining >= 9 && remaining <= 10);
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
}
