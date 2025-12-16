//! Atomic Spending Limit FFI Bindings
//!
//! This module provides FFI-safe wrappers for atomic spending limit operations,
//! enabling mobile applications to safely enforce spending limits during
//! background auto-pay operations.
//!
//! # Security
//!
//! These operations are designed to prevent TOCTOU (time-of-check-time-of-use)
//! race conditions when checking and enforcing spending limits. The atomic
//! reserve-commit-rollback pattern ensures:
//!
//! - Spending is reserved before payment execution
//! - Reservation is committed on payment success
//! - Reservation is rolled back on payment failure
//!
//! # Example Flow
//!
//! ```ignore
//! // 1. Reserve spending before payment
//! let reservation = manager.try_reserve_spending(peer_pubkey, amount_sats)?;
//!
//! // 2. Execute payment
//! let payment_result = execute_payment(...);
//!
//! // 3. Commit or rollback based on result
//! if payment_result.is_ok() {
//!     manager.commit_spending(reservation.reservation_id)?;
//! } else {
//!     manager.rollback_spending(reservation.reservation_id)?;
//! }
//! ```

use crate::{PaykitMobileError, Result};
use paykit_subscriptions::{Amount, PeerSpendingLimit};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

// ============================================================================
// FFI Types
// ============================================================================

/// FFI-safe spending reservation token.
///
/// Returned by `try_reserve_spending()` and must be either committed
/// or rolled back after payment execution.
#[derive(Clone, Debug, uniffi::Record)]
pub struct SpendingReservationFFI {
    /// Unique identifier for this reservation
    pub reservation_id: String,
    /// Peer public key (z-base32 encoded)
    pub peer_pubkey: String,
    /// Reserved amount in satoshis
    pub amount_sats: i64,
    /// Unix timestamp when reservation was created
    pub created_at: i64,
}

/// FFI-safe peer spending limit information.
#[derive(Clone, Debug, uniffi::Record)]
pub struct PeerSpendingLimitFFI {
    /// Peer public key (z-base32 encoded)
    pub peer_pubkey: String,
    /// Total spending limit in satoshis
    pub total_limit_sats: i64,
    /// Currently spent amount in satoshis
    pub current_spent_sats: i64,
    /// Period for limit reset ("daily", "weekly", "monthly")
    pub period: String,
    /// Remaining limit in satoshis
    pub remaining_sats: i64,
    /// Unix timestamp of last reset
    pub last_reset: i64,
}

impl From<&PeerSpendingLimit> for PeerSpendingLimitFFI {
    fn from(limit: &PeerSpendingLimit) -> Self {
        Self {
            peer_pubkey: limit.peer.to_string(),
            total_limit_sats: limit.total_amount_limit.as_sats(),
            current_spent_sats: limit.current_spent.as_sats(),
            period: limit.period.clone(),
            remaining_sats: limit.remaining_limit().as_sats(),
            last_reset: limit.last_reset.timestamp(),
        }
    }
}

/// Result of checking if amount would exceed limit.
#[derive(Clone, Debug, uniffi::Record)]
pub struct SpendingCheckResultFFI {
    /// Whether the amount would exceed the limit
    pub would_exceed: bool,
    /// Current spent amount in satoshis
    pub current_spent_sats: i64,
    /// Remaining limit in satoshis
    pub remaining_sats: i64,
    /// Amount being checked in satoshis
    pub check_amount_sats: i64,
}

// ============================================================================
// Spending Manager
// ============================================================================

/// Manager for atomic spending limit operations.
///
/// This manager handles the reserve-commit-rollback pattern for spending limits,
/// ensuring thread-safe atomic operations during background auto-pay.
#[derive(uniffi::Object)]
pub struct SpendingManagerFFI {
    /// Base path for storage (stored as String for FFI compatibility)
    storage_path_str: String,
    /// In-flight reservations (reservation_id -> reservation data)
    reservations: RwLock<HashMap<String, ReservationData>>,
}

/// Internal reservation data
struct ReservationData {
    peer_pubkey: String,
    amount_sats: i64,
    #[allow(dead_code)]
    created_at: i64,
}

