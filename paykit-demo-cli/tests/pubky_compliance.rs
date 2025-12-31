//! Pubky-SDK compliance tests
//!
//! These tests verify that paykit-demo-cli correctly uses pubky-SDK APIs
//! for session creation, publishing, and discovery.
//!
//! Pubky-SDK compliance tests
//!
//! These tests verify that paykit-demo-cli correctly uses pubky-SDK 0.6.0-rc.6+ APIs
//! for session creation, publishing, and discovery.
//!
//! # Running these tests
//!
//! These tests require network access and are feature-gated:
//! ```bash
//! cargo test --features pubky_compliance_tests --test pubky_compliance
//! ```

#[cfg(feature = "pubky_compliance_tests")]
mod common;

#[cfg(feature = "pubky_compliance_tests")]
mod pubky_tests {
    use paykit_lib::{
        AuthenticatedTransport, EndpointData, MethodId, PubkyAuthenticatedTransport,
        PubkyUnauthenticatedTransport, UnauthenticatedTransportRead,
    };
    use pubky::PublicStorage;
    use pubky_testnet::{pubky::Keypair, EphemeralTestnet};

    #[tokio::test]
    async fn test_publish_and_discover_compliance() {
        // Start testnet homeserver
        let testnet = EphemeralTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver = testnet.homeserver_app();
        let sdk = testnet.sdk().expect("Failed to get SDK");

        // 1. Create session
        let keypair = Keypair::random();
        let public_key = keypair.public_key();

        let signer = sdk.signer(keypair.clone());
        let session = signer
            .signup(&homeserver.public_key(), None)
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
        let unauth_transport = PubkyUnauthenticatedTransport::new(sdk.public_storage());

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
    }

    #[tokio::test]
    async fn test_endpoint_rotation_compliance() {
        // Test that multiple publishes to same method_id replace old values
        // per pubky-sdk spec
        let testnet = EphemeralTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver = testnet.homeserver_app();
        let sdk = testnet.sdk().expect("Failed to get SDK");

        let keypair = Keypair::random();
        let public_key = keypair.public_key();

        let signer = sdk.signer(keypair.clone());
        let session = signer
            .signup(&homeserver.public_key(), None)
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
        let unauth_transport = PubkyUnauthenticatedTransport::new(sdk.public_storage());

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
    }

    #[tokio::test]
    async fn test_multiple_methods_compliance() {
        // Test publishing multiple payment methods
        let testnet = EphemeralTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver = testnet.homeserver_app();
        let sdk = testnet.sdk().expect("Failed to get SDK");

        let keypair = Keypair::random();
        let public_key = keypair.public_key();

        let signer = sdk.signer(keypair.clone());
        let session = signer
            .signup(&homeserver.public_key(), None)
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
        let unauth_transport = PubkyUnauthenticatedTransport::new(sdk.public_storage());

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
    }
}
