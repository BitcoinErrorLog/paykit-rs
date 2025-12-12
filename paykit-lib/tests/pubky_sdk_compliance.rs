//! Pubky SDK compliance tests for paykit-lib
//!
//! These tests verify that paykit-lib correctly integrates with pubky-sdk,
//! testing the transport adapters and directory operations against a real homeserver.
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
//!
//! To re-enable these tests:
//! 1. Update the imports to match the new Pubky SDK API
//! 2. Remove the `#[cfg(feature = "pubky_compliance_tests")]` attribute below
//! 3. Add `pubky_compliance_tests` to Cargo.toml features (optional)
//!
//! Tracking: Requires migration to Pubky SDK 0.6.x+ API

// The entire test module is disabled until Pubky SDK API migration is complete.
// Enable by adding `pubky_compliance_tests` feature to Cargo.toml and running:
// cargo test --features pubky_compliance_tests

#[cfg(feature = "pubky_compliance_tests")]
mod pubky_tests {
    use paykit_lib::{
        AuthenticatedTransport, EndpointData, MethodId, PubkyAuthenticatedTransport,
        PubkyUnauthenticatedTransport, PublicKey, SupportedPayments, UnauthenticatedTransportRead,
    };
    // NOTE: These imports need to be updated for new Pubky SDK API
    use pubky::{PubkyClient, PublicStorage};
    use pubky_testnet::PubkyTestnet;
    use std::time::Duration;
    use tokio::time::timeout;

    /// Helper to create test endpoint data
    fn create_test_endpoint(method: &str, data: &str) -> (MethodId, EndpointData) {
        (MethodId(method.to_string()), EndpointData(data.to_string()))
    }

    #[tokio::test]
    async fn test_pubky_directory_operations() {
        // Test 1: Directory publish/query roundtrip

        // Start local Pubky testnet
        let testnet = PubkyTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver_url = testnet.homeserver().to_string();

        // Create a test identity
        let keypair = pubky::generate_keypair();
        let public_key = keypair.public_key();

        // Create session
        let mut client = PubkyClient::new(&homeserver_url, None)
            .await
            .expect("Failed to create client");

        let session = client
            .signup(&keypair, &homeserver_url)
            .await
            .expect("Failed to signup");

        // Create authenticated transport adapter
        let auth_transport = PubkyAuthenticatedTransport::new(session.clone());

        // Test: Publish multiple payment methods
        let (onchain_method, onchain_data) =
            create_test_endpoint("onchain", "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh");

        let (lightning_method, lightning_data) =
            create_test_endpoint("lightning", "lnbc1000n1pj9rfz3pp5...");

        auth_transport
            .upsert_payment_endpoint(&onchain_method, &onchain_data)
            .await
            .expect("Failed to publish onchain");

        auth_transport
            .upsert_payment_endpoint(&lightning_method, &lightning_data)
            .await
            .expect("Failed to publish lightning");

        // Give homeserver time to propagate
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Test: Query via unauthenticated transport
        let unauth_transport = PubkyUnauthenticatedTransport::new(
            PublicStorage::new(&homeserver_url).expect("Failed to create public storage"),
        );

        let supported_payments = unauth_transport
            .fetch_supported_payments(&public_key)
            .await
            .expect("Failed to fetch supported payments");

        // Verify both methods are present
        assert_eq!(supported_payments.entries.len(), 2);
        assert_eq!(
            supported_payments.entries.get(&onchain_method),
            Some(&onchain_data)
        );
        assert_eq!(
            supported_payments.entries.get(&lightning_method),
            Some(&lightning_data)
        );

        // Test: Fetch individual endpoint
        let onchain_endpoint = unauth_transport
            .fetch_payment_endpoint(&public_key, &onchain_method)
            .await
            .expect("Failed to fetch onchain endpoint");

        assert_eq!(onchain_endpoint, Some(onchain_data.clone()));

        // Test: Query non-existent method returns None
        let nonexistent = unauth_transport
            .fetch_payment_endpoint(&public_key, &MethodId("nonexistent".to_string()))
            .await
            .expect("Failed to query nonexistent");

        assert_eq!(nonexistent, None);

        // Test: Update endpoint
        let new_onchain_data = EndpointData("bc1qnew_address_here".to_string());
        auth_transport
            .upsert_payment_endpoint(&onchain_method, &new_onchain_data)
            .await
            .expect("Failed to update onchain");

        tokio::time::sleep(Duration::from_millis(200)).await;

        let updated_endpoint = unauth_transport
            .fetch_payment_endpoint(&public_key, &onchain_method)
            .await
            .expect("Failed to fetch updated endpoint");

        assert_eq!(updated_endpoint, Some(new_onchain_data));

        // Test: Remove endpoint
        auth_transport
            .remove_payment_endpoint(&lightning_method)
            .await
            .expect("Failed to remove lightning");

        tokio::time::sleep(Duration::from_millis(200)).await;

        let removed_endpoint = unauth_transport
            .fetch_payment_endpoint(&public_key, &lightning_method)
            .await
            .expect("Failed to query after removal");

        assert_eq!(removed_endpoint, None);

        // Verify only onchain remains
        let final_payments = unauth_transport
            .fetch_supported_payments(&public_key)
            .await
            .expect("Failed to fetch final payments");

        assert_eq!(final_payments.entries.len(), 1);
        assert!(final_payments.entries.contains_key(&onchain_method));

        println!("✅ Directory publish/query roundtrip test passed");

        // Cleanup
        testnet.shutdown().await;
    }