#[uniffi::export]
impl SpendingManagerFFI {
    /// Create a new spending manager with the given storage path.
    ///
    /// # Arguments
    ///
    /// * `storage_path` - Path to the storage directory for spending limits
    #[uniffi::constructor]
    pub fn new(storage_path: String) -> Result<Arc<Self>> {
        let path = PathBuf::from(&storage_path);
        
        // Ensure the peer_limits directory exists
        let limits_path = path.join("peer_limits");
        std::fs::create_dir_all(&limits_path).map_err(|e| PaykitMobileError::Internal {
            msg: format!("Failed to create storage directory: {}", e),
        })?;

        Ok(Arc::new(Self {
            storage_path_str: storage_path,
            reservations: RwLock::new(HashMap::new()),
        }))
    }

    /// Set a spending limit for a peer.
    ///
    /// # Arguments
    ///
    /// * `peer_pubkey` - Peer's public key (z-base32 encoded)
    /// * `limit_sats` - Maximum spending limit in satoshis
    /// * `period` - Reset period ("daily", "weekly", or "monthly")
    ///
    /// # Example
    ///
    /// ```ignore
    /// manager.set_peer_spending_limit(
    ///     "8pinxxgqs41...",
    ///     100000,  // 100,000 sats
    ///     "monthly"
    /// )?;
    /// ```
    pub fn set_peer_spending_limit(
        &self,
        peer_pubkey: String,
        limit_sats: i64,
        period: String,
    ) -> Result<PeerSpendingLimitFFI> {
        use std::str::FromStr;

        let peer = paykit_lib::PublicKey::from_str(&peer_pubkey).map_err(|e| {
            PaykitMobileError::Validation {
                msg: format!("Invalid peer public key: {}", e),
            }
        })?;

        let limit = PeerSpendingLimit::new(peer, Amount::from_sats(limit_sats), period);

        // Save to file
        let path = self.peer_limit_path(&peer_pubkey);
        let json = serde_json::to_string_pretty(&limit).map_err(|e| {
            PaykitMobileError::Serialization {
                msg: e.to_string(),
            }
        })?;
        std::fs::write(&path, json).map_err(|e| PaykitMobileError::Internal {
            msg: format!("Failed to save spending limit: {}", e),
        })?;

        Ok(PeerSpendingLimitFFI::from(&limit))
    }

    /// Get the current spending limit for a peer.
    ///
    /// # Arguments
    ///
    /// * `peer_pubkey` - Peer's public key (z-base32 encoded)
    ///
    /// # Returns
    ///
    /// The spending limit if one exists, None otherwise.
    pub fn get_peer_spending_limit(
        &self,
        peer_pubkey: String,
    ) -> Result<Option<PeerSpendingLimitFFI>> {
        let path = self.peer_limit_path(&peer_pubkey);
        
        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(&path).map_err(|e| PaykitMobileError::Internal {
            msg: format!("Failed to read spending limit: {}", e),
        })?;

        let limit: PeerSpendingLimit =
            serde_json::from_str(&json).map_err(|e| PaykitMobileError::Serialization {
                msg: e.to_string(),
            })?;

        Ok(Some(PeerSpendingLimitFFI::from(&limit)))
    }

