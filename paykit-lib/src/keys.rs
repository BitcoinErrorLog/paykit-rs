//! Key types and helpers per PUBKY_CRYPTO_SPEC v2.5.
//!
//! This module provides type-safe wrappers for the different key types used in Paykit:
//!
//! - **InboxKey**: X25519 keypair for Sealed Blob stored delivery
//! - **TransportKey**: X25519 keypair for Noise protocol sessions
//! - **KeyBinding**: CBOR-encoded key discovery record published via PKARR
//! - **AppCert**: Delegated application certificate per PUBKY_UNIFIED_KEY_DELEGATION_SPEC v0.2
//!
//! # Key Separation
//!
//! Per PUBKY_CRYPTO_SPEC v2.5 Section 7.2, these key types serve distinct purposes:
//!
//! | Key Type     | Usage                               | Storage Location |
//! |--------------|-------------------------------------|------------------|
//! | InboxKey     | Sealed Blob encryption (stored)     | KeyBinding       |
//! | TransportKey | Noise protocol sessions (ephemeral) | KeyBinding       |
//!
//! # inbox_kid Derivation
//!
//! The `inbox_kid` is a 16-byte identifier derived from the InboxKey:
//!
//! ```text
//! inbox_kid = first_16_bytes(SHA256(inbox_public_key))
//! ```
//!
//! This allows O(1) key selection when a recipient has multiple InboxKeys.
//!
//! # KeyBinding
//!
//! The `KeyBinding` struct is used to publish keys via PKARR for peer discovery.
//! It contains:
//! - `inbox_keys`: List of InboxKey entries with inbox_kid identifiers
//! - `transport_keys`: List of TransportKey entries
//! - `app_keys`: Optional list of delegated AppKey entries
//!
//! # AppCert and Typed Signing
//!
//! Per PUBKY_UNIFIED_KEY_DELEGATION_SPEC v0.2, applications can be issued delegated
//! signing certificates (AppCert) that allow typed content signing without exposing
//! generic "sign anything" APIs.
//!
//! # Crypto Delegation
//!
//! This module delegates cryptographic operations to `pubky-noise` when the `pubky`
//! feature is enabled. This consolidates crypto primitives and avoids version drift.
//!
//! # Example
//!
//! ```rust
//! use paykit_lib::keys::{InboxKey, compute_inbox_kid};
//!
//! // Generate or load an InboxKey
//! let inbox_key = InboxKey::generate();
//!
//! // Compute the inbox_kid for publishing in KeyBinding
//! let kid = compute_inbox_kid(&inbox_key.public_key());
//! assert_eq!(kid.len(), 16);
//! ```

// Re-export KeyBinding types from pubky-noise when the feature is enabled
#[cfg(feature = "pubky")]
pub use pubky_crypto::ukd::{
    AppKeyEntry, InboxKeyEntry, KeyBinding, TransportKeyEntry,
};

// Re-export AppCert types for key delegation per PUBKY_UNIFIED_KEY_DELEGATION_SPEC v0.2
#[cfg(feature = "pubky")]
pub use pubky_crypto::ukd::{
    AppCert, AppCertInput, CERT_ID_LEN as APP_CERT_ID_LEN,
};

// Re-export AppCert functions
#[cfg(feature = "pubky")]
pub use pubky_crypto::ukd::{
    derive_cert_id, generate_app_keypair, issue_app_cert, verify_app_cert,
};

// Re-export typed content signing functions per PUBKY_CRYPTO_SPEC v2.5
#[cfg(feature = "pubky")]
pub use pubky_crypto::ukd::{sign_typed_content, verify_typed_content};

/// Length of an inbox_kid in bytes.
pub const INBOX_KID_LEN: usize = 16;

/// Length of an X25519 public key in bytes.
pub const X25519_PUBLIC_KEY_LEN: usize = 32;

/// Length of an X25519 secret key in bytes.
pub const X25519_SECRET_KEY_LEN: usize = 32;

