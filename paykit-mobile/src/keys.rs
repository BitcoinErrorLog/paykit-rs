//! Key management for Paykit Mobile
//!
//! This module provides FFI-safe key generation, derivation, and management
//! for pkarr (Ed25519) identity keys and pubky-noise (X25519) device keys.
//!
//! # Key Architecture
//!
//! - **Ed25519 Identity Key**: The user's pkarr identity, used for signing and
//!   as the source of truth for identity. The 32-byte secret key (seed) is the
//!   root secret that must be backed up.
//!
//! - **X25519 Device Keys**: Derived deterministically from the Ed25519 seed
//!   using HKDF with device_id and epoch parameters. Used for Noise protocol
//!   encrypted channels.
//!
//! # Security
//!
//! - Secret keys should be stored in platform-secure storage (Keychain/EncryptedSharedPreferences)
//! - Keys are zeroized from memory after use where possible
//! - Export uses encryption to protect backup data

use crate::{PaykitMobileError, Result};

/// Generated Ed25519 keypair for identity.
#[derive(Clone, uniffi::Record)]
pub struct Ed25519Keypair {
    /// Secret key (seed) - 32 bytes, hex encoded.
    /// SENSITIVE: Store securely, this is the root identity secret.
    pub secret_key_hex: String,
    /// Public key - 32 bytes, hex encoded.
    pub public_key_hex: String,
    /// Public key in z-base32 format (pkarr format).
    pub public_key_z32: String,
}

/// Derived X25519 keypair for Noise protocol.
#[derive(Clone, uniffi::Record)]
pub struct X25519Keypair {
    /// Secret key - 32 bytes, hex encoded.
    pub secret_key_hex: String,
    /// Public key - 32 bytes, hex encoded.
    pub public_key_hex: String,
    /// Device ID used for derivation.
    pub device_id: String,
    /// Epoch used for derivation.
    pub epoch: u32,
}

/// Encrypted key backup for export/import.
#[derive(Clone, uniffi::Record)]
pub struct KeyBackup {
    /// Version of the backup format.
    pub version: u32,
    /// Encrypted secret key (AES-GCM).
    pub encrypted_data_hex: String,
    /// Salt for key derivation from password.
    pub salt_hex: String,
    /// Nonce for AES-GCM.
    pub nonce_hex: String,
    /// Public key (not encrypted, for identification).
    pub public_key_z32: String,
}

/// Generate a new Ed25519 keypair for identity.
///
/// This creates a new random identity. The secret key should be stored
/// securely and backed up.
///
/// # Returns
///
/// A new Ed25519 keypair with the secret in hex format.
#[uniffi::export]
pub fn generate_ed25519_keypair() -> Result<Ed25519Keypair> {
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let secret_bytes = signing_key.to_bytes();
    let public_bytes = verifying_key.to_bytes();

    Ok(Ed25519Keypair {
        secret_key_hex: hex::encode(secret_bytes),
        public_key_hex: hex::encode(public_bytes),
        public_key_z32: z32_encode(&public_bytes),
    })
}

/// Derive Ed25519 public key from secret key.
///
/// # Arguments
///
/// * `secret_key_hex` - The 32-byte secret key in hex format.
///
/// # Returns
///
/// The complete keypair derived from the secret.
#[uniffi::export]
pub fn ed25519_keypair_from_secret(secret_key_hex: String) -> Result<Ed25519Keypair> {
    use ed25519_dalek::SigningKey;

    let secret_bytes = hex::decode(&secret_key_hex).map_err(|e| PaykitMobileError::Validation {
        msg: format!("Invalid hex: {}", e),
    })?;

    if secret_bytes.len() != 32 {
        return Err(PaykitMobileError::Validation {
            msg: format!("Secret key must be 32 bytes, got {}", secret_bytes.len()),
        });
    }

    let mut secret_arr = [0u8; 32];
    secret_arr.copy_from_slice(&secret_bytes);

    let signing_key = SigningKey::from_bytes(&secret_arr);
    let verifying_key = signing_key.verifying_key();
    let public_bytes = verifying_key.to_bytes();

    Ok(Ed25519Keypair {
        secret_key_hex,
        public_key_hex: hex::encode(public_bytes),
        public_key_z32: z32_encode(&public_bytes),
    })
}

