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
    ) -> std::result::Result<[u8; 32], pubky_noise::NoiseError> {
        // Use pubky-noise's KDF for deterministic key derivation
        let secret = self.keypair.secret_key();
        Ok(pubky_noise::kdf::derive_x25519_static(&secret, device_id))
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
