//! Post-handshake attestation for NN pattern connections.
//!
//! NN pattern provides no authentication during the Noise handshake - both parties
//! use only ephemeral keys. This module provides attestation functions to prove
//! identity after the encrypted channel is established.
//!
//! # Security Model
//!
//! Without attestation, NN connections are vulnerable to MITM attacks. Attestation
//! works by having each party sign a message that includes both ephemeral keys
//! from the handshake. This proves:
//! 1. The signer controls the claimed Ed25519 identity
//! 2. The signer participated in this specific handshake (not a replay)
//!
//! # Usage
//!
//! ```rust,ignore
//! use paykit_demo_core::attestation::{create_attestation, verify_attestation};
//!
//! // Server creates attestation after NN handshake
//! let attestation = create_attestation(
//!     &server_ed25519_sk,
//!     &server_ephemeral_pk,
//!     &client_ephemeral_pk,
//! );
//!
//! // Client verifies server's attestation
//! let valid = verify_attestation(
//!     &server_ed25519_pk,  // Expected server identity
//!     &attestation,
//!     &server_ephemeral_pk,
//!     &client_ephemeral_pk,
//! );
//! ```

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

/// Domain separator for attestation messages
const ATTESTATION_DOMAIN: &[u8] = b"pubky-noise-nn-attestation-v1:";

/// Create an attestation message binding an Ed25519 identity to an NN session.
///
/// This should be called after the NN handshake completes. The attestation
/// proves that the holder of `ed25519_sk` participated in this specific
/// handshake.
///
/// # Arguments
/// * `ed25519_sk` - Your Ed25519 secret key (32 bytes)
/// * `local_ephemeral` - Your ephemeral public key from the NN handshake (32 bytes)
/// * `remote_ephemeral` - Peer's ephemeral public key from the NN handshake (32 bytes)
///
/// # Returns
/// A 64-byte Ed25519 signature over the attestation message
pub fn create_attestation(
    ed25519_sk: &[u8; 32],
    local_ephemeral: &[u8; 32],
    remote_ephemeral: &[u8; 32],
) -> [u8; 64] {
    let signing_key = SigningKey::from_bytes(ed25519_sk);
    let message = attestation_message(local_ephemeral, remote_ephemeral);
    signing_key.sign(&message).to_bytes()
}

/// Verify an attestation from a peer.
///
/// Call this to verify that your peer controls the claimed Ed25519 identity
/// and participated in this specific NN handshake.
///
/// # Arguments
/// * `ed25519_pk` - The peer's claimed Ed25519 public key (32 bytes)
/// * `signature` - The peer's attestation signature (64 bytes)
/// * `peer_ephemeral` - Peer's ephemeral public key from the NN handshake (32 bytes)
/// * `our_ephemeral` - Our ephemeral public key from the NN handshake (32 bytes)
///
/// # Returns
/// `true` if the attestation is valid, `false` otherwise
pub fn verify_attestation(
    ed25519_pk: &[u8; 32],
    signature: &[u8; 64],
    peer_ephemeral: &[u8; 32],
    our_ephemeral: &[u8; 32],
) -> bool {
    let Ok(verifying_key) = VerifyingKey::from_bytes(ed25519_pk) else {
        return false;
    };

    // Note: peer signs (their_eph, our_eph), so we verify with that order
    let message = attestation_message(peer_ephemeral, our_ephemeral);
    let sig = Signature::from_bytes(signature);

    verifying_key.verify(&message, &sig).is_ok()
}

/// Generate the attestation message to be signed.
///
/// The message is a SHA-256 hash of:
/// - Domain separator
/// - Local ephemeral public key
/// - Remote ephemeral public key
fn attestation_message(local_ephemeral: &[u8; 32], remote_ephemeral: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(ATTESTATION_DOMAIN);
    hasher.update(local_ephemeral);
    hasher.update(remote_ephemeral);

    let result = hasher.finalize();
    let mut message = [0u8; 32];
    message.copy_from_slice(&result);
    message
}

/// Get the Ed25519 public key from a secret key.
///
/// Convenience function for attestation workflows.
pub fn ed25519_public_key(ed25519_sk: &[u8; 32]) -> [u8; 32] {
    let signing_key = SigningKey::from_bytes(ed25519_sk);
    signing_key.verifying_key().to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attestation_roundtrip() {
        // Generate keypairs
        let server_sk = [1u8; 32];
        let server_pk = ed25519_public_key(&server_sk);

        let client_sk = [2u8; 32];
        let client_pk = ed25519_public_key(&client_sk);

        // Simulate ephemeral keys from NN handshake
        let server_ephemeral = [10u8; 32];
        let client_ephemeral = [20u8; 32];

        // Server creates attestation
        let server_attestation =
            create_attestation(&server_sk, &server_ephemeral, &client_ephemeral);

        // Client verifies server's attestation
        assert!(verify_attestation(
            &server_pk,
            &server_attestation,
            &server_ephemeral,
            &client_ephemeral,
        ));

        // Client creates attestation
        let client_attestation =
            create_attestation(&client_sk, &client_ephemeral, &server_ephemeral);

        // Server verifies client's attestation
        assert!(verify_attestation(
            &client_pk,
            &client_attestation,
            &client_ephemeral,
            &server_ephemeral,
        ));
    }

    #[test]
    fn test_attestation_wrong_key_fails() {
        let server_sk = [1u8; 32];
        let wrong_pk = [99u8; 32]; // Wrong public key

        let server_ephemeral = [10u8; 32];
        let client_ephemeral = [20u8; 32];

        let attestation = create_attestation(&server_sk, &server_ephemeral, &client_ephemeral);

        // Verification with wrong public key should fail
        assert!(!verify_attestation(
            &wrong_pk,
            &attestation,
            &server_ephemeral,
            &client_ephemeral,
        ));
    }

    #[test]
    fn test_attestation_wrong_ephemeral_fails() {
        let server_sk = [1u8; 32];
        let server_pk = ed25519_public_key(&server_sk);

        let server_ephemeral = [10u8; 32];
        let client_ephemeral = [20u8; 32];
        let wrong_ephemeral = [99u8; 32];

        let attestation = create_attestation(&server_sk, &server_ephemeral, &client_ephemeral);

        // Verification with wrong ephemeral should fail (prevents replay)
        assert!(!verify_attestation(
            &server_pk,
            &attestation,
            &wrong_ephemeral,
            &client_ephemeral,
        ));
    }

    #[test]
    fn test_attestation_deterministic() {
        let sk = [42u8; 32];
        let local_eph = [1u8; 32];
        let remote_eph = [2u8; 32];

        let sig1 = create_attestation(&sk, &local_eph, &remote_eph);
        let sig2 = create_attestation(&sk, &local_eph, &remote_eph);

        assert_eq!(sig1, sig2);
    }
}
