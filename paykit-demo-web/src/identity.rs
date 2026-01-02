//! Identity management for WASM

use paykit_lib::PublicKey;
use pkarr::Keypair;
use pubky_noise::RingKeyProvider;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

use crate::utils;

/// Represents a user's identity for WASM
#[derive(Clone, Serialize, Deserialize)]
struct CoreIdentity {
    #[serde(
        serialize_with = "serialize_keypair",
        deserialize_with = "deserialize_keypair"
    )]
    keypair: Keypair,
    nickname: Option<String>,
}

fn serialize_keypair<S>(keypair: &Keypair, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    keypair.secret_key().serialize(serializer)
}

fn deserialize_keypair<'de, D>(deserializer: D) -> Result<Keypair, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let secret: [u8; 32] = <[u8; 32]>::deserialize(deserializer)?;
    Ok(Keypair::from_secret_key(&secret))
}

impl CoreIdentity {
    fn generate() -> Self {
        Self {
            keypair: Keypair::random(),
            nickname: None,
        }
    }

    fn with_nickname(mut self, nickname: impl Into<String>) -> Self {
        self.nickname = Some(nickname.into());
        self
    }

    fn public_key(&self) -> PublicKey {
        PublicKey::from_str(&self.keypair.public_key().to_z32()).unwrap()
    }

    fn pubky_uri(&self) -> String {
        format!("pubky://{}", self.public_key())
    }

    fn ed25519_secret(&self) -> [u8; 32] {
        self.keypair.secret_key()
    }

    fn ed25519_public(&self) -> [u8; 32] {
        self.keypair.public_key().to_bytes()
    }
}

/// Encrypted backup format for identity
#[derive(Clone, Serialize, Deserialize)]
struct EncryptedBackup {
    version: u32,
    encrypted_data_hex: String,
    salt_hex: String,
    nonce_hex: String,
    public_key_z32: String,
}

/// JavaScript-facing identity wrapper
#[wasm_bindgen]
#[derive(Clone)]
pub struct Identity {
    inner: CoreIdentity,
}

#[wasm_bindgen]
impl Identity {
    /// Generate a new random identity
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Identity, JsValue> {
        let inner = CoreIdentity::generate();
        Ok(Identity { inner })
    }

    /// Create an identity with a nickname
    #[wasm_bindgen(js_name = withNickname)]
    pub fn with_nickname(nickname: &str) -> Result<Identity, JsValue> {
        let inner = CoreIdentity::generate().with_nickname(nickname);
        Ok(Identity { inner })
    }

    /// Get the public key as a hex string
    #[wasm_bindgen(js_name = publicKey)]
    pub fn public_key(&self) -> String {
        self.inner.public_key().to_string()
    }

    /// Get the Pubky URI
    #[wasm_bindgen(js_name = pubkyUri)]
    pub fn pubky_uri(&self) -> String {
        self.inner.pubky_uri()
    }

    /// Get the nickname (if set)
    #[wasm_bindgen(js_name = nickname)]
    pub fn nickname(&self) -> Option<String> {
        self.inner.nickname.clone()
    }

