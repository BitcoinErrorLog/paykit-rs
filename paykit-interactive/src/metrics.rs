//! Metrics collection for monitoring Paykit operations.
//!
//! This module provides a simple metrics interface for monitoring Paykit
//! deployments. It can be extended with Prometheus, StatsD, or other backends.
//!
//! # Example
//!
//! ```rust
//! use paykit_interactive::metrics::{Metrics, MetricsSnapshot};
//!
//! let metrics = Metrics::new();
//!
//! // Record events
//! metrics.record_handshake_attempt();
//! metrics.record_handshake_success();
//! metrics.record_message_sent(1024);
//!
//! // Get snapshot for monitoring
//! let snapshot = metrics.snapshot();
//! println!("Handshakes: {} attempts, {} successful", 
//!     snapshot.handshake_attempts, 
//!     snapshot.handshake_successes);
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Collected metrics for Paykit operations.
///
/// Thread-safe via atomic operations.
#[derive(Debug)]
pub struct Metrics {
    // Handshake metrics
    handshake_attempts: AtomicU64,
    handshake_successes: AtomicU64,
    handshake_failures: AtomicU64,
    handshake_rate_limited: AtomicU64,

    // Message metrics
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,

    // Connection metrics
    active_connections: AtomicU64,
    total_connections: AtomicU64,
    connection_rejections: AtomicU64,

    // Payment metrics
    payment_requests_sent: AtomicU64,
    payment_requests_received: AtomicU64,
    receipts_generated: AtomicU64,
    receipts_verified: AtomicU64,

    // Error metrics
    encryption_errors: AtomicU64,
    decryption_errors: AtomicU64,
    protocol_errors: AtomicU64,

