//! Security hardening utilities for Paykit.
//!
//! This module provides security-related utilities for:
//! - Session expiry tracking and proactive renewal
//! - Rate limiting for Noise handshakes and API calls
//! - Re-authentication flow handling for 401/403 responses
//!
//! # SECURITY_ARCHITECTURE.md "Next Release" Items
//!
//! This module addresses the following items from SECURITY_ARCHITECTURE.md:
//!
//! 1. **Session Expiry Tracking** - `SessionExpiryTracker` and `SessionState`
//! 2. **Rate Limiting** - `RateLimiter` and `NoiseHandshakeRateLimiter`
//! 3. **401/403 Re-auth Flow** - `ReauthRequired` error type and `ReauthHandler` trait
//!
//! Memory zeroization is handled via the `zeroize` crate at the type level.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Default session expiry warning threshold (7 days before expiry).
pub const DEFAULT_EXPIRY_WARNING_DAYS: u64 = 7;

/// Default maximum handshake attempts per minute.
pub const DEFAULT_MAX_HANDSHAKES_PER_MINUTE: u32 = 10;

/// Default maximum API calls per minute.
pub const DEFAULT_MAX_API_CALLS_PER_MINUTE: u32 = 60;

// ============================================================================
// Session Expiry Tracking
// ============================================================================

/// Session expiry state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Session is valid and not close to expiry.
    Valid,
    /// Session is valid but needs proactive renewal soon.
    NeedsRenewal,
    /// Session has expired.
    Expired,
    /// Session has no expiry (never expires until revoked).
    NoExpiry,
}

/// Tracks session expiry and provides renewal hints.
#[derive(Debug, Clone)]
pub struct SessionExpiryTracker {
    /// Session creation timestamp (Unix seconds).
    pub created_at: u64,
    /// Session expiry timestamp (Unix seconds), if any.
    pub expires_at: Option<u64>,
    /// Threshold for triggering proactive renewal (seconds before expiry).
    pub renewal_threshold_secs: u64,
}

impl SessionExpiryTracker {
    /// Create a new tracker with default renewal threshold (7 days).
    pub fn new(created_at: u64, expires_at: Option<u64>) -> Self {
        Self {
            created_at,
            expires_at,
            renewal_threshold_secs: DEFAULT_EXPIRY_WARNING_DAYS * 24 * 3600,
        }
    }

    /// Create a tracker with custom renewal threshold.
    pub fn with_renewal_threshold(mut self, threshold_secs: u64) -> Self {
        self.renewal_threshold_secs = threshold_secs;
        self
    }

    /// Check the current session state.
    pub fn state(&self) -> SessionState {
        let now = current_unix_timestamp();

        match self.expires_at {
            None => SessionState::NoExpiry,
            Some(expires) => {
                if now >= expires {
                    SessionState::Expired
                } else if now + self.renewal_threshold_secs >= expires {
                    SessionState::NeedsRenewal
                } else {
                    SessionState::Valid
                }
            }
        }
    }

    /// Check if the session is expired.
    pub fn is_expired(&self) -> bool {
        self.state() == SessionState::Expired
    }

    /// Check if the session needs proactive renewal.
    pub fn needs_renewal(&self) -> bool {
        self.state() == SessionState::NeedsRenewal
    }

    /// Get the remaining time until expiry (if any).
    pub fn time_until_expiry(&self) -> Option<Duration> {
        let now = current_unix_timestamp();
        self.expires_at.and_then(|expires| {
            if now < expires {
                Some(Duration::from_secs(expires - now))
            } else {
                None
            }
        })
    }

    /// Get the age of the session.
    pub fn age(&self) -> Duration {
        let now = current_unix_timestamp();
        if now > self.created_at {
            Duration::from_secs(now - self.created_at)
        } else {
            Duration::ZERO
        }
    }
}

// ============================================================================
// Rate Limiting
// ============================================================================

/// Generic rate limiter using a sliding window.
#[derive(Debug)]
pub struct RateLimiter {
    /// Maximum attempts allowed per window.
    max_attempts: u32,
    /// Window duration.
    window: Duration,
    /// Tracking data: key -> list of attempt timestamps.
    attempts: HashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new(max_attempts: u32, window: Duration) -> Self {
        Self {
            max_attempts,
            window,
            attempts: HashMap::new(),
        }
    }

