//! Payment Method Selector
//!
//! This module implements the core logic for selecting payment methods.

use super::preferences::{SelectionPreferences, SelectionStrategy};
use crate::methods::{Amount, PaymentMethodPlugin, PaymentMethodRegistry};
use crate::{MethodId, PaykitError, Result, SupportedPayments};
use std::sync::Arc;

/// Result of payment method selection.
#[derive(Clone, Debug)]
pub struct SelectionResult {
    /// The primary selected method.
    pub primary: MethodId,
    /// Fallback methods in priority order.
    pub fallbacks: Vec<MethodId>,
    /// Score for the primary method (higher = better).
    pub score: f64,
    /// Reason for the selection.
    pub reason: String,
}

impl SelectionResult {
    /// Get all methods (primary + fallbacks) in priority order.
    pub fn all_methods(&self) -> Vec<MethodId> {
        let mut methods = vec![self.primary.clone()];
        methods.extend(self.fallbacks.clone());
        methods
    }
}

/// Scored method for internal ranking.
#[derive(Clone)]
struct ScoredMethod {
    method_id: MethodId,
    score: f64,
    plugin: Arc<dyn PaymentMethodPlugin>,
}

impl std::fmt::Debug for ScoredMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScoredMethod")
            .field("method_id", &self.method_id)
            .field("score", &self.score)
            .field("plugin", &self.plugin.display_name())
            .finish()
    }
}

/// Payment method selector.
///
/// The selector chooses the best payment method based on:
/// - Available methods from the payee
/// - User preferences
/// - Amount being paid
/// - Method capabilities and constraints
pub struct PaymentMethodSelector {
    registry: PaymentMethodRegistry,
}

impl PaymentMethodSelector {
    /// Create a new selector with the given registry.
    pub fn new(registry: PaymentMethodRegistry) -> Self {
        Self { registry }
    }

    /// Create a selector with the default registry.
    pub fn with_defaults() -> Self {
        Self {
            registry: crate::methods::default_registry(),
        }
    }

    /// Select the best payment method.
    ///
    /// Returns the primary method and a list of fallback methods.
    pub fn select(
        &self,
        supported: &SupportedPayments,
        amount: &Amount,
        preferences: &SelectionPreferences,
    ) -> Result<SelectionResult> {
        // Get available methods from supported payments
        let available: Vec<MethodId> = supported.entries.keys().cloned().collect();

        if available.is_empty() {
            return Err(PaykitError::Transport(
                "No payment methods available".to_string(),
            ));
        }

        // Score and rank methods
        let scored = self.score_methods(&available, amount, preferences)?;

        if scored.is_empty() {
            return Err(PaykitError::Transport(
                "No suitable payment methods found".to_string(),
            ));
        }

        // Build result
        let primary = scored[0].clone();
        let fallbacks: Vec<MethodId> = scored[1..].iter().map(|s| s.method_id.clone()).collect();

        let reason = self.format_reason(&primary, preferences);

        Ok(SelectionResult {
            primary: primary.method_id,
            fallbacks,
            score: primary.score,
            reason,
        })
    }

    /// Select method with emphasis on having fallbacks.
    ///
    /// Ensures at least one fallback if available.
    pub fn select_with_fallback(
        &self,
        supported: &SupportedPayments,
        amount: &Amount,
        preferences: &SelectionPreferences,
    ) -> Result<SelectionResult> {
        self.select(supported, amount, preferences)
    }

    /// Score and rank methods based on preferences.
    fn score_methods(
        &self,
        available: &[MethodId],
        amount: &Amount,
        preferences: &SelectionPreferences,
    ) -> Result<Vec<ScoredMethod>> {
        let mut scored: Vec<ScoredMethod> = Vec::new();

        for method_id in available {
            // Skip excluded methods
            if preferences.is_excluded(method_id) {
                continue;
            }

            // Get plugin
            let plugin = match self.registry.get(method_id) {
                Some(p) => p,
                None => continue, // Skip unregistered methods
            };

            // Check if method supports the amount
            if !plugin.supports_amount(amount) {
                continue;
            }

            // Check confirmation time constraint
            if let Some(max_time) = preferences.max_confirmation_time_secs {
                if let Some(est_time) = plugin.estimated_confirmation_time() {
                    if est_time > max_time {
                        continue;
                    }
                }
            }

            // Calculate score
            let score = self.calculate_score(&plugin, amount, preferences);

            scored.push(ScoredMethod {
                method_id: method_id.clone(),
                score,
                plugin,
            });
        }

        // Sort by score (descending)
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(scored)
    }

