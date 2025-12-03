//! Identity management for Paykit demos
//!
//! Handles Ed25519 keypairs, X25519 key derivation for Noise, and Pubky URI generation.
//!
//! # Security Considerations
//!
//! **WARNING**: This is demo code for development and testing purposes.
//!
//! ## Key Storage
//! - Private keys are stored in **plaintext JSON files** on disk
//! - This is INSECURE for production use
//! - Keys are NOT encrypted at rest
//! - Keys are NOT protected by OS keychain/keyring
//!
//! ## Production Requirements
//! For production applications:
//! - Use OS-specific secure storage (Keychain on macOS/iOS, KeyStore on Android, Windows Credential Manager)
//! - Encrypt keys at rest with user-derived keys
//! - Consider hardware security modules (HSM) or secure enclaves
//! - Implement key rotation policies
//! - Use memory-safe zeroization for key material
//!
//! ## X25519 Key Derivation
//! Uses `pubky_noise::kdf::derive_x25519_static` which implements:
//! - HKDF-based derivation from Ed25519 seed
//! - Device-specific binding (prevents key reuse across devices)
//! - Deterministic output (same inputs → same key)

use anyhow::{Context, Result};
use ed25519_dalek::SigningKey;
use pubky::{Keypair, PublicKey};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents a user's identity with keypairs and metadata
///
/// # Security Note
/// This struct does NOT automatically zeroize secret key material on drop.
/// For production use, implement proper key zeroization using the `zeroize` crate.
#[derive(Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Ed25519 keypair for signing and identity
    #[serde(
        serialize_with = "serialize_keypair",
        deserialize_with = "deserialize_keypair"
    )]
    pub keypair: Keypair,
    /// Human-readable nickname
    pub nickname: Option<String>,
}

// Custom serialization for Keypair
fn serialize_keypair<S>(keypair: &Keypair, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    keypair.secret_key().serialize(serializer)
}

// Custom deserialization for Keypair
fn deserialize_keypair<'de, D>(deserializer: D) -> Result<Keypair, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let secret: [u8; 32] = <[u8; 32]>::deserialize(deserializer)?;
    Ok(Keypair::from_secret_key(&secret))
}

impl Identity {
    /// Generate a new random identity
    pub fn generate() -> Self {
        Self {
            keypair: Keypair::random(),
            nickname: None,
        }
    }

    /// Create identity from existing keypair
    pub fn from_keypair(keypair: Keypair) -> Self {
        Self {
            keypair,
            nickname: None,
        }
    }

    /// Set a nickname for this identity
    pub fn with_nickname(mut self, nickname: impl Into<String>) -> Self {
        self.nickname = Some(nickname.into());
        self
    }

    /// Get the public key
    pub fn public_key(&self) -> PublicKey {
        self.keypair.public_key()
    }

    /// Get the Pubky URI for this identity
    pub fn pubky_uri(&self) -> String {
        format!("pubky://{}", self.public_key())
    }

    /// Derive X25519 key for Noise protocol from Ed25519 keypair
    ///
    /// Uses HKDF-based key derivation to convert the Ed25519 secret key into
    /// an X25519 key suitable for Noise protocol encryption.
    ///
    /// # Arguments
    /// * `device_id` - Unique device identifier (binds key to specific device)
    ///
    /// # Security Properties
    /// - Deterministic: same inputs always produce same output
    /// - Device-bound: different devices produce different keys
    /// - One-way: cannot recover Ed25519 key from X25519 key
    ///
    /// # Example
    /// ```
    /// # use paykit_demo_core::Identity;
    /// let identity = Identity::generate();
    /// let x25519_key = identity.derive_x25519_key(b"device-001");
    /// assert_eq!(x25519_key.len(), 32);
    /// ```
    pub fn derive_x25519_key(&self, device_id: &[u8]) -> [u8; 32] {
        let seed = self.keypair.secret_key();
        // Uses pubky_noise KDF which implements HKDF-based derivation
        // with proper domain separation and security properties
        pubky_noise::kdf::derive_x25519_static(&seed, device_id)
    }
}

