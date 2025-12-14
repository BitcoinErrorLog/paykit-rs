//! Integration tests for Noise FFI bindings
//!
//! These tests verify the Noise protocol FFI layer works correctly for mobile apps.
//! They test endpoint discovery, message creation/parsing, and server configuration.

use paykit_mobile::{
    noise_ffi::*,
    transport_ffi::{AuthenticatedTransportFFI, UnauthenticatedTransportFFI},
};

// ============================================================================
// Endpoint Discovery Tests
// ============================================================================

#[test]
fn test_discover_endpoint_not_found() {
    // Test discovering endpoint for user without published endpoint
    let unauth = UnauthenticatedTransportFFI::new_mock();
    let result = discover_noise_endpoint(unauth, "nonexistent_user".to_string()).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_publish_and_discover_endpoint_roundtrip() {
    // Test full publish/discover flow
    let auth = AuthenticatedTransportFFI::new_mock("test_user".to_string());
    let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

    // Publish endpoint
    let result = publish_noise_endpoint(
        auth,
        "127.0.0.1".to_string(),
        9999,
        "abcdef1234567890".to_string(),
        Some("Test endpoint metadata".to_string()),
    );
    assert!(result.is_ok());

    // Discover endpoint
    let endpoint = discover_noise_endpoint(unauth, "test_user".to_string())
        .unwrap()
        .expect("Endpoint should be found");

    assert_eq!(endpoint.host, "127.0.0.1");
    assert_eq!(endpoint.port, 9999);
    assert_eq!(endpoint.server_noise_pubkey, "abcdef1234567890");
    assert_eq!(
        endpoint.metadata,
        Some("Test endpoint metadata".to_string())
    );
    assert_eq!(endpoint.recipient_pubkey, "test_user");
}

#[test]
fn test_endpoint_connection_address() {
    let endpoint = NoiseEndpointInfo {
        recipient_pubkey: "test_pk".to_string(),
        host: "192.168.1.100".to_string(),
        port: 8888,
        server_noise_pubkey: "pubkey_hex".to_string(),
        metadata: None,
    };

    assert_eq!(endpoint.connection_address(), "192.168.1.100:8888");
}

// ============================================================================
// Message Creation Tests
// ============================================================================

#[test]
fn test_create_receipt_request_message_minimal() {
    let msg = create_receipt_request_message(
        "rcpt_001".to_string(),
        "payer_pubkey".to_string(),
        "payee_pubkey".to_string(),
        "lightning".to_string(),
        None,
        None,
    )
    .unwrap();

    assert!(matches!(
        msg.message_type,
        NoisePaymentMessageType::ReceiptRequest
    ));
    assert!(msg.payload_json.contains("request_receipt"));
    assert!(msg.payload_json.contains("rcpt_001"));
    assert!(msg.payload_json.contains("payer_pubkey"));
    assert!(msg.payload_json.contains("payee_pubkey"));
    assert!(msg.payload_json.contains("lightning"));
}

#[test]
fn test_create_receipt_request_message_full() {
    let msg = create_receipt_request_message(
        "rcpt_002".to_string(),
        "payer_pubkey".to_string(),
        "payee_pubkey".to_string(),
        "onchain".to_string(),
        Some("50000".to_string()),
        Some("SAT".to_string()),
    )
    .unwrap();

    assert!(msg.payload_json.contains("50000"));
    assert!(msg.payload_json.contains("SAT"));
    assert!(msg.payload_json.contains("onchain"));
}

#[test]
fn test_create_receipt_confirmation_message() {
    let msg = create_receipt_confirmation_message(
        "rcpt_003".to_string(),
        "payer_pubkey".to_string(),
        "payee_pubkey".to_string(),
        "lightning".to_string(),
        Some("1000".to_string()),
        Some("SAT".to_string()),
        None,
    )
    .unwrap();

    assert!(matches!(
        msg.message_type,
        NoisePaymentMessageType::ReceiptConfirmation
    ));
    assert!(msg.payload_json.contains("confirm_receipt"));
    assert!(msg.payload_json.contains("rcpt_003"));
}

#[test]
fn test_create_receipt_confirmation_with_notes() {
    let msg = create_receipt_confirmation_message(
        "rcpt_004".to_string(),
        "payer_pubkey".to_string(),
        "payee_pubkey".to_string(),
        "lightning".to_string(),
        Some("2000".to_string()),
        Some("SAT".to_string()),
        Some("Payment for coffee".to_string()),
    )
    .unwrap();

    assert!(msg.payload_json.contains("Payment for coffee"));
}

#[test]
fn test_create_private_endpoint_offer_message() {
    let msg = create_private_endpoint_offer_message(
        "lightning".to_string(),
        "lnbc1000n1...".to_string(),
        Some(3600),
    )
    .unwrap();

    assert!(matches!(
        msg.message_type,
        NoisePaymentMessageType::PrivateEndpointOffer
    ));
    assert!(msg.payload_json.contains("private_endpoint_offer"));
    assert!(msg.payload_json.contains("lnbc1000n1..."));
    // expires_at is computed from current time + expires_in_secs, so we just check it exists
    assert!(msg.payload_json.contains("expires_at"));
}

#[test]
fn test_create_private_endpoint_offer_no_expiry() {
    let msg = create_private_endpoint_offer_message(
        "onchain".to_string(),
        "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string(),
        None,
    )
    .unwrap();

    assert!(msg.payload_json.contains("onchain"));
    assert!(msg
        .payload_json
        .contains("bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"));
}

#[test]
fn test_create_error_message() {
    let msg = create_error_message(
        "insufficient_funds".to_string(),
        "The payment amount exceeds available balance".to_string(),
    )
    .unwrap();

    assert!(matches!(msg.message_type, NoisePaymentMessageType::Error));
    assert!(msg.payload_json.contains("error"));
    assert!(msg.payload_json.contains("insufficient_funds"));
    assert!(msg
        .payload_json
        .contains("The payment amount exceeds available balance"));
}

// ============================================================================
// Message Parsing Tests
// ============================================================================

#[test]
fn test_parse_receipt_request_message() {
    let json = r#"{"type":"request_receipt","receipt_id":"rcpt_100","payer":"pk1","payee":"pk2","method_id":"lightning"}"#;
    let msg = parse_payment_message(json.to_string()).unwrap();

    assert!(matches!(
        msg.message_type,
        NoisePaymentMessageType::ReceiptRequest
    ));
}

#[test]
fn test_parse_receipt_confirmation_message() {
    let json = r#"{"type":"confirm_receipt","receipt_id":"rcpt_101","payer":"pk1","payee":"pk2"}"#;
    let msg = parse_payment_message(json.to_string()).unwrap();

    assert!(matches!(
        msg.message_type,
        NoisePaymentMessageType::ReceiptConfirmation
    ));
}

#[test]
fn test_parse_private_endpoint_offer_message() {
    let json = r#"{"type":"private_endpoint_offer","method_id":"lightning","endpoint":"lnbc1..."}"#;
    let msg = parse_payment_message(json.to_string()).unwrap();

    assert!(matches!(
        msg.message_type,
        NoisePaymentMessageType::PrivateEndpointOffer
    ));
}

#[test]
fn test_parse_error_message() {
    let json = r#"{"type":"error","code":"timeout","message":"Request timed out"}"#;
    let msg = parse_payment_message(json.to_string()).unwrap();

    assert!(matches!(msg.message_type, NoisePaymentMessageType::Error));
}

#[test]
fn test_parse_unknown_message_type_fails() {
    let json = r#"{"type":"unknown_message_type","data":"some_data"}"#;
    let result = parse_payment_message(json.to_string());

    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_json_fails() {
    let json = "not valid json at all";
    let result = parse_payment_message(json.to_string());

    assert!(result.is_err());
}

#[test]
fn test_parse_missing_type_field_fails() {
    let json = r#"{"receipt_id":"rcpt_102","payer":"pk1"}"#;
    let result = parse_payment_message(json.to_string());

    assert!(result.is_err());
}

// ============================================================================
// Server Configuration Tests
// ============================================================================

#[test]
fn test_create_default_server_config() {
    let config = create_noise_server_config();

    assert_eq!(config.port, 0);
    assert_eq!(config.max_connections, 10);
    assert!(!config.auto_publish);
}

#[test]
fn test_create_server_config_with_port() {
    let config = create_noise_server_config_with_port(8765);

    assert_eq!(config.port, 8765);
    assert_eq!(config.max_connections, 10);
}

// ============================================================================
// End-to-End Flow Tests
// ============================================================================

#[test]
fn test_complete_payment_message_exchange_flow() {
    // Simulate a complete payment flow using message creation and parsing

    // Step 1: Payer creates receipt request
    let request_msg = create_receipt_request_message(
        "e2e_rcpt_001".to_string(),
        "payer_e2e".to_string(),
        "payee_e2e".to_string(),
        "lightning".to_string(),
        Some("5000".to_string()),
        Some("SAT".to_string()),
    )
    .unwrap();

    // Simulate sending over network (serialize -> deserialize)
    let parsed_request = parse_payment_message(request_msg.payload_json.clone()).unwrap();
    assert!(matches!(
        parsed_request.message_type,
        NoisePaymentMessageType::ReceiptRequest
    ));

    // Step 2: Payee creates confirmation
    let confirm_msg = create_receipt_confirmation_message(
        "e2e_rcpt_001".to_string(),
        "payer_e2e".to_string(),
        "payee_e2e".to_string(),
        "lightning".to_string(),
        Some("5000".to_string()),
        Some("SAT".to_string()),
        None,
    )
    .unwrap();

    // Simulate sending back
    let parsed_confirm = parse_payment_message(confirm_msg.payload_json.clone()).unwrap();
    assert!(matches!(
        parsed_confirm.message_type,
        NoisePaymentMessageType::ReceiptConfirmation
    ));
}

#[test]
fn test_payment_flow_with_error() {
    // Simulate a payment flow that results in an error

    // Step 1: Payer creates receipt request
    let _request_msg = create_receipt_request_message(
        "error_rcpt_001".to_string(),
        "payer".to_string(),
        "payee".to_string(),
        "lightning".to_string(),
        Some("999999999".to_string()), // Very large amount
        Some("SAT".to_string()),
    )
    .unwrap();

    // Step 2: Payee rejects with error
    let error_msg = create_error_message(
        "amount_too_large".to_string(),
        "Requested amount exceeds maximum allowed".to_string(),
    )
    .unwrap();

    // Verify error can be parsed
    let parsed_error = parse_payment_message(error_msg.payload_json.clone()).unwrap();
    assert!(matches!(
        parsed_error.message_type,
        NoisePaymentMessageType::Error
    ));
}

#[test]
fn test_private_endpoint_negotiation_flow() {
    // Simulate private endpoint negotiation

    // Step 1: After receiving payment request, payee offers private endpoint
    let offer_msg = create_private_endpoint_offer_message(
        "lightning".to_string(),
        "lnbc50000n1pj...invoice".to_string(),
        Some(600), // 10 minute expiry
    )
    .unwrap();

    let parsed_offer = parse_payment_message(offer_msg.payload_json.clone()).unwrap();
    assert!(matches!(
        parsed_offer.message_type,
        NoisePaymentMessageType::PrivateEndpointOffer
    ));

    // Verify offer contains expected data
    assert!(offer_msg.payload_json.contains("lightning"));
    assert!(offer_msg.payload_json.contains("lnbc50000n1pj"));
    // expires_at is computed from current time + 600 seconds, so we just check it exists
    assert!(offer_msg.payload_json.contains("expires_at"));
}

// ============================================================================
// Endpoint Removal Tests
// ============================================================================

#[test]
fn test_remove_noise_endpoint() {
    let auth = AuthenticatedTransportFFI::new_mock("removal_test_user".to_string());
    let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

    // First publish
    publish_noise_endpoint(
        auth.clone(),
        "127.0.0.1".to_string(),
        7777,
        "remove_test_pk".to_string(),
        None,
    )
    .unwrap();

    // Verify it exists
    let endpoint = discover_noise_endpoint(unauth.clone(), "removal_test_user".to_string())
        .unwrap()
        .expect("Should exist");
    assert_eq!(endpoint.port, 7777);

    // Remove it
    remove_noise_endpoint(auth).unwrap();

    // Verify it's gone
    let result = discover_noise_endpoint(unauth, "removal_test_user".to_string()).unwrap();
    assert!(result.is_none());
}

// ============================================================================
// Connection Status Tests
// ============================================================================

#[test]
fn test_connection_status_enum_values() {
    // Verify all connection status values are accessible
    let statuses = vec![
        NoiseConnectionStatus::Disconnected,
        NoiseConnectionStatus::Connecting,
        NoiseConnectionStatus::Handshaking,
        NoiseConnectionStatus::Connected,
        NoiseConnectionStatus::Failed,
    ];

    for status in statuses {
        match status {
            NoiseConnectionStatus::Disconnected => assert!(true),
            NoiseConnectionStatus::Connecting => assert!(true),
            NoiseConnectionStatus::Handshaking => assert!(true),
            NoiseConnectionStatus::Connected => assert!(true),
            NoiseConnectionStatus::Failed => assert!(true),
        }
    }
}

#[test]
fn test_handshake_result_success() {
    let result = NoiseHandshakeResult {
        success: true,
        session_id: Some("session_123".to_string()),
        remote_pubkey: Some("remote_pk_z32".to_string()),
        error: None,
    };

    assert!(result.success);
    assert_eq!(result.session_id, Some("session_123".to_string()));
    assert!(result.error.is_none());
}

#[test]
fn test_handshake_result_failure() {
    let result = NoiseHandshakeResult {
        success: false,
        session_id: None,
        remote_pubkey: None,
        error: Some("Connection refused".to_string()),
    };

    assert!(!result.success);
    assert!(result.session_id.is_none());
    assert_eq!(result.error, Some("Connection refused".to_string()));
}