/// An InboxKey is an X25519 keypair used for Sealed Blob encryption (stored delivery).
///
/// Per PUBKY_CRYPTO_SPEC v2.5:
/// - InboxKey is used ONLY for Sealed Blob encryption
/// - It is NOT used for Noise protocol sessions
/// - The `inbox_kid` is derived from the public key for O(1) selection
#[derive(Clone)]
pub struct InboxKey {
    secret: [u8; X25519_SECRET_KEY_LEN],
    public: [u8; X25519_PUBLIC_KEY_LEN],
}

impl InboxKey {
    /// Generate a new random InboxKey.
    ///
    /// Delegates to `pubky_crypto::x25519_generate_keypair()`.
    ///
    /// Requires the `pubky` feature.
    #[cfg(feature = "pubky")]
    pub fn generate() -> Self {
        let (secret, public) = pubky_crypto::x25519_generate_keypair();
        Self { secret, public }
    }

    /// Create an InboxKey from a secret key.
    ///
    /// The secret key should be 32 random bytes. Clamping is applied automatically.
    ///
    /// Requires the `pubky` feature.
    #[cfg(feature = "pubky")]
    pub fn from_secret(mut secret: [u8; X25519_SECRET_KEY_LEN]) -> Self {
        // Clamp for X25519
        secret[0] &= 248;
        secret[31] &= 127;
        secret[31] |= 64;

        let public = pubky_crypto::x25519_public_from_secret(&secret);
        Self { secret, public }
    }

    /// Create an InboxKey from existing keypair bytes (no derivation).
    ///
    /// This constructor accepts pre-computed secret and public keys.
    /// Use this when loading keys from storage.
    pub fn from_keypair(secret: [u8; X25519_SECRET_KEY_LEN], public: [u8; X25519_PUBLIC_KEY_LEN]) -> Self {
        Self { secret, public }
    }

    /// Get the secret key bytes.
    ///
    /// # Security
    ///
    /// Handle this data with care - it allows decryption of messages.
    pub fn secret_key(&self) -> &[u8; X25519_SECRET_KEY_LEN] {
        &self.secret
    }

    /// Get the public key bytes.
    pub fn public_key(&self) -> &[u8; X25519_PUBLIC_KEY_LEN] {
        &self.public
    }

    /// Compute the inbox_kid for this key.
    ///
    /// ```text
    /// inbox_kid = first_16_bytes(SHA256(public_key))
    /// ```
    ///
    /// Requires the `pubky` feature for derivation.
    #[cfg(feature = "pubky")]
    pub fn inbox_kid(&self) -> [u8; INBOX_KID_LEN] {
        compute_inbox_kid(&self.public)
    }
}

impl std::fmt::Debug for InboxKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(feature = "pubky")]
        {
            f.debug_struct("InboxKey")
                .field("public", &hex::encode(&self.public))
                .field("inbox_kid", &hex::encode(self.inbox_kid()))
                .finish_non_exhaustive()
        }
        #[cfg(not(feature = "pubky"))]
        {
            f.debug_struct("InboxKey")
                .field("public", &hex::encode(&self.public))
                .finish_non_exhaustive()
        }
    }
}

/// A TransportKey is an X25519 keypair used for Noise protocol sessions.
///
/// Per PUBKY_CRYPTO_SPEC v2.5:
/// - TransportKey is used ONLY for Noise protocol sessions
/// - It is NOT used for Sealed Blob encryption
/// - Published in KeyBinding for peer discovery
#[derive(Clone)]
pub struct TransportKey {
    secret: [u8; X25519_SECRET_KEY_LEN],
    public: [u8; X25519_PUBLIC_KEY_LEN],
}

impl TransportKey {
    /// Generate a new random TransportKey.
    ///
    /// Delegates to `pubky_crypto::x25519_generate_keypair()`.
    ///
    /// Requires the `pubky` feature.
    #[cfg(feature = "pubky")]
    pub fn generate() -> Self {
        let (secret, public) = pubky_crypto::x25519_generate_keypair();
        Self { secret, public }
    }

