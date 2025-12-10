//! Rate limiting for Noise handshake attempts.
//!
//! This module provides a simple rate limiter to protect against DoS attacks
//! targeting the Noise handshake process.

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
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_attempts_per_ip: 10,
            window: Duration::from_secs(60),
            max_tracked_ips: 10_000,
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
        }
    }

    /// Strict rate limiting for high-security deployments.
    pub fn strict() -> Self {
        Self {
            max_attempts_per_ip: 3,
            window: Duration::from_secs(60),
            max_tracked_ips: 10_000,
        }
    }

    /// Relaxed rate limiting for development/testing.
    pub fn relaxed() -> Self {
        Self {
            max_attempts_per_ip: 100,
            window: Duration::from_secs(60),
            max_tracked_ips: 1_000,
        }
    }

    /// Disable rate limiting (use with caution).
    pub fn disabled() -> Self {
        Self {
            max_attempts_per_ip: u32::MAX,
            window: Duration::from_secs(1),
            max_tracked_ips: 1,
        }
    }
}

/// Tracks handshake attempts per IP address.
#[derive(Debug)]
struct IpRecord {
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
///     eprintln!("Rate limit exceeded for {}", ip);
/// }
/// ```
#[derive(Debug)]
pub struct HandshakeRateLimiter {
    config: RateLimitConfig,
    records: Mutex<HashMap<IpAddr, IpRecord>>,
}

impl HandshakeRateLimiter {
    /// Create a new rate limiter with the given configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            records: Mutex::new(HashMap::new()),
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
    pub fn check_and_record(&self, ip: IpAddr) -> bool {
        let mut records = self.records.lock().unwrap();
        let now = Instant::now();

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

    /// Check without recording (peek at current state).
    pub fn is_rate_limited(&self, ip: IpAddr) -> bool {
        let records = self.records.lock().unwrap();
        let now = Instant::now();

        if let Some(record) = records.get(&ip) {
            if now.duration_since(record.window_start) <= self.config.window {
                return record.count >= self.config.max_attempts_per_ip;
            }
        }
        false
    }

    /// Get remaining attempts for an IP.
    pub fn remaining_attempts(&self, ip: IpAddr) -> u32 {
        let records = self.records.lock().unwrap();
        let now = Instant::now();

        if let Some(record) = records.get(&ip) {
            if now.duration_since(record.window_start) <= self.config.window {
                return self.config.max_attempts_per_ip.saturating_sub(record.count);
            }
        }
        self.config.max_attempts_per_ip
    }

    /// Manually reset limits for an IP (e.g., after successful authentication).
    pub fn reset(&self, ip: IpAddr) {
        let mut records = self.records.lock().unwrap();
        records.remove(&ip);
    }

    /// Clear all tracked records.
    pub fn clear(&self) {
        let mut records = self.records.lock().unwrap();
        records.clear();
    }

    /// Get current number of tracked IPs.
    pub fn tracked_count(&self) -> usize {
        let records = self.records.lock().unwrap();
        records.len()
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
