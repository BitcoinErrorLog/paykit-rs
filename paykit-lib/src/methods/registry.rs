//! Payment Method Plugin Registry
//!
//! This module provides a registry for managing payment method plugins.
//! Applications can register plugins at runtime and query them by method ID.
//!
//! # Thread Safety
//!
//! The registry uses `RwLock` for thread-safe access. All public methods will panic
//! if the internal lock is poisoned (which only happens if a thread panics while
//! holding the lock). In well-tested applications, this should never occur.
//!
//! If you need fallible access, use [`get_required`](PaymentMethodRegistry::get_required)
//! which returns a `Result`.

use super::traits::PaymentMethodPlugin;
use crate::{MethodId, PaykitError, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registry for payment method plugins.
///
/// The registry allows dynamic registration and lookup of payment methods.
/// It is thread-safe and can be shared across async tasks.
///
/// # Example
///
/// ```ignore
/// use paykit_lib::methods::{PaymentMethodRegistry, OnchainPlugin};
///
/// let registry = PaymentMethodRegistry::new();
/// registry.register(Box::new(OnchainPlugin::new()));
///
/// let onchain = registry.get(&MethodId("onchain".into()));
/// assert!(onchain.is_some());
/// ```
pub struct PaymentMethodRegistry {
    plugins: RwLock<HashMap<String, Arc<dyn PaymentMethodPlugin>>>,
}

impl PaymentMethodRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Creates a registry with the default built-in plugins.
    pub fn with_defaults() -> Self {
        let registry = Self::new();

        // Register built-in plugins
        registry.register(Box::new(super::onchain::OnchainPlugin::new()));
        registry.register(Box::new(super::lightning::LightningPlugin::new()));

        registry
    }

    /// Registers a payment method plugin.
    ///
    /// If a plugin with the same method ID already exists, it will be replaced.
    pub fn register(&self, plugin: Box<dyn PaymentMethodPlugin>) {
        let method_id = plugin.method_id().0.clone();
        let mut plugins = self.plugins.write().unwrap_or_else(|e| e.into_inner());
        plugins.insert(method_id, Arc::from(plugin));
    }

    /// Unregisters a payment method plugin.
    ///
    /// Returns the removed plugin if it existed.
    pub fn unregister(&self, method_id: &MethodId) -> Option<Arc<dyn PaymentMethodPlugin>> {
        let mut plugins = self.plugins.write().unwrap_or_else(|e| e.into_inner());
        plugins.remove(&method_id.0)
    }

    /// Gets a payment method plugin by its ID.
    pub fn get(&self, method_id: &MethodId) -> Option<Arc<dyn PaymentMethodPlugin>> {
        let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
        plugins.get(&method_id.0).cloned()
    }

    /// Gets a payment method plugin, returning an error if not found.
    pub fn get_required(&self, method_id: &MethodId) -> Result<Arc<dyn PaymentMethodPlugin>> {
        self.get(method_id).ok_or_else(|| {
            PaykitError::Transport(format!("Payment method not registered: {}", method_id.0))
        })
    }

    /// Returns all registered method IDs.
    pub fn list_methods(&self) -> Vec<MethodId> {
        let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
        plugins.keys().map(|k| MethodId(k.clone())).collect()
    }

    /// Returns the number of registered plugins.
    pub fn len(&self) -> usize {
        let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
        plugins.len()
    }

    /// Returns true if no plugins are registered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Checks if a method is registered.
    pub fn has_method(&self, method_id: &MethodId) -> bool {
        let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
        plugins.contains_key(&method_id.0)
    }

    /// Gets plugins for multiple method IDs.
    ///
    /// Returns a vector of (method_id, plugin) pairs for methods that exist.
    pub fn get_multiple(
        &self,
        method_ids: &[MethodId],
    ) -> Vec<(MethodId, Arc<dyn PaymentMethodPlugin>)> {
        let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
        method_ids
            .iter()
            .filter_map(|id| plugins.get(&id.0).map(|p| (id.clone(), p.clone())))
            .collect()
    }

    /// Filters methods by a predicate.
    pub fn filter<F>(&self, predicate: F) -> Vec<Arc<dyn PaymentMethodPlugin>>
    where
        F: Fn(&dyn PaymentMethodPlugin) -> bool,
    {
        let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
        plugins
            .values()
            .filter(|p| predicate(p.as_ref()))
            .cloned()
            .collect()
    }
}

impl Default for PaymentMethodRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PaymentMethodRegistry {
    fn clone(&self) -> Self {
        let plugins = self.plugins.read().unwrap_or_else(|e| e.into_inner());
        Self {
            plugins: RwLock::new(plugins.clone()),
        }
    }
}

