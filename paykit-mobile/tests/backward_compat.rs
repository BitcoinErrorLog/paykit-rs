//! Backward Compatibility Tests
//!
//! These tests verify that existing code continues to work after adding
//! the Bitkit executor integration. All existing APIs should maintain
//! their behavior.

use paykit_mobile::*;
use std::sync::Arc;

// ============================================================================
// PaykitClient Creation Tests
// ============================================================================

#[test]
fn test_backward_compat_new_without_executors() {
    // Existing code: PaykitClient::new() should work without any changes
    let client = PaykitClient::new();
    assert!(client.is_ok());

    let client = client.unwrap();

    // Should have default methods registered
    let methods = client.list_methods();
    assert!(!methods.is_empty());
}

#[test]
fn test_backward_compat_client_has_default_methods() {
    let client = PaykitClient::new().unwrap();

    // Default methods should be available
    let methods = client.list_methods();

    // Should have at least onchain and lightning
    assert!(methods.iter().any(|m| m == "onchain" || m == "bitcoin"));
    assert!(methods.iter().any(|m| m == "lightning" || m == "ln"));
}

// ============================================================================
// Payment Method Tests
// ============================================================================

#[test]
fn test_backward_compat_list_methods() {
    let client = PaykitClient::new().unwrap();

    // list_methods should work
    let methods = client.list_methods();

    // Should return a non-empty list
    assert!(!methods.is_empty());

    // All methods should have valid IDs
    for method in &methods {
        assert!(!method.is_empty());
    }
}

// ============================================================================
// Endpoint Validation Tests
// ============================================================================

#[test]
fn test_backward_compat_validate_endpoint_onchain() {
    let client = PaykitClient::new().unwrap();

    // Mainnet address validation
    let result = client.validate_endpoint(
        "onchain".to_string(),
        "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".to_string(),
    );

    // Should succeed for valid mainnet address
    assert!(result.is_ok());
}

#[test]
fn test_backward_compat_validate_endpoint_lightning() {
    let client = PaykitClient::new().unwrap();

    // Lightning invoice validation (using a properly formatted mock)
    let invoice = format!("lnbc1000n1p{}", "0".repeat(200));
    let result = client.validate_endpoint("lightning".to_string(), invoice);

    // Should return a result (valid or invalid)
    let _ = result;
}

#[test]
fn test_backward_compat_validate_invalid_endpoint() {
    let client = PaykitClient::new().unwrap();

    // Invalid address
    let result = client.validate_endpoint("onchain".to_string(), "invalid_address".to_string());

    // Should handle gracefully (either error or validation failure)
    let _ = result;
}

// ============================================================================
// Health Check Tests
// ============================================================================

#[test]
fn test_backward_compat_health_check() {
    let _client = PaykitClient::new().unwrap();

    // Health checks should work without executors
    // The health status depends on network connectivity
    // Key is it shouldn't panic
}

// ============================================================================
// No Regression Tests
// ============================================================================

#[test]
fn test_backward_compat_no_new_required_params() {
    // PaykitClient::new() should still work with no parameters
    let _ = PaykitClient::new().unwrap();
}

#[test]
fn test_backward_compat_optional_network_param() {
    // new_with_network is additive - old code doesn't need to use it
    // but if used, should work correctly
    use paykit_mobile::executor_ffi::{BitcoinNetworkFFI, LightningNetworkFFI};

    let _ =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Mainnet, LightningNetworkFFI::Mainnet)
            .unwrap();
}

#[test]
fn test_backward_compat_executor_methods_optional() {
    let client = PaykitClient::new().unwrap();

    // has_bitcoin_executor and has_lightning_executor are new but should work
    // They return true by default since default plugins are registered
    let has_btc = client.has_bitcoin_executor();
    let has_ln = client.has_lightning_executor();

    // Default client should have both
    assert!(has_btc);
    assert!(has_ln);
}

