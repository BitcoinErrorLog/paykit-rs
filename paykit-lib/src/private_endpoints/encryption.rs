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
//! - **Memory Safety**: Keys are zeroized on drop using the `zeroize` crate
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
use zeroize::Zeroizing;

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
/// # Security
///
/// The master key is wrapped in `Zeroizing` to ensure it's securely
/// cleared from memory when the context is dropped.
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
pub struct EncryptionContext {
    /// Master key wrapped in Zeroizing for secure memory clearing on drop.
    master_key: Zeroizing<[u8; 32]>,
}

impl Clone for EncryptionContext {
    fn clone(&self) -> Self {
        Self {
            master_key: Zeroizing::new(*self.master_key),
        }
    }
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
    ///
    /// The key will be automatically zeroized when this context is dropped.
    pub fn new(master_key: [u8; 32]) -> Self {
        Self {
            master_key: Zeroizing::new(master_key),
        }
    }

    /// Derive a key for a specific context using HKDF.
    ///
    /// This ensures different files use different encryption keys,
    /// providing key separation even with the same master key.
    ///
    /// # Security
    ///
    /// The derived key is wrapped in `Zeroizing` to ensure secure cleanup.
    fn derive_key(&self, context: &[u8]) -> EncryptionResult<Zeroizing<[u8; 32]>> {
        let hk = Hkdf::<Sha256>::new(None, &*self.master_key);
        let mut key = Zeroizing::new([0u8; 32]);
        hk.expand(context, &mut *key)
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
    /// - Derived key is zeroized after use
    pub fn encrypt(&self, plaintext: &[u8], context: &[u8]) -> EncryptionResult<Vec<u8>> {
        // Derive a key specific to this context (wrapped in Zeroizing)
        let key = self.derive_key(context)?;

        // Create cipher using the derived key
        let cipher = Aes256Gcm::new_from_slice(&*key)
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

        // Key is automatically zeroized when `key` goes out of scope

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
    ///
    /// # Security
    ///
    /// Derived key is zeroized after use.
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

        // Derive key (wrapped in Zeroizing)
        let key = self.derive_key(context)?;

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&*key)
            .map_err(|e| EncryptionError::DecryptFailed(e.to_string()))?;

        // Decrypt
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, encrypted_data)
            .map_err(|_| EncryptionError::DecryptFailed("Authentication failed".to_string()))?;

        // Key is automatically zeroized when `key` goes out of scope

        Ok(plaintext)
    }

    /// Check if data appears to be encrypted (starts with valid version byte).
    ///
    /// This is a heuristic check and doesn't guarantee the data is valid ciphertext.
    pub fn is_encrypted(data: &[u8]) -> bool {
        !data.is_empty() && data[0] == ENCRYPTION_VERSION
    }
}

// Note: Zeroizing<T> automatically zeroizes on drop, so no manual Drop impl needed

/// Generate a random 256-bit encryption key.
///
/// # Security
///
/// - Uses the system's cryptographically secure random number generator.
/// - Returns a `Zeroizing` wrapper that clears the key from memory on drop.
pub fn generate_key() -> Zeroizing<[u8; 32]> {
    let mut key = Zeroizing::new([0u8; 32]);
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut *key);
    key
}

/// Derive an encryption key from a passphrase using HKDF.
///
/// # Arguments
///
/// * `passphrase` - User-provided passphrase
/// * `salt` - Application-specific salt (should be unique per application)
///
/// # Returns
///
/// A `Zeroizing`-wrapped key that will be cleared from memory on drop.
///
/// # Security
///
/// This uses HKDF which is fast but provides minimal protection against
/// brute-force attacks on weak passphrases. For better security with
/// user-provided passphrases, consider using `derive_key_from_passphrase_argon2`
/// which uses Argon2id for memory-hard key derivation.
pub fn derive_key_from_passphrase(
    passphrase: &[u8],
    salt: &[u8],
) -> EncryptionResult<Zeroizing<[u8; 32]>> {
    let hk = Hkdf::<Sha256>::new(Some(salt), passphrase);
    let mut key = Zeroizing::new([0u8; 32]);
    hk.expand(b"paykit-private-endpoints-v1", &mut *key)
        .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;
    Ok(key)
}

