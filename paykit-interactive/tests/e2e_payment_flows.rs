//! End-to-end payment flow tests
//!
//! These tests verify complete payment scenarios including:
//! - Multiple concurrent payments
//! - Error recovery scenarios
//! - Rate limiting integration
//! - Full payment lifecycle with receipts

mod mock_implementations;

use mock_implementations::{MockNoiseChannel, MockReceiptGenerator, MockStorage};
use paykit_interactive::{
    rate_limit::{HandshakeRateLimiter, RateLimitConfig},
    PaykitInteractiveManager, PaykitNoiseChannel, PaykitNoiseMessage, PaykitReceipt, PaykitStorage,
    ReceiptGenerator,
};
use paykit_lib::MethodId;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

/// Helper to create test public keys
fn test_pubkey(_name: &str) -> paykit_lib::PublicKey {
    let keypair = pubky::Keypair::random();
    keypair.public_key()
}

/// Helper to create a manager with fresh mocks
fn create_manager() -> (
    PaykitInteractiveManager,
    Arc<Box<dyn PaykitStorage>>,
    Arc<Box<dyn ReceiptGenerator>>,
) {
    let storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let generator = Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let manager = PaykitInteractiveManager::new(storage.clone(), generator.clone());
    (manager, storage, generator)
}

/// Helper to create a provisional receipt
fn create_receipt(
    receipt_id: &str,
    payer: &paykit_lib::PublicKey,
    payee: &paykit_lib::PublicKey,
    amount: &str,
) -> PaykitReceipt {
    PaykitReceipt::new(
        receipt_id.to_string(),
        payer.clone(),
        payee.clone(),
        MethodId("lightning".to_string()),
        Some(amount.to_string()),
        Some("SAT".to_string()),
        json!({"test": true}),
    )
}

// =============================================================================
// E2E Test: Multiple Concurrent Payments
// =============================================================================

#[tokio::test]
async fn test_multiple_concurrent_payments() {
    let payer_pk = test_pubkey("payer");
    let payee_pk = test_pubkey("payee");

    let (payer_manager, payer_storage, _) = create_manager();
    let (payee_manager, payee_storage, _) = create_manager();
    let payee_manager = Arc::new(payee_manager);

    // Create 5 concurrent payment flows
    let mut handles = Vec::new();

    for i in 0..5 {
        let (mut payer_channel, mut payee_channel) = MockNoiseChannel::pair();
        let receipt = create_receipt(
            &format!("concurrent_receipt_{}", i),
            &payer_pk,
            &payee_pk,
            &format!("{}", 1000 + i * 100),
        );
        let payer_mgr = PaykitInteractiveManager::new(
            Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>),
            Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>),
        );
        let payee_mgr = payee_manager.clone();
        let payer_pk_clone = payer_pk.clone();
        let payee_pk_clone = payee_pk.clone();

        // Spawn payee handler
        let payee_handle = tokio::spawn(async move {
            let msg = payee_channel.recv().await.unwrap();
            let response = payee_mgr
                .handle_message(msg, &payer_pk_clone, &payee_pk_clone)
                .await
                .unwrap();
            if let Some(response_msg) = response {
                payee_channel.send(response_msg).await.unwrap();
            }
        });

        // Spawn payer
        let payer_handle = tokio::spawn(async move {
            let result = payer_mgr
                .initiate_payment(&mut payer_channel, receipt)
                .await;
            (result, payee_handle)
        });

        handles.push(payer_handle);
    }

    // Wait for all payments to complete
    let mut success_count = 0;
    for handle in handles {
        let (result, payee_handle) = handle.await.unwrap();
        payee_handle.await.unwrap();
        if result.is_ok() {
            success_count += 1;
        }
    }

    // All payments should succeed
    assert_eq!(success_count, 5, "All concurrent payments should succeed");
}

// =============================================================================
// E2E Test: Payment with Private Endpoint Exchange
// =============================================================================

