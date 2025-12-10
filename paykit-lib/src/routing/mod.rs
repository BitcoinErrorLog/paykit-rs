//! Payment Routing Hints and Fallback Chains
//!
//! This module provides routing hints for payment method selection
//! and automatic fallback execution when primary methods fail.

use crate::methods::{Amount, PaymentExecution, PaymentMethodRegistry};
use crate::{EndpointData, MethodId, PaykitError, Result, SupportedPayments};
use serde::{Deserialize, Serialize};

/// A routing hint for a specific payment method.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutingHint {
    /// The payment method.
    pub method: MethodId,
    /// The endpoint data for this method.
    pub endpoint: EndpointData,
    /// Priority (lower = higher priority).
    pub priority: u8,
    /// Estimated cost in satoshis (for comparison).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost_sats: Option<u64>,
    /// Estimated confirmation time in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_time_secs: Option<u64>,
}

impl RoutingHint {
    /// Create a new routing hint.
    pub fn new(method: MethodId, endpoint: EndpointData, priority: u8) -> Self {
        Self {
            method,
            endpoint,
            priority,
            estimated_cost_sats: None,
            estimated_time_secs: None,
        }
    }

    /// Set estimated cost.
    pub fn with_estimated_cost(mut self, cost_sats: u64) -> Self {
        self.estimated_cost_sats = Some(cost_sats);
        self
    }

    /// Set estimated time.
    pub fn with_estimated_time(mut self, time_secs: u64) -> Self {
        self.estimated_time_secs = Some(time_secs);
        self
    }
}

/// Complete routing information for a payment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutingInfo {
    /// The primary/preferred method.
    pub primary_method: MethodId,
    /// Primary endpoint.
    pub primary_endpoint: EndpointData,
    /// Routing hints for fallback methods.
    #[serde(default)]
    pub routing_hints: Vec<RoutingHint>,
    /// Ordered fallback chain.
    #[serde(default)]
    pub fallback_chain: Vec<MethodId>,
}

impl RoutingInfo {
    /// Create routing info with a single method.
    pub fn single(method: MethodId, endpoint: EndpointData) -> Self {
        Self {
            primary_method: method,
            primary_endpoint: endpoint,
            routing_hints: Vec::new(),
            fallback_chain: Vec::new(),
        }
    }

    /// Add a routing hint.
    pub fn add_hint(mut self, hint: RoutingHint) -> Self {
        // Also add to fallback chain if not present
        if !self.fallback_chain.iter().any(|m| m.0 == hint.method.0) {
            self.fallback_chain.push(hint.method.clone());
        }
        self.routing_hints.push(hint);
        self
    }

    /// Get all methods in priority order.
    pub fn all_methods(&self) -> Vec<MethodId> {
        let mut methods = vec![self.primary_method.clone()];
        methods.extend(self.fallback_chain.clone());
        methods
    }

    /// Get endpoint for a specific method.
    pub fn get_endpoint(&self, method_id: &MethodId) -> Option<EndpointData> {
        if method_id.0 == self.primary_method.0 {
            return Some(self.primary_endpoint.clone());
        }
        
        self.routing_hints
            .iter()
            .find(|h| h.method.0 == method_id.0)
            .map(|h| h.endpoint.clone())
    }
}

/// Builder for generating routing information.
pub struct RoutingHintGenerator {
    registry: PaymentMethodRegistry,
}

impl RoutingHintGenerator {
    /// Create a new generator.
    pub fn new(registry: PaymentMethodRegistry) -> Self {
        Self { registry }
    }

    /// Create with default registry.
    pub fn with_defaults() -> Self {
        Self {
            registry: crate::methods::default_registry(),
        }
    }

    /// Generate routing info from supported payments.
    pub fn generate(&self, supported: &SupportedPayments, amount: &Amount) -> Result<RoutingInfo> {
        let mut hints: Vec<(MethodId, EndpointData, u8, Option<u64>)> = Vec::new();

        for (method_id, endpoint) in &supported.entries {
            if let Some(plugin) = self.registry.get(method_id) {
                // Check if method supports the amount
                if !plugin.supports_amount(amount) {
                    continue;
                }

                // Calculate priority based on method characteristics
                let est_time = plugin.estimated_confirmation_time();
                let priority = match est_time {
                    Some(t) if t <= 10 => 1,    // Fast (Lightning)
                    Some(t) if t <= 600 => 2,   // Medium
                    _ => 3,                      // Slow (on-chain)
                };

                hints.push((method_id.clone(), endpoint.clone(), priority, est_time));
            }
        }

        if hints.is_empty() {
            return Err(PaykitError::Transport(
                "No compatible payment methods available".to_string()
            ));
        }

        // Sort by priority
        hints.sort_by_key(|(_, _, p, _)| *p);

        // Build routing info
        let (primary_method, primary_endpoint, _, _) = hints.remove(0);
        
        let routing_hints: Vec<RoutingHint> = hints
            .into_iter()
            .map(|(method, endpoint, priority, est_time)| {
                let mut hint = RoutingHint::new(method, endpoint, priority);
                if let Some(t) = est_time {
                    hint = hint.with_estimated_time(t);
                }
                hint
            })
            .collect();

        let fallback_chain: Vec<MethodId> = routing_hints.iter()
            .map(|h| h.method.clone())
            .collect();

        Ok(RoutingInfo {
            primary_method,
            primary_endpoint,
            routing_hints,
            fallback_chain,
        })
    }
}

