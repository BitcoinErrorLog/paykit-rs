//! AES-256-GCM Encryption for Private Endpoint Storage
//!
//! This module provides authenticated encryption for private endpoint data
//! using AES-256-GCM with HKDF for key derivation.
//!
//! # Security Properties
//!
//! - **Confidentiality**: AES-256 encryption prevents unauthorized access
//! - **Integrity**: GCM authentication tag detects tampering
//! - **Key Derivation**: HKDF-SHA256 derives per-file keys from master key
//! - **Unique Nonces**: Random 96-bit nonces prevent nonce reuse
//!
//! # Wire Format
//!
//! ```text
//! [1 byte version][12 bytes nonce][N bytes ciphertext][16 bytes auth tag]
//! ```
//!
//! Version 1 uses AES-256-GCM with random nonces.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use hkdf::Hkdf;
use sha2::Sha256;

/// Current encryption format version.
const ENCRYPTION_VERSION: u8 = 1;

/// Size of the nonce in bytes (96 bits for GCM).
const NONCE_SIZE: usize = 12;

/// Size of the authentication tag in bytes.
const TAG_SIZE: usize = 16;

/// Encryption error types.
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptFailed(String),
    #[error("Invalid ciphertext format")]
    InvalidFormat,
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u8),
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),
}

/// Result type for encryption operations.
pub type EncryptionResult<T> = Result<T, EncryptionError>;

/// Encryption context for private endpoint storage.
///
/// Uses AES-256-GCM for authenticated encryption with HKDF for
/// deriving per-purpose keys from a master key.
///
/// # Example
///
/// ```ignore
/// use paykit_lib::private_endpoints::encryption::EncryptionContext;
///
/// // Create from a 32-byte master key
/// let master_key = [0u8; 32]; // In practice, use a securely generated key
/// let ctx = EncryptionContext::new(master_key);
///
/// // Encrypt data
/// let plaintext = b"secret endpoint data";
/// let ciphertext = ctx.encrypt(plaintext, b"peer:method")?;
///
/// // Decrypt data
/// let decrypted = ctx.decrypt(&ciphertext, b"peer:method")?;
/// assert_eq!(decrypted, plaintext);
/// ```
#[derive(Clone)]
pub struct EncryptionContext {
    master_key: [u8; 32],
}

impl EncryptionContext {
    /// Create a new encryption context from a master key.
    ///
    /// # Arguments
    ///
    /// * `master_key` - A 256-bit (32-byte) master key for encryption.
    ///
    /// # Security
    ///
    /// The master key should be:
    /// - Generated from a cryptographically secure random source
    /// - Stored securely (e.g., OS keychain, HSM)
    /// - Never hardcoded or logged
    pub fn new(master_key: [u8; 32]) -> Self {
        Self { master_key }
    }

    /// Derive a key for a specific context using HKDF.
    ///
    /// This ensures different files use different encryption keys,
    /// providing key separation even with the same master key.
    fn derive_key(&self, context: &[u8]) -> EncryptionResult<[u8; 32]> {
        let hk = Hkdf::<Sha256>::new(None, &self.master_key);
        let mut key = [0u8; 32];
        hk.expand(context, &mut key)
            .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;
        Ok(key)
    }

