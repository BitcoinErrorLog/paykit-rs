//! Unified Payment Status System
//!
//! This module provides tracking and notification for payment status
//! across all payment methods.
//!
//! # Thread Safety
//!
//! The status tracker uses `RwLock` for thread-safe access. Public methods
//! will panic if the internal lock is poisoned (which only happens if a thread
//! panics while holding the lock).

use crate::PaykitReceipt;
use paykit_lib::MethodId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Payment status states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    /// Payment has been initiated but not yet confirmed.
    Pending,
    /// Payment is being processed (e.g., broadcast but unconfirmed).
    Processing,
    /// Payment has been confirmed on the network.
    Confirmed,
    /// Payment has reached final confirmation (e.g., 6 blocks for Bitcoin).
    Finalized,
    /// Payment failed.
    Failed,
    /// Payment was cancelled.
    Cancelled,
    /// Payment expired before completion.
    Expired,
}

impl PaymentStatus {
    /// Check if this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Finalized | Self::Failed | Self::Cancelled | Self::Expired
        )
    }

    /// Check if payment is still in progress.
    pub fn is_in_progress(&self) -> bool {
        matches!(self, Self::Pending | Self::Processing | Self::Confirmed)
    }

    /// Check if payment succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Confirmed | Self::Finalized)
    }
}

/// Extended status information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentStatusInfo {
    /// The current status.
    pub status: PaymentStatus,
    /// Receipt ID this status belongs to.
    pub receipt_id: String,
    /// Method used for payment.
    pub method_id: MethodId,
    /// Timestamp when status was last updated.
    pub updated_at: i64,
    /// Number of confirmations (for blockchain payments).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmations: Option<u64>,
    /// Required confirmations for finality.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_confirmations: Option<u64>,
    /// Error message if status is Failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Additional status details.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub details: serde_json::Value,
}

impl PaymentStatusInfo {
    /// Create a new pending status.
    pub fn pending(receipt_id: impl Into<String>, method_id: MethodId) -> Self {
        Self {
            status: PaymentStatus::Pending,
            receipt_id: receipt_id.into(),
            method_id,
            updated_at: current_timestamp(),
            confirmations: None,
            required_confirmations: None,
            error: None,
            details: serde_json::Value::Null,
        }
    }

    /// Update the status.
    pub fn update(&mut self, new_status: PaymentStatus) {
        self.status = new_status;
        self.updated_at = current_timestamp();
    }

    /// Update confirmations.
    pub fn update_confirmations(&mut self, confirmations: u64, required: u64) {
        self.confirmations = Some(confirmations);
        self.required_confirmations = Some(required);

        if confirmations >= required {
            self.status = PaymentStatus::Finalized;
        } else if confirmations > 0 {
            self.status = PaymentStatus::Confirmed;
        }
        self.updated_at = current_timestamp();
    }

    /// Mark as failed.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = PaymentStatus::Failed;
        self.error = Some(error.into());
        self.updated_at = current_timestamp();
    }

    /// Calculate progress percentage.
    pub fn progress_percentage(&self) -> f64 {
        match self.status {
            PaymentStatus::Pending => 0.0,
            PaymentStatus::Processing => 25.0,
            PaymentStatus::Confirmed => {
                if let (Some(confs), Some(required)) =
                    (self.confirmations, self.required_confirmations)
                {
                    50.0 + (confs as f64 / required as f64) * 50.0
                } else {
                    75.0
                }
            }
            PaymentStatus::Finalized => 100.0,
            _ => 0.0,
        }
    }
}

/// Callback for status changes.
pub type StatusCallback = Arc<dyn Fn(&PaymentStatusInfo) + Send + Sync>;

/// Tracker for payment statuses.
pub struct PaymentStatusTracker {
    /// Status storage by receipt ID.
    statuses: RwLock<HashMap<String, PaymentStatusInfo>>,
    /// Callbacks for status changes.
    callbacks: RwLock<Vec<StatusCallback>>,
}

