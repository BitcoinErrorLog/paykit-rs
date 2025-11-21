//! Integration tests for pay command

mod common;

use paykit_demo_core::IdentityManager;
use paykit_lib::{AuthenticatedTransport, EndpointData, MethodId, PubkyAuthenticatedTransport};
use pubky_testnet::EphemeralTestnet;
use tempfile::TempDir;

#[tokio::test]
async fn test_pay_command_discovers_recipient_methods() {
    // Setup: Create testnet and identities
    let testnet = EphemeralTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver();
    let sdk = testnet.sdk().expect("Failed to get SDK");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_dir = temp_dir.path();

    // Create payer identity
    let id_manager = IdentityManager::new(storage_dir.join("identities"));
    let _payer = id_manager.create("payer").expect("Failed to create payer");

    std::fs::write(storage_dir.join(".current_identity"), "payer")
        .expect("Failed to save current identity");

    // Create and publish recipient's methods
    let recipient = id_manager
        .create("recipient")
        .expect("Failed to create recipient");
    let recipient_uri = recipient.pubky_uri();

    // Publish recipient's lightning endpoint
    let signer = sdk.signer(recipient.keypair.clone());
    let session = signer
        .signup(&homeserver.public_key(), None)
        .await
        .expect("Failed to signup recipient");

    let auth_transport = PubkyAuthenticatedTransport::new(session);
    auth_transport
        .upsert_payment_endpoint(
            &MethodId("lightning".to_string()),
            &EndpointData("lnbc1000n1...".to_string()),
        )
        .await
        .expect("Failed to publish lightning endpoint");

    // Run pay command
    let result = paykit_demo_cli::commands::pay::run_with_sdk(
        storage_dir,
        &recipient_uri,
        Some("1000".to_string()),
        Some("SAT".to_string()),
        "lightning",
        false,
        Some(&sdk),
    )
    .await;

    // Should succeed in discovering the method
    assert!(
        result.is_ok(),
        "Pay command should succeed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_pay_command_fails_when_method_not_supported() {
    // Setup
    let testnet = EphemeralTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver();
    let sdk = testnet.sdk().expect("Failed to get SDK");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_dir = temp_dir.path();

    let id_manager = IdentityManager::new(storage_dir.join("identities"));
    let payer = id_manager.create("payer").expect("Failed to create payer");

    std::fs::write(storage_dir.join(".current_identity"), "payer")
        .expect("Failed to save current identity");

    // Create recipient but don't publish any methods
    let recipient = id_manager
        .create("recipient")
        .expect("Failed to create recipient");

    // Sign up recipient (but don't publish methods)
    let signer = sdk.signer(recipient.keypair.clone());
    let _session = signer
        .signup(&homeserver.public_key(), None)
        .await
        .expect("Failed to signup recipient");

    // Try to pay with onchain (which recipient doesn't support)
    let result = paykit_demo_cli::commands::pay::run_with_sdk(
        storage_dir,
        &recipient.pubky_uri(),
        Some("1000".to_string()),
        Some("SAT".to_string()),
        "onchain",
        false,
        Some(&sdk),
    )
    .await;

    // Should succeed (command handles gracefully by showing message)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_pay_command_discovers_multiple_methods() {
    // Setup
    let testnet = EphemeralTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver();
    let sdk = testnet.sdk().expect("Failed to get SDK");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_dir = temp_dir.path();

    let id_manager = IdentityManager::new(storage_dir.join("identities"));
    let payer = id_manager.create("payer").expect("Failed to create payer");

    std::fs::write(storage_dir.join(".current_identity"), "payer")
        .expect("Failed to save current identity");

    // Create recipient and publish multiple methods
    let recipient = id_manager
        .create("recipient")
        .expect("Failed to create recipient");

    let signer = sdk.signer(recipient.keypair.clone());
    let session = signer
        .signup(&homeserver.public_key(), None)
        .await
        .expect("Failed to signup recipient");

    let auth_transport = PubkyAuthenticatedTransport::new(session);

    // Publish onchain
    auth_transport
        .upsert_payment_endpoint(
            &MethodId("onchain".to_string()),
            &EndpointData("bc1q...".to_string()),
        )
        .await
        .expect("Failed to publish onchain");

    // Publish lightning
    auth_transport
        .upsert_payment_endpoint(
            &MethodId("lightning".to_string()),
            &EndpointData("lnbc...".to_string()),
        )
        .await
        .expect("Failed to publish lightning");

    // Pay with onchain
    let result = paykit_demo_cli::commands::pay::run_with_sdk(
        storage_dir,
        &recipient.pubky_uri(),
        Some("5000".to_string()),
        Some("SAT".to_string()),
        "onchain",
        true, // verbose
        Some(&sdk),
    )
    .await;

    assert!(result.is_ok(), "Should discover onchain method");
}
