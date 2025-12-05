//! Noise protocol client for interactive payments
//!
//! Provides helpers to establish encrypted Noise channels with payment recipients.
//!
//! # Security Warning
//!
//! **This module uses `DummyRing` for demonstration purposes only!**
//!
//! Production applications MUST use platform-specific secure storage:
//! - **iOS**: Keychain with `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`
//! - **Android**: KeyStore with `setUserAuthenticationRequired(true)`
//! - **Desktop**: OS credential manager or hardware security module (HSM)
//!
//! Never store Ed25519 seeds in plaintext files, environment variables, or unprotected memory.
//!
//! ## Pattern Support
//!
//! - **IK (default)**: Mutual authentication, Ed25519 signing at handshake time
//! - **IK-raw**: Cold key scenario, identity via pkarr lookup
//! - **N**: Anonymous client, authenticated server
//! - **NN**: Fully anonymous, post-handshake attestation required
//! - **XX**: Trust-on-first-use, both parties exchange static keys during handshake

use anyhow::{anyhow, Context, Result};
use paykit_interactive::transport::PubkyNoiseChannel;
use pubky_noise::{datalink_adapter, kdf, DummyRing, NoiseClient};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use zeroize::Zeroizing;

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
    pub fn create_client(identity: &Identity, device_id: &[u8]) -> Arc<NoiseClient<DummyRing>> {
        // Use Ed25519 secret key as the seed for DummyRing
        // DummyRing will derive X25519 keys from this seed using HKDF
        let seed = identity.keypair.secret_key();

        // Create a ring key provider
        // Note: In production, you'd want a more secure key management system
        let ring = Arc::new(DummyRing::new(seed, identity.public_key().to_string()));

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

    /// Connect with pattern negotiation (sends pattern byte before handshake).
    ///
    /// Use this with `NoiseServerHelper::run_pattern_server` which expects a pattern byte.
    ///
    /// # Arguments
    /// * `identity` - The user's identity
    /// * `recipient_host` - The recipient's host address (e.g., "127.0.0.1:9735")
    /// * `recipient_static_pk` - The recipient's Noise static public key (32 bytes)
    ///
    /// # Returns
    /// An encrypted Noise channel ready for payment messages
    pub async fn connect_to_recipient_with_negotiation(
        identity: &Identity,
        recipient_host: &str,
        recipient_static_pk: &[u8; 32],
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        use tokio::io::AsyncWriteExt;

        // Create client
        let device_id = format!("paykit-demo-{}", identity.public_key());
        let client = Self::create_client(identity, device_id.as_bytes());

        // Connect TCP and send pattern byte
        let mut stream = TcpStream::connect(recipient_host)
            .await
            .with_context(|| format!("Failed to connect to {}", recipient_host))?;

        // Send IK pattern byte (0x00) for pattern-aware servers
        stream
            .write_all(&[NoisePattern::IK.negotiation_byte()])
            .await
            .context("Failed to send pattern byte")?;

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

/// Maximum handshake message size (Noise handshakes are typically <512 bytes)
const MAX_HANDSHAKE_SIZE: usize = 4096;

// Re-export NoisePattern from pubky-noise
pub use pubky_noise::NoisePattern;

/// Convert a negotiation byte to NoisePattern with anyhow error handling.
///
/// This wraps pubky_noise::NoisePattern::from_byte for convenient error handling.
pub fn pattern_from_byte(byte: u8) -> Result<NoisePattern> {
    NoisePattern::from_byte(byte).ok_or_else(|| anyhow!("Unknown Noise pattern byte: {}", byte))
}

/// Helper for raw-key Noise connections (cold key scenarios).
///
/// Unlike `NoiseClientHelper` which uses the Ring abstraction for key management,
/// `NoiseRawClientHelper` accepts pre-derived X25519 keys directly. This is designed
/// for cold key architectures where Ed25519 keys are kept offline.
///
/// ## Use Cases
///
/// - **IK-raw**: Connect to a server whose X25519 key is published via pkarr
/// - **N**: Anonymous client connecting to a known server (e.g., donation box)
/// - **NN**: Fully anonymous connection with post-handshake attestation
///
/// ## Security
///
/// When using raw patterns, the caller is responsible for:
/// - Verifying server identity through pkarr (for IK-raw and N)
/// - Implementing post-handshake attestation (for NN)
/// - Proper key zeroization
pub struct NoiseRawClientHelper;

impl NoiseRawClientHelper {
    /// Connect using IK-raw pattern (cold key scenario).
    ///
    /// Identity is verified externally via pkarr, not during the handshake.
    ///
    /// # Arguments
    /// * `x25519_sk` - Your X25519 secret key (derived from Ed25519 seed)
    /// * `recipient_host` - The recipient's host address (e.g., "127.0.0.1:9735")
    /// * `recipient_static_pk` - The recipient's X25519 public key (from pkarr)
    ///
    /// # Returns
    /// An encrypted Noise channel ready for payment messages
    ///
    /// # Example
    /// ```no_run
    /// # use paykit_demo_core::NoiseRawClientHelper;
    /// # use zeroize::Zeroizing;
    /// # use pubky_noise::kdf;
    /// # async fn example() -> anyhow::Result<()> {
    /// // Derive X25519 key from seed (done once, published to pkarr)
    /// let seed = [0u8; 32];
    /// let x25519_sk = Zeroizing::new(kdf::derive_x25519_static(&seed, b"device"));
    /// let server_pk = [0u8; 32]; // From pkarr lookup
    ///
    /// let channel = NoiseRawClientHelper::connect_ik_raw(
    ///     &x25519_sk,
    ///     "127.0.0.1:9735",
    ///     &server_pk,
    /// ).await?;
    ///
    /// // When connecting to a pattern-aware server, prefer
    /// // `connect_ik_raw_with_negotiation` so the pattern byte is written
    /// // before the handshake begins.
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_ik_raw(
        x25519_sk: &Zeroizing<[u8; 32]>,
        recipient_host: &str,
        recipient_static_pk: &[u8; 32],
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        let stream = TcpStream::connect(recipient_host)
            .await
            .with_context(|| format!("Failed to connect to {}", recipient_host))?;
        Self::connect_ik_raw_with_stream(x25519_sk, stream, recipient_static_pk).await
    }

    /// Connect using IK-raw pattern and automatically negotiate the pattern byte.
    pub async fn connect_ik_raw_with_negotiation(
        x25519_sk: &Zeroizing<[u8; 32]>,
        recipient_host: &str,
        recipient_static_pk: &[u8; 32],
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        let stream = Self::connect_stream_with_pattern(recipient_host, NoisePattern::IKRaw).await?;
        Self::connect_ik_raw_with_stream(x25519_sk, stream, recipient_static_pk).await
    }

    /// Connect using IK-raw pattern over an existing TCP stream.
    ///
    /// Use this when a pattern byte has already been sent (pattern-aware servers).
    pub async fn connect_ik_raw_with_stream(
        x25519_sk: &Zeroizing<[u8; 32]>,
        mut stream: TcpStream,
        recipient_static_pk: &[u8; 32],
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        let (hs, first_msg) = datalink_adapter::start_ik_raw(x25519_sk, recipient_static_pk)
            .map_err(|e| anyhow!("Handshake init failed: {}", e))?;

        let len = (first_msg.len() as u32).to_be_bytes();
        stream
            .write_all(&len)
            .await
            .context("Failed to send handshake length")?;
        stream
            .write_all(&first_msg)
            .await
            .context("Failed to send handshake")?;

        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .context("Failed to read response length")?;
        let response_len = u32::from_be_bytes(len_bytes) as usize;
        if response_len > MAX_HANDSHAKE_SIZE {
            return Err(anyhow!(
                "Handshake response too large: {} bytes",
                response_len
            ));
        }

        let mut response = vec![0u8; response_len];
        stream
            .read_exact(&mut response)
            .await
            .context("Failed to read response")?;

        let session = datalink_adapter::complete_raw(hs, &response)
            .map_err(|e| anyhow!("Handshake completion failed: {}", e))?;

        Ok(PubkyNoiseChannel::new(stream, session))
    }

    /// Connect using N pattern (anonymous client, authenticated server).
    ///
    /// The client is anonymous (no static key), while the server is authenticated
    /// via pkarr. Use for anonymous payment requests like donation boxes.
    ///
    /// # Arguments
    /// * `recipient_host` - The recipient's host address (e.g., "127.0.0.1:9735")
    /// * `recipient_static_pk` - The recipient's X25519 public key (from pkarr)
    ///
    /// # Returns
    /// An encrypted Noise channel ready for payment messages
    ///
    /// # Example
    /// ```no_run
    /// # use paykit_demo_core::NoiseRawClientHelper;
    /// # async fn example() -> anyhow::Result<()> {
    /// let server_pk = [0u8; 32]; // From pkarr lookup
    ///
    /// let channel = NoiseRawClientHelper::connect_anonymous(
    ///     "127.0.0.1:9735",
    ///     &server_pk,
    /// ).await?;
    /// // Client is anonymous, server is authenticated
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_anonymous(
        recipient_host: &str,
        recipient_static_pk: &[u8; 32],
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        let stream = TcpStream::connect(recipient_host)
            .await
            .with_context(|| format!("Failed to connect to {}", recipient_host))?;
        Self::connect_anonymous_with_stream(stream, recipient_static_pk).await
    }

    /// Anonymous N-pattern connection with automatic pattern negotiation.
    pub async fn connect_anonymous_with_negotiation(
        recipient_host: &str,
        recipient_static_pk: &[u8; 32],
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        let stream = Self::connect_stream_with_pattern(recipient_host, NoisePattern::N).await?;
        Self::connect_anonymous_with_stream(stream, recipient_static_pk).await
    }

    /// Anonymous N-pattern connection over an existing TCP stream.
    pub async fn connect_anonymous_with_stream(
        mut stream: TcpStream,
        recipient_static_pk: &[u8; 32],
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        let (hs, first_msg) = datalink_adapter::start_n(recipient_static_pk)
            .map_err(|e| anyhow!("Handshake init failed: {}", e))?;

        let len = (first_msg.len() as u32).to_be_bytes();
        stream
            .write_all(&len)
            .await
            .context("Failed to send handshake length")?;
        stream
            .write_all(&first_msg)
            .await
            .context("Failed to send handshake")?;

        let session = datalink_adapter::complete_n(hs)
            .map_err(|e| anyhow!("Handshake completion failed: {}", e))?;

        Ok(PubkyNoiseChannel::new(stream, session))
    }

    /// Connect using NN pattern (fully anonymous).
    ///
    /// Neither party has a static key - purely ephemeral key exchange.
    /// **Important**: Without post-handshake attestation, this is vulnerable to MITM.
    ///
    /// # Arguments
    /// * `recipient_host` - The recipient's host address (e.g., "127.0.0.1:9735")
    ///
    /// # Returns
    /// A tuple of (channel, server_ephemeral_pk, client_ephemeral_pk) for post-handshake verification
    ///
    /// # Example
    /// ```no_run
    /// # use paykit_demo_core::NoiseRawClientHelper;
    /// # async fn example() -> anyhow::Result<()> {
    /// let (mut channel, server_ephemeral, client_ephemeral) = NoiseRawClientHelper::connect_ephemeral(
    ///     "127.0.0.1:9735",
    /// ).await?;
    ///
    /// // IMPORTANT: Perform post-handshake attestation
    /// // e.g., server signs a challenge with their Ed25519 key using both ephemerals
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_ephemeral(
        recipient_host: &str,
    ) -> Result<(PubkyNoiseChannel<TcpStream>, [u8; 32], [u8; 32])> {
        let stream = TcpStream::connect(recipient_host)
            .await
            .with_context(|| format!("Failed to connect to {}", recipient_host))?;
        Self::connect_ephemeral_with_stream(stream).await
    }

    /// NN-pattern connection over an existing TCP stream.
    pub async fn connect_ephemeral_with_stream(
        mut stream: TcpStream,
    ) -> Result<(PubkyNoiseChannel<TcpStream>, [u8; 32], [u8; 32])> {
        let (hs, first_msg) =
            datalink_adapter::start_nn().map_err(|e| anyhow!("Handshake init failed: {}", e))?;

        // Validate message length before slicing to prevent panic
        if first_msg.len() < 32 {
            return Err(anyhow!(
                "NN handshake message too short: {} bytes (need at least 32)",
                first_msg.len()
            ));
        }

        let client_ephemeral: [u8; 32] = first_msg[..32]
            .try_into()
            .map_err(|_| anyhow!("Invalid first message length"))?;

        let len = (first_msg.len() as u32).to_be_bytes();
        stream
            .write_all(&len)
            .await
            .context("Failed to send handshake length")?;
        stream
            .write_all(&first_msg)
            .await
            .context("Failed to send handshake")?;

        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .context("Failed to read response length")?;
        let response_len = u32::from_be_bytes(len_bytes) as usize;
        if response_len > MAX_HANDSHAKE_SIZE {
            return Err(anyhow!(
                "Handshake response too large: {} bytes",
                response_len
            ));
        }

        let mut response = vec![0u8; response_len];
        stream
            .read_exact(&mut response)
            .await
            .context("Failed to read response")?;

        // Validate response length before slicing to prevent panic
        if response.len() < 32 {
            return Err(anyhow!(
                "NN handshake response too short: {} bytes (need at least 32)",
                response.len()
            ));
        }

        let server_ephemeral: [u8; 32] = response[..32]
            .try_into()
            .map_err(|_| anyhow!("Invalid response length"))?;

        let session = datalink_adapter::complete_raw(hs, &response)
            .map_err(|e| anyhow!("Handshake completion failed: {}", e))?;

        Ok((
            PubkyNoiseChannel::new(stream, session),
            server_ephemeral,
            client_ephemeral,
        ))
    }

    /// NN-pattern connection with automatic pattern negotiation.
    pub async fn connect_ephemeral_with_negotiation(
        recipient_host: &str,
    ) -> Result<(PubkyNoiseChannel<TcpStream>, [u8; 32], [u8; 32])> {
        let stream = Self::connect_stream_with_pattern(recipient_host, NoisePattern::NN).await?;
        Self::connect_ephemeral_with_stream(stream).await
    }

    /// Connect using XX pattern (trust-on-first-use).
    ///
    /// Both parties exchange static keys during a 3-message handshake.
    /// Use for TOFU scenarios where keys are cached after first contact.
    ///
    /// # Arguments
    /// * `x25519_sk` - Your X25519 secret key (derived from Ed25519 seed)
    /// * `recipient_host` - The recipient's host address (e.g., "127.0.0.1:9735")
    ///
    /// # Returns
    /// A tuple of (channel, server_static_pk) - cache server_static_pk for future IK connections
    ///
    /// # Example
    /// ```no_run
    /// # use paykit_demo_core::NoiseRawClientHelper;
    /// # use zeroize::Zeroizing;
    /// # async fn example() -> anyhow::Result<()> {
    /// let seed = [42u8; 32];
    /// let x25519_sk = NoiseRawClientHelper::derive_x25519_key(&seed, b"device");
    ///
    /// let (channel, server_static_pk) = NoiseRawClientHelper::connect_xx(
    ///     &x25519_sk,
    ///     "127.0.0.1:9735",
    /// ).await?;
    ///
    /// // Cache server_static_pk for future IK pattern connections
    /// println!("Server static key: {}", hex::encode(&server_static_pk));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_xx(
        x25519_sk: &Zeroizing<[u8; 32]>,
        recipient_host: &str,
    ) -> Result<(PubkyNoiseChannel<TcpStream>, [u8; 32])> {
        let stream = TcpStream::connect(recipient_host)
            .await
            .with_context(|| format!("Failed to connect to {}", recipient_host))?;
        Self::connect_xx_with_stream(x25519_sk, stream).await
    }

    /// XX-pattern connection with automatic pattern negotiation.
    pub async fn connect_xx_with_negotiation(
        x25519_sk: &Zeroizing<[u8; 32]>,
        recipient_host: &str,
    ) -> Result<(PubkyNoiseChannel<TcpStream>, [u8; 32])> {
        let stream = Self::connect_stream_with_pattern(recipient_host, NoisePattern::XX).await?;
        Self::connect_xx_with_stream(x25519_sk, stream).await
    }

    /// XX-pattern connection over an existing TCP stream.
    ///
    /// This implements the full 3-message XX handshake:
    /// 1. Client -> Server: `e` (client ephemeral)
    /// 2. Server -> Client: `e, ee, s, es` (server ephemeral + static)
    /// 3. Client -> Server: `s, se` (client static)
    pub async fn connect_xx_with_stream(
        x25519_sk: &Zeroizing<[u8; 32]>,
        mut stream: TcpStream,
    ) -> Result<(PubkyNoiseChannel<TcpStream>, [u8; 32])> {
        use pubky_noise::NoiseSender;

        // 1. Start XX handshake (message 1: -> e)
        let sender = NoiseSender::new();
        let (mut hs, first_msg) = sender
            .initiate_xx(x25519_sk)
            .map_err(|e| anyhow!("XX handshake init failed: {}", e))?;

        // Send first message (length-prefixed)
        let len = (first_msg.len() as u32).to_be_bytes();
        stream
            .write_all(&len)
            .await
            .context("Failed to send XX first message length")?;
        stream
            .write_all(&first_msg)
            .await
            .context("Failed to send XX first message")?;

        // 2. Read server response (message 2: <- e, ee, s, es)
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .context("Failed to read XX response length")?;
        let response_len = u32::from_be_bytes(len_bytes) as usize;

        if response_len > MAX_HANDSHAKE_SIZE {
            return Err(anyhow!(
                "XX handshake response too large: {} bytes (max {})",
                response_len,
                MAX_HANDSHAKE_SIZE
            ));
        }

        let mut response = vec![0u8; response_len];
        stream
            .read_exact(&mut response)
            .await
            .context("Failed to read XX response")?;

        // Process server's response (reads e, ee, s, es)
        let mut buf = vec![0u8; response.len() + 256];
        hs.read_message(&response, &mut buf)
            .context("Failed to process XX server response")?;

        // 3. Send final message (message 3: -> s, se)
        let mut final_msg = vec![0u8; 128];
        let n = hs
            .write_message(&[], &mut final_msg)
            .context("Failed to generate XX final message")?;
        final_msg.truncate(n);

        // Send final message (length-prefixed)
        let len = (final_msg.len() as u32).to_be_bytes();
        stream
            .write_all(&len)
            .await
            .context("Failed to send XX final message length")?;
        stream
            .write_all(&final_msg)
            .await
            .context("Failed to send XX final message")?;

        // Extract server's static key (learned during message 2)
        let server_static_pk: [u8; 32] = hs
            .get_remote_static()
            .ok_or_else(|| anyhow!("No server static key in XX handshake"))?
            .try_into()
            .map_err(|_| anyhow!("Invalid server static key length"))?;

        // Complete handshake to get transport session
        let session = pubky_noise::NoiseSession::from_handshake(hs)
            .map_err(|e| anyhow!("XX handshake completion failed: {}", e))?;

        Ok((PubkyNoiseChannel::new(stream, session), server_static_pk))
    }

    async fn connect_stream_with_pattern(
        recipient_host: &str,
        pattern: NoisePattern,
    ) -> Result<TcpStream> {
        let mut stream = TcpStream::connect(recipient_host)
            .await
            .with_context(|| format!("Failed to connect to {}", recipient_host))?;
        stream
            .write_all(&[pattern.negotiation_byte()])
            .await
            .context("Failed to send pattern byte")?;
        Ok(stream)
    }

    /// Derive an X25519 secret key from an Ed25519 seed.
    ///
    /// Convenience method for cold key scenarios.
    ///
    /// # Example
    /// ```
    /// # use paykit_demo_core::NoiseRawClientHelper;
    /// let seed = [0u8; 32]; // Your Ed25519 seed
    /// let x25519_sk = NoiseRawClientHelper::derive_x25519_key(&seed, b"device-001");
    /// assert_eq!(x25519_sk.len(), 32);
    /// ```
    pub fn derive_x25519_key(
        ed25519_seed: &[u8; 32],
        device_context: &[u8],
    ) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(kdf::derive_x25519_static(ed25519_seed, device_context))
    }

    /// Get the X25519 public key from a secret key.
    ///
    /// # Example
    /// ```
    /// # use paykit_demo_core::NoiseRawClientHelper;
    /// # use zeroize::Zeroizing;
    /// let x25519_sk = Zeroizing::new([0u8; 32]);
    /// let x25519_pk = NoiseRawClientHelper::x25519_public_key(&x25519_sk);
    /// assert_eq!(x25519_pk.len(), 32);
    /// ```
    pub fn x25519_public_key(secret_key: &Zeroizing<[u8; 32]>) -> [u8; 32] {
        kdf::x25519_pk_from_sk(secret_key)
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

    // NoisePattern tests

    #[test]
    fn test_noise_pattern_display() {
        assert_eq!(format!("{}", NoisePattern::IK), "IK");
        assert_eq!(format!("{}", NoisePattern::IKRaw), "IK-raw");
        assert_eq!(format!("{}", NoisePattern::N), "N");
        assert_eq!(format!("{}", NoisePattern::NN), "NN");
        assert_eq!(format!("{}", NoisePattern::XX), "XX");
    }

    #[test]
    fn test_noise_pattern_parse() {
        assert_eq!("ik".parse::<NoisePattern>().unwrap(), NoisePattern::IK);
        assert_eq!("IK".parse::<NoisePattern>().unwrap(), NoisePattern::IK);
        assert_eq!(
            "ik-raw".parse::<NoisePattern>().unwrap(),
            NoisePattern::IKRaw
        );
        assert_eq!(
            "IK_raw".parse::<NoisePattern>().unwrap(),
            NoisePattern::IKRaw
        );
        assert_eq!("n".parse::<NoisePattern>().unwrap(), NoisePattern::N);
        assert_eq!("nn".parse::<NoisePattern>().unwrap(), NoisePattern::NN);
        assert_eq!("xx".parse::<NoisePattern>().unwrap(), NoisePattern::XX);
        assert_eq!("XX".parse::<NoisePattern>().unwrap(), NoisePattern::XX);
        assert!("invalid".parse::<NoisePattern>().is_err());
    }

    // NoiseRawClientHelper tests

    #[test]
    fn test_derive_x25519_key() {
        let seed = [42u8; 32];
        let key1 = NoiseRawClientHelper::derive_x25519_key(&seed, b"device-1");
        let key2 = NoiseRawClientHelper::derive_x25519_key(&seed, b"device-2");
        let key1_again = NoiseRawClientHelper::derive_x25519_key(&seed, b"device-1");

        // Same inputs should produce same outputs
        assert_eq!(*key1, *key1_again);
        // Different contexts should produce different keys
        assert_ne!(*key1, *key2);
    }

    #[test]
    fn test_x25519_public_key() {
        let seed = [42u8; 32];
        let sk = NoiseRawClientHelper::derive_x25519_key(&seed, b"device");
        let pk = NoiseRawClientHelper::x25519_public_key(&sk);

        assert_eq!(pk.len(), 32);
        // Public key should not be all zeros
        assert!(pk.iter().any(|&b| b != 0));
    }
}