    /// Check if an action is allowed for the given key.
    ///
    /// Returns `Ok(())` if allowed, `Err(remaining_time)` if rate limited.
    pub fn check(&mut self, key: &str) -> Result<(), Duration> {
        let now = Instant::now();
        let cutoff = now - self.window;

        // Remove expired attempts
        if let Some(attempts) = self.attempts.get_mut(key) {
            attempts.retain(|&t| t > cutoff);
        }

        let count = self.attempts.get(key).map(|v| v.len()).unwrap_or(0) as u32;

        if count >= self.max_attempts {
            // Calculate time until oldest attempt expires
            if let Some(attempts) = self.attempts.get(key) {
                if let Some(&oldest) = attempts.first() {
                    let wait_time = oldest + self.window - now;
                    return Err(wait_time);
                }
            }
            return Err(self.window);
        }

        // Record attempt
        self.attempts.entry(key.to_string()).or_default().push(now);
        Ok(())
    }

    /// Reset rate limiting for a specific key.
    pub fn reset(&mut self, key: &str) {
        self.attempts.remove(key);
    }

    /// Reset all rate limiting.
    pub fn reset_all(&mut self) {
        self.attempts.clear();
    }

    /// Get the current attempt count for a key.
    pub fn current_count(&self, key: &str) -> u32 {
        self.attempts.get(key).map(|v| v.len()).unwrap_or(0) as u32
    }
}

/// Specialized rate limiter for Noise handshake attempts.
///
/// Per SECURITY_ARCHITECTURE.md, this limits handshake attempts per recipient.
#[derive(Debug)]
pub struct NoiseHandshakeRateLimiter {
    inner: RateLimiter,
}

impl NoiseHandshakeRateLimiter {
    /// Create a new handshake rate limiter with default settings (10/min).
    pub fn new() -> Self {
        Self {
            inner: RateLimiter::new(
                DEFAULT_MAX_HANDSHAKES_PER_MINUTE,
                Duration::from_secs(60),
            ),
        }
    }

    /// Create with custom limits.
    pub fn with_limits(max_per_minute: u32) -> Self {
        Self {
            inner: RateLimiter::new(max_per_minute, Duration::from_secs(60)),
        }
    }

    /// Check if a handshake to the given recipient is allowed.
    ///
    /// # Arguments
    ///
    /// * `recipient_pubkey` - The recipient's public key (any format).
    ///
    /// # Returns
    ///
    /// `Ok(())` if allowed, `Err(wait_time)` if rate limited.
    pub fn check_handshake(&mut self, recipient_pubkey: &str) -> Result<(), Duration> {
        self.inner.check(recipient_pubkey)
    }

    /// Reset rate limiting for a specific recipient.
    pub fn reset(&mut self, recipient_pubkey: &str) {
        self.inner.reset(recipient_pubkey);
    }
}

impl Default for NoiseHandshakeRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Re-authentication Flow
// ============================================================================

/// Reason for requiring re-authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReauthReason {
    /// HTTP 401 Unauthorized response.
    Unauthorized,
    /// HTTP 403 Forbidden response.
    Forbidden,
    /// Session has expired.
    SessionExpired,
    /// Session needs proactive renewal.
    SessionNeedsRenewal,
    /// User requested re-authentication.
    UserRequested,
}

impl std::fmt::Display for ReauthReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReauthReason::Unauthorized => write!(f, "unauthorized (401)"),
            ReauthReason::Forbidden => write!(f, "forbidden (403)"),
            ReauthReason::SessionExpired => write!(f, "session expired"),
            ReauthReason::SessionNeedsRenewal => write!(f, "session needs renewal"),
            ReauthReason::UserRequested => write!(f, "user requested"),
        }
    }
}

/// Result of checking whether re-authentication is required.
#[derive(Debug, Clone)]
pub struct ReauthCheck {
    /// Whether re-authentication is required.
    pub required: bool,
    /// Reason for re-authentication (if required).
    pub reason: Option<ReauthReason>,
    /// Original error message (if from HTTP response).
    pub error_message: Option<String>,
}

impl ReauthCheck {
    /// Create a check result indicating no re-auth needed.
    pub fn not_required() -> Self {
        Self {
            required: false,
            reason: None,
            error_message: None,
        }
    }

    /// Create a check result indicating re-auth is required.
    pub fn required(reason: ReauthReason) -> Self {
        Self {
            required: true,
            reason: Some(reason),
            error_message: None,
        }
    }

    /// Create a check result from an HTTP status code.
    pub fn from_http_status(status: u16, message: Option<String>) -> Self {
        match status {
            401 => Self {
                required: true,
                reason: Some(ReauthReason::Unauthorized),
                error_message: message,
            },
            403 => Self {
                required: true,
                reason: Some(ReauthReason::Forbidden),
                error_message: message,
            },
            _ => Self::not_required(),
        }
    }
}

