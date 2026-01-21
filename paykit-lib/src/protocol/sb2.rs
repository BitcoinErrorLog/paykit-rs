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
//!
//! # Signing
//!
//! Per PUBKY_CRYPTO_SPEC v2.5, Paykit SB2 envelopes MUST be signed. This module provides:
//! - `Sb2Signer` trait for abstracting signing capability
//! - `sb2_encrypt_signed` for producing signed SB2 envelopes
//! - Signature verification on decrypt via `sb2_decrypt`

#[cfg(feature = "pubky")]
use crate::{PaykitError, Result};

// =============================================================================
// Sb2Signer Trait - Abstracts signing capability for SB2 envelopes
// =============================================================================

/// Trait for signing SB2 envelopes without requiring direct ed25519-dalek dependency.
///
/// Implementors provide signing capability using either a RootKey or an AppKey.
/// This allows paykit-lib to produce signed SB2 envelopes while delegating
/// the actual Ed25519 signing to pubky-noise helpers.
///
/// # Example
///
/// ```rust,ignore
/// use paykit_lib::protocol::{Sb2Signer, Sb2EncryptParams, sb2_encrypt_signed};
///
/// struct RootKeySigner {
///     secret_key: [u8; 32],
///     public_key: [u8; 32],
/// }
///
/// impl Sb2Signer for RootKeySigner {
///     fn sender_peerid(&self) -> [u8; 32] {
///         self.public_key
///     }
///
///     fn cert_id(&self) -> Option<[u8; 16]> {
///         None // RootKey signing, no cert_id
///     }
///
///     fn sign_sig_input(&self, sig_input: &[u8]) -> Result<[u8; 64], String> {
///         pubky_noise::ed25519_sign(&self.secret_key, sig_input)
///             .map_err(|e| e.to_string())
///     }
/// }
/// ```
pub trait Sb2Signer {
    /// Returns the sender's Ed25519 public key (peerid).
    ///
    /// For RootKey signing, this is the RootKey public key.
    /// For AppKey signing, this is still the RootKey public key (identity),
    /// but the signature is made with the AppKey.
    fn sender_peerid(&self) -> [u8; 32];

    /// Returns the AppCert identifier if signing with an AppKey.
    ///
    /// - Returns `None` for RootKey signing (no delegation)
    /// - Returns `Some([u8; 16])` for AppKey signing (delegated via cert_id)
    fn cert_id(&self) -> Option<[u8; 16]>;

    /// Sign the sig_input bytes and return a 64-byte Ed25519 signature.
    ///
    /// The sig_input is computed per PUBKY_CRYPTO_SPEC Section 7.2.1:
    /// ```text
    /// sig_input = BLAKE3("pubky-envelope-sig/v2" || aad || header_no_sig || ciphertext)
    /// ```
    ///
    /// # Arguments
    ///
    /// * `sig_input` - 32-byte BLAKE3 hash to sign
    ///
    /// # Returns
    ///
    /// 64-byte Ed25519 signature on success, or error message on failure.
    fn sign_sig_input(&self, sig_input: &[u8]) -> std::result::Result<[u8; 64], String>;
}

/// A signer implementation using a RootKey (Ed25519 identity key).
///
/// This is the simplest signer that signs directly with the user's identity key.
#[cfg(feature = "pubky")]
pub struct RootKeySigner {
    /// Ed25519 secret key (32 bytes seed)
    secret_key: [u8; 32],
    /// Ed25519 public key (32 bytes)
    public_key: [u8; 32],
}

#[cfg(feature = "pubky")]
impl RootKeySigner {
    /// Create a new RootKeySigner from an Ed25519 keypair.
    ///
    /// # Arguments
    ///
    /// * `secret_key` - 32-byte Ed25519 secret key (seed)
    /// * `public_key` - 32-byte Ed25519 public key
    pub fn new(secret_key: [u8; 32], public_key: [u8; 32]) -> Self {
        Self { secret_key, public_key }
    }
}

#[cfg(feature = "pubky")]
impl Sb2Signer for RootKeySigner {
    fn sender_peerid(&self) -> [u8; 32] {
        self.public_key
    }

