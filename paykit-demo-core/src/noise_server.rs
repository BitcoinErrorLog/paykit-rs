//! Noise protocol server for receiving payments
//!
//! Provides helpers to run a Noise server that accepts payment requests.

use anyhow::{Context, Result};
use paykit_interactive::transport::PubkyNoiseChannel;
use pubky_noise::{datalink_adapter, DummyRing, NoiseServer};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

use crate::Identity;

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
        // Derive X25519 key for Noise from Ed25519 identity
        let x25519_key = identity.derive_x25519_key(device_id, epoch);

        // Create a ring key provider
        let ring = Arc::new(DummyRing::new(
            x25519_key,
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
    pub async fn accept_connection(
        server: Arc<NoiseServer<DummyRing, ()>>,
        mut stream: TcpStream,
    ) -> Result<PubkyNoiseChannel<TcpStream>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Read the client's handshake initiation (step 1)
        let mut first_msg = vec![0u8; 256];
        let n = stream
            .read(&mut first_msg)
            .await
            .context("Failed to read handshake initiation")?;
        first_msg.truncate(n);

        // Process handshake (step 2 - server accepts)
        #[allow(deprecated)]
        let (hs, _payload, response) = datalink_adapter::server_accept_ik(&server, &first_msg)
            .context("Failed to accept handshake")?;

        // Send response to client
        stream
            .write_all(&response)
            .await
            .context("Failed to send handshake response")?;

        // Complete handshake to get transport link (step 3)
        let link =
            datalink_adapter::server_complete_ik(hs).context("Failed to complete handshake")?;

        // Create channel
        Ok(PubkyNoiseChannel::new(stream, link))
    }

    /// Get the static public key for this server
    ///
    /// This is the key that clients need to connect to this server.
    pub fn get_static_public_key(identity: &Identity, device_id: &[u8], epoch: u32) -> [u8; 32] {
        let x25519_key = identity.derive_x25519_key(device_id, epoch);
        pubky_noise::kdf::x25519_pk_from_sk(&x25519_key)
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
