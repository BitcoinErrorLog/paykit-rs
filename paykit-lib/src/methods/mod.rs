//! Payment Method Plugin System
//!
//! This module provides the extensible payment method plugin architecture.
//! It allows any payment protocol to integrate with Paykit by implementing
//! the `PaymentMethodPlugin` trait.
//!
//! # Architecture
//!
//! The plugin system consists of:
//! - **Traits**: Core abstractions for payment methods (`PaymentMethodPlugin`)
//! - **Registry**: Dynamic registration and lookup of plugins (`PaymentMethodRegistry`)
//! - **Built-in Plugins**: Default implementations for Bitcoin on-chain and Lightning
//!
//! # Example
//!
//! ```ignore
//! use paykit_lib::methods::{PaymentMethodRegistry, OnchainPlugin, LightningPlugin};
//! use paykit_lib::MethodId;
//!
//! // Create a registry with built-in plugins
//! let registry = PaymentMethodRegistry::new();
//! registry.register(Box::new(OnchainPlugin::new()));
//! registry.register(Box::new(LightningPlugin::new()));
//!
//! // Get a plugin by method ID
//! if let Some(plugin) = registry.get(&MethodId("lightning".into())) {
//!     println!("Plugin: {}", plugin.display_name());
//! }
//! ```
//!
//! # Creating Custom Plugins
//!
//! Implement `PaymentMethodPlugin` to add support for new payment methods:
//!
//! ```ignore
//! use paykit_lib::methods::{PaymentMethodPlugin, Amount, PaymentExecution, PaymentProof, ValidationResult};
//! use paykit_lib::{MethodId, EndpointData, Result};
//! use async_trait::async_trait;
//! use serde_json::Value;
//!
//! struct MyPaymentPlugin;
//!
//! #[async_trait]
//! impl PaymentMethodPlugin for MyPaymentPlugin {
//!     fn method_id(&self) -> MethodId {
//!         MethodId("my-method".into())
//!     }
//!
//!     fn display_name(&self) -> &str {
//!         "My Payment Method"
//!     }
//!
//!     fn description(&self) -> &str {
//!         "Description of my payment method"
//!     }
//!
//!     fn validate_endpoint(&self, data: &EndpointData) -> ValidationResult {
//!         // Validate the endpoint format
//!         ValidationResult::valid()
//!     }
//!
//!     async fn execute_payment(
//!         &self,
//!         endpoint: &EndpointData,
//!         amount: &Amount,
//!         metadata: &Value,
//!     ) -> Result<PaymentExecution> {
//!         // Execute the payment
//!         todo!()
//!     }
//!
//!     fn generate_proof(&self, execution: &PaymentExecution) -> Result<PaymentProof> {
//!         // Generate proof of payment
//!         todo!()
//!     }
//!
//!     fn format_receipt_metadata(&self, execution: &PaymentExecution) -> Value {
//!         // Format metadata for receipt
//!         serde_json::json!({})
//!     }
//! }
//! ```

mod traits;
mod registry;
mod onchain;
mod lightning;
mod executor;

// Re-export core traits and types
pub use traits::{
    Amount,
    PaymentExecution,
    PaymentMethodPlugin,
    PaymentProof,
    ValidationResult,
};

// Re-export registry
pub use registry::{PaymentMethodRegistry, global};

// Re-export built-in plugins
pub use onchain::{OnchainPlugin, BitcoinNetwork, verify_bitcoin_proof};
pub use lightning::{LightningPlugin, LightningNetwork, verify_lightning_proof};

// Re-export executor traits and types
pub use executor::{
    BitcoinExecutor,
    BitcoinTxResult,
    LightningExecutor,
    LightningPaymentResult,
    LightningPaymentStatus,
    DecodedInvoice,
    MockBitcoinExecutor,
    MockLightningExecutor,
};

/// Convenience function to create a registry with all built-in plugins.
pub fn default_registry() -> PaymentMethodRegistry {
    let registry = PaymentMethodRegistry::new();
    registry.register(Box::new(OnchainPlugin::new()));
    registry.register(Box::new(LightningPlugin::new()));
    registry
}

/// Convenience function to create a registry for testnet.
pub fn testnet_registry() -> PaymentMethodRegistry {
    let registry = PaymentMethodRegistry::new();
    registry.register(Box::new(OnchainPlugin::with_network(BitcoinNetwork::Testnet)));
    registry.register(Box::new(LightningPlugin::with_network(LightningNetwork::Testnet)));
    registry
}

/// Convenience function to create a registry for regtest.
pub fn regtest_registry() -> PaymentMethodRegistry {
    let registry = PaymentMethodRegistry::new();
    registry.register(Box::new(OnchainPlugin::with_network(BitcoinNetwork::Regtest)));
    registry.register(Box::new(LightningPlugin::with_network(LightningNetwork::Regtest)));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MethodId;

    #[test]
    fn test_default_registry() {
        let registry = default_registry();
        
        assert!(registry.has_method(&MethodId("onchain".into())));
        assert!(registry.has_method(&MethodId("lightning".into())));
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_testnet_registry() {
        let registry = testnet_registry();
        
        assert!(registry.has_method(&MethodId("onchain".into())));
        assert!(registry.has_method(&MethodId("lightning".into())));
        
        // Verify network configuration
        let onchain = registry.get(&MethodId("onchain".into())).unwrap();
        assert_eq!(onchain.method_id().0, "onchain");
    }

    #[test]
    fn test_plugin_validation() {
        let registry = default_registry();
        
        // Test onchain validation
        let onchain = registry.get(&MethodId("onchain".into())).unwrap();
        let result = onchain.validate_endpoint(&crate::EndpointData(
            "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".into()
        ));
        assert!(result.valid);
        
        // Test lightning validation
        let lightning = registry.get(&MethodId("lightning".into())).unwrap();
        let result = lightning.validate_endpoint(&crate::EndpointData(
            "lnurl1dp68gurn8ghj7um9wfmxjcm99e3k7mf0v9cxj0m385ekvcenxc6r2c35xvukxefcv5mkvv34x5ekzd3ev56nyd3hxqyf3ex".into()
        ));
        assert!(result.valid);
    }

    #[test]
    fn test_plugin_amount_support() {
        let registry = default_registry();
        
        let onchain = registry.get(&MethodId("onchain".into())).unwrap();
        let lightning = registry.get(&MethodId("lightning".into())).unwrap();
        
        // Both support 10000 sats
        let amount = Amount::sats(10000);
        assert!(onchain.supports_amount(&amount));
        assert!(lightning.supports_amount(&amount));
        
        // Onchain doesn't support dust
        let dust = Amount::sats(100);
        assert!(!onchain.supports_amount(&dust));
        assert!(lightning.supports_amount(&dust));
    }

    #[test]
    fn test_confirmation_times() {
        let registry = default_registry();
        
        let onchain = registry.get(&MethodId("onchain".into())).unwrap();
        let lightning = registry.get(&MethodId("lightning".into())).unwrap();
        
        // On-chain is slow, lightning is fast
        assert!(onchain.estimated_confirmation_time().unwrap() > 
                lightning.estimated_confirmation_time().unwrap());
    }
}