/// Manages identity persistence and loading
pub struct IdentityManager {
    storage_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct StoredIdentity {
    secret_key_hex: String,
    nickname: Option<String>,
}

impl IdentityManager {
    /// Create a new identity manager with the given storage directory
    pub fn new(storage_dir: impl AsRef<Path>) -> Self {
        Self {
            storage_dir: storage_dir.as_ref().to_path_buf(),
        }
    }

    /// Save an identity to disk
    ///
    /// # Security Warning
    /// This writes the private key to disk in **plaintext** (hex-encoded).
    /// The file is NOT encrypted. This is ONLY suitable for demo/development.
    /// For production, use OS-provided secure storage (Keychain, KeyStore, etc.).
    pub fn save(&self, identity: &Identity, name: &str) -> Result<()> {
        std::fs::create_dir_all(&self.storage_dir).context("Failed to create storage directory")?;

        let path = self.identity_path(name);

        // Convert secret key to hex for storage
        // WARNING: This stores the key in plaintext - insecure for production
        let secret_key_hex = hex::encode(identity.keypair.secret_key());

        let stored = StoredIdentity {
            secret_key_hex,
            nickname: identity.nickname.clone(),
        };

        let json = serde_json::to_string_pretty(&stored)?;
        std::fs::write(&path, json)
            .with_context(|| format!("Failed to write identity to {:?}", path))?;

        Ok(())
    }

    /// Load an identity from disk
    pub fn load(&self, name: &str) -> Result<Identity> {
        let path = self.identity_path(name);

        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read identity from {:?}", path))?;

        let stored: StoredIdentity = serde_json::from_str(&json)?;

        // Decode secret key from hex
        let secret_bytes = hex::decode(&stored.secret_key_hex)?;
        if secret_bytes.len() != 32 {
            anyhow::bail!("Invalid secret key length: expected 32 bytes");
        }

        let mut secret_key = [0u8; 32];
        secret_key.copy_from_slice(&secret_bytes);

        let _signing_key = SigningKey::from_bytes(&secret_key);
        let keypair = Keypair::from_secret_key(&secret_key);

        let mut identity = Identity::from_keypair(keypair);
        identity.nickname = stored.nickname;

        Ok(identity)
    }

    /// List all saved identities
    pub fn list(&self) -> Result<Vec<String>> {
        if !self.storage_dir.exists() {
            return Ok(Vec::new());
        }

        let mut names = Vec::new();
        for entry in std::fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "json" {
                        if let Some(stem) = path.file_stem() {
                            names.push(stem.to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }

        Ok(names)
    }

    /// Create and save a new identity
    pub fn create(&self, name: &str) -> Result<Identity> {
        let identity = Identity::generate().with_nickname(name);
        self.save(&identity, name)?;
        Ok(identity)
    }

    /// Delete an identity
    pub fn delete(&self, name: &str) -> Result<()> {
        let path = self.identity_path(name);
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to delete identity {:?}", path))?;
        Ok(())
    }

    fn identity_path(&self, name: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let identity = Identity::generate();
        assert!(identity.nickname.is_none());

        let uri = identity.pubky_uri();
        assert!(uri.starts_with("pubky://"));
    }

    #[test]
    fn test_identity_with_nickname() {
        let identity = Identity::generate().with_nickname("Alice");
        assert_eq!(identity.nickname, Some("Alice".to_string()));
    }

    #[test]
    fn test_x25519_derivation() {
        let identity = Identity::generate();
        let device_id = b"test_device";
        let key1 = identity.derive_x25519_key(device_id);
        let key2 = identity.derive_x25519_key(device_id);

        // Same inputs should produce same output
        assert_eq!(key1, key2);

        // Different device should produce different key
        let key3 = identity.derive_x25519_key(b"other_device");
        assert_ne!(key1, key3);
    }
}