#[tokio::test]
async fn test_payment_with_private_endpoint() {
    let merchant_pk = test_pubkey("merchant");
    let customer_pk = test_pubkey("customer");

    let (merchant_manager, merchant_storage, _) = create_manager();
    let (customer_manager, customer_storage, _) = create_manager();

    // Phase 1: Merchant offers private endpoint
    let (mut merchant_channel, mut customer_channel) = MockNoiseChannel::pair();

    // Spawn customer to receive and store endpoint
    let merchant_pk_clone = merchant_pk.clone();
    let customer_mgr = Arc::new(customer_manager);
    let customer_mgr_clone = customer_mgr.clone();

    let customer_handle = tokio::spawn(async move {
        let msg = customer_channel.recv().await.unwrap();
        match &msg {
            PaykitNoiseMessage::OfferPrivateEndpoint {
                method_id,
                endpoint,
            } => {
                assert_eq!(method_id.0, "lightning");
                assert!(endpoint.starts_with("lnbc"));
            }
            _ => panic!("Expected OfferPrivateEndpoint"),
        }
        let response = customer_mgr_clone
            .handle_message(msg, &merchant_pk_clone, &test_pubkey("customer"))
            .await
            .unwrap();
        assert!(matches!(response, Some(PaykitNoiseMessage::Ack)));
    });

    // Merchant sends private endpoint
    merchant_manager
        .offer_private_endpoint(
            &mut merchant_channel,
            MethodId("lightning".to_string()),
            "lnbc10000n1...private_invoice".to_string(),
        )
        .await
        .unwrap();

    customer_handle.await.unwrap();

    // Phase 2: Customer initiates payment using stored endpoint
    let (mut customer_pay_channel, mut merchant_pay_channel) = MockNoiseChannel::pair();

    let receipt = create_receipt("private_payment_001", &customer_pk, &merchant_pk, "10000");

    let merchant_mgr = Arc::new(merchant_manager);
    let customer_pk_clone = customer_pk.clone();
    let merchant_pk_clone = merchant_pk.clone();

    let merchant_handle = tokio::spawn(async move {
        let msg = merchant_pay_channel.recv().await.unwrap();
        let response = merchant_mgr
            .handle_message(msg, &customer_pk_clone, &merchant_pk_clone)
            .await
            .unwrap();
        if let Some(response_msg) = response {
            merchant_pay_channel.send(response_msg).await.unwrap();
        }
    });

    let final_receipt = customer_mgr
        .initiate_payment(&mut customer_pay_channel, receipt)
        .await
        .unwrap();

    merchant_handle.await.unwrap();

    assert_eq!(final_receipt.receipt_id, "private_payment_001");
    assert!(final_receipt.metadata.get("invoice").is_some());
}

// =============================================================================
// E2E Test: Receipt Persistence and Recovery
// =============================================================================

#[tokio::test]
async fn test_receipt_persistence() {
    let payer_pk = test_pubkey("payer");
    let payee_pk = test_pubkey("payee");

    let payer_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payer_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payer_manager = PaykitInteractiveManager::new(payer_storage.clone(), payer_generator);

    let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payee_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payee_manager = PaykitInteractiveManager::new(payee_storage.clone(), payee_generator);

    let (mut payer_channel, mut payee_channel) = MockNoiseChannel::pair();

    let receipt_id = format!("persist_test_{}", chrono_now());
    let receipt = create_receipt(&receipt_id, &payer_pk, &payee_pk, "5000");

    let payer_pk_clone = payer_pk.clone();
    let payee_pk_clone = payee_pk.clone();

    // Payee handles request
    let payee_handle = tokio::spawn(async move {
        let msg = payee_channel.recv().await.unwrap();
        let response = payee_manager
            .handle_message(msg, &payer_pk_clone, &payee_pk_clone)
            .await
            .unwrap();
        if let Some(response_msg) = response {
            payee_channel.send(response_msg).await.unwrap();
        }
    });

    // Payer initiates
    let final_receipt = payer_manager
        .initiate_payment(&mut payer_channel, receipt)
        .await
        .unwrap();

    payee_handle.await.unwrap();

    // Verify payer has receipt stored
    let stored = payer_storage.get_receipt(&receipt_id).await.unwrap();
    assert!(stored.is_some());
    let stored_receipt = stored.unwrap();
    assert_eq!(stored_receipt.receipt_id, receipt_id);
    assert_eq!(stored_receipt.payer, payer_pk);
    assert_eq!(stored_receipt.payee, payee_pk);

    // Simulate "recovery" - verify receipt can be retrieved later
    let recovered = payer_storage.get_receipt(&receipt_id).await.unwrap();
    assert!(recovered.is_some());
    assert_eq!(recovered.unwrap().receipt_id, final_receipt.receipt_id);
}

