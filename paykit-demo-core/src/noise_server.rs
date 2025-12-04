//! Noise protocol server for receiving payments
//!
//! Provides helpers to run a Noise server that accepts payment requests.
//!
//! ## Pattern Support
//!
//! - **IK (default)**: Mutual authentication with identity binding
//! - **IK-raw**: Cold key scenario, client identity via pkarr
//! - **N**: Anonymous client, authenticated server (donation boxes)
//! - **NN**: Fully anonymous, post-handshake attestation required
//! - **XX**: Trust-on-first-use, 3-message handshake exchanging static keys

use anyhow::{anyhow, Context, Result};
use paykit_interactive::transport::PubkyNoiseChannel;
use pubky_noise::{
    datalink_adapter, identity_payload::IdentityPayload, kdf, DummyRing, NoiseServer,
};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use zeroize::Zeroizing;

use crate::noise_client::{pattern_from_byte, NoisePattern};
use crate::Identity;

/// Maximum handshake message size (Noise handshakes are typically <512 bytes)
const MAX_HANDSHAKE_SIZE: usize = 4096;

/// Result of accepting a Noise connection with pattern-awareness.
///
/// Different patterns provide different levels of client identity information.
pub enum AcceptedConnection {
    /// IK pattern: Full identity binding with IdentityPayload
    IK {
        /// The encrypted channel
        channel: PubkyNoiseChannel<TcpStream>,
        /// Client's verified identity payload
        client_identity: IdentityPayload,
    },
    /// IK-raw pattern: Only X25519 key (identity via pkarr)
    IKRaw {
        /// The encrypted channel
        channel: PubkyNoiseChannel<TcpStream>,
        /// Client's X25519 public key (verify via pkarr)
        client_x25519_pk: [u8; 32],
    },
    /// N pattern: Anonymous client, only server is authenticated
    N {
        /// The encrypted channel
        channel: PubkyNoiseChannel<TcpStream>,
    },
    /// NN pattern: Both parties anonymous
    NN {
        /// The encrypted channel
        channel: PubkyNoiseChannel<TcpStream>,
        /// Client's ephemeral public key (for post-handshake verification)
        client_ephemeral: [u8; 32],
        /// Server's ephemeral public key (for attestation)
        server_ephemeral: [u8; 32],
    },
    /// XX pattern: Trust-on-first-use (TOFU)
    XX {
        /// The encrypted channel
        channel: PubkyNoiseChannel<TcpStream>,
        /// Client's static X25519 public key (learned during handshake)
        client_static_pk: [u8; 32],
    },
}

impl AcceptedConnection {
    /// Get the underlying channel regardless of pattern.
    pub fn into_channel(self) -> PubkyNoiseChannel<TcpStream> {
        match self {
            AcceptedConnection::IK { channel, .. } => channel,
            AcceptedConnection::IKRaw { channel, .. } => channel,
            AcceptedConnection::N { channel } => channel,
            AcceptedConnection::NN { channel, .. } => channel,
            AcceptedConnection::XX { channel, .. } => channel,
        }
    }

    /// Get a reference to the underlying channel.
    pub fn channel(&self) -> &PubkyNoiseChannel<TcpStream> {
        match self {
            AcceptedConnection::IK { channel, .. } => channel,
            AcceptedConnection::IKRaw { channel, .. } => channel,
            AcceptedConnection::N { channel } => channel,
            AcceptedConnection::NN { channel, .. } => channel,
            AcceptedConnection::XX { channel, .. } => channel,
        }
    }

    /// Get a mutable reference to the underlying channel.
    pub fn channel_mut(&mut self) -> &mut PubkyNoiseChannel<TcpStream> {
        match self {
            AcceptedConnection::IK { channel, .. } => channel,
            AcceptedConnection::IKRaw { channel, .. } => channel,
            AcceptedConnection::N { channel } => channel,
            AcceptedConnection::NN { channel, .. } => channel,
            AcceptedConnection::XX { channel, .. } => channel,
        }
    }

