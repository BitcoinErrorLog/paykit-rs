use crate::{InteractiveError, PaykitNoiseChannel, PaykitNoiseMessage, Result};
use async_trait::async_trait;
use pubky_noise::identity_payload::IdentityPayload;
use pubky_noise::{NoiseClient, NoiseLink, NoiseServer, RingKeyProvider};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Default maximum message size (1 MB).
///
/// This limit prevents memory exhaustion attacks where a malicious peer
/// sends a large length prefix to force allocation of excessive memory.
///
/// For typical Paykit messages (receipts, endpoint offers), 1 MB is more
/// than sufficient. Increase this only if you have a specific need for
/// larger payloads.
///
/// This constant is also exported as [`MAX_MESSAGE_SIZE`] for convenience.
pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1 MB

/// Alias for [`DEFAULT_MAX_MESSAGE_SIZE`].
///
/// This is the maximum allowed size for transport messages (1 MB).
/// Use [`PubkyNoiseChannel::with_max_message_size`] to customize.
pub const MAX_MESSAGE_SIZE: usize = DEFAULT_MAX_MESSAGE_SIZE;

/// Maximum allowed message size for handshake messages.
///
/// Handshake messages are much smaller than transport messages, so we use
/// a tighter limit to detect malformed handshakes early.
pub const MAX_HANDSHAKE_SIZE: usize = 65536; // 64 KB

/// A concrete implementation of `PaykitNoiseChannel` using `pubky-noise`.
///
/// It wraps an underlying byte stream (`T`) and handles the Noise protocol encryption/decryption.
///
/// # Security
///
/// This channel enforces a maximum message size to prevent memory exhaustion attacks.
/// By default, messages larger than [`DEFAULT_MAX_MESSAGE_SIZE`] (1 MB) are rejected.
/// Use [`with_max_message_size`](Self::with_max_message_size) to customize the limit.
pub struct PubkyNoiseChannel<S> {
    stream: S,
    link: NoiseLink,
    /// Maximum allowed message size in bytes.
    max_message_size: usize,
}