    /// Encrypt plaintext with associated data for context binding.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The data to encrypt
    /// * `context` - Context data used for key derivation (e.g., "peer:method")
    ///
    /// # Returns
    ///
    /// Encrypted data in wire format: `[version][nonce][ciphertext+tag]`
    ///
    /// # Security
    ///
    /// - Uses a random 96-bit nonce for each encryption
    /// - Context is used for key derivation, not as AAD
    /// - Authentication tag prevents tampering
    pub fn encrypt(&self, plaintext: &[u8], context: &[u8]) -> EncryptionResult<Vec<u8>> {
        // Derive a key specific to this context
        let key = self.derive_key(context)?;

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| EncryptionError::EncryptFailed(e.to_string()))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| EncryptionError::EncryptFailed(e.to_string()))?;

        // Build wire format: [version][nonce][ciphertext]
        let mut result = Vec::with_capacity(1 + NONCE_SIZE + ciphertext.len());
        result.push(ENCRYPTION_VERSION);
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt ciphertext with context binding.
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - The encrypted data in wire format
    /// * `context` - Context data used for key derivation (must match encryption)
    ///
    /// # Returns
    ///
    /// The decrypted plaintext.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The ciphertext is too short or malformed
    /// - The version is unsupported
    /// - Decryption or authentication fails
    /// - The context doesn't match what was used for encryption
    pub fn decrypt(&self, ciphertext: &[u8], context: &[u8]) -> EncryptionResult<Vec<u8>> {
        // Minimum length: version (1) + nonce (12) + tag (16) = 29 bytes
        let min_len = 1 + NONCE_SIZE + TAG_SIZE;
        if ciphertext.len() < min_len {
            return Err(EncryptionError::InvalidFormat);
        }

        // Parse version
        let version = ciphertext[0];
        if version != ENCRYPTION_VERSION {
            return Err(EncryptionError::UnsupportedVersion(version));
        }

        // Extract nonce and ciphertext
        let nonce_bytes = &ciphertext[1..1 + NONCE_SIZE];
        let encrypted_data = &ciphertext[1 + NONCE_SIZE..];

        // Derive key
        let key = self.derive_key(context)?;

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| EncryptionError::DecryptFailed(e.to_string()))?;

        // Decrypt
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, encrypted_data)
            .map_err(|_| EncryptionError::DecryptFailed("Authentication failed".to_string()))?;

        Ok(plaintext)
    }

    /// Check if data appears to be encrypted (starts with valid version byte).
    ///
    /// This is a heuristic check and doesn't guarantee the data is valid ciphertext.
    pub fn is_encrypted(data: &[u8]) -> bool {
        !data.is_empty() && data[0] == ENCRYPTION_VERSION
    }

    /// Zeroize the master key when the context is dropped.
    ///
    /// Note: For production use, consider using the `zeroize` crate
    /// for more robust memory clearing.
    pub fn zeroize(&mut self) {
        self.master_key.iter_mut().for_each(|b| *b = 0);
    }
}

impl Drop for EncryptionContext {
    fn drop(&mut self) {
        // Clear the key from memory
        self.zeroize();
    }
}

/// Generate a random 256-bit encryption key.
///
/// # Security
///
/// Uses the system's cryptographically secure random number generator.
pub fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut key);
    key
}

