//! pkarr-based Noise key discovery for Paykit.
//!
//! This module provides async helpers for discovering Noise X25519 keys via pubky
//! storage and publishing your own keys for cold key scenarios.
//!
//! # Architecture
//!
//! In the Pubky ecosystem:
//! 1. Identity is based on Ed25519 keys stored in pkarr
//! 2. Noise sessions require X25519 keys
//! 3. X25519 keys are derived from Ed25519 and published to pubky storage with a binding signature
//!
//! # Storage Path Convention
//!
//! Noise keys are stored at: `/pub/noise.app/v0/{device_id}`
//! The content follows the pkarr TXT record format: `v=1;k={base64_x25519_pk};sig={base64_signature}`
//!
//! # Usage
//!
//! ```rust,ignore
//! use paykit_demo_core::pkarr_discovery::{discover_noise_key, publish_noise_key};
//!
//! // Discover a peer's X25519 key before connecting
//! let peer_x25519 = discover_noise_key(&storage, &peer_pubkey, "default").await?;
//!
//! // Publish your own X25519 key (one-time cold signing operation)
//! publish_noise_key(&session, &ed25519_sk, &x25519_pk, "default").await?;
//! ```

use crate::Result;
use anyhow::{anyhow, Context};

/// Storage path prefix for Noise keys in pubky storage.
/// Keys are stored at: `/pub/noise.app/v0/{device_id}`
pub const NOISE_KEY_PATH_PREFIX: &str = "/pub/noise.app/v0/";

/// Noise key discovery configuration.
#[derive(Clone, Debug)]
pub struct NoiseKeyConfig {
    /// Device identifier for multi-device scenarios
    pub device_id: String,
    /// Whether to verify the Ed25519 binding signature
    pub verify_binding: bool,
    /// Maximum acceptable key age in seconds (None = accept any age)
    /// Recommended: 30 days (2,592,000 seconds)
    pub max_age_seconds: Option<u64>,
}

impl Default for NoiseKeyConfig {
    fn default() -> Self {
        Self {
            device_id: "default".to_string(),
            verify_binding: true,
            max_age_seconds: Some(pubky_noise::pkarr_helpers::DEFAULT_MAX_KEY_AGE_SECONDS),
        }
    }
}

/// Discover a peer's X25519 Noise key via pubky storage lookup.
///
/// This is the primary method for cold key scenarios where:
/// 1. The peer has published their X25519 key to pubky storage
/// 2. The key is bound to their Ed25519 identity with a signature
///
/// # Arguments
/// * `storage` - The pubky PublicStorage client for reading
/// * `peer_pubkey` - The peer's Ed25519 public key
/// * `device_id` - Device identifier for multi-device lookup (e.g., "default", "mobile")
///
/// # Returns
/// The peer's 32-byte X25519 public key
///
/// # Security
/// This function verifies the Ed25519 signature binding the X25519 key to the identity.
/// If verification fails, an error is returned.
pub async fn discover_noise_key(
    storage: &pubky::PublicStorage,
    peer_pubkey: &pubky::PublicKey,
    device_id: &str,
) -> Result<[u8; 32]> {
    // Construct the storage path
    let path = format!(
        "pubky://{}{}{}",
        peer_pubkey, NOISE_KEY_PATH_PREFIX, device_id
    );

    // Fetch the noise key record
    let response = storage
        .get(&path)
        .await
        .with_context(|| format!("Failed to fetch noise key for {}", peer_pubkey))?;

    let txt_record = response
        .text()
        .await
        .with_context(|| "Failed to read noise key response body")?;

    if txt_record.is_empty() {
        return Err(anyhow!(
            "No noise key published for {} device '{}'",
            peer_pubkey,
            device_id
        ));
    }

    // Parse and verify the key binding
    let ed25519_pk_bytes = peer_pubkey.to_bytes();
    let x25519_pk = pubky_noise::pkarr_helpers::parse_and_verify_pkarr_key(
        &txt_record,
        &ed25519_pk_bytes,
        device_id,
    )
    .map_err(|e| anyhow!("Invalid noise key record: {}", e))?;

    Ok(x25519_pk)
}

