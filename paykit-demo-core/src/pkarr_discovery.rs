//! pkarr-based Noise key discovery for Paykit.
//!
//! This module provides async helpers for discovering Noise X25519 keys via pkarr
//! and publishing your own keys for cold key scenarios.
//!
//! # Architecture
//!
//! In the Pubky ecosystem:
//! 1. Identity is based on Ed25519 keys stored in pkarr
//! 2. Noise sessions require X25519 keys
//! 3. X25519 keys are derived from Ed25519 and published to pkarr with a binding signature
//!
//! # Usage
//!
//! ```rust,ignore
//! use paykit_demo_core::pkarr_discovery::{discover_noise_key, publish_noise_key};
//!
//! // Discover a peer's X25519 key before connecting
//! let peer_x25519 = discover_noise_key(&sdk, &peer_pubky, "default").await?;
//!
//! // Publish your own X25519 key (one-time cold signing operation)
//! publish_noise_key(&session, &ed25519_keypair, &x25519_pk, "default").await?;
//! ```

use crate::Result;
use anyhow::anyhow;

/// Noise key discovery configuration.
#[derive(Clone, Debug)]
pub struct NoiseKeyConfig {
    /// Device identifier for multi-device scenarios
    pub device_id: String,
    /// Whether to verify the Ed25519 binding signature
    pub verify_binding: bool,
}

impl Default for NoiseKeyConfig {
    fn default() -> Self {
        Self {
            device_id: "default".to_string(),
            verify_binding: true,
        }
    }
}

/// Discover a peer's X25519 Noise key via pkarr lookup.
///
/// This is the primary method for cold key scenarios where:
/// 1. The peer has published their X25519 key to pkarr
/// 2. The key is bound to their Ed25519 identity with a signature
///
/// # Arguments
/// * `pubky_uri` - The peer's pubky URI (e.g., "pk:abc123...")
/// * `device_id` - Device identifier for multi-device lookup (e.g., "default", "mobile")
///
/// # Returns
/// The peer's 32-byte X25519 public key
///
/// # Note
/// This is a stub implementation. Full implementation requires pkarr SDK integration.
#[allow(unused_variables)]
pub async fn discover_noise_key(pubky_uri: &str, device_id: &str) -> Result<[u8; 32]> {
    // TODO: Implement full pkarr lookup using pubky SDK
    //
    // The implementation should:
    // 1. Parse the pubky URI to extract the Ed25519 public key
    // 2. Construct the TXT record path: _noise.{device_id}.{pubky}
    // 3. Query pkarr for the TXT record
    // 4. Parse the record using pubky_noise::pkarr_helpers::parse_x25519_from_pkarr
    // 5. Verify the binding signature if present
    //
    // Example (pseudocode):
    // ```
    // let ed25519_pk = parse_pubky_uri(pubky_uri)?;
    // let subdomain = pubky_noise::pkarr_helpers::pkarr_noise_subdomain(device_id);
    // let txt_record = pkarr_client.lookup_txt(&subdomain, &ed25519_pk).await?;
    // let x25519_pk = pubky_noise::pkarr_helpers::parse_and_verify_pkarr_key(
    //     &txt_record,
    //     &ed25519_pk,
    //     device_id,
    // )?;
    // Ok(x25519_pk)
    // ```

    Err(anyhow!(
        "pkarr discovery not yet implemented - requires pkarr SDK integration"
    ))
}

/// Discover a peer's X25519 key with full configuration.
#[allow(unused_variables)]
pub async fn discover_noise_key_with_config(
    pubky_uri: &str,
    config: &NoiseKeyConfig,
) -> Result<[u8; 32]> {
    discover_noise_key(pubky_uri, &config.device_id).await
}

/// Publish your X25519 Noise key to pkarr.
///
/// This should be called once during device setup (cold signing operation).
/// The Ed25519 key signs the X25519 key binding, then can be stored cold.
///
/// # Arguments
/// * `ed25519_sk` - Your Ed25519 secret key (32 bytes)
/// * `x25519_pk` - Your X25519 public key to publish (32 bytes)
/// * `device_id` - Device identifier for multi-device scenarios
///
/// # Note
/// This is a stub implementation. Full implementation requires pkarr SDK integration.
#[allow(unused_variables)]
pub async fn publish_noise_key(
    ed25519_sk: &[u8; 32],
    x25519_pk: &[u8; 32],
    device_id: &str,
) -> Result<()> {
    // TODO: Implement full pkarr publication using pubky SDK
    //
    // The implementation should:
    // 1. Sign the X25519 key binding using pubky_noise::pkarr_helpers::sign_pkarr_key_binding
    // 2. Format the TXT record using pubky_noise::pkarr_helpers::format_x25519_for_pkarr
    // 3. Publish to pkarr at path: _noise.{device_id}
    //
    // Example (pseudocode):
    // ```
    // let signature = pubky_noise::pkarr_helpers::sign_pkarr_key_binding(
    //     ed25519_sk,
    //     x25519_pk,
    //     device_id,
    // );
    // let txt_value = pubky_noise::pkarr_helpers::format_x25519_for_pkarr(x25519_pk, Some(&signature));
    // let subdomain = pubky_noise::pkarr_helpers::pkarr_noise_subdomain(device_id);
    // pkarr_client.publish_txt(&subdomain, &txt_value).await?;
    // Ok(())
    // ```

    Err(anyhow!(
        "pkarr publication not yet implemented - requires pkarr SDK integration"
    ))
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
    // The X25519 function computes the public key by scalar multiplication with the base point
    let public = x25519_dalek::x25519(x25519_sk, x25519_dalek::X25519_BASEPOINT_BYTES);

    (x25519_sk, public)
}

/// Complete cold key setup: derive X25519 key and prepare for publication.
///
/// This is a convenience function for the complete cold key setup flow.
/// Call this once with your Ed25519 key, then publish the result to pkarr.
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

    // Sign the key binding
    let signature =
        pubky_noise::pkarr_helpers::sign_pkarr_key_binding(ed25519_sk, &x25519_pk, device_id);

    // Format for pkarr
    let txt_value =
        pubky_noise::pkarr_helpers::format_x25519_for_pkarr(&x25519_pk, Some(&signature));

    (x25519_sk, x25519_pk, txt_value)
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

        // Verify the TXT record format
        assert!(txt_value.starts_with("v=1;k="));
        assert!(txt_value.contains(";sig="));

        // Verify we can parse it back
        let parsed = pubky_noise::pkarr_helpers::parse_x25519_from_pkarr(&txt_value).unwrap();
        assert_eq!(parsed, x25519_pk);
    }

    #[tokio::test]
    async fn test_discover_returns_not_implemented() {
        let result = discover_noise_key("pk:abc123", "default").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }

    #[tokio::test]
    async fn test_publish_returns_not_implemented() {
        let ed25519_sk = [1u8; 32];
        let x25519_pk = [2u8; 32];
        let result = publish_noise_key(&ed25519_sk, &x25519_pk, "default").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }
}