    /// Remove a spending limit for a peer.
    ///
    /// # Arguments
    ///
    /// * `peer_pubkey` - Peer's public key (z-base32 encoded)
    pub fn remove_peer_spending_limit(&self, peer_pubkey: String) -> Result<()> {
        let path = self.peer_limit_path(&peer_pubkey);
        
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| PaykitMobileError::Internal {
                msg: format!("Failed to remove spending limit: {}", e),
            })?;
        }

        Ok(())
    }

    /// Check if an amount would exceed the spending limit (non-blocking check).
    ///
    /// This is a read-only check that does not modify the spending limit.
    /// For actual payment execution, use `try_reserve_spending()` instead.
    ///
    /// # Arguments
    ///
    /// * `peer_pubkey` - Peer's public key
    /// * `amount_sats` - Amount to check in satoshis
    ///
    /// # Returns
    ///
    /// A result containing whether the amount would exceed the limit.
    pub fn would_exceed_spending_limit(
        &self,
        peer_pubkey: String,
        amount_sats: i64,
    ) -> Result<SpendingCheckResultFFI> {
        let path = self.peer_limit_path(&peer_pubkey);

        if !path.exists() {
            return Err(PaykitMobileError::NotFound {
                msg: format!("No spending limit set for peer: {}", peer_pubkey),
            });
        }

        let json = std::fs::read_to_string(&path).map_err(|e| PaykitMobileError::Internal {
            msg: format!("Failed to read spending limit: {}", e),
        })?;

        let mut limit: PeerSpendingLimit =
            serde_json::from_str(&json).map_err(|e| PaykitMobileError::Serialization {
                msg: e.to_string(),
            })?;

        // Check if reset needed
        if limit.should_reset() {
            limit.reset();
        }

        let check_amount = Amount::from_sats(amount_sats);
        let would_exceed = limit.would_exceed_limit(&check_amount);

        Ok(SpendingCheckResultFFI {
            would_exceed,
            current_spent_sats: limit.current_spent.as_sats(),
            remaining_sats: limit.remaining_limit().as_sats(),
            check_amount_sats: amount_sats,
        })
    }

    /// Try to reserve spending amount atomically.
    ///
    /// This method performs an atomic check-and-reserve operation:
    /// 1. Acquires exclusive file lock
    /// 2. Checks if amount would exceed limit
    /// 3. If within limit, reserves the amount
    /// 4. Returns a reservation token
    ///
    /// The reservation MUST be either committed or rolled back after payment.
    ///
    /// # Arguments
    ///
    /// * `peer_pubkey` - Peer's public key
    /// * `amount_sats` - Amount to reserve in satoshis
    ///
    /// # Errors
    ///
    /// - `NotFound` if no spending limit is set for this peer
    /// - `Validation` with "Limit exceeded" if amount would exceed limit
    ///
    /// # Example
    ///
    /// ```ignore
    /// let reservation = manager.try_reserve_spending("8pinxxgqs41...", 10000)?;
    /// // Execute payment...
    /// manager.commit_spending(reservation.reservation_id)?;
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_reserve_spending(
        &self,
        peer_pubkey: String,
        amount_sats: i64,
    ) -> Result<SpendingReservationFFI> {
        use fs2::FileExt;
        use std::fs::OpenOptions;

        let path = self.peer_limit_path(&peer_pubkey);

        // Open/create the file for locking
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| PaykitMobileError::Internal {
                msg: format!("Failed to open spending limit file: {}", e),
            })?;

        // Acquire exclusive lock (blocks until available)
        file.lock_exclusive().map_err(|e| PaykitMobileError::Internal {
            msg: format!("Failed to acquire lock: {}", e),
        })?;

        // Load current limit
        let result = (|| -> Result<SpendingReservationFFI> {
            if !path.exists() || std::fs::metadata(&path).map(|m| m.len() == 0).unwrap_or(true) {
                return Err(PaykitMobileError::NotFound {
                    msg: format!("No spending limit set for peer: {}", peer_pubkey),
                });
            }

            let json = std::fs::read_to_string(&path).map_err(|e| PaykitMobileError::Internal {
                msg: format!("Failed to read spending limit: {}", e),
            })?;

            let mut limit: PeerSpendingLimit =
                serde_json::from_str(&json).map_err(|e| PaykitMobileError::Serialization {
                    msg: e.to_string(),
                })?;

            // Check if reset needed
            if limit.should_reset() {
                limit.reset();
            }

            // Check if would exceed limit
            let reserve_amount = Amount::from_sats(amount_sats);
            if limit.would_exceed_limit(&reserve_amount) {
                return Err(PaykitMobileError::Validation {
                    msg: format!(
                        "Limit exceeded: trying to spend {} sats, but only {} sats remaining",
                        amount_sats,
                        limit.remaining_limit().as_sats()
                    ),
                });
            }

            // Reserve amount
            limit.current_spent = limit
                .current_spent
                .checked_add(&reserve_amount)
                .ok_or_else(|| PaykitMobileError::Internal {
                    msg: "Spending amount overflow".to_string(),
                })?;

            // Save updated limit
            let json = serde_json::to_string_pretty(&limit).map_err(|e| {
                PaykitMobileError::Serialization {
                    msg: e.to_string(),
                }
            })?;
            std::fs::write(&path, json).map_err(|e| PaykitMobileError::Internal {
                msg: format!("Failed to save spending limit: {}", e),
            })?;

            // Generate reservation ID
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let reservation_id = format!("rsv_{}_{:08x}", now, rand::random::<u32>());

            // Store reservation in memory for tracking
            let mut reservations = self.reservations.write().map_err(|_| {
                PaykitMobileError::Internal {
                    msg: "Failed to acquire reservations lock".to_string(),
                }
            })?;
            reservations.insert(
                reservation_id.clone(),
                ReservationData {
                    peer_pubkey: peer_pubkey.clone(),
                    amount_sats,
                    created_at: now,
                },
            );

            Ok(SpendingReservationFFI {
                reservation_id,
                peer_pubkey,
                amount_sats,
                created_at: now,
            })
        })();

        // Release lock (important: do this even on error)
        let _ = file.unlock();

        result
    }

    /// Commit a spending reservation after successful payment.
    ///
    /// This finalizes the reservation - the spent amount becomes permanent.
    /// This operation is idempotent.
    ///
    /// # Arguments
    ///
    /// * `reservation_id` - The reservation ID from `try_reserve_spending()`
    pub fn commit_spending(&self, reservation_id: String) -> Result<()> {
        let mut reservations = self.reservations.write().map_err(|_| {
            PaykitMobileError::Internal {
                msg: "Failed to acquire reservations lock".to_string(),
            }
        })?;

        // Remove the reservation from tracking
        // The spending was already applied when we reserved it
        reservations.remove(&reservation_id);

        Ok(())
    }

    /// Rollback a spending reservation after failed payment.
    ///
    /// This releases the reserved amount back to the limit.
    /// This operation is idempotent.
    ///
    /// # Arguments
    ///
    /// * `reservation_id` - The reservation ID from `try_reserve_spending()`
    #[cfg(not(target_arch = "wasm32"))]
    pub fn rollback_spending(&self, reservation_id: String) -> Result<()> {
        use fs2::FileExt;
        use std::fs::OpenOptions;

        // Get the reservation data
        let reservation_data = {
            let mut reservations = self.reservations.write().map_err(|_| {
                PaykitMobileError::Internal {
                    msg: "Failed to acquire reservations lock".to_string(),
                }
            })?;

            match reservations.remove(&reservation_id) {
                Some(data) => data,
                None => {
                    // Already rolled back or committed - idempotent
                    return Ok(());
                }
            }
        };

        let path = self.peer_limit_path(&reservation_data.peer_pubkey);

        if !path.exists() {
            // Limit was deleted, nothing to rollback
            return Ok(());
        }

        // Open file for locking
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| PaykitMobileError::Internal {
                msg: format!("Failed to open spending limit file: {}", e),
            })?;

        // Acquire exclusive lock
        file.lock_exclusive().map_err(|e| PaykitMobileError::Internal {
            msg: format!("Failed to acquire lock: {}", e),
        })?;

        let result = (|| -> Result<()> {
            let json = std::fs::read_to_string(&path).map_err(|e| PaykitMobileError::Internal {
                msg: format!("Failed to read spending limit: {}", e),
            })?;

            let mut limit: PeerSpendingLimit =
                serde_json::from_str(&json).map_err(|e| PaykitMobileError::Serialization {
                    msg: e.to_string(),
                })?;

            // Rollback the reserved amount
            let rollback_amount = Amount::from_sats(reservation_data.amount_sats);
            limit.current_spent = limit
                .current_spent
                .checked_sub(&rollback_amount)
                .unwrap_or(Amount::from_sats(0)); // Defensive: don't go negative

            // Save updated limit
            let json = serde_json::to_string_pretty(&limit).map_err(|e| {
                PaykitMobileError::Serialization {
                    msg: e.to_string(),
                }
            })?;
            std::fs::write(&path, json).map_err(|e| PaykitMobileError::Internal {
                msg: format!("Failed to save spending limit: {}", e),
            })?;

            Ok(())
        })();

        // Release lock
        let _ = file.unlock();

        result
    }

    /// List all spending limits.
    ///
    /// # Returns
    ///
    /// List of all configured spending limits.
    pub fn list_spending_limits(&self) -> Result<Vec<PeerSpendingLimitFFI>> {
        let limits_path = self.storage_path().join("peer_limits");
        let mut result = Vec::new();

        if !limits_path.exists() {
            return Ok(result);
        }

        for entry in std::fs::read_dir(limits_path).map_err(|e| PaykitMobileError::Internal {
            msg: format!("Failed to read limits directory: {}", e),
        })? {
            let entry = entry.map_err(|e| PaykitMobileError::Internal {
                msg: format!("Failed to read directory entry: {}", e),
            })?;
            let path = entry.path();

            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let json = std::fs::read_to_string(&path).map_err(|e| PaykitMobileError::Internal {
                msg: format!("Failed to read spending limit: {}", e),
            })?;

            let limit: PeerSpendingLimit =
                serde_json::from_str(&json).map_err(|e| PaykitMobileError::Serialization {
                    msg: e.to_string(),
                })?;

            result.push(PeerSpendingLimitFFI::from(&limit));
        }

        Ok(result)
    }

    /// Get the number of active (in-flight) reservations.
    ///
    /// Useful for debugging and monitoring.
    pub fn active_reservations_count(&self) -> u32 {
        self.reservations
            .read()
            .map(|r| r.len() as u32)
            .unwrap_or(0)
    }
}