    fn cert_id(&self) -> Option<[u8; 16]> {
        None
    }

    fn sign_sig_input(&self, sig_input: &[u8]) -> std::result::Result<[u8; 64], String> {
        pubky_noise::ed25519_sign(&self.secret_key, sig_input)
            .map_err(|e| e.to_string())
    }
}

/// A signer implementation using an AppKey (delegated signing via AppCert).
///
/// This signer uses a delegated AppKey to sign, with the cert_id included
/// in the SB2 header so verifiers can fetch the AppCert to verify.
#[cfg(feature = "pubky")]
pub struct AppKeySigner {
    /// AppKey Ed25519 secret key (32 bytes seed)
    app_secret_key: [u8; 32],
    /// RootKey Ed25519 public key (identity/peerid)
    root_public_key: [u8; 32],
    /// AppCert identifier (16 bytes)
    cert_id: [u8; 16],
}

#[cfg(feature = "pubky")]
impl AppKeySigner {
    /// Create a new AppKeySigner for delegated signing.
    ///
    /// # Arguments
    ///
    /// * `app_secret_key` - 32-byte AppKey Ed25519 secret key
    /// * `root_public_key` - 32-byte RootKey Ed25519 public key (identity)
    /// * `cert_id` - 16-byte AppCert identifier
    pub fn new(app_secret_key: [u8; 32], root_public_key: [u8; 32], cert_id: [u8; 16]) -> Self {
        Self {
            app_secret_key,
            root_public_key,
            cert_id,
        }
    }
}

#[cfg(feature = "pubky")]
impl Sb2Signer for AppKeySigner {
    fn sender_peerid(&self) -> [u8; 32] {
        self.root_public_key
    }

    fn cert_id(&self) -> Option<[u8; 16]> {
        Some(self.cert_id)
    }

    fn sign_sig_input(&self, sig_input: &[u8]) -> std::result::Result<[u8; 64], String> {
        pubky_noise::ed25519_sign(&self.app_secret_key, sig_input)
            .map_err(|e| e.to_string())
    }
}

// =============================================================================
// SB2 Constants and Helpers
// =============================================================================

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
///
/// Delegates to `pubky_noise::Sb2Header::compute_inbox_kid()`.
///
/// Requires the `pubky` feature.
#[cfg(feature = "pubky")]
pub fn compute_inbox_kid(inbox_pk: &[u8; 32]) -> [u8; 16] {
    pubky_noise::Sb2Header::compute_inbox_kid(inbox_pk)
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

/// Encrypt plaintext to SB2 binary format with signature.
///
/// Uses the recipient's InboxKey for encryption, per PUBKY_CRYPTO_SPEC v2.5.
/// The envelope is signed using the provided `Sb2Signer`.
///
/// # Arguments
///
/// * `plaintext` - Data to encrypt (max 64 KiB)
/// * `params` - Encryption parameters
/// * `signer` - Signing capability (RootKey or AppKey)
///
/// # Returns
///
/// Signed SB2 binary blob suitable for storage.
#[cfg(feature = "pubky")]
pub fn sb2_encrypt_signed(
    plaintext: &[u8],
    params: &Sb2EncryptParams,
    signer: &dyn Sb2Signer,
) -> Result<Vec<u8>> {
    use pubky_noise::sealed_blob_v2::Sb2;
    use pubky_noise::{sb2_build_aad, sb2_compute_sig_input};

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Create the unsigned SB2 envelope with cert_id (if signing with AppKey)
    // cert_id must be included before encryption so AAD includes it
    let mut sb2 = Sb2::encrypt_with_cert_id(
        &params.recipient_inbox_pk,
        plaintext,
        params.context_id,
        Some(params.msg_id.clone()),
        params.purpose.clone(),
        &params.owner_peerid,
        &signer.sender_peerid(),
        &params.recipient_peerid,
        &params.canonical_path,
        Some(now),
        params.expires_at,
        signer.cert_id(),
    )
    .map_err(|e: pubky_noise::errors::NoiseError| PaykitError::Crypto {
        operation: "sb2_encrypt_signed".into(),
        details: e.to_string(),
    })?;

    // Compute signature input per PUBKY_CRYPTO_SPEC Section 7.2.1
    let header_no_sig = sb2.header.encode_no_sig();
    let aad = sb2_build_aad(&params.owner_peerid, &params.canonical_path, &header_no_sig);
    let sig_input = sb2_compute_sig_input(&aad, &header_no_sig, &sb2.ciphertext);

    // Sign and set signature
    let sig = signer.sign_sig_input(&sig_input).map_err(|e| PaykitError::Crypto {
        operation: "sb2_encrypt_signed".into(),
        details: format!("Signing failed: {}", e),
    })?;
    sb2.header.sig = Some(sig);

    Ok(sb2.encode())
}

/// Encrypt plaintext to SB2 binary format (unsigned, for backward compatibility).
///
/// **DEPRECATED for Paykit purposes**: Use `sb2_encrypt_signed` instead.
/// Paykit protocol messages (requests, proposals, ACKs) MUST be signed per spec.
///
/// This function is retained for backward compatibility during migration
/// and for non-Paykit use cases where signatures are optional.
///
/// # Arguments
///
/// * `plaintext` - Data to encrypt (max 64 KiB)
/// * `params` - Encryption parameters
///
/// # Returns
///
/// Unsigned SB2 binary blob.
#[cfg(feature = "pubky")]
#[deprecated(
    since = "2.1.0",
    note = "Use sb2_encrypt_signed for Paykit protocol messages which MUST be signed per PUBKY_CRYPTO_SPEC v2.5"
)]
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

    Ok(sb2.encode())
}

