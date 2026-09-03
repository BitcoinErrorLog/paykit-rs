//! AAD (Additional Authenticated Data) builders for Sealed Blob v2.
//!
//! AAD binds the ciphertext to its storage context and owner, preventing relocation attacks.
//! All Paykit clients must use identical AAD formats.
//!
//! # AAD Formats
//!
//! ## Binary AAD (PUBKY_CRYPTO_SPEC v2.5 Section 7.5)
//!
//! ```text
//! aad = aad_prefix || owner_peerid_bytes || canonical_path_bytes || header_bytes
//! ```
//!
//! Where:
//! - `aad_prefix` = `"pubky-envelope/v2:"` (18 bytes)
//! - `owner_peerid_bytes` = 32-byte Ed25519 public key of storage owner
//! - `canonical_path_bytes` = UTF-8 bytes of canonical storage path
//! - `header_bytes` = deterministic CBOR-encoded header (without signature)
//!
//! ## Legacy String AAD (for migration)
//!
//! `paykit:v0:{purpose}:{owner_z32}:{path}:{id}`
//!
//! Where:
//! - `purpose` is the object type (e.g., "request", "subscription_proposal", "handoff", "ack_request")
//! - `owner_z32` is the normalized z-base-32 pubkey of the storage owner
//! - `path` is the full storage path
//! - `id` is the object identifier
//!
//! # Migration Strategy
//!
//! During the migration period:
//! 1. **Encrypt**: Use binary AAD for all new SB2 encrypted objects
//! 2. **Decrypt**: Support both binary AAD (SB2) and legacy string AAD (JSON envelopes)
//! 3. **Detect format**: Check for SB2 magic bytes to determine which AAD format to use

use super::paths::{
    ack_path, payment_request_path, secure_handoff_path, subscription_proposal_path,
};
use crate::errors::PaykitError;
use crate::Result;

/// AAD prefix for all Paykit v0 sealed blobs.
pub const AAD_PREFIX: &str = "paykit:v0";

/// Purpose label for payment requests.
pub const PURPOSE_REQUEST: &str = "request";

/// Purpose label for subscription proposals.
pub const PURPOSE_SUBSCRIPTION_PROPOSAL: &str = "subscription_proposal";

/// Purpose label for secure handoff payloads.
pub const PURPOSE_HANDOFF: &str = "handoff";

/// Purpose label for ACK (acknowledgment) messages.
pub const PURPOSE_ACK: &str = "ack";

/// Build AAD for a payment request.
///
/// Format: `paykit:v0:request:{owner_z32}:{path}:{request_id}`
///
/// # Arguments
///
/// * `owner_pubkey_z32` - The storage owner's z-base-32 encoded pubkey (sender)
/// * `sender_pubkey_z32` - The sender's z-base-32 encoded pubkey
/// * `recipient_pubkey_z32` - The recipient's z-base-32 encoded pubkey
/// * `request_id` - Unique identifier for this request
///
/// # Returns
///
/// The AAD string to use with Sealed Blob v2 encryption.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::payment_request_aad;
///
/// let aad = payment_request_aad(
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "req-123"
/// ).unwrap();
/// assert!(aad.starts_with("paykit:v0:request:"));
/// ```
pub fn payment_request_aad(
    owner_pubkey_z32: &str,
    sender_pubkey_z32: &str,
    recipient_pubkey_z32: &str,
    request_id: &str,
) -> Result<String> {
    let path = payment_request_path(sender_pubkey_z32, recipient_pubkey_z32, request_id)?;
    Ok(format!(
        "{}:{}:{}:{}:{}",
        AAD_PREFIX, PURPOSE_REQUEST, owner_pubkey_z32, path, request_id
    ))
}

/// Build AAD for a subscription proposal.
///
/// Format: `paykit:v0:subscription_proposal:{owner_z32}:{path}:{proposal_id}`
///
/// # Arguments
///
/// * `owner_pubkey_z32` - The storage owner's z-base-32 encoded pubkey (provider)
/// * `provider_pubkey_z32` - The provider's z-base-32 encoded pubkey
/// * `subscriber_pubkey_z32` - The subscriber's z-base-32 encoded pubkey
/// * `proposal_id` - Unique identifier for this proposal
///
/// # Returns
///
/// The AAD string to use with Sealed Blob v2 encryption.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::subscription_proposal_aad;
///
/// let aad = subscription_proposal_aad(
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "prop-456"
/// ).unwrap();
/// assert!(aad.starts_with("paykit:v0:subscription_proposal:"));
/// ```
pub fn subscription_proposal_aad(
    owner_pubkey_z32: &str,
    provider_pubkey_z32: &str,
    subscriber_pubkey_z32: &str,
    proposal_id: &str,
) -> Result<String> {
    let path = subscription_proposal_path(provider_pubkey_z32, subscriber_pubkey_z32, proposal_id)?;
    Ok(format!(
        "{}:{}:{}:{}:{}",
        AAD_PREFIX, PURPOSE_SUBSCRIPTION_PROPOSAL, owner_pubkey_z32, path, proposal_id
    ))
}

