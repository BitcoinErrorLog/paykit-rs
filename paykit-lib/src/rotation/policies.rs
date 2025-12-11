//! Endpoint Rotation Policies
//!
//! This module defines policies for when and how to rotate payment endpoints.

use serde::{Deserialize, Serialize};

/// Policy for endpoint rotation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotationPolicy {
    /// Rotate immediately after each use.
    /// Best for privacy (no address reuse).
    RotateOnUse,

    /// Rotate after a specified number of uses.
    /// Balance between privacy and stability.
    RotateOnThreshold {
        /// Number of uses before rotation.
        threshold: u32,
    },

    /// Rotate on a time interval.
    /// Good for scheduled maintenance.
    RotatePeriodic {
        /// Interval in seconds.
        interval_secs: u64,
    },

    /// Never rotate automatically.
    /// Manual rotation only.
    Manual,
}

impl Default for RotationPolicy {
    fn default() -> Self {
        // Default to rotating after each use for privacy
        Self::RotateOnUse
    }
}

impl RotationPolicy {
    /// Create a policy that rotates after N uses.
    pub fn after_uses(n: u32) -> Self {
        Self::RotateOnThreshold { threshold: n }
    }

    /// Create a policy that rotates every N hours.
    pub fn every_hours(hours: u64) -> Self {
        Self::RotatePeriodic {
            interval_secs: hours * 3600,
        }
    }

    /// Create a policy that rotates every N days.
    pub fn every_days(days: u64) -> Self {
        Self::RotatePeriodic {
            interval_secs: days * 86400,
        }
    }

    /// Check if rotation is needed based on usage count.
    pub fn should_rotate_on_use(&self, use_count: u32) -> bool {
        match self {
            Self::RotateOnUse => true,
            Self::RotateOnThreshold { threshold } => use_count >= *threshold,
            _ => false,
        }
    }

    /// Check if rotation is needed based on time.
    pub fn should_rotate_on_time(&self, last_rotation_secs: i64, now_secs: i64) -> bool {
        match self {
            Self::RotatePeriodic { interval_secs } => {
                let elapsed = now_secs.saturating_sub(last_rotation_secs) as u64;
                elapsed >= *interval_secs
            }
            _ => false,
        }
    }
}

/// Tracking information for an endpoint.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EndpointTracker {
    /// Number of times this endpoint has been used.
    pub use_count: u32,
    /// Timestamp when the endpoint was created/last rotated.
    pub created_at: i64,
    /// Timestamp of last use.
    pub last_used_at: Option<i64>,
    /// Whether rotation is pending.
    pub rotation_pending: bool,
}

impl EndpointTracker {
    /// Create a new tracker with current timestamp.
    pub fn new() -> Self {
        Self {
            use_count: 0,
            created_at: current_timestamp(),
            last_used_at: None,
            rotation_pending: false,
        }
    }

    /// Record a use of the endpoint.
    pub fn record_use(&mut self) {
        self.use_count = self.use_count.saturating_add(1);
        self.last_used_at = Some(current_timestamp());
    }

    /// Check if rotation is needed based on policy.
    pub fn needs_rotation(&self, policy: &RotationPolicy) -> bool {
        if self.rotation_pending {
            return true;
        }

        let now = current_timestamp();

        policy.should_rotate_on_use(self.use_count)
            || policy.should_rotate_on_time(self.created_at, now)
    }

    /// Mark rotation as pending.
    pub fn mark_pending(&mut self) {
        self.rotation_pending = true;
    }

    /// Reset after rotation.
    pub fn reset(&mut self) {
        self.use_count = 0;
        self.created_at = current_timestamp();
        self.last_used_at = None;
        self.rotation_pending = false;
    }
}

/// Helper function to get current timestamp.
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

    #[test]
    fn test_default_policy() {
        let policy = RotationPolicy::default();
        assert_eq!(policy, RotationPolicy::RotateOnUse);
    }

    #[test]
    fn test_rotate_on_use() {
        let policy = RotationPolicy::RotateOnUse;
        assert!(policy.should_rotate_on_use(0));
        assert!(policy.should_rotate_on_use(1));
        assert!(policy.should_rotate_on_use(100));
    }

    #[test]
    fn test_rotate_on_threshold() {
        let policy = RotationPolicy::after_uses(3);
        assert!(!policy.should_rotate_on_use(0));
        assert!(!policy.should_rotate_on_use(1));
        assert!(!policy.should_rotate_on_use(2));
        assert!(policy.should_rotate_on_use(3));
        assert!(policy.should_rotate_on_use(4));
    }

    #[test]
    fn test_rotate_periodic() {
        let policy = RotationPolicy::every_hours(1);
        let now = current_timestamp();

        // Just created - no rotation
        assert!(!policy.should_rotate_on_time(now, now));

        // 30 minutes later - no rotation
        assert!(!policy.should_rotate_on_time(now, now + 1800));

        // 1 hour later - should rotate
        assert!(policy.should_rotate_on_time(now, now + 3600));

        // 2 hours later - should rotate
        assert!(policy.should_rotate_on_time(now, now + 7200));
    }

    #[test]
    fn test_endpoint_tracker() {
        let mut tracker = EndpointTracker::new();
        assert_eq!(tracker.use_count, 0);
        assert!(tracker.last_used_at.is_none());

        tracker.record_use();
        assert_eq!(tracker.use_count, 1);
        assert!(tracker.last_used_at.is_some());

        tracker.record_use();
        assert_eq!(tracker.use_count, 2);
    }

    #[test]
    fn test_tracker_needs_rotation() {
        let mut tracker = EndpointTracker::new();

        // With RotateOnUse - needs rotation after any use
        let policy = RotationPolicy::RotateOnUse;
        assert!(tracker.needs_rotation(&policy)); // Even 0 uses triggers

        tracker.record_use();
        assert!(tracker.needs_rotation(&policy));

        // With threshold policy
        let policy = RotationPolicy::after_uses(3);
        tracker.reset();
        assert!(!tracker.needs_rotation(&policy));

        tracker.record_use();
        tracker.record_use();
        assert!(!tracker.needs_rotation(&policy));

        tracker.record_use();
        assert!(tracker.needs_rotation(&policy));
    }

    #[test]
    fn test_tracker_reset() {
        let mut tracker = EndpointTracker::new();
        tracker.record_use();
        tracker.record_use();
        tracker.mark_pending();

        assert_eq!(tracker.use_count, 2);
        assert!(tracker.rotation_pending);

        tracker.reset();
        assert_eq!(tracker.use_count, 0);
        assert!(!tracker.rotation_pending);
    }
}
