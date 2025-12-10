//! Subscription Fallback Chains
//!
//! This module provides fallback payment method handling for subscriptions,
//! enabling automatic retry with alternative payment methods when the primary
//! method fails.
//!
//! # Overview
//!
//! When a subscription payment fails, the fallback system:
//! 1. Tries alternative payment methods in priority order
//! 2. Respects configurable retry policies
//! 3. Records failed attempts for later analysis
//! 4. Notifies on success or final failure
//!
//! # Example
//!
//! ```ignore
//! use paykit_subscriptions::fallback::{SubscriptionFallbackPolicy, FallbackHandler};
//!
//! let policy = SubscriptionFallbackPolicy::default()
//!     .with_method("lightning".into(), 1)
//!     .with_method("onchain".into(), 2);
//!
//! let handler = FallbackHandler::new(policy, registry);
//! let result = handler.execute_with_fallback(&subscription, &endpoints).await;
//! ```

use crate::{Result, Subscription, SubscriptionError};
use paykit_lib::routing::{FallbackAttempt, FallbackConfig, RoutingHint, RoutingInfo};
use paykit_lib::{EndpointData, MethodId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Policy for subscription payment fallback.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscriptionFallbackPolicy {
    /// Ordered list of fallback methods with priorities.
    /// Lower priority number = tried first.
    pub method_priorities: Vec<(MethodId, u8)>,
    /// Maximum number of methods to try before giving up.
    pub max_methods: u8,
    /// Maximum retries per method.
    pub max_retries_per_method: u8,
    /// Delay between retries in seconds.
    pub retry_delay_secs: u64,
    /// Whether to notify on fallback activation.
    pub notify_on_fallback: bool,
    /// Grace period before marking subscription as failed (in seconds).
    pub grace_period_secs: u64,
}

impl Default for SubscriptionFallbackPolicy {
    fn default() -> Self {
        Self {
            method_priorities: vec![
                (MethodId("lightning".to_string()), 1),
                (MethodId("onchain".to_string()), 2),
            ],
            max_methods: 3,
            max_retries_per_method: 2,
            retry_delay_secs: 60,
            notify_on_fallback: true,
            grace_period_secs: 86400, // 24 hours
        }
    }
}

impl SubscriptionFallbackPolicy {
    /// Create a new empty policy.
    pub fn new() -> Self {
        Self {
            method_priorities: Vec::new(),
            ..Default::default()
        }
    }

    /// Add a method with priority.
    pub fn with_method(mut self, method: MethodId, priority: u8) -> Self {
        self.method_priorities.push((method, priority));
        self.method_priorities.sort_by_key(|(_, p)| *p);
        self
    }

    /// Set max methods to try.
    pub fn with_max_methods(mut self, max: u8) -> Self {
        self.max_methods = max;
        self
    }

    /// Set max retries per method.
    pub fn with_max_retries(mut self, max: u8) -> Self {
        self.max_retries_per_method = max;
        self
    }

    /// Set retry delay.
    pub fn with_retry_delay(mut self, secs: u64) -> Self {
        self.retry_delay_secs = secs;
        self
    }

    /// Set grace period.
    pub fn with_grace_period(mut self, secs: u64) -> Self {
        self.grace_period_secs = secs;
        self
    }

    /// Get methods in priority order.
    pub fn ordered_methods(&self) -> Vec<MethodId> {
        self.method_priorities.iter().map(|(m, _)| m.clone()).collect()
    }

    /// Get priority for a method.
    pub fn get_priority(&self, method: &MethodId) -> Option<u8> {
        self.method_priorities
            .iter()
            .find(|(m, _)| m.0 == method.0)
            .map(|(_, p)| *p)
    }

    /// Check if a method is in the policy.
    pub fn has_method(&self, method: &MethodId) -> bool {
        self.method_priorities.iter().any(|(m, _)| m.0 == method.0)
    }

    /// Convert to routing FallbackConfig.
    pub fn to_fallback_config(&self) -> FallbackConfig {
        FallbackConfig {
            max_attempts: self.max_methods,
            timeout_ms: self.retry_delay_secs * 1000,
            continue_on_timeout: true,
        }
    }
}

/// Record of a fallback execution for a subscription.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FallbackRecord {
    /// Subscription ID.
    pub subscription_id: String,
    /// Billing period this fallback was for.
    pub period_start: i64,
    /// When the fallback was initiated.
    pub initiated_at: i64,
    /// All attempts made.
    pub attempts: Vec<FallbackAttemptRecord>,
    /// Final status.
    pub status: FallbackStatus,
    /// Method that ultimately succeeded (if any).
    pub successful_method: Option<MethodId>,
}

impl FallbackRecord {
    /// Create a new fallback record.
    pub fn new(subscription_id: String, period_start: i64) -> Self {
        Self {
            subscription_id,
            period_start,
            initiated_at: chrono::Utc::now().timestamp(),
            attempts: Vec::new(),
            status: FallbackStatus::InProgress,
            successful_method: None,
        }
    }

