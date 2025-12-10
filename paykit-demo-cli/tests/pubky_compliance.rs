//! Pubky-SDK compliance tests
//!
//! These tests verify that paykit-demo-cli correctly uses pubky-SDK APIs
//! for session creation, publishing, and discovery.

mod common;

use paykit_lib::{
    AuthenticatedTransport, EndpointData, MethodId, PubkyAuthenticatedTransport,
    PubkyUnauthenticatedTransport, UnauthenticatedTransportRead,
};
use pubky::{PubkyClient, PublicStorage};
use pubky_testnet::PubkyTestnet;

#[tokio::test]
async fn test_publish_and_discover_compliance() {
    // Start testnet homeserver
    let testnet = PubkyTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver().to_string();

    // 1. Create session
    let keypair = pubky::generate_keypair();
    let public_key = keypair.public_key();

    let mut client = PubkyClient::new(&homeserver, None)
        .await
        .expect("Session creation must follow pubky-sdk spec");

    let session = client
        .signup(&keypair, &homeserver)
        .await
        .expect("Signup must succeed");

    // 2. Publish via AuthenticatedTransport
    let auth_transport = PubkyAuthenticatedTransport::new(session);

    let method_id = MethodId("lightning".to_string());
    let endpoint = EndpointData("lnbc1...test".to_string());

    auth_transport
        .upsert_payment_endpoint(&method_id, &endpoint)
        .await
        .expect("Publish must succeed");

    // 3. Query via UnauthenticatedTransport
    let public_storage =
        PublicStorage::new(&homeserver).expect("Public storage creation must succeed");
    let unauth_transport = PubkyUnauthenticatedTransport::new(public_storage);

    let methods = unauth_transport
        .fetch_supported_payments(&public_key)
        .await
        .expect("Query must succeed");

    // 4. Verify compliance with spec
    assert!(
        !methods.entries.is_empty(),
        "Published method must be discoverable"
    );
    assert_eq!(methods.entries[0].method_id.0, "lightning");
    assert_eq!(methods.entries[0].endpoint_data.0, "lnbc1...test");

    // Cleanup
    testnet.shutdown().await;
}

#[tokio::test]
async fn test_endpoint_rotation_compliance() {
    // Test that multiple publishes to same method_id replace old values
    // per pubky-sdk spec
    let testnet = PubkyTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver().to_string();

    let keypair = pubky::generate_keypair();
    let public_key = keypair.public_key();

    let mut client = PubkyClient::new(&homeserver, None)
        .await
        .expect("Failed to create client");

    let session = client
        .signup(&keypair, &homeserver)
        .await
        .expect("Failed to signup");

    let auth_transport = PubkyAuthenticatedTransport::new(session);

    // Publish initial endpoint
    let method_id = MethodId("onchain".to_string());
    let endpoint1 = EndpointData("bc1q...old".to_string());

    auth_transport
        .upsert_payment_endpoint(&method_id, &endpoint1)
        .await
        .expect("First publish must succeed");

    // Publish updated endpoint (rotation)
    let endpoint2 = EndpointData("bc1q...new".to_string());

    auth_transport
        .upsert_payment_endpoint(&method_id, &endpoint2)
        .await
        .expect("Second publish must succeed");

    // Verify only the latest endpoint is returned
    let public_storage = PublicStorage::new(&homeserver).expect("Failed to create public storage");
    let unauth_transport = PubkyUnauthenticatedTransport::new(public_storage);

    let methods = unauth_transport
        .fetch_supported_payments(&public_key)
        .await
        .expect("Query must succeed");

    assert_eq!(
        methods.entries.len(),
        1,
        "Should only have one endpoint after rotation"
    );
    assert_eq!(
        methods.entries[0].endpoint_data.0, "bc1q...new",
        "Should return the latest endpoint"
    );

    testnet.shutdown().await;
}

#[tokio::test]
async fn test_multiple_methods_compliance() {
    // Test publishing multiple payment methods
    let testnet = PubkyTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver().to_string();

    let keypair = pubky::generate_keypair();
    let public_key = keypair.public_key();

    let mut client = PubkyClient::new(&homeserver, None)
        .await
        .expect("Failed to create client");

    let session = client
        .signup(&keypair, &homeserver)
        .await
        .expect("Failed to signup");

    let auth_transport = PubkyAuthenticatedTransport::new(session);

    // Publish multiple methods
    let methods_to_publish = vec![
        (
            MethodId("onchain".to_string()),
            EndpointData("bc1q...".to_string()),
        ),
        (
            MethodId("lightning".to_string()),
            EndpointData("lnbc...".to_string()),
        ),
        (
            MethodId("liquid".to_string()),
            EndpointData("lq1...".to_string()),
        ),
    ];

    for (method_id, endpoint_data) in &methods_to_publish {
        auth_transport
            .upsert_payment_endpoint(method_id, endpoint_data)
            .await
            .expect("Publish must succeed");
    }

    // Query all methods
    let public_storage = PublicStorage::new(&homeserver).expect("Failed to create public storage");
    let unauth_transport = PubkyUnauthenticatedTransport::new(public_storage);

    let methods = unauth_transport
        .fetch_supported_payments(&public_key)
        .await
        .expect("Query must succeed");

    assert_eq!(methods.entries.len(), 3, "Should have all three methods");

    // Verify all methods are present
    for (method_id, endpoint_data) in &methods_to_publish {
        let found = methods.entries.iter().any(|entry| {
            entry.method_id.0 == method_id.0 && entry.endpoint_data.0 == endpoint_data.0
        });
        assert!(found, "Method {} should be present", method_id.0);
    }

    testnet.shutdown().await;
}
