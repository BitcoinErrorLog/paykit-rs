//! # Cryptographic Signature System
//!
//! ## Security Model
//!
//! This module provides Ed25519 digital signatures over subscription data.
//! It MUST provide:
//! - Deterministic hashing (canonical serialization via postcard)
//! - Replay protection (nonce + timestamp + expiration)
//! - Domain separation (unique context per signature type)
//!
//! ## Breaking Changes from v0.1
//! - Uses postcard instead of JSON for deterministic serialization
//! - Signature now includes nonce, timestamp, and expiration
//! - X25519 signing removed (Ed25519 only)
//! - Domain separation added

use crate::{Result, Subscription, SubscriptionError};
use ed25519_dalek::{Signature as DalekSig, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Domain separation constant for subscription signatures
const SUBSCRIPTION_DOMAIN: &[u8] = b"PAYKIT_SUBSCRIPTION_V2";

/// Signature with replay protection
///
/// # Security
///
/// - Includes nonce for uniqueness (must never be reused)
/// - Includes timestamp and expiration for temporal validity
/// - Verification checks expiration before cryptographic validation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Signature {
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>, // Changed to Vec for serde compatibility
    pub public_key: [u8; 32],
    pub nonce: [u8; 32], // Unique per signature - prevents replay
    pub timestamp: i64,  // When signed
    pub expires_at: i64, // Expiration time
}

impl Signature {
    /// Create a new Ed25519 signature
    pub fn new_ed25519(
        signature: [u8; 64],
        public_key: [u8; 32],
        nonce: [u8; 32],
        timestamp: i64,
        expires_at: i64,
    ) -> Self {
        Self {
            signature: signature.to_vec(),
            public_key,
            nonce,
            timestamp,
            expires_at,
        }
    }

    /// Get signature as fixed-size array
    pub fn signature_bytes(&self) -> Option<[u8; 64]> {
        if self.signature.len() == 64 {
            let mut arr = [0u8; 64];
            arr.copy_from_slice(&self.signature);
            Some(arr)
        } else {
            None
        }
    }
}

/// Data structure for signing (includes replay protection)
#[derive(Serialize)]
struct SignaturePayload<'a> {
    domain: &'static [u8],
    subscription: &'a Subscription,
    nonce: &'a [u8; 32],
    timestamp: i64,
    expires_at: i64,
}

/// Hash subscription data for signing (DETERMINISTIC)
///
/// # Security
///
/// This function MUST produce identical output for identical input.
/// Uses postcard with deterministic serialization (pubky-sdk standard).
///
/// # Arguments
///
/// * `subscription` - The subscription to hash
/// * `nonce` - Unique 32-byte nonce
/// * `timestamp` - When the signature was created
/// * `expires_at` - When the signature expires
///
/// # Panics
///
/// Never panics - all errors returned as Result.
fn hash_subscription_canonical(
    subscription: &Subscription,
    nonce: &[u8; 32],
    timestamp: i64,
    expires_at: i64,
) -> Result<[u8; 32]> {
    let payload = SignaturePayload {
        domain: SUBSCRIPTION_DOMAIN,
        subscription,
        nonce,
        timestamp,
        expires_at,
    };

    // postcard is deterministic (fixed-size fields, defined order)
    let canonical_bytes = postcard::to_allocvec(&payload)
        .map_err(|e| SubscriptionError::Serialization(format!("Serialization error: {}", e)))?;

    let hash = Sha256::digest(&canonical_bytes);
    let mut result = [0u8; 32];
    result.copy_from_slice(&hash);
    Ok(result)
}

