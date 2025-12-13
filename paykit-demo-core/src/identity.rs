//! Identity management for Paykit demos
//!
//! Handles Ed25519 keypairs, X25519 key derivation for Noise, and Pubky URI generation.

use anyhow::{Context, Result};
use ed25519_dalek::SigningKey;
use paykit_lib::prelude::{SecureKeyStorage, StoreOptions};
use pubky::{Keypair, PublicKey};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents a user's identity with keypairs and metadata
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

/// Encrypted backup of an identity
#[derive(Clone, Serialize, Deserialize)]
pub struct KeyBackup {
    /// Version of the backup format
    pub version: u32,
    /// Encrypted secret key data (hex encoded)
    pub encrypted_data_hex: String,
    /// Salt used for key derivation (hex encoded)
    pub salt_hex: String,
    /// Nonce used for encryption (hex encoded)
    pub nonce_hex: String,
    /// Public key for identification (z32 encoded)
    pub public_key_z32: String,
}

impl KeyBackup {
    /// Serialize backup to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize backup")
    }

    /// Deserialize backup from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse backup")
    }
}

impl Identity {
    /// Generate a new random identity
    pub fn generate() -> Self {
        Self {
            keypair: Keypair::random(),
            nickname: None,
        }
    }

    /// Export identity to encrypted backup
    ///
    /// Uses Argon2 for password-based key derivation and AES-256-GCM for encryption.
    pub fn export_backup(&self, password: &str) -> Result<KeyBackup> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        use argon2::Argon2;
        use rand::RngCore;

        let secret_bytes = self.keypair.secret_key();

        // Derive encryption key from password using Argon2
        let mut salt = [0u8; 16];
        rand::rngs::OsRng.fill_bytes(&mut salt);