/// Signature verification options for SB2 decryption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureRequirement {
    /// Signature is required. Fails if missing or invalid.
    Required,
    /// Signature is optional. Verifies if present, allows missing.
    Optional,
    /// Skip signature verification entirely.
    Skip,
}

impl Default for SignatureRequirement {
    fn default() -> Self {
        Self::Required
    }
}

/// Callback trait for fetching AppCert to verify delegated signatures.
///
/// When an SB2 envelope has a `cert_id` in the header, the signature was made
/// with an AppKey. To verify, we need to fetch the AppCert and extract the
/// AppKey public key.
#[cfg(feature = "pubky")]
pub trait AppCertFetcher {
    /// Fetch AppCert for the given sender and cert_id.
    ///
    /// # Arguments
    ///
    /// * `sender_peerid` - Sender's RootKey Ed25519 public key (identity)
    /// * `cert_id` - AppCert identifier from the SB2 header
    ///
    /// # Returns
    ///
    /// The AppKey Ed25519 public key if AppCert is valid, or error message.
    fn fetch_app_key(
        &self,
        sender_peerid: &[u8; 32],
        cert_id: &[u8; 16],
    ) -> std::result::Result<[u8; 32], String>;
}

/// A no-op AppCert fetcher that always fails.
///
/// Use this when you don't support delegated signatures and want to
/// reject any SB2 with a cert_id.
#[cfg(feature = "pubky")]
pub struct NoAppCertSupport;

#[cfg(feature = "pubky")]
impl AppCertFetcher for NoAppCertSupport {
    fn fetch_app_key(
        &self,
        _sender_peerid: &[u8; 32],
        _cert_id: &[u8; 16],
    ) -> std::result::Result<[u8; 32], String> {
        Err("Delegated signatures (AppKey/cert_id) not supported".into())
    }
}