// =============================================================================
// E2E Test: Rate Limiter Integration
// =============================================================================

#[tokio::test]
async fn test_rate_limiter_integration() {
    use std::net::IpAddr;

    // Create strict rate limiter (2 attempts per minute)
    let config = RateLimitConfig::new(2, 60, 100);
    let limiter = HandshakeRateLimiter::new(config);

    let test_ip: IpAddr = "192.168.1.100".parse().unwrap();

    // First two attempts should succeed
    assert!(limiter.check_and_record(test_ip));
    assert!(limiter.check_and_record(test_ip));

    // Third attempt should be blocked
    assert!(!limiter.check_and_record(test_ip));

    // Different IP should still work
    let other_ip: IpAddr = "192.168.1.101".parse().unwrap();
    assert!(limiter.check_and_record(other_ip));

    // Reset the first IP
    limiter.reset(test_ip);

    // Should be allowed again
    assert!(limiter.check_and_record(test_ip));
}

// =============================================================================
// E2E Test: Error Recovery - Malformed Response
// =============================================================================

#[tokio::test]
async fn test_error_recovery_malformed_response() {
    let payer_pk = test_pubkey("payer");
    let payee_pk = test_pubkey("payee");

    let (payer_manager, _, _) = create_manager();

    let (mut payer_channel, mut payee_channel) = MockNoiseChannel::pair();

    let receipt = create_receipt("error_recovery_001", &payer_pk, &payee_pk, "1000");

    // Spawn malicious payee that sends wrong message type
    tokio::spawn(async move {
        let _msg = payee_channel.recv().await.unwrap();
        // Send wrong message type instead of ConfirmReceipt
        payee_channel
            .send(PaykitNoiseMessage::Error {
                code: "UNEXPECTED".to_string(),
                message: "Server error".to_string(),
            })
            .await
            .unwrap();
    });

    // Payer should handle this gracefully
    let result = payer_manager
        .initiate_payment(&mut payer_channel, receipt)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("protocol") || err.to_string().contains("Expected"));
}

// =============================================================================
// E2E Test: Multiple Payment Methods
// =============================================================================

#[tokio::test]
async fn test_multiple_payment_methods() {
    let merchant_pk = test_pubkey("merchant");
    let customer_pk = test_pubkey("customer");

    let (merchant_manager, merchant_storage, _) = create_manager();

    // Merchant offers multiple payment methods
    let methods = vec![
        ("lightning", "lnbc1000..."),
        ("onchain", "bc1q..."),
        ("liquid", "VJL..."),
    ];

    for (method, endpoint) in methods {
        let (mut merchant_channel, mut customer_channel) = MockNoiseChannel::pair();

        // Spawn customer to receive
        let method_clone = method.to_string();
        let endpoint_clone = endpoint.to_string();
        let customer_handle = tokio::spawn(async move {
            let msg = customer_channel.recv().await.unwrap();
            match msg {
                PaykitNoiseMessage::OfferPrivateEndpoint {
                    method_id,
                    endpoint,
                } => {
                    assert_eq!(method_id.0, method_clone);
                    assert!(endpoint.starts_with(&endpoint_clone[..3]));
                }
                _ => panic!("Expected OfferPrivateEndpoint"),
            }
        });

        merchant_manager
            .offer_private_endpoint(
                &mut merchant_channel,
                MethodId(method.to_string()),
                endpoint.to_string(),
            )
            .await
            .unwrap();

        customer_handle.await.unwrap();
    }
}