        let mut encryption_key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), &salt, &mut encryption_key)
            .map_err(|e| anyhow::anyhow!("Key derivation failed: {}", e))?;

        // Encrypt with AES-GCM
        let cipher = Aes256Gcm::new_from_slice(&encryption_key)
            .map_err(|e| anyhow::anyhow!("Cipher init failed: {}", e))?;

        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let encrypted = cipher
            .encrypt(nonce, secret_bytes.as_ref())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        Ok(KeyBackup {
            version: 1,
            encrypted_data_hex: hex::encode(encrypted),
            salt_hex: hex::encode(salt),
            nonce_hex: hex::encode(nonce_bytes),
            public_key_z32: self.public_key().to_string(),
        })
    }

    /// Import identity from encrypted backup
    ///
    /// Uses Argon2 for password-based key derivation and AES-256-GCM for decryption.
    pub fn import_backup(backup: &KeyBackup, password: &str) -> Result<Self> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        use argon2::Argon2;

        if backup.version != 1 {
            anyhow::bail!("Unsupported backup version: {}", backup.version);
        }

        let salt = hex::decode(&backup.salt_hex).context("Invalid salt")?;
        let nonce_bytes = hex::decode(&backup.nonce_hex).context("Invalid nonce")?;
        let encrypted = hex::decode(&backup.encrypted_data_hex).context("Invalid encrypted data")?;

        // Derive encryption key from password
        let mut encryption_key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), &salt, &mut encryption_key)
            .map_err(|e| anyhow::anyhow!("Key derivation failed: {}", e))?;

        // Decrypt with AES-GCM
        let cipher = Aes256Gcm::new_from_slice(&encryption_key)
            .map_err(|e| anyhow::anyhow!("Cipher init failed: {}", e))?;

        if nonce_bytes.len() != 12 {
            anyhow::bail!("Invalid nonce length");
        }
        let nonce = Nonce::from_slice(&nonce_bytes);

        let decrypted = cipher
            .decrypt(nonce, encrypted.as_ref())
            .map_err(|_| anyhow::anyhow!("Decryption failed - wrong password or corrupted data"))?;

        if decrypted.len() != 32 {
            anyhow::bail!("Invalid decrypted data length");
        }

        let mut secret_key = [0u8; 32];
        secret_key.copy_from_slice(&decrypted);

        let keypair = Keypair::from_secret_key(&secret_key);

        // Verify public key matches
        let derived_public_key = keypair.public_key().to_string();
        if derived_public_key != backup.public_key_z32 {
            anyhow::bail!("Public key mismatch - backup may be corrupted");
        }

        Ok(Self {
            keypair,
            nickname: None,
        })
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
    pub fn derive_x25519_key(&self, device_id: &[u8], epoch: u32) -> [u8; 32] {
        let seed = self.keypair.secret_key();
        pubky_noise::kdf::derive_x25519_for_device_epoch(&seed, device_id, epoch)
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
    pub fn save(&self, identity: &Identity, name: &str) -> Result<()> {
        std::fs::create_dir_all(&self.storage_dir).context("Failed to create storage directory")?;

        let path = self.identity_path(name);

        // Convert secret key to hex
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

    /// Delete an identity
    pub fn delete(&self, name: &str) -> Result<()> {
        let path = self.identity_path(name);
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to delete identity {:?}", path))?;
        Ok(())
    }

    /// Create a new identity (generate and save)
    ///
    /// This is a convenience method that generates a new identity and saves it.
    /// Equivalent to calling `Identity::generate()` and then `save()`.
    pub fn create(&mut self, name: &str) -> Result<Identity> {
        let identity = Identity::generate();
        self.save(&identity, name)?;
        Ok(identity)
    }

    fn identity_path(&self, name: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", name))
    }
}

/// Manages identity persistence using OS secure storage (keychain/credential manager)
pub struct SecureIdentityManager {
    storage: paykit_lib::secure_storage::DesktopKeyStorage,
    metadata_path: PathBuf,
}

#[derive(Serialize, Deserialize, Default)]
struct IdentitiesMetadata {
    identities: Vec<IdentityMetadata>,
}

#[derive(Serialize, Deserialize)]
struct IdentityMetadata {
    name: String,
    public_key_hex: String,
    public_key_z32: String,
    nickname: Option<String>,
    created_at: i64,
}

impl SecureIdentityManager {
    /// Create a new secure identity manager
    pub fn new(storage_dir: impl AsRef<Path>) -> Self {
        let storage = paykit_lib::secure_storage::DesktopKeyStorage::new("paykit-demo");
        let metadata_path = storage_dir.as_ref().join("identities_metadata.json");
        
        Self { storage, metadata_path }
    }
    
    /// Save an identity to secure storage
    pub async fn save(&self, identity: &Identity, name: &str) -> Result<()> {
        // Store secret key in OS keychain
        let secret_key_hex = hex::encode(identity.keypair.secret_key());
        self.storage.store(
            &format!("identity.{}.secret", name),
            secret_key_hex.as_bytes(),
            StoreOptions::default()
        ).await
        .map_err(|e| anyhow::anyhow!("Failed to store secret key: {}", e))?;
        
        // Update metadata file
        self.update_metadata(name, identity).await?;
        
        Ok(())
    }
    
    /// Load an identity from secure storage
    pub async fn load(&self, name: &str) -> Result<Identity> {
        // Load secret key from OS keychain
        let secret_bytes = self.storage.retrieve(&format!("identity.{}.secret", name))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to retrieve secret key: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Identity '{}' not found", name))?;
        
        let secret_key_hex = String::from_utf8(secret_bytes)?;
        let secret_bytes = hex::decode(&secret_key_hex)?;
        
        if secret_bytes.len() != 32 {
            anyhow::bail!("Invalid secret key length");
        }
        
        let mut secret_key = [0u8; 32];
        secret_key.copy_from_slice(&secret_bytes);
        
        let keypair = Keypair::from_secret_key(&secret_key);
        
        // Load metadata for nickname
        let metadata = self.load_metadata()?;
        let identity_info = metadata.identities.iter()
            .find(|i| i.name == name)
            .ok_or_else(|| anyhow::anyhow!("Identity metadata not found"))?;
        
        let mut identity = Identity::from_keypair(keypair);
        identity.nickname = identity_info.nickname.clone();
        
        Ok(identity)
    }
    
    /// List all identities
    pub fn list(&self) -> Result<Vec<String>> {
        let metadata = self.load_metadata()?;
        Ok(metadata.identities.iter().map(|i| i.name.clone()).collect())
    }
    
    /// Delete an identity
    pub async fn delete(&self, name: &str) -> Result<()> {
        // Delete from keychain
        self.storage.delete(&format!("identity.{}.secret", name))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete from keychain: {}", e))?;
        
        // Remove from metadata
        let mut metadata = self.load_metadata()?;
        metadata.identities.retain(|i| i.name != name);
        self.save_metadata(&metadata)?;
        
        Ok(())
    }
    
    async fn update_metadata(&self, name: &str, identity: &Identity) -> Result<()> {
        let mut metadata = self.load_metadata().unwrap_or_default();
        
        let identity_info = IdentityMetadata {
            name: name.to_string(),
            public_key_hex: hex::encode(identity.public_key().as_bytes()),
            public_key_z32: identity.public_key().to_string(),
            nickname: identity.nickname.clone(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64,
        };
        
        // Update or insert
        if let Some(existing) = metadata.identities.iter_mut().find(|i| i.name == name) {
            *existing = identity_info;
        } else {
            metadata.identities.push(identity_info);
        }
        
        self.save_metadata(&metadata)
    }
    
    fn load_metadata(&self) -> Result<IdentitiesMetadata> {
        if !self.metadata_path.exists() {
            return Ok(IdentitiesMetadata { identities: Vec::new() });
        }
        
        let json = std::fs::read_to_string(&self.metadata_path)?;
        Ok(serde_json::from_str(&json)?)
    }
    
    fn save_metadata(&self, metadata: &IdentitiesMetadata) -> Result<()> {
        if let Some(parent) = self.metadata_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let json = serde_json::to_string_pretty(metadata)?;
        std::fs::write(&self.metadata_path, json)?;
        Ok(())
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
        let key1 = identity.derive_x25519_key(device_id, 0);
        let key2 = identity.derive_x25519_key(device_id, 0);

        // Same inputs should produce same output
        assert_eq!(key1, key2);

        // Different epoch should produce different key
        let key3 = identity.derive_x25519_key(device_id, 1);
        assert_ne!(key1, key3);
    }
}
