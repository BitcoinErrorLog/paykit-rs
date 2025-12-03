use crate::{InteractiveError, PaykitNoiseChannel, PaykitNoiseMessage, Result};
use async_trait::async_trait;
use pubky_noise::{datalink_adapter, NoiseClient, NoiseSession, RingKeyProvider};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use zeroize::Zeroizing;

/// Maximum handshake message size (Noise handshakes are typically <512 bytes)
const MAX_HANDSHAKE_SIZE: usize = 4096;

/// Maximum encrypted message size (16MB should be more than enough for payment messages)
const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// A concrete implementation of `PaykitNoiseChannel` using `pubky-noise`.
///
/// It wraps an underlying byte stream (`T`) and handles the Noise protocol encryption/decryption.
pub struct PubkyNoiseChannel<S> {
    stream: S,
    session: NoiseSession,
}

impl<S> PubkyNoiseChannel<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    /// Create a new channel from an established Noise session and an underlying stream.
    pub fn new(stream: S, session: NoiseSession) -> Self {
        Self { stream, session }
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
        client: &NoiseClient<R>,
        mut stream: S,
        server_static_pub: &[u8; 32],
    ) -> Result<Self> {
        // 1. Build the IK handshake initiation message
        let (hs, first_msg) = pubky_noise::datalink_adapter::client_start_ik_direct(
            client,
            server_static_pub,
        )
        .map_err(|e| InteractiveError::Transport(format!("Handshake build failed: {}", e)))?;

        // 2. Send length-prefixed handshake initiation message
        let len = (first_msg.len() as u32).to_be_bytes();
        stream
            .write_all(&len)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to send handshake length: {}", e)))?;
        stream
            .write_all(&first_msg)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to send handshake: {}", e)))?;

        // 3. Read length-prefixed server response
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to read response length: {}", e)))?;
        let response_len = u32::from_be_bytes(len_bytes) as usize;

        // Validate response length to prevent DoS via memory exhaustion
        if response_len > MAX_HANDSHAKE_SIZE {
            return Err(InteractiveError::Transport(format!(
                "Handshake response too large: {} bytes (max {})",
                response_len, MAX_HANDSHAKE_SIZE
            )));
        }

        let mut response = vec![0u8; response_len];
        stream
            .read_exact(&mut response)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to read handshake response: {}", e)))?;

        // 4. Complete the handshake
        let session =
            pubky_noise::datalink_adapter::client_complete_ik(hs, &response).map_err(|e| {
                InteractiveError::Transport(format!("Failed to complete handshake: {}", e))
            })?;

        // 5. Channel is now ready for encrypted transport messages
        Ok(Self { stream, session })
    }

    /// Connect using IK-raw pattern (cold key scenario).
    ///
    /// Identity is verified externally via pkarr, not during the handshake.
    /// Use this when Ed25519 keys are kept cold and identity binding is
    /// provided through pkarr records.
    ///
    /// # Arguments
    /// * `x25519_sk` - Your X25519 secret key (derived from Ed25519 seed)
    /// * `stream` - The underlying transport stream (TCP, etc.)
    /// * `server_static_pub` - The server's X25519 public key (from pkarr)
    ///
    /// # Noise_IK Pattern (without identity binding)
    ///
    /// This uses the same IK wire format but without the Ed25519 signature
    /// in the handshake payload. Identity must be verified via pkarr lookup.
    pub async fn connect_ik_raw(
        x25519_sk: &Zeroizing<[u8; 32]>,
        mut stream: S,
        server_static_pub: &[u8; 32],
    ) -> Result<Self> {
        // 1. Start IK-raw handshake
        let (hs, first_msg) = datalink_adapter::start_ik_raw(x25519_sk, server_static_pub)
            .map_err(|e| InteractiveError::Transport(format!("Handshake init failed: {}", e)))?;

        // 2. Send length-prefixed first message
        let len = (first_msg.len() as u32).to_be_bytes();
        stream
            .write_all(&len)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to send handshake length: {}", e)))?;
        stream
            .write_all(&first_msg)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to send handshake: {}", e)))?;

        // 3. Read length-prefixed server response
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to read response length: {}", e)))?;
        let response_len = u32::from_be_bytes(len_bytes) as usize;

        if response_len > MAX_HANDSHAKE_SIZE {
            return Err(InteractiveError::Transport(format!(
                "Handshake response too large: {} bytes (max {})",
                response_len, MAX_HANDSHAKE_SIZE
            )));
        }

        let mut response = vec![0u8; response_len];
        stream
            .read_exact(&mut response)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to read response: {}", e)))?;

        // 4. Complete handshake
        let session = datalink_adapter::complete_raw(hs, &response)
            .map_err(|e| InteractiveError::Transport(format!("Handshake completion failed: {}", e)))?;

        Ok(Self { stream, session })
    }

    /// Connect using N pattern (anonymous client, authenticated server).
    ///
    /// The client is anonymous (uses only ephemeral keys), while the server
    /// is authenticated via its static X25519 key (verified through pkarr).
    /// Use for anonymous payment requests like donation boxes.
    ///
    /// # Arguments
    /// * `stream` - The underlying transport stream (TCP, etc.)
    /// * `server_static_pub` - The server's X25519 public key (from pkarr)
    ///
    /// # Noise_N Pattern
    ///
    /// This is a one-way pattern:
    /// 1. Client sends `-> e, es` (single message)
    /// 2. Channel enters transport mode immediately
    ///
    /// Note: The client cannot be identified by the server.
    pub async fn connect_anonymous(
        mut stream: S,
        server_static_pub: &[u8; 32],
    ) -> Result<Self> {
        // 1. Start N pattern handshake
        let (hs, first_msg) = datalink_adapter::start_n(server_static_pub)
            .map_err(|e| InteractiveError::Transport(format!("Handshake init failed: {}", e)))?;

        // 2. Send length-prefixed first message
        let len = (first_msg.len() as u32).to_be_bytes();
        stream
            .write_all(&len)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to send handshake length: {}", e)))?;
        stream
            .write_all(&first_msg)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to send handshake: {}", e)))?;

        // 3. N pattern completes in one message
        let session = NoiseSession::from_handshake(hs)
            .map_err(|e| InteractiveError::Transport(format!("Handshake completion failed: {}", e)))?;

        Ok(Self { stream, session })
    }

    /// Connect using NN pattern (fully anonymous).
    ///
    /// Neither party has a static key - purely ephemeral key exchange.
    /// **Important**: Without post-handshake attestation, this is vulnerable to MITM.
    ///
    /// # Arguments
    /// * `stream` - The underlying transport stream (TCP, etc.)
    ///
    /// # Returns
    /// A tuple of (channel, server_ephemeral_pk) for post-handshake verification.
    /// The caller should implement attestation (e.g., server signs a challenge).
    ///
    /// # Noise_NN Pattern
    ///
    /// Two-message pattern:
    /// 1. Client sends `-> e`
    /// 2. Server responds `<- e, ee`
    /// 3. Both sides enter transport mode
    ///
    /// # Security Warning
    ///
    /// NN provides forward secrecy but NO authentication. You MUST implement
    /// post-handshake attestation to prevent MITM attacks.
    pub async fn connect_ephemeral(mut stream: S) -> Result<(Self, [u8; 32])> {
        // 1. Start NN pattern handshake
        let (hs, first_msg) = datalink_adapter::start_nn()
            .map_err(|e| InteractiveError::Transport(format!("Handshake init failed: {}", e)))?;

        // 2. Send length-prefixed first message
        let len = (first_msg.len() as u32).to_be_bytes();
        stream
            .write_all(&len)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to send handshake length: {}", e)))?;
        stream
            .write_all(&first_msg)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to send handshake: {}", e)))?;

        // 3. Read length-prefixed server response
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to read response length: {}", e)))?;
        let response_len = u32::from_be_bytes(len_bytes) as usize;

        if response_len > MAX_HANDSHAKE_SIZE {
            return Err(InteractiveError::Transport(format!(
                "Handshake response too large: {} bytes (max {})",
                response_len, MAX_HANDSHAKE_SIZE
            )));
        }

        let mut response = vec![0u8; response_len];
        stream
            .read_exact(&mut response)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to read response: {}", e)))?;

        // 4. Extract server's ephemeral key from response (first 32 bytes of NN pattern)
        let server_ephemeral: [u8; 32] = response
            .get(..32)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| InteractiveError::Transport("Invalid response length for NN pattern".to_string()))?;

        // 5. Complete handshake
        let session = datalink_adapter::complete_raw(hs, &response)
            .map_err(|e| InteractiveError::Transport(format!("Handshake completion failed: {}", e)))?;

        Ok((Self { stream, session }, server_ephemeral))
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
            .session
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

        // Validate message length to prevent DoS via memory exhaustion
        if len > MAX_MESSAGE_SIZE {
            return Err(InteractiveError::Transport(format!(
                "Message too large: {} bytes (max {})",
                len, MAX_MESSAGE_SIZE
            )));
        }

        // 2. Read ciphertext
        let mut ciphertext = vec![0u8; len];
        self.stream
            .read_exact(&mut ciphertext)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Read failed: {}", e)))?;

        // 3. Decrypt
        let plaintext = self
            .session
            .decrypt(&ciphertext)
            .map_err(|e| InteractiveError::Transport(format!("Decryption failed: {}", e)))?;

        // 4. Deserialize
        let msg = serde_json::from_slice(&plaintext)
            .map_err(|e| InteractiveError::Serialization(e.to_string()))?;

        Ok(msg)
    }
}
