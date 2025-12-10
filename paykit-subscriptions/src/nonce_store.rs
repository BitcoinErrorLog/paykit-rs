//! Nonce tracking for replay attack prevention
//!
//! This module provides a `NonceStore` that tracks used nonces to prevent
//! signature replay attacks.
//!
//! # Security
//!
//! - Each nonce can only be used once
//! - Expired nonces are periodically cleaned up
//! - Thread-safe (uses RwLock)

use crate::Result;
use std::collections::HashMap;
use std::sync::RwLock;

/// Store for tracking used nonces to prevent replay attacks
///
/// # Security
///
/// - Tracks nonces with their expiration times
/// - Prevents reuse of nonces (replay attack prevention)
/// - Automatically cleans up expired nonces
/// - Thread-safe with RwLock
pub struct NonceStore {
    // Maps nonce -> expiration timestamp
    used_nonces: RwLock<HashMap<[u8; 32], i64>>,
}

impl NonceStore {
    /// Create a new empty nonce store
    pub fn new() -> Self {
        Self {
            used_nonces: RwLock::new(HashMap::new()),
        }
    }

    /// Check if a nonce has been used, and mark it as used if not
    ///
    /// # Security
    ///
    /// This is the critical function for replay attack prevention.
    /// Returns `Ok(true)` if nonce is fresh (never seen before).
    /// Returns `Ok(false)` if nonce has been used (potential replay attack).
    ///
    /// # Arguments
    ///
    /// * `nonce` - The nonce to check
    /// * `expires_at` - When this nonce's signature expires
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use paykit_subscriptions::NonceStore;
    /// # fn example() -> anyhow::Result<()> {
    /// let store = NonceStore::new();
    /// let nonce = [42u8; 32];
    /// let expires_at = chrono::Utc::now().timestamp() + 3600;
    ///
    /// // First use - should succeed
    /// assert!(store.check_and_mark(&nonce, expires_at)?);
    ///
    /// // Second use - should fail (replay attack)
    /// assert!(!store.check_and_mark(&nonce, expires_at)?);
    /// # Ok(())
    /// # }
    /// ```
    pub fn check_and_mark(&self, nonce: &[u8; 32], expires_at: i64) -> Result<bool> {
        let mut nonces = self
            .used_nonces
            .write()
            .map_err(|e| crate::SubscriptionError::Other(format!("Lock poisoned: {}", e)))?;

        // Check if nonce already exists
        if nonces.contains_key(nonce) {
            // Replay attack detected
            return Ok(false);
        }

        // Mark nonce as used with expiration time
        nonces.insert(*nonce, expires_at);
        Ok(true)
    }

    /// Clean up expired nonces to prevent unbounded memory growth
    ///
    /// This should be called periodically (e.g., hourly) to remove
    /// nonces from expired signatures.
    ///
    /// # Arguments
    ///
    /// * `before` - Remove nonces that expired before this timestamp
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use paykit_subscriptions::NonceStore;
    /// # fn example() -> anyhow::Result<()> {
    /// let store = NonceStore::new();
    /// let now = chrono::Utc::now().timestamp();
    ///
    /// // Clean up nonces from signatures that expired before now
    /// store.cleanup_expired(now)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn cleanup_expired(&self, before: i64) -> Result<()> {
        let mut nonces = self
            .used_nonces
            .write()
            .map_err(|e| crate::SubscriptionError::Other(format!("Lock poisoned: {}", e)))?;

        // Remove all nonces with expiration time before the threshold
        nonces.retain(|_, expires_at| *expires_at >= before);

        Ok(())
    }

    /// Get the count of tracked nonces (for monitoring/debugging)
    pub fn count(&self) -> usize {
        let nonces = self.used_nonces.read().expect("NonceStore lock poisoned");
        nonces.len()
    }

    /// Check if a nonce has been used (read-only, doesn't mark)
    ///
    /// This is useful for testing or validation without modifying state.
    pub fn has_nonce(&self, nonce: &[u8; 32]) -> bool {
        let nonces = self.used_nonces.read().expect("NonceStore lock poisoned");
        nonces.contains_key(nonce)
    }
}

