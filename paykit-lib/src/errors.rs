//! Error types for Paykit operations.
//!
//! This module provides structured error types for the Paykit library,
//! enabling precise error handling and recovery strategies.

use std::fmt;

/// Error codes for FFI and mobile integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PaykitErrorCode {
    /// Feature not implemented
    Unimplemented = 1000,
    /// Transport/network layer error
    Transport = 2000,
    /// Connection failed
    ConnectionFailed = 2001,
    /// Connection timeout
    ConnectionTimeout = 2002,
    /// Authentication/authorization error
    Auth = 3000,
    /// Session expired
    SessionExpired = 3001,
    /// Invalid credentials
    InvalidCredentials = 3002,
    /// Endpoint not found
    NotFound = 4000,
    /// Payment method not supported
    MethodNotSupported = 4001,
    /// Invalid request/data
    InvalidData = 5000,
    /// Validation failed
    ValidationFailed = 5001,
    /// Serialization error
    Serialization = 5002,
    /// Payment-specific errors
    Payment = 6000,
    /// Insufficient funds
    InsufficientFunds = 6001,
    /// Invoice expired
    InvoiceExpired = 6002,
    /// Payment rejected
    PaymentRejected = 6003,
    /// Payment already completed
    PaymentAlreadyCompleted = 6004,
    /// Storage error
    Storage = 7000,
    /// Quota exceeded
    QuotaExceeded = 7001,
    /// Rate limited
    RateLimited = 8000,
    /// Internal/unexpected error
    Internal = 9999,
}

/// Comprehensive error type for Paykit operations.
#[derive(Debug)]
pub enum PaykitError {
    /// Feature not implemented yet.
    Unimplemented(&'static str),

    /// Transport/network layer error.
    Transport(String),

    /// Connection failed.
    ConnectionFailed {
        /// Target endpoint or service
        target: String,
        /// Underlying error message
        reason: String,
    },

    /// Connection timeout.
    ConnectionTimeout {
        /// Operation that timed out
        operation: String,
        /// Timeout duration in milliseconds
        timeout_ms: u64,
    },

    /// Authentication or authorization failed.
    Auth(String),

    /// Session expired, needs re-authentication.
    SessionExpired,

    /// Invalid credentials provided.
    InvalidCredentials(String),

    /// Resource not found (endpoint, method, user, etc.).
    NotFound {
        /// Type of resource (e.g., "endpoint", "user", "method")
        resource_type: String,
        /// Resource identifier
        identifier: String,
    },

    /// Payment method not supported.
    MethodNotSupported(String),

    /// Invalid data provided.
    InvalidData {
        /// Field or parameter name
        field: String,
        /// Reason for invalidity
        reason: String,
    },

    /// Validation failed.
    ValidationFailed(String),

    /// Serialization/deserialization error.
    Serialization(String),

    /// Payment operation failed.
    Payment {
        /// Payment ID if available
        payment_id: Option<String>,
        /// Failure reason
        reason: String,
    },

    /// Insufficient funds for the requested payment.
    InsufficientFunds {
        /// Required amount (as string to handle various currencies)
        required: String,
        /// Available amount
        available: String,
        /// Currency code
        currency: String,
    },

    /// Invoice or payment request expired.
    InvoiceExpired {
        /// Invoice ID
        invoice_id: String,
        /// Expiration timestamp (unix epoch)
        expired_at: i64,
    },

    /// Payment was rejected by the recipient.
    PaymentRejected {
        /// Payment ID
        payment_id: String,
        /// Rejection reason
        reason: String,
    },

    /// Payment has already been completed.
    PaymentAlreadyCompleted {
        /// Payment ID
        payment_id: String,
    },

    /// Storage operation failed.
    Storage(String),

    /// Storage quota exceeded.
    QuotaExceeded {
        /// Current usage
        used: u64,
        /// Maximum allowed
        limit: u64,
    },

    /// Rate limited, should retry after delay.
    RateLimited {
        /// Suggested retry delay in milliseconds
        retry_after_ms: u64,
    },

    /// Internal/unexpected error.
    Internal(String),
}

impl PaykitError {
    /// Get the error code for FFI/mobile integration.
    pub fn code(&self) -> PaykitErrorCode {
        match self {
            Self::Unimplemented(_) => PaykitErrorCode::Unimplemented,
            Self::Transport(_) => PaykitErrorCode::Transport,
            Self::ConnectionFailed { .. } => PaykitErrorCode::ConnectionFailed,
            Self::ConnectionTimeout { .. } => PaykitErrorCode::ConnectionTimeout,
            Self::Auth(_) => PaykitErrorCode::Auth,
            Self::SessionExpired => PaykitErrorCode::SessionExpired,
            Self::InvalidCredentials(_) => PaykitErrorCode::InvalidCredentials,
            Self::NotFound { .. } => PaykitErrorCode::NotFound,
            Self::MethodNotSupported(_) => PaykitErrorCode::MethodNotSupported,
            Self::InvalidData { .. } => PaykitErrorCode::InvalidData,
            Self::ValidationFailed(_) => PaykitErrorCode::ValidationFailed,
            Self::Serialization(_) => PaykitErrorCode::Serialization,
            Self::Payment { .. } => PaykitErrorCode::Payment,
            Self::InsufficientFunds { .. } => PaykitErrorCode::InsufficientFunds,
            Self::InvoiceExpired { .. } => PaykitErrorCode::InvoiceExpired,
            Self::PaymentRejected { .. } => PaykitErrorCode::PaymentRejected,
            Self::PaymentAlreadyCompleted { .. } => PaykitErrorCode::PaymentAlreadyCompleted,
            Self::Storage(_) => PaykitErrorCode::Storage,
            Self::QuotaExceeded { .. } => PaykitErrorCode::QuotaExceeded,
            Self::RateLimited { .. } => PaykitErrorCode::RateLimited,
            Self::Internal(_) => PaykitErrorCode::Internal,
        }
    }