    /// Export identity to JSON string
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner).map_err(|e| utils::js_error(&e.to_string()))
    }

    /// Import identity from JSON string
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(json: &str) -> Result<Identity, JsValue> {
        let inner: CoreIdentity =
            serde_json::from_str(json).map_err(|e| utils::js_error(&e.to_string()))?;
        Ok(Identity { inner })
    }

    /// Export identity to encrypted backup
    ///
    /// Uses Argon2 for password-based key derivation and AES-256-GCM for encryption.
    /// Returns a JSON string containing the encrypted backup.
    #[wasm_bindgen(js_name = exportBackup)]
    pub fn export_backup(&self, password: &str) -> Result<String, JsValue> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        use argon2::Argon2;
        use rand::RngCore;

        let secret_bytes = self.inner.keypair.secret_key();

        // Derive encryption key from password using Argon2
        let mut salt = [0u8; 16];
        rand::rngs::OsRng.fill_bytes(&mut salt);

        let mut encryption_key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), &salt, &mut encryption_key)
            .map_err(|e| utils::js_error(&format!("Key derivation failed: {}", e)))?;

        // Encrypt with AES-GCM
        let cipher = Aes256Gcm::new_from_slice(&encryption_key)
            .map_err(|e| utils::js_error(&format!("Cipher init failed: {}", e)))?;

        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let encrypted = cipher
            .encrypt(nonce, secret_bytes.as_ref())
            .map_err(|e| utils::js_error(&format!("Encryption failed: {}", e)))?;

        let backup = EncryptedBackup {
            version: 1,
            encrypted_data_hex: hex::encode(encrypted),
            salt_hex: hex::encode(salt),
            nonce_hex: hex::encode(nonce_bytes),
            public_key_z32: self.public_key(),
        };

        serde_json::to_string(&backup)
            .map_err(|e| utils::js_error(&format!("Serialization failed: {}", e)))
    }

    /// Import identity from encrypted backup
    ///
    /// Uses Argon2 for password-based key derivation and AES-256-GCM for decryption.
    #[wasm_bindgen(js_name = importBackup)]
    pub fn import_backup(backup_json: &str, password: &str) -> Result<Identity, JsValue> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        use argon2::Argon2;

        let backup: EncryptedBackup = serde_json::from_str(backup_json)
            .map_err(|e| utils::js_error(&format!("Invalid backup format: {}", e)))?;

        if backup.version != 1 {
            return Err(utils::js_error(&format!(
                "Unsupported backup version: {}",
                backup.version
            )));
        }

        let salt = hex::decode(&backup.salt_hex)
            .map_err(|e| utils::js_error(&format!("Invalid salt: {}", e)))?;
        let nonce_bytes = hex::decode(&backup.nonce_hex)
            .map_err(|e| utils::js_error(&format!("Invalid nonce: {}", e)))?;
        let encrypted = hex::decode(&backup.encrypted_data_hex)
            .map_err(|e| utils::js_error(&format!("Invalid encrypted data: {}", e)))?;

        // Derive encryption key from password
        let mut encryption_key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), &salt, &mut encryption_key)
            .map_err(|e| utils::js_error(&format!("Key derivation failed: {}", e)))?;

        // Decrypt with AES-GCM
        let cipher = Aes256Gcm::new_from_slice(&encryption_key)
            .map_err(|e| utils::js_error(&format!("Cipher init failed: {}", e)))?;

        if nonce_bytes.len() != 12 {
            return Err(utils::js_error("Invalid nonce length"));
        }
        let nonce = Nonce::from_slice(&nonce_bytes);

        let decrypted = cipher
            .decrypt(nonce, encrypted.as_ref())
            .map_err(|_| utils::js_error("Decryption failed - wrong password or corrupted data"))?;

        if decrypted.len() != 32 {
            return Err(utils::js_error("Invalid decrypted data length"));
        }

        let mut secret_key = [0u8; 32];
        secret_key.copy_from_slice(&decrypted);

        let keypair = Keypair::from_secret_key(&secret_key);

        // Verify public key matches
        let derived_public_key = keypair.public_key().to_z32();
        if derived_public_key != backup.public_key_z32 {
            return Err(utils::js_error(
                "Public key mismatch - backup may be corrupted",
            ));
        }

        Ok(Identity {
            inner: CoreIdentity {
                keypair,
                nickname: None,
            },
        })
    }

    /// Get Ed25519 secret key (for Noise key derivation)
    /// Returns hex-encoded secret key
    #[wasm_bindgen(js_name = ed25519SecretKeyHex)]
    pub fn ed25519_secret_key_hex(&self) -> String {
        hex::encode(self.inner.ed25519_secret())
    }

    /// Get Ed25519 public key (for Noise identity)
    /// Returns hex-encoded public key
    #[wasm_bindgen(js_name = ed25519PublicKeyHex)]
    pub fn ed25519_public_key_hex(&self) -> String {
        hex::encode(self.inner.ed25519_public())
    }
}

/// WASM-compatible key provider for Noise protocol
///
/// This wraps a pkarr Keypair and provides key derivation for Noise.
pub struct WasmKeyProvider {
    keypair: Keypair,
}

impl WasmKeyProvider {
    pub fn new(keypair: Keypair) -> Self {
        Self { keypair }
    }

    pub fn from_identity(identity: &Identity) -> Self {
        Self {
            keypair: identity.inner.keypair.clone(),
        }
    }
}

impl RingKeyProvider for WasmKeyProvider {
    fn derive_device_x25519(
        &self,
        _kid: &str,
        device_id: &[u8],
        epoch: u32,
    ) -> std::result::Result<[u8; 32], pubky_noise::NoiseError> {
        // Use pubky-noise's KDF for deterministic key derivation
        let secret = self.keypair.secret_key();
        pubky_noise::kdf::derive_x25519_for_device_epoch(&secret, device_id, epoch)
    }

    fn ed25519_pubkey(&self, _kid: &str) -> std::result::Result<[u8; 32], pubky_noise::NoiseError> {
        Ok(self.keypair.public_key().to_bytes())
    }

    fn sign_ed25519(
        &self,
        _kid: &str,
        msg: &[u8],
    ) -> std::result::Result<[u8; 64], pubky_noise::NoiseError> {
        Ok(self.keypair.sign(msg).to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_identity_generation() {
        let identity = Identity::new().unwrap();
        assert!(!identity.public_key().is_empty());
        assert!(identity.pubky_uri().starts_with("pubky://"));
    }

    #[wasm_bindgen_test]
    fn test_identity_with_nickname() {
        let identity = Identity::with_nickname("alice").unwrap();
        assert_eq!(identity.nickname(), Some("alice".to_string()));
    }

    #[wasm_bindgen_test]
    fn test_identity_json_round_trip() {
        let identity = Identity::with_nickname("bob").unwrap();
        let json = identity.to_json().unwrap();
        let restored = Identity::from_json(&json).unwrap();
        assert_eq!(identity.public_key(), restored.public_key());
        assert_eq!(identity.nickname(), restored.nickname());
    }
}
