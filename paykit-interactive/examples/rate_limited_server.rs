//! Rate-Limited Handshake Server Example
//!
//! This example demonstrates integrating the HandshakeRateLimiter with a
//! Noise protocol handshake server to protect against DoS attacks.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example rate_limited_server
//! ```
//!
//! # Key Concepts
//!
//! 1. **Pre-handshake rate limiting**: Check rate limits BEFORE accepting
//!    the cryptographic handshake to minimize resource consumption.
//!
//! 2. **Reset on success**: After successful authentication, reset the
//!    rate limit for that IP to allow normal operation.
//!
//! 3. **Fail-open design**: The rate limiter fails open on lock poisoning
//!    to prevent blocking legitimate traffic during edge cases.

use paykit_interactive::rate_limit::{HandshakeRateLimiter, RateLimitConfig};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

/// Simulated connection handler showing rate limiter integration pattern.
struct ConnectionHandler {
    rate_limiter: Arc<HandshakeRateLimiter>,
}

impl ConnectionHandler {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            rate_limiter: HandshakeRateLimiter::new_shared(config),
        }
    }

    /// Handle an incoming connection.
    ///
    /// Returns `Ok(true)` if connection was accepted and authenticated.
    /// Returns `Ok(false)` if connection was rate limited.
    /// Returns `Err` if handshake failed.
    async fn handle_connection(&self, addr: SocketAddr) -> Result<bool, String> {
        let ip = addr.ip();

        // Step 1: Check rate limit BEFORE doing any expensive work
        if !self.rate_limiter.check_and_record(ip) {
            println!(
                "[{}] Rate limited - {} remaining attempts",
                ip,
                self.rate_limiter.remaining_attempts(ip)
            );
            return Ok(false);
        }

        // Step 2: Perform the handshake (simulated)
        println!("[{}] Starting handshake...", ip);
        match self.perform_handshake(ip).await {
            Ok(()) => {
                // Step 3: Reset rate limit on successful auth
                self.rate_limiter.reset(ip);
                println!("[{}] Handshake successful, rate limit reset", ip);
                Ok(true)
            }
            Err(e) => {
                // Failed handshake - rate limit remains in effect
                println!(
                    "[{}] Handshake failed: {} ({} attempts remaining)",
                    ip,
                    e,
                    self.rate_limiter.remaining_attempts(ip)
                );
                Err(e)
            }
        }
    }

    /// Simulated handshake - in real code, this would be Noise XX pattern.
    async fn perform_handshake(&self, _ip: IpAddr) -> Result<(), String> {
        // Simulate some work
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Simulate 50% success rate for demo
        if rand::random::<bool>() {
            Ok(())
        } else {
            Err("Handshake verification failed".to_string())
        }
    }
}

/// Demonstration of different rate limit configurations.
fn demonstrate_configs() {
    println!("=== Rate Limit Configurations ===\n");

    // Default: Balanced for most use cases
    let default = RateLimitConfig::default();
    println!(
        "Default: {} attempts per {:?}, max {} IPs",
        default.max_attempts_per_ip, default.window, default.max_tracked_ips
    );

    // Strict: High-security environments
    let strict = RateLimitConfig::strict();
    println!(
        "Strict:  {} attempts per {:?}, max {} IPs",
        strict.max_attempts_per_ip, strict.window, strict.max_tracked_ips
    );

    // Relaxed: Development/testing
    let relaxed = RateLimitConfig::relaxed();
    println!(
        "Relaxed: {} attempts per {:?}, max {} IPs",
        relaxed.max_attempts_per_ip, relaxed.window, relaxed.max_tracked_ips
    );

    // Custom: Tune for your workload
    let custom = RateLimitConfig::new(5, 30, 50_000);
    println!(
        "Custom:  {} attempts per {:?}, max {} IPs",
        custom.max_attempts_per_ip, custom.window, custom.max_tracked_ips
    );

    println!();
}

/// Simulate multiple connection attempts from various IPs.
async fn simulate_connections(handler: &ConnectionHandler) {
    println!("=== Simulating Connections ===\n");

    // Simulate connections from multiple IPs
    let ips: Vec<SocketAddr> = vec![
        "192.168.1.1:12345".parse().unwrap(),
        "192.168.1.1:12346".parse().unwrap(), // Same IP, different port
        "192.168.1.1:12347".parse().unwrap(),
        "192.168.1.1:12348".parse().unwrap(), // Should be rate limited with strict config
        "192.168.1.2:12345".parse().unwrap(), // Different IP
        "192.168.1.2:12346".parse().unwrap(),
    ];

    for addr in ips {
        match handler.handle_connection(addr).await {
            Ok(true) => println!("  -> Connection accepted\n"),
            Ok(false) => println!("  -> Connection rejected (rate limited)\n"),
            Err(e) => println!("  -> Connection failed: {}\n", e),
        }
    }
}

/// Show rate limiter statistics.
fn show_stats(limiter: &HandshakeRateLimiter) {
    println!("=== Rate Limiter Statistics ===");
    println!("Tracked IPs: {}", limiter.tracked_count());

    // Check specific IPs
    let test_ip: IpAddr = "192.168.1.1".parse().unwrap();
    println!(
        "192.168.1.1: {} remaining attempts, rate_limited={}",
        limiter.remaining_attempts(test_ip),
        limiter.is_rate_limited(test_ip)
    );
}

#[tokio::main]
async fn main() {
    println!("Paykit Rate Limiter Integration Example\n");
    println!("This demonstrates protecting Noise handshakes from DoS attacks.\n");

    // Show available configurations
    demonstrate_configs();

    // Create handler with strict rate limiting for demo
    let handler = ConnectionHandler::new(RateLimitConfig::strict());

    // Simulate various connection scenarios
    simulate_connections(&handler).await;

    // Show final statistics
    show_stats(&handler.rate_limiter);

    println!("\n=== Example Complete ===");
    println!("\nKey Takeaways:");
    println!("1. Check rate limits BEFORE expensive handshake operations");
    println!("2. Reset limits after successful authentication");
    println!("3. Configure limits based on your security requirements");
    println!("4. Monitor tracked_count() for memory usage");
}
