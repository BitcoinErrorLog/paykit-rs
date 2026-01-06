//! FFI callback interface for persistent nonce storage.
//!
//! This module provides the FFI layer for mobile apps to implement
//! persistent nonce storage using platform-native storage mechanisms.
//!
//! # Architecture
//!
//! Mobile apps implement the [`NonceStorageFFI`] callback interface in
//! Swift/Kotlin. The [`NonceStorageBridge`] wraps this callback and
//! implements the Rust [`NonceStorage`] trait, allowing it to be used
//! with [`SubscriptionManager`].
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                 Mobile App (Swift/Kotlin)                        │
//! │  ┌─────────────────────────────────────────────────────────────┐ │
//! │  │  NonceStorageFFI Implementation                             │ │
//! │  │  (SharedPreferences on Android, UserDefaults on iOS)        │ │
//! │  └─────────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼ (UniFFI callback)
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      Rust FFI Layer                              │
//! │  ┌─────────────────────────────────────────────────────────────┐ │
//! │  │  NonceStorageBridge                                         │ │
//! │  │  (Implements paykit_subscriptions::NonceStorage trait)      │ │
//! │  └─────────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    SubscriptionManager                           │
//! │  (Uses NonceStorage trait for replay attack prevention)          │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example (Kotlin)
//!
//! ```kotlin
//! class BitkitNonceStorage(private val context: Context) : NonceStorageFFI {
//!     private val prefs = context.getSharedPreferences("nonces", Context.MODE_PRIVATE)
//!
//!     override fun checkAndMark(nonceHex: String, expiresAt: Long): Boolean {
//!         synchronized(prefs) {
//!             if (prefs.contains(nonceHex)) return false
//!             prefs.edit().putLong(nonceHex, expiresAt).apply()
//!             return true
//!         }
//!     }
//!
//!     override fun isUsed(nonceHex: String): Boolean = prefs.contains(nonceHex)
//!
//!     override fun cleanupExpired(before: Long): UInt {
//!         synchronized(prefs) {
//!             var count = 0u
//!             prefs.all.forEach { (key, value) ->
//!                 if (value is Long && value < before) {
//!                     prefs.edit().remove(key).apply()
//!                     count++
//!                 }
//!             }
//!             return count
//!         }
//!     }
//!
//!     override fun count(): UInt = prefs.all.size.toUInt()
//! }
//! ```

use std::sync::Arc;

use crate::PaykitMobileError;

/// Result type for nonce storage operations.
pub type Result<T> = std::result::Result<T, PaykitMobileError>;

/// FFI callback interface for persistent nonce storage.
///
/// Mobile apps implement this trait to provide platform-native
/// persistent storage for nonces used in replay attack prevention.
///
/// # Security
///
/// Implementations MUST:
/// - Persist nonces across app restarts (critical for replay attack prevention)
/// - Be thread-safe for concurrent access
/// - Atomically check-and-mark nonces to prevent race conditions
///
/// # Platform Recommendations
///
/// - **Android**: Use SharedPreferences or Room database
/// - **iOS**: Use UserDefaults or Keychain
///
/// # Thread Safety
///
/// All methods may be called from any thread. Implementations must be
/// thread-safe.
#[uniffi::export(callback_interface)]
pub trait NonceStorageFFI: Send + Sync {
    /// Check if a nonce has been used, and mark it as used if not.
    ///
    /// This is the critical function for replay attack prevention.
    /// This operation MUST be atomic to prevent TOCTOU race conditions.
    ///
    /// # Arguments
    ///
    /// * `nonce_hex` - The 32-byte nonce as a hex string (64 characters)
    /// * `expires_at` - When this nonce's signature expires (Unix timestamp)
    ///
    /// # Returns
    ///
    /// - `true` - Nonce is fresh (never seen before), now marked as used
    /// - `false` - Nonce has been used (potential replay attack)
    fn check_and_mark(&self, nonce_hex: String, expires_at: i64) -> bool;

    /// Check if a nonce has been used (read-only).
    ///
    /// Does not modify state. Useful for validation without marking.
    ///
    /// # Arguments
    ///
    /// * `nonce_hex` - The 32-byte nonce as a hex string (64 characters)
    fn is_used(&self, nonce_hex: String) -> bool;

    /// Clean up expired nonces to prevent unbounded storage growth.
    ///
    /// Should be called periodically (e.g., hourly or on app startup).
    ///
    /// # Arguments
    ///
    /// * `before` - Remove nonces that expired before this timestamp (Unix seconds)
    ///
    /// # Returns
    ///
    /// The number of nonces removed.
    fn cleanup_expired(&self, before: i64) -> u32;

    /// Get the count of tracked nonces (for monitoring/debugging).
    fn count(&self) -> u32;
}