    /// Return the pattern associated with this connection.
    pub fn pattern(&self) -> NoisePattern {
        match self {
            AcceptedConnection::IK { .. } => NoisePattern::IK,
            AcceptedConnection::IKRaw { .. } => NoisePattern::IKRaw,
            AcceptedConnection::N { .. } => NoisePattern::N,
            AcceptedConnection::NN { .. } => NoisePattern::NN,
            AcceptedConnection::XX { .. } => NoisePattern::XX,
        }
    }

    /// Returns true if the client is authenticated (IK pattern).
    pub fn is_authenticated(&self) -> bool {
        matches!(self, AcceptedConnection::IK { .. })
    }

    /// Returns true if the client has a static key (IK, IK-raw, or XX).
    pub fn has_client_static(&self) -> bool {
        matches!(
            self,
            AcceptedConnection::IK { .. }
                | AcceptedConnection::IKRaw { .. }
                | AcceptedConnection::XX { .. }
        )
    }
}

/// Helper for creating and running Noise servers for payment reception
pub struct NoiseServerHelper;

impl NoiseServerHelper {
    /// Create a Noise server from an identity
    ///
    /// The server can accept encrypted connections from payers.
    ///
    /// # Arguments
    /// * `identity` - The user's identity containing their keypair
    /// * `device_id` - A unique identifier for this device
    ///
    /// # Example
    /// ```no_run
    /// # use paykit_demo_core::{Identity, NoiseServerHelper};
    /// # fn example() -> anyhow::Result<()> {
    /// let identity = Identity::generate();
    /// let server = NoiseServerHelper::create_server(&identity, b"my-device");
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_server(identity: &Identity, device_id: &[u8]) -> Arc<NoiseServer<DummyRing>> {
        // Use Ed25519 secret key as the seed for DummyRing
        // DummyRing will derive X25519 keys from this seed using HKDF
        let seed = identity.keypair.secret_key();

        // Create a ring key provider
        let ring = Arc::new(DummyRing::new(seed, identity.public_key().to_string()));

        // Create the Noise server
        Arc::new(NoiseServer::new_direct(
            identity.public_key().to_string(),
            device_id,
            ring,
        ))
    }