    /// Record an attempt.
    pub fn record_attempt(&mut self, attempt: FallbackAttemptRecord) {
        if attempt.success {
            self.status = FallbackStatus::Succeeded;
            self.successful_method = Some(attempt.method.clone());
        }
        self.attempts.push(attempt);
    }

    /// Mark as failed.
    pub fn mark_failed(&mut self) {
        self.status = FallbackStatus::Failed;
    }

    /// Mark as in grace period.
    pub fn mark_grace_period(&mut self) {
        self.status = FallbackStatus::GracePeriod;
    }

    /// Get total attempts count.
    pub fn total_attempts(&self) -> usize {
        self.attempts.len()
    }

    /// Check if succeeded.
    pub fn succeeded(&self) -> bool {
        self.status == FallbackStatus::Succeeded
    }
}

/// Status of a fallback execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackStatus {
    /// Still trying fallback methods.
    InProgress,
    /// All methods exhausted, in grace period.
    GracePeriod,
    /// A fallback method succeeded.
    Succeeded,
    /// All fallback methods failed.
    Failed,
}

/// Record of a single fallback attempt.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FallbackAttemptRecord {
    /// Method tried.
    pub method: MethodId,
    /// Whether it succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// When the attempt was made.
    pub attempted_at: i64,
    /// Duration of the attempt in milliseconds.
    pub duration_ms: u64,
}

impl From<&FallbackAttempt> for FallbackAttemptRecord {
    fn from(attempt: &FallbackAttempt) -> Self {
        Self {
            method: attempt.method.clone(),
            success: attempt.success,
            error: attempt.error.clone(),
            attempted_at: chrono::Utc::now().timestamp(),
            duration_ms: 0, // Would need to be tracked externally
        }
    }
}

/// Handler for executing subscription payments with fallback.
pub struct FallbackHandler {
    policy: SubscriptionFallbackPolicy,
    registry: paykit_lib::methods::PaymentMethodRegistry,
}

impl FallbackHandler {
    /// Create a new handler.
    pub fn new(
        policy: SubscriptionFallbackPolicy,
        registry: paykit_lib::methods::PaymentMethodRegistry,
    ) -> Self {
        Self { policy, registry }
    }

    /// Create with default policy and registry.
    pub fn with_defaults() -> Self {
        Self {
            policy: SubscriptionFallbackPolicy::default(),
            registry: paykit_lib::methods::default_registry(),
        }
    }

    /// Get the policy.
    pub fn policy(&self) -> &SubscriptionFallbackPolicy {
        &self.policy
    }

    /// Build routing info for a subscription using available endpoints.
    pub fn build_routing(
        &self,
        subscription: &Subscription,
        available_endpoints: &HashMap<MethodId, EndpointData>,
    ) -> Result<RoutingInfo> {
        // Get primary method from subscription
        let primary_method = subscription.terms.method.clone();
        let primary_endpoint = available_endpoints
            .get(&primary_method)
            .cloned()
            .ok_or_else(|| SubscriptionError::NotFound(
                format!("No endpoint for primary method: {}", primary_method.0)
            ))?;

        let mut routing = RoutingInfo::single(primary_method.clone(), primary_endpoint);

        // Add fallback methods from policy
        for method in self.policy.ordered_methods() {
            if method.0 == subscription.terms.method.0 {
                continue; // Skip primary
            }

            if let Some(endpoint) = available_endpoints.get(&method) {
                let priority = self.policy.get_priority(&method).unwrap_or(10);
                routing = routing.add_hint(RoutingHint::new(method, endpoint.clone(), priority));
            }
        }

        Ok(routing)
    }

    /// Execute a subscription payment with automatic fallback.
    pub async fn execute_with_fallback(
        &self,
        subscription: &Subscription,
        available_endpoints: &HashMap<MethodId, EndpointData>,
        metadata: &serde_json::Value,
    ) -> Result<FallbackRecord> {
        let routing = self.build_routing(subscription, available_endpoints)?;
        let amount = paykit_lib::methods::Amount::new(
            subscription.terms.amount.to_string(),
            subscription.terms.currency.clone(),
        );

        // Create executor
        let executor = paykit_lib::routing::FallbackExecutor::new(
            self.policy.to_fallback_config(),
            self.registry.clone(),
        );

        // Execute with fallback
        let (success, attempts) = executor.execute(&routing, &amount, metadata).await;

        // Build record
        let mut record = FallbackRecord::new(
            subscription.subscription_id.clone(),
            chrono::Utc::now().timestamp(),
        );

        for attempt in &attempts {
            record.record_attempt(FallbackAttemptRecord::from(attempt));
        }

        if !success {
            if self.is_within_grace_period(&record) {
                record.mark_grace_period();
            } else {
                record.mark_failed();
            }
        }

        Ok(record)
    }

    /// Check if we're still within the grace period.
    fn is_within_grace_period(&self, record: &FallbackRecord) -> bool {
        let elapsed = chrono::Utc::now().timestamp() - record.initiated_at;
        elapsed < self.policy.grace_period_secs as i64
    }

    /// Get fallback methods for a subscription.
    pub fn get_fallback_methods(&self, subscription: &Subscription) -> Vec<MethodId> {
        self.policy
            .ordered_methods()
            .into_iter()
            .filter(|m| m.0 != subscription.terms.method.0)
            .collect()
    }

