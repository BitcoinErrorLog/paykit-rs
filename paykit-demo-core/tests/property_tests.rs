//! Property-based tests for paykit-demo-core

use paykit_demo_core::{Identity, IdentityManager};
use proptest::prelude::*;

proptest! {
    /// Test that identity generation always produces valid keypairs
    #[test]
    fn test_identity_generation_produces_valid_keys(nickname in "\\w{1,50}") {
        let identity = Identity::generate().with_nickname(nickname.clone());

        // Public key should be valid
        let uri = identity.pubky_uri();
        prop_assert!(uri.starts_with("pubky://"));
        prop_assert_eq!(identity.nickname, Some(nickname));
    }

    /// Test that X25519 derivation is deterministic
    #[test]
    fn test_x25519_derivation_is_deterministic(
        device_id in prop::collection::vec(any::<u8>(), 1..64),
        epoch in 0u32..100u32
    ) {
        let identity = Identity::generate();

        let key1 = identity.derive_x25519_key(&device_id, epoch).unwrap();
        let key2 = identity.derive_x25519_key(&device_id, epoch).unwrap();

        prop_assert_eq!(key1, key2);
    }

    /// Test that different epochs produce different keys
    #[test]
    fn test_different_epochs_produce_different_keys(
        device_id in prop::collection::vec(any::<u8>(), 1..64)
    ) {
        let identity = Identity::generate();

        let key1 = identity.derive_x25519_key(&device_id, 0).unwrap();
        let key2 = identity.derive_x25519_key(&device_id, 1).unwrap();

        prop_assert_ne!(key1, key2);
    }

    /// Test that identity save/load round-trips correctly
    #[test]
    fn test_identity_save_load_roundtrip(nickname in "\\w{1,50}") {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = IdentityManager::new(temp_dir.path());

        let original = Identity::generate().with_nickname(nickname.clone());
        let original_pk = original.public_key();

        manager.save(&original, "test").unwrap();
        let loaded = manager.load("test").unwrap();

        prop_assert_eq!(loaded.public_key(), original_pk);
        prop_assert_eq!(loaded.nickname, Some(nickname));
    }

    /// Test that receipt IDs are unique
    #[test]
    fn test_receipt_ids_are_unique(_iter in 0..100u32) {
        use paykit_demo_core::models::Receipt;
        use pubky::Keypair;

        let payer = Keypair::random().public_key();
        let payee = Keypair::random().public_key();

        let receipt1 = Receipt::new(
            format!("receipt_{}", uuid::Uuid::new_v4()),
            payer.clone(),
            payee.clone(),
            "lightning".to_string(),
        );

        let receipt2 = Receipt::new(
            format!("receipt_{}", uuid::Uuid::new_v4()),
            payer.clone(),
            payee.clone(),
            "lightning".to_string(),
        );

        prop_assert_ne!(receipt1.id, receipt2.id);
    }
}

#[test]
fn test_x25519_key_length() {
    let identity = Identity::generate();
    let key = identity.derive_x25519_key(b"device", 0).unwrap();
    assert_eq!(key.len(), 32);
}

#[test]
fn test_pubky_uri_format() {
    let identity = Identity::generate();
    let uri = identity.pubky_uri();

    assert!(uri.starts_with("pubky://"));
    assert!(uri.len() > 10); // Should have actual key data
}