/// Derive X25519 keypair for Noise protocol from Ed25519 seed.
///
/// This uses the pubky-noise KDF to derive device-specific encryption keys
/// from the Ed25519 identity seed.
///
/// # Arguments
///
/// * `ed25519_secret_hex` - The Ed25519 secret key (seed) in hex format.
/// * `device_id` - A unique identifier for this device.
/// * `epoch` - Key rotation epoch (increment to rotate keys).
///
/// # Returns
///
/// The derived X25519 keypair for use with Noise protocol.
#[uniffi::export]
pub fn derive_x25519_keypair(
    ed25519_secret_hex: String,
    device_id: String,
    epoch: u32,
) -> Result<X25519Keypair> {
    let seed = hex_to_32_bytes(&ed25519_secret_hex)?;
    let device_id_bytes = device_id.as_bytes();

    // Use pubky-noise KDF for derivation
    let x25519_secret = derive_x25519_for_device_epoch(&seed, device_id_bytes, epoch);
    let x25519_public = x25519_pk_from_sk(&x25519_secret);

    Ok(X25519Keypair {
        secret_key_hex: hex::encode(x25519_secret),
        public_key_hex: hex::encode(x25519_public),
        device_id,
        epoch,
    })
}

/// Sign a message with Ed25519 secret key.
///
/// # Arguments
///
/// * `secret_key_hex` - The Ed25519 secret key in hex format.
/// * `message` - The message bytes to sign.
///
/// # Returns
///
/// The 64-byte signature in hex format.
#[uniffi::export]
pub fn sign_message(secret_key_hex: String, message: Vec<u8>) -> Result<String> {
    use ed25519_dalek::{Signer, SigningKey};

    let secret_bytes = hex_to_32_bytes(&secret_key_hex)?;
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let signature = signing_key.sign(&message);

    Ok(hex::encode(signature.to_bytes()))
}

/// Verify an Ed25519 signature.
///
/// # Arguments
///
/// * `public_key_hex` - The Ed25519 public key in hex format.
/// * `message` - The original message bytes.
/// * `signature_hex` - The 64-byte signature in hex format.
///
/// # Returns
///
/// True if the signature is valid, false otherwise.
#[uniffi::export]
pub fn verify_signature(
    public_key_hex: String,
    message: Vec<u8>,
    signature_hex: String,
) -> Result<bool> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let public_bytes = hex_to_32_bytes(&public_key_hex)?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_bytes).map_err(|e| PaykitMobileError::Validation {
            msg: format!("Invalid public key: {}", e),
        })?;

    let sig_bytes = hex::decode(&signature_hex).map_err(|e| PaykitMobileError::Validation {
        msg: format!("Invalid signature hex: {}", e),
    })?;

    if sig_bytes.len() != 64 {
        return Err(PaykitMobileError::Validation {
            msg: format!("Signature must be 64 bytes, got {}", sig_bytes.len()),
        });
    }

    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);
    let signature = Signature::from_bytes(&sig_arr);

    Ok(verifying_key.verify(&message, &signature).is_ok())
}