impl PaymentStatusTracker {
    /// Create a new tracker.
    pub fn new() -> Self {
        Self {
            statuses: RwLock::new(HashMap::new()),
            callbacks: RwLock::new(Vec::new()),
        }
    }

    /// Register a callback for status changes.
    pub fn on_status_change(&self, callback: StatusCallback) {
        let mut callbacks = self
            .callbacks
            .write()
            .unwrap_or_else(|e| e.into_inner());
        callbacks.push(callback);
    }

    /// Create a new pending status for a receipt.
    pub fn track(&self, receipt: &PaykitReceipt) {
        let status = PaymentStatusInfo::pending(&receipt.receipt_id, receipt.method_id.clone());

        {
            let mut statuses = self
                .statuses
                .write()
                .unwrap_or_else(|e| e.into_inner());
            statuses.insert(receipt.receipt_id.clone(), status.clone());
        }

        self.notify(&status);
    }

    /// Get status for a receipt.
    pub fn get(&self, receipt_id: &str) -> Option<PaymentStatusInfo> {
        let statuses = self
            .statuses
            .read()
            .unwrap_or_else(|e| e.into_inner());
        statuses.get(receipt_id).cloned()
    }

    /// Update status for a receipt.
    pub fn update(&self, receipt_id: &str, new_status: PaymentStatus) -> Option<PaymentStatusInfo> {
        let mut statuses = self
            .statuses
            .write()
            .unwrap_or_else(|e| e.into_inner());

        if let Some(status) = statuses.get_mut(receipt_id) {
            status.update(new_status);
            let status_clone = status.clone();
            drop(statuses);
            self.notify(&status_clone);
            Some(status_clone)
        } else {
            None
        }
    }

    /// Update confirmations for a receipt.
    pub fn update_confirmations(
        &self,
        receipt_id: &str,
        confirmations: u64,
        required: u64,
    ) -> Option<PaymentStatusInfo> {
        let mut statuses = self
            .statuses
            .write()
            .unwrap_or_else(|e| e.into_inner());

        if let Some(status) = statuses.get_mut(receipt_id) {
            status.update_confirmations(confirmations, required);
            let status_clone = status.clone();
            drop(statuses);
            self.notify(&status_clone);
            Some(status_clone)
        } else {
            None
        }
    }

    /// Mark a payment as failed.
    pub fn mark_failed(
        &self,
        receipt_id: &str,
        error: impl Into<String>,
    ) -> Option<PaymentStatusInfo> {
        let mut statuses = self
            .statuses
            .write()
            .unwrap_or_else(|e| e.into_inner());

        if let Some(status) = statuses.get_mut(receipt_id) {
            status.mark_failed(error);
            let status_clone = status.clone();
            drop(statuses);
            self.notify(&status_clone);
            Some(status_clone)
        } else {
            None
        }
    }

    /// Get all pending/in-progress payments.
    pub fn get_in_progress(&self) -> Vec<PaymentStatusInfo> {
        let statuses = self
            .statuses
            .read()
            .unwrap_or_else(|e| e.into_inner());
        statuses
            .values()
            .filter(|s| s.status.is_in_progress())
            .cloned()
            .collect()
    }

    /// Get all payments by status.
    pub fn get_by_status(&self, status: PaymentStatus) -> Vec<PaymentStatusInfo> {
        let statuses = self
            .statuses
            .read()
            .unwrap_or_else(|e| e.into_inner());
        statuses
            .values()
            .filter(|s| s.status == status)
            .cloned()
            .collect()
    }

    /// Remove completed/terminal statuses older than the given timestamp.
    pub fn cleanup_old(&self, before_timestamp: i64) -> usize {
        let mut statuses = self
            .statuses
            .write()
            .unwrap_or_else(|e| e.into_inner());
        let count = statuses.len();
        statuses.retain(|_, status| {
            !status.status.is_terminal() || status.updated_at >= before_timestamp
        });
        count - statuses.len()
    }

    fn notify(&self, status: &PaymentStatusInfo) {
        let callbacks = self
            .callbacks
            .read()
            .unwrap_or_else(|e| e.into_inner());
        for callback in callbacks.iter() {
            callback(status);
        }
    }
}