/// Discover a peer's X25519 key without signature verification.
///
/// Use this only when you have verified the peer's identity through another channel.
/// For production use, prefer `discover_noise_key` which verifies signatures.
pub async fn discover_noise_key_unverified(
    storage: &pubky::PublicStorage,
    peer_pubkey: &pubky::PublicKey,
    device_id: &str,
) -> Result<[u8; 32]> {
    let path = format!(
        "pubky://{}{}{}",
        peer_pubkey, NOISE_KEY_PATH_PREFIX, device_id
    );

    let response = storage
        .get(&path)
        .await
        .with_context(|| format!("Failed to fetch noise key for {}", peer_pubkey))?;

    let txt_record = response
        .text()
        .await
        .with_context(|| "Failed to read noise key response body")?;

    if txt_record.is_empty() {
        return Err(anyhow!(
            "No noise key published for {} device '{}'",
            peer_pubkey,
            device_id
        ));
    }

    // Parse without verification
    let x25519_pk = pubky_noise::pkarr_helpers::parse_x25519_from_pkarr(&txt_record)
        .map_err(|e| anyhow!("Invalid noise key record format: {}", e))?;

    Ok(x25519_pk)
}

/// Discover a peer's X25519 key with full configuration.
///
/// This function supports:
/// - Signature verification (if `verify_binding` is true)
/// - Timestamp verification (if `max_age_seconds` is set)
/// - Device-specific key lookup
///
/// # Arguments
/// * `storage` - The pubky PublicStorage client
/// * `peer_pubkey` - The peer's Ed25519 public key
/// * `config` - Configuration for discovery and verification
///
/// # Returns
/// The verified 32-byte X25519 public key
pub async fn discover_noise_key_with_config(
    storage: &pubky::PublicStorage,
    peer_pubkey: &pubky::PublicKey,
    config: &NoiseKeyConfig,
) -> Result<[u8; 32]> {
    // Construct the storage path
    let path = format!(
        "pubky://{}{}{}",
        peer_pubkey, NOISE_KEY_PATH_PREFIX, config.device_id
    );

    // Fetch the noise key record
    let response = storage
        .get(&path)
        .await
        .with_context(|| format!("Failed to fetch noise key for {}", peer_pubkey))?;

    let txt_record = response
        .text()
        .await
        .with_context(|| "Failed to read noise key response body")?;

    if txt_record.is_empty() {
        return Err(anyhow!(
            "No noise key published for {} device '{}'",
            peer_pubkey,
            config.device_id
        ));
    }

    let ed25519_pk_bytes = peer_pubkey.to_bytes();

    // Choose verification strategy based on config
    match (config.verify_binding, config.max_age_seconds) {
        (true, Some(max_age)) => {
            // Full verification with timestamp check
            pubky_noise::pkarr_helpers::parse_and_verify_with_expiry(
                &txt_record,
                &ed25519_pk_bytes,
                &config.device_id,
                max_age,
            )
            .map_err(|e| anyhow!("pkarr verification failed: {}", e))
        }
        (true, None) => {
            // Signature verification only (no timestamp check)
            pubky_noise::pkarr_helpers::parse_and_verify_pkarr_key(
                &txt_record,
                &ed25519_pk_bytes,
                &config.device_id,
            )
            .map_err(|e| anyhow!("pkarr signature verification failed: {}", e))
        }
        (false, _) => {
            // No verification (parse only)
            pubky_noise::pkarr_helpers::parse_x25519_from_pkarr(&txt_record)
                .map_err(|e| anyhow!("pkarr parsing failed: {}", e))
        }
    }
}

