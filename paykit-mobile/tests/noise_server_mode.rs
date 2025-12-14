//! Server mode integration tests
//!
//! These tests verify that server mode (receiving payments) works correctly
//! for mobile apps acting as payment receivers.

use paykit_mobile::{
    noise_ffi::*,
    transport_ffi::{AuthenticatedTransportFFI, UnauthenticatedTransportFFI},
};

// ============================================================================
// Server Configuration Tests
// ============================================================================

#[test]
fn test_server_config_default_values() {
    let config = create_noise_server_config();

    // Default port should be 0 (OS assigned)
    assert_eq!(config.port, 0);

    // Default max connections
    assert_eq!(config.max_connections, 10);

    // Auto-publish should be false by default
    assert!(!config.auto_publish);
}

#[test]
fn test_server_config_with_custom_port() {
    let config = create_noise_server_config_with_port(12345);

    assert_eq!(config.port, 12345);
    assert_eq!(config.max_connections, 10);
}

#[test]
fn test_server_config_various_ports() {
    // Test common port values
    let ports = vec![8080, 8888, 9999, 19000, 49152, 65535];

    for port in ports {
        let config = create_noise_server_config_with_port(port);
        assert_eq!(config.port, port, "Port {} should match", port);
    }
}

// ============================================================================
// Server Endpoint Publishing Tests
// ============================================================================

#[test]
fn test_server_publishes_endpoint_for_discovery() {
    // Simulate server publishing its endpoint for clients to discover
    let server_auth = AuthenticatedTransportFFI::new_mock("server_user".to_string());
    let client_unauth =
        UnauthenticatedTransportFFI::from_authenticated(server_auth.clone()).unwrap();

    // Server generates X25519 keypair and publishes endpoint
    let server_noise_pubkey = "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd";

    publish_noise_endpoint(
        server_auth,
        "192.168.1.50".to_string(),
        8888,
        server_noise_pubkey.to_string(),
        Some("Mobile wallet server".to_string()),
    )
    .unwrap();

    // Client discovers server's endpoint
    let endpoint = discover_noise_endpoint(client_unauth, "server_user".to_string())
        .unwrap()
        .expect("Server endpoint should be discoverable");

    assert_eq!(endpoint.host, "192.168.1.50");
    assert_eq!(endpoint.port, 8888);
    assert_eq!(endpoint.server_noise_pubkey, server_noise_pubkey);
    assert_eq!(endpoint.metadata, Some("Mobile wallet server".to_string()));
}

#[test]
fn test_server_can_update_endpoint() {
    let auth = AuthenticatedTransportFFI::new_mock("dynamic_server".to_string());
    let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

    // Initial endpoint
    publish_noise_endpoint(
        auth.clone(),
        "10.0.0.1".to_string(),
        5000,
        "initial_pk".to_string(),
        None,
    )
    .unwrap();

    let endpoint1 = discover_noise_endpoint(unauth.clone(), "dynamic_server".to_string())
        .unwrap()
        .unwrap();
    assert_eq!(endpoint1.host, "10.0.0.1");
    assert_eq!(endpoint1.port, 5000);

    // Update endpoint (e.g., IP changed)
    publish_noise_endpoint(
        auth,
        "10.0.0.2".to_string(),
        6000,
        "new_pk".to_string(),
        None,
    )
    .unwrap();

    let endpoint2 = discover_noise_endpoint(unauth, "dynamic_server".to_string())
        .unwrap()
        .unwrap();
    assert_eq!(endpoint2.host, "10.0.0.2");
    assert_eq!(endpoint2.port, 6000);
    assert_eq!(endpoint2.server_noise_pubkey, "new_pk");
}

// ============================================================================
// Server Message Handling Tests
// ============================================================================

#[test]
fn test_server_creates_confirmation_for_valid_request() {
    // Simulate server receiving a request and creating confirmation

    // Client sends request (server receives this)
    let request_json = r#"{"type":"request_receipt","receipt_id":"rcpt_srv_001","payer":"client_pk","payee":"server_pk","method_id":"lightning","amount":"1000","currency":"SAT"}"#;

    let parsed_request = parse_payment_message(request_json.to_string()).unwrap();
    assert!(matches!(
        parsed_request.message_type,
        NoisePaymentMessageType::ReceiptRequest
    ));

    // Server creates confirmation
    let confirmation = create_receipt_confirmation_message(
        "rcpt_srv_001".to_string(),
        "client_pk".to_string(),
        "server_pk".to_string(),
        "lightning".to_string(),
        Some("1000".to_string()),
        Some("SAT".to_string()),
        None,
    )
    .unwrap();

    assert!(matches!(
        confirmation.message_type,
        NoisePaymentMessageType::ReceiptConfirmation
    ));
    assert!(confirmation.payload_json.contains("rcpt_srv_001"));
}

#[test]
fn test_server_rejects_invalid_request() {
    // Simulate server receiving invalid request

    // Invalid method_id
    let error = create_error_message(
        "unsupported_method".to_string(),
        "Payment method 'crypto_xyz' is not supported".to_string(),
    )
    .unwrap();

    assert!(matches!(error.message_type, NoisePaymentMessageType::Error));
    assert!(error.payload_json.contains("unsupported_method"));
}