    /// Create a TransportKey from a secret key.
    ///
    /// Requires the `pubky` feature.
    #[cfg(feature = "pubky")]
    pub fn from_secret(mut secret: [u8; X25519_SECRET_KEY_LEN]) -> Self {
        // Clamp for X25519
        secret[0] &= 248;
        secret[31] &= 127;
        secret[31] |= 64;

        let public = pubky_crypto::x25519_public_from_secret(&secret);
        Self { secret, public }
    }

    /// Create a TransportKey from existing keypair bytes (no derivation).
    ///
    /// This constructor accepts pre-computed secret and public keys.
    /// Use this when loading keys from storage.
    pub fn from_keypair(secret: [u8; X25519_SECRET_KEY_LEN], public: [u8; X25519_PUBLIC_KEY_LEN]) -> Self {
        Self { secret, public }
    }

    /// Get the secret key bytes.
    pub fn secret_key(&self) -> &[u8; X25519_SECRET_KEY_LEN] {
        &self.secret
    }

    /// Get the public key bytes.
    pub fn public_key(&self) -> &[u8; X25519_PUBLIC_KEY_LEN] {
        &self.public
    }
}

impl std::fmt::Debug for TransportKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransportKey")
            .field("public", &hex::encode(&self.public))
            .finish_non_exhaustive()
    }
}

/// Compute inbox_kid from an InboxKey public key.
///
/// Per PUBKY_CRYPTO_SPEC v2.5 Section 7.2:
///
/// ```text
/// inbox_kid = first_16_bytes(SHA256(inbox_public_key))
/// ```
///
/// The inbox_kid is used in SB2 headers for O(1) key selection when
/// a recipient has multiple InboxKeys.
///
/// Delegates to `pubky_crypto::Sb2Header::compute_inbox_kid()`.
///
/// Requires the `pubky` feature.
///
/// # Example
///
/// ```rust
/// use paykit_lib::keys::{compute_inbox_kid, InboxKey};
///
/// let inbox_key = InboxKey::generate();
/// let kid = compute_inbox_kid(inbox_key.public_key());
/// assert_eq!(kid.len(), 16);
/// ```
#[cfg(feature = "pubky")]
pub fn compute_inbox_kid(inbox_public_key: &[u8; X25519_PUBLIC_KEY_LEN]) -> [u8; INBOX_KID_LEN] {
    pubky_crypto::Sb2Header::compute_inbox_kid(inbox_public_key)
}

/// Compute inbox_kid and return as hex string.
///
/// Returns a 32-character lowercase hex string.
///
/// Requires the `pubky` feature.
#[cfg(feature = "pubky")]
pub fn compute_inbox_kid_hex(inbox_public_key: &[u8; X25519_PUBLIC_KEY_LEN]) -> String {
    hex::encode(compute_inbox_kid(inbox_public_key))
}

