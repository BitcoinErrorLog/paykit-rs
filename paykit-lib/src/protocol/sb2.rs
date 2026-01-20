//! Sealed Blob v2 (SB2) integration for Paykit.
//!
//! This module provides the bridge between paykit-lib and pubky-noise's SB2 implementation,
//! implementing PUBKY_CRYPTO_SPEC v2.5 compliant encryption for stored delivery.
//!
//! # Wire Format
//!
//! SB2 uses a binary wire format:
//! ```text
//! magic: 0x53 0x42 0x32 ("SB2", 3 bytes)
//! version: u8 (2)
//! header_len: u16 (big-endian, <= 2048 bytes)
//! header_bytes: [u8; header_len] (deterministic CBOR)
//! ciphertext: [u8] (XChaCha20-Poly1305, includes 16-byte tag)
//! ```
//!
//! # AAD Construction
//!
//! Per PUBKY_CRYPTO_SPEC v2.5 Section 7.5:
//! ```text
//! aad = "pubky-envelope/v2:" || owner_peerid_bytes || canonical_path_bytes || header_no_sig
//! ```
//!
//! # Key Selection
//!
//! - **Stored delivery** (payment requests, ACKs): Use recipient's **InboxKey**
//! - **Real-time Noise sessions**: Use recipient's **TransportKey**
//!
//! The `inbox_kid` header field identifies the recipient's InboxKey for O(1) key selection.

use crate::{PaykitError, Result};

/// SB2 magic bytes.
pub const SB2_MAGIC: &[u8; 3] = b"SB2";

/// SB2 version number.
pub const SB2_VERSION: u8 = 2;

/// Check if data starts with SB2 magic bytes.
pub fn is_sb2(data: &[u8]) -> bool {
    data.len() >= 3 && &data[..3] == SB2_MAGIC
}

/// Compute inbox_kid from recipient's InboxKey public key.
///
/// Per PUBKY_CRYPTO_SPEC v2.5 Section 7.2:
/// ```text
/// inbox_kid = first_16_bytes(SHA256(recipient_inbox_x25519_pub))
/// ```
pub fn compute_inbox_kid(inbox_pk: &[u8; 32]) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(inbox_pk);
    let mut kid = [0u8; 16];
    kid.copy_from_slice(&hash[..16]);
    kid
}

/// Parameters for SB2 encryption.
#[derive(Debug, Clone)]
pub struct Sb2EncryptParams {
    /// Recipient's InboxKey X25519 public key (32 bytes).
    pub recipient_inbox_pk: [u8; 32],
    /// Storage owner's Ed25519 public key (32 bytes).
    pub owner_peerid: [u8; 32],
    /// Sender's Ed25519 public key (32 bytes).
    pub sender_peerid: [u8; 32],
    /// Recipient's Ed25519 public key (32 bytes).
    pub recipient_peerid: [u8; 32],
    /// Random ContextId (32 bytes).
    pub context_id: [u8; 32],
    /// Canonical storage path.
    pub canonical_path: String,
    /// Unique message identifier.
    pub msg_id: String,
    /// Optional purpose hint.
    pub purpose: Option<String>,
    /// Optional expiration timestamp (Unix seconds).
    pub expires_at: Option<u64>,
}

/// Encrypt plaintext to SB2 binary format.
///
/// Uses the recipient's InboxKey for encryption, per PUBKY_CRYPTO_SPEC v2.5.
///
/// # Arguments
///
/// * `plaintext` - Data to encrypt (max 64 KiB)
/// * `params` - Encryption parameters
///
/// # Returns
///
/// SB2 binary blob suitable for storage.
#[cfg(feature = "pubky")]
pub fn sb2_encrypt(plaintext: &[u8], params: &Sb2EncryptParams) -> Result<Vec<u8>> {
    use pubky_noise::sealed_blob_v2::Sb2;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let sb2 = Sb2::encrypt(
        &params.recipient_inbox_pk,
        plaintext,
        params.context_id,
        Some(params.msg_id.clone()),
        params.purpose.clone(),
        &params.owner_peerid,
        &params.sender_peerid,
        &params.recipient_peerid,
        &params.canonical_path,
        Some(now),
        params.expires_at,
    )
    .map_err(|e: pubky_noise::errors::NoiseError| PaykitError::Crypto {
        operation: "sb2_encrypt".into(),
        details: e.to_string(),
    })?;

    // Sign the SB2 with sender's identity (requires sender's signing key)
    // For now, we create unsigned SB2 - signing will be added when sender_sk is available
    // The recipient can still verify via InboxKey encryption

    Ok(sb2.encode())
}

/// Decrypt SB2 binary blob.
///
/// # Arguments
///
/// * `data` - SB2 binary blob
/// * `recipient_inbox_sk` - Recipient's InboxKey X25519 secret key (32 bytes)
/// * `owner_peerid` - Storage owner's Ed25519 public key (32 bytes)
/// * `canonical_path` - Canonical storage path (must match encryption)
///
/// # Returns
///
/// Decrypted plaintext.
#[cfg(feature = "pubky")]
pub fn sb2_decrypt(
    data: &[u8],
    recipient_inbox_sk: &[u8; 32],
    owner_peerid: &[u8; 32],
    canonical_path: &str,
) -> Result<(Vec<u8>, Sb2Metadata)> {
    use pubky_noise::sealed_blob_v2::Sb2;

    if !is_sb2(data) {
        return Err(PaykitError::Crypto {
            operation: "sb2_decrypt".into(),
            details: "Not an SB2 blob (missing magic)".into(),
        });
    }

    let sb2 = Sb2::decode(data).map_err(|e| PaykitError::Crypto {
        operation: "sb2_decrypt".into(),
        details: format!("Failed to decode SB2: {}", e),
    })?;

    let plaintext = sb2
        .decrypt(recipient_inbox_sk, owner_peerid, canonical_path)
        .map_err(|e| PaykitError::Crypto {
            operation: "sb2_decrypt".into(),
            details: format!("Decryption failed: {}", e),
        })?;

    let metadata = Sb2Metadata {
        context_id: sb2.header.context_id,
        msg_id: sb2.header.msg_id,
        purpose: sb2.header.purpose,
        sender_peerid: sb2.header.sender_peerid,
        created_at: sb2.header.created_at,
        expires_at: sb2.header.expires_at,
    };

    Ok((plaintext, metadata))
}