    #[tokio::test]
    async fn test_pubky_authenticated_transport() {
        // Test 2: Transport adapter compliance

        // Start testnet
        let testnet = PubkyTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver_url = testnet.homeserver().to_string();

        // Create identity and session
        let keypair = pubky::generate_keypair();
        let public_key = keypair.public_key();

        let mut client = PubkyClient::new(&homeserver_url, None)
            .await
            .expect("Failed to create client");

        let session = client
            .signup(&keypair, &homeserver_url)
            .await
            .expect("Failed to signup");

        // Test: PubkyAuthenticatedTransport correctly wraps PubkySession
        let auth_transport = PubkyAuthenticatedTransport::new(session.clone());

        // Verify we can access the wrapped session
        assert_eq!(auth_transport.session().public_key(), session.public_key());

        // Test: from() trait implementation
        let auth_transport2: PubkyAuthenticatedTransport = session.clone().into();
        assert_eq!(auth_transport2.session().public_key(), session.public_key());

        // Test: Upsert and remove operations
        let (method, data) = create_test_endpoint("test_method", "test_data_12345");

        auth_transport
            .upsert_payment_endpoint(&method, &data)
            .await
            .expect("Failed to upsert");

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify via public storage
        let unauth_transport = PubkyUnauthenticatedTransport::new(
            PublicStorage::new(&homeserver_url).expect("Failed to create public storage"),
        );

        let fetched = unauth_transport
            .fetch_payment_endpoint(&public_key, &method)
            .await
            .expect("Failed to fetch");

        assert_eq!(fetched, Some(data));

        // Test: Remove operation
        auth_transport
            .remove_payment_endpoint(&method)
            .await
            .expect("Failed to remove");

        tokio::time::sleep(Duration::from_millis(200)).await;

        let removed = unauth_transport
            .fetch_payment_endpoint(&public_key, &method)
            .await
            .expect("Failed to fetch after removal");

        assert_eq!(removed, None);

        println!("✅ Authenticated transport compliance test passed");

        // Cleanup
        testnet.shutdown().await;
    }

