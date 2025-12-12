//! Connection limiting for DoS protection.
//!
//! This module provides connection limits to protect against resource exhaustion
//! attacks targeting the Paykit server.
//!
//! # Example
//!
//! ```rust
//! use paykit_interactive::connection_limit::{ConnectionLimiter, ConnectionLimitConfig};
//! use std::net::IpAddr;
//!
//! let config = ConnectionLimitConfig::default();
//! let limiter = ConnectionLimiter::new(config);
//!
//! let ip: IpAddr = "192.168.1.1".parse().unwrap();
//!
//! // Try to acquire a connection slot
//! if let Some(guard) = limiter.try_acquire(ip) {
//!     // Connection allowed - guard will release slot when dropped
//!     println!("Connection accepted");
//! } else {
//!     // Connection rejected - limit reached
//!     println!("Connection rejected: limit exceeded");
//! }
//! ```

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Configuration for connection limiting.
#[derive(Clone, Debug)]
pub struct ConnectionLimitConfig {
    /// Maximum total concurrent connections.
    pub max_total_connections: u64,
    /// Maximum connections per IP address.
    pub max_per_ip: u64,
    /// Maximum connections per /24 subnet (IPv4) or /64 subnet (IPv6).
    pub max_per_subnet: u64,
}

impl Default for ConnectionLimitConfig {
    fn default() -> Self {
        Self {
            max_total_connections: 1000,
            max_per_ip: 10,
            max_per_subnet: 50,
        }
    }
}

impl ConnectionLimitConfig {
    /// Create a strict config for high-security deployments.
    pub fn strict() -> Self {
        Self {
            max_total_connections: 500,
            max_per_ip: 5,
            max_per_subnet: 25,
        }
    }

    /// Create a relaxed config for development/testing.
    pub fn relaxed() -> Self {
        Self {
            max_total_connections: 10000,
            max_per_ip: 100,
            max_per_subnet: 500,
        }
    }
}

/// RAII guard that releases the connection slot when dropped.
pub struct ConnectionGuard {
    limiter: Arc<ConnectionLimiterInner>,
    ip: IpAddr,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.limiter.release(self.ip);
    }
}

/// Internal state for the connection limiter.
struct ConnectionLimiterInner {
    config: ConnectionLimitConfig,
    total_connections: AtomicU64,
    per_ip_connections: Mutex<HashMap<IpAddr, u64>>,
    per_subnet_connections: Mutex<HashMap<String, u64>>,
}

impl ConnectionLimiterInner {
    fn release(&self, ip: IpAddr) {
        self.total_connections.fetch_sub(1, Ordering::Relaxed);

        if let Ok(mut per_ip) = self.per_ip_connections.lock() {
            if let Some(count) = per_ip.get_mut(&ip) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    per_ip.remove(&ip);
                }
            }
        }

        let subnet = ip_to_subnet(&ip);
        if let Ok(mut per_subnet) = self.per_subnet_connections.lock() {
            if let Some(count) = per_subnet.get_mut(&subnet) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    per_subnet.remove(&subnet);
                }
            }
        }
    }
}

/// Connection limiter for DoS protection.
pub struct ConnectionLimiter {
    inner: Arc<ConnectionLimiterInner>,
}

