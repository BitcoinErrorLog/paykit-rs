//! Cold key pattern integration tests.
//!
//! Tests the IK-raw, N, and NN patterns that support cold key architectures
//! where Ed25519 keys are kept offline.

use paykit_demo_core::{Identity, NoisePattern, NoiseRawClientHelper, NoiseServerHelper};
use pubky_noise::kdf;

#[test]
fn test_noise_pattern_parsing() {
    // IK variants
    assert_eq!("ik".parse::<NoisePattern>().unwrap(), NoisePattern::IK);
    assert_eq!("IK".parse::<NoisePattern>().unwrap(), NoisePattern::IK);

    // IK-raw variants
    assert_eq!(
        "ik-raw".parse::<NoisePattern>().unwrap(),
        NoisePattern::IKRaw
    );
    assert_eq!(
        "IK-raw".parse::<NoisePattern>().unwrap(),
        NoisePattern::IKRaw
    );
    assert_eq!(
        "ikraw".parse::<NoisePattern>().unwrap(),
        NoisePattern::IKRaw
    );
    assert_eq!(
        "ik_raw".parse::<NoisePattern>().unwrap(),
        NoisePattern::IKRaw
    );

    // N pattern
    assert_eq!("n".parse::<NoisePattern>().unwrap(), NoisePattern::N);
    assert_eq!("N".parse::<NoisePattern>().unwrap(), NoisePattern::N);

    // NN pattern
    assert_eq!("nn".parse::<NoisePattern>().unwrap(), NoisePattern::NN);
    assert_eq!("NN".parse::<NoisePattern>().unwrap(), NoisePattern::NN);

    // Invalid patterns
    assert!("invalid".parse::<NoisePattern>().is_err());
    assert!("xxx".parse::<NoisePattern>().is_err());
}

#[test]
fn test_noise_pattern_display() {
    assert_eq!(format!("{}", NoisePattern::IK), "IK");
    assert_eq!(format!("{}", NoisePattern::IKRaw), "IK-raw");
    assert_eq!(format!("{}", NoisePattern::N), "N");
    assert_eq!(format!("{}", NoisePattern::NN), "NN");
}

#[test]
fn test_derive_x25519_key() {
    let seed = [42u8; 32];
    let device1 = b"device-1";
    let device2 = b"device-2";

    let key1 = NoiseRawClientHelper::derive_x25519_key(&seed, device1);
    let key2 = NoiseRawClientHelper::derive_x25519_key(&seed, device2);
    let key1_again = NoiseRawClientHelper::derive_x25519_key(&seed, device1);

    // Same inputs should produce same outputs (deterministic)
    assert_eq!(*key1, *key1_again);

    // Different device contexts should produce different keys
    assert_ne!(*key1, *key2);

    // Key should not be all zeros
    assert!(key1.iter().any(|&b| b != 0));
    assert!(key2.iter().any(|&b| b != 0));
}

#[test]
fn test_x25519_public_key_derivation() {
    use zeroize::Zeroizing;

    let seed = [42u8; 32];
    let sk = Zeroizing::new(kdf::derive_x25519_static(&seed, b"device"));
    let pk = NoiseRawClientHelper::x25519_public_key(&sk);

    // Public key should be 32 bytes
    assert_eq!(pk.len(), 32);

    // Public key should not be all zeros
    assert!(pk.iter().any(|&b| b != 0));

    // Should be reproducible
    let pk2 = NoiseRawClientHelper::x25519_public_key(&sk);
    assert_eq!(pk, pk2);
}

#[test]
fn test_server_derive_x25519_key() {
    let identity = Identity::generate();
    let device = b"test-device";

    let sk = NoiseServerHelper::derive_x25519_key(&identity, device);

    // Should be 32 bytes
    assert_eq!(sk.len(), 32);

    // Should not be all zeros
    assert!(sk.iter().any(|&b| b != 0));

    // Should be deterministic
    let sk2 = NoiseServerHelper::derive_x25519_key(&identity, device);
    assert_eq!(*sk, *sk2);
}

#[test]
fn test_server_get_static_public_key_consistency() {
    let identity = Identity::generate();
    let device = b"test-device";

    // Get public key via the helper method
    let pk1 = NoiseServerHelper::get_static_public_key(&identity, device);

    // Derive secret key and compute public key manually
    let sk = NoiseServerHelper::derive_x25519_key(&identity, device);
    let pk2 = kdf::x25519_pk_from_sk(&sk);

    // Both methods should produce the same public key
    assert_eq!(pk1, pk2);
}

#[test]
fn test_identity_has_correct_key_format() {
    let identity = Identity::generate();

    // Public key should be accessible
    let pk = identity.public_key();
    assert_eq!(pk.as_bytes().len(), 32);

    // Secret key should be accessible for derivation
    let sk = identity.keypair.secret_key();
    assert_eq!(sk.len(), 32);
}

/// Pattern security properties documentation test.
/// This test documents the expected security properties of each pattern.
#[test]
fn test_pattern_security_properties() {
    // Document expected behavior (not runtime verification)

    // IK: Both parties authenticated via in-handshake Ed25519 signatures
    // - Client sends signed identity binding
    // - Server verifies signature before completing handshake
    // - Full MITM protection
    let _ = NoisePattern::IK;

    // IK-raw: Both parties authenticated via external mechanism (pkarr)
    // - No Ed25519 signing during handshake
    // - Caller must verify identities via pkarr records
    // - MITM protection depends on pkarr verification
    let _ = NoisePattern::IKRaw;

    // N: Server authenticated, client anonymous
    // - Server's static key must be verified via pkarr
    // - Client uses ephemeral key only
    // - Server cannot identify client
    // - Useful for donation boxes, anonymous tips
    let _ = NoisePattern::N;

    // NN: Neither party authenticated
    // - Both parties use ephemeral keys only
    // - MITM vulnerable without post-handshake attestation
    // - Requires explicit caller attestation after handshake
    let _ = NoisePattern::NN;
}

/// Test that all patterns can be serialized and deserialized.
#[test]
fn test_pattern_roundtrip() {
    let patterns = [
        NoisePattern::IK,
        NoisePattern::IKRaw,
        NoisePattern::N,
        NoisePattern::NN,
    ];

    for pattern in patterns {
        let s = format!("{}", pattern);
        let parsed: NoisePattern = s.parse().unwrap();
        assert_eq!(pattern, parsed);
    }
}