impl Default for NonceStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_fresh_nonce_accepted() {
        let store = NonceStore::new();
        let nonce = [42u8; 32];
        let expires_at = Utc::now().timestamp() + 3600;

        let result = store.check_and_mark(&nonce, expires_at).unwrap();
        assert!(result, "Fresh nonce should be accepted");
    }

    #[test]
    fn test_duplicate_nonce_rejected() {
        let store = NonceStore::new();
        let nonce = [42u8; 32];
        let expires_at = Utc::now().timestamp() + 3600;

        // First use - should succeed
        let first = store.check_and_mark(&nonce, expires_at).unwrap();
        assert!(first, "First use should succeed");

        // Second use - should fail
        let second = store.check_and_mark(&nonce, expires_at).unwrap();
        assert!(!second, "Duplicate nonce should be rejected");
    }

    #[test]
    fn test_different_nonces_both_accepted() {
        let store = NonceStore::new();
        let nonce1 = [1u8; 32];
        let nonce2 = [2u8; 32];
        let expires_at = Utc::now().timestamp() + 3600;

        let first = store.check_and_mark(&nonce1, expires_at).unwrap();
        let second = store.check_and_mark(&nonce2, expires_at).unwrap();

        assert!(first, "First nonce should be accepted");
        assert!(second, "Second nonce should be accepted");
    }

    #[test]
    fn test_cleanup_expired() {
        let store = NonceStore::new();
        let now = Utc::now().timestamp();

        // Add nonces with different expiration times
        let old_nonce = [1u8; 32];
        let recent_nonce = [2u8; 32];

        store.check_and_mark(&old_nonce, now - 1000).unwrap(); // Expired
        store.check_and_mark(&recent_nonce, now + 1000).unwrap(); // Valid

        assert_eq!(store.count(), 2, "Should have 2 nonces");

        // Clean up expired nonces
        store.cleanup_expired(now).unwrap();

        assert_eq!(store.count(), 1, "Should have 1 nonce after cleanup");
        assert!(store.has_nonce(&recent_nonce), "Recent nonce should remain");
        assert!(!store.has_nonce(&old_nonce), "Old nonce should be removed");
    }

    #[test]
    fn test_count() {
        let store = NonceStore::new();
        let expires_at = Utc::now().timestamp() + 3600;

        assert_eq!(store.count(), 0, "Should start empty");

        store.check_and_mark(&[1u8; 32], expires_at).unwrap();
        assert_eq!(store.count(), 1);

        store.check_and_mark(&[2u8; 32], expires_at).unwrap();
        assert_eq!(store.count(), 2);
    }

    #[test]
    fn test_has_nonce() {
        let store = NonceStore::new();
        let nonce = [42u8; 32];
        let expires_at = Utc::now().timestamp() + 3600;

        assert!(!store.has_nonce(&nonce), "Should not have nonce initially");

        store.check_and_mark(&nonce, expires_at).unwrap();

        assert!(store.has_nonce(&nonce), "Should have nonce after marking");
    }

    #[test]
    fn test_concurrent_nonce_checks() {
        use std::sync::Arc;
        use std::thread;

        let store = Arc::new(NonceStore::new());
        let nonce = [42u8; 32];
        let expires_at = Utc::now().timestamp() + 3600;

        // Try to use the same nonce from multiple threads concurrently
        let mut handles = vec![];
        for _ in 0..10 {
            let store_clone = store.clone();
            handles.push(thread::spawn(move || {
                store_clone.check_and_mark(&nonce, expires_at).unwrap()
            }));
        }

        // Collect results
        let mut successes = 0;
        for handle in handles {
            if handle.join().unwrap() {
                successes += 1;
            }
        }

        // Exactly one should succeed (first one to acquire the write lock)
        assert_eq!(successes, 1, "Only one concurrent attempt should succeed");
    }
}