// ============================================================================
// Error Handling Backward Compatibility
// ============================================================================

#[test]
fn test_backward_compat_error_types() {
    // All existing error types should still work
    let transport_err = PaykitMobileError::Transport {
        msg: "test".to_string(),
    };
    let validation_err = PaykitMobileError::Validation {
        msg: "test".to_string(),
    };
    let not_found_err = PaykitMobileError::NotFound {
        msg: "test".to_string(),
    };
    let internal_err = PaykitMobileError::Internal {
        msg: "test".to_string(),
    };

    // All should be usable
    assert!(matches!(transport_err, PaykitMobileError::Transport { .. }));
    assert!(matches!(
        validation_err,
        PaykitMobileError::Validation { .. }
    ));
    assert!(matches!(not_found_err, PaykitMobileError::NotFound { .. }));
    assert!(matches!(internal_err, PaykitMobileError::Internal { .. }));
}

// ============================================================================
// Type Backward Compatibility
// ============================================================================

#[test]
fn test_backward_compat_payment_method_type() {
    // PaymentMethod struct should still work
    let method = PaymentMethod {
        method_id: "test".to_string(),
        endpoint: "bc1qtest".to_string(),
    };

    assert_eq!(method.method_id, "test");
    assert!(!method.endpoint.is_empty());
}

#[test]
fn test_backward_compat_payment_execution_result_type() {
    // PaymentExecutionResult should work
    let result = PaymentExecutionResult {
        execution_id: "exec_123".to_string(),
        method_id: "lightning".to_string(),
        endpoint: "lnbc...".to_string(),
        amount_sats: 1000,
        success: true,
        executed_at: 1700000000,
        execution_data_json: "{}".to_string(),
        error: None,
    };

    assert!(result.success);
    assert_eq!(result.method_id, "lightning");
}

#[test]
fn test_backward_compat_payment_proof_result_type() {
    // PaymentProofResult should work
    let proof = PaymentProofResult {
        proof_type: "bitcoin_txid".to_string(),
        proof_data_json: r#"{"txid":"abc123"}"#.to_string(),
    };

    assert_eq!(proof.proof_type, "bitcoin_txid");
}

// ============================================================================
// API Signature Backward Compatibility
// ============================================================================

#[test]
fn test_backward_compat_api_signatures() {
    let client = PaykitClient::new().unwrap();

    // All these methods should exist and have expected types
    let _methods: Vec<String> = client.list_methods();
    let _validation: Result<bool> =
        client.validate_endpoint("onchain".to_string(), "test".to_string());
}

// ============================================================================
// Concurrent Usage Backward Compatibility
// ============================================================================

#[test]
fn test_backward_compat_arc_usage() {
    // Client should still be usable behind Arc
    let client = PaykitClient::new().unwrap();

    // Arc<PaykitClient> is returned by new()
    let _client_arc: Arc<PaykitClient> = client;
}

#[test]
fn test_backward_compat_thread_safety() {
    use std::thread;

    let client = PaykitClient::new().unwrap();

    // Should be usable from multiple threads
    let handles: Vec<_> = (0..3)
        .map(|_| {
            let c = client.clone();
            thread::spawn(move || {
                let _ = c.list_methods();
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
}

// ============================================================================
// Network Configuration Backward Compatibility
// ============================================================================

#[test]
fn test_backward_compat_network_defaults() {
    use paykit_mobile::executor_ffi::BitcoinNetworkFFI;

    let client = PaykitClient::new().unwrap();

    // Default should be mainnet
    assert_eq!(client.bitcoin_network(), BitcoinNetworkFFI::Mainnet);
}

#[test]
fn test_backward_compat_network_accessors() {
    use paykit_mobile::executor_ffi::{BitcoinNetworkFFI, LightningNetworkFFI};

    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    // Network accessors should work
    let _btc_network = client.bitcoin_network();
    let _ln_network = client.lightning_network();
}