/// Export keypair to encrypted backup.
///
/// # Arguments
///
/// * `secret_key_hex` - The secret key to backup.
/// * `password` - Password to encrypt the backup.
///
/// # Returns
///
/// Encrypted backup that can be stored or transferred.
#[uniffi::export]
pub fn export_keypair_to_backup(secret_key_hex: String, password: String) -> Result<KeyBackup> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    use argon2::Argon2;
    use rand::RngCore;

    let secret_bytes = hex_to_32_bytes(&secret_key_hex)?;

    // Derive encryption key from password using Argon2
    let mut salt = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut salt);

    let mut encryption_key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), &salt, &mut encryption_key)
        .map_err(|e| PaykitMobileError::Internal {
            msg: format!("Key derivation failed: {}", e),
        })?;

    // Encrypt with AES-GCM
    let cipher =
        Aes256Gcm::new_from_slice(&encryption_key).map_err(|e| PaykitMobileError::Internal {
            msg: format!("Cipher init failed: {}", e),
        })?;

    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let encrypted =
        cipher
            .encrypt(nonce, secret_bytes.as_ref())
            .map_err(|e| PaykitMobileError::Internal {
                msg: format!("Encryption failed: {}", e),
            })?;

    // Get public key for identification
    let keypair = ed25519_keypair_from_secret(secret_key_hex)?;

    Ok(KeyBackup {
        version: 1,
        encrypted_data_hex: hex::encode(encrypted),
        salt_hex: hex::encode(salt),
        nonce_hex: hex::encode(nonce_bytes),
        public_key_z32: keypair.public_key_z32,
    })
}

/// Import keypair from encrypted backup.
///
/// # Arguments
///
/// * `backup` - The encrypted backup.
/// * `password` - Password to decrypt the backup.
///
/// # Returns
///
/// The decrypted keypair.
#[uniffi::export]
pub fn import_keypair_from_backup(backup: KeyBackup, password: String) -> Result<Ed25519Keypair> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };
    use argon2::Argon2;

    if backup.version != 1 {
        return Err(PaykitMobileError::Validation {
            msg: format!("Unsupported backup version: {}", backup.version),
        });
    }

    let salt = hex::decode(&backup.salt_hex).map_err(|e| PaykitMobileError::Validation {
        msg: format!("Invalid salt: {}", e),
    })?;

    let nonce_bytes =
        hex::decode(&backup.nonce_hex).map_err(|e| PaykitMobileError::Validation {
            msg: format!("Invalid nonce: {}", e),
        })?;

    let encrypted =
        hex::decode(&backup.encrypted_data_hex).map_err(|e| PaykitMobileError::Validation {
            msg: format!("Invalid encrypted data: {}", e),
        })?;

    // Derive encryption key from password
    let mut encryption_key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), &salt, &mut encryption_key)
        .map_err(|e| PaykitMobileError::Internal {
            msg: format!("Key derivation failed: {}", e),
        })?;

    // Decrypt
    let cipher =
        Aes256Gcm::new_from_slice(&encryption_key).map_err(|e| PaykitMobileError::Internal {
            msg: format!("Cipher init failed: {}", e),
        })?;

    if nonce_bytes.len() != 12 {
        return Err(PaykitMobileError::Validation {
            msg: "Invalid nonce length".to_string(),
        });
    }
    let nonce = Nonce::from_slice(&nonce_bytes);

    let decrypted = cipher.decrypt(nonce, encrypted.as_ref()).map_err(|_| {
        PaykitMobileError::AuthenticationError {
            msg: "Invalid password or corrupted backup".to_string(),
        }
    })?;

    let secret_key_hex = hex::encode(&decrypted);
    let keypair = ed25519_keypair_from_secret(secret_key_hex)?;

    // Verify public key matches
    if keypair.public_key_z32 != backup.public_key_z32 {
        return Err(PaykitMobileError::Validation {
            msg: "Backup public key mismatch".to_string(),
        });
    }

    Ok(keypair)
}

/// Format public key as z-base32 (pkarr format).
#[uniffi::export]
pub fn format_public_key_z32(public_key_hex: String) -> Result<String> {
    let bytes = hex_to_32_bytes(&public_key_hex)?;
    Ok(z32_encode(&bytes))
}

/// Parse z-base32 public key to hex.
#[uniffi::export]
pub fn parse_public_key_z32(public_key_z32: String) -> Result<String> {
    let bytes = z32_decode(&public_key_z32)?;
    Ok(hex::encode(bytes))
}