impl Default for PaymentStatusTracker {
    fn default() -> Self {
        Self::new()
    }
}

fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use paykit_lib::PublicKey;
    use std::str::FromStr;

    fn test_pubkey() -> PublicKey {
        // Use pubky's Keypair for testing
        use pubky::Keypair;
        let keypair = Keypair::random();
        PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
    }

    fn test_receipt() -> PaykitReceipt {
        PaykitReceipt::new(
            "test_receipt_123".to_string(),
            test_pubkey(),
            test_pubkey(),
            MethodId("lightning".to_string()),
            Some("1000".to_string()),
            Some("SAT".to_string()),
            serde_json::json!({}),
        )
    }

    #[test]
    fn test_status_states() {
        assert!(PaymentStatus::Pending.is_in_progress());
        assert!(PaymentStatus::Processing.is_in_progress());
        assert!(PaymentStatus::Confirmed.is_in_progress());
        assert!(!PaymentStatus::Finalized.is_in_progress());

        assert!(PaymentStatus::Finalized.is_terminal());
        assert!(PaymentStatus::Failed.is_terminal());
        assert!(!PaymentStatus::Pending.is_terminal());

        assert!(PaymentStatus::Confirmed.is_success());
        assert!(PaymentStatus::Finalized.is_success());
        assert!(!PaymentStatus::Failed.is_success());
    }

    #[test]
    fn test_status_info() {
        let mut info = PaymentStatusInfo::pending("rcpt_1", MethodId("onchain".to_string()));
        assert_eq!(info.status, PaymentStatus::Pending);
        assert_eq!(info.progress_percentage(), 0.0);

        info.update(PaymentStatus::Processing);
        assert_eq!(info.status, PaymentStatus::Processing);

        info.update_confirmations(3, 6);
        assert_eq!(info.status, PaymentStatus::Confirmed);
        assert!(info.progress_percentage() > 50.0);

        info.update_confirmations(6, 6);
        assert_eq!(info.status, PaymentStatus::Finalized);
        assert_eq!(info.progress_percentage(), 100.0);
    }

    #[test]
    fn test_tracker_basic() {
        let tracker = PaymentStatusTracker::new();
        let receipt = test_receipt();

        tracker.track(&receipt);

        let status = tracker.get(&receipt.receipt_id);
        assert!(status.is_some());
        assert_eq!(status.unwrap().status, PaymentStatus::Pending);
    }

    #[test]
    fn test_tracker_update() {
        let tracker = PaymentStatusTracker::new();
        let receipt = test_receipt();

        tracker.track(&receipt);
        tracker.update(&receipt.receipt_id, PaymentStatus::Processing);

        let status = tracker.get(&receipt.receipt_id).unwrap();
        assert_eq!(status.status, PaymentStatus::Processing);
    }

    #[test]
    fn test_tracker_confirmations() {
        let tracker = PaymentStatusTracker::new();
        let receipt = test_receipt();

        tracker.track(&receipt);
        tracker.update_confirmations(&receipt.receipt_id, 3, 6);

        let status = tracker.get(&receipt.receipt_id).unwrap();
        assert_eq!(status.status, PaymentStatus::Confirmed);
        assert_eq!(status.confirmations, Some(3));
    }

    #[test]
    fn test_tracker_callbacks() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let tracker = PaymentStatusTracker::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        tracker.on_status_change(Arc::new(move |_status| {
            called_clone.store(true, Ordering::SeqCst);
        }));

        let receipt = test_receipt();
        tracker.track(&receipt);

        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_get_in_progress() {
        let tracker = PaymentStatusTracker::new();
        let receipt1 = test_receipt();
        let mut receipt2 = test_receipt();
        receipt2.receipt_id = "receipt_2".to_string();

        tracker.track(&receipt1);
        tracker.track(&receipt2);
        tracker.update(&receipt2.receipt_id, PaymentStatus::Finalized);

        let in_progress = tracker.get_in_progress();
        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].receipt_id, receipt1.receipt_id);
    }
}