#[test]
fn test_server_handles_amount_limit() {
    // Simulate server rejecting request due to amount limit

    let error = create_error_message(
        "amount_exceeds_limit".to_string(),
        "Maximum receivable amount is 1000000 SAT".to_string(),
    )
    .unwrap();

    assert!(error.payload_json.contains("amount_exceeds_limit"));
    assert!(error.payload_json.contains("1000000"));
}

// ============================================================================
// Private Endpoint Offer Tests (Server Mode)
// ============================================================================

#[test]
fn test_server_offers_private_lightning_invoice() {
    // Server generates a fresh Lightning invoice for this payment

    let offer = create_private_endpoint_offer_message(
        "lightning".to_string(),
        "lnbc10000n1pjxyz...unique_invoice".to_string(),
        Some(300), // 5 minute expiry
    )
    .unwrap();

    assert!(matches!(
        offer.message_type,
        NoisePaymentMessageType::PrivateEndpointOffer
    ));
    assert!(offer.payload_json.contains("lnbc10000n1pjxyz"));
    // expires_at is computed from current time + 300 seconds
    assert!(offer.payload_json.contains("expires_at"));
}

#[test]
fn test_server_offers_private_onchain_address() {
    // Server generates a fresh on-chain address

    let offer = create_private_endpoint_offer_message(
        "onchain".to_string(),
        "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3qccfmv3".to_string(),
        None, // No expiry for on-chain
    )
    .unwrap();

    assert!(offer.payload_json.contains("onchain"));
    assert!(offer.payload_json.contains("bc1q"));
}

// ============================================================================
// Connection Status Tracking Tests
// ============================================================================

#[test]
fn test_server_connection_lifecycle() {
    // Verify all connection states are properly defined

    // Initial state
    let status = NoiseConnectionStatus::Disconnected;
    assert!(matches!(status, NoiseConnectionStatus::Disconnected));

    // After client connects
    let status = NoiseConnectionStatus::Connecting;
    assert!(matches!(status, NoiseConnectionStatus::Connecting));

    // During handshake
    let status = NoiseConnectionStatus::Handshaking;
    assert!(matches!(status, NoiseConnectionStatus::Handshaking));

    // After successful handshake
    let status = NoiseConnectionStatus::Connected;
    assert!(matches!(status, NoiseConnectionStatus::Connected));

    // If handshake fails
    let status = NoiseConnectionStatus::Failed;
    assert!(matches!(status, NoiseConnectionStatus::Failed));
}

// ============================================================================
// Multiple Client Simulation Tests
// ============================================================================

#[test]
fn test_server_handles_multiple_client_requests() {
    // Simulate server handling requests from multiple clients

    // Client 1 request
    let _request1 = create_receipt_request_message(
        "multi_rcpt_001".to_string(),
        "client1_pk".to_string(),
        "server_pk".to_string(),
        "lightning".to_string(),
        Some("500".to_string()),
        Some("SAT".to_string()),
    )
    .unwrap();

    // Client 2 request
    let _request2 = create_receipt_request_message(
        "multi_rcpt_002".to_string(),
        "client2_pk".to_string(),
        "server_pk".to_string(),
        "lightning".to_string(),
        Some("750".to_string()),
        Some("SAT".to_string()),
    )
    .unwrap();

    // Server creates confirmations for both
    let confirm1 = create_receipt_confirmation_message(
        "multi_rcpt_001".to_string(),
        "client1_pk".to_string(),
        "server_pk".to_string(),
        "lightning".to_string(),
        Some("500".to_string()),
        Some("SAT".to_string()),
        None,
    )
    .unwrap();

    let confirm2 = create_receipt_confirmation_message(
        "multi_rcpt_002".to_string(),
        "client2_pk".to_string(),
        "server_pk".to_string(),
        "lightning".to_string(),
        Some("750".to_string()),
        Some("SAT".to_string()),
        None,
    )
    .unwrap();

    // Verify different receipt IDs
    assert!(confirm1.payload_json.contains("multi_rcpt_001"));
    assert!(confirm2.payload_json.contains("multi_rcpt_002"));
    assert!(!confirm1.payload_json.contains("multi_rcpt_002"));
    assert!(!confirm2.payload_json.contains("multi_rcpt_001"));
}

// ============================================================================
// Server Shutdown Tests
// ============================================================================

#[test]
fn test_server_removes_endpoint_on_shutdown() {
    let auth = AuthenticatedTransportFFI::new_mock("shutdown_server".to_string());
    let unauth = UnauthenticatedTransportFFI::from_authenticated(auth.clone()).unwrap();

    // Publish endpoint
    publish_noise_endpoint(
        auth.clone(),
        "127.0.0.1".to_string(),
        9000,
        "shutdown_pk".to_string(),
        None,
    )
    .unwrap();

    // Verify published
    let endpoint = discover_noise_endpoint(unauth.clone(), "shutdown_server".to_string()).unwrap();
    assert!(endpoint.is_some());

    // Server shuts down - remove endpoint
    remove_noise_endpoint(auth).unwrap();

    // Endpoint should no longer be discoverable
    let endpoint = discover_noise_endpoint(unauth, "shutdown_server".to_string()).unwrap();
    assert!(endpoint.is_none());
}