    /// Get the error message as an owned String (useful for FFI).
    pub fn message(&self) -> String {
        self.to_string()
    }

    /// Returns true if this error is potentially recoverable by retrying.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Transport(_)
                | Self::ConnectionFailed { .. }
                | Self::ConnectionTimeout { .. }
                | Self::RateLimited { .. }
                | Self::Storage(_)
        )
    }

    /// Returns a suggested retry delay in milliseconds, if applicable.
    pub fn retry_after_ms(&self) -> Option<u64> {
        match self {
            Self::RateLimited { retry_after_ms } => Some(*retry_after_ms),
            Self::ConnectionTimeout { .. } => Some(1000),
            Self::ConnectionFailed { .. } => Some(2000),
            Self::Transport(_) => Some(1000),
            Self::Storage(_) => Some(500),
            _ => None,
        }
    }

    /// Create a transport error from any error type.
    pub fn transport<E: std::error::Error>(err: E) -> Self {
        Self::Transport(err.to_string())
    }

    /// Create a not found error.
    pub fn not_found(resource_type: impl Into<String>, identifier: impl Into<String>) -> Self {
        Self::NotFound {
            resource_type: resource_type.into(),
            identifier: identifier.into(),
        }
    }

    /// Create an invalid data error.
    pub fn invalid_data(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidData {
            field: field.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for PaykitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unimplemented(label) => write!(f, "{} is not implemented yet", label),
            Self::Transport(msg) => write!(f, "transport error: {}", msg),
            Self::ConnectionFailed { target, reason } => {
                write!(f, "connection to {} failed: {}", target, reason)
            }
            Self::ConnectionTimeout {
                operation,
                timeout_ms,
            } => {
                write!(f, "{} timed out after {}ms", operation, timeout_ms)
            }
            Self::Auth(msg) => write!(f, "authentication error: {}", msg),
            Self::SessionExpired => write!(f, "session expired, please re-authenticate"),
            Self::InvalidCredentials(msg) => write!(f, "invalid credentials: {}", msg),
            Self::NotFound {
                resource_type,
                identifier,
            } => {
                write!(f, "{} not found: {}", resource_type, identifier)
            }
            Self::MethodNotSupported(method) => {
                write!(f, "payment method not supported: {}", method)
            }
            Self::InvalidData { field, reason } => {
                write!(f, "invalid {}: {}", field, reason)
            }
            Self::ValidationFailed(msg) => write!(f, "validation failed: {}", msg),
            Self::Serialization(msg) => write!(f, "serialization error: {}", msg),
            Self::Payment { payment_id, reason } => {
                if let Some(id) = payment_id {
                    write!(f, "payment {} failed: {}", id, reason)
                } else {
                    write!(f, "payment failed: {}", reason)
                }
            }
            Self::InsufficientFunds {
                required,
                available,
                currency,
            } => {
                write!(
                    f,
                    "insufficient funds: need {} {}, have {} {}",
                    required, currency, available, currency
                )
            }
            Self::InvoiceExpired {
                invoice_id,
                expired_at,
            } => {
                write!(
                    f,
                    "invoice {} expired at timestamp {}",
                    invoice_id, expired_at
                )
            }
            Self::PaymentRejected { payment_id, reason } => {
                write!(f, "payment {} rejected: {}", payment_id, reason)
            }
            Self::PaymentAlreadyCompleted { payment_id } => {
                write!(f, "payment {} already completed", payment_id)
            }
            Self::Storage(msg) => write!(f, "storage error: {}", msg),
            Self::QuotaExceeded { used, limit } => {
                write!(f, "quota exceeded: using {} of {} allowed", used, limit)
            }
            Self::RateLimited { retry_after_ms } => {
                write!(f, "rate limited, retry after {}ms", retry_after_ms)
            }
            Self::Internal(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl std::error::Error for PaykitError {}

impl From<serde_json::Error> for PaykitError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = PaykitError::RateLimited {
            retry_after_ms: 1000,
        };
        assert_eq!(err.code(), PaykitErrorCode::RateLimited);
        assert!(err.is_retryable());
        assert_eq!(err.retry_after_ms(), Some(1000));
    }

    #[test]
    fn test_error_display() {
        let err = PaykitError::InsufficientFunds {
            required: "1000".to_string(),
            available: "500".to_string(),
            currency: "SAT".to_string(),
        };
        assert!(err.to_string().contains("insufficient funds"));
        assert!(err.to_string().contains("SAT"));
    }

    #[test]
    fn test_helper_constructors() {
        let err = PaykitError::not_found("endpoint", "lightning");
        assert_eq!(err.code(), PaykitErrorCode::NotFound);

        let err = PaykitError::invalid_data("amount", "must be positive");
        assert_eq!(err.code(), PaykitErrorCode::InvalidData);
    }
}