/// Global registry instance for convenience.
///
/// Use this when you need a shared registry across your application.
/// For more control, create your own `PaymentMethodRegistry` instance.
pub mod global {
    use super::*;
    use std::sync::OnceLock;

    static GLOBAL_REGISTRY: OnceLock<PaymentMethodRegistry> = OnceLock::new();

    /// Gets the global registry, initializing it if necessary.
    pub fn registry() -> &'static PaymentMethodRegistry {
        GLOBAL_REGISTRY.get_or_init(PaymentMethodRegistry::with_defaults)
    }

    /// Registers a plugin in the global registry.
    pub fn register(plugin: Box<dyn PaymentMethodPlugin>) {
        registry().register(plugin);
    }

    /// Gets a plugin from the global registry.
    pub fn get(method_id: &MethodId) -> Option<Arc<dyn PaymentMethodPlugin>> {
        registry().get(method_id)
    }

    /// Lists all methods in the global registry.
    pub fn list_methods() -> Vec<MethodId> {
        registry().list_methods()
    }
}

#[cfg(test)]
mod tests {
    use super::super::traits::{Amount, PaymentExecution, PaymentProof, ValidationResult};
    use super::*;
    use async_trait::async_trait;
    use serde_json::Value;

    /// Mock plugin for testing.
    struct MockPlugin {
        id: String,
    }

    impl MockPlugin {
        fn new(id: &str) -> Self {
            Self { id: id.to_string() }
        }
    }

    #[async_trait]
    impl PaymentMethodPlugin for MockPlugin {
        fn method_id(&self) -> MethodId {
            MethodId(self.id.clone())
        }

        fn display_name(&self) -> &str {
            "Mock Plugin"
        }

        fn description(&self) -> &str {
            "A mock payment method for testing"
        }

        fn validate_endpoint(&self, _data: &crate::EndpointData) -> ValidationResult {
            ValidationResult::valid()
        }

        async fn execute_payment(
            &self,
            endpoint: &crate::EndpointData,
            amount: &Amount,
            _metadata: &Value,
        ) -> Result<PaymentExecution> {
            Ok(PaymentExecution::success(
                self.method_id(),
                endpoint.clone(),
                amount.clone(),
                serde_json::json!({"mock": true}),
            ))
        }

        fn generate_proof(&self, _execution: &PaymentExecution) -> Result<PaymentProof> {
            Ok(PaymentProof::custom(
                self.method_id(),
                serde_json::json!({"mock_proof": true}),
            ))
        }

        fn format_receipt_metadata(&self, _execution: &PaymentExecution) -> Value {
            serde_json::json!({"mock_metadata": true})
        }
    }

    #[test]
    fn test_registry_register_and_get() {
        let registry = PaymentMethodRegistry::new();
        assert!(registry.is_empty());

        registry.register(Box::new(MockPlugin::new("test-method")));
        assert_eq!(registry.len(), 1);

        let plugin = registry.get(&MethodId("test-method".into()));
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().method_id().0, "test-method");
    }

    #[test]
    fn test_registry_unregister() {
        let registry = PaymentMethodRegistry::new();
        registry.register(Box::new(MockPlugin::new("to-remove")));
        assert!(registry.has_method(&MethodId("to-remove".into())));

        let removed = registry.unregister(&MethodId("to-remove".into()));
        assert!(removed.is_some());
        assert!(!registry.has_method(&MethodId("to-remove".into())));
    }

    #[test]
    fn test_registry_list_methods() {
        let registry = PaymentMethodRegistry::new();
        registry.register(Box::new(MockPlugin::new("method-a")));
        registry.register(Box::new(MockPlugin::new("method-b")));

        let methods = registry.list_methods();
        assert_eq!(methods.len(), 2);
    }

    #[test]
    fn test_registry_get_multiple() {
        let registry = PaymentMethodRegistry::new();
        registry.register(Box::new(MockPlugin::new("a")));
        registry.register(Box::new(MockPlugin::new("b")));
        registry.register(Box::new(MockPlugin::new("c")));

        let ids = vec![
            MethodId("a".into()),
            MethodId("b".into()),
            MethodId("missing".into()),
        ];
        let results = registry.get_multiple(&ids);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_registry_clone() {
        let registry = PaymentMethodRegistry::new();
        registry.register(Box::new(MockPlugin::new("original")));

        let cloned = registry.clone();
        assert!(cloned.has_method(&MethodId("original".into())));
    }
}
