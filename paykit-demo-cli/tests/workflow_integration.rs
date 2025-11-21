//! Simple end-to-end test validating the basic flow

mod common;

use paykit_demo_core::IdentityManager;
use paykit_lib::{
    AuthenticatedTransport, EndpointData, MethodId, PubkyAuthenticatedTransport,
    UnauthenticatedTransportRead,
};
use pubky_testnet::EphemeralTestnet;
use tempfile::TempDir;

#[tokio::test]
async fn test_complete_publish_discover_workflow() {
    // This test validates the full workflow:
    // 1. Receiver creates identity and publishes methods
    // 2. Payer discovers receiver's published methods
    // 3. Validates methods can be queried

    let testnet = EphemeralTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver();
    let sdk = testnet.sdk().expect("Failed to get SDK");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_dir = temp_dir.path();

    let id_manager = IdentityManager::new(storage_dir.join("identities"));

    // Step 1: Receiver publishes payment methods
    let receiver = id_manager
        .create("receiver")
        .expect("Failed to create receiver");

    let signer = sdk.signer(receiver.keypair.clone());
    let session = signer
        .signup(&homeserver.public_key(), None)
        .await
        .expect("Failed to signup receiver");

    let auth_transport = PubkyAuthenticatedTransport::new(session);

    // Publish multiple methods
    auth_transport
        .upsert_payment_endpoint(
            &MethodId("onchain".to_string()),
            &EndpointData("bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string()),
        )
        .await
        .expect("Failed to publish onchain");

    auth_transport
        .upsert_payment_endpoint(
            &MethodId("lightning".to_string()),
            &EndpointData("lnbc1000n1pj9rfz3pp5...".to_string()),
        )
        .await
        .expect("Failed to publish lightning");

    println!("✅ Step 1: Receiver published methods");

    // Step 2: Payer discovers receiver's methods
    let _payer = id_manager.create("payer").expect("Failed to create payer");

    let unauth_transport = paykit_lib::PubkyUnauthenticatedTransport::new(sdk.public_storage());
    let methods = unauth_transport
        .fetch_supported_payments(&receiver.public_key())
        .await
        .expect("Failed to fetch methods");

    println!(
        "✅ Step 2: Payer discovered {} methods",
        methods.entries.len()
    );

    // Step 3: Validate discovered methods
    assert_eq!(methods.entries.len(), 2, "Should have 2 published methods");

    let onchain = methods.entries.get(&MethodId("onchain".to_string()));
    assert!(onchain.is_some(), "Should have onchain method");
    assert_eq!(
        onchain.unwrap().0,
        "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
    );

    let lightning = methods.entries.get(&MethodId("lightning".to_string()));
    assert!(lightning.is_some(), "Should have lightning method");
    assert_eq!(lightning.unwrap().0, "lnbc1000n1pj9rfz3pp5...");

    println!("✅ Step 3: Methods validated successfully");
    println!("✅ Complete workflow test PASSED");
}

#[tokio::test]
async fn test_method_rotation_and_updates() {
    // Test that methods can be updated/rotated
    let testnet = EphemeralTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver();
    let sdk = testnet.sdk().expect("Failed to get SDK");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_dir = temp_dir.path();

    let id_manager = IdentityManager::new(storage_dir.join("identities"));
    let receiver = id_manager
        .create("receiver")
        .expect("Failed to create receiver");

    let signer = sdk.signer(receiver.keypair.clone());
    let session = signer
        .signup(&homeserver.public_key(), None)
        .await
        .expect("Failed to signup");

    let auth_transport = PubkyAuthenticatedTransport::new(session);

    // Publish initial endpoint
    auth_transport
        .upsert_payment_endpoint(
            &MethodId("lightning".to_string()),
            &EndpointData("lnbc_old_invoice".to_string()),
        )
        .await
        .expect("Failed to publish");

    // Update to new endpoint (rotation)
    auth_transport
        .upsert_payment_endpoint(
            &MethodId("lightning".to_string()),
            &EndpointData("lnbc_new_invoice".to_string()),
        )
        .await
        .expect("Failed to update");

    // Verify only new endpoint is returned
    let unauth_transport = paykit_lib::PubkyUnauthenticatedTransport::new(sdk.public_storage());
    let methods = unauth_transport
        .fetch_supported_payments(&receiver.public_key())
        .await
        .expect("Failed to fetch");

    assert_eq!(methods.entries.len(), 1);
    let endpoint = methods
        .entries
        .get(&MethodId("lightning".to_string()))
        .unwrap();
    assert_eq!(
        endpoint.0, "lnbc_new_invoice",
        "Should have updated endpoint"
    );

    println!("✅ Method rotation test PASSED");
}

#[tokio::test]
async fn test_multiple_users_publishing() {
    // Test that multiple users can publish independently
    let testnet = EphemeralTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver();
    let sdk = testnet.sdk().expect("Failed to get SDK");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_dir = temp_dir.path();

    let id_manager = IdentityManager::new(storage_dir.join("identities"));

    // User 1 publishes
    let user1 = id_manager.create("user1").expect("Failed to create user1");
    let signer1 = sdk.signer(user1.keypair.clone());
    let session1 = signer1
        .signup(&homeserver.public_key(), None)
        .await
        .expect("Signup failed");
    let auth1 = PubkyAuthenticatedTransport::new(session1);

    auth1
        .upsert_payment_endpoint(
            &MethodId("onchain".to_string()),
            &EndpointData("bc1q_user1_address".to_string()),
        )
        .await
        .expect("Failed to publish");

    // User 2 publishes
    let user2 = id_manager.create("user2").expect("Failed to create user2");
    let signer2 = sdk.signer(user2.keypair.clone());
    let session2 = signer2
        .signup(&homeserver.public_key(), None)
        .await
        .expect("Signup failed");
    let auth2 = PubkyAuthenticatedTransport::new(session2);

    auth2
        .upsert_payment_endpoint(
            &MethodId("lightning".to_string()),
            &EndpointData("lnbc_user2_invoice".to_string()),
        )
        .await
        .expect("Failed to publish");

    // Verify each user has their own methods
    let unauth = paykit_lib::PubkyUnauthenticatedTransport::new(sdk.public_storage());

    let user1_methods = unauth
        .fetch_supported_payments(&user1.public_key())
        .await
        .expect("Failed");
    assert_eq!(user1_methods.entries.len(), 1);
    assert!(user1_methods
        .entries
        .contains_key(&MethodId("onchain".to_string())));

    let user2_methods = unauth
        .fetch_supported_payments(&user2.public_key())
        .await
        .expect("Failed");
    assert_eq!(user2_methods.entries.len(), 1);
    assert!(user2_methods
        .entries
        .contains_key(&MethodId("lightning".to_string())));

    println!("✅ Multiple users publishing test PASSED");
}