    /// Calculate score for a method.
    fn calculate_score(
        &self,
        plugin: &Arc<dyn PaymentMethodPlugin>,
        amount: &Amount,
        preferences: &SelectionPreferences,
    ) -> f64 {
        let mut score = 50.0; // Base score

        // Apply strategy-specific scoring
        match preferences.strategy {
            SelectionStrategy::Balanced => {
                score += self.score_balanced(plugin, amount, preferences);
            }
            SelectionStrategy::CostOptimized => {
                score += self.score_cost_optimized(plugin, amount);
            }
            SelectionStrategy::SpeedOptimized => {
                score += self.score_speed_optimized(plugin);
            }
            SelectionStrategy::PrivacyOptimized => {
                score += self.score_privacy_optimized(plugin);
            }
            SelectionStrategy::PriorityList => {
                score += self.score_priority_list(plugin, preferences);
            }
        }

        // Apply amount-based adjustments
        score += self.score_amount_fit(plugin, amount, preferences);

        score
    }

    /// Balanced scoring (default strategy).
    fn score_balanced(
        &self,
        plugin: &Arc<dyn PaymentMethodPlugin>,
        amount: &Amount,
        preferences: &SelectionPreferences,
    ) -> f64 {
        let mut score = 0.0;
        let method_id = plugin.method_id();
        let method = method_id.0.as_str();

        // Speed component (up to 15 points)
        if let Some(time) = plugin.estimated_confirmation_time() {
            if time <= 10 {
                score += 15.0; // Instant
            } else if time <= 600 {
                score += 10.0; // Under 10 minutes
            } else if time <= 3600 {
                score += 5.0; // Under 1 hour
            }
        }

        // Amount fit component (up to 15 points)
        if let Some(sats) = amount.as_u64() {
            if method == "lightning" && sats < 100_000 {
                score += 15.0; // Lightning preferred for small amounts
            } else if method == "onchain" && sats >= 100_000 {
                score += 10.0; // On-chain for larger amounts
            }
        }

        // Privacy component (up to 10 points)
        if preferences.prefer_privacy && method == "lightning" {
            score += 10.0; // Lightning is more private
        }

        score
    }

    /// Cost-optimized scoring.
    fn score_cost_optimized(&self, plugin: &Arc<dyn PaymentMethodPlugin>, amount: &Amount) -> f64 {
        let mut score = 0.0;
        let method_id = plugin.method_id();
        let method = method_id.0.as_str();

        // Lightning typically has lower fees for small amounts
        if let Some(sats) = amount.as_u64() {
            if method == "lightning" && sats < 100_000 {
                score += 40.0; // Lightning for small amounts
            } else if method == "onchain" && sats >= 500_000 {
                // On-chain becomes cost-effective for large amounts
                score += 30.0;
            }
        }

        score
    }

    /// Speed-optimized scoring.
    fn score_speed_optimized(&self, plugin: &Arc<dyn PaymentMethodPlugin>) -> f64 {
        if let Some(time) = plugin.estimated_confirmation_time() {
            // Exponential scoring for speed
            if time <= 1 {
                50.0 // Instant
            } else if time <= 10 {
                40.0 // Under 10 seconds
            } else if time <= 60 {
                30.0 // Under 1 minute
            } else if time <= 600 {
                20.0 // Under 10 minutes
            } else {
                10.0 // Slower
            }
        } else {
            0.0 // Unknown speed
        }
    }

    /// Privacy-optimized scoring.
    fn score_privacy_optimized(&self, plugin: &Arc<dyn PaymentMethodPlugin>) -> f64 {
        let method_id = plugin.method_id();
        let method = method_id.0.as_str();

        match method {
            "lightning" => 40.0, // Off-chain, no public record
            "onchain" => 10.0,   // Public blockchain
            _ => 20.0,           // Unknown
        }
    }

    /// Priority list scoring.
    fn score_priority_list(
        &self,
        plugin: &Arc<dyn PaymentMethodPlugin>,
        preferences: &SelectionPreferences,
    ) -> f64 {
        let method = plugin.method_id();

        if let Some(index) = preferences.priority_index(&method) {
            // Higher score for earlier in the list
            let max_priority = preferences.priority_list.len() as f64;
            (max_priority - index as f64) * 10.0
        } else {
            0.0 // Not in priority list
        }
    }