/// Publish your X25519 Noise key to pubky storage with timestamp.
///
/// This should be called once during device setup (cold signing operation).
/// The Ed25519 key signs the X25519 key binding, then can be stored cold.
///
/// The published record includes a timestamp for freshness validation.
///
/// # Arguments
/// * `session` - Authenticated pubky session for writing
/// * `ed25519_sk` - Your Ed25519 secret key (32 bytes) - used only for signing, then can be stored cold
/// * `x25519_pk` - Your X25519 public key to publish (32 bytes)
/// * `device_id` - Device identifier for multi-device scenarios
///
/// # Security
/// After calling this function, the Ed25519 secret key can be stored cold.
/// Only the derived X25519 key is needed for runtime Noise sessions.
pub async fn publish_noise_key(
    session: &pubky::PubkySession,
    ed25519_sk: &[u8; 32],
    x25519_pk: &[u8; 32],
    device_id: &str,
) -> Result<()> {
    // Get current timestamp
    let timestamp = current_unix_timestamp();

    // Sign the key binding
    let signature =
        pubky_noise::pkarr_helpers::sign_pkarr_key_binding(ed25519_sk, x25519_pk, device_id);

    // Format the record with timestamp (recommended for production)
    let txt_value = pubky_noise::pkarr_helpers::format_x25519_for_pkarr_with_timestamp(
        x25519_pk,
        Some(&signature),
        timestamp,
    );

    // Construct the storage path
    let path = format!("{}{}", NOISE_KEY_PATH_PREFIX, device_id);

    // Publish to pubky storage via session's storage API
    session
        .storage()
        .put(path, txt_value)
        .await
        .with_context(|| "Failed to publish noise key")?;

    Ok(())
}

/// Get current Unix timestamp (seconds since epoch).
fn current_unix_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Complete cold key setup: derive X25519 key, sign binding, and publish.
///
/// This is a convenience function for the complete cold key setup flow.
/// After calling this, the Ed25519 key can be stored cold.
///
/// # Arguments
/// * `session` - Authenticated pubky session for writing
/// * `ed25519_sk` - Your Ed25519 secret key (32 bytes)
/// * `device_id` - Device identifier
///
/// # Returns
/// Tuple of (X25519 secret key, X25519 public key) - keep the secret key secure!
#[cfg(not(target_arch = "wasm32"))]
pub async fn setup_cold_key(
    session: &pubky::PubkySession,
    ed25519_sk: &[u8; 32],
    device_id: &str,
) -> Result<([u8; 32], [u8; 32])> {
    let (x25519_sk, x25519_pk) = derive_noise_keypair(ed25519_sk, device_id);

    publish_noise_key(session, ed25519_sk, &x25519_pk, device_id).await?;

    Ok((x25519_sk, x25519_pk))
}

/// Generate X25519 keypair from Ed25519 secret key for cold key setup.
///
/// This derives a deterministic X25519 keypair from the Ed25519 identity,
/// suitable for one-time cold key derivation.
///
/// # Arguments
/// * `ed25519_sk` - Your Ed25519 secret key (32 bytes)
/// * `device_id` - Device identifier for derivation context
///
/// # Returns
/// Tuple of (X25519 secret key, X25519 public key)
pub fn derive_noise_keypair(ed25519_sk: &[u8; 32], device_id: &str) -> ([u8; 32], [u8; 32]) {
    // Derive X25519 secret key from Ed25519 using KDF
    let x25519_sk = pubky_noise::kdf::derive_x25519_static(ed25519_sk, device_id.as_bytes());

    // Compute public key using X25519 base point multiplication
    let public = x25519_dalek::x25519(x25519_sk, x25519_dalek::X25519_BASEPOINT_BYTES);

    (x25519_sk, public)
}

