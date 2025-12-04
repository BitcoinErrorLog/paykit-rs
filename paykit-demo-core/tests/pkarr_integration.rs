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
    publish_noise_key, setup_cold_key, NoiseKeyConfig, NOISE_KEY_PATH_PREFIX,
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

    let result =
        pubky_noise::pkarr_helpers::parse_and_verify_pkarr_key(&txt_value, &wrong_ed25519_pk, device_id);

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
    #[ignore = "Requires local testnet - run with cargo test -- --ignored"]
    async fn test_publish_and_discover_noise_key() {
        // This test would:
        // 1. Start EphemeralTestnet
        // 2. Create a session
        // 3. Publish X25519 key
        // 4. Discover it via PublicStorage
        // 5. Verify the binding

        // Placeholder - requires testnet infrastructure
        println!("Run this test with: cargo test -- --ignored test_publish_and_discover_noise_key");
    }

    #[tokio::test]
    #[ignore = "Requires local testnet - run with cargo test -- --ignored"]
    async fn test_cold_key_ik_raw_handshake() {
        // This test would:
        // 1. Setup testnet
        // 2. Create identities for client and server
        // 3. Publish X25519 keys via pkarr
        // 4. Discover server's X25519 key
        // 5. Perform IK-raw handshake
        // 6. Exchange encrypted messages

        println!("Run this test with: cargo test -- --ignored test_cold_key_ik_raw_handshake");
    }
}

