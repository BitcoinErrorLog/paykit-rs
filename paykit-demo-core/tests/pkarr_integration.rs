//! Integration tests for pkarr-based Noise key discovery and publication.
//!
//! These tests verify the complete flow:
//! 1. Derive X25519 key from Ed25519 identity
//! 2. Publish X25519 key to pubky storage with Ed25519 binding signature
//! 3. Discover X25519 key from pubky storage
//! 4. Verify the binding signature
//! 5. Use discovered key for Noise connection

use paykit_demo_core::pkarr_discovery::{
    derive_noise_keypair, discover_noise_key, noise_key_path, prepare_cold_key_publication,
    publish_noise_key, NoiseKeyConfig, NOISE_KEY_PATH_PREFIX,
};

#[test]
fn test_derive_noise_keypair_deterministic() {
    let ed25519_sk = [42u8; 32];
    let device_id = "test-device";

    let (sk1, pk1) = derive_noise_keypair(&ed25519_sk, device_id);
    let (sk2, pk2) = derive_noise_keypair(&ed25519_sk, device_id);

    assert_eq!(sk1, sk2);
    assert_eq!(pk1, pk2);
}

#[test]
fn test_derive_noise_keypair_different_devices_produce_different_keys() {
    let ed25519_sk = [42u8; 32];

    let (sk_a, pk_a) = derive_noise_keypair(&ed25519_sk, "device-a");
    let (sk_b, pk_b) = derive_noise_keypair(&ed25519_sk, "device-b");

    assert_ne!(sk_a, sk_b);
    assert_ne!(pk_a, pk_b);
}

#[test]
fn test_prepare_cold_key_publication_format() {
    let ed25519_sk = [1u8; 32];
    let device_id = "mobile";

    let (x25519_sk, x25519_pk, txt_value) = prepare_cold_key_publication(&ed25519_sk, device_id);

    // Keys should be non-zero
    assert_ne!(x25519_sk, [0u8; 32]);
    assert_ne!(x25519_pk, [0u8; 32]);

    // TXT record format should be correct
    assert!(txt_value.starts_with("v=1;k="));
    assert!(txt_value.contains(";sig="));

    // Should be able to parse the X25519 key back
    let parsed = pubky_noise::pkarr_helpers::parse_x25519_from_pkarr(&txt_value).unwrap();
    assert_eq!(parsed, x25519_pk);
}

#[test]
fn test_noise_key_path_format() {
    assert_eq!(noise_key_path("default"), "/pub/noise.app/v0/default");
    assert_eq!(noise_key_path("mobile"), "/pub/noise.app/v0/mobile");
    assert_eq!(
        noise_key_path("bitkit-device-1"),
        "/pub/noise.app/v0/bitkit-device-1"
    );
}

#[test]
fn test_pkarr_binding_roundtrip_without_network() {
    use ed25519_dalek::SigningKey;

    let ed25519_sk = SigningKey::from_bytes(&[7u8; 32]);
    let ed25519_pk = ed25519_sk.verifying_key().to_bytes();
    let x25519_pk = [3u8; 32];
    let signature = pubky_noise::pkarr_helpers::sign_pkarr_key_binding(
        ed25519_sk.as_bytes(),
        &x25519_pk,
        "default",
    );
    let txt_value =
        pubky_noise::pkarr_helpers::format_x25519_for_pkarr(&x25519_pk, Some(&signature));

    let parsed =
        pubky_noise::pkarr_helpers::parse_and_verify_pkarr_key(&txt_value, &ed25519_pk, "default")
            .expect("verify pkarr binding");
    assert_eq!(parsed, x25519_pk);
}

#[test]
fn test_noise_key_config_default() {
    let config = NoiseKeyConfig::default();
    assert_eq!(config.device_id, "default");
    assert!(config.verify_binding);
}

