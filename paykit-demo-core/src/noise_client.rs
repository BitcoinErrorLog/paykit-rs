//! Noise protocol client for interactive payments
//!
//! Provides helpers to establish encrypted Noise channels with payment recipients.

use anyhow::{Context, Result};
use paykit_interactive::transport::PubkyNoiseChannel;
use pubky_noise::{DummyRing, NoiseClient};
use std::sync::Arc;
use tokio::net::TcpStream;

use crate::Identity;

/// Helper for creating Noise clients for payment communication
pub struct NoiseClientHelper;

impl NoiseClientHelper {
    /// Create a Noise client from an identity
    ///
    /// The client can be used to establish encrypted channels with payment recipients.
    ///
    /// # Arguments
    /// * `identity` - The user's identity containing their keypair
    /// * `device_id` - A unique identifier for this device
    ///
    /// # Example
    /// ```no_run
    /// # use paykit_demo_core::{Identity, NoiseClientHelper};
    /// # fn example() -> anyhow::Result<()> {
    /// let identity = Identity::generate();
    /// let client = NoiseClientHelper::create_client(&identity, b"my-device");
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_client(identity: &Identity, device_id: &[u8]) -> Arc<NoiseClient<DummyRing, ()>> {
        // Use Ed25519 secret key as the seed for DummyRing
        // DummyRing will derive X25519 keys from this seed using HKDF
        let seed = identity.keypair.secret_key();

        // Create a ring key provider
        // Note: In production, you'd want a more secure key management system
        let ring = Arc::new(DummyRing::new(
            seed,
            identity.public_key().to_string(),
        ));

        // Create the Noise client
        Arc::new(NoiseClient::new_direct(
            identity.public_key().to_string(),
            device_id,
            ring,
        ))
    }

    /// Connect to a recipient's Noise server
    ///
    /// Establishes a TCP connection and performs Noise handshake.
    ///
    /// # Arguments
    /// * `identity` - The user's identity
    /// * `recipient_host` - The recipient's host address (e.g., "127.0.0.1:9735")
    /// * `recipient_static_pk` - The recipient's Noise static public key (32 bytes)
    ///
    /// # Returns
    /// An encrypted Noise channel ready for payment messages
    pub async fn connect_to_recipient(
        identity: &Identity,
        recipient_host: &str,
        recipient_static_pk: &[u8; 32],
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        // Create client
        let device_id = format!("paykit-demo-{}", identity.public_key());
        let client = Self::create_client(identity, device_id.as_bytes());

        // Connect TCP
        let stream = TcpStream::connect(recipient_host)
            .await
            .with_context(|| format!("Failed to connect to {}", recipient_host))?;

        // Perform Noise handshake
        let channel = PubkyNoiseChannel::connect(&client, stream, recipient_static_pk)
            .await
            .context("Failed to complete Noise handshake")?;

        Ok(channel)
    }

    /// Parse a recipient address into host and public key
    ///
    /// Expected format: "host:port@pubkey" or just "host:port" (pubkey resolved separately)
    ///
    /// # Example
    /// ```
    /// # use paykit_demo_core::NoiseClientHelper;
    /// let (host, pk) = NoiseClientHelper::parse_recipient_address(
    ///     "127.0.0.1:9735@8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
    /// ).unwrap();
    /// assert_eq!(host, "127.0.0.1:9735");
    /// ```
    pub fn parse_recipient_address(address: &str) -> Result<(String, Option<[u8; 32]>)> {
        if let Some((host, pk_str)) = address.split_once('@') {
            // Parse the public key
            let pk = Self::parse_public_key(pk_str)?;
            Ok((host.to_string(), Some(pk)))
        } else {
            // Just a host, no public key
            Ok((address.to_string(), None))
        }
    }

    /// Parse a z32-encoded public key string into bytes
    fn parse_public_key(pk_str: &str) -> Result<[u8; 32]> {
        // Try to parse as a Pubky PublicKey (z32 encoded)
        let pubkey: pubky::PublicKey = pk_str
            .parse()
            .with_context(|| format!("Invalid public key: {}", pk_str))?;

        // Convert to bytes
        Ok(pubkey.to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_client() {
        let identity = Identity::generate();
        let client = NoiseClientHelper::create_client(&identity, b"test-device");

        assert!(!client.kid.is_empty());
        assert_eq!(client.device_id, b"test-device");
    }

    #[test]
    fn test_parse_recipient_address_with_pk() {
        let address = "127.0.0.1:9735@8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
        let (host, pk) = NoiseClientHelper::parse_recipient_address(address).unwrap();

        assert_eq!(host, "127.0.0.1:9735");
        assert!(pk.is_some());
    }

    #[test]
    fn test_parse_recipient_address_without_pk() {
        let address = "127.0.0.1:9735";
        let (host, pk) = NoiseClientHelper::parse_recipient_address(address).unwrap();

        assert_eq!(host, "127.0.0.1:9735");
        assert!(pk.is_none());
    }

    #[test]
    fn test_parse_public_key() {
        let pk_str = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
        let result = NoiseClientHelper::parse_public_key(pk_str);

        assert!(result.is_ok());
        let pk = result.unwrap();
        assert_eq!(pk.len(), 32);
    }
}