/// Decrypt SB2 binary blob with signature verification.
///
/// This is the recommended decryption function for Paykit protocol messages.
/// It verifies the signature before returning plaintext, ensuring message
/// authenticity and integrity.
///
/// # Arguments
///
/// * `data` - SB2 binary blob
/// * `recipient_inbox_sk` - Recipient's InboxKey X25519 secret key (32 bytes)
/// * `owner_peerid` - Storage owner's Ed25519 public key (32 bytes)
/// * `canonical_path` - Canonical storage path (must match encryption)
/// * `sig_requirement` - Signature verification requirement
/// * `cert_fetcher` - Optional callback for fetching AppCert for delegated signatures
///
/// # Returns
///
/// Decrypted plaintext and metadata.
#[cfg(feature = "pubky")]
pub fn sb2_decrypt_verified(
    data: &[u8],
    recipient_inbox_sk: &[u8; 32],
    owner_peerid: &[u8; 32],
    canonical_path: &str,
    sig_requirement: SignatureRequirement,
    cert_fetcher: Option<&dyn AppCertFetcher>,
) -> Result<(Vec<u8>, Sb2Metadata)> {
    use pubky_noise::sealed_blob_v2::Sb2;
    use pubky_noise::{sb2_build_aad, sb2_compute_sig_input, ed25519_verify};

    if !is_sb2(data) {
        return Err(PaykitError::Crypto {
            operation: "sb2_decrypt_verified".into(),
            details: "Not an SB2 blob (missing magic)".into(),
        });
    }

    let sb2 = Sb2::decode(data).map_err(|e| PaykitError::Crypto {
        operation: "sb2_decrypt_verified".into(),
        details: format!("Failed to decode SB2: {}", e),
    })?;

    // Verify signature before decryption (if required)
    if sig_requirement != SignatureRequirement::Skip {
        match &sb2.header.sig {
            Some(sig) => {
                // Compute signature input
                let header_no_sig = sb2.header.encode_no_sig();
                let aad = sb2_build_aad(owner_peerid, canonical_path, &header_no_sig);
                let sig_input = sb2_compute_sig_input(&aad, &header_no_sig, &sb2.ciphertext);

                // Determine which public key to verify against
                let verifying_key = if let Some(cert_id) = &sb2.header.cert_id {
                    // Delegated signature - need to fetch AppKey via AppCert
                    let fetcher = cert_fetcher.ok_or_else(|| PaykitError::Crypto {
                        operation: "sb2_decrypt_verified".into(),
                        details: "SB2 has cert_id but no AppCertFetcher provided".into(),
                    })?;

                    fetcher
                        .fetch_app_key(&sb2.header.sender_peerid, cert_id)
                        .map_err(|e| PaykitError::Crypto {
                            operation: "sb2_decrypt_verified".into(),
                            details: format!("Failed to fetch AppCert: {}", e),
                        })?
                } else {
                    // RootKey signature - verify against sender_peerid
                    sb2.header.sender_peerid
                };

                // Verify signature
                if !ed25519_verify(&verifying_key, &sig_input, sig) {
                    return Err(PaykitError::Crypto {
                        operation: "sb2_decrypt_verified".into(),
                        details: "Signature verification failed".into(),
                    });
                }
            }
            None => {
                if sig_requirement == SignatureRequirement::Required {
                    return Err(PaykitError::Crypto {
                        operation: "sb2_decrypt_verified".into(),
                        details: "Signature required but missing".into(),
                    });
                }
                // sig_requirement == Optional, allow missing signature
            }
        }
    }

    // Decrypt
    let plaintext = sb2
        .decrypt(recipient_inbox_sk, owner_peerid, canonical_path)
        .map_err(|e| PaykitError::Crypto {
            operation: "sb2_decrypt_verified".into(),
            details: format!("Decryption failed: {}", e),
        })?;

    let metadata = Sb2Metadata {
        context_id: sb2.header.context_id,
        msg_id: sb2.header.msg_id,
        purpose: sb2.header.purpose,
        sender_peerid: sb2.header.sender_peerid,
        created_at: sb2.header.created_at,
        expires_at: sb2.header.expires_at,
        cert_id: sb2.header.cert_id,
        signature_verified: sb2.header.sig.is_some() && sig_requirement != SignatureRequirement::Skip,
    };

    Ok((plaintext, metadata))
}