#[test]
fn test_prepare_and_verify_cold_key_binding() {
    use ed25519_dalek::SigningKey;

    // Create a proper Ed25519 keypair
    let mut ed25519_sk_bytes = [0u8; 32];
    ed25519_sk_bytes[0] = 1; // Non-zero seed
    let signing_key = SigningKey::from_bytes(&ed25519_sk_bytes);
    let ed25519_pk = signing_key.verifying_key().to_bytes();

    let device_id = "test-device";

    // Prepare the cold key publication
    let (_, x25519_pk, txt_value) = prepare_cold_key_publication(&ed25519_sk_bytes, device_id);

    // Verify the binding signature
    let verified_key =
        pubky_noise::pkarr_helpers::parse_and_verify_pkarr_key(&txt_value, &ed25519_pk, device_id)
            .expect("Binding verification should succeed");

    assert_eq!(verified_key, x25519_pk);
}

#[test]
fn test_wrong_device_id_fails_verification() {
    use ed25519_dalek::SigningKey;

    let mut ed25519_sk_bytes = [0u8; 32];
    ed25519_sk_bytes[0] = 2;
    let signing_key = SigningKey::from_bytes(&ed25519_sk_bytes);
    let ed25519_pk = signing_key.verifying_key().to_bytes();

    let device_id = "correct-device";

    let (_, _, txt_value) = prepare_cold_key_publication(&ed25519_sk_bytes, device_id);

    // Try to verify with wrong device ID
    let result = pubky_noise::pkarr_helpers::parse_and_verify_pkarr_key(
        &txt_value,
        &ed25519_pk,
        "wrong-device",
    );

    assert!(result.is_err());
}

#[test]
fn test_wrong_ed25519_key_fails_verification() {
    use ed25519_dalek::SigningKey;

    let mut ed25519_sk_bytes = [0u8; 32];
    ed25519_sk_bytes[0] = 3;

    let device_id = "test-device";
    let (_, _, txt_value) = prepare_cold_key_publication(&ed25519_sk_bytes, device_id);

    // Try to verify with different Ed25519 key
    let mut wrong_sk = [0u8; 32];
    wrong_sk[0] = 99;
    let wrong_signing_key = SigningKey::from_bytes(&wrong_sk);
    let wrong_ed25519_pk = wrong_signing_key.verifying_key().to_bytes();

    let result = pubky_noise::pkarr_helpers::parse_and_verify_pkarr_key(
        &txt_value,
        &wrong_ed25519_pk,
        device_id,
    );

    assert!(result.is_err());
}

#[test]
fn test_noise_key_path_prefix_constant() {
    assert_eq!(NOISE_KEY_PATH_PREFIX, "/pub/noise.app/v0/");
}

// ============================================================================
// Tests that require pubky-testnet (async)
// These test the actual storage operations
// ============================================================================

#[cfg(test)]
mod with_testnet {
    use super::*;

    // Note: These tests require pubky-testnet::EphemeralTestnet which
    // starts a local homeserver. They are marked as ignored by default
    // to avoid CI issues with network access.

    #[tokio::test]
    #[ignore] // Requires local testnet - run with cargo test -- --ignored
    async fn test_publish_and_discover_noise_key() {
        use pubky_testnet::EphemeralTestnet;

        // 1. Start testnet
        let testnet = EphemeralTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver = testnet.homeserver();
        let sdk = testnet.sdk().expect("Failed to get SDK");

        // 2. Create identity and session
        let identity = paykit_demo_core::Identity::generate();
        let signer = sdk.signer(identity.keypair.clone());
        let session = signer
            .signup(&homeserver.public_key(), None)
            .await
            .expect("Failed to signup");

        // 3. Derive X25519 key and publish
        let device_id = "test-device";
        let (_x25519_sk, x25519_pk) = derive_noise_keypair(&identity.keypair.secret_key(), device_id);

        publish_noise_key(&session, &identity.keypair.secret_key(), &x25519_pk, device_id)
            .await
            .expect("Failed to publish noise key");

        // 4. Discover via PublicStorage
        let discovered_key = discover_noise_key(
            &sdk.public_storage(),
            &identity.public_key(),
            device_id,
        )
        .await
        .expect("Failed to discover noise key");

        // 5. Verify the discovered key matches
        assert_eq!(discovered_key, x25519_pk);

        // 6. Verify timestamp is present and recent
        let path = format!(
            "pubky://{}{}{}",
            identity.public_key(),
            NOISE_KEY_PATH_PREFIX,
            device_id
        );
        let response = sdk.public_storage().get(&path).await.expect("Failed to fetch");
        let txt_record = response.text().await.expect("Failed to read");
        
        let timestamp = pubky_noise::pkarr_helpers::extract_timestamp_from_pkarr(&txt_record);
        assert!(timestamp.is_some(), "Timestamp should be present");
        
        println!("✅ pkarr publish-discover roundtrip successful");
    }