/// Bridge that wraps a [`NonceStorageFFI`] callback and implements
/// the [`paykit_subscriptions::NonceStorage`] trait.
///
/// This allows mobile apps to provide their own nonce storage implementation
/// while still being usable with the core Paykit subscription system.
pub struct NonceStorageBridge {
    callback: Arc<dyn NonceStorageFFI>,
}

impl NonceStorageBridge {
    /// Create a new bridge wrapping the given FFI callback.
    pub fn new(callback: Arc<dyn NonceStorageFFI>) -> Self {
        Self { callback }
    }

    /// Convert a 32-byte nonce to a hex string.
    fn nonce_to_hex(nonce: &[u8; 32]) -> String {
        hex::encode(nonce)
    }
}

impl paykit_subscriptions::NonceStorage for NonceStorageBridge {
    fn check_and_mark(
        &self,
        nonce: &[u8; 32],
        expires_at: i64,
    ) -> paykit_subscriptions::Result<bool> {
        let nonce_hex = Self::nonce_to_hex(nonce);
        Ok(self.callback.check_and_mark(nonce_hex, expires_at))
    }

    fn is_used(&self, nonce: &[u8; 32]) -> paykit_subscriptions::Result<bool> {
        let nonce_hex = Self::nonce_to_hex(nonce);
        Ok(self.callback.is_used(nonce_hex))
    }

    fn cleanup_expired(&self, before: i64) -> paykit_subscriptions::Result<()> {
        self.callback.cleanup_expired(before);
        Ok(())
    }

    fn count(&self) -> paykit_subscriptions::Result<usize> {
        Ok(self.callback.count() as usize)
    }
}

// Make NonceStorageBridge thread-safe
unsafe impl Send for NonceStorageBridge {}
unsafe impl Sync for NonceStorageBridge {}

#[cfg(test)]
mod tests {
    use super::*;
    use paykit_subscriptions::NonceStorage;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock implementation for testing
    struct MockNonceStorage {
        nonces: Mutex<HashMap<String, i64>>,
    }

    impl MockNonceStorage {
        fn new() -> Self {
            Self {
                nonces: Mutex::new(HashMap::new()),
            }
        }
    }

    impl NonceStorageFFI for MockNonceStorage {
        fn check_and_mark(&self, nonce_hex: String, expires_at: i64) -> bool {
            let mut nonces = self.nonces.lock().unwrap();
            if nonces.contains_key(&nonce_hex) {
                false
            } else {
                nonces.insert(nonce_hex, expires_at);
                true
            }
        }

        fn is_used(&self, nonce_hex: String) -> bool {
            self.nonces.lock().unwrap().contains_key(&nonce_hex)
        }

        fn cleanup_expired(&self, before: i64) -> u32 {
            let mut nonces = self.nonces.lock().unwrap();
            let before_count = nonces.len();
            nonces.retain(|_, &mut expires| expires >= before);
            (before_count - nonces.len()) as u32
        }

        fn count(&self) -> u32 {
            self.nonces.lock().unwrap().len() as u32
        }
    }

    #[test]
    fn test_bridge_check_and_mark() {
        let mock = Arc::new(MockNonceStorage::new());
        let bridge = NonceStorageBridge::new(mock);

        let nonce = [0u8; 32];
        let expires_at = 1234567890;

        // First check should return true (fresh nonce)
        assert!(bridge.check_and_mark(&nonce, expires_at).unwrap());

        // Second check should return false (already used)
        assert!(!bridge.check_and_mark(&nonce, expires_at).unwrap());
    }

    #[test]
    fn test_bridge_is_used() {
        let mock = Arc::new(MockNonceStorage::new());
        let bridge = NonceStorageBridge::new(mock);

        let nonce = [1u8; 32];

        assert!(!bridge.is_used(&nonce).unwrap());
        bridge.check_and_mark(&nonce, 999).unwrap();
        assert!(bridge.is_used(&nonce).unwrap());
    }

    #[test]
    fn test_bridge_cleanup_expired() {
        let mock = Arc::new(MockNonceStorage::new());
        let bridge = NonceStorageBridge::new(mock);

        let nonce1 = [1u8; 32];
        let nonce2 = [2u8; 32];

        // Add one expired and one fresh nonce
        bridge.check_and_mark(&nonce1, 100).unwrap();
        bridge.check_and_mark(&nonce2, 200).unwrap();

        assert_eq!(bridge.count().unwrap(), 2);

        // Cleanup nonces that expired before 150
        bridge.cleanup_expired(150).unwrap();

        assert_eq!(bridge.count().unwrap(), 1);
        assert!(!bridge.is_used(&nonce1).unwrap());
        assert!(bridge.is_used(&nonce2).unwrap());
    }
}
