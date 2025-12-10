//! Endpoint Rotation Manager
//!
//! This module provides the manager for automatic endpoint rotation.

use super::policies::{EndpointTracker, RotationPolicy};
use crate::methods::PaymentMethodRegistry;
use crate::{AuthenticatedTransport, EndpointData, MethodId, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Configuration for the rotation manager.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RotationConfig {
    /// Default policy for methods without specific policy.
    pub default_policy: RotationPolicy,
    /// Per-method policy overrides.
    pub method_policies: HashMap<String, RotationPolicy>,
    /// Whether to auto-rotate on payment execution.
    pub auto_rotate_on_payment: bool,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            default_policy: RotationPolicy::RotateOnUse,
            method_policies: HashMap::new(),
            auto_rotate_on_payment: true,
        }
    }
}

impl RotationConfig {
    /// Get the policy for a specific method.
    pub fn policy_for(&self, method_id: &MethodId) -> &RotationPolicy {
        self.method_policies
            .get(&method_id.0)
            .unwrap_or(&self.default_policy)
    }

    /// Set a policy for a specific method.
    pub fn set_policy(mut self, method_id: MethodId, policy: RotationPolicy) -> Self {
        self.method_policies.insert(method_id.0, policy);
        self
    }
}

/// Callback type for rotation events.
pub type RotationCallback = Arc<dyn Fn(&MethodId, &EndpointData) + Send + Sync>;

/// Manager for endpoint rotation.
///
/// The rotation manager:
/// - Tracks endpoint usage per method
/// - Determines when rotation is needed based on policies
/// - Coordinates with plugins to generate new endpoints
/// - Updates the Pubky directory with new endpoints
pub struct EndpointRotationManager {
    /// Configuration.
    config: RotationConfig,
    /// Plugin registry for generating new endpoints.
    registry: PaymentMethodRegistry,
    /// Tracking info per method.
    trackers: RwLock<HashMap<String, EndpointTracker>>,
    /// Current endpoints per method.
    endpoints: RwLock<HashMap<String, EndpointData>>,
    /// Callbacks for rotation events.
    callbacks: RwLock<Vec<RotationCallback>>,
}

impl EndpointRotationManager {
    /// Create a new rotation manager.
    pub fn new(config: RotationConfig, registry: PaymentMethodRegistry) -> Self {
        Self {
            config,
            registry,
            trackers: RwLock::new(HashMap::new()),
            endpoints: RwLock::new(HashMap::new()),
            callbacks: RwLock::new(Vec::new()),
        }
    }

    /// Create with default configuration and registry.
    pub fn with_defaults() -> Self {
        Self::new(
            RotationConfig::default(),
            crate::methods::default_registry(),
        )
    }

    /// Register a callback for rotation events.
    pub fn on_rotation(&self, callback: RotationCallback) {
        let mut callbacks = self.callbacks.write().expect("lock poisoned");
        callbacks.push(callback);
    }

    /// Set the current endpoint for a method.
    pub fn set_endpoint(&self, method_id: &MethodId, endpoint: EndpointData) {
        let mut endpoints = self.endpoints.write().expect("lock poisoned");
        endpoints.insert(method_id.0.clone(), endpoint);
        
        // Initialize tracker if needed
        let mut trackers = self.trackers.write().expect("lock poisoned");
        trackers.entry(method_id.0.clone()).or_insert_with(EndpointTracker::new);
    }

    /// Get the current endpoint for a method.
    pub fn get_endpoint(&self, method_id: &MethodId) -> Option<EndpointData> {
        let endpoints = self.endpoints.read().expect("lock poisoned");
        endpoints.get(&method_id.0).cloned()
    }

    /// Record that an endpoint was used.
    ///
    /// This updates the usage counter and may trigger rotation
    /// if the policy requires it.
    pub fn record_use(&self, method_id: &MethodId) {
        let mut trackers = self.trackers.write().expect("lock poisoned");
        if let Some(tracker) = trackers.get_mut(&method_id.0) {
            tracker.record_use();
        }
    }

    /// Check if rotation is needed for a method.
    pub fn needs_rotation(&self, method_id: &MethodId) -> bool {
        let policy = self.config.policy_for(method_id);
        let trackers = self.trackers.read().expect("lock poisoned");
        
        if let Some(tracker) = trackers.get(&method_id.0) {
            tracker.needs_rotation(policy)
        } else {
            false
        }
    }

    /// Get all methods that need rotation.
    pub fn methods_needing_rotation(&self) -> Vec<MethodId> {
        let trackers = self.trackers.read().expect("lock poisoned");
        let mut needs_rotation = Vec::new();

        for (method_id_str, tracker) in trackers.iter() {
            let method_id = MethodId(method_id_str.clone());
            let policy = self.config.policy_for(&method_id);
            if tracker.needs_rotation(policy) {
                needs_rotation.push(method_id);
            }
        }

        needs_rotation
    }

