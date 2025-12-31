//! Payment Method Health Check System
//!
//! This module provides health monitoring for payment methods,
//! enabling automatic failover and status-based selection.
//!
//! # Thread Safety
//!
//! The health monitor uses `RwLock` for thread-safe cache access. Public methods
//! will panic if the internal lock is poisoned (which only happens if a thread
//! panics while holding the lock).

use crate::methods::PaymentMethodRegistry;
use crate::MethodId;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Health status of a payment method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Method is fully operational.
    Healthy,
    /// Method has some issues but is still functional.
    Degraded,
    /// Method is currently unavailable.
    Unavailable,
    /// Health status is unknown (not yet checked).
    Unknown,
}

impl HealthStatus {
    /// Check if the method is usable.
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded)
    }

    /// Check if the method is healthy.
    pub fn is_healthy(&self) -> bool {
        *self == Self::Healthy
    }
}

/// Health check result with details.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// The method checked.
    pub method_id: MethodId,
    /// Current status.
    pub status: HealthStatus,
    /// Timestamp of check.
    pub checked_at: i64,
    /// Latency in milliseconds (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Error message if unhealthy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Additional details.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub details: serde_json::Value,
}

impl HealthCheckResult {
    /// Create a healthy result.
    pub fn healthy(method_id: MethodId) -> Self {
        Self {
            method_id,
            status: HealthStatus::Healthy,
            checked_at: current_timestamp(),
            latency_ms: None,
            error: None,
            details: serde_json::Value::Null,
        }
    }

    /// Create an unhealthy result.
    pub fn unhealthy(method_id: MethodId, error: impl Into<String>) -> Self {
        Self {
            method_id,
            status: HealthStatus::Unavailable,
            checked_at: current_timestamp(),
            latency_ms: None,
            error: Some(error.into()),
            details: serde_json::Value::Null,
        }
    }

    /// Create a degraded result.
    pub fn degraded(method_id: MethodId, reason: impl Into<String>) -> Self {
        Self {
            method_id,
            status: HealthStatus::Degraded,
            checked_at: current_timestamp(),
            latency_ms: None,
            error: Some(reason.into()),
            details: serde_json::Value::Null,
        }
    }

    /// Set latency.
    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }

    /// Set details.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }
}

/// Trait for health checkers.
#[async_trait]
pub trait HealthChecker: Send + Sync {
    /// Get the method ID this checker handles.
    fn method_id(&self) -> MethodId;

    /// Perform a health check.
    async fn check(&self) -> HealthCheckResult;
}

/// On-chain health checker (placeholder implementation).
pub struct OnchainHealthChecker {
    /// Node URL for checking connectivity.
    #[allow(dead_code)]
    node_url: Option<String>,
}

impl OnchainHealthChecker {
    /// Create a new checker.
    pub fn new(node_url: Option<String>) -> Self {
        Self { node_url }
    }
}

#[async_trait]
impl HealthChecker for OnchainHealthChecker {
    fn method_id(&self) -> MethodId {
        MethodId("onchain".to_string())
    }

    async fn check(&self) -> HealthCheckResult {
        // In production, this would:
        // 1. Connect to a Bitcoin node
        // 2. Check block height is recent
        // 3. Check fee estimation is working

        // For now, always return healthy
        HealthCheckResult::healthy(self.method_id()).with_details(serde_json::json!({
            "note": "Placeholder check - production should verify node connectivity"
        }))
    }
}

/// Lightning health checker (placeholder implementation).
pub struct LightningHealthChecker {
    /// Node URL for checking connectivity.
    #[allow(dead_code)]
    node_url: Option<String>,
}

impl LightningHealthChecker {
    /// Create a new checker.
    pub fn new(node_url: Option<String>) -> Self {
        Self { node_url }
    }
}

#[async_trait]
impl HealthChecker for LightningHealthChecker {
    fn method_id(&self) -> MethodId {
        MethodId("lightning".to_string())
    }

    async fn check(&self) -> HealthCheckResult {
        // In production, this would:
        // 1. Connect to Lightning node
        // 2. Check channel status
        // 3. Check liquidity

        // For now, always return healthy
        HealthCheckResult::healthy(self.method_id()).with_details(serde_json::json!({
            "note": "Placeholder check - production should verify node connectivity"
        }))
    }
}

/// Health monitor that tracks all payment method health.
pub struct HealthMonitor {
    /// Registered health checkers.
    checkers: Vec<Box<dyn HealthChecker>>,
    /// Cached health results.
    cache: RwLock<HashMap<String, HealthCheckResult>>,
    /// Cache TTL in seconds.
    cache_ttl_secs: i64,
}

impl HealthMonitor {
    /// Create a new monitor.
    pub fn new() -> Self {
        Self {
            checkers: Vec::new(),
            cache: RwLock::new(HashMap::new()),
            cache_ttl_secs: 60, // 1 minute default
        }
    }

    /// Create with default checkers.
    pub fn with_defaults() -> Self {
        let mut monitor = Self::new();
        monitor.register(Box::new(OnchainHealthChecker::new(None)));
        monitor.register(Box::new(LightningHealthChecker::new(None)));
        monitor
    }

    /// Set cache TTL.
    pub fn with_ttl(mut self, ttl_secs: i64) -> Self {
        self.cache_ttl_secs = ttl_secs;
        self
    }

    /// Register a health checker.
    pub fn register(&mut self, checker: Box<dyn HealthChecker>) {
        self.checkers.push(checker);
    }

    /// Get cached status for a method.
    pub fn get_status(&self, method_id: &MethodId) -> Option<HealthStatus> {
        let cache = self
            .cache
            .read()
            .unwrap_or_else(|e| e.into_inner());
        cache.get(&method_id.0).map(|r| r.status)
    }

