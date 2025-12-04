//! Integration tests for directory operations

use paykit_demo_core::{Identity, PaymentMethod, SessionManager};
use paykit_lib::{
    AuthenticatedTransport, EndpointData, MethodId, PubkyAuthenticatedTransport,
    PubkyUnauthenticatedTransport, UnauthenticatedTransportRead,
};
use pubky_testnet::EphemeralTestnet;

#[tokio::test]
#[ignore] // Requires external DHT - run manually with --ignored
async fn test_publish_and_query_payment_methods() {
    // Start testnet
    let testnet = EphemeralTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver();
    let sdk = testnet.sdk().expect("Failed to get SDK");

    // Create identity
    let identity = Identity::generate();

    // Create authenticated session
    let session = SessionManager::create_with_sdk(&sdk, &identity, &homeserver.public_key())
        .await
        .expect("Failed to create session");

    // Create transport using testnet SDK (not public network)
    let auth_transport = PubkyAuthenticatedTransport::new(session.session().clone());
    let read_transport = PubkyUnauthenticatedTransport::new(sdk.public_storage());

    // Publish payment methods
    let methods = vec![
        PaymentMethod::new("lightning".to_string(), "lnbc1...test".to_string(), true),
        PaymentMethod::new("onchain".to_string(), "bc1q...test".to_string(), true),
    ];

    for method in &methods {
        auth_transport
            .upsert_payment_endpoint(
                &MethodId(method.method_id.clone()),
                &EndpointData(method.endpoint.clone()),
            )
            .await
            .expect("Failed to publish method");
    }

    // Query the methods back using testnet transport
    let supported = read_transport
        .fetch_supported_payments(&identity.public_key())
        .await
        .expect("Failed to query methods");

    assert_eq!(supported.entries.len(), 2);
    assert!(supported
        .entries
        .contains_key(&MethodId("lightning".to_string())));
    assert!(supported
        .entries
        .contains_key(&MethodId("onchain".to_string())));
}

#[tokio::test]
#[ignore] // Requires external DHT - run manually with --ignored
async fn test_delete_payment_method() {
    // Start testnet
    let testnet = EphemeralTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver();
    let sdk = testnet.sdk().expect("Failed to get SDK");

    let identity = Identity::generate();
    let session = SessionManager::create_with_sdk(&sdk, &identity, &homeserver.public_key())
        .await
        .expect("Failed to create session");

    // Create transports using testnet SDK
    let auth_transport = PubkyAuthenticatedTransport::new(session.session().clone());
    let read_transport = PubkyUnauthenticatedTransport::new(sdk.public_storage());

    // Publish a method
    auth_transport
        .upsert_payment_endpoint(
            &MethodId("lightning".to_string()),
            &EndpointData("lnbc1...test".to_string()),
        )
        .await
        .expect("Failed to publish method");

    // Delete it
    auth_transport
        .remove_payment_endpoint(&MethodId("lightning".to_string()))
        .await
        .expect("Failed to delete method");

    // Verify it's gone
    let supported = read_transport
        .fetch_supported_payments(&identity.public_key())
        .await
        .expect("Failed to query methods");

    assert_eq!(supported.entries.len(), 0);
}