    /// Rotate an endpoint for a method.
    ///
    /// This generates a new endpoint using the method's plugin
    /// and updates the stored endpoint.
    pub async fn rotate(&self, method_id: &MethodId) -> Result<EndpointData> {
        // Get the plugin
        let plugin = self.registry.get_required(method_id)?;

        // Generate new endpoint
        let new_endpoint = plugin.generate_endpoint().await?;

        // Update stored endpoint
        {
            let mut endpoints = self.endpoints.write().expect("lock poisoned");
            endpoints.insert(method_id.0.clone(), new_endpoint.clone());
        }

        // Reset tracker
        {
            let mut trackers = self.trackers.write().expect("lock poisoned");
            if let Some(tracker) = trackers.get_mut(&method_id.0) {
                tracker.reset();
            }
        }

        // Notify callbacks
        {
            let callbacks = self.callbacks.read().expect("lock poisoned");
            for callback in callbacks.iter() {
                callback(method_id, &new_endpoint);
            }
        }

        Ok(new_endpoint)
    }

    /// Rotate an endpoint and publish to Pubky directory.
    pub async fn rotate_and_publish<T: AuthenticatedTransport>(
        &self,
        method_id: &MethodId,
        transport: &T,
    ) -> Result<EndpointData> {
        // Rotate the endpoint
        let new_endpoint = self.rotate(method_id).await?;

        // Publish to directory
        crate::set_payment_endpoint(transport, method_id.clone(), new_endpoint.clone()).await?;

        Ok(new_endpoint)
    }

    /// Rotate all methods that need rotation.
    pub async fn rotate_all_pending(&self) -> Vec<(MethodId, Result<EndpointData>)> {
        let methods = self.methods_needing_rotation();
        let mut results = Vec::new();

        for method_id in methods {
            let result = self.rotate(&method_id).await;
            results.push((method_id, result));
        }

        results
    }

    /// Hook to be called after payment execution.
    ///
    /// Records usage and triggers rotation if configured.
    pub async fn on_payment_executed(&self, method_id: &MethodId) -> Option<EndpointData> {
        self.record_use(method_id);

        if self.config.auto_rotate_on_payment && self.needs_rotation(method_id) {
            match self.rotate(method_id).await {
                Ok(new_endpoint) => Some(new_endpoint),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    /// Get the tracker for a method.
    pub fn get_tracker(&self, method_id: &MethodId) -> Option<EndpointTracker> {
        let trackers = self.trackers.read().expect("lock poisoned");
        trackers.get(&method_id.0).cloned()
    }

    /// Get the configuration.
    pub fn config(&self) -> &RotationConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation_config_default() {
        let config = RotationConfig::default();
        assert_eq!(config.default_policy, RotationPolicy::RotateOnUse);
        assert!(config.auto_rotate_on_payment);
    }

    #[test]
    fn test_rotation_config_per_method() {
        let config = RotationConfig::default()
            .set_policy(MethodId("onchain".into()), RotationPolicy::after_uses(5));

        let onchain_policy = config.policy_for(&MethodId("onchain".into()));
        assert_eq!(*onchain_policy, RotationPolicy::RotateOnThreshold { threshold: 5 });

        let lightning_policy = config.policy_for(&MethodId("lightning".into()));
        assert_eq!(*lightning_policy, RotationPolicy::RotateOnUse);
    }

    #[test]
    fn test_manager_set_get_endpoint() {
        let manager = EndpointRotationManager::with_defaults();
        let method = MethodId("onchain".into());
        let endpoint = EndpointData("bc1q...".into());

        assert!(manager.get_endpoint(&method).is_none());

        manager.set_endpoint(&method, endpoint.clone());
        assert_eq!(manager.get_endpoint(&method), Some(endpoint));
    }

    #[test]
    fn test_manager_record_use() {
        let manager = EndpointRotationManager::with_defaults();
        let method = MethodId("onchain".into());
        let endpoint = EndpointData("bc1q...".into());

        manager.set_endpoint(&method, endpoint);
        
        let tracker = manager.get_tracker(&method).unwrap();
        assert_eq!(tracker.use_count, 0);

        manager.record_use(&method);
        let tracker = manager.get_tracker(&method).unwrap();
        assert_eq!(tracker.use_count, 1);
    }

    #[test]
    fn test_manager_needs_rotation() {
        let config = RotationConfig::default()
            .set_policy(MethodId("onchain".into()), RotationPolicy::after_uses(2));
        
        let manager = EndpointRotationManager::new(config, crate::methods::default_registry());
        let method = MethodId("onchain".into());
        let endpoint = EndpointData("bc1q...".into());

        manager.set_endpoint(&method, endpoint);

        // Initially no rotation needed
        assert!(!manager.needs_rotation(&method));

        // After 1 use - still no
        manager.record_use(&method);
        assert!(!manager.needs_rotation(&method));

        // After 2 uses - needs rotation
        manager.record_use(&method);
        assert!(manager.needs_rotation(&method));
    }

    #[test]
    fn test_manager_callbacks() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let manager = EndpointRotationManager::with_defaults();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        manager.on_rotation(Arc::new(move |_method, _endpoint| {
            called_clone.store(true, Ordering::SeqCst);
        }));

        // Callbacks are stored
        let callbacks = manager.callbacks.read().unwrap();
        assert_eq!(callbacks.len(), 1);
    }
}