/// Build AAD for a secure handoff payload.
///
/// Format: `paykit:v0:handoff:{owner_z32}:{path}:{request_id}`
///
/// # Arguments
///
/// * `owner_pubkey_z32` - The Ring user's z-base-32 encoded pubkey
/// * `request_id` - Unique identifier for this handoff
///
/// # Returns
///
/// The AAD string to use with Sealed Blob v2 encryption.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::secure_handoff_aad;
///
/// let aad = secure_handoff_aad(
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "handoff-789"
/// );
/// assert!(aad.starts_with("paykit:v0:handoff:"));
/// ```
pub fn secure_handoff_aad(owner_pubkey_z32: &str, request_id: &str) -> String {
    let path = secure_handoff_path(request_id);
    format!(
        "{}:{}:{}:{}:{}",
        AAD_PREFIX, PURPOSE_HANDOFF, owner_pubkey_z32, path, request_id
    )
}

/// Build AAD for an ACK.
///
/// Format: `paykit:v0:ack_{object_type}:{ack_writer_z32}:{path}:{msg_id}`
///
/// # Arguments
///
/// * `object_type` - Type of object being ACKed (e.g., "request", "subscription_proposal")
/// * `ack_writer_pubkey_z32` - The ACK writer's z-base-32 encoded pubkey (receiver)
/// * `sender_pubkey_z32` - The original sender's z-base-32 encoded pubkey
/// * `recipient_pubkey_z32` - The recipient's z-base-32 encoded pubkey
/// * `msg_id` - The original message's identifier
///
/// # Returns
///
/// The AAD string to use with Sealed Blob v2 encryption.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::ack_aad;
///
/// let aad = ack_aad(
///     "request",
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "req_001"
/// ).unwrap();
/// assert!(aad.starts_with("paykit:v0:ack_request:"));
/// ```
pub fn ack_aad(
    object_type: &str,
    ack_writer_pubkey_z32: &str,
    sender_pubkey_z32: &str,
    recipient_pubkey_z32: &str,
    msg_id: &str,
) -> Result<String> {
    let path = ack_path(object_type, sender_pubkey_z32, recipient_pubkey_z32, msg_id)?;
    Ok(format!(
        "{}:ack_{}:{}:{}:{}",
        AAD_PREFIX, object_type, ack_writer_pubkey_z32, path, msg_id
    ))
}

/// Build AAD from explicit owner, path and ID.
///
/// This is the low-level builder for cases where you already have the path.
///
/// Format: `paykit:v0:{purpose}:{owner_z32}:{path}:{id}`
///
/// # Arguments
///
/// * `purpose` - The object type (use constants like `PURPOSE_REQUEST`)
/// * `owner` - The storage owner's z-base-32 pubkey
/// * `path` - The full storage path
/// * `id` - The object identifier
pub fn build_aad(purpose: &str, owner: &str, path: &str, id: &str) -> String {
    format!("{}:{}:{}:{}:{}", AAD_PREFIX, purpose, owner, path, id)
}

// ============================================================================
// Binary AAD per PUBKY_CRYPTO_SPEC v2.5 Section 7.5
// ============================================================================

/// Binary AAD prefix per PUBKY_CRYPTO_SPEC v2.5 Section 7.5.
pub const BINARY_AAD_PREFIX: &[u8] = b"pubky-envelope/v2:";