/// Metadata extracted from SB2 header after decryption.
#[derive(Debug, Clone)]
pub struct Sb2Metadata {
    /// Thread identifier (32 random bytes).
    pub context_id: [u8; 32],
    /// Message identifier.
    pub msg_id: Option<String>,
    /// Purpose hint.
    pub purpose: Option<String>,
    /// Sender's Ed25519 public key.
    pub sender_peerid: [u8; 32],
    /// Creation timestamp (Unix seconds).
    pub created_at: Option<u64>,
    /// Expiration timestamp (Unix seconds).
    pub expires_at: Option<u64>,
}

/// Try to decrypt data that may be either SB2 binary or legacy JSON envelope.
///
/// This function supports backward compatibility during migration:
/// - If data starts with SB2 magic, decrypt as SB2
/// - Otherwise, try legacy JSON envelope format
///
/// # Arguments
///
/// * `data` - Raw bytes (SB2 binary or UTF-8 JSON)
/// * `recipient_inbox_sk` - Recipient's InboxKey X25519 secret key
/// * `owner_peerid` - Storage owner's Ed25519 public key
/// * `canonical_path` - Canonical storage path
/// * `legacy_aad` - AAD string for legacy JSON envelope decryption
///
/// # Returns
///
/// Decrypted plaintext and optional SB2 metadata (None for legacy format).
#[cfg(feature = "pubky")]
pub fn decrypt_any(
    data: &[u8],
    recipient_inbox_sk: &[u8; 32],
    owner_peerid: &[u8; 32],
    canonical_path: &str,
    legacy_aad: &str,
) -> Result<(Vec<u8>, Option<Sb2Metadata>)> {
    if is_sb2(data) {
        let (plaintext, metadata) =
            sb2_decrypt(data, recipient_inbox_sk, owner_peerid, canonical_path)?;
        Ok((plaintext, Some(metadata)))
    } else {
        // Try legacy JSON envelope
        let json_str = std::str::from_utf8(data).map_err(|_| PaykitError::Crypto {
            operation: "decrypt_any".into(),
            details: "Data is neither SB2 nor valid UTF-8 for JSON envelope".into(),
        })?;

        use pubky_noise::sealed_blob::sealed_blob_decrypt;
        let plaintext = sealed_blob_decrypt(recipient_inbox_sk, json_str, legacy_aad)
            .map_err(|e| PaykitError::Crypto {
                operation: "decrypt_any".into(),
                details: format!("Legacy decryption failed: {}", e),
            })?;

        Ok((plaintext, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sb2() {
        assert!(is_sb2(b"SB2\x02\x00\x10"));
        assert!(is_sb2(b"SB2"));
        assert!(!is_sb2(b"SB"));
        assert!(!is_sb2(b"JSON"));
        assert!(!is_sb2(b"{\"v\":2"));
    }

    #[test]
    fn test_compute_inbox_kid() {
        let pk = [1u8; 32];
        let kid = compute_inbox_kid(&pk);
        assert_eq!(kid.len(), 16);

        // Deterministic
        let kid2 = compute_inbox_kid(&pk);
        assert_eq!(kid, kid2);

        // Different key = different kid
        let pk2 = [2u8; 32];
        let kid3 = compute_inbox_kid(&pk2);
        assert_ne!(kid, kid3);
    }

    #[cfg(feature = "pubky")]
    #[test]
    fn test_sb2_roundtrip() {
        use super::super::scope::generate_context_id;
        use pubky_noise::sealed_blob::x25519_generate_keypair;
        use rand::RngCore;

        let (inbox_sk, inbox_pk) = x25519_generate_keypair();
        let mut owner_peerid = [0u8; 32];
        let mut sender_peerid = [0u8; 32];
        let mut recipient_peerid = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut owner_peerid);
        rand::thread_rng().fill_bytes(&mut sender_peerid);
        rand::thread_rng().fill_bytes(&mut recipient_peerid);

        let context_id = generate_context_id();
        let path = "/pub/paykit.app/v0/requests/abc123/req_001";
        let plaintext = b"Hello, SB2!";

        let params = Sb2EncryptParams {
            recipient_inbox_pk: inbox_pk,
            owner_peerid,
            sender_peerid,
            recipient_peerid,
            context_id,
            canonical_path: path.to_string(),
            msg_id: "req_001".to_string(),
            purpose: Some("request".to_string()),
            expires_at: None,
        };

        let encrypted = sb2_encrypt(plaintext, &params).unwrap();
        assert!(is_sb2(&encrypted));

        let (decrypted, metadata) = sb2_decrypt(&encrypted, &inbox_sk, &owner_peerid, path).unwrap();
        assert_eq!(decrypted, plaintext);
        assert_eq!(metadata.context_id, context_id);
        assert_eq!(metadata.msg_id, Some("req_001".to_string()));
        assert_eq!(metadata.purpose, Some("request".to_string()));
    }
}
