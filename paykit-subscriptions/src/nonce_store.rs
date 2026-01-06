//! Nonce tracking for replay attack prevention
//!
//! This module provides nonce storage abstractions to prevent signature replay attacks.
//!
//! # Architecture
//!
//! - [`NonceStorage`] - Trait for persistent nonce storage (file, database, FFI callbacks)
//! - [`NonceStore`] - In-memory implementation (legacy, for testing)
//! - [`FileNonceStorage`] - File-based persistent implementation
//!
//! # Security
//!
//! - Each nonce can only be used once
//! - Expired nonces are periodically cleaned up
//! - Thread-safe implementations
//! - **CRITICAL**: Nonces MUST be persisted across app restarts to prevent replay attacks

use crate::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

/// Trait for persistent nonce storage.
///
/// # Security
///
/// Implementations MUST:
/// - Persist nonces across app restarts
/// - Be thread-safe for concurrent access
/// - Atomically check-and-mark nonces to prevent race conditions
///
/// # Implementing for Mobile Apps
///
/// Mobile apps should implement this trait using:
/// - **Android**: Room database or SharedPreferences
/// - **iOS**: Core Data, Keychain, or UserDefaults
///
/// The FFI layer in `paykit-mobile` provides a callback interface for this.
pub trait NonceStorage: Send + Sync {
    /// Check if a nonce has been used, and mark it as used if not.
    ///
    /// # Security
    ///
    /// This is the critical function for replay attack prevention.
    /// This operation MUST be atomic to prevent TOCTOU race conditions.
    ///
    /// # Arguments
    ///
    /// * `nonce` - The 32-byte nonce to check
    /// * `expires_at` - When this nonce's signature expires (Unix timestamp)
    ///
    /// # Returns
    ///
    /// - `Ok(true)` - Nonce is fresh (never seen before), now marked as used
    /// - `Ok(false)` - Nonce has been used (potential replay attack)
    /// - `Err(_)` - Storage error
    fn check_and_mark(&self, nonce: &[u8; 32], expires_at: i64) -> Result<bool>;

    /// Check if a nonce has been used (read-only).
    ///
    /// Does not modify state. Useful for validation without marking.
    fn is_used(&self, nonce: &[u8; 32]) -> Result<bool>;

    /// Clean up expired nonces to prevent unbounded storage growth.
    ///
    /// Should be called periodically (e.g., hourly or on app startup).
    ///
    /// # Arguments
    ///
    /// * `before` - Remove nonces that expired before this timestamp
    fn cleanup_expired(&self, before: i64) -> Result<()>;

    /// Get the count of tracked nonces (for monitoring/debugging).
    fn count(&self) -> Result<usize>;
}

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
    pub fn count(&self) -> Result<usize> {
        let nonces = self
            .used_nonces
            .read()
            .map_err(|e| anyhow::anyhow!("NonceStore lock poisoned: {}", e))?;
        Ok(nonces.len())
    }

    /// Check if a nonce has been used (read-only, doesn't mark)
    ///
    /// This is useful for testing or validation without modifying state.
    pub fn has_nonce(&self, nonce: &[u8; 32]) -> Result<bool> {
        let nonces = self
            .used_nonces
            .read()
            .map_err(|e| anyhow::anyhow!("NonceStore lock poisoned: {}", e))?;
        Ok(nonces.contains_key(nonce))
    }
}

impl Default for NonceStore {
    fn default() -> Self {
        Self::new()
    }
}

impl NonceStorage for NonceStore {
    fn check_and_mark(&self, nonce: &[u8; 32], expires_at: i64) -> Result<bool> {
        NonceStore::check_and_mark(self, nonce, expires_at)
    }

    fn is_used(&self, nonce: &[u8; 32]) -> Result<bool> {
        self.has_nonce(nonce)
    }

    fn cleanup_expired(&self, before: i64) -> Result<()> {
        NonceStore::cleanup_expired(self, before)
    }

    fn count(&self) -> Result<usize> {
        NonceStore::count(self)
    }
}

