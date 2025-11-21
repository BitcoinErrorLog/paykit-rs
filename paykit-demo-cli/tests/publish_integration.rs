//! Integration test for publish command
//!
//! Tests the full publish workflow end-to-end

mod common;

use paykit_demo_core::IdentityManager;
use paykit_lib::{PubkyUnauthenticatedTransport, UnauthenticatedTransportRead};
use pubky_testnet::EphemeralTestnet;
use tempfile::TempDir;

#[tokio::test]
async fn test_publish_command_end_to_end() {
    // Setup: Create testnet and temporary storage
    let testnet = EphemeralTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver();
    let sdk = testnet.sdk().expect("Failed to get SDK");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_dir = temp_dir.path();

    // Create an identity
    let identity_manager = IdentityManager::new(storage_dir.join("identities"));
    let identity = identity_manager
        .create("test_publisher")
        .expect("Failed to create identity");

    // Save as current identity
    std::fs::write(storage_dir.join(".current_identity"), "test_publisher")
        .expect("Failed to save current identity");

    // Run publish command with lightning endpoint
    let result = paykit_demo_cli::commands::publish::run_with_sdk(
        storage_dir,
        None,                                        // No onchain
        Some("lnbc1000n1pj9rfz3pp5...".to_string()), // Lightning
        &homeserver.public_key().to_string(),
        false,      // Not verbose
        Some(&sdk), // Use testnet SDK
    )
    .await;

    assert!(
        result.is_ok(),
        "Publish command should succeed: {:?}",
        result.err()
    );

    // Verify: Query the published method
    let unauth_transport = PubkyUnauthenticatedTransport::new(sdk.public_storage());
    let methods = unauth_transport
        .fetch_supported_payments(&identity.public_key())
        .await
        .expect("Failed to query published methods");

    // Check that lightning method was published
    assert_eq!(methods.entries.len(), 1, "Should have one published method");

    let lightning_method = paykit_lib::MethodId("lightning".to_string());
    assert!(
        methods.entries.contains_key(&lightning_method),
        "Should have published lightning method"
    );

    let endpoint = methods.entries.get(&lightning_method).unwrap();
    assert_eq!(endpoint.0, "lnbc1000n1pj9rfz3pp5...");
}

#[tokio::test]
async fn test_publish_multiple_methods() {
    // Setup
    let testnet = EphemeralTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver();
    let sdk = testnet.sdk().expect("Failed to get SDK");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_dir = temp_dir.path();

    let identity_manager = IdentityManager::new(storage_dir.join("identities"));
    let identity = identity_manager
        .create("multi_publisher")
        .expect("Failed to create identity");

    std::fs::write(storage_dir.join(".current_identity"), "multi_publisher")
        .expect("Failed to save current identity");

    // Publish both onchain and lightning
    let result = paykit_demo_cli::commands::publish::run_with_sdk(
        storage_dir,
        Some("bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string()),
        Some("lnbc1000n1...".to_string()),
        &homeserver.public_key().to_string(),
        true,       // Verbose
        Some(&sdk), // Use testnet SDK
    )
    .await;

    assert!(result.is_ok(), "Publish should succeed: {:?}", result.err());

    // Verify both methods
    let unauth_transport = PubkyUnauthenticatedTransport::new(sdk.public_storage());
    let methods = unauth_transport
        .fetch_supported_payments(&identity.public_key())
        .await
        .expect("Failed to query methods");

    assert_eq!(methods.entries.len(), 2, "Should have two methods");
    assert!(methods
        .entries
        .contains_key(&paykit_lib::MethodId("onchain".to_string())));
    assert!(methods
        .entries
        .contains_key(&paykit_lib::MethodId("lightning".to_string())));
}

#[tokio::test]
async fn test_publish_no_methods_specified() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_dir = temp_dir.path();

    let identity_manager = IdentityManager::new(storage_dir.join("identities"));
    identity_manager
        .create("no_methods")
        .expect("Failed to create identity");

    std::fs::write(storage_dir.join(".current_identity"), "no_methods")
        .expect("Failed to save current identity");

    // Try to publish without specifying any methods
    let result =
        paykit_demo_cli::commands::publish::run(storage_dir, None, None, "some_homeserver", false)
            .await;

    // Should succeed but do nothing (early return)
    assert!(result.is_ok(), "Should handle no methods gracefully");
}