    // Timing
    start_time: Instant,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    /// Create a new metrics collector.
    pub fn new() -> Self {
        Self {
            handshake_attempts: AtomicU64::new(0),
            handshake_successes: AtomicU64::new(0),
            handshake_failures: AtomicU64::new(0),
            handshake_rate_limited: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            total_connections: AtomicU64::new(0),
            connection_rejections: AtomicU64::new(0),
            payment_requests_sent: AtomicU64::new(0),
            payment_requests_received: AtomicU64::new(0),
            receipts_generated: AtomicU64::new(0),
            receipts_verified: AtomicU64::new(0),
            encryption_errors: AtomicU64::new(0),
            decryption_errors: AtomicU64::new(0),
            protocol_errors: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    // === Handshake Metrics ===

    /// Record a handshake attempt.
    pub fn record_handshake_attempt(&self) {
        self.handshake_attempts.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful handshake.
    pub fn record_handshake_success(&self) {
        self.handshake_successes.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed handshake.
    pub fn record_handshake_failure(&self) {
        self.handshake_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a rate-limited handshake attempt.
    pub fn record_handshake_rate_limited(&self) {
        self.handshake_rate_limited.fetch_add(1, Ordering::Relaxed);
    }

    // === Message Metrics ===

    /// Record a message sent.
    pub fn record_message_sent(&self, bytes: u64) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a message received.
    pub fn record_message_received(&self, bytes: u64) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    // === Connection Metrics ===

    /// Record a new connection.
    pub fn record_connection_opened(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.total_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a connection closed.
    pub fn record_connection_closed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a connection rejection (e.g., max connections reached).
    pub fn record_connection_rejected(&self) {
        self.connection_rejections.fetch_add(1, Ordering::Relaxed);
    }

    // === Payment Metrics ===

    /// Record a payment request sent.
    pub fn record_payment_request_sent(&self) {
        self.payment_requests_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a payment request received.
    pub fn record_payment_request_received(&self) {
        self.payment_requests_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a receipt generated.
    pub fn record_receipt_generated(&self) {
        self.receipts_generated.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a receipt verified.
    pub fn record_receipt_verified(&self) {
        self.receipts_verified.fetch_add(1, Ordering::Relaxed);
    }

    // === Error Metrics ===

    /// Record an encryption error.
    pub fn record_encryption_error(&self) {
        self.encryption_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a decryption error.
    pub fn record_decryption_error(&self) {
        self.decryption_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a protocol error.
    pub fn record_protocol_error(&self) {
        self.protocol_errors.fetch_add(1, Ordering::Relaxed);
    }

    // === Snapshot ===

    /// Get a snapshot of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            uptime_secs: self.start_time.elapsed().as_secs(),

            handshake_attempts: self.handshake_attempts.load(Ordering::Relaxed),
            handshake_successes: self.handshake_successes.load(Ordering::Relaxed),
            handshake_failures: self.handshake_failures.load(Ordering::Relaxed),
            handshake_rate_limited: self.handshake_rate_limited.load(Ordering::Relaxed),

            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),

            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_connections: self.total_connections.load(Ordering::Relaxed),
            connection_rejections: self.connection_rejections.load(Ordering::Relaxed),

            payment_requests_sent: self.payment_requests_sent.load(Ordering::Relaxed),
            payment_requests_received: self.payment_requests_received.load(Ordering::Relaxed),
            receipts_generated: self.receipts_generated.load(Ordering::Relaxed),
            receipts_verified: self.receipts_verified.load(Ordering::Relaxed),

            encryption_errors: self.encryption_errors.load(Ordering::Relaxed),
            decryption_errors: self.decryption_errors.load(Ordering::Relaxed),
            protocol_errors: self.protocol_errors.load(Ordering::Relaxed),
        }
    }

    /// Reset all counters to zero.
    pub fn reset(&self) {
        self.handshake_attempts.store(0, Ordering::Relaxed);
        self.handshake_successes.store(0, Ordering::Relaxed);
        self.handshake_failures.store(0, Ordering::Relaxed);
        self.handshake_rate_limited.store(0, Ordering::Relaxed);

        self.messages_sent.store(0, Ordering::Relaxed);
        self.messages_received.store(0, Ordering::Relaxed);
        self.bytes_sent.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);

        // Don't reset active_connections as it represents current state
        self.total_connections.store(0, Ordering::Relaxed);
        self.connection_rejections.store(0, Ordering::Relaxed);

        self.payment_requests_sent.store(0, Ordering::Relaxed);
        self.payment_requests_received.store(0, Ordering::Relaxed);
        self.receipts_generated.store(0, Ordering::Relaxed);
        self.receipts_verified.store(0, Ordering::Relaxed);

        self.encryption_errors.store(0, Ordering::Relaxed);
        self.decryption_errors.store(0, Ordering::Relaxed);
        self.protocol_errors.store(0, Ordering::Relaxed);
    }
}

/// A point-in-time snapshot of all metrics.
#[derive(Debug, Clone, Default)]
pub struct MetricsSnapshot {
    /// Uptime in seconds since metrics collector was created.
    pub uptime_secs: u64,

    // Handshake metrics
    pub handshake_attempts: u64,
    pub handshake_successes: u64,
    pub handshake_failures: u64,
    pub handshake_rate_limited: u64,

    // Message metrics
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,

    // Connection metrics
    pub active_connections: u64,
    pub total_connections: u64,
    pub connection_rejections: u64,

    // Payment metrics
    pub payment_requests_sent: u64,
    pub payment_requests_received: u64,
    pub receipts_generated: u64,
    pub receipts_verified: u64,

    // Error metrics
    pub encryption_errors: u64,
    pub decryption_errors: u64,
    pub protocol_errors: u64,
}

impl MetricsSnapshot {
    /// Calculate handshake success rate (0.0 to 1.0).
    pub fn handshake_success_rate(&self) -> f64 {
        if self.handshake_attempts == 0 {
            return 1.0;
        }
        self.handshake_successes as f64 / self.handshake_attempts as f64
    }

    /// Calculate total errors.
    pub fn total_errors(&self) -> u64 {
        self.encryption_errors + self.decryption_errors + self.protocol_errors
    }

    /// Format as Prometheus-style metrics.
    #[cfg(feature = "prometheus")]
    pub fn to_prometheus(&self) -> String {
        format!(
            r#"# HELP paykit_uptime_seconds Time since service started
# TYPE paykit_uptime_seconds gauge
paykit_uptime_seconds {}

# HELP paykit_handshake_attempts_total Total handshake attempts
# TYPE paykit_handshake_attempts_total counter
paykit_handshake_attempts_total {}

# HELP paykit_handshake_successes_total Successful handshakes
# TYPE paykit_handshake_successes_total counter
paykit_handshake_successes_total {}

# HELP paykit_handshake_failures_total Failed handshakes
# TYPE paykit_handshake_failures_total counter
paykit_handshake_failures_total {}

# HELP paykit_handshake_rate_limited_total Rate-limited handshake attempts
# TYPE paykit_handshake_rate_limited_total counter
paykit_handshake_rate_limited_total {}

# HELP paykit_messages_sent_total Messages sent
# TYPE paykit_messages_sent_total counter
paykit_messages_sent_total {}

# HELP paykit_messages_received_total Messages received
# TYPE paykit_messages_received_total counter
paykit_messages_received_total {}

# HELP paykit_bytes_sent_total Bytes sent
# TYPE paykit_bytes_sent_total counter
paykit_bytes_sent_total {}

# HELP paykit_bytes_received_total Bytes received
# TYPE paykit_bytes_received_total counter
paykit_bytes_received_total {}

# HELP paykit_active_connections Current active connections
# TYPE paykit_active_connections gauge
paykit_active_connections {}

# HELP paykit_total_connections_total Total connections since start
# TYPE paykit_total_connections_total counter
paykit_total_connections_total {}

# HELP paykit_connection_rejections_total Rejected connection attempts
# TYPE paykit_connection_rejections_total counter
paykit_connection_rejections_total {}

# HELP paykit_payment_requests_sent_total Payment requests sent
# TYPE paykit_payment_requests_sent_total counter
paykit_payment_requests_sent_total {}

# HELP paykit_payment_requests_received_total Payment requests received
# TYPE paykit_payment_requests_received_total counter
paykit_payment_requests_received_total {}

# HELP paykit_receipts_generated_total Receipts generated
# TYPE paykit_receipts_generated_total counter
paykit_receipts_generated_total {}

# HELP paykit_receipts_verified_total Receipts verified
# TYPE paykit_receipts_verified_total counter
paykit_receipts_verified_total {}

# HELP paykit_errors_total Total errors by type
# TYPE paykit_errors_total counter
paykit_errors_total{{type="encryption"}} {}
paykit_errors_total{{type="decryption"}} {}
paykit_errors_total{{type="protocol"}} {}
"#,
            self.uptime_secs,
            self.handshake_attempts,
            self.handshake_successes,
            self.handshake_failures,
            self.handshake_rate_limited,
            self.messages_sent,
            self.messages_received,
            self.bytes_sent,
            self.bytes_received,
            self.active_connections,
            self.total_connections,
            self.connection_rejections,
            self.payment_requests_sent,
            self.payment_requests_received,
            self.receipts_generated,
            self.receipts_verified,
            self.encryption_errors,
            self.decryption_errors,
            self.protocol_errors,
        )
    }

    /// Format as JSON for logging/monitoring.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

impl serde::Serialize for MetricsSnapshot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("MetricsSnapshot", 19)?;
        state.serialize_field("uptime_secs", &self.uptime_secs)?;
        state.serialize_field("handshake_attempts", &self.handshake_attempts)?;
        state.serialize_field("handshake_successes", &self.handshake_successes)?;
        state.serialize_field("handshake_failures", &self.handshake_failures)?;
        state.serialize_field("handshake_rate_limited", &self.handshake_rate_limited)?;
        state.serialize_field("messages_sent", &self.messages_sent)?;
        state.serialize_field("messages_received", &self.messages_received)?;
        state.serialize_field("bytes_sent", &self.bytes_sent)?;
        state.serialize_field("bytes_received", &self.bytes_received)?;
        state.serialize_field("active_connections", &self.active_connections)?;
        state.serialize_field("total_connections", &self.total_connections)?;
        state.serialize_field("connection_rejections", &self.connection_rejections)?;
        state.serialize_field("payment_requests_sent", &self.payment_requests_sent)?;
        state.serialize_field("payment_requests_received", &self.payment_requests_received)?;
        state.serialize_field("receipts_generated", &self.receipts_generated)?;
        state.serialize_field("receipts_verified", &self.receipts_verified)?;
        state.serialize_field("encryption_errors", &self.encryption_errors)?;
        state.serialize_field("decryption_errors", &self.decryption_errors)?;
        state.serialize_field("protocol_errors", &self.protocol_errors)?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let metrics = Metrics::new();

        metrics.record_handshake_attempt();
        metrics.record_handshake_attempt();
        metrics.record_handshake_success();
        metrics.record_message_sent(1024);
        metrics.record_message_received(512);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.handshake_attempts, 2);
        assert_eq!(snapshot.handshake_successes, 1);
        assert_eq!(snapshot.messages_sent, 1);
        assert_eq!(snapshot.bytes_sent, 1024);
        assert_eq!(snapshot.messages_received, 1);
        assert_eq!(snapshot.bytes_received, 512);
    }

    #[test]
    fn test_success_rate() {
        let snapshot = MetricsSnapshot {
            handshake_attempts: 100,
            handshake_successes: 95,
            ..Default::default()
        };
        assert!((snapshot.handshake_success_rate() - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_reset() {
        let metrics = Metrics::new();
        metrics.record_handshake_attempt();
        metrics.record_message_sent(100);

        metrics.reset();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.handshake_attempts, 0);
        assert_eq!(snapshot.messages_sent, 0);
    }
}