/// Build binary AAD per PUBKY_CRYPTO_SPEC v2.5 Section 7.5.
///
/// ```text
/// aad = aad_prefix || owner_peerid_bytes || canonical_path_bytes || header_bytes
/// ```
///
/// # Arguments
///
/// * `owner_peerid` - Storage owner's 32-byte Ed25519 public key
/// * `canonical_path` - Canonical storage path (e.g., "/pub/paykit.app/v0/requests/{context_id}/{msg_id}")
/// * `header_bytes` - Deterministic CBOR-encoded SB2 header (without signature)
///
/// # Returns
///
/// Binary AAD bytes for use with SB2 encryption/decryption.
///
/// # Example
///
/// ```rust,ignore
/// let owner_peerid = [0u8; 32]; // Owner's Ed25519 public key
/// let path = "/pub/paykit.app/v0/requests/abc123/req_001";
/// let header_bytes = vec![/* CBOR-encoded header */];
/// let aad = build_binary_aad(&owner_peerid, path, &header_bytes);
/// ```
pub fn build_binary_aad(
    owner_peerid: &[u8; 32],
    canonical_path: &str,
    header_bytes: &[u8],
) -> Vec<u8> {
    let path_bytes = canonical_path.as_bytes();
    let mut aad =
        Vec::with_capacity(BINARY_AAD_PREFIX.len() + 32 + path_bytes.len() + header_bytes.len());
    aad.extend_from_slice(BINARY_AAD_PREFIX);
    aad.extend_from_slice(owner_peerid);
    aad.extend_from_slice(path_bytes);
    aad.extend_from_slice(header_bytes);
    aad
}

/// Build binary AAD without header bytes.
///
/// This variant is useful when you need the AAD prefix for signature verification
/// before the header is available.
///
/// # Arguments
///
/// * `owner_peerid` - Storage owner's 32-byte Ed25519 public key
/// * `canonical_path` - Canonical storage path
///
/// # Returns
///
/// Binary AAD bytes without the header component.
pub fn build_binary_aad_prefix(owner_peerid: &[u8; 32], canonical_path: &str) -> Vec<u8> {
    let path_bytes = canonical_path.as_bytes();
    let mut aad = Vec::with_capacity(BINARY_AAD_PREFIX.len() + 32 + path_bytes.len());
    aad.extend_from_slice(BINARY_AAD_PREFIX);
    aad.extend_from_slice(owner_peerid);
    aad.extend_from_slice(path_bytes);
    aad
}

/// Check if data starts with SB2 magic bytes.
///
/// SB2 binary format starts with `SB2` (0x53 0x42 0x32).
/// Use this to determine whether to use binary or legacy string AAD.
pub fn is_sb2_format(data: &[u8]) -> bool {
    data.len() >= 3 && &data[..3] == b"SB2"
}

// ============================================================================
// z32 → owner_peerid_bytes Conversion (PUBKY_CRYPTO_SPEC v2.5)
// ============================================================================

/// Convert a z-base-32 encoded public key to 32-byte owner peerid bytes.
///
/// This is required for binary AAD construction per PUBKY_CRYPTO_SPEC v2.5 Section 7.5.
/// The `*_with_context` APIs in `pubky-noise` require the owner's Ed25519 public key
/// as raw bytes, but many call sites only have the z32 string representation.
///
/// # Arguments
///
/// * `z32` - z-base-32 encoded Ed25519 public key (52 characters)
///
/// # Returns
///
/// 32-byte owner peerid bytes suitable for `sealed_blob_encrypt_with_context` and
/// `sealed_blob_decrypt_with_context`.
///
/// # Errors
///
/// Returns `PaykitError::InvalidData` if the z32 string is invalid.
///
/// # Example
///
/// ```rust,ignore
/// use paykit_lib::protocol::owner_peerid_bytes_from_z32;
///
/// let z32 = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
/// let bytes = owner_peerid_bytes_from_z32(z32)?;
/// assert_eq!(bytes.len(), 32);
/// ```
#[cfg(feature = "pubky")]
pub fn owner_peerid_bytes_from_z32(z32: &str) -> Result<[u8; 32]> {
    use std::str::FromStr;
    let public_key = pubky::PublicKey::from_str(z32).map_err(|e| PaykitError::InvalidData {
        field: "z32_pubkey".into(),
        reason: format!("invalid z32 public key: {}", e),
    })?;
    Ok(public_key.to_bytes())
}