/// Get the unique device ID for this device.
///
/// This should be stored persistently and reused for consistent key derivation.
/// If not available, generates a new random device ID.
#[uniffi::export]
pub fn generate_device_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

// ============================================================================
// Internal Helper Functions
// ============================================================================

fn hex_to_32_bytes(hex_str: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(hex_str).map_err(|e| PaykitMobileError::Validation {
        msg: format!("Invalid hex: {}", e),
    })?;

    if bytes.len() != 32 {
        return Err(PaykitMobileError::Validation {
            msg: format!("Expected 32 bytes, got {}", bytes.len()),
        });
    }

    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Z-base32 encoding (pkarr format).
fn z32_encode(bytes: &[u8; 32]) -> String {
    // z-base32 alphabet: ybndrfg8ejkmcpqxot1uwisza345h769
    const ALPHABET: &[u8] = b"ybndrfg8ejkmcpqxot1uwisza345h769";

    let mut result = String::with_capacity(52);
    let mut buffer: u64 = 0;
    let mut bits = 0;

    for &byte in bytes.iter() {
        buffer = (buffer << 8) | u64::from(byte);
        bits += 8;

        while bits >= 5 {
            bits -= 5;
            let index = ((buffer >> bits) & 0x1F) as usize;
            result.push(ALPHABET[index] as char);
        }
    }

    // Handle remaining bits
    if bits > 0 {
        let index = ((buffer << (5 - bits)) & 0x1F) as usize;
        result.push(ALPHABET[index] as char);
    }

    result
}

/// Z-base32 decoding (pkarr format).
fn z32_decode(s: &str) -> Result<[u8; 32]> {
    const ALPHABET: &[u8] = b"ybndrfg8ejkmcpqxot1uwisza345h769";

    let mut result = Vec::with_capacity(32);
    let mut buffer: u64 = 0;
    let mut bits = 0;

    for c in s.chars() {
        let value = ALPHABET.iter().position(|&x| x == c as u8).ok_or_else(|| {
            PaykitMobileError::Validation {
                msg: format!("Invalid z-base32 character: {}", c),
            }
        })? as u64;

        buffer = (buffer << 5) | value;
        bits += 5;

        if bits >= 8 {
            bits -= 8;
            result.push((buffer >> bits) as u8);
        }
    }

    if result.len() != 32 {
        return Err(PaykitMobileError::Validation {
            msg: format!(
                "Invalid z-base32 length: expected 32 bytes, got {}",
                result.len()
            ),
        });
    }

    let mut arr = [0u8; 32];
    arr.copy_from_slice(&result);
    Ok(arr)
}

// Re-implement the KDF functions from pubky-noise to avoid circular dependency
fn derive_x25519_for_device_epoch(seed: &[u8; 32], device_id: &[u8], epoch: u32) -> [u8; 32] {
    use hkdf::Hkdf;
    use sha2::Sha512;

    let salt = b"pubky-noise-x25519:v1";
    let hk = Hkdf::<Sha512>::new(Some(salt), seed);

    let mut info = Vec::with_capacity(device_id.len() + 4);
    info.extend_from_slice(device_id);
    info.extend_from_slice(&epoch.to_le_bytes());

    let mut sk = [0u8; 32];
    hk.expand(&info, &mut sk).expect("hkdf expand");

    // Clamp for X25519
    sk[0] &= 248;
    sk[31] &= 127;
    sk[31] |= 64;

    sk
}

fn x25519_pk_from_sk(sk: &[u8; 32]) -> [u8; 32] {
    use curve25519_dalek::constants::ED25519_BASEPOINT_TABLE;
    use curve25519_dalek::scalar::Scalar;

    let scalar = Scalar::from_bytes_mod_order(*sk);
    let point = &scalar * ED25519_BASEPOINT_TABLE;
    point.to_montgomery().to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ed25519_keypair() {
        let keypair = generate_ed25519_keypair().unwrap();

        assert_eq!(keypair.secret_key_hex.len(), 64); // 32 bytes hex
        assert_eq!(keypair.public_key_hex.len(), 64);
        assert!(!keypair.public_key_z32.is_empty());
    }

    #[test]
    fn test_ed25519_keypair_from_secret() {
        let keypair1 = generate_ed25519_keypair().unwrap();
        let keypair2 = ed25519_keypair_from_secret(keypair1.secret_key_hex.clone()).unwrap();

        assert_eq!(keypair1.public_key_hex, keypair2.public_key_hex);
        assert_eq!(keypair1.public_key_z32, keypair2.public_key_z32);
    }

    #[test]
    fn test_derive_x25519_keypair() {
        let ed_keypair = generate_ed25519_keypair().unwrap();
        let x_keypair = derive_x25519_keypair(
            ed_keypair.secret_key_hex.clone(),
            "test-device".to_string(),
            0,
        )
        .unwrap();

        assert_eq!(x_keypair.secret_key_hex.len(), 64);
        assert_eq!(x_keypair.public_key_hex.len(), 64);
        assert_eq!(x_keypair.device_id, "test-device");
        assert_eq!(x_keypair.epoch, 0);

        // Same inputs should produce same outputs
        let x_keypair2 =
            derive_x25519_keypair(ed_keypair.secret_key_hex, "test-device".to_string(), 0).unwrap();

        assert_eq!(x_keypair.secret_key_hex, x_keypair2.secret_key_hex);
        assert_eq!(x_keypair.public_key_hex, x_keypair2.public_key_hex);
    }

    #[test]
    fn test_different_epochs_produce_different_keys() {
        let ed_keypair = generate_ed25519_keypair().unwrap();

        let x_keypair0 = derive_x25519_keypair(
            ed_keypair.secret_key_hex.clone(),
            "test-device".to_string(),
            0,
        )
        .unwrap();

        let x_keypair1 =
            derive_x25519_keypair(ed_keypair.secret_key_hex, "test-device".to_string(), 1).unwrap();

        assert_ne!(x_keypair0.secret_key_hex, x_keypair1.secret_key_hex);
        assert_ne!(x_keypair0.public_key_hex, x_keypair1.public_key_hex);
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = generate_ed25519_keypair().unwrap();
        let message = b"Hello, World!".to_vec();

        let signature = sign_message(keypair.secret_key_hex, message.clone()).unwrap();
        let valid = verify_signature(keypair.public_key_hex, message, signature).unwrap();

        assert!(valid);
    }

    #[test]
    fn test_export_import_backup() {
        let keypair = generate_ed25519_keypair().unwrap();
        let password = "test-password-123".to_string();

        let backup =
            export_keypair_to_backup(keypair.secret_key_hex.clone(), password.clone()).unwrap();

        assert_eq!(backup.version, 1);
        assert_eq!(backup.public_key_z32, keypair.public_key_z32);

        let restored = import_keypair_from_backup(backup, password).unwrap();

        assert_eq!(restored.secret_key_hex, keypair.secret_key_hex);
        assert_eq!(restored.public_key_hex, keypair.public_key_hex);
        assert_eq!(restored.public_key_z32, keypair.public_key_z32);
    }

    #[test]
    fn test_wrong_password_fails() {
        let keypair = generate_ed25519_keypair().unwrap();
        let backup =
            export_keypair_to_backup(keypair.secret_key_hex, "correct".to_string()).unwrap();

        let result = import_keypair_from_backup(backup, "wrong".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_z32_roundtrip() {
        let keypair = generate_ed25519_keypair().unwrap();
        let z32 = format_public_key_z32(keypair.public_key_hex.clone()).unwrap();
        let hex = parse_public_key_z32(z32).unwrap();

        assert_eq!(hex, keypair.public_key_hex);
    }
}