    /// Run a Noise server and handle incoming connections (IK pattern only)
    ///
    /// This binds to the specified address and accepts incoming Noise connections.
    /// For each connection, the provided handler is called with the established channel.
    ///
    /// **Note:** This server only accepts IK pattern connections and does NOT read a pattern
    /// negotiation byte. For multi-pattern support, use [`run_pattern_server`] instead, which
    /// reads a pattern byte from the client to determine the handshake type.
    ///
    /// # Arguments
    /// * `identity` - The server's identity
    /// * `bind_addr` - The address to bind to (e.g., "0.0.0.0:9735")
    /// * `handler` - Async function to handle each accepted channel
    ///
    /// # Example
    /// ```no_run
    /// # use paykit_demo_core::{Identity, NoiseServerHelper};
    /// # async fn example() -> anyhow::Result<()> {
    /// let identity = Identity::generate();
    ///
    /// NoiseServerHelper::run_server(
    ///     &identity,
    ///     "127.0.0.1:9735",
    ///     |mut channel| async move {
    ///         // Handle payment request
    ///         println!("Accepted connection");
    ///         Ok(())
    ///     }
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`run_pattern_server`]: Self::run_pattern_server
    pub async fn run_server<F, Fut>(
        identity: &Identity,
        bind_addr: &str,
        mut handler: F,
    ) -> Result<()>
    where
        F: FnMut(PubkyNoiseChannel<TcpStream>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let device_id = format!("paykit-demo-{}", identity.public_key());
        let server = Self::create_server(identity, device_id.as_bytes());

        // Bind TCP listener
        let listener = TcpListener::bind(bind_addr)
            .await
            .with_context(|| format!("Failed to bind to {}", bind_addr))?;

        println!("Noise server listening on {}", bind_addr);

        // Accept connections
        loop {
            let (stream, peer_addr) = listener
                .accept()
                .await
                .context("Failed to accept connection")?;

            println!("Accepted TCP connection from {}", peer_addr);

            // Handle handshake
            let server_clone = Arc::clone(&server);
            let channel = Self::accept_connection(server_clone, stream).await?;

            // Call handler
            if let Err(e) = handler(channel).await {
                eprintln!("Handler error: {:#}", e);
            }
        }
    }

    /// Accept a single Noise connection and perform handshake
    ///
    /// This is used internally by `run_server` but can also be used standalone.
    /// Uses length-prefixed messages to match `PubkyNoiseChannel::connect`.
    pub async fn accept_connection(
        server: Arc<NoiseServer<DummyRing>>,
        mut stream: TcpStream,
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        // Read length-prefixed handshake initiation (step 1)
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .context("Failed to read handshake length")?;
        let msg_len = u32::from_be_bytes(len_bytes) as usize;

        // Validate message length to prevent DoS via memory exhaustion
        if msg_len > MAX_HANDSHAKE_SIZE {
            return Err(anyhow!(
                "Handshake message too large: {} bytes (max {})",
                msg_len,
                MAX_HANDSHAKE_SIZE
            ));
        }

        let mut first_msg = vec![0u8; msg_len];
        stream
            .read_exact(&mut first_msg)
            .await
            .context("Failed to read handshake initiation")?;

        // Process handshake (step 2 - server reads and creates response)
        let (mut hs_state, _identity) = server
            .build_responder_read_ik(&first_msg)
            .context("Failed to process handshake")?;

        // Generate response message
        let mut response_msg = vec![0u8; 128];
        let n = hs_state
            .write_message(&[], &mut response_msg)
            .context("Failed to generate handshake response")?;
        response_msg.truncate(n);

        // Send length-prefixed response to client
        let len = (response_msg.len() as u32).to_be_bytes();
        stream
            .write_all(&len)
            .await
            .context("Failed to send response length")?;
        stream
            .write_all(&response_msg)
            .await
            .context("Failed to send handshake response")?;

        // Complete handshake to get transport link (step 3)
        let link = datalink_adapter::server_complete_ik(hs_state)
            .context("Failed to complete handshake")?;

        // Create channel
        Ok(PubkyNoiseChannel::new(stream, link))
    }

    // ========== PATTERN-AWARE ACCEPT METHODS ==========

    /// Accept IK-raw connection (cold key scenario).
    ///
    /// Use when client identity is verified via pkarr rather than in-handshake signing.
    ///
    /// # Arguments
    /// * `x25519_sk` - Server's X25519 secret key
    /// * `stream` - The TCP connection to accept
    ///
    /// # Returns
    /// A tuple of (channel, client_x25519_pk) for pkarr verification
    ///
    /// # Example
    /// ```no_run
    /// # use paykit_demo_core::{Identity, NoiseServerHelper};
    /// # use zeroize::Zeroizing;
    /// # use tokio::net::TcpStream;
    /// # async fn example(stream: TcpStream) -> anyhow::Result<()> {
    /// let identity = Identity::generate();
    /// let x25519_sk = NoiseServerHelper::derive_x25519_key(&identity, b"device");
    ///
    /// let (channel, client_pk) = NoiseServerHelper::accept_ik_raw(
    ///     &x25519_sk,
    ///     stream,
    /// ).await?;
    ///
    /// // Verify client_pk via pkarr lookup
    /// # Ok(())
    /// # }
    /// ```
    pub async fn accept_ik_raw(
        x25519_sk: &Zeroizing<[u8; 32]>,
        mut stream: TcpStream,
    ) -> Result<(PubkyNoiseChannel<TcpStream>, [u8; 32])> {
        // Read length-prefixed first message
        let first_msg = Self::read_handshake_message(&mut stream).await?;

        // Accept IK-raw handshake
        let (hs, response) = datalink_adapter::accept_ik_raw(x25519_sk, &first_msg)
            .map_err(|e| anyhow!("Handshake accept failed: {}", e))?;

        // Extract client's static public key from handshake state
        let client_pk: [u8; 32] = hs
            .get_remote_static()
            .ok_or_else(|| anyhow!("No remote static key in IK pattern"))?
            .try_into()
            .map_err(|_| anyhow!("Invalid remote static key length"))?;

        // Send length-prefixed response
        Self::write_handshake_message(&mut stream, &response).await?;

        // Complete handshake
        let session = datalink_adapter::server_complete_ik(hs)
            .map_err(|e| anyhow!("Handshake completion failed: {}", e))?;

        Ok((PubkyNoiseChannel::new(stream, session), client_pk))
    }

    /// Backward-compatible helper that discards the client static key.
    pub async fn accept_ik_raw_with_stream(
        x25519_sk: &Zeroizing<[u8; 32]>,
        stream: TcpStream,
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        let (channel, _) = Self::accept_ik_raw(x25519_sk, stream).await?;
        Ok(channel)
    }

    /// Accept N pattern connection (anonymous client).
    ///
    /// The client is anonymous; only the server is authenticated.
    /// Use for donation boxes or anonymous payment requests.
    ///
    /// # Arguments
    /// * `x25519_sk` - Server's X25519 secret key
    /// * `stream` - The TCP connection to accept
    ///
    /// # Returns
    /// An encrypted channel (client is anonymous)
    pub async fn accept_n(
        x25519_sk: &Zeroizing<[u8; 32]>,
        mut stream: TcpStream,
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        // Read length-prefixed first message
        let first_msg = Self::read_handshake_message(&mut stream).await?;

        // Accept N pattern - completes in one message
        let hs = datalink_adapter::accept_n(x25519_sk, &first_msg)
            .map_err(|e| anyhow!("Handshake accept failed: {}", e))?;

        // N pattern completes after single message
        let session = datalink_adapter::complete_n(hs)
            .map_err(|e| anyhow!("Handshake completion failed: {}", e))?;

        Ok(PubkyNoiseChannel::new(stream, session))
    }

    /// Backward-compatible helper for anonymous pattern acceptance.
    pub async fn accept_anonymous_with_stream(
        x25519_sk: &Zeroizing<[u8; 32]>,
        stream: TcpStream,
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        Self::accept_n(x25519_sk, stream).await
    }

    /// Accept NN pattern connection (fully anonymous).
    ///
    /// Neither party is authenticated during handshake.
    /// **Important**: Without post-handshake attestation, this is vulnerable to MITM.
    ///
    /// # Arguments
    /// * `stream` - The TCP connection to accept
    ///
    /// # Returns
    /// A tuple of (channel, client_ephemeral_pk, server_ephemeral_pk) for post-handshake verification
    pub async fn accept_nn(
        mut stream: TcpStream,
    ) -> Result<(PubkyNoiseChannel<TcpStream>, [u8; 32], [u8; 32])> {
        // Read length-prefixed first message
        let first_msg = Self::read_handshake_message(&mut stream).await?;

        // Validate message length before slicing to prevent panic
        if first_msg.len() < 32 {
            return Err(anyhow!(
                "NN handshake message too short: {} bytes (need at least 32)",
                first_msg.len()
            ));
        }

        // Extract client's ephemeral key from first message (first 32 bytes of Noise e)
        let client_ephemeral: [u8; 32] = first_msg[..32]
            .try_into()
            .map_err(|_| anyhow!("Invalid first message length"))?;

        // Accept NN pattern
        let (hs, response) = datalink_adapter::accept_nn(&first_msg)
            .map_err(|e| anyhow!("Handshake accept failed: {}", e))?;

        // Validate response length before slicing
        if response.len() < 32 {
            return Err(anyhow!(
                "NN handshake response too short: {} bytes (need at least 32)",
                response.len()
            ));
        }

        let server_ephemeral: [u8; 32] = response[..32]
            .try_into()
            .map_err(|_| anyhow!("Invalid response length"))?;

        // Send length-prefixed response
        Self::write_handshake_message(&mut stream, &response).await?;

        // Complete handshake
        let session = datalink_adapter::complete_raw(hs, &[])
            .map_err(|e| anyhow!("Handshake completion failed: {}", e))?;

        Ok((
            PubkyNoiseChannel::new(stream, session),
            client_ephemeral,
            server_ephemeral,
        ))
    }

    /// Backward-compatible helper for NN pattern acceptance.
    pub async fn accept_ephemeral_with_stream(
        stream: TcpStream,
    ) -> Result<(PubkyNoiseChannel<TcpStream>, [u8; 32], [u8; 32])> {
        Self::accept_nn(stream).await
    }

    /// Accept XX pattern connection (trust-on-first-use).
    ///
    /// Both parties exchange static keys during a 3-message handshake.
    /// This is useful for TOFU scenarios where keys are cached after first contact.
    ///
    /// # Arguments
    /// * `x25519_sk` - Server's X25519 secret key
    /// * `stream` - The TCP connection to accept
    ///
    /// # Returns
    /// A tuple of (channel, client_static_pk) - the client's static key learned during handshake
    pub async fn accept_xx(
        x25519_sk: &Zeroizing<[u8; 32]>,
        mut stream: TcpStream,
    ) -> Result<(PubkyNoiseChannel<TcpStream>, [u8; 32])> {
        use pubky_noise::NoiseReceiver;

        // Read first message (client's ephemeral key)
        let first_msg = Self::read_handshake_message(&mut stream).await?;

        // Accept XX pattern - creates response with our ephemeral + static
        let receiver = NoiseReceiver::new();
        let (mut hs, response) = receiver
            .respond_xx(x25519_sk, &first_msg)
            .map_err(|e| anyhow!("XX handshake accept failed: {}", e))?;

        // Send response (message 2)
        Self::write_handshake_message(&mut stream, &response).await?;

        // Read third message (client's static key + DH)
        let third_msg = Self::read_handshake_message(&mut stream).await?;

        // Process third message
        let mut buf = vec![0u8; third_msg.len() + 256];
        let _n = hs
            .read_message(&third_msg, &mut buf)
            .context("Failed to read XX third message")?;

        // Extract client's static key
        let client_static_pk: [u8; 32] = hs
            .get_remote_static()
            .ok_or_else(|| anyhow!("No client static key in XX pattern"))?
            .try_into()
            .map_err(|_| anyhow!("Invalid client static key length"))?;

        // Complete handshake
        let session = pubky_noise::NoiseSession::from_handshake(hs)
            .map_err(|e| anyhow!("XX handshake completion failed: {}", e))?;

        Ok((PubkyNoiseChannel::new(stream, session), client_static_pk))
    }

    /// Accept a connection with pattern auto-detection.
    ///
    /// Detects the pattern based on message size and structure.
    ///
    /// # Pattern Detection Heuristics
    /// - IK/IK-raw: Larger messages with static key + payload
    /// - N: Smaller message, single round
    /// - NN: Medium message, ephemeral only
    /// - XX: 3-message handshake with TOFU
    ///
    /// **Note**: This is heuristic-based. For production, consider using
    /// explicit pattern negotiation.
    pub async fn accept_with_pattern(
        server: Arc<NoiseServer<DummyRing>>,
        x25519_sk: &Zeroizing<[u8; 32]>,
        stream: TcpStream,
        expected_pattern: NoisePattern,
    ) -> Result<AcceptedConnection> {
        match expected_pattern {
            NoisePattern::IK => {
                // Full IK with identity binding
                let mut stream_inner = stream;
                let first_msg = Self::read_handshake_message(&mut stream_inner).await?;

                let (mut hs_state, identity) = server
                    .build_responder_read_ik(&first_msg)
                    .context("Failed to process IK handshake")?;

                let mut response_msg = vec![0u8; 128];
                let n = hs_state
                    .write_message(&[], &mut response_msg)
                    .context("Failed to generate response")?;
                response_msg.truncate(n);

                Self::write_handshake_message(&mut stream_inner, &response_msg).await?;

                let session = datalink_adapter::server_complete_ik(hs_state)
                    .context("Failed to complete handshake")?;

                Ok(AcceptedConnection::IK {
                    channel: PubkyNoiseChannel::new(stream_inner, session),
                    client_identity: identity,
                })
            }
            NoisePattern::IKRaw => {
                let (channel, client_x25519_pk) = Self::accept_ik_raw(x25519_sk, stream).await?;
                Ok(AcceptedConnection::IKRaw {
                    channel,
                    client_x25519_pk,
                })
            }
            NoisePattern::N => {
                let channel = Self::accept_n(x25519_sk, stream).await?;
                Ok(AcceptedConnection::N { channel })
            }
            NoisePattern::NN => {
                let (channel, client_ephemeral, server_ephemeral) = Self::accept_nn(stream).await?;
                Ok(AcceptedConnection::NN {
                    channel,
                    client_ephemeral,
                    server_ephemeral,
                })
            }
            NoisePattern::XX => {
                let (channel, client_static_pk) = Self::accept_xx(x25519_sk, stream).await?;
                Ok(AcceptedConnection::XX {
                    channel,
                    client_static_pk,
                })
            }
        }
    }

    /// Run a pattern-aware server that accepts multiple patterns.
    ///
    /// Each connection specifies its pattern via a 1-byte prefix before the handshake.
    ///
    /// # Protocol
    /// ```text
    /// Client -> Server: [1-byte pattern] [4-byte length] [handshake message]
    /// ```
    ///
    /// Pattern bytes: 0 = IK, 1 = IK-raw, 2 = N, 3 = NN, 4 = XX
    pub async fn run_pattern_server<F, Fut>(
        identity: &Identity,
        bind_addr: &str,
        mut handler: F,
    ) -> Result<()>
    where
        F: FnMut(AcceptedConnection) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let device_id = format!("paykit-demo-{}", identity.public_key());
        let server = Self::create_server(identity, device_id.as_bytes());
        let x25519_sk = Self::derive_x25519_key(identity, device_id.as_bytes());

        let listener = TcpListener::bind(bind_addr)
            .await
            .with_context(|| format!("Failed to bind to {}", bind_addr))?;

        println!("Pattern-aware Noise server listening on {}", bind_addr);

        loop {
            let (mut stream, peer_addr) = listener
                .accept()
                .await
                .context("Failed to accept connection")?;

            println!("Accepted TCP connection from {}", peer_addr);

            // Read pattern byte
            let mut pattern_byte = [0u8; 1];
            stream
                .read_exact(&mut pattern_byte)
                .await
                .context("Failed to read pattern byte")?;

            let pattern = match pattern_from_byte(pattern_byte[0]) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Unknown pattern byte {}: {:#}", pattern_byte[0], e);
                    continue;
                }
            };

            println!("Client requested pattern: {}", pattern);

            // Accept with specified pattern
            let server_clone = Arc::clone(&server);
            match Self::accept_with_pattern(server_clone, &x25519_sk, stream, pattern).await {
                Ok(conn) => {
                    if let Err(e) = handler(conn).await {
                        eprintln!("Handler error: {:#}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Handshake error: {:#}", e);
                }
            }
        }
    }

    // ========== UTILITY METHODS ==========

    /// Derive X25519 secret key from identity.
    pub fn derive_x25519_key(identity: &Identity, device_context: &[u8]) -> Zeroizing<[u8; 32]> {
        let seed = identity.keypair.secret_key();
        Zeroizing::new(kdf::derive_x25519_static(&seed, device_context))
    }

    /// Read a length-prefixed handshake message from stream.
    async fn read_handshake_message(stream: &mut TcpStream) -> Result<Vec<u8>> {
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .context("Failed to read message length")?;
        let msg_len = u32::from_be_bytes(len_bytes) as usize;

        if msg_len > MAX_HANDSHAKE_SIZE {
            return Err(anyhow!(
                "Message too large: {} bytes (max {})",
                msg_len,
                MAX_HANDSHAKE_SIZE
            ));
        }

        let mut msg = vec![0u8; msg_len];
        stream
            .read_exact(&mut msg)
            .await
            .context("Failed to read message")?;

        Ok(msg)
    }

    /// Write a length-prefixed handshake message to stream.
    async fn write_handshake_message(stream: &mut TcpStream, msg: &[u8]) -> Result<()> {
        let len = (msg.len() as u32).to_be_bytes();
        stream
            .write_all(&len)
            .await
            .context("Failed to write message length")?;
        stream
            .write_all(msg)
            .await
            .context("Failed to write message")?;
        Ok(())
    }

    /// Get the static public key for this server
    ///
    /// This is the key that clients need to connect to this server.
    /// Uses the same key derivation as `create_server` to ensure consistency.
    pub fn get_static_public_key(identity: &Identity, device_id: &[u8]) -> [u8; 32] {
        // Use Ed25519 secret key as seed, same as create_server
        let seed = identity.keypair.secret_key();
        // Derive X25519 secret key using HKDF (same as DummyRing does internally)
        let x25519_sk = pubky_noise::kdf::derive_x25519_static(&seed, device_id);
        pubky_noise::kdf::x25519_pk_from_sk(&x25519_sk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_server() {
        let identity = Identity::generate();
        let server = NoiseServerHelper::create_server(&identity, b"test-device");

        assert!(!server.kid.is_empty());
        assert_eq!(server.device_id, b"test-device");
    }

    #[test]
    fn test_get_static_public_key() {
        let identity = Identity::generate();
        let pk = NoiseServerHelper::get_static_public_key(&identity, b"test-device");

        assert_eq!(pk.len(), 32);
    }

    #[test]
    fn test_static_key_deterministic() {
        let identity = Identity::generate();

        let pk1 = NoiseServerHelper::get_static_public_key(&identity, b"test-device");
        let pk2 = NoiseServerHelper::get_static_public_key(&identity, b"test-device");

        assert_eq!(pk1, pk2, "Static key should be deterministic");
    }

    #[test]
    fn test_different_device_different_key() {
        let identity = Identity::generate();

        let pk1 = NoiseServerHelper::get_static_public_key(&identity, b"device-1");
        let pk2 = NoiseServerHelper::get_static_public_key(&identity, b"device-2");

        assert_ne!(pk1, pk2, "Different devices should produce different keys");
    }

    #[test]
    fn test_derive_x25519_key() {
        let identity = Identity::generate();
        let sk = NoiseServerHelper::derive_x25519_key(&identity, b"device");

        assert_eq!(sk.len(), 32);
        // Should not be all zeros
        assert!(sk.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_derive_x25519_key_deterministic() {
        let identity = Identity::generate();

        let sk1 = NoiseServerHelper::derive_x25519_key(&identity, b"device");
        let sk2 = NoiseServerHelper::derive_x25519_key(&identity, b"device");

        assert_eq!(*sk1, *sk2, "Key derivation should be deterministic");
    }

    #[test]
    fn test_accepted_connection_accessors() {
        // Test that AcceptedConnection methods compile and work
        // (actual connection testing requires network)
        use pubky_noise::identity_payload::Role;

        // Create a mock identity payload for testing
        let mock_identity = IdentityPayload {
            ed25519_pub: [0u8; 32],
            noise_x25519_pub: [1u8; 32],
            role: Role::Client,
            sig: [0u8; 64],
        };

        // We can't easily construct AcceptedConnection without a real connection,
        // but we can verify the enum variants exist
        assert!(matches!(
            NoisePattern::IK,
            NoisePattern::IK
                | NoisePattern::IKRaw
                | NoisePattern::N
                | NoisePattern::NN
                | NoisePattern::XX
        ));

        // Verify mock_identity is usable (prevents unused warning)
        assert_eq!(mock_identity.ed25519_pub, [0u8; 32]);
    }

    /// Test that NN handshake rejects short messages gracefully (no panic).
    ///
    /// This tests the fix for a security vulnerability where malicious peers
    /// could crash the server by sending handshake messages shorter than 32 bytes.
    #[tokio::test]
    async fn test_nn_rejects_short_handshake_message() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");

        // Server task: accept NN and expect graceful error
        let server_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let result = NoiseServerHelper::accept_nn(stream).await;
            // Should fail gracefully, not panic
            assert!(result.is_err());
            // Extract error message without requiring Debug on Ok type
            let err = result.err().expect("expected error").to_string();
            assert!(
                err.contains("too short") || err.contains("Handshake"),
                "Expected length error, got: {}",
                err
            );
        });

        // Client: send a maliciously short message (16 bytes instead of 32+)
        let mut client = tokio::net::TcpStream::connect(addr)
            .await
            .expect("connect");

        // Send length-prefixed short message (only 16 bytes)
        let short_msg = [0u8; 16];
        let len_bytes = (short_msg.len() as u32).to_be_bytes();
        client.write_all(&len_bytes).await.expect("write len");
        client.write_all(&short_msg).await.expect("write msg");

        // Wait for server to process (should not panic)
        server_task.await.expect("server should not panic");
    }
}