/// Derive an encryption key from a passphrase using HKDF.
///
/// # Arguments
///
/// * `passphrase` - User-provided passphrase
/// * `salt` - Application-specific salt (should be unique per application)
///
/// # Security
///
/// For better security with weak passphrases, consider using a
/// password-based KDF like Argon2 or scrypt instead.
pub fn derive_key_from_passphrase(passphrase: &[u8], salt: &[u8]) -> EncryptionResult<[u8; 32]> {
    let hk = Hkdf::<Sha256>::new(Some(salt), passphrase);
    let mut key = [0u8; 32];
    hk.expand(b"paykit-private-endpoints-v1", &mut key)
        .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        for (i, b) in key.iter_mut().enumerate() {
            *b = i as u8;
        }
        key
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let ctx = EncryptionContext::new(test_key());
        let plaintext = b"Hello, World! This is secret data.";
        let context = b"peer:method";

        let ciphertext = ctx.encrypt(plaintext, context).unwrap();
        let decrypted = ctx.decrypt(&ciphertext, context).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_ciphertext_format() {
        let ctx = EncryptionContext::new(test_key());
        let plaintext = b"test";
        let context = b"ctx";

        let ciphertext = ctx.encrypt(plaintext, context).unwrap();

        // Check format: version (1) + nonce (12) + plaintext (4) + tag (16) = 33
        assert_eq!(ciphertext.len(), 1 + NONCE_SIZE + plaintext.len() + TAG_SIZE);
        assert_eq!(ciphertext[0], ENCRYPTION_VERSION);
    }

    #[test]
    fn test_different_contexts_different_ciphertext() {
        let ctx = EncryptionContext::new(test_key());
        let plaintext = b"same data";

        let ct1 = ctx.encrypt(plaintext, b"context1").unwrap();
        let ct2 = ctx.encrypt(plaintext, b"context2").unwrap();

        // Different contexts should produce different ciphertext
        // (also different due to random nonces, but this tests key derivation)
        assert_ne!(ct1, ct2);
    }

    #[test]
    fn test_wrong_context_fails() {
        let ctx = EncryptionContext::new(test_key());
        let plaintext = b"secret";

        let ciphertext = ctx.encrypt(plaintext, b"correct_context").unwrap();
        let result = ctx.decrypt(&ciphertext, b"wrong_context");

        assert!(result.is_err());
    }

    #[test]
    fn test_tampering_detected() {
        let ctx = EncryptionContext::new(test_key());
        let plaintext = b"secret";
        let context = b"ctx";

        let mut ciphertext = ctx.encrypt(plaintext, context).unwrap();

        // Tamper with the ciphertext
        let last_idx = ciphertext.len() - 1;
        ciphertext[last_idx] ^= 1;

        let result = ctx.decrypt(&ciphertext, context);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_format_rejected() {
        let ctx = EncryptionContext::new(test_key());

        // Too short
        let result = ctx.decrypt(&[1, 2, 3], b"ctx");
        assert!(matches!(result, Err(EncryptionError::InvalidFormat)));

        // Wrong version
        let mut bad_version = vec![99u8]; // Invalid version
        bad_version.extend_from_slice(&[0u8; 28]); // Padding
        let result = ctx.decrypt(&bad_version, b"ctx");
        assert!(matches!(result, Err(EncryptionError::UnsupportedVersion(99))));
    }

    #[test]
    fn test_generate_key_is_random() {
        let key1 = generate_key();
        let key2 = generate_key();

        // Two random keys should be different
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_key_from_passphrase() {
        let key1 = derive_key_from_passphrase(b"password123", b"salt1").unwrap();
        let key2 = derive_key_from_passphrase(b"password123", b"salt1").unwrap();
        let key3 = derive_key_from_passphrase(b"password123", b"salt2").unwrap();
        let key4 = derive_key_from_passphrase(b"different", b"salt1").unwrap();

        // Same inputs = same key
        assert_eq!(key1, key2);
        // Different salt = different key
        assert_ne!(key1, key3);
        // Different passphrase = different key
        assert_ne!(key1, key4);
    }

    #[test]
    fn test_is_encrypted() {
        let ctx = EncryptionContext::new(test_key());
        let ciphertext = ctx.encrypt(b"test", b"ctx").unwrap();

        assert!(EncryptionContext::is_encrypted(&ciphertext));
        assert!(!EncryptionContext::is_encrypted(b"plain text"));
        assert!(!EncryptionContext::is_encrypted(&[]));
    }

    #[test]
    fn test_empty_plaintext() {
        let ctx = EncryptionContext::new(test_key());
        let plaintext = b"";
        let context = b"ctx";

        let ciphertext = ctx.encrypt(plaintext, context).unwrap();
        let decrypted = ctx.decrypt(&ciphertext, context).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_large_plaintext() {
        let ctx = EncryptionContext::new(test_key());
        let plaintext = vec![0x42u8; 1024 * 1024]; // 1 MB
        let context = b"ctx";

        let ciphertext = ctx.encrypt(&plaintext, context).unwrap();
        let decrypted = ctx.decrypt(&ciphertext, context).unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