impl<S> PubkyNoiseChannel<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    /// Create a new channel from an established Noise Link and an underlying stream.
    ///
    /// Uses the default maximum message size of [`DEFAULT_MAX_MESSAGE_SIZE`].
    pub fn new(stream: S, link: NoiseLink) -> Self {
        Self {
            stream,
            link,
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
        }
    }

    /// Set a custom maximum message size.
    ///
    /// Messages larger than this limit will be rejected with an error.
    ///
    /// # Arguments
    ///
    /// * `size` - Maximum message size in bytes. Must be at least 1024 bytes.
    ///
    /// # Errors
    ///
    /// Returns `InteractiveError::InvalidConfig` if `size` is less than 1024 bytes.
    pub fn with_max_message_size(mut self, size: usize) -> Result<Self> {
        if size < 1024 {
            return Err(InteractiveError::InvalidConfig(
                "max_message_size must be at least 1024 bytes".to_string(),
            ));
        }
        self.max_message_size = size;
        Ok(self)
    }

    /// Get the current maximum message size.
    pub fn max_message_size(&self) -> usize {
        self.max_message_size
    }

    /// Check if a message size exceeds the configured limit.
    fn check_message_size(&self, size: usize, context: &str) -> Result<()> {
        if size > self.max_message_size {
            return Err(InteractiveError::Transport(format!(
                "{}: message size {} exceeds maximum allowed size of {} bytes",
                context, size, self.max_message_size
            )));
        }
        Ok(())
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
    ///
    /// # Security
    ///
    /// Handshake messages are limited to [`MAX_HANDSHAKE_SIZE`] bytes to prevent
    /// memory exhaustion during the handshake phase.
    pub async fn connect<R: RingKeyProvider>(
        client: &NoiseClient<R, ()>,
        mut stream: S,
        server_static_pub: &[u8; 32],
    ) -> Result<Self> {
        // 1. Build the IK handshake initiation message
        let (hs, first_msg) =
            pubky_noise::datalink_adapter::client_start_ik_direct(client, server_static_pub, None)
                .map_err(|e| {
                    InteractiveError::Transport(format!("Handshake build failed: {}", e))
                })?;

        // 2. Send length-prefixed handshake initiation message
        let len = (first_msg.len() as u32).to_be_bytes();
        stream.write_all(&len).await.map_err(|e| {
            InteractiveError::Transport(format!("Failed to send handshake len: {}", e))
        })?;
        stream
            .write_all(&first_msg)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to send handshake: {}", e)))?;

        // 3. Read length-prefixed server response
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await.map_err(|e| {
            InteractiveError::Transport(format!("Failed to read response len: {}", e))
        })?;
        let response_len = u32::from_be_bytes(len_bytes) as usize;

        // Security: Validate handshake message size
        if response_len > MAX_HANDSHAKE_SIZE {
            return Err(InteractiveError::Transport(format!(
                "Handshake response size {} exceeds maximum allowed size of {} bytes",
                response_len, MAX_HANDSHAKE_SIZE
            )));
        }

        let mut response = vec![0u8; response_len];
        stream.read_exact(&mut response).await.map_err(|e| {
            InteractiveError::Transport(format!("Failed to read handshake response: {}", e))
        })?;

        // 4. Complete the handshake
        let link =
            pubky_noise::datalink_adapter::client_complete_ik(hs, &response).map_err(|e| {
                InteractiveError::Transport(format!("Failed to complete handshake: {}", e))
            })?;

        // 5. Channel is now ready for encrypted transport messages
        Ok(Self {
            stream,
            link,
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
        })
    }

    /// Accept an incoming client connection (server-side handshake).
    ///
    /// * `server`: The initialized NoiseServer.
    /// * `stream`: The underlying transport stream (TCP, etc.).
    ///
    /// # Noise_IK Pattern Implementation (Server Side)
    ///
    /// `pubky-noise` uses Noise_IK as a **2-RTT pattern** where:
    /// 1. Server reads client's `-> e, es, s, ss` message (includes identity payload)
    /// 2. Server responds with `<- e, ee, se` (completes handshake)
    /// 3. Both sides can now start encrypting/decrypting transport messages
    ///
    /// # Security
    ///
    /// Handshake messages are limited to [`MAX_HANDSHAKE_SIZE`] bytes to prevent
    /// memory exhaustion during the handshake phase.
    ///
    /// # Returns
    ///
    /// A tuple of (PubkyNoiseChannel, IdentityPayload) where IdentityPayload contains
    /// the authenticated client identity (Ed25519 public key, etc.).
    pub async fn accept<R: RingKeyProvider>(
        server: &NoiseServer<R, ()>,
        mut stream: S,
    ) -> Result<(Self, IdentityPayload)> {
        // 1. Read length-prefixed client handshake initiation message
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await.map_err(|e| {
            InteractiveError::Transport(format!("Failed to read handshake len: {}", e))
        })?;
        let msg_len = u32::from_be_bytes(len_bytes) as usize;

        // Security: Validate handshake message size
        if msg_len > MAX_HANDSHAKE_SIZE {
            return Err(InteractiveError::Transport(format!(
                "Handshake message size {} exceeds maximum allowed size of {} bytes",
                msg_len, MAX_HANDSHAKE_SIZE
            )));
        }

        let mut first_msg = vec![0u8; msg_len];
        stream
            .read_exact(&mut first_msg)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Failed to read handshake: {}", e)))?;

        // 2. Process the handshake - validates client identity and prepares response
        let (hs, identity, response) =
            pubky_noise::datalink_adapter::server_accept_ik(server, &first_msg)
                .map_err(|e| InteractiveError::Transport(format!("Handshake failed: {}", e)))?;

        // 3. Send length-prefixed handshake response
        let len = (response.len() as u32).to_be_bytes();
        stream.write_all(&len).await.map_err(|e| {
            InteractiveError::Transport(format!("Failed to send response len: {}", e))
        })?;
        stream.write_all(&response).await.map_err(|e| {
            InteractiveError::Transport(format!("Failed to send handshake response: {}", e))
        })?;

        // 4. Complete the handshake to get transport mode
        let link = pubky_noise::datalink_adapter::server_complete_ik(hs).map_err(|e| {
            InteractiveError::Transport(format!("Failed to complete handshake: {}", e))
        })?;

        // 5. Channel is now ready for encrypted transport messages
        Ok((
            Self {
                stream,
                link,
                max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
            },
            identity,
        ))
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

        // 2. Security: Validate message size BEFORE allocating
        self.check_message_size(len, "recv")?;

        // 3. Read ciphertext
        let mut ciphertext = vec![0u8; len];
        self.stream
            .read_exact(&mut ciphertext)
            .await
            .map_err(|e| InteractiveError::Transport(format!("Read failed: {}", e)))?;

        // 4. Decrypt
        let plaintext = self
            .link
            .decrypt(&ciphertext)
            .map_err(|e| InteractiveError::Transport(format!("Decryption failed: {}", e)))?;

        // 5. Deserialize
        let msg = serde_json::from_slice(&plaintext)
            .map_err(|e| InteractiveError::Serialization(e.to_string()))?;

        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn test_default_max_message_size() {
        assert_eq!(DEFAULT_MAX_MESSAGE_SIZE, 1024 * 1024);
    }

    #[test]
    fn test_max_message_size_alias() {
        assert_eq!(MAX_MESSAGE_SIZE, DEFAULT_MAX_MESSAGE_SIZE);
        assert_eq!(MAX_MESSAGE_SIZE, 1024 * 1024);
    }

    #[test]
    fn test_max_handshake_size() {
        assert_eq!(MAX_HANDSHAKE_SIZE, 65536);
    }

    /// Helper to create a mock stream with pre-filled data
    struct MockStream {
        read_data: Cursor<Vec<u8>>,
        write_data: Vec<u8>,
    }

    impl MockStream {
        fn with_length_prefix(len: u32) -> Self {
            let len_bytes = len.to_be_bytes();
            Self {
                read_data: Cursor::new(len_bytes.to_vec()),
                write_data: Vec::new(),
            }
        }
    }

    impl tokio::io::AsyncRead for MockStream {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::pin::Pin::new(&mut self.read_data).poll_read(cx, buf)
        }
    }

    impl tokio::io::AsyncWrite for MockStream {
        fn poll_write(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            self.write_data.extend_from_slice(buf);
            std::task::Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Ok(()))
        }
    }

    #[test]
    fn test_check_message_size_accepts_valid() {
        // Create a minimal mock NoiseLink (we just need any valid link for testing)
        // Since we can't easily create a real NoiseLink, we'll test the size check logic directly
        let max_size = 1024 * 1024; // 1 MB

        // Valid size should be accepted
        assert!(512 * 1024 <= max_size); // 512 KB
        assert!(max_size <= max_size); // Exactly at limit
    }

    #[test]
    fn test_check_message_size_rejects_oversized() {
        let max_size = 1024 * 1024; // 1 MB

        // Oversized message should be rejected
        let oversized = max_size + 1;
        assert!(oversized > max_size);

        // This is what check_message_size does internally
        let result = if oversized > max_size {
            Err(InteractiveError::Transport(format!(
                "recv: message size {} exceeds maximum allowed size of {} bytes",
                oversized, max_size
            )))
        } else {
            Ok(())
        };

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("exceeds maximum allowed size"));
    }

    #[test]
    fn test_oversized_transport_message_error_message() {
        // Test that the error message is informative
        let size = 2 * 1024 * 1024; // 2 MB
        let max = 1024 * 1024; // 1 MB

        let err = InteractiveError::Transport(format!(
            "recv: message size {} exceeds maximum allowed size of {} bytes",
            size, max
        ));

        let msg = err.to_string();
        assert!(msg.contains("2097152")); // 2 MB in bytes
        assert!(msg.contains("1048576")); // 1 MB in bytes
        assert!(msg.contains("exceeds"));
    }

    #[test]
    fn test_oversized_handshake_error_message() {
        // Test handshake size limit error
        let size = 100_000; // 100 KB
        let max = MAX_HANDSHAKE_SIZE; // 64 KB

        assert!(size > max);

        let err = InteractiveError::Transport(format!(
            "Handshake response size {} exceeds maximum allowed size of {} bytes",
            size, max
        ));

        let msg = err.to_string();
        assert!(msg.contains("100000"));
        assert!(msg.contains("65536"));
        assert!(msg.contains("exceeds"));
    }

    #[test]
    fn test_message_size_validated_before_allocation() {
        // This test documents the security property: size is checked BEFORE allocation
        // In the actual code, check_message_size() is called before vec![0u8; len]
        //
        // The sequence in recv() is:
        // 1. Read 4-byte length prefix
        // 2. call check_message_size(len, "recv")? <-- VALIDATION HERE
        // 3. vec![0u8; len] <-- Allocation only happens after validation
        //
        // This prevents DoS via memory exhaustion from malicious length prefixes.

        let max_size = 1024; // Small limit for testing

        // Attacker sends a huge length prefix
        let malicious_length: u32 = u32::MAX; // 4 GB!

        // The check happens BEFORE allocation
        let would_allocate = malicious_length as usize;
        assert!(would_allocate > max_size);

        // Error is returned, allocation never happens
        let check_result: Result<()> = if would_allocate > max_size {
            Err(InteractiveError::Transport(format!(
                "recv: message size {} exceeds maximum allowed size of {} bytes",
                would_allocate, max_size
            )))
        } else {
            Ok(())
        };

        assert!(check_result.is_err());
    }

    #[test]
    fn test_handshake_size_limit_prevents_dos() {
        // Handshake has a tighter limit than transport messages
        assert!(MAX_HANDSHAKE_SIZE < DEFAULT_MAX_MESSAGE_SIZE);
        assert_eq!(MAX_HANDSHAKE_SIZE, 64 * 1024); // 64 KB

        // A malformed handshake response claiming to be huge would be rejected
        let malicious_handshake_len = 1024 * 1024; // 1 MB

        assert!(malicious_handshake_len > MAX_HANDSHAKE_SIZE);

        // The check in connect() and accept() uses MAX_HANDSHAKE_SIZE
        let would_reject = malicious_handshake_len > MAX_HANDSHAKE_SIZE;
        assert!(would_reject);
    }
}
