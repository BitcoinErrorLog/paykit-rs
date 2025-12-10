//! Custom Payment Method Plugin Example
//!
//! This example demonstrates how to create a custom payment method plugin.
//! It implements a simple "demo" payment method that simulates payments.

use async_trait::async_trait;
use paykit_lib::methods::{
    Amount, PaymentExecution, PaymentMethodPlugin, PaymentMethodRegistry, PaymentProof,
    ValidationResult,
};
use paykit_lib::{EndpointData, MethodId, PaykitError, Result};
use serde_json::Value;

/// A demonstration payment method plugin.
///
/// This plugin shows the structure of a custom payment method.
/// It validates "demo://" URIs and simulates payment execution.
struct DemoPaymentPlugin {
    /// Simulated success rate (0.0 - 1.0).
    success_rate: f64,
}

impl DemoPaymentPlugin {
    /// Create a new demo plugin with 100% success rate.
    fn new() -> Self {
        Self { success_rate: 1.0 }
    }

    /// Create a demo plugin with a specific success rate.
    #[allow(dead_code)]
    fn with_success_rate(rate: f64) -> Self {
        Self {
            success_rate: rate.clamp(0.0, 1.0),
        }
    }

    /// Validate a demo URI.
    fn validate_demo_uri(&self, uri: &str) -> ValidationResult {
        if uri.starts_with("demo://") {
            let path = &uri[7..]; // Strip "demo://"
            if path.is_empty() {
                ValidationResult::invalid(vec!["Demo URI path is empty".to_string()])
            } else if path.contains(' ') {
                ValidationResult::invalid(vec!["Demo URI cannot contain spaces".to_string()])
            } else {
                ValidationResult::valid()
            }
        } else {
            ValidationResult::invalid(vec![format!(
                "Expected demo:// URI, got: {}",
                &uri[..uri.len().min(20)]
            )])
        }
    }
}

#[async_trait]
impl PaymentMethodPlugin for DemoPaymentPlugin {
    fn method_id(&self) -> MethodId {
        MethodId("demo".to_string())
    }

    fn display_name(&self) -> &str {
        "Demo Payment"
    }

    fn description(&self) -> &str {
        "A demonstration payment method for testing and development."
    }

    fn validate_endpoint(&self, data: &EndpointData) -> ValidationResult {
        self.validate_demo_uri(&data.0)
    }

    async fn execute_payment(
        &self,
        endpoint: &EndpointData,
        amount: &Amount,
        metadata: &Value,
    ) -> Result<PaymentExecution> {
        // Validate the endpoint
        let validation = self.validate_endpoint(endpoint);
        if !validation.valid {
            return Err(PaykitError::Transport(validation.errors.join(", ")));
        }

        // Simulate random failure based on success rate
        let success = rand_success(self.success_rate);

        if success {
            // Generate a fake transaction ID
            let tx_id = format!("demo_tx_{}", current_timestamp());

            Ok(PaymentExecution::success(
                self.method_id(),
                endpoint.clone(),
                amount.clone(),
                serde_json::json!({
                    "demo_tx_id": tx_id,
                    "endpoint": endpoint.0,
                    "metadata": metadata,
                    "simulated": true,
                }),
            ))
        } else {
            Ok(PaymentExecution::failure(
                self.method_id(),
                endpoint.clone(),
                amount.clone(),
                "Simulated payment failure".to_string(),
            ))
        }
    }

    fn generate_proof(&self, execution: &PaymentExecution) -> Result<PaymentProof> {
        let tx_id = execution
            .execution_data
            .get("demo_tx_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        Ok(PaymentProof::custom(
            self.method_id(),
            serde_json::json!({
                "demo_tx_id": tx_id,
                "verified": true,
            }),
        ))
    }

    fn format_receipt_metadata(&self, execution: &PaymentExecution) -> Value {
        serde_json::json!({
            "method": "demo",
            "demo_tx_id": execution.execution_data.get("demo_tx_id"),
            "simulated": true,
            "executed_at": execution.executed_at,
        })
    }

    fn supports_amount(&self, _amount: &Amount) -> bool {
        // Demo supports all amounts
        true
    }

    fn estimated_confirmation_time(&self) -> Option<u64> {
        // Instant confirmation for demo
        Some(0)
    }
}

/// Simulate random success based on rate.
fn rand_success(rate: f64) -> bool {
    // Simple deterministic "random" for demo purposes
    // In production, use actual RNG
    rate >= 0.5
}

/// Get current timestamp.
fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[tokio::main]
async fn main() {
    println!("=== Custom Payment Method Plugin Example ===\n");

    // Create a registry and register our custom plugin
    let registry = PaymentMethodRegistry::new();
    registry.register(Box::new(DemoPaymentPlugin::new()));

    // List all registered methods
    println!("Registered methods:");
    for method_id in registry.list_methods() {
        let plugin = registry.get(&method_id).unwrap();
        println!("  - {} ({})", plugin.display_name(), method_id.0);
        println!("    {}", plugin.description());
    }
    println!();

    // Get our demo plugin
    let demo = registry.get(&MethodId("demo".into())).unwrap();

    // Test endpoint validation
    println!("Testing endpoint validation:");
    let valid_endpoint = EndpointData("demo://payment/123".into());
    let invalid_endpoint = EndpointData("https://example.com".into());

    let result = demo.validate_endpoint(&valid_endpoint);
    println!("  demo://payment/123 -> valid: {}", result.valid);

    let result = demo.validate_endpoint(&invalid_endpoint);
    println!(
        "  https://example.com -> valid: {} ({})",
        result.valid,
        result.errors.join(", ")
    );
    println!();

    // Execute a payment
    println!("Executing a demo payment:");
    let amount = Amount::sats(10000);
    let metadata = serde_json::json!({
        "order_id": "ORD-123",
        "description": "Test payment"
    });

    match demo
        .execute_payment(&valid_endpoint, &amount, &metadata)
        .await
    {
        Ok(execution) => {
            println!("  Success: {}", execution.success);
            println!("  Amount: {}", execution.amount);
            println!("  Executed at: {}", execution.executed_at);
            println!("  Data: {}", execution.execution_data);

            // Generate proof
            if let Ok(proof) = demo.generate_proof(&execution) {
                println!("  Proof: {:?}", proof);
            }

            // Format receipt metadata
            let receipt_meta = demo.format_receipt_metadata(&execution);
            println!("  Receipt metadata: {}", receipt_meta);
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }

    println!("\n=== Example Complete ===");
}