    #[tokio::test]
    async fn test_endpoint_rotation_logic() {
        // Test 3: Endpoint rotation

        // Start testnet
        let testnet = PubkyTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver_url = testnet.homeserver().to_string();

        // Create identity and session
        let keypair = pubky::generate_keypair();
        let public_key = keypair.public_key();

        let mut client = PubkyClient::new(&homeserver_url, None)
            .await
            .expect("Failed to create client");

        let session = client
            .signup(&keypair, &homeserver_url)
            .await
            .expect("Failed to signup");

        let auth_transport = PubkyAuthenticatedTransport::new(session);
        let unauth_transport = PubkyUnauthenticatedTransport::new(
            PublicStorage::new(&homeserver_url).expect("Failed to create public storage"),
        );

        // Step 1: Publish initial endpoints
        let methods = vec![
            create_test_endpoint("onchain", "address_v1"),
            create_test_endpoint("lightning", "invoice_v1"),
            create_test_endpoint("lnurl", "lnurl_v1"),
        ];

        for (method, data) in &methods {
            auth_transport
                .upsert_payment_endpoint(method, data)
                .await
                .expect("Failed to publish initial");
        }

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify all published
        let initial_payments = unauth_transport
            .fetch_supported_payments(&public_key)
            .await
            .expect("Failed to fetch initial");

        assert_eq!(initial_payments.entries.len(), 3);

        // Step 2: Simulate rotation - delete all old endpoints
        for (method, _) in &methods {
            auth_transport
                .remove_payment_endpoint(method)
                .await
                .expect("Failed to remove old endpoint");
        }

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify all removed
        let after_removal = unauth_transport
            .fetch_supported_payments(&public_key)
            .await
            .expect("Failed to fetch after removal");

        assert_eq!(after_removal.entries.len(), 0);

        // Step 3: Publish new endpoints (rotated)
        let new_methods = vec![
            create_test_endpoint("onchain", "address_v2_rotated"),
            create_test_endpoint("lightning", "invoice_v2_rotated"),
        ];

        for (method, data) in &new_methods {
            auth_transport
                .upsert_payment_endpoint(method, data)
                .await
                .expect("Failed to publish rotated");
        }

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify new endpoints are discoverable
        let rotated_payments = unauth_transport
            .fetch_supported_payments(&public_key)
            .await
            .expect("Failed to fetch rotated");

        assert_eq!(rotated_payments.entries.len(), 2);

        // Verify old data is gone, new data is present
        let onchain_endpoint = unauth_transport
            .fetch_payment_endpoint(&public_key, &MethodId("onchain".to_string()))
            .await
            .expect("Failed to fetch rotated onchain");

        assert_eq!(
            onchain_endpoint,
            Some(EndpointData("address_v2_rotated".to_string()))
        );

        // Verify removed method stays gone
        let lnurl_endpoint = unauth_transport
            .fetch_payment_endpoint(&public_key, &MethodId("lnurl".to_string()))
            .await
            .expect("Failed to fetch removed lnurl");

        assert_eq!(lnurl_endpoint, None);

        println!("✅ Endpoint rotation test passed");

        // Cleanup
        testnet.shutdown().await;
    }

    #[tokio::test]
    async fn test_unauthenticated_transport_404_handling() {
        // Test: Verify 404 handling for non-existent users

        // Start testnet
        let testnet = PubkyTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver_url = testnet.homeserver().to_string();

        let unauth_transport = PubkyUnauthenticatedTransport::new(
            PublicStorage::new(&homeserver_url).expect("Failed to create public storage"),
        );

        // Create a random public key that doesn't exist
        let keypair = pubky::generate_keypair();
        let nonexistent_key = keypair.public_key();

        // Query non-existent user
        let result = unauth_transport
            .fetch_supported_payments(&nonexistent_key)
            .await
            .expect("Failed to query nonexistent user");

        // Should return empty list, not error
        assert_eq!(result.entries.len(), 0);

        // Query specific endpoint for non-existent user
        let endpoint_result = unauth_transport
            .fetch_payment_endpoint(&nonexistent_key, &MethodId("onchain".to_string()))
            .await
            .expect("Failed to query endpoint for nonexistent user");

        assert_eq!(endpoint_result, None);

        println!("✅ 404 handling test passed");

        // Cleanup
        testnet.shutdown().await;
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        // Test: Verify transport adapters are safe for concurrent use

        // Start testnet
        let testnet = PubkyTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver_url = testnet.homeserver().to_string();

        // Create identity and session
        let keypair = pubky::generate_keypair();
        let public_key = keypair.public_key();

        let mut client = PubkyClient::new(&homeserver_url, None)
            .await
            .expect("Failed to create client");

        let session = client
            .signup(&keypair, &homeserver_url)
            .await
            .expect("Failed to signup");

        let auth_transport = PubkyAuthenticatedTransport::new(session);

        // Spawn multiple concurrent upsert operations
        let mut handles = vec![];

        for i in 0..10 {
            let auth_transport_clone = auth_transport.clone();
            let handle = tokio::spawn(async move {
                let method = MethodId(format!("method_{}", i));
                let data = EndpointData(format!("data_{}", i));

                auth_transport_clone
                    .upsert_payment_endpoint(&method, &data)
                    .await
                    .expect("Failed to upsert");
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            timeout(Duration::from_secs(10), handle)
                .await
                .expect("Operation timeout")
                .expect("Task panicked");
        }

        tokio::time::sleep(Duration::from_millis(300)).await;

        // Verify all methods were published
        let unauth_transport = PubkyUnauthenticatedTransport::new(
            PublicStorage::new(&homeserver_url).expect("Failed to create public storage"),
        );

        let payments = unauth_transport
            .fetch_supported_payments(&public_key)
            .await
            .expect("Failed to fetch");

        assert_eq!(payments.entries.len(), 10);

        println!("✅ Concurrent operations test passed");

        // Cleanup
        testnet.shutdown().await;
    }
}