// ============================================================================
// Private Helpers (not exposed via FFI)
// ============================================================================

impl SpendingManagerFFI {
    fn storage_path(&self) -> PathBuf {
        PathBuf::from(&self.storage_path_str)
    }

    fn peer_limit_path(&self, peer_pubkey: &str) -> PathBuf {
        // Sanitize the pubkey for use as filename
        let safe_name = peer_pubkey.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        self.storage_path()
            .join("peer_limits")
            .join(format!("{}.json", safe_name))
    }
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Create a new spending manager.
///
/// # Arguments
///
/// * `storage_path` - Path to the storage directory
#[uniffi::export]
pub fn create_spending_manager(storage_path: String) -> Result<Arc<SpendingManagerFFI>> {
    SpendingManagerFFI::new(storage_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("paykit_test_{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn create_test_manager() -> (std::path::PathBuf, Arc<SpendingManagerFFI>) {
        let temp_dir = create_test_dir();
        let manager = SpendingManagerFFI::new(temp_dir.to_string_lossy().to_string()).unwrap();
        (temp_dir, manager)
    }
    
    fn generate_test_pubkey() -> String {
        // Generate a valid pkarr keypair and return the z-base32 encoded public key
        let keypair = pkarr::Keypair::random();
        keypair.public_key().to_z32()
    }

    #[test]
    fn test_set_and_get_spending_limit() {
        let (_temp_dir, manager) = create_test_manager();
        let peer = generate_test_pubkey();

        // Set limit
        let limit = manager
            .set_peer_spending_limit(peer.clone(), 100000, "monthly".to_string())
            .unwrap();

        assert_eq!(limit.total_limit_sats, 100000);
        assert_eq!(limit.current_spent_sats, 0);
        assert_eq!(limit.remaining_sats, 100000);
        assert_eq!(limit.period, "monthly");

        // Get limit
        let loaded = manager
            .get_peer_spending_limit(peer)
            .unwrap();

        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.total_limit_sats, 100000);
    }

    #[test]
    fn test_would_exceed_spending_limit() {
        let (_temp_dir, manager) = create_test_manager();
        let peer = generate_test_pubkey();

        manager
            .set_peer_spending_limit(peer.clone(), 10000, "daily".to_string())
            .unwrap();

        // Check amount within limit
        let result = manager
            .would_exceed_spending_limit(peer.clone(), 5000)
            .unwrap();
        assert!(!result.would_exceed);
        assert_eq!(result.remaining_sats, 10000);

        // Check amount exceeding limit
        let result = manager
            .would_exceed_spending_limit(peer, 15000)
            .unwrap();
        assert!(result.would_exceed);
    }

    #[test]
    fn test_reserve_commit_flow() {
        let (_temp_dir, manager) = create_test_manager();
        let peer = generate_test_pubkey();

        manager
            .set_peer_spending_limit(peer.clone(), 10000, "daily".to_string())
            .unwrap();

        // Reserve spending
        let reservation = manager
            .try_reserve_spending(peer.clone(), 3000)
            .unwrap();

        assert_eq!(reservation.amount_sats, 3000);
        assert_eq!(manager.active_reservations_count(), 1);

        // Check limit is reduced
        let limit = manager
            .get_peer_spending_limit(peer.clone())
            .unwrap()
            .unwrap();
        assert_eq!(limit.current_spent_sats, 3000);
        assert_eq!(limit.remaining_sats, 7000);

        // Commit the reservation
        manager.commit_spending(reservation.reservation_id).unwrap();
        assert_eq!(manager.active_reservations_count(), 0);

        // Limit should still be reduced
        let limit = manager
            .get_peer_spending_limit(peer)
            .unwrap()
            .unwrap();
        assert_eq!(limit.current_spent_sats, 3000);
    }

    #[test]
    fn test_reserve_rollback_flow() {
        let (_temp_dir, manager) = create_test_manager();
        let peer = generate_test_pubkey();

        manager
            .set_peer_spending_limit(peer.clone(), 10000, "daily".to_string())
            .unwrap();

        // Reserve spending
        let reservation = manager
            .try_reserve_spending(peer.clone(), 3000)
            .unwrap();

        // Rollback the reservation
        manager
            .rollback_spending(reservation.reservation_id)
            .unwrap();
        assert_eq!(manager.active_reservations_count(), 0);

        // Limit should be restored
        let limit = manager
            .get_peer_spending_limit(peer)
            .unwrap()
            .unwrap();
        assert_eq!(limit.current_spent_sats, 0);
        assert_eq!(limit.remaining_sats, 10000);
    }

    #[test]
    fn test_reserve_exceeds_limit() {
        let (_temp_dir, manager) = create_test_manager();
        let peer = generate_test_pubkey();

        manager
            .set_peer_spending_limit(peer.clone(), 10000, "daily".to_string())
            .unwrap();

        // Try to reserve more than limit
        let result = manager.try_reserve_spending(peer, 15000);

        assert!(result.is_err());
        match result {
            Err(PaykitMobileError::Validation { msg }) => {
                assert!(msg.contains("Limit exceeded"));
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_list_spending_limits() {
        let (_temp_dir, manager) = create_test_manager();
        let peer1 = generate_test_pubkey();
        let peer2 = generate_test_pubkey();

        manager
            .set_peer_spending_limit(peer1, 10000, "daily".to_string())
            .unwrap();
        manager
            .set_peer_spending_limit(peer2, 20000, "weekly".to_string())
            .unwrap();

        let limits = manager.list_spending_limits().unwrap();
        assert_eq!(limits.len(), 2);
    }

    #[test]
    fn test_remove_spending_limit() {
        let (_temp_dir, manager) = create_test_manager();
        let peer = generate_test_pubkey();

        manager
            .set_peer_spending_limit(peer.clone(), 10000, "daily".to_string())
            .unwrap();

        manager
            .remove_peer_spending_limit(peer.clone())
            .unwrap();

        let limit = manager
            .get_peer_spending_limit(peer)
            .unwrap();
        assert!(limit.is_none());
    }

    #[test]
    fn test_commit_idempotent() {
        let (_temp_dir, manager) = create_test_manager();
        let peer = generate_test_pubkey();

        manager
            .set_peer_spending_limit(peer.clone(), 10000, "daily".to_string())
            .unwrap();

        let reservation = manager
            .try_reserve_spending(peer, 3000)
            .unwrap();

        // Commit multiple times should be fine
        manager
            .commit_spending(reservation.reservation_id.clone())
            .unwrap();
        manager.commit_spending(reservation.reservation_id).unwrap();
    }

    #[test]
    fn test_rollback_idempotent() {
        let (_temp_dir, manager) = create_test_manager();
        let peer = generate_test_pubkey();

        manager
            .set_peer_spending_limit(peer.clone(), 10000, "daily".to_string())
            .unwrap();

        let reservation = manager
            .try_reserve_spending(peer.clone(), 3000)
            .unwrap();

        // Rollback multiple times should be fine
        manager
            .rollback_spending(reservation.reservation_id.clone())
            .unwrap();
        manager
            .rollback_spending(reservation.reservation_id)
            .unwrap();

        // Limit should only be restored once
        let limit = manager
            .get_peer_spending_limit(peer)
            .unwrap()
            .unwrap();
        assert_eq!(limit.current_spent_sats, 0);
    }
}