// =============================================================================
// E2E Test: Large Metadata Handling
// =============================================================================

#[tokio::test]
async fn test_large_metadata() {
    let payer_pk = test_pubkey("payer");
    let payee_pk = test_pubkey("payee");

    let (payer_manager, _, _) = create_manager();
    let (payee_manager, _, _) = create_manager();

    let (mut payer_channel, mut payee_channel) = MockNoiseChannel::pair();

    // Create receipt with large metadata
    let large_metadata = json!({
        "order_id": "LARGE-ORDER-001",
        "items": (0..100).map(|i| json!({
            "id": format!("item_{}", i),
            "name": format!("Product {} with a very long description to test large payloads", i),
            "price": i * 100,
            "quantity": (i % 5) + 1,
        })).collect::<Vec<_>>(),
        "shipping": {
            "address": "123 Test Street, Test City, TC 12345, Test Country",
            "method": "express",
            "tracking": "TRK123456789012345678901234567890"
        },
        "notes": "x".repeat(1000), // 1KB of notes
    });

    let receipt = PaykitReceipt::new(
        "large_metadata_001".to_string(),
        payer_pk.clone(),
        payee_pk.clone(),
        MethodId("lightning".to_string()),
        Some("50000".to_string()),
        Some("SAT".to_string()),
        large_metadata.clone(),
    );

    let payer_pk_clone = payer_pk.clone();
    let payee_pk_clone = payee_pk.clone();

    let payee_handle = tokio::spawn(async move {
        let msg = payee_channel.recv().await.unwrap();
        let response = payee_manager
            .handle_message(msg, &payer_pk_clone, &payee_pk_clone)
            .await
            .unwrap();
        if let Some(response_msg) = response {
            payee_channel.send(response_msg).await.unwrap();
        }
    });

    let final_receipt = payer_manager
        .initiate_payment(&mut payer_channel, receipt)
        .await
        .unwrap();

    payee_handle.await.unwrap();

    // Verify metadata was preserved
    assert!(final_receipt.metadata.get("order_id").is_some());
    assert!(final_receipt.metadata.get("items").is_some());
}

// =============================================================================
// E2E Test: Sequential Payments Same Peer
// =============================================================================

#[tokio::test]
async fn test_sequential_payments_same_peer() {
    let payer_pk = test_pubkey("payer");
    let payee_pk = test_pubkey("payee");

    let payer_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payer_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payer_manager = PaykitInteractiveManager::new(payer_storage.clone(), payer_generator);

    let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payee_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payee_manager = Arc::new(PaykitInteractiveManager::new(
        payee_storage.clone(),
        payee_generator,
    ));

    // Make 3 sequential payments to same payee
    for i in 0..3 {
        let (mut payer_channel, mut payee_channel) = MockNoiseChannel::pair();

        let receipt = create_receipt(
            &format!("sequential_{}", i),
            &payer_pk,
            &payee_pk,
            &format!("{}", (i + 1) * 1000),
        );

        let payee_mgr = payee_manager.clone();
        let payer_pk_clone = payer_pk.clone();
        let payee_pk_clone = payee_pk.clone();

        let payee_handle = tokio::spawn(async move {
            let msg = payee_channel.recv().await.unwrap();
            let response = payee_mgr
                .handle_message(msg, &payer_pk_clone, &payee_pk_clone)
                .await
                .unwrap();
            if let Some(response_msg) = response {
                payee_channel.send(response_msg).await.unwrap();
            }
        });

        let result = payer_manager
            .initiate_payment(&mut payer_channel, receipt)
            .await;

        payee_handle.await.unwrap();

        assert!(result.is_ok(), "Payment {} should succeed", i);
        let final_receipt = result.unwrap();
        assert_eq!(final_receipt.receipt_id, format!("sequential_{}", i));
    }

    // Verify all 3 receipts are stored
    for i in 0..3 {
        let stored = payer_storage
            .get_receipt(&format!("sequential_{}", i))
            .await
            .unwrap();
        assert!(stored.is_some(), "Receipt {} should be stored", i);
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn chrono_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
