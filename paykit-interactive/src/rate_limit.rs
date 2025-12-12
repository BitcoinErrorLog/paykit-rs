//! Rate limiting for Noise handshake attempts.
//!
//! This module provides a simple rate limiter to protect against DoS attacks
//! targeting the Noise handshake process.
//!
//! # Thread Safety
//!
//! The rate limiter uses `Mutex` for thread-safe access. Lock poisoning
//! is handled gracefully by failing open (allowing requests) rather than
//! panicking, to avoid blocking legitimate traffic.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Configuration for handshake rate limiting.
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maximum handshake attempts per IP within the time window.
    pub max_attempts_per_ip: u32,
    /// Time window for rate limiting (default: 60 seconds).
    pub window: Duration,
    /// Maximum tracked IPs to prevent memory exhaustion.
    pub max_tracked_ips: usize,
    /// Optional global rate limit across all IPs.
    ///
    /// When set, limits total requests across all IPs within the window.
    /// Useful for protecting against distributed attacks from many IPs.
    pub global_max_attempts: Option<u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_attempts_per_ip: 10,
            window: Duration::from_secs(60),
            max_tracked_ips: 10_000,
            global_max_attempts: None,
        }
    }
}

impl RateLimitConfig {
    /// Create a new config with custom values.
    pub fn new(max_attempts: u32, window_secs: u64, max_ips: usize) -> Self {
        Self {
            max_attempts_per_ip: max_attempts,
            window: Duration::from_secs(window_secs),
            max_tracked_ips: max_ips,
            global_max_attempts: None,
        }
    }

    /// Create a config with a global rate limit.
    ///
    /// # Arguments
    ///
    /// * `max_per_ip` - Maximum attempts per IP within the window
    /// * `global_max` - Maximum total attempts across all IPs within the window
    /// * `window_secs` - Time window in seconds
    /// * `max_ips` - Maximum tracked IPs
    ///
    /// # Example
    ///
    /// ```rust
    /// use paykit_interactive::rate_limit::RateLimitConfig;
    ///
    /// // Allow 5 per IP, 100 total per minute
    /// let config = RateLimitConfig::with_global_limit(5, 100, 60, 10_000);
    /// ```
    pub fn with_global_limit(
        max_per_ip: u32,
        global_max: u32,
        window_secs: u64,
        max_ips: usize,
    ) -> Self {
        Self {
            max_attempts_per_ip: max_per_ip,
            window: Duration::from_secs(window_secs),
            max_tracked_ips: max_ips,
            global_max_attempts: Some(global_max),
        }
    }

    /// Strict rate limiting for high-security deployments.
    pub fn strict() -> Self {
        Self {
            max_attempts_per_ip: 3,
            window: Duration::from_secs(60),
            max_tracked_ips: 10_000,
            global_max_attempts: None,
        }
    }

    /// Strict rate limiting with global limit for high-security deployments.
    ///
    /// Limits each IP to 3 attempts AND total to 50 attempts per minute.
    pub fn strict_with_global() -> Self {
        Self {
            max_attempts_per_ip: 3,
            window: Duration::from_secs(60),
            max_tracked_ips: 10_000,
            global_max_attempts: Some(50),
        }
    }

    /// Relaxed rate limiting for development/testing.
    pub fn relaxed() -> Self {
        Self {
            max_attempts_per_ip: 100,
            window: Duration::from_secs(60),
            max_tracked_ips: 1_000,
            global_max_attempts: None,
        }
    }

    /// Disable rate limiting (use with caution).
    pub fn disabled() -> Self {
        Self {
            max_attempts_per_ip: u32::MAX,
            window: Duration::from_secs(1),
            max_tracked_ips: 1,
            global_max_attempts: None,
        }
    }
}

/// Tracks handshake attempts per IP address.
#[derive(Debug)]
struct IpRecord {
    count: u32,
    window_start: Instant,
}

/// Global rate limit tracking across all IPs.
#[derive(Debug)]
struct GlobalRecord {
    count: u32,
    window_start: Instant,
}