    /// Get cached result for a method.
    pub fn get_result(&self, method_id: &MethodId) -> Option<HealthCheckResult> {
        let cache = self
            .cache
            .read()
            .unwrap_or_else(|e| e.into_inner());
        cache.get(&method_id.0).cloned()
    }

    /// Check if a method is usable.
    pub fn is_usable(&self, method_id: &MethodId) -> bool {
        self.get_status(method_id)
            .map(|s| s.is_usable())
            .unwrap_or(true) // Default to usable if unknown
    }

    /// Check health of a specific method.
    pub async fn check(&self, method_id: &MethodId) -> Option<HealthCheckResult> {
        // Find the checker
        let checker = self
            .checkers
            .iter()
            .find(|c| c.method_id().0 == method_id.0)?;

        // Perform check
        let result = checker.check().await;

        // Update cache
        {
            let mut cache = self
                .cache
                .write()
                .unwrap_or_else(|e| e.into_inner());
            cache.insert(method_id.0.clone(), result.clone());
        }

        Some(result)
    }

    /// Check health of all methods.
    pub async fn check_all(&self) -> Vec<HealthCheckResult> {
        let mut results = Vec::new();

        for checker in &self.checkers {
            let result = checker.check().await;

            // Update cache
            {
                let mut cache = self
                    .cache
                    .write()
                    .unwrap_or_else(|e| e.into_inner());
                cache.insert(result.method_id.0.clone(), result.clone());
            }

            results.push(result);
        }

        results
    }

    /// Get all healthy methods.
    pub fn get_healthy_methods(&self) -> Vec<MethodId> {
        let cache = self
            .cache
            .read()
            .unwrap_or_else(|e| e.into_inner());
        cache
            .iter()
            .filter(|(_, r)| r.status.is_healthy())
            .map(|(id, _)| MethodId(id.clone()))
            .collect()
    }

    /// Get all usable methods.
    pub fn get_usable_methods(&self) -> Vec<MethodId> {
        let cache = self
            .cache
            .read()
            .unwrap_or_else(|e| e.into_inner());
        cache
            .iter()
            .filter(|(_, r)| r.status.is_usable())
            .map(|(id, _)| MethodId(id.clone()))
            .collect()
    }

    /// Check if cache is stale for a method.
    pub fn is_stale(&self, method_id: &MethodId) -> bool {
        let cache = self
            .cache
            .read()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(result) = cache.get(&method_id.0) {
            let now = current_timestamp();
            (now - result.checked_at) > self.cache_ttl_secs
        } else {
            true // No cache = stale
        }
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Health-aware method selector integration.
pub struct HealthAwareSelector {
    /// The health monitor.
    monitor: Arc<HealthMonitor>,
    /// The method registry.
    #[allow(dead_code)]
    registry: PaymentMethodRegistry,
}

impl HealthAwareSelector {
    /// Create a new health-aware selector.
    pub fn new(monitor: Arc<HealthMonitor>, registry: PaymentMethodRegistry) -> Self {
        Self { monitor, registry }
    }

    /// Filter methods by health status.
    pub fn filter_usable(&self, methods: &[MethodId]) -> Vec<MethodId> {
        methods
            .iter()
            .filter(|m| self.monitor.is_usable(m))
            .cloned()
            .collect()
    }

    /// Get health status for methods.
    pub fn get_statuses(&self, methods: &[MethodId]) -> HashMap<MethodId, HealthStatus> {
        methods
            .iter()
            .filter_map(|m| self.monitor.get_status(m).map(|s| (m.clone(), s)))
            .collect()
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

    #[test]
    fn test_health_status() {
        assert!(HealthStatus::Healthy.is_usable());
        assert!(HealthStatus::Healthy.is_healthy());

        assert!(HealthStatus::Degraded.is_usable());
        assert!(!HealthStatus::Degraded.is_healthy());

        assert!(!HealthStatus::Unavailable.is_usable());
        assert!(!HealthStatus::Unknown.is_usable());
    }

    #[test]
    fn test_health_check_result() {
        let result = HealthCheckResult::healthy(MethodId("lightning".into())).with_latency(50);

        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.latency_ms, Some(50));
    }

    #[test]
    fn test_unhealthy_result() {
        let result = HealthCheckResult::unhealthy(MethodId("onchain".into()), "Node not reachable");

        assert_eq!(result.status, HealthStatus::Unavailable);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_health_monitor() {
        let monitor = HealthMonitor::with_defaults();

        // Check all methods
        let results = monitor.check_all().await;
        assert_eq!(results.len(), 2);

        // All should be healthy (placeholder implementation)
        for result in &results {
            assert!(result.status.is_healthy());
        }
    }

    #[tokio::test]
    async fn test_get_status() {
        let monitor = HealthMonitor::with_defaults();
        monitor.check_all().await;

        let status = monitor.get_status(&MethodId("lightning".into()));
        assert_eq!(status, Some(HealthStatus::Healthy));
    }

    #[tokio::test]
    async fn test_get_usable_methods() {
        let monitor = HealthMonitor::with_defaults();
        monitor.check_all().await;

        let usable = monitor.get_usable_methods();
        assert_eq!(usable.len(), 2);
    }

    #[test]
    fn test_health_aware_selector() {
        let monitor = Arc::new(HealthMonitor::with_defaults());
        let registry = crate::methods::default_registry();
        let selector = HealthAwareSelector::new(monitor, registry);

        let methods = vec![MethodId("lightning".into()), MethodId("onchain".into())];

        // Without cache, all should be considered usable
        let usable = selector.filter_usable(&methods);
        assert_eq!(usable.len(), 2);
    }
}