/// Prepare cold key data for publication (offline operation) with timestamp.
///
/// This creates the signed record without publishing. Use this when you need
/// to prepare the data offline before publishing.
///
/// The record includes a timestamp for freshness validation (recommended for production).
///
/// # Arguments
/// * `ed25519_sk` - Your Ed25519 secret key (32 bytes)
/// * `device_id` - Device identifier
///
/// # Returns
/// Tuple of (X25519 secret key, X25519 public key, pkarr TXT record value)
pub fn prepare_cold_key_publication(
    ed25519_sk: &[u8; 32],
    device_id: &str,
) -> ([u8; 32], [u8; 32], String) {
    let (x25519_sk, x25519_pk) = derive_noise_keypair(ed25519_sk, device_id);

    // Get current timestamp
    let timestamp = current_unix_timestamp();

    // Sign the key binding
    let signature =
        pubky_noise::pkarr_helpers::sign_pkarr_key_binding(ed25519_sk, &x25519_pk, device_id);

    // Format for pkarr with timestamp
    let txt_value = pubky_noise::pkarr_helpers::format_x25519_for_pkarr_with_timestamp(
        &x25519_pk,
        Some(&signature),
        timestamp,
    );

    (x25519_sk, x25519_pk, txt_value)
}

/// Get the storage path for a noise key.
pub fn noise_key_path(device_id: &str) -> String {
    format!("{}{}", NOISE_KEY_PATH_PREFIX, device_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_noise_keypair_deterministic() {
        let ed25519_sk = [42u8; 32];
        let device_id = "test-device";

        let (sk1, pk1) = derive_noise_keypair(&ed25519_sk, device_id);
        let (sk2, pk2) = derive_noise_keypair(&ed25519_sk, device_id);

        assert_eq!(sk1, sk2);
        assert_eq!(pk1, pk2);
    }

    #[test]
    fn test_derive_noise_keypair_different_devices() {
        let ed25519_sk = [42u8; 32];

        let (sk1, pk1) = derive_noise_keypair(&ed25519_sk, "device-a");
        let (sk2, pk2) = derive_noise_keypair(&ed25519_sk, "device-b");

        assert_ne!(sk1, sk2);
        assert_ne!(pk1, pk2);
    }

    #[test]
    fn test_prepare_cold_key_publication() {
        let ed25519_sk = [1u8; 32];
        let device_id = "mobile";

        let (x25519_sk, x25519_pk, txt_value) =
            prepare_cold_key_publication(&ed25519_sk, device_id);

        // Verify the keys are valid
        assert_ne!(x25519_sk, [0u8; 32]);
        assert_ne!(x25519_pk, [0u8; 32]);

        // Verify the TXT record format includes timestamp
        assert!(txt_value.starts_with("v=1;k="));
        assert!(txt_value.contains(";sig="));
        assert!(txt_value.contains(";ts="), "TXT record should include timestamp");

        // Verify we can parse it back
        let parsed = pubky_noise::pkarr_helpers::parse_x25519_from_pkarr(&txt_value).unwrap();
        assert_eq!(parsed, x25519_pk);

        // Verify timestamp is present and reasonable
        let timestamp = pubky_noise::pkarr_helpers::extract_timestamp_from_pkarr(&txt_value);
        assert!(timestamp.is_some(), "Timestamp should be present");
        let ts = timestamp.unwrap();
        assert!(ts > 1700000000, "Timestamp should be recent (after Nov 2023)");
    }

    #[test]
    fn test_noise_key_path() {
        assert_eq!(noise_key_path("default"), "/pub/noise.app/v0/default");
        assert_eq!(noise_key_path("mobile"), "/pub/noise.app/v0/mobile");
    }

    #[test]
    fn test_noise_key_config_default() {
        let config = NoiseKeyConfig::default();
        assert_eq!(config.device_id, "default");
        assert!(config.verify_binding);
        assert!(config.max_age_seconds.is_some(), "Default should have max age");
        assert_eq!(
            config.max_age_seconds.unwrap(),
            pubky_noise::pkarr_helpers::DEFAULT_MAX_KEY_AGE_SECONDS
        );
    }
}
