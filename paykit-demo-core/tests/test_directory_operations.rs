//! Integration tests for directory operations

use paykit_demo_core::{DirectoryClient, Identity, PaymentMethod, SessionManager};
use paykit_lib::AuthenticatedTransport;
use pubky_testnet::EphemeralTestnet;

#[tokio::test]
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

    // Create directory client
    let client = DirectoryClient::new("https://demo.httprelay.io");

    // Publish payment methods
    let methods = vec![
        PaymentMethod::new("lightning".to_string(), "lnbc1...test".to_string(), true),
        PaymentMethod::new("onchain".to_string(), "bc1q...test".to_string(), true),
    ];

    client
        .publish_methods(session.session(), &methods)
        .await
        .expect("Failed to publish methods");

    // Query the methods back
    let queried = client
        .query_methods(&identity.public_key())
        .await
        .expect("Failed to query methods");

    assert_eq!(queried.len(), 2);
    assert!(queried.iter().any(|m| m.method_id == "lightning"));
    assert!(queried.iter().any(|m| m.method_id == "onchain"));
}

#[tokio::test]
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

    let client = DirectoryClient::new("https://demo.httprelay.io");

    // Publish a method
    let methods = vec![PaymentMethod::new(
        "lightning".to_string(),
        "lnbc1...test".to_string(),
        true,
    )];

    client
        .publish_methods(session.session(), &methods)
        .await
        .expect("Failed to publish methods");

    // Delete it
    client
        .delete_method(session.session(), "lightning")
        .await
        .expect("Failed to delete method");

    // Verify it's gone
    let queried = client
        .query_methods(&identity.public_key())
        .await
        .expect("Failed to query methods");

    assert_eq!(queried.len(), 0);
}