/// Handler trait for re-authentication flows.
///
/// Implement this trait to integrate with platform-specific auth flows.
#[allow(async_fn_in_trait)]
pub trait ReauthHandler: Send + Sync {
    /// Request re-authentication from the user/system.
    ///
    /// This should trigger the appropriate flow (e.g., Ring callback, biometric prompt).
    async fn request_reauth(&self, reason: ReauthReason) -> Result<(), crate::PaykitError>;

    /// Called when re-authentication succeeds.
    async fn on_reauth_success(&self);

    /// Called when re-authentication fails.
    async fn on_reauth_failure(&self, error: &crate::PaykitError);
}

// ============================================================================
// Helpers
// ============================================================================

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn session_expiry_valid() {
        let now = current_unix_timestamp();
        let tracker = SessionExpiryTracker::new(now, Some(now + 30 * 24 * 3600)); // 30 days

        assert_eq!(tracker.state(), SessionState::Valid);
        assert!(!tracker.is_expired());
        assert!(!tracker.needs_renewal());
    }

    #[test]
    fn session_expiry_needs_renewal() {
        let now = current_unix_timestamp();
        // Expires in 5 days (< 7 day threshold)
        let tracker = SessionExpiryTracker::new(now - 1000, Some(now + 5 * 24 * 3600));

        assert_eq!(tracker.state(), SessionState::NeedsRenewal);
        assert!(!tracker.is_expired());
        assert!(tracker.needs_renewal());
    }

    #[test]
    fn session_expiry_expired() {
        let now = current_unix_timestamp();
        let tracker = SessionExpiryTracker::new(now - 1000, Some(now - 100)); // Already expired

        assert_eq!(tracker.state(), SessionState::Expired);
        assert!(tracker.is_expired());
    }

    #[test]
    fn session_no_expiry() {
        let now = current_unix_timestamp();
        let tracker = SessionExpiryTracker::new(now, None);

        assert_eq!(tracker.state(), SessionState::NoExpiry);
        assert!(!tracker.is_expired());
        assert!(!tracker.needs_renewal());
    }

    #[test]
    fn rate_limiter_allows_under_limit() {
        let mut limiter = RateLimiter::new(5, Duration::from_secs(60));

        for _ in 0..5 {
            assert!(limiter.check("test").is_ok());
        }
    }

    #[test]
    fn rate_limiter_blocks_over_limit() {
        let mut limiter = RateLimiter::new(2, Duration::from_secs(60));

        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_err());
    }

    #[test]
    fn rate_limiter_per_key_isolation() {
        let mut limiter = RateLimiter::new(1, Duration::from_secs(60));

        assert!(limiter.check("key1").is_ok());
        assert!(limiter.check("key1").is_err());
        assert!(limiter.check("key2").is_ok()); // Different key, should work
    }

    #[test]
    fn rate_limiter_reset() {
        let mut limiter = RateLimiter::new(1, Duration::from_secs(60));

        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_err());

        limiter.reset("test");
        assert!(limiter.check("test").is_ok());
    }

    #[test]
    fn rate_limiter_sliding_window() {
        let mut limiter = RateLimiter::new(2, Duration::from_millis(100));

        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_ok());
        assert!(limiter.check("test").is_err());

        // Wait for window to slide
        sleep(Duration::from_millis(150));

        // Should be allowed again
        assert!(limiter.check("test").is_ok());
    }

    #[test]
    fn handshake_rate_limiter() {
        let mut limiter = NoiseHandshakeRateLimiter::with_limits(2);

        assert!(limiter.check_handshake("pubkey1").is_ok());
        assert!(limiter.check_handshake("pubkey1").is_ok());
        assert!(limiter.check_handshake("pubkey1").is_err());

        // Different recipient should work
        assert!(limiter.check_handshake("pubkey2").is_ok());
    }

    #[test]
    fn reauth_check_from_http_status() {
        let check_401 = ReauthCheck::from_http_status(401, Some("Unauthorized".into()));
        assert!(check_401.required);
        assert_eq!(check_401.reason, Some(ReauthReason::Unauthorized));

        let check_403 = ReauthCheck::from_http_status(403, None);
        assert!(check_403.required);
        assert_eq!(check_403.reason, Some(ReauthReason::Forbidden));

        let check_200 = ReauthCheck::from_http_status(200, None);
        assert!(!check_200.required);
    }

    #[test]
    fn reauth_reason_display() {
        assert_eq!(ReauthReason::Unauthorized.to_string(), "unauthorized (401)");
        assert_eq!(ReauthReason::Forbidden.to_string(), "forbidden (403)");
        assert_eq!(ReauthReason::SessionExpired.to_string(), "session expired");
    }
}