/// Thread-safe rate limiter for handshake attempts.
///
/// # Example
///
/// ```rust
/// use paykit_interactive::rate_limit::{HandshakeRateLimiter, RateLimitConfig};
/// use std::net::IpAddr;
///
/// let limiter = HandshakeRateLimiter::new(RateLimitConfig::default());
///
/// // Check before accepting handshake
/// let ip: IpAddr = "192.168.1.1".parse().unwrap();
/// if !limiter.check_and_record(ip) {
///     // Reject connection - rate limit exceeded
/// }
/// ```
#[derive(Debug)]
pub struct HandshakeRateLimiter {
    config: RateLimitConfig,
    records: Mutex<HashMap<IpAddr, IpRecord>>,
    /// Global counter for total requests (optional)
    global: Mutex<GlobalRecord>,
}

impl HandshakeRateLimiter {
    /// Create a new rate limiter with the given configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            records: Mutex::new(HashMap::new()),
            global: Mutex::new(GlobalRecord {
                count: 0,
                window_start: Instant::now(),
            }),
        }
    }

    /// Create a rate limiter wrapped in an Arc for sharing across tasks.
    pub fn new_shared(config: RateLimitConfig) -> Arc<Self> {
        Arc::new(Self::new(config))
    }

    /// Check if a handshake attempt from this IP should be allowed.
    ///
    /// Returns `true` if allowed, `false` if rate limited.
    /// Also records the attempt for future checks.
    ///
    /// Checks both per-IP and global limits (if configured).
    ///
    /// If the lock is poisoned, this fails open (returns `true`) to avoid
    /// blocking legitimate traffic.
    pub fn check_and_record(&self, ip: IpAddr) -> bool {
        let now = Instant::now();

        // Check global limit first (if configured)
        if let Some(global_max) = self.config.global_max_attempts {
            if !self.check_global_limit(global_max, now) {
                return false;
            }
        }

        // Then check per-IP limit
        let mut records = match self.records.lock() {
            Ok(r) => r,
            Err(_) => return true, // Fail open on lock poisoning
        };

        // Clean up old entries if we're over capacity
        if records.len() >= self.config.max_tracked_ips {
            self.cleanup_expired(&mut records, now);
        }

        if let Some(record) = records.get_mut(&ip) {
            // Check if window has expired
            if now.duration_since(record.window_start) > self.config.window {
                // Reset window
                record.count = 1;
                record.window_start = now;
                true
            } else if record.count >= self.config.max_attempts_per_ip {
                // Rate limit exceeded
                false
            } else {
                // Allow and increment
                record.count += 1;
                true
            }
        } else {
            // First attempt from this IP
            records.insert(
                ip,
                IpRecord {
                    count: 1,
                    window_start: now,
                },
            );
            true
        }
    }

    /// Check and record global rate limit.
    ///
    /// Returns `true` if under global limit, `false` if exceeded.
    fn check_global_limit(&self, max: u32, now: Instant) -> bool {
        let mut global = match self.global.lock() {
            Ok(g) => g,
            Err(_) => return true, // Fail open on lock poisoning
        };

        // Check if window has expired
        if now.duration_since(global.window_start) > self.config.window {
            // Reset window
            global.count = 1;
            global.window_start = now;
            true
        } else if global.count >= max {
            // Global rate limit exceeded
            false
        } else {
            // Allow and increment
            global.count += 1;
            true
        }
    }

    /// Get the current global request count (for monitoring).
    ///
    /// Returns 0 if the lock is poisoned.
    pub fn global_count(&self) -> u32 {
        self.global.lock().map(|g| g.count).unwrap_or(0)
    }

    /// Check without recording (peek at current state).
    ///
    /// Returns `false` (not rate limited) if the lock is poisoned.
    pub fn is_rate_limited(&self, ip: IpAddr) -> bool {
        let records = match self.records.lock() {
            Ok(r) => r,
            Err(_) => return false, // Fail open on lock poisoning
        };
        let now = Instant::now();

        if let Some(record) = records.get(&ip) {
            if now.duration_since(record.window_start) <= self.config.window {
                return record.count >= self.config.max_attempts_per_ip;
            }
        }
        false
    }

    /// Get remaining attempts for an IP.
    ///
    /// Returns max attempts if the lock is poisoned.
    pub fn remaining_attempts(&self, ip: IpAddr) -> u32 {
        let records = match self.records.lock() {
            Ok(r) => r,
            Err(_) => return self.config.max_attempts_per_ip, // Fail open
        };
        let now = Instant::now();

        if let Some(record) = records.get(&ip) {
            if now.duration_since(record.window_start) <= self.config.window {
                return self.config.max_attempts_per_ip.saturating_sub(record.count);
            }
        }
        self.config.max_attempts_per_ip
    }

    /// Manually reset limits for an IP (e.g., after successful authentication).
    ///
    /// Silently ignored if the lock is poisoned.
    pub fn reset(&self, ip: IpAddr) {
        if let Ok(mut records) = self.records.lock() {
            records.remove(&ip);
        }
    }

    /// Clear all tracked records.
    ///
    /// Silently ignored if the lock is poisoned.
    pub fn clear(&self) {
        if let Ok(mut records) = self.records.lock() {
            records.clear();
        }
    }

    /// Get current number of tracked IPs.
    ///
    /// Returns 0 if the lock is poisoned.
    pub fn tracked_count(&self) -> usize {
        self.records.lock().map(|r| r.len()).unwrap_or(0)
    }

    /// Remove expired entries (called automatically when over capacity).
    fn cleanup_expired(&self, records: &mut HashMap<IpAddr, IpRecord>, now: Instant) {
        records.retain(|_, record| now.duration_since(record.window_start) <= self.config.window);
    }
}