/// Argon2 parameters for key derivation.
///
/// These parameters balance security with usability on mobile devices.
#[derive(Debug, Clone, Copy)]
pub struct Argon2Params {
    /// Memory cost in KiB (default: 65536 = 64 MB)
    pub memory_kib: u32,
    /// Time cost (iterations, default: 3)
    pub iterations: u32,
    /// Parallelism (threads, default: 4)
    pub parallelism: u32,
}

impl Default for Argon2Params {
    /// Default parameters following OWASP recommendations for high-security.
    ///
    /// - Memory: 64 MB (provides strong protection against GPU attacks)
    /// - Iterations: 3 (balances security and usability)
    /// - Parallelism: 4 (utilizes multi-core processors)
    fn default() -> Self {
        Self {
            memory_kib: 65536, // 64 MB
            iterations: 3,
            parallelism: 4,
        }
    }
}

impl Argon2Params {
    /// Mobile-optimized parameters for constrained devices.
    ///
    /// Uses less memory but more iterations to compensate.
    /// - Memory: 19 MB (suitable for mobile devices)
    /// - Iterations: 4
    /// - Parallelism: 2
    pub fn mobile() -> Self {
        Self {
            memory_kib: 19456, // ~19 MB
            iterations: 4,
            parallelism: 2,
        }
    }

    /// Low-memory parameters for very constrained environments.
    ///
    /// - Memory: 4 MB
    /// - Iterations: 6
    /// - Parallelism: 1
    ///
    /// Note: Lower security than default, use only when necessary.
    pub fn low_memory() -> Self {
        Self {
            memory_kib: 4096, // 4 MB
            iterations: 6,
            parallelism: 1,
        }
    }
}