impl ConnectionLimiter {
    /// Create a new connection limiter with the given configuration.
    pub fn new(config: ConnectionLimitConfig) -> Self {
        Self {
            inner: Arc::new(ConnectionLimiterInner {
                config,
                total_connections: AtomicU64::new(0),
                per_ip_connections: Mutex::new(HashMap::new()),
                per_subnet_connections: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Create a connection limiter wrapped in an Arc for sharing.
    pub fn new_shared(config: ConnectionLimitConfig) -> Arc<Self> {
        Arc::new(Self::new(config))
    }

    /// Try to acquire a connection slot.
    ///
    /// Returns `Some(ConnectionGuard)` if successful, `None` if limit reached.
    /// The guard will automatically release the slot when dropped.
    pub fn try_acquire(&self, ip: IpAddr) -> Option<ConnectionGuard> {
        // Check total connections
        let current_total = self.inner.total_connections.load(Ordering::Relaxed);
        if current_total >= self.inner.config.max_total_connections {
            return None;
        }

        // Check per-IP limit
        {
            let per_ip = self.inner.per_ip_connections.lock().ok()?;
            let current_ip = per_ip.get(&ip).copied().unwrap_or(0);
            if current_ip >= self.inner.config.max_per_ip {
                return None;
            }
        }

        // Check per-subnet limit
        let subnet = ip_to_subnet(&ip);
        {
            let per_subnet = self.inner.per_subnet_connections.lock().ok()?;
            let current_subnet = per_subnet.get(&subnet).copied().unwrap_or(0);
            if current_subnet >= self.inner.config.max_per_subnet {
                return None;
            }
        }

        // Acquire the connection
        self.inner.total_connections.fetch_add(1, Ordering::Relaxed);

        {
            let mut per_ip = self.inner.per_ip_connections.lock().ok()?;
            *per_ip.entry(ip).or_insert(0) += 1;
        }

        {
            let mut per_subnet = self.inner.per_subnet_connections.lock().ok()?;
            *per_subnet.entry(subnet).or_insert(0) += 1;
        }

        Some(ConnectionGuard {
            limiter: self.inner.clone(),
            ip,
        })
    }

    /// Get current total connection count.
    pub fn total_connections(&self) -> u64 {
        self.inner.total_connections.load(Ordering::Relaxed)
    }

    /// Get current connection count for an IP.
    pub fn connections_for_ip(&self, ip: IpAddr) -> u64 {
        self.inner
            .per_ip_connections
            .lock()
            .ok()
            .and_then(|m| m.get(&ip).copied())
            .unwrap_or(0)
    }

    /// Get current connection count for a subnet.
    pub fn connections_for_subnet(&self, ip: IpAddr) -> u64 {
        let subnet = ip_to_subnet(&ip);
        self.inner
            .per_subnet_connections
            .lock()
            .ok()
            .and_then(|m| m.get(&subnet).copied())
            .unwrap_or(0)
    }

    /// Check if connections from this IP would be allowed (without acquiring).
    pub fn would_allow(&self, ip: IpAddr) -> bool {
        let current_total = self.inner.total_connections.load(Ordering::Relaxed);
        if current_total >= self.inner.config.max_total_connections {
            return false;
        }

        if let Ok(per_ip) = self.inner.per_ip_connections.lock() {
            let current_ip = per_ip.get(&ip).copied().unwrap_or(0);
            if current_ip >= self.inner.config.max_per_ip {
                return false;
            }
        }

        let subnet = ip_to_subnet(&ip);
        if let Ok(per_subnet) = self.inner.per_subnet_connections.lock() {
            let current_subnet = per_subnet.get(&subnet).copied().unwrap_or(0);
            if current_subnet >= self.inner.config.max_per_subnet {
                return false;
            }
        }

        true
    }
}

impl Default for ConnectionLimiter {
    fn default() -> Self {
        Self::new(ConnectionLimitConfig::default())
    }
}

/// Convert an IP address to its subnet representation.
///
/// For IPv4: /24 subnet (e.g., 192.168.1.0/24)
/// For IPv6: /64 subnet
fn ip_to_subnet(ip: &IpAddr) -> String {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            format!("{}.{}.{}.0/24", octets[0], octets[1], octets[2])
        }
        IpAddr::V6(ipv6) => {
            let segments = ipv6.segments();
            format!(
                "{:x}:{:x}:{:x}:{:x}::/64",
                segments[0], segments[1], segments[2], segments[3]
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_limit_basic() {
        let config = ConnectionLimitConfig {
            max_total_connections: 5,
            max_per_ip: 2,
            max_per_subnet: 3,
        };
        let limiter = ConnectionLimiter::new(config);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Should allow 2 connections from same IP
        let guard1 = limiter.try_acquire(ip);
        assert!(guard1.is_some());
        let guard2 = limiter.try_acquire(ip);
        assert!(guard2.is_some());

        // Third should be blocked (per-IP limit)
        assert!(limiter.try_acquire(ip).is_none());

        // Different IP in same subnet should work
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();
        let guard3 = limiter.try_acquire(ip2);
        assert!(guard3.is_some());

        assert_eq!(limiter.total_connections(), 3);
    }

    #[test]
    fn test_connection_guard_release() {
        let config = ConnectionLimitConfig {
            max_total_connections: 2,
            max_per_ip: 2,
            max_per_subnet: 10,
        };
        let limiter = ConnectionLimiter::new(config);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        {
            let _guard1 = limiter.try_acquire(ip).unwrap();
            let _guard2 = limiter.try_acquire(ip).unwrap();
            assert_eq!(limiter.total_connections(), 2);
            // Guards dropped here
        }

        // Should be able to acquire again after guards dropped
        assert_eq!(limiter.total_connections(), 0);
        assert!(limiter.try_acquire(ip).is_some());
    }

    #[test]
    fn test_subnet_limit() {
        let config = ConnectionLimitConfig {
            max_total_connections: 100,
            max_per_ip: 10,
            max_per_subnet: 2,
        };
        let limiter = ConnectionLimiter::new(config);

        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();
        let ip3: IpAddr = "192.168.1.3".parse().unwrap();

        let _guard1 = limiter.try_acquire(ip1).unwrap();
        let _guard2 = limiter.try_acquire(ip2).unwrap();

        // Third IP in same /24 subnet should be blocked
        assert!(limiter.try_acquire(ip3).is_none());

        // Different subnet should work
        let ip4: IpAddr = "192.168.2.1".parse().unwrap();
        assert!(limiter.try_acquire(ip4).is_some());
    }
}