/// Decrypt SB2 binary blob (legacy function without signature verification).
///
/// **DEPRECATED for Paykit purposes**: Use `sb2_decrypt_verified` instead.
/// This function does not verify signatures and should only be used for
/// backward compatibility during migration.
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
#[deprecated(
    since = "2.1.0",
    note = "Use sb2_decrypt_verified for Paykit protocol messages to ensure signature verification"
)]
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
        cert_id: sb2.header.cert_id,
        signature_verified: false,
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
    /// Sender's Ed25519 public key (RootKey identity).
    pub sender_peerid: [u8; 32],
    /// Creation timestamp (Unix seconds).
    pub created_at: Option<u64>,
    /// Expiration timestamp (Unix seconds).
    pub expires_at: Option<u64>,
    /// AppCert identifier if signed with AppKey (delegated signature).
    pub cert_id: Option<[u8; 16]>,
    /// Whether the signature was verified during decryption.
    pub signature_verified: bool,
}

/// Try to decrypt data that may be either SB2 binary or legacy JSON envelope.
///
/// This function supports backward compatibility during migration:
/// - If data starts with SB2 magic, decrypt as SB2 (with optional signature verification)
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
///
/// # Note
///
/// This function uses `SignatureRequirement::Optional` for backward compatibility.
/// For new Paykit protocol messages, use `sb2_decrypt_verified` with `SignatureRequirement::Required`.
#[cfg(feature = "pubky")]
pub fn decrypt_any(
    data: &[u8],
    recipient_inbox_sk: &[u8; 32],
    owner_peerid: &[u8; 32],
    canonical_path: &str,
    legacy_aad: &str,
) -> Result<(Vec<u8>, Option<Sb2Metadata>)> {
    if is_sb2(data) {
        // Use optional signature verification for backward compatibility
        let (plaintext, metadata) = sb2_decrypt_verified(
            data,
            recipient_inbox_sk,
            owner_peerid,
            canonical_path,
            SignatureRequirement::Optional,
            None,
        )?;
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
    #[allow(deprecated)]
    fn test_sb2_roundtrip_unsigned_legacy() {
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
        assert!(!metadata.signature_verified);
    }

    #[cfg(feature = "pubky")]
    #[test]
    fn test_sb2_signed_roundtrip() {
        use super::super::scope::generate_context_id;
        use pubky_noise::sealed_blob::x25519_generate_keypair;
        use ed25519_dalek::SigningKey;
        use rand::RngCore;

        // Generate keys
        let (inbox_sk, inbox_pk) = x25519_generate_keypair();
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);
        let sender_signing_key = SigningKey::from_bytes(&seed);
        let sender_peerid = sender_signing_key.verifying_key().to_bytes();

        let mut owner_peerid = [0u8; 32];
        let mut recipient_peerid = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut owner_peerid);
        rand::thread_rng().fill_bytes(&mut recipient_peerid);

        let context_id = generate_context_id();
        let path = "/pub/paykit.app/v0/requests/abc123/req_002";
        let plaintext = b"Hello, Signed SB2!";

        let params = Sb2EncryptParams {
            recipient_inbox_pk: inbox_pk,
            owner_peerid,
            sender_peerid,
            recipient_peerid,
            context_id,
            canonical_path: path.to_string(),
            msg_id: "req_002".to_string(),
            purpose: Some("request".to_string()),
            expires_at: None,
        };

        // Create signer
        let signer = RootKeySigner::new(seed, sender_peerid);

        // Encrypt with signature
        let encrypted = sb2_encrypt_signed(plaintext, &params, &signer).unwrap();
        assert!(is_sb2(&encrypted));

        // Decrypt with signature verification required
        let (decrypted, metadata) = sb2_decrypt_verified(
            &encrypted,
            &inbox_sk,
            &owner_peerid,
            path,
            SignatureRequirement::Required,
            None,
        )
        .unwrap();

        assert_eq!(decrypted, plaintext);
        assert_eq!(metadata.context_id, context_id);
        assert_eq!(metadata.msg_id, Some("req_002".to_string()));
        assert_eq!(metadata.purpose, Some("request".to_string()));
        assert!(metadata.signature_verified);
        assert!(metadata.cert_id.is_none()); // RootKey signing
    }

    #[cfg(feature = "pubky")]
    #[test]
    fn test_sb2_missing_signature_fails_when_required() {
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
        let path = "/pub/paykit.app/v0/requests/abc123/req_003";
        let plaintext = b"Unsigned message";

        let params = Sb2EncryptParams {
            recipient_inbox_pk: inbox_pk,
            owner_peerid,
            sender_peerid,
            recipient_peerid,
            context_id,
            canonical_path: path.to_string(),
            msg_id: "req_003".to_string(),
            purpose: Some("request".to_string()),
            expires_at: None,
        };

        // Encrypt without signature (using deprecated function)
        #[allow(deprecated)]
        let encrypted = sb2_encrypt(plaintext, &params).unwrap();

        // Decrypt with signature required should fail
        let result = sb2_decrypt_verified(
            &encrypted,
            &inbox_sk,
            &owner_peerid,
            path,
            SignatureRequirement::Required,
            None,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Signature required but missing"));
    }

    #[cfg(feature = "pubky")]
    #[test]
    fn test_sb2_invalid_signature_fails() {
        use super::super::scope::generate_context_id;
        use pubky_noise::sealed_blob::x25519_generate_keypair;
        use ed25519_dalek::SigningKey;
        use rand::RngCore;

        // Generate keys
        let (inbox_sk, inbox_pk) = x25519_generate_keypair();
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);

        let sender_signing_key = SigningKey::from_bytes(&seed);
        let sender_peerid = sender_signing_key.verifying_key().to_bytes();

        let mut owner_peerid = [0u8; 32];
        let mut recipient_peerid = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut owner_peerid);
        rand::thread_rng().fill_bytes(&mut recipient_peerid);

        let context_id = generate_context_id();
        let path = "/pub/paykit.app/v0/requests/abc123/req_004";
        let plaintext = b"Message with tampered signature";

        let params = Sb2EncryptParams {
            recipient_inbox_pk: inbox_pk,
            owner_peerid,
            sender_peerid,
            recipient_peerid,
            context_id,
            canonical_path: path.to_string(),
            msg_id: "req_004".to_string(),
            purpose: Some("request".to_string()),
            expires_at: None,
        };

        let signer = RootKeySigner::new(seed, sender_peerid);
        let mut encrypted = sb2_encrypt_signed(plaintext, &params, &signer).unwrap();

        // Tamper with ciphertext to invalidate signature
        if let Some(last) = encrypted.last_mut() {
            *last ^= 0xFF;
        }

        // Decrypt with signature required should fail due to tampering
        let result = sb2_decrypt_verified(
            &encrypted,
            &inbox_sk,
            &owner_peerid,
            path,
            SignatureRequirement::Required,
            None,
        );

        assert!(result.is_err());
    }

    #[cfg(feature = "pubky")]
    #[test]
    fn test_sb2_signature_optional_with_valid_signature() {
        use super::super::scope::generate_context_id;
        use pubky_noise::sealed_blob::x25519_generate_keypair;
        use ed25519_dalek::SigningKey;
        use rand::RngCore;

        let (inbox_sk, inbox_pk) = x25519_generate_keypair();
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);
        let sender_signing_key = SigningKey::from_bytes(&seed);
        let sender_peerid = sender_signing_key.verifying_key().to_bytes();

        let mut owner_peerid = [0u8; 32];
        let mut recipient_peerid = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut owner_peerid);
        rand::thread_rng().fill_bytes(&mut recipient_peerid);

        let context_id = generate_context_id();
        let path = "/pub/paykit.app/v0/requests/abc123/req_005";
        let plaintext = b"Signed message with optional verification";

        let params = Sb2EncryptParams {
            recipient_inbox_pk: inbox_pk,
            owner_peerid,
            sender_peerid,
            recipient_peerid,
            context_id,
            canonical_path: path.to_string(),
            msg_id: "req_005".to_string(),
            purpose: None,
            expires_at: None,
        };

        let signer = RootKeySigner::new(seed, sender_peerid);
        let encrypted = sb2_encrypt_signed(plaintext, &params, &signer).unwrap();

        // Decrypt with optional signature - should verify and succeed
        let (decrypted, metadata) = sb2_decrypt_verified(
            &encrypted,
            &inbox_sk,
            &owner_peerid,
            path,
            SignatureRequirement::Optional,
            None,
        )
        .unwrap();

        assert_eq!(decrypted, plaintext);
        assert!(metadata.signature_verified);
    }

    // =========================================================================
    // Delegated Signature (cert_id/AppKey) Tests
    // =========================================================================

    /// Mock AppCertFetcher for testing that returns a known AppKey public key.
    #[cfg(feature = "pubky")]
    struct MockAppCertFetcher {
        expected_sender: [u8; 32],
        expected_cert_id: [u8; 16],
        app_public_key: [u8; 32],
    }

    #[cfg(feature = "pubky")]
    impl AppCertFetcher for MockAppCertFetcher {
        fn fetch_app_key(
            &self,
            sender_peerid: &[u8; 32],
            cert_id: &[u8; 16],
        ) -> std::result::Result<[u8; 32], String> {
            if sender_peerid != &self.expected_sender {
                return Err("Wrong sender".into());
            }
            if cert_id != &self.expected_cert_id {
                return Err("Wrong cert_id".into());
            }
            Ok(self.app_public_key)
        }
    }

    #[cfg(feature = "pubky")]
    #[test]
    fn test_sb2_delegated_signature_success() {
        use super::super::scope::generate_context_id;
        use pubky_noise::sealed_blob::x25519_generate_keypair;
        use ed25519_dalek::SigningKey;
        use rand::RngCore;

        // Generate keys
        let (inbox_sk, inbox_pk) = x25519_generate_keypair();
        let mut root_seed = [0u8; 32];
        let mut app_seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut root_seed);
        rand::thread_rng().fill_bytes(&mut app_seed);

        let root_signing_key = SigningKey::from_bytes(&root_seed);
        let sender_peerid = root_signing_key.verifying_key().to_bytes();

        let app_signing_key = SigningKey::from_bytes(&app_seed);
        let app_public_key = app_signing_key.verifying_key().to_bytes();

        let mut owner_peerid = [0u8; 32];
        let mut recipient_peerid = [0u8; 32];
        let mut cert_id = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut owner_peerid);
        rand::thread_rng().fill_bytes(&mut recipient_peerid);
        rand::thread_rng().fill_bytes(&mut cert_id);

        let context_id = generate_context_id();
        let path = "/pub/paykit.app/v0/requests/abc123/req_006";
        let plaintext = b"Delegated signature message";

        let params = Sb2EncryptParams {
            recipient_inbox_pk: inbox_pk,
            owner_peerid,
            sender_peerid,
            recipient_peerid,
            context_id,
            canonical_path: path.to_string(),
            msg_id: "req_006".to_string(),
            purpose: Some("request".to_string()),
            expires_at: None,
        };

        // Create AppKeySigner
        let app_signer = AppKeySigner::new(app_seed, sender_peerid, cert_id);
        let encrypted = sb2_encrypt_signed(plaintext, &params, &app_signer).unwrap();

        // Create mock fetcher that returns the correct AppKey
        let fetcher = MockAppCertFetcher {
            expected_sender: sender_peerid,
            expected_cert_id: cert_id,
            app_public_key,
        };

        // Decrypt with required signature - should succeed with fetcher
        let (decrypted, metadata) = sb2_decrypt_verified(
            &encrypted,
            &inbox_sk,
            &owner_peerid,
            path,
            SignatureRequirement::Required,
            Some(&fetcher),
        )
        .unwrap();

        assert_eq!(decrypted, plaintext);
        assert!(metadata.signature_verified);
        assert_eq!(metadata.cert_id, Some(cert_id));
    }

    #[cfg(feature = "pubky")]
    #[test]
    fn test_sb2_delegated_signature_no_fetcher_fails() {
        use super::super::scope::generate_context_id;
        use pubky_noise::sealed_blob::x25519_generate_keypair;
        use ed25519_dalek::SigningKey;
        use rand::RngCore;

        // Generate keys
        let (inbox_sk, inbox_pk) = x25519_generate_keypair();
        let mut root_seed = [0u8; 32];
        let mut app_seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut root_seed);
        rand::thread_rng().fill_bytes(&mut app_seed);

        let root_signing_key = SigningKey::from_bytes(&root_seed);
        let sender_peerid = root_signing_key.verifying_key().to_bytes();

        let mut owner_peerid = [0u8; 32];
        let mut recipient_peerid = [0u8; 32];
        let mut cert_id = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut owner_peerid);
        rand::thread_rng().fill_bytes(&mut recipient_peerid);
        rand::thread_rng().fill_bytes(&mut cert_id);

        let context_id = generate_context_id();
        let path = "/pub/paykit.app/v0/requests/abc123/req_007";
        let plaintext = b"Delegated signature without fetcher";

        let params = Sb2EncryptParams {
            recipient_inbox_pk: inbox_pk,
            owner_peerid,
            sender_peerid,
            recipient_peerid,
            context_id,
            canonical_path: path.to_string(),
            msg_id: "req_007".to_string(),
            purpose: Some("request".to_string()),
            expires_at: None,
        };

        // Create AppKeySigner
        let app_signer = AppKeySigner::new(app_seed, sender_peerid, cert_id);
        let encrypted = sb2_encrypt_signed(plaintext, &params, &app_signer).unwrap();

        // Decrypt with required signature but NO fetcher - should fail
        let result = sb2_decrypt_verified(
            &encrypted,
            &inbox_sk,
            &owner_peerid,
            path,
            SignatureRequirement::Required,
            None, // No fetcher!
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("no AppCertFetcher provided") || err.contains("cert_id"));
    }

    #[cfg(feature = "pubky")]
    #[test]
    fn test_sb2_delegated_signature_wrong_key_fails() {
        use super::super::scope::generate_context_id;
        use pubky_noise::sealed_blob::x25519_generate_keypair;
        use ed25519_dalek::SigningKey;
        use rand::RngCore;

        // Generate keys
        let (inbox_sk, inbox_pk) = x25519_generate_keypair();
        let mut root_seed = [0u8; 32];
        let mut app_seed = [0u8; 32];
        let mut wrong_app_seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut root_seed);
        rand::thread_rng().fill_bytes(&mut app_seed);
        rand::thread_rng().fill_bytes(&mut wrong_app_seed);

        let root_signing_key = SigningKey::from_bytes(&root_seed);
        let sender_peerid = root_signing_key.verifying_key().to_bytes();

        // Wrong key - different from what we sign with
        let wrong_signing_key = SigningKey::from_bytes(&wrong_app_seed);
        let wrong_public_key = wrong_signing_key.verifying_key().to_bytes();

        let mut owner_peerid = [0u8; 32];
        let mut recipient_peerid = [0u8; 32];
        let mut cert_id = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut owner_peerid);
        rand::thread_rng().fill_bytes(&mut recipient_peerid);
        rand::thread_rng().fill_bytes(&mut cert_id);

        let context_id = generate_context_id();
        let path = "/pub/paykit.app/v0/requests/abc123/req_008";
        let plaintext = b"Delegated signature with wrong key";

        let params = Sb2EncryptParams {
            recipient_inbox_pk: inbox_pk,
            owner_peerid,
            sender_peerid,
            recipient_peerid,
            context_id,
            canonical_path: path.to_string(),
            msg_id: "req_008".to_string(),
            purpose: Some("request".to_string()),
            expires_at: None,
        };

        // Create AppKeySigner with app_seed (correct key for signing)
        let app_signer = AppKeySigner::new(app_seed, sender_peerid, cert_id);
        let encrypted = sb2_encrypt_signed(plaintext, &params, &app_signer).unwrap();

        // Create mock fetcher that returns WRONG key
        let fetcher = MockAppCertFetcher {
            expected_sender: sender_peerid,
            expected_cert_id: cert_id,
            app_public_key: wrong_public_key, // Wrong key!
        };

        // Decrypt with required signature - should fail due to wrong key
        let result = sb2_decrypt_verified(
            &encrypted,
            &inbox_sk,
            &owner_peerid,
            path,
            SignatureRequirement::Required,
            Some(&fetcher),
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Signature verification failed"));
    }
}