    #[tokio::test]
    #[ignore] // Requires local testnet - run with cargo test -- --ignored
    async fn test_cold_key_ik_raw_handshake() {
        use paykit_demo_core::{NoiseRawClientHelper, NoiseServerHelper};
        use paykit_interactive::{PaykitNoiseChannel, PaykitNoiseMessage};
        use pubky_testnet::EphemeralTestnet;
        use tokio::net::TcpListener;

        // 1. Setup testnet
        let testnet = EphemeralTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver = testnet.homeserver();
        let sdk = testnet.sdk().expect("Failed to get SDK");

        // 2. Create server identity and session
        let server_identity = paykit_demo_core::Identity::generate();
        let server_signer = sdk.signer(server_identity.keypair.clone());
        let server_session = server_signer
            .signup(&homeserver.public_key(), None)
            .await
            .expect("Failed to signup server");

        // 3. Server publishes X25519 key
        let device_id = "server-device";
        let (server_x25519_sk, server_x25519_pk) =
            derive_noise_keypair(&server_identity.keypair.secret_key(), device_id);

        publish_noise_key(
            &server_session,
            &server_identity.keypair.secret_key(),
            &server_x25519_pk,
            device_id,
        )
        .await
        .expect("Failed to publish server key");

        // 4. Client discovers server's X25519 key via pkarr
        let discovered_server_pk = discover_noise_key(
            &sdk.public_storage(),
            &server_identity.public_key(),
            device_id,
        )
        .await
        .expect("Failed to discover server key");

        assert_eq!(discovered_server_pk, server_x25519_pk);

        // 5. Setup TCP server
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let server_addr = listener.local_addr().expect("addr");

        // 6. Server task: Accept IK-raw connection
        use zeroize::Zeroizing;
        let server_task = tokio::spawn({
            let server_x25519_sk = Zeroizing::new(server_x25519_sk);
            async move {
                let (stream, _) = listener.accept().await.expect("accept");
                let (mut channel, _client_pk) =
                    NoiseServerHelper::accept_ik_raw(&server_x25519_sk, stream)
                        .await
                        .expect("IK-raw handshake");

                // Receive encrypted message
                let msg = channel.recv().await.expect("recv");
                assert!(matches!(msg, PaykitNoiseMessage::Ack));

                // Send response
                channel
                    .send(PaykitNoiseMessage::Ack)
                    .await
                    .expect("send");
            }
        });

        // 7. Client: Derive X25519 and connect with IK-raw
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let client_seed = [99u8; 32];
        let client_sk = NoiseRawClientHelper::derive_x25519_key(&client_seed, b"client");

        let mut channel = NoiseRawClientHelper::connect_ik_raw(
            &client_sk,
            &server_addr.to_string(),
            &discovered_server_pk,
        )
        .await
        .expect("IK-raw connect");

        // 8. Exchange encrypted messages
        channel
            .send(PaykitNoiseMessage::Ack)
            .await
            .expect("send");
        let response = channel.recv().await.expect("recv");
        assert!(matches!(response, PaykitNoiseMessage::Ack));

        server_task.await.expect("server task");

        println!("✅ Cold key IK-raw handshake with pkarr discovery successful");
    }
}