impl Default for HandshakeRateLimiter {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let config = RateLimitConfig::new(3, 60, 100);
        let limiter = HandshakeRateLimiter::new(config);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        assert!(limiter.check_and_record(ip));
        assert!(limiter.check_and_record(ip));
        assert!(limiter.check_and_record(ip));
        // 4th attempt should be blocked
        assert!(!limiter.check_and_record(ip));
    }

    #[test]
    fn test_rate_limiter_different_ips() {
        let config = RateLimitConfig::new(2, 60, 100);
        let limiter = HandshakeRateLimiter::new(config);
        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();

        assert!(limiter.check_and_record(ip1));
        assert!(limiter.check_and_record(ip1));
        assert!(!limiter.check_and_record(ip1)); // ip1 blocked

        // ip2 should still be allowed
        assert!(limiter.check_and_record(ip2));
        assert!(limiter.check_and_record(ip2));
    }

    #[test]
    fn test_remaining_attempts() {
        let config = RateLimitConfig::new(5, 60, 100);
        let limiter = HandshakeRateLimiter::new(config);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        assert_eq!(limiter.remaining_attempts(ip), 5);
        limiter.check_and_record(ip);
        assert_eq!(limiter.remaining_attempts(ip), 4);
        limiter.check_and_record(ip);
        limiter.check_and_record(ip);
        assert_eq!(limiter.remaining_attempts(ip), 2);
    }

    #[test]
    fn test_reset() {
        let config = RateLimitConfig::new(2, 60, 100);
        let limiter = HandshakeRateLimiter::new(config);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        limiter.check_and_record(ip);
        limiter.check_and_record(ip);
        assert!(!limiter.check_and_record(ip)); // blocked

        limiter.reset(ip);
        assert!(limiter.check_and_record(ip)); // allowed again
    }

    #[test]
    fn test_disabled_config() {
        let config = RateLimitConfig::disabled();
        let limiter = HandshakeRateLimiter::new(config);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // Should essentially never block
        for _ in 0..1000 {
            assert!(limiter.check_and_record(ip));
        }
    }
}