/// Check if an inbox_kid matches a given InboxKey public key.
///
/// Requires the `pubky` feature.
#[cfg(feature = "pubky")]
pub fn verify_inbox_kid(
    inbox_kid: &[u8; INBOX_KID_LEN],
    inbox_public_key: &[u8; X25519_PUBLIC_KEY_LEN],
) -> bool {
    let computed = compute_inbox_kid(inbox_public_key);
    // Constant-time comparison
    computed
        .iter()
        .zip(inbox_kid.iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// Helper to create a KeyBinding from InboxKey and TransportKey.
///
/// # Example
///
/// ```rust,ignore
/// use paykit_lib::keys::{InboxKey, TransportKey, create_key_binding};
///
/// let inbox_key = InboxKey::generate();
/// let transport_key = TransportKey::generate();
///
/// let binding = create_key_binding(&[&inbox_key], &[&transport_key]);
/// let encoded = binding.encode();
/// ```
#[cfg(feature = "pubky")]
pub fn create_key_binding(inbox_keys: &[&InboxKey], transport_keys: &[&TransportKey]) -> KeyBinding {
    let mut binding = KeyBinding::new();

    for inbox_key in inbox_keys {
        binding.add_inbox_key(*inbox_key.public_key());
    }

    for transport_key in transport_keys {
        binding.add_transport_key(*transport_key.public_key());
    }

    binding
}

/// Find an InboxKey in a KeyBinding by its inbox_kid.
///
/// Returns the X25519 public key if found.
#[cfg(feature = "pubky")]
pub fn find_inbox_key_by_kid(
    binding: &KeyBinding,
    inbox_kid: &[u8; INBOX_KID_LEN],
) -> Option<[u8; X25519_PUBLIC_KEY_LEN]> {
    for entry in &binding.inbox_keys {
        if &entry.inbox_kid == inbox_kid {
            return Some(entry.x25519_pub);
        }
    }
    None
}

/// Get the first TransportKey from a KeyBinding.
///
/// For Noise session establishment, typically the first transport key is used.
#[cfg(feature = "pubky")]
pub fn get_primary_transport_key(binding: &KeyBinding) -> Option<[u8; X25519_PUBLIC_KEY_LEN]> {
    binding.transport_keys.first().map(|e| e.x25519_pub)
}

/// Get the first InboxKey from a KeyBinding.
///
/// For Sealed Blob encryption when inbox_kid is not specified.
#[cfg(feature = "pubky")]
pub fn get_primary_inbox_key(binding: &KeyBinding) -> Option<([u8; INBOX_KID_LEN], [u8; X25519_PUBLIC_KEY_LEN])> {
    binding.inbox_keys.first().map(|e| (e.inbox_kid, e.x25519_pub))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inbox_key_generation() {
        let key = InboxKey::generate();
        assert_ne!(key.secret_key(), &[0u8; 32]);
        assert_ne!(key.public_key(), &[0u8; 32]);
    }

    #[test]
    fn inbox_key_from_secret() {
        let mut secret = [0u8; 32];
        secret[0] = 42;
        let key = InboxKey::from_secret(secret);
        assert_ne!(key.public_key(), &[0u8; 32]);
    }

    #[test]
    fn inbox_kid_is_16_bytes() {
        let key = InboxKey::generate();
        let kid = key.inbox_kid();
        assert_eq!(kid.len(), INBOX_KID_LEN);
        assert_eq!(kid.len(), 16);
    }

    #[test]
    fn inbox_kid_is_deterministic() {
        let key = InboxKey::generate();
        let kid1 = key.inbox_kid();
        let kid2 = key.inbox_kid();
        assert_eq!(kid1, kid2);

        let kid3 = compute_inbox_kid(key.public_key());
        assert_eq!(kid1, kid3);
    }

    #[test]
    fn inbox_kid_differs_for_different_keys() {
        let key1 = InboxKey::generate();
        let key2 = InboxKey::generate();
        assert_ne!(key1.inbox_kid(), key2.inbox_kid());
    }

    #[test]
    fn transport_key_generation() {
        let key = TransportKey::generate();
        assert_ne!(key.secret_key(), &[0u8; 32]);
        assert_ne!(key.public_key(), &[0u8; 32]);
    }

    #[test]
    fn verify_inbox_kid_works() {
        let key = InboxKey::generate();
        let kid = key.inbox_kid();
        assert!(verify_inbox_kid(&kid, key.public_key()));

        let wrong_kid = [0u8; INBOX_KID_LEN];
        assert!(!verify_inbox_kid(&wrong_kid, key.public_key()));
    }

    #[test]
    fn inbox_kid_hex_format() {
        let key = InboxKey::generate();
        let kid_hex = compute_inbox_kid_hex(key.public_key());
        assert_eq!(kid_hex.len(), 32); // 16 bytes = 32 hex chars

        // Should be valid hex
        let bytes = hex::decode(&kid_hex).unwrap();
        assert_eq!(bytes.len(), 16);
    }

    #[test]
    fn key_debug_does_not_leak_secrets() {
        let inbox = InboxKey::generate();
        let debug_str = format!("{:?}", inbox);
        // Should show public key, not secret
        assert!(debug_str.contains("public"));
        assert!(!debug_str.contains(&hex::encode(inbox.secret_key())));
    }

    #[cfg(feature = "pubky")]
    #[test]
    fn key_binding_creation() {
        let inbox = InboxKey::generate();
        let transport = TransportKey::generate();

        let binding = create_key_binding(&[&inbox], &[&transport]);

        assert_eq!(binding.inbox_keys.len(), 1);
        assert_eq!(binding.transport_keys.len(), 1);
        assert!(binding.app_keys.is_none());
    }

    #[cfg(feature = "pubky")]
    #[test]
    fn key_binding_find_by_kid() {
        let inbox = InboxKey::generate();
        let transport = TransportKey::generate();

        let binding = create_key_binding(&[&inbox], &[&transport]);

        // Should find by inbox_kid
        let kid = inbox.inbox_kid();
        let found = find_inbox_key_by_kid(&binding, &kid);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), *inbox.public_key());

        // Should not find wrong kid
        let wrong_kid = [0u8; INBOX_KID_LEN];
        assert!(find_inbox_key_by_kid(&binding, &wrong_kid).is_none());
    }

    #[cfg(feature = "pubky")]
    #[test]
    fn key_binding_encode_decode_roundtrip() {
        let inbox = InboxKey::generate();
        let transport = TransportKey::generate();

        let binding = create_key_binding(&[&inbox], &[&transport]);
        let encoded = binding.encode();

        let decoded = KeyBinding::decode(&encoded).unwrap();
        assert_eq!(decoded.inbox_keys.len(), 1);
        assert_eq!(decoded.transport_keys.len(), 1);
        assert_eq!(decoded.inbox_keys[0].x25519_pub, *inbox.public_key());
        assert_eq!(decoded.transport_keys[0].x25519_pub, *transport.public_key());
    }

    #[cfg(feature = "pubky")]
    #[test]
    fn key_binding_primary_accessors() {
        let inbox = InboxKey::generate();
        let transport = TransportKey::generate();

        let binding = create_key_binding(&[&inbox], &[&transport]);

        let primary_transport = get_primary_transport_key(&binding);
        assert!(primary_transport.is_some());
        assert_eq!(primary_transport.unwrap(), *transport.public_key());

        let primary_inbox = get_primary_inbox_key(&binding);
        assert!(primary_inbox.is_some());
        let (kid, pk) = primary_inbox.unwrap();
        assert_eq!(kid, inbox.inbox_kid());
        assert_eq!(pk, *inbox.public_key());
    }

    // ========================================================================
    // AppCert and Typed Signing Tests (PUBKY_UNIFIED_KEY_DELEGATION_SPEC v0.2)
    // ========================================================================

    #[cfg(feature = "pubky")]
    mod app_cert_tests {
        use super::*;

        fn generate_ed25519_keypair() -> ([u8; 32], [u8; 32]) {
            use ed25519_dalek::SigningKey;
            use rand::RngCore;

            let mut seed = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut seed);
            let signing_key = SigningKey::from_bytes(&seed);
            (*signing_key.as_bytes(), *signing_key.verifying_key().as_bytes())
        }

        #[test]
        fn app_cert_issue_and_verify() {
            let (root_sk, root_pk) = generate_ed25519_keypair();
            let (app_sk, app_pk) = generate_app_keypair();
            let transport = TransportKey::generate();
            let inbox = InboxKey::generate();

            let input = AppCertInput {
                issuer_peerid: root_pk,
                app_id: "paykit.test".to_string(),
                device_id: Some(b"device-123".to_vec()),
                app_ed25519_pub: app_pk,
                transport_x25519_pub: *transport.public_key(),
                inbox_x25519_pub: *inbox.public_key(),
                scopes: Some(vec!["payment.sign".to_string()]),
                not_before: None,
                expires_at: Some(u64::MAX),
                flags: None,
            };

            let cert = issue_app_cert(&root_sk, &input).expect("issue_app_cert should succeed");

            // Verify the cert
            let verified_cert_id = verify_app_cert(&root_pk, &cert.cert_body, &cert.sig)
                .expect("verify_app_cert should succeed");

            assert_eq!(verified_cert_id, cert.cert_id);
            assert_eq!(cert.cert_id.len(), APP_CERT_ID_LEN);
        }

        #[test]
        fn app_cert_derive_cert_id_matches() {
            let (root_sk, root_pk) = generate_ed25519_keypair();
            let (_, app_pk) = generate_app_keypair();
            let transport = TransportKey::generate();
            let inbox = InboxKey::generate();

            let input = AppCertInput {
                issuer_peerid: root_pk,
                app_id: "paykit.test".to_string(),
                device_id: None,
                app_ed25519_pub: app_pk,
                transport_x25519_pub: *transport.public_key(),
                inbox_x25519_pub: *inbox.public_key(),
                scopes: None,
                not_before: None,
                expires_at: None,
                flags: None,
            };

            let cert = issue_app_cert(&root_sk, &input).expect("issue_app_cert should succeed");
            let derived = derive_cert_id(&cert.cert_body);
            assert_eq!(derived, cert.cert_id);
        }

        #[test]
        fn typed_content_sign_and_verify() {
            let (root_sk, root_pk) = generate_ed25519_keypair();
            let (app_sk, app_pk) = generate_app_keypair();
            let transport = TransportKey::generate();
            let inbox = InboxKey::generate();

            let input = AppCertInput {
                issuer_peerid: root_pk,
                app_id: "paykit".to_string(),
                device_id: None,
                app_ed25519_pub: app_pk,
                transport_x25519_pub: *transport.public_key(),
                inbox_x25519_pub: *inbox.public_key(),
                scopes: None,
                not_before: None,
                expires_at: None,
                flags: None,
            };

            let cert = issue_app_cert(&root_sk, &input).unwrap();

            // Sign typed content
            let content = b"payment receipt data";
            let sig = sign_typed_content(
                &app_sk,
                &root_pk,
                &cert.cert_id,
                "paykit.receipt",
                content,
            )
            .expect("sign_typed_content should succeed");

            // Verify
            verify_typed_content(&app_pk, &root_pk, &cert.cert_id, "paykit.receipt", content, &sig)
                .expect("verify_typed_content should succeed");
        }

        #[test]
        fn typed_content_wrong_type_fails() {
            let (root_sk, root_pk) = generate_ed25519_keypair();
            let (app_sk, app_pk) = generate_app_keypair();
            let transport = TransportKey::generate();
            let inbox = InboxKey::generate();

            let input = AppCertInput {
                issuer_peerid: root_pk,
                app_id: "paykit".to_string(),
                device_id: None,
                app_ed25519_pub: app_pk,
                transport_x25519_pub: *transport.public_key(),
                inbox_x25519_pub: *inbox.public_key(),
                scopes: None,
                not_before: None,
                expires_at: None,
                flags: None,
            };

            let cert = issue_app_cert(&root_sk, &input).unwrap();

            let content = b"payment data";
            let sig = sign_typed_content(&app_sk, &root_pk, &cert.cert_id, "paykit.receipt", content)
                .unwrap();

            // Verify with wrong content_type should fail
            let result = verify_typed_content(
                &app_pk,
                &root_pk,
                &cert.cert_id,
                "paykit.invoice", // Wrong type!
                content,
                &sig,
            );

            assert!(result.is_err());
        }

        #[test]
        fn typed_content_wrong_payload_fails() {
            let (root_sk, root_pk) = generate_ed25519_keypair();
            let (app_sk, app_pk) = generate_app_keypair();
            let transport = TransportKey::generate();
            let inbox = InboxKey::generate();

            let input = AppCertInput {
                issuer_peerid: root_pk,
                app_id: "paykit".to_string(),
                device_id: None,
                app_ed25519_pub: app_pk,
                transport_x25519_pub: *transport.public_key(),
                inbox_x25519_pub: *inbox.public_key(),
                scopes: None,
                not_before: None,
                expires_at: None,
                flags: None,
            };

            let cert = issue_app_cert(&root_sk, &input).unwrap();

            let content = b"original data";
            let sig = sign_typed_content(&app_sk, &root_pk, &cert.cert_id, "paykit.receipt", content)
                .unwrap();

            // Verify with wrong payload should fail
            let result = verify_typed_content(
                &app_pk,
                &root_pk,
                &cert.cert_id,
                "paykit.receipt",
                b"tampered data", // Wrong payload!
                &sig,
            );

            assert!(result.is_err());
        }

        #[test]
        fn generate_app_keypair_produces_valid_keys() {
            let (sk, pk) = generate_app_keypair();

            // Keys should be 32 bytes
            assert_eq!(sk.len(), 32);
            assert_eq!(pk.len(), 32);

            // Keys should not be all zeros
            assert_ne!(sk, [0u8; 32]);
            assert_ne!(pk, [0u8; 32]);

            // Should be able to derive public from secret
            use ed25519_dalek::SigningKey;
            let signing_key = SigningKey::from_bytes(&sk);
            assert_eq!(*signing_key.verifying_key().as_bytes(), pk);
        }
    }

    // ========================================================================
    // Interop Test Vectors for inbox_kid (INTEROP_TEST_VECTORS.md)
    //
    // These tests verify exact matches with the documented test vectors.
    // If any of these fail, the implementation is incompatible with the spec.
    // ========================================================================

    mod inbox_kid_interop_tests {
        use super::*;

        #[test]
        fn interop_inbox_kid_vector_1_all_zeros() {
            // From INTEROP_TEST_VECTORS.md: all-zeros key
            let inbox_pk = [0u8; 32];
            let kid = compute_inbox_kid(&inbox_pk);
            assert_eq!(
                hex::encode(kid),
                "66687aadf862bd776c8fc18b8e9f8e20",
                "inbox_kid vector 1 (all-zeros) mismatch"
            );
        }

        #[test]
        fn interop_inbox_kid_vector_2_all_ones() {
            // From INTEROP_TEST_VECTORS.md: all-ones key
            let inbox_pk = [0x01u8; 32];
            let kid = compute_inbox_kid(&inbox_pk);
            assert_eq!(
                hex::encode(kid),
                "72cd6e8422c407fb6d098690f1130b7d",
                "inbox_kid vector 2 (all-ones) mismatch"
            );
        }

        #[test]
        fn interop_inbox_kid_vector_3_all_ff() {
            // From INTEROP_TEST_VECTORS.md: all-ff key
            let inbox_pk = [0xffu8; 32];
            let kid = compute_inbox_kid(&inbox_pk);
            assert_eq!(
                hex::encode(kid),
                "af9613760f72635fbdb44a5a0a63c39f",
                "inbox_kid vector 3 (all-ff) mismatch"
            );
        }

        #[test]
        fn inbox_kid_derivation_matches_sha256_first_16() {
            // Verify the algorithm: inbox_kid = first_16_bytes(SHA256(public_key))
            use sha2::{Digest, Sha256};

            let inbox_pk = [0x42u8; 32];
            let kid = compute_inbox_kid(&inbox_pk);

            let hash = Sha256::digest(&inbox_pk);
            let expected: [u8; 16] = hash[..16].try_into().unwrap();

            assert_eq!(kid, expected, "inbox_kid should be first 16 bytes of SHA256");
        }
    }
}