    /// Check if fallback is available for a method.
    pub fn has_fallback_for(&self, method: &MethodId) -> bool {
        let methods = self.policy.ordered_methods();
        methods.len() > 1 && methods.iter().any(|m| m.0 == method.0)
    }
}

/// Notification for fallback events.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FallbackNotification {
    /// Primary method failed, trying fallback.
    FallbackActivated {
        subscription_id: String,
        failed_method: MethodId,
        next_method: MethodId,
    },
    /// A fallback method succeeded.
    FallbackSucceeded {
        subscription_id: String,
        successful_method: MethodId,
    },
    /// All fallback methods failed.
    FallbackExhausted {
        subscription_id: String,
        total_attempts: usize,
    },
    /// Entering grace period.
    GracePeriodStarted {
        subscription_id: String,
        expires_at: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Amount;
    use std::str::FromStr;

    fn test_pubkey() -> paykit_lib::PublicKey {
        let keypair = pkarr::Keypair::random();
        paykit_lib::PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
    }

    fn test_subscription() -> Subscription {
        use crate::{PaymentFrequency, SubscriptionTerms};

        Subscription::new(
            test_pubkey(),
            test_pubkey(),
            SubscriptionTerms::new(
                Amount::from_sats(1000),
                "SAT".to_string(),
                PaymentFrequency::Monthly { day_of_month: 1 },
                MethodId("lightning".to_string()),
                "Test subscription".to_string(),
            ),
        )
    }

    #[test]
    fn test_policy_default() {
        let policy = SubscriptionFallbackPolicy::default();
        assert_eq!(policy.max_methods, 3);
        assert_eq!(policy.max_retries_per_method, 2);
        assert!(!policy.method_priorities.is_empty());
    }

    #[test]
    fn test_policy_builder() {
        let policy = SubscriptionFallbackPolicy::new()
            .with_method(MethodId("lightning".to_string()), 1)
            .with_method(MethodId("onchain".to_string()), 2)
            .with_max_methods(5)
            .with_retry_delay(120);

        assert_eq!(policy.method_priorities.len(), 2);
        assert_eq!(policy.max_methods, 5);
        assert_eq!(policy.retry_delay_secs, 120);
    }

    #[test]
    fn test_policy_ordering() {
        let policy = SubscriptionFallbackPolicy::new()
            .with_method(MethodId("onchain".to_string()), 3)
            .with_method(MethodId("lightning".to_string()), 1)
            .with_method(MethodId("lnurl".to_string()), 2);

        let ordered = policy.ordered_methods();
        assert_eq!(ordered[0].0, "lightning");
        assert_eq!(ordered[1].0, "lnurl");
        assert_eq!(ordered[2].0, "onchain");
    }

    #[test]
    fn test_fallback_record() {
        let mut record = FallbackRecord::new("sub_123".to_string(), 1000000);
        assert_eq!(record.status, FallbackStatus::InProgress);
        assert!(record.attempts.is_empty());

        record.record_attempt(FallbackAttemptRecord {
            method: MethodId("lightning".to_string()),
            success: false,
            error: Some("Network error".to_string()),
            attempted_at: chrono::Utc::now().timestamp(),
            duration_ms: 100,
        });

        assert_eq!(record.total_attempts(), 1);
        assert!(!record.succeeded());

        record.record_attempt(FallbackAttemptRecord {
            method: MethodId("onchain".to_string()),
            success: true,
            error: None,
            attempted_at: chrono::Utc::now().timestamp(),
            duration_ms: 200,
        });

        assert_eq!(record.total_attempts(), 2);
        assert!(record.succeeded());
        assert_eq!(record.successful_method.as_ref().unwrap().0, "onchain");
    }

    #[test]
    fn test_handler_build_routing() {
        let handler = FallbackHandler::with_defaults();
        let subscription = test_subscription();

        let mut endpoints = HashMap::new();
        endpoints.insert(
            MethodId("lightning".to_string()),
            EndpointData("lnbc...".to_string()),
        );
        endpoints.insert(
            MethodId("onchain".to_string()),
            EndpointData("bc1q...".to_string()),
        );

        let routing = handler.build_routing(&subscription, &endpoints);
        assert!(routing.is_ok());

        let routing = routing.unwrap();
        assert_eq!(routing.primary_method.0, "lightning");
        assert!(!routing.fallback_chain.is_empty());
    }

    #[test]
    fn test_handler_get_fallback_methods() {
        let handler = FallbackHandler::with_defaults();
        let subscription = test_subscription();

        let fallbacks = handler.get_fallback_methods(&subscription);
        // Should not include the primary method (lightning)
        assert!(!fallbacks.iter().any(|m| m.0 == "lightning"));
        assert!(fallbacks.iter().any(|m| m.0 == "onchain"));
    }

    #[test]
    fn test_to_fallback_config() {
        let policy = SubscriptionFallbackPolicy::default()
            .with_max_methods(5)
            .with_retry_delay(120);

        let config = policy.to_fallback_config();
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.timeout_ms, 120000);
    }
}