    /// Amount-based fit scoring.
    fn score_amount_fit(
        &self,
        plugin: &Arc<dyn PaymentMethodPlugin>,
        amount: &Amount,
        preferences: &SelectionPreferences,
    ) -> f64 {
        let Some(sats) = amount.as_u64() else {
            return 0.0;
        };

        let method_id = plugin.method_id();
        let method = method_id.0.as_str();
        let thresholds = &preferences.amount_thresholds;

        if method == "lightning" {
            if thresholds.prefers_lightning(sats) {
                10.0
            } else if !thresholds.lightning_viable(sats) {
                -50.0 // Penalize if not viable
            } else {
                0.0
            }
        } else if method == "onchain" {
            if thresholds.prefers_onchain(sats) {
                10.0
            } else if !thresholds.onchain_viable(sats) {
                -50.0 // Penalize if not viable
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Format a human-readable reason for the selection.
    fn format_reason(&self, selected: &ScoredMethod, preferences: &SelectionPreferences) -> String {
        let method_name = selected.plugin.display_name();

        match preferences.strategy {
            SelectionStrategy::Balanced => {
                format!("Selected {} as best balanced option", method_name)
            }
            SelectionStrategy::CostOptimized => {
                format!("Selected {} for lowest fees", method_name)
            }
            SelectionStrategy::SpeedOptimized => {
                format!("Selected {} for fastest confirmation", method_name)
            }
            SelectionStrategy::PrivacyOptimized => {
                format!("Selected {} for best privacy", method_name)
            }
            SelectionStrategy::PriorityList => {
                format!("Selected {} from priority list", method_name)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EndpointData;

    fn create_test_supported() -> SupportedPayments {
        let mut entries = std::collections::HashMap::new();
        entries.insert(MethodId("onchain".into()), EndpointData("bc1q...".into()));
        entries.insert(MethodId("lightning".into()), EndpointData("lnbc...".into()));
        SupportedPayments { entries }
    }

    #[test]
    fn test_select_balanced_small_amount() {
        let selector = PaymentMethodSelector::with_defaults();
        let supported = create_test_supported();
        let amount = Amount::sats(10000); // Small amount
        let prefs = SelectionPreferences::balanced();

        let result = selector.select(&supported, &amount, &prefs).unwrap();

        // Lightning should be preferred for small amounts
        assert_eq!(result.primary.0, "lightning");
        assert!(!result.fallbacks.is_empty());
    }

    #[test]
    fn test_select_balanced_large_amount() {
        let selector = PaymentMethodSelector::with_defaults();
        let supported = create_test_supported();
        let amount = Amount::sats(2_000_000); // Large amount
        let prefs = SelectionPreferences::balanced();

        let result = selector.select(&supported, &amount, &prefs).unwrap();

        // On-chain should be preferred for large amounts
        assert_eq!(result.primary.0, "onchain");
    }

    #[test]
    fn test_select_speed_optimized() {
        let selector = PaymentMethodSelector::with_defaults();
        let supported = create_test_supported();
        let amount = Amount::sats(100_000);
        let prefs = SelectionPreferences::speed_optimized();

        let result = selector.select(&supported, &amount, &prefs).unwrap();

        // Lightning should be selected for speed
        assert_eq!(result.primary.0, "lightning");
    }

    #[test]
    fn test_select_privacy_optimized() {
        let selector = PaymentMethodSelector::with_defaults();
        let supported = create_test_supported();
        let amount = Amount::sats(100_000);
        let prefs = SelectionPreferences::privacy_optimized();

        let result = selector.select(&supported, &amount, &prefs).unwrap();

        // Lightning should be selected for privacy
        assert_eq!(result.primary.0, "lightning");
    }

    #[test]
    fn test_select_with_priority_list() {
        let selector = PaymentMethodSelector::with_defaults();
        let supported = create_test_supported();
        let amount = Amount::sats(100_000);
        let prefs = SelectionPreferences::with_priority_list(vec![
            MethodId("onchain".into()),
            MethodId("lightning".into()),
        ]);

        let result = selector.select(&supported, &amount, &prefs).unwrap();

        // On-chain should be selected due to priority
        assert_eq!(result.primary.0, "onchain");
    }

    #[test]
    fn test_select_with_exclusion() {
        let selector = PaymentMethodSelector::with_defaults();
        let supported = create_test_supported();
        let amount = Amount::sats(10000);
        let prefs = SelectionPreferences::balanced().exclude_method(MethodId("lightning".into()));

        let result = selector.select(&supported, &amount, &prefs).unwrap();

        // On-chain should be selected since lightning is excluded
        assert_eq!(result.primary.0, "onchain");
        assert!(result.fallbacks.is_empty());
    }

    #[test]
    fn test_select_no_methods() {
        let selector = PaymentMethodSelector::with_defaults();
        let supported = SupportedPayments::default();
        let amount = Amount::sats(10000);
        let prefs = SelectionPreferences::balanced();

        let result = selector.select(&supported, &amount, &prefs);
        assert!(result.is_err());
    }

    #[test]
    fn test_selection_result_all_methods() {
        let result = SelectionResult {
            primary: MethodId("lightning".into()),
            fallbacks: vec![MethodId("onchain".into())],
            score: 100.0,
            reason: "Test".into(),
        };

        let all = result.all_methods();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].0, "lightning");
        assert_eq!(all[1].0, "onchain");
    }
}
