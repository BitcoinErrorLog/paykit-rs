//! Payment Method Selection Preferences
//!
//! This module defines user preferences for payment method selection.

use crate::MethodId;
use serde::{Deserialize, Serialize};

/// Strategy for selecting payment methods.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionStrategy {
    /// Balance between cost, speed, and privacy (default).
    #[default]
    Balanced,
    /// Minimize transaction fees.
    CostOptimized,
    /// Prioritize fastest confirmation.
    SpeedOptimized,
    /// Maximize privacy.
    PrivacyOptimized,
    /// Use methods in the order specified by priority list.
    PriorityList,
}

/// User preferences for payment method selection.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SelectionPreferences {
    /// The selection strategy to use.
    pub strategy: SelectionStrategy,
    /// Ordered list of preferred methods (highest priority first).
    /// If empty, all available methods are considered.
    pub priority_list: Vec<MethodId>,
    /// Methods to exclude from selection.
    pub excluded_methods: Vec<MethodId>,
    /// Maximum acceptable fee in satoshis (None = no limit).
    pub max_fee_sats: Option<u64>,
    /// Maximum acceptable confirmation time in seconds (None = no limit).
    pub max_confirmation_time_secs: Option<u64>,
    /// Prefer methods that don't reuse public addresses.
    pub prefer_privacy: bool,
    /// Amount thresholds for method selection (in satoshis).
    pub amount_thresholds: AmountThresholds,
}

impl SelectionPreferences {
    /// Create preferences with the Balanced strategy.
    pub fn balanced() -> Self {
        Self {
            strategy: SelectionStrategy::Balanced,
            ..Default::default()
        }
    }

    /// Create preferences optimized for low fees.
    pub fn cost_optimized() -> Self {
        Self {
            strategy: SelectionStrategy::CostOptimized,
            prefer_privacy: false,
            ..Default::default()
        }
    }

    /// Create preferences optimized for speed.
    pub fn speed_optimized() -> Self {
        Self {
            strategy: SelectionStrategy::SpeedOptimized,
            max_confirmation_time_secs: Some(60), // 1 minute
            ..Default::default()
        }
    }

    /// Create preferences optimized for privacy.
    pub fn privacy_optimized() -> Self {
        Self {
            strategy: SelectionStrategy::PrivacyOptimized,
            prefer_privacy: true,
            ..Default::default()
        }
    }

    /// Create preferences with a specific priority list.
    pub fn with_priority_list(methods: Vec<MethodId>) -> Self {
        Self {
            strategy: SelectionStrategy::PriorityList,
            priority_list: methods,
            ..Default::default()
        }
    }

    /// Add a method to the exclusion list.
    pub fn exclude_method(mut self, method: MethodId) -> Self {
        if !self.excluded_methods.contains(&method) {
            self.excluded_methods.push(method);
        }
        self
    }

    /// Set maximum acceptable fee.
    pub fn with_max_fee(mut self, max_fee_sats: u64) -> Self {
        self.max_fee_sats = Some(max_fee_sats);
        self
    }

    /// Set maximum acceptable confirmation time.
    pub fn with_max_confirmation_time(mut self, max_secs: u64) -> Self {
        self.max_confirmation_time_secs = Some(max_secs);
        self
    }

    /// Check if a method is excluded.
    pub fn is_excluded(&self, method: &MethodId) -> bool {
        self.excluded_methods.iter().any(|m| m.0 == method.0)
    }

    /// Get priority index for a method (lower = higher priority).
    pub fn priority_index(&self, method: &MethodId) -> Option<usize> {
        self.priority_list.iter().position(|m| m.0 == method.0)
    }
}

/// Amount thresholds for automatic method selection.
///
/// These thresholds help the selector choose appropriate methods
/// based on the payment amount.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AmountThresholds {
    /// Below this amount (in sats), prefer Lightning over on-chain.
    /// Default: 100,000 sats (~$40 at $40k/BTC)
    pub lightning_preferred_below: u64,
    /// Above this amount (in sats), prefer on-chain over Lightning.
    /// Default: 1,000,000 sats (~$400 at $40k/BTC)
    pub onchain_preferred_above: u64,
    /// Minimum amount for on-chain (dust limit).
    /// Default: 546 sats
    pub onchain_minimum: u64,
    /// Maximum amount for Lightning (channel capacity limits).
    /// Default: 4,000,000 sats (~$1600 at $40k/BTC)
    pub lightning_maximum: u64,
}