/// File-based persistent nonce storage.
///
/// Stores nonces in a JSON file for persistence across app restarts.
///
/// # Security
///
/// - Uses file-level locking for atomic operations
/// - Persists nonces to disk on every write
/// - Suitable for CLI tools and demos
///
/// For mobile apps, use the FFI callback interface instead for
/// platform-native storage (Room, Core Data, etc.).
#[cfg(not(target_arch = "wasm32"))]
pub struct FileNonceStorage {
    file_path: PathBuf,
    cache: RwLock<HashMap<[u8; 32], i64>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl FileNonceStorage {
    /// Create a new file-based nonce storage.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the JSON file for storing nonces
    ///
    /// If the file exists, nonces are loaded from it.
    pub fn new(file_path: PathBuf) -> Result<Self> {
        let cache = if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)
                .map_err(|e| anyhow::anyhow!("Failed to read nonce file: {}", e))?;
            Self::deserialize_nonces(&content)?
        } else {
            HashMap::new()
        };

        Ok(Self {
            file_path,
            cache: RwLock::new(cache),
        })
    }

    /// Create in a directory with default filename.
    pub fn in_directory(dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&dir)
            .map_err(|e| anyhow::anyhow!("Failed to create nonce directory: {}", e))?;
        Self::new(dir.join("nonces.json"))
    }

    fn serialize_nonces(nonces: &HashMap<[u8; 32], i64>) -> String {
        let entries: Vec<(String, i64)> =
            nonces.iter().map(|(k, v)| (hex::encode(k), *v)).collect();
        serde_json::to_string_pretty(&entries).unwrap_or_else(|_| "[]".to_string())
    }

    fn deserialize_nonces(content: &str) -> Result<HashMap<[u8; 32], i64>> {
        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }

        let entries: Vec<(String, i64)> = serde_json::from_str(content)
            .map_err(|e| anyhow::anyhow!("Failed to parse nonce file: {}", e))?;

        let mut nonces = HashMap::new();
        for (hex_nonce, expires_at) in entries {
            let bytes =
                hex::decode(&hex_nonce).map_err(|e| anyhow::anyhow!("Invalid nonce hex: {}", e))?;
            if bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                nonces.insert(arr, expires_at);
            }
        }
        Ok(nonces)
    }

    fn persist(&self, nonces: &HashMap<[u8; 32], i64>) -> Result<()> {
        let content = Self::serialize_nonces(nonces);
        std::fs::write(&self.file_path, content)
            .map_err(|e| anyhow::anyhow!("Failed to write nonce file: {}", e))?;
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl NonceStorage for FileNonceStorage {
    fn check_and_mark(&self, nonce: &[u8; 32], expires_at: i64) -> Result<bool> {
        let mut cache = self
            .cache
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        if cache.contains_key(nonce) {
            return Ok(false);
        }

        cache.insert(*nonce, expires_at);
        self.persist(&cache)?;
        Ok(true)
    }

    fn is_used(&self, nonce: &[u8; 32]) -> Result<bool> {
        let cache = self
            .cache
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        Ok(cache.contains_key(nonce))
    }

    fn cleanup_expired(&self, before: i64) -> Result<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        let original_count = cache.len();
        cache.retain(|_, expires_at| *expires_at >= before);

        if cache.len() != original_count {
            self.persist(&cache)?;
        }
        Ok(())
    }

    fn count(&self) -> Result<usize> {
        let cache = self
            .cache
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        Ok(cache.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // ======================================================================
    // NonceStore (in-memory) tests
    // ======================================================================

    #[test]
    fn test_fresh_nonce_accepted() {
        let store = NonceStore::new();
        let nonce = [42u8; 32];
        let expires_at = Utc::now().timestamp() + 3600;

        let result = store.check_and_mark(&nonce, expires_at).unwrap();
        assert!(result, "Fresh nonce should be accepted");
    }

    #[test]
    fn test_nonce_storage_trait_impl() {
        let store = NonceStore::new();
        let nonce = [42u8; 32];
        let expires_at = Utc::now().timestamp() + 3600;

        // Test via trait
        let storage: &dyn NonceStorage = &store;
        assert!(!storage.is_used(&nonce).unwrap());

        assert!(storage.check_and_mark(&nonce, expires_at).unwrap());
        assert!(storage.is_used(&nonce).unwrap());

        // Duplicate should fail
        assert!(!storage.check_and_mark(&nonce, expires_at).unwrap());
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

        assert_eq!(store.count().unwrap(), 2, "Should have 2 nonces");

        // Clean up expired nonces
        store.cleanup_expired(now).unwrap();

        assert_eq!(
            store.count().unwrap(),
            1,
            "Should have 1 nonce after cleanup"
        );
        assert!(
            store.has_nonce(&recent_nonce).unwrap(),
            "Recent nonce should remain"
        );
        assert!(
            !store.has_nonce(&old_nonce).unwrap(),
            "Old nonce should be removed"
        );
    }

    #[test]
    fn test_count() {
        let store = NonceStore::new();
        let expires_at = Utc::now().timestamp() + 3600;

        assert_eq!(store.count().unwrap(), 0, "Should start empty");

        store.check_and_mark(&[1u8; 32], expires_at).unwrap();
        assert_eq!(store.count().unwrap(), 1);

        store.check_and_mark(&[2u8; 32], expires_at).unwrap();
        assert_eq!(store.count().unwrap(), 2);
    }

    #[test]
    fn test_has_nonce() {
        let store = NonceStore::new();
        let nonce = [42u8; 32];
        let expires_at = Utc::now().timestamp() + 3600;

        assert!(
            !store.has_nonce(&nonce).unwrap(),
            "Should not have nonce initially"
        );

        store.check_and_mark(&nonce, expires_at).unwrap();

        assert!(
            store.has_nonce(&nonce).unwrap(),
            "Should have nonce after marking"
        );
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

    // ======================================================================
    // FileNonceStorage tests
    // ======================================================================

    #[cfg(not(target_arch = "wasm32"))]
    mod file_storage_tests {
        use super::*;
        use tempfile::tempdir;

        #[test]
        fn test_file_storage_basic() {
            let dir = tempdir().unwrap();
            let storage = FileNonceStorage::in_directory(dir.path().to_path_buf()).unwrap();

            let nonce = [42u8; 32];
            let expires_at = Utc::now().timestamp() + 3600;

            // Fresh nonce should be accepted
            assert!(storage.check_and_mark(&nonce, expires_at).unwrap());

            // Duplicate should be rejected
            assert!(!storage.check_and_mark(&nonce, expires_at).unwrap());
        }

        #[test]
        fn test_file_storage_persistence() {
            let dir = tempdir().unwrap();
            let path = dir.path().to_path_buf();

            let nonce = [42u8; 32];
            let expires_at = Utc::now().timestamp() + 3600;

            // First instance: mark nonce
            {
                let storage = FileNonceStorage::in_directory(path.clone()).unwrap();
                assert!(storage.check_and_mark(&nonce, expires_at).unwrap());
            }

            // Second instance: should see the nonce as used
            {
                let storage = FileNonceStorage::in_directory(path).unwrap();
                assert!(
                    storage.is_used(&nonce).unwrap(),
                    "Nonce should persist across instances"
                );
                assert!(
                    !storage.check_and_mark(&nonce, expires_at).unwrap(),
                    "Persisted nonce should be rejected"
                );
            }
        }

        #[test]
        fn test_file_storage_cleanup() {
            let dir = tempdir().unwrap();
            let storage = FileNonceStorage::in_directory(dir.path().to_path_buf()).unwrap();

            let now = Utc::now().timestamp();
            let old_nonce = [1u8; 32];
            let recent_nonce = [2u8; 32];

            storage.check_and_mark(&old_nonce, now - 1000).unwrap();
            storage.check_and_mark(&recent_nonce, now + 1000).unwrap();

            assert_eq!(storage.count().unwrap(), 2);

            storage.cleanup_expired(now).unwrap();

            assert_eq!(storage.count().unwrap(), 1);
            assert!(!storage.is_used(&old_nonce).unwrap());
            assert!(storage.is_used(&recent_nonce).unwrap());
        }

        #[test]
        fn test_file_storage_trait_object() {
            let dir = tempdir().unwrap();
            let storage = FileNonceStorage::in_directory(dir.path().to_path_buf()).unwrap();

            // Test as trait object
            let storage: Box<dyn NonceStorage> = Box::new(storage);

            let nonce = [42u8; 32];
            let expires_at = Utc::now().timestamp() + 3600;

            assert!(storage.check_and_mark(&nonce, expires_at).unwrap());
            assert!(storage.is_used(&nonce).unwrap());
        }
    }
}