/// Sign subscription with Ed25519 keypair
///
/// # Security
///
/// - Uses deterministic canonical serialization (postcard)
/// - Includes nonce for replay protection (MUST be unique)
/// - Includes timestamp and expiration
/// - Domain-separated (PAYKIT_SUBSCRIPTION_V2)
///
/// # Arguments
///
/// * `subscription` - The subscription to sign
/// * `keypair` - Ed25519 signing keypair
/// * `nonce` - Unique 32-byte nonce (MUST be random, NEVER reused)
/// * `lifetime_seconds` - How long signature is valid
///
/// # Examples
///
/// ```rust,no_run
/// use rand::RngCore;
/// # use paykit_subscriptions::signing::sign_subscription_ed25519;
/// # let subscription = todo!();
/// # let keypair = todo!();
/// let mut nonce = [0u8; 32];
/// rand::thread_rng().fill_bytes(&mut nonce);
/// let signature = sign_subscription_ed25519(
///     &subscription,
///     &keypair,
///     &nonce,
///     3600  // Valid for 1 hour
/// )?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn sign_subscription_ed25519(
    subscription: &Subscription,
    keypair: &pubky::Keypair,
    nonce: &[u8; 32],
    lifetime_seconds: i64,
) -> Result<Signature> {
    // Get current time
    let timestamp = chrono::Utc::now().timestamp();
    let expires_at = timestamp + lifetime_seconds;

    // Compute canonical hash
    let message = hash_subscription_canonical(subscription, nonce, timestamp, expires_at)?;

    // Sign with Ed25519
    let secret_bytes = keypair.secret_key();
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let signature_bytes = signing_key.sign(&message);

    Ok(Signature::new_ed25519(
        signature_bytes.to_bytes(),
        keypair.public_key().to_bytes(),
        *nonce,
        timestamp,
        expires_at,
    ))
}

/// Verify Ed25519 signature
///
/// # Security
///
/// - Checks expiration FIRST (fail fast)
/// - Checks signature validity
/// - Uses constant-time comparison where applicable
/// - Caller MUST track nonces to prevent replay attacks
///
/// # Returns
///
/// `Ok(true)` if signature is valid and not expired  
/// `Ok(false)` if signature is invalid or expired  
/// `Err(_)` if data is malformed
///
/// # Nonce Tracking
///
/// WARNING: Caller MUST maintain a nonce database and reject
/// any signature with a previously-seen nonce to prevent replay attacks.
pub fn verify_signature_ed25519(
    subscription: &Subscription,
    signature: &Signature,
) -> Result<bool> {
    // Check expiration FIRST (fail fast)
    let now = chrono::Utc::now().timestamp();
    if now > signature.expires_at {
        return Ok(false);
    }

    // Reconstruct canonical hash
    let message = hash_subscription_canonical(
        subscription,
        &signature.nonce,
        signature.timestamp,
        signature.expires_at,
    )?;

    // Parse and verify public key
    let verifying_key = VerifyingKey::from_bytes(&signature.public_key)
        .map_err(|e| SubscriptionError::Crypto(format!("Invalid public key: {}", e)))?;

    // Parse signature - need fixed size array
    let sig_bytes = signature
        .signature_bytes()
        .ok_or_else(|| SubscriptionError::Crypto("Invalid signature length".to_string()))?;
    let sig = DalekSig::from_bytes(&sig_bytes);

    // Verify (ed25519-dalek uses constant-time internally)
    Ok(verifying_key.verify(&message, &sig).is_ok())
}

/// Generic signing function (Ed25519 only in v0.2)
///
/// # Security
///
/// X25519 signing has been removed in v0.2 for security reasons.
/// Use Ed25519 keys for all signatures.
pub fn sign_subscription(
    subscription: &Subscription,
    keypair: &pubky::Keypair,
    nonce: &[u8; 32],
    lifetime_seconds: i64,
) -> Result<Signature> {
    sign_subscription_ed25519(subscription, keypair, nonce, lifetime_seconds)
}