impl Default for AmountThresholds {
    fn default() -> Self {
        Self {
            lightning_preferred_below: 100_000,
            onchain_preferred_above: 1_000_000,
            onchain_minimum: 546,
            lightning_maximum: 4_000_000,
        }
    }
}

impl AmountThresholds {
    /// Check if Lightning is preferred for the given amount.
    pub fn prefers_lightning(&self, amount_sats: u64) -> bool {
        amount_sats < self.lightning_preferred_below && amount_sats <= self.lightning_maximum
    }

    /// Check if on-chain is preferred for the given amount.
    pub fn prefers_onchain(&self, amount_sats: u64) -> bool {
        amount_sats >= self.onchain_preferred_above && amount_sats >= self.onchain_minimum
    }

    /// Check if amount is within Lightning limits.
    pub fn lightning_viable(&self, amount_sats: u64) -> bool {
        amount_sats >= 1 && amount_sats <= self.lightning_maximum
    }

    /// Check if amount is within on-chain limits.
    pub fn onchain_viable(&self, amount_sats: u64) -> bool {
        amount_sats >= self.onchain_minimum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_preferences() {
        let prefs = SelectionPreferences::default();
        assert_eq!(prefs.strategy, SelectionStrategy::Balanced);
        assert!(prefs.priority_list.is_empty());
        assert!(prefs.excluded_methods.is_empty());
    }

    #[test]
    fn test_cost_optimized() {
        let prefs = SelectionPreferences::cost_optimized();
        assert_eq!(prefs.strategy, SelectionStrategy::CostOptimized);
    }

    #[test]
    fn test_speed_optimized() {
        let prefs = SelectionPreferences::speed_optimized();
        assert_eq!(prefs.strategy, SelectionStrategy::SpeedOptimized);
        assert_eq!(prefs.max_confirmation_time_secs, Some(60));
    }

    #[test]
    fn test_privacy_optimized() {
        let prefs = SelectionPreferences::privacy_optimized();
        assert_eq!(prefs.strategy, SelectionStrategy::PrivacyOptimized);
        assert!(prefs.prefer_privacy);
    }

    #[test]
    fn test_priority_list() {
        let prefs = SelectionPreferences::with_priority_list(vec![
            MethodId("lightning".into()),
            MethodId("onchain".into()),
        ]);
        assert_eq!(prefs.strategy, SelectionStrategy::PriorityList);
        assert_eq!(prefs.priority_list.len(), 2);
        assert_eq!(prefs.priority_index(&MethodId("lightning".into())), Some(0));
        assert_eq!(prefs.priority_index(&MethodId("onchain".into())), Some(1));
    }

    #[test]
    fn test_exclude_method() {
        let prefs = SelectionPreferences::balanced().exclude_method(MethodId("onchain".into()));
        assert!(prefs.is_excluded(&MethodId("onchain".into())));
        assert!(!prefs.is_excluded(&MethodId("lightning".into())));
    }

    #[test]
    fn test_amount_thresholds() {
        let thresholds = AmountThresholds::default();

        // Small amount - prefer Lightning
        assert!(thresholds.prefers_lightning(1000));
        assert!(!thresholds.prefers_onchain(1000));

        // Large amount - prefer on-chain
        assert!(thresholds.prefers_onchain(2_000_000));
        assert!(!thresholds.prefers_lightning(2_000_000));

        // Lightning viable range
        assert!(thresholds.lightning_viable(1));
        assert!(thresholds.lightning_viable(1_000_000));
        assert!(!thresholds.lightning_viable(5_000_000));

        // On-chain viable range
        assert!(thresholds.onchain_viable(1000));
        assert!(!thresholds.onchain_viable(100));
    }
}