/// Derive an encryption key from a passphrase using Argon2id.
///
/// Argon2id is the recommended algorithm for password hashing and key derivation.
/// It provides strong protection against:
/// - GPU/ASIC brute-force attacks (memory-hard)
/// - Side-channel attacks (hybrid mode)
/// - Time-memory tradeoff attacks
///
/// # Arguments
///
/// * `passphrase` - User-provided passphrase
/// * `salt` - Application-specific salt (should be unique per user and at least 16 bytes)
/// * `params` - Argon2 parameters (use `None` for defaults)
///
/// # Returns
///
/// A `Zeroizing`-wrapped 256-bit key that will be cleared from memory on drop.
///
/// # Example
///
/// ```ignore
/// use paykit_lib::private_endpoints::encryption::{
///     derive_key_from_passphrase_argon2, Argon2Params
/// };
///
/// // Using default parameters (recommended for desktop)
/// let key = derive_key_from_passphrase_argon2(
///     b"user_passphrase",
///     b"unique_16_byte_salt!",
///     None,
/// )?;
///
/// // Using mobile-optimized parameters
/// let key = derive_key_from_passphrase_argon2(
///     b"user_passphrase",
///     b"unique_16_byte_salt!",
///     Some(Argon2Params::mobile()),
/// )?;
/// ```
///
/// # Security Notes
///
/// - Salt should be at least 16 bytes and unique per user
/// - Store the salt alongside the encrypted data (it doesn't need to be secret)
/// - For new applications, prefer Argon2id over PBKDF2 or bcrypt
/// - Default parameters provide ~1 second derivation time on modern hardware
pub fn derive_key_from_passphrase_argon2(
    passphrase: &[u8],
    salt: &[u8],
    params: Option<Argon2Params>,
) -> EncryptionResult<Zeroizing<[u8; 32]>> {
    use argon2::{Algorithm, Argon2, Params, Version};

    let p = params.unwrap_or_default();

    // Build Argon2 parameters
    let argon2_params = Params::new(
        p.memory_kib,
        p.iterations,
        p.parallelism,
        Some(32), // Output length
    )
    .map_err(|e| EncryptionError::KeyDerivation(format!("Invalid Argon2 parameters: {}", e)))?;

    // Create Argon2id instance
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params);

    // Derive key
    let mut key = Zeroizing::new([0u8; 32]);
    argon2
        .hash_password_into(passphrase, salt, &mut *key)
        .map_err(|e| EncryptionError::KeyDerivation(format!("Argon2 derivation failed: {}", e)))?;

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
        assert_eq!(
            ciphertext.len(),
            1 + NONCE_SIZE + plaintext.len() + TAG_SIZE
        );
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
        assert!(matches!(
            result,
            Err(EncryptionError::UnsupportedVersion(99))
        ));
    }

    #[test]
    fn test_generate_key_is_random() {
        let key1 = generate_key();
        let key2 = generate_key();

        // Two random keys should be different
        assert_ne!(*key1, *key2);
    }

    #[test]
    fn test_derive_key_from_passphrase() {
        let key1 = derive_key_from_passphrase(b"password123", b"salt1").unwrap();
        let key2 = derive_key_from_passphrase(b"password123", b"salt1").unwrap();
        let key3 = derive_key_from_passphrase(b"password123", b"salt2").unwrap();
        let key4 = derive_key_from_passphrase(b"different", b"salt1").unwrap();

        // Same inputs = same key
        assert_eq!(*key1, *key2);
        // Different salt = different key
        assert_ne!(*key1, *key3);
        // Different passphrase = different key
        assert_ne!(*key1, *key4);
    }

    #[test]
    fn test_zeroizing_key_creation() {
        // Test that keys are created with Zeroizing wrapper
        let key = generate_key();
        assert_eq!(key.len(), 32);

        // Create context from the key
        let ctx = EncryptionContext::new(*key);
        let plaintext = b"test";
        let context = b"ctx";

        // Verify encryption still works
        let ciphertext = ctx.encrypt(plaintext, context).unwrap();
        let decrypted = ctx.decrypt(&ciphertext, context).unwrap();
        assert_eq!(decrypted, plaintext);
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

    // ========================================================================
    // Argon2 Tests
    // ========================================================================

    #[test]
    fn test_argon2_default_params() {
        let params = Argon2Params::default();
        assert_eq!(params.memory_kib, 65536);
        assert_eq!(params.iterations, 3);
        assert_eq!(params.parallelism, 4);
    }

    #[test]
    fn test_argon2_mobile_params() {
        let params = Argon2Params::mobile();
        assert_eq!(params.memory_kib, 19456);
        assert_eq!(params.iterations, 4);
        assert_eq!(params.parallelism, 2);
    }

    #[test]
    fn test_argon2_low_memory_params() {
        let params = Argon2Params::low_memory();
        assert_eq!(params.memory_kib, 4096);
        assert_eq!(params.iterations, 6);
        assert_eq!(params.parallelism, 1);
    }

    #[test]
    fn test_derive_key_from_passphrase_argon2() {
        // Use low memory params for faster tests
        let params = Some(Argon2Params::low_memory());

        let key1 =
            derive_key_from_passphrase_argon2(b"password123", b"unique_salt_16b!", params).unwrap();
        let key2 =
            derive_key_from_passphrase_argon2(b"password123", b"unique_salt_16b!", params).unwrap();
        let key3 =
            derive_key_from_passphrase_argon2(b"password123", b"different_salt!!", params).unwrap();
        let key4 = derive_key_from_passphrase_argon2(b"different_pw", b"unique_salt_16b!", params)
            .unwrap();

        // Same inputs = same key
        assert_eq!(*key1, *key2);
        // Different salt = different key
        assert_ne!(*key1, *key3);
        // Different passphrase = different key
        assert_ne!(*key1, *key4);
    }

    #[test]
    fn test_argon2_produces_different_key_than_hkdf() {
        let passphrase = b"test_password";
        let salt = b"test_salt_16bytes";

        let hkdf_key = derive_key_from_passphrase(passphrase, salt).unwrap();
        let argon2_key =
            derive_key_from_passphrase_argon2(passphrase, salt, Some(Argon2Params::low_memory()))
                .unwrap();

        // Keys should be different (different algorithms)
        assert_ne!(*hkdf_key, *argon2_key);
    }

    #[test]
    fn test_argon2_key_works_with_encryption() {
        let key = derive_key_from_passphrase_argon2(
            b"passphrase",
            b"salt_must_be_16+",
            Some(Argon2Params::low_memory()),
        )
        .unwrap();

        let ctx = EncryptionContext::new(*key);
        let plaintext = b"secret data protected by Argon2-derived key";
        let context = b"test_context";

        let ciphertext = ctx.encrypt(plaintext, context).unwrap();
        let decrypted = ctx.decrypt(&ciphertext, context).unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