/// Generic verification function (Ed25519 only in v0.2)
pub fn verify_signature(subscription: &Subscription, signature: &Signature) -> Result<bool> {
    verify_signature_ed25519(subscription, signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use paykit_lib::{MethodId, PublicKey};
    use rand::RngCore;
    use std::str::FromStr;

    fn create_test_subscription() -> Subscription {
        let keypair1 = pkarr::Keypair::random();
        let keypair2 = pkarr::Keypair::random();

        Subscription {
            subscription_id: "sub_test_123".to_string(),
            subscriber: PublicKey::from_str(&keypair1.public_key().to_z32()).unwrap(),
            provider: PublicKey::from_str(&keypair2.public_key().to_z32()).unwrap(),
            terms: crate::subscription::SubscriptionTerms {
                amount: crate::Amount::from_sats(1000),
                currency: "SAT".to_string(),
                frequency: crate::subscription::PaymentFrequency::Monthly { day_of_month: 1 },
                method: MethodId("lightning".to_string()),
                max_amount_per_period: Some(crate::Amount::from_sats(5000)),
                description: "Test subscription".to_string(),
            },
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now().timestamp(),
            starts_at: chrono::Utc::now().timestamp(),
            ends_at: None,
        }
    }

    #[test]
    fn test_sign_and_verify_ed25519() {
        let subscription = create_test_subscription();
        let keypair = pkarr::Keypair::random();
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce);

        // Sign
        let signature = sign_subscription_ed25519(&subscription, &keypair, &nonce, 3600).unwrap();

        // Verify structure
        assert_eq!(signature.signature.len(), 64);
        assert_eq!(signature.public_key.len(), 32);
        assert_eq!(signature.nonce, nonce);
        assert!(signature.expires_at > signature.timestamp);

        // Verify signature
        let valid = verify_signature_ed25519(&subscription, &signature).unwrap();
        assert!(valid, "Signature should be valid");
    }

    #[test]
    fn test_signature_hash_is_deterministic() {
        let sub1 = create_test_subscription();
        let sub2 = sub1.clone();
        let nonce = [42u8; 32];
        let timestamp = 1000i64;
        let expires_at = 2000i64;

        let hash1 = hash_subscription_canonical(&sub1, &nonce, timestamp, expires_at).unwrap();
        let hash2 = hash_subscription_canonical(&sub2, &nonce, timestamp, expires_at).unwrap();

        assert_eq!(hash1, hash2, "Hash must be deterministic");
    }

    #[test]
    fn test_expired_signature_rejected() {
        let subscription = create_test_subscription();
        let keypair = pkarr::Keypair::random();
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce);

        // Create signature that expires in 1 second
        let signature = sign_subscription_ed25519(&subscription, &keypair, &nonce, 1).unwrap();

        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Should be rejected
        let valid = verify_signature_ed25519(&subscription, &signature).unwrap();
        assert!(!valid, "Expired signature should be rejected");
    }

    #[test]
    fn test_invalid_signature_fails() {
        let subscription = create_test_subscription();
        let keypair = pkarr::Keypair::random();
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce);

        let mut signature =
            sign_subscription_ed25519(&subscription, &keypair, &nonce, 3600).unwrap();

        // Corrupt the signature
        signature.signature[0] ^= 1;

        let valid = verify_signature_ed25519(&subscription, &signature).unwrap();
        assert!(!valid, "Corrupted signature should be invalid");
    }

    #[test]
    fn test_modified_subscription_fails_verification() {
        let mut subscription = create_test_subscription();
        let keypair = pkarr::Keypair::random();
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce);

        let signature = sign_subscription_ed25519(&subscription, &keypair, &nonce, 3600).unwrap();

        // Modify subscription
        subscription.terms.amount = crate::Amount::from_sats(999999);

        let valid = verify_signature_ed25519(&subscription, &signature).unwrap();
        assert!(
            !valid,
            "Signature should not be valid for modified subscription"
        );
    }

    #[test]
    fn test_generic_sign_and_verify() {
        let subscription = create_test_subscription();
        let keypair = pkarr::Keypair::random();
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce);

        let signature = sign_subscription(&subscription, &keypair, &nonce, 3600).unwrap();
        let valid = verify_signature(&subscription, &signature).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_different_nonces_produce_different_signatures() {
        let subscription = create_test_subscription();
        let keypair = pkarr::Keypair::random();

        let mut nonce1 = [0u8; 32];
        let mut nonce2 = [1u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce1);
        rand::thread_rng().fill_bytes(&mut nonce2);

        let sig1 = sign_subscription_ed25519(&subscription, &keypair, &nonce1, 3600).unwrap();
        let sig2 = sign_subscription_ed25519(&subscription, &keypair, &nonce2, 3600).unwrap();

        assert_ne!(
            sig1.signature, sig2.signature,
            "Different nonces should produce different signatures"
        );
    }

    // ========================================================================
    // RFC 8032 Ed25519 Test Vectors
    // ========================================================================
    // These tests verify our Ed25519 implementation against known test vectors
    // from RFC 8032 Section 7.1.

    /// RFC 8032 Test Vector 1: Empty message
    #[test]
    fn test_ed25519_rfc8032_vector1_empty_message() {
        // Test vector from RFC 8032 Section 7.1
        let secret_key_hex = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
        let public_key_hex = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        let message = b"";
        let expected_sig_hex = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";

        // Decode test vectors
        let secret_key_bytes = hex::decode(secret_key_hex).unwrap();
        let public_key_bytes = hex::decode(public_key_hex).unwrap();
        let expected_sig_bytes = hex::decode(expected_sig_hex).unwrap();

        // Create signing key from secret
        let mut secret_arr = [0u8; 32];
        secret_arr.copy_from_slice(&secret_key_bytes);
        let signing_key = SigningKey::from_bytes(&secret_arr);

        // Verify public key derivation
        let derived_public = signing_key.verifying_key();
        assert_eq!(
            derived_public.as_bytes(),
            &public_key_bytes[..],
            "Public key derivation must match RFC 8032"
        );

        // Sign and verify signature matches
        let signature = signing_key.sign(message);
        assert_eq!(
            signature.to_bytes().as_slice(),
            &expected_sig_bytes[..],
            "Signature must match RFC 8032 test vector"
        );

        // Verify signature
        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes.try_into().unwrap()).unwrap();
        assert!(
            verifying_key.verify(message, &signature).is_ok(),
            "Verification must succeed"
        );
    }

    /// RFC 8032 Test Vector 2: Single byte message (0x72)
    #[test]
    fn test_ed25519_rfc8032_vector2_single_byte() {
        let secret_key_hex = "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb";
        let public_key_hex = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
        let message = &[0x72u8]; // Single byte: 'r' in ASCII
        let expected_sig_hex = "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00";

        let secret_key_bytes = hex::decode(secret_key_hex).unwrap();
        let public_key_bytes = hex::decode(public_key_hex).unwrap();
        let expected_sig_bytes = hex::decode(expected_sig_hex).unwrap();

        let mut secret_arr = [0u8; 32];
        secret_arr.copy_from_slice(&secret_key_bytes);
        let signing_key = SigningKey::from_bytes(&secret_arr);

        let signature = signing_key.sign(message);
        assert_eq!(
            signature.to_bytes().as_slice(),
            &expected_sig_bytes[..],
            "Signature must match RFC 8032 test vector 2"
        );

        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes.try_into().unwrap()).unwrap();
        assert!(verifying_key.verify(message, &signature).is_ok());
    }

    /// RFC 8032 Test Vector 3: Two byte message
    #[test]
    fn test_ed25519_rfc8032_vector3_two_bytes() {
        let secret_key_hex = "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7";
        let public_key_hex = "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025";
        let message = &[0xafu8, 0x82u8];
        let expected_sig_hex = "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a";

        let secret_key_bytes = hex::decode(secret_key_hex).unwrap();
        let public_key_bytes = hex::decode(public_key_hex).unwrap();
        let expected_sig_bytes = hex::decode(expected_sig_hex).unwrap();

        let mut secret_arr = [0u8; 32];
        secret_arr.copy_from_slice(&secret_key_bytes);
        let signing_key = SigningKey::from_bytes(&secret_arr);

        let signature = signing_key.sign(message);
        assert_eq!(
            signature.to_bytes().as_slice(),
            &expected_sig_bytes[..],
            "Signature must match RFC 8032 test vector 3"
        );

        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes.try_into().unwrap()).unwrap();
        assert!(verifying_key.verify(message, &signature).is_ok());
    }
}