/// Convert a z-base-32 encoded public key to 32-byte owner peerid bytes.
///
/// Stub implementation when the `pubky` feature is disabled.
/// Returns an error indicating the feature is required.
#[cfg(not(feature = "pubky"))]
pub fn owner_peerid_bytes_from_z32(_z32: &str) -> Result<[u8; 32]> {
    Err(PaykitError::InvalidData {
        field: "z32_pubkey".into(),
        reason: "z32 conversion requires the 'pubky' feature".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER_PUBKEY: &str = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
    const RECIPIENT_PUBKEY: &str = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

    #[test]
    fn payment_request_aad_format() {
        let aad =
            payment_request_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123").unwrap();
        assert!(aad.starts_with("paykit:v0:request:"));
        assert!(aad.contains(SENDER_PUBKEY));
        assert!(aad.contains("/pub/paykit.app/v0/requests/"));
        assert!(aad.ends_with(":req-123"));
    }

    #[test]
    fn subscription_proposal_aad_format() {
        let aad =
            subscription_proposal_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "prop-456")
                .unwrap();
        assert!(aad.starts_with("paykit:v0:subscription_proposal:"));
        assert!(aad.contains(SENDER_PUBKEY));
        assert!(aad.contains("/pub/paykit.app/v0/subscriptions/proposals/"));
        assert!(aad.ends_with(":prop-456"));
    }

    #[test]
    fn secure_handoff_aad_format() {
        let aad = secure_handoff_aad(SENDER_PUBKEY, "handoff-789");
        assert!(aad.starts_with("paykit:v0:handoff:"));
        assert!(aad.contains(SENDER_PUBKEY));
        assert!(aad.contains("/pub/paykit.app/v0/handoff/handoff-789"));
        assert!(aad.ends_with(":handoff-789"));
    }

    #[test]
    fn ack_aad_format() {
        let aad = ack_aad(
            "request",
            RECIPIENT_PUBKEY,
            SENDER_PUBKEY,
            RECIPIENT_PUBKEY,
            "req_001",
        )
        .unwrap();
        assert!(aad.starts_with("paykit:v0:ack_request:"));
        assert!(aad.contains(RECIPIENT_PUBKEY));
        assert!(aad.contains("/pub/paykit.app/v0/acks/request/"));
        assert!(aad.ends_with(":req_001"));
    }

    #[test]
    fn build_aad_produces_correct_format() {
        let aad = build_aad("custom", "owner123", "/some/path", "id-123");
        assert_eq!(aad, "paykit:v0:custom:owner123:/some/path:id-123");
    }

    #[test]
    fn aad_is_deterministic() {
        let aad1 =
            payment_request_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123").unwrap();
        let aad2 =
            payment_request_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123").unwrap();
        assert_eq!(aad1, aad2);
    }

    #[test]
    fn aad_differs_for_different_ids() {
        let aad1 =
            payment_request_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123").unwrap();
        let aad2 =
            payment_request_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-456").unwrap();
        assert_ne!(aad1, aad2);
    }

    #[test]
    fn aad_differs_for_different_owners() {
        let aad1 =
            payment_request_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123").unwrap();
        let aad2 =
            payment_request_aad(RECIPIENT_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123")
                .unwrap();
        assert_ne!(aad1, aad2);
    }

    // ========================================================================
    // Binary AAD Tests (PUBKY_CRYPTO_SPEC v2.5)
    // ========================================================================

    #[test]
    fn binary_aad_has_correct_prefix() {
        let owner = [1u8; 32];
        let path = "/pub/paykit.app/v0/requests/abc/req_001";
        let header = vec![0xa1, 0x00, 0x01]; // minimal CBOR map

        let aad = build_binary_aad(&owner, path, &header);

        // Should start with the AAD prefix
        assert!(aad.starts_with(BINARY_AAD_PREFIX));
    }

    #[test]
    fn binary_aad_contains_all_components() {
        let owner = [42u8; 32];
        let path = "/pub/paykit.app/v0/requests/abc/req_001";
        let header = vec![0xa1, 0x00, 0x01];

        let aad = build_binary_aad(&owner, path, &header);

        // Expected length: prefix (18) + owner (32) + path (40) + header (3)
        let expected_len = BINARY_AAD_PREFIX.len() + 32 + path.len() + header.len();
        assert_eq!(aad.len(), expected_len);

        // Verify each component is present
        let prefix_end = BINARY_AAD_PREFIX.len();
        let owner_end = prefix_end + 32;
        let path_end = owner_end + path.len();

        assert_eq!(&aad[..prefix_end], BINARY_AAD_PREFIX);
        assert_eq!(&aad[prefix_end..owner_end], &owner);
        assert_eq!(&aad[owner_end..path_end], path.as_bytes());
        assert_eq!(&aad[path_end..], &header);
    }

    #[test]
    fn binary_aad_is_deterministic() {
        let owner = [1u8; 32];
        let path = "/pub/paykit.app/v0/requests/abc/req_001";
        let header = vec![0xa1, 0x00, 0x01];

        let aad1 = build_binary_aad(&owner, path, &header);
        let aad2 = build_binary_aad(&owner, path, &header);

        assert_eq!(aad1, aad2);
    }

    #[test]
    fn binary_aad_differs_for_different_owners() {
        let owner1 = [1u8; 32];
        let owner2 = [2u8; 32];
        let path = "/pub/paykit.app/v0/requests/abc/req_001";
        let header = vec![0xa1, 0x00, 0x01];

        let aad1 = build_binary_aad(&owner1, path, &header);
        let aad2 = build_binary_aad(&owner2, path, &header);

        assert_ne!(aad1, aad2);
    }

    #[test]
    fn binary_aad_differs_for_different_paths() {
        let owner = [1u8; 32];
        let path1 = "/pub/paykit.app/v0/requests/abc/req_001";
        let path2 = "/pub/paykit.app/v0/requests/xyz/req_001";
        let header = vec![0xa1, 0x00, 0x01];

        let aad1 = build_binary_aad(&owner, path1, &header);
        let aad2 = build_binary_aad(&owner, path2, &header);

        assert_ne!(aad1, aad2);
    }

    #[test]
    fn binary_aad_prefix_works_correctly() {
        let owner = [1u8; 32];
        let path = "/pub/paykit.app/v0/requests/abc/req_001";

        let prefix = build_binary_aad_prefix(&owner, path);

        // Should be prefix + owner + path (no header)
        let expected_len = BINARY_AAD_PREFIX.len() + 32 + path.len();
        assert_eq!(prefix.len(), expected_len);
    }

    #[test]
    fn is_sb2_format_detects_magic() {
        assert!(is_sb2_format(b"SB2\x02\x00\x10"));
        assert!(is_sb2_format(b"SB2"));
        assert!(!is_sb2_format(b"SB"));
        assert!(!is_sb2_format(b"JSON"));
        assert!(!is_sb2_format(b"{\"v\":2"));
    }

    // ========================================================================
    // z32 → owner_peerid_bytes Tests
    // ========================================================================

    #[cfg(feature = "pubky")]
    mod z32_tests {
        use super::*;

        #[test]
        fn valid_z32_converts_to_32_bytes() {
            // Generate a known keypair and verify roundtrip
            use ed25519_dalek::SigningKey;
            use rand::RngCore;

            let mut secret_bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut secret_bytes);
            let signing_key = SigningKey::from_bytes(&secret_bytes);
            let verifying_key = signing_key.verifying_key();
            let expected_bytes = verifying_key.to_bytes();

            // Create pubky PublicKey from the VerifyingKey and get z32
            let pubkey = pubky::PublicKey::from(verifying_key);
            let z32 = pubkey.to_string();

            // Convert back via our helper
            let result = owner_peerid_bytes_from_z32(&z32).unwrap();
            assert_eq!(result, expected_bytes);
        }

        #[test]
        fn z32_conversion_is_deterministic() {
            let z32 = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
            let result1 = owner_peerid_bytes_from_z32(z32);
            let result2 = owner_peerid_bytes_from_z32(z32);

            // Both should succeed or both should fail consistently
            assert_eq!(result1.is_ok(), result2.is_ok());
            if let Ok(bytes1) = result1 {
                assert_eq!(bytes1, result2.expect("consistent with first call"));
            }
        }

        #[test]
        fn invalid_z32_returns_error() {
            // Too short
            let result = owner_peerid_bytes_from_z32("abc");
            assert!(result.is_err());

            // Invalid characters
            let result = owner_peerid_bytes_from_z32("INVALID!!!");
            assert!(result.is_err());

            // Empty string
            let result = owner_peerid_bytes_from_z32("");
            assert!(result.is_err());
        }

        #[test]
        fn z32_result_has_correct_length() {
            // Use a well-formed z32 string from a random keypair
            use ed25519_dalek::SigningKey;
            use rand::RngCore;

            let mut secret_bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut secret_bytes);
            let signing_key = SigningKey::from_bytes(&secret_bytes);
            let verifying_key = signing_key.verifying_key();
            let pubkey = pubky::PublicKey::from(verifying_key);
            let z32 = pubkey.to_string();

            let result = owner_peerid_bytes_from_z32(&z32).unwrap();
            assert_eq!(result.len(), 32);
        }
    }
}