/// Configuration for fallback execution.
#[derive(Clone, Debug)]
pub struct FallbackConfig {
    /// Maximum number of fallback attempts.
    pub max_attempts: u8,
    /// Timeout per attempt in milliseconds.
    pub timeout_ms: u64,
    /// Whether to continue to next fallback on timeout.
    pub continue_on_timeout: bool,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            timeout_ms: 30000,
            continue_on_timeout: true,
        }
    }
}

/// Result of a fallback execution attempt.
#[derive(Clone, Debug)]
pub struct FallbackAttempt {
    /// The method that was tried.
    pub method: MethodId,
    /// Whether this attempt succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Execution result if succeeded.
    pub execution: Option<PaymentExecution>,
}

/// Executor for payments with automatic fallback.
pub struct FallbackExecutor {
    config: FallbackConfig,
    registry: PaymentMethodRegistry,
}

impl FallbackExecutor {
    /// Create a new executor.
    pub fn new(config: FallbackConfig, registry: PaymentMethodRegistry) -> Self {
        Self { config, registry }
    }

    /// Create with defaults.
    pub fn with_defaults() -> Self {
        Self {
            config: FallbackConfig::default(),
            registry: crate::methods::default_registry(),
        }
    }

    /// Execute payment with automatic fallback.
    ///
    /// Tries each method in the routing info until one succeeds or all fail.
    pub async fn execute(
        &self,
        routing: &RoutingInfo,
        amount: &Amount,
        metadata: &serde_json::Value,
    ) -> (bool, Vec<FallbackAttempt>) {
        let mut attempts = Vec::new();
        let methods = routing.all_methods();

        for (i, method_id) in methods.iter().enumerate() {
            if i >= self.config.max_attempts as usize {
                break;
            }

            let endpoint = match routing.get_endpoint(method_id) {
                Some(e) => e,
                None => {
                    attempts.push(FallbackAttempt {
                        method: method_id.clone(),
                        success: false,
                        error: Some("Endpoint not found".to_string()),
                        execution: None,
                    });
                    continue;
                }
            };

            let plugin = match self.registry.get(method_id) {
                Some(p) => p,
                None => {
                    attempts.push(FallbackAttempt {
                        method: method_id.clone(),
                        success: false,
                        error: Some("Plugin not registered".to_string()),
                        execution: None,
                    });
                    continue;
                }
            };

            match plugin.execute_payment(&endpoint, amount, metadata).await {
                Ok(execution) if execution.success => {
                    attempts.push(FallbackAttempt {
                        method: method_id.clone(),
                        success: true,
                        error: None,
                        execution: Some(execution),
                    });
                    return (true, attempts);
                }
                Ok(execution) => {
                    attempts.push(FallbackAttempt {
                        method: method_id.clone(),
                        success: false,
                        error: execution.error.clone(),
                        execution: Some(execution),
                    });
                }
                Err(e) => {
                    attempts.push(FallbackAttempt {
                        method: method_id.clone(),
                        success: false,
                        error: Some(e.to_string()),
                        execution: None,
                    });
                }
            }
        }

        (false, attempts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_supported() -> SupportedPayments {
        let mut entries = std::collections::HashMap::new();
        entries.insert(
            MethodId("onchain".into()),
            EndpointData("bc1q...".into()),
        );
        entries.insert(
            MethodId("lightning".into()),
            EndpointData("lnbc...".into()),
        );
        SupportedPayments { entries }
    }

    #[test]
    fn test_routing_hint() {
        let hint = RoutingHint::new(
            MethodId("lightning".into()),
            EndpointData("lnbc...".into()),
            1,
        )
        .with_estimated_cost(10)
        .with_estimated_time(1);

        assert_eq!(hint.priority, 1);
        assert_eq!(hint.estimated_cost_sats, Some(10));
        assert_eq!(hint.estimated_time_secs, Some(1));
    }

    #[test]
    fn test_routing_info_single() {
        let info = RoutingInfo::single(
            MethodId("lightning".into()),
            EndpointData("lnbc...".into()),
        );

        assert_eq!(info.primary_method.0, "lightning");
        assert!(info.fallback_chain.is_empty());
    }

    #[test]
    fn test_routing_info_with_hints() {
        let info = RoutingInfo::single(
            MethodId("lightning".into()),
            EndpointData("lnbc...".into()),
        )
        .add_hint(RoutingHint::new(
            MethodId("onchain".into()),
            EndpointData("bc1q...".into()),
            2,
        ));

        assert_eq!(info.all_methods().len(), 2);
        assert!(info.get_endpoint(&MethodId("onchain".into())).is_some());
    }

    #[test]
    fn test_hint_generator() {
        let generator = RoutingHintGenerator::with_defaults();
        let supported = create_test_supported();
        let amount = Amount::sats(10000);

        let result = generator.generate(&supported, &amount);
        assert!(result.is_ok());

        let routing = result.unwrap();
        // Lightning should be primary (faster)
        assert_eq!(routing.primary_method.0, "lightning");
        assert!(!routing.fallback_chain.is_empty());
    }

    #[test]
    fn test_fallback_config() {
        let config = FallbackConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert!(config.continue_on_timeout);
    }
}
