//! Noise protocol server for receiving payments
//!
//! Provides helpers to run a Noise server that accepts payment requests.

use anyhow::{anyhow, Context, Result};
use paykit_interactive::transport::PubkyNoiseChannel;
use pubky_noise::{datalink_adapter, DummyRing, NoiseServer};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

use crate::Identity;

/// Maximum handshake message size (Noise handshakes are typically <512 bytes)
const MAX_HANDSHAKE_SIZE: usize = 4096;

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
    /// * `epoch` - The epoch number for key rotation (use 0 for simple cases)
    ///
    /// # Example
    /// ```no_run
    /// # use paykit_demo_core::{Identity, NoiseServerHelper};
    /// # fn example() -> anyhow::Result<()> {
    /// let identity = Identity::generate();
    /// let server = NoiseServerHelper::create_server(&identity, b"my-device", 0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_server(
        identity: &Identity,
        device_id: &[u8],
        epoch: u32,
    ) -> Arc<NoiseServer<DummyRing, ()>> {
        // Use Ed25519 secret key as the seed for DummyRing
        // DummyRing will derive X25519 keys from this seed using HKDF
        let seed = identity.keypair.secret_key();

        // Create a ring key provider
        let ring = Arc::new(DummyRing::new(
            seed,
            identity.public_key().to_string(),
        ));

        // Create the Noise server
        Arc::new(NoiseServer::new_direct(
            identity.public_key().to_string(),
            device_id,
            ring,
            epoch,
        ))
    }

    /// Run a Noise server and handle incoming connections
    ///
    /// This binds to the specified address and accepts incoming Noise connections.
    /// For each connection, the provided handler is called with the established channel.
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
        let server = Self::create_server(identity, device_id.as_bytes(), 0);

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
        server: Arc<NoiseServer<DummyRing, ()>>,
        mut stream: TcpStream,
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
        let link =
            datalink_adapter::server_complete_ik(hs_state).context("Failed to complete handshake")?;

        // Create channel
        Ok(PubkyNoiseChannel::new(stream, link))
    }

    /// Get the static public key for this server
    ///
    /// This is the key that clients need to connect to this server.
    /// Uses the same key derivation as `create_server` to ensure consistency.
    pub fn get_static_public_key(identity: &Identity, device_id: &[u8], epoch: u32) -> [u8; 32] {
        // Use Ed25519 secret key as seed, same as create_server
        let seed = identity.keypair.secret_key();
        // Derive X25519 secret key using HKDF (same as DummyRing does internally)
        let x25519_sk = pubky_noise::kdf::derive_x25519_for_device_epoch(&seed, device_id, epoch);
        pubky_noise::kdf::x25519_pk_from_sk(&x25519_sk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_server() {
        let identity = Identity::generate();
        let server = NoiseServerHelper::create_server(&identity, b"test-device", 0);

        assert!(!server.kid.is_empty());
        assert_eq!(server.device_id, b"test-device");
        assert_eq!(server.current_epoch, 0);
    }

    #[test]
    fn test_get_static_public_key() {
        let identity = Identity::generate();
        let pk = NoiseServerHelper::get_static_public_key(&identity, b"test-device", 0);

        assert_eq!(pk.len(), 32);
    }

    #[test]
    fn test_static_key_deterministic() {
        let identity = Identity::generate();

        let pk1 = NoiseServerHelper::get_static_public_key(&identity, b"test-device", 0);
        let pk2 = NoiseServerHelper::get_static_public_key(&identity, b"test-device", 0);

        assert_eq!(pk1, pk2, "Static key should be deterministic");
    }

    #[test]
    fn test_different_epoch_different_key() {
        let identity = Identity::generate();

        let pk1 = NoiseServerHelper::get_static_public_key(&identity, b"test-device", 0);
        let pk2 = NoiseServerHelper::get_static_public_key(&identity, b"test-device", 1);

        assert_ne!(pk1, pk2, "Different epochs should produce different keys");
    }
}
