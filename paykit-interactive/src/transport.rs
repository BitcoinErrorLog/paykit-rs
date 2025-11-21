use crate::{InteractiveError, PaykitNoiseChannel, PaykitNoiseMessage, Result};
use async_trait::async_trait;
use pubky_noise::{NoiseClient, NoiseLink, RingKeyProvider};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// A concrete implementation of `PaykitNoiseChannel` using `pubky-noise`.
///
/// It wraps an underlying byte stream (`T`) and handles the Noise protocol encryption/decryption.
pub struct PubkyNoiseChannel<S> {
    stream: S,
    link: NoiseLink,
}

impl<S> PubkyNoiseChannel<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    /// Create a new channel from an established Noise Link and an underlying stream.
    pub fn new(stream: S, link: NoiseLink) -> Self {
        Self { stream, link }
    }

    /// Perform a client-side handshake and return a new channel.
    ///
    /// * `client`: The initialized NoiseClient.
    /// * `stream`: The underlying transport stream (TCP, etc.).
    /// * `server_static_pub`: The server's static public key (32 bytes).
    ///
    /// # Noise_IK Pattern Implementation
    ///
    /// `pubky-noise` uses Noise_IK as a **2-RTT pattern** where:
    /// 1. Client sends `-> e, es, s, ss` (includes identity payload)
    /// 2. Server responds with `<- e, ee, se` (completes handshake)
    /// 3. Both sides can now start encrypting/decrypting transport messages
    ///
    /// This follows the standard Noise_IK pattern which requires completing
    /// the full handshake before entering transport mode.
    pub async fn connect<R: RingKeyProvider>(
        client: &NoiseClient<R, ()>,
        mut stream: S,
        server_static_pub: &[u8; 32],
    ) -> Result<Self> {
        // 1. Build the IK handshake initiation message
        let (hs, _epoch, first_msg) = pubky_noise::datalink_adapter::client_start_ik_direct(
            client,
            server_static_pub,
            0,
            None,
        )
        .map_err(|e| InteractiveError::Transport(format!("Handshake build failed: {}", e)))?;

        // 2. Send the handshake initiation message
        stream
            .write_all(&first_msg)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to send handshake: {}", e)))?;

        // 3. Read server's response message
        let mut response = vec![0u8; 128];
        let n = stream.read(&mut response).await.map_err(|e| {
            InteractiveError::Transport(format!("Failed to read handshake response: {}", e))
        })?;
        response.truncate(n);

        // 4. Complete the handshake
        let link =
            pubky_noise::datalink_adapter::client_complete_ik(hs, &response).map_err(|e| {
                InteractiveError::Transport(format!("Failed to complete handshake: {}", e))
            })?;

        // 5. Channel is now ready for encrypted transport messages
        Ok(Self { stream, link })
    }
}

#[async_trait]
impl<S> PaykitNoiseChannel for PubkyNoiseChannel<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    async fn send(&mut self, msg: PaykitNoiseMessage) -> Result<()> {
        // 1. Serialize message
        let json_bytes =
            serde_json::to_vec(&msg).map_err(|e| InteractiveError::Serialization(e.to_string()))?;

        // 2. Encrypt
        let ciphertext = self
            .link
            .encrypt(&json_bytes)
            .map_err(|e| InteractiveError::Transport(format!("Encryption failed: {}", e)))?;

        // 3. Send length-prefixed
        let len = (ciphertext.len() as u32).to_be_bytes();
        self.stream
            .write_all(&len)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Write failed: {}", e)))?;
        self.stream
            .write_all(&ciphertext)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Write failed: {}", e)))?;

        Ok(())
    }

    async fn recv(&mut self) -> Result<PaykitNoiseMessage> {
        // 1. Read length
        let mut len_bytes = [0u8; 4];
        self.stream
            .read_exact(&mut len_bytes)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Read failed: {}", e)))?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        // 2. Read ciphertext
        let mut ciphertext = vec![0u8; len];
        self.stream
            .read_exact(&mut ciphertext)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Read failed: {}", e)))?;

        // 3. Decrypt
        let plaintext = self
            .link
            .decrypt(&ciphertext)
            .map_err(|e| InteractiveError::Transport(format!("Decryption failed: {}", e)))?;

        // 4. Deserialize
        let msg = serde_json::from_slice(&plaintext)
            .map_err(|e| InteractiveError::Serialization(e.to_string()))?;

        Ok(msg)
    }
}
