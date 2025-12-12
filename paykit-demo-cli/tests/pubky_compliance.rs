//! Pubky-SDK compliance tests
//!
//! These tests verify that paykit-demo-cli correctly uses pubky-SDK APIs
//! for session creation, publishing, and discovery.
//!
//! # Status: DISABLED
//!
//! These tests are currently disabled because the Pubky SDK API has changed significantly.
//! The following breaking changes need to be addressed:
//!
//! - `pubky::PubkyClient` no longer exists
//! - `pubky::generate_keypair()` no longer exists
//! - `pubky_testnet::PubkyTestnet` no longer exists
//! - `PublicStorage::new()` signature changed (now takes 0 arguments)
//! - `SupportedPayments.entries` is now a HashMap, not a Vec
//!
//! To re-enable these tests:
//! 1. Update the imports to match the new Pubky SDK API
//! 2. Remove the `#[cfg(feature = "pubky_compliance_tests")]` attribute below
//!
//! Tracking: Requires migration to Pubky SDK 0.6.x+ API

// The entire test module is disabled until Pubky SDK API migration is complete.
// Enable by adding `pubky_compliance_tests` feature to Cargo.toml and running:
// cargo test --features pubky_compliance_tests

#[cfg(feature = "pubky_compliance_tests")]
mod common;

#[cfg(feature = "pubky_compliance_tests")]
mod pubky_tests {
    use paykit_lib::{
        AuthenticatedTransport, EndpointData, MethodId, PubkyAuthenticatedTransport,
        PubkyUnauthenticatedTransport, UnauthenticatedTransportRead,
    };
    // NOTE: These imports need to be updated for new Pubky SDK API
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
        assert!(methods.entries.contains_key(&method_id));
        assert_eq!(methods.entries.get(&method_id), Some(&endpoint));

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
        let public_storage =
            PublicStorage::new(&homeserver).expect("Failed to create public storage");
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
            methods.entries.get(&method_id),
            Some(&endpoint2),
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
        let public_storage =
            PublicStorage::new(&homeserver).expect("Failed to create public storage");
        let unauth_transport = PubkyUnauthenticatedTransport::new(public_storage);

        let methods = unauth_transport
            .fetch_supported_payments(&public_key)
            .await
            .expect("Query must succeed");

        assert_eq!(methods.entries.len(), 3, "Should have all three methods");

        // Verify all methods are present
        for (method_id, endpoint_data) in &methods_to_publish {
            assert!(
                methods.entries.get(method_id) == Some(endpoint_data),
                "Method {} should be present with correct data",
                method_id.0
            );
        }

        testnet.shutdown().await;
    }
}
