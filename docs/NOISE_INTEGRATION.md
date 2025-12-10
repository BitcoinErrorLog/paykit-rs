# Pubky Noise Integration Guide

This document explains how to integrate `pubky-noise` with `paykit-interactive` to establish secure, authenticated payment channels.

## Overview

`paykit-interactive` uses `pubky-noise` to provide end-to-end encrypted communication for private payment negotiation. The integration uses the **Noise_IK** handshake pattern, which provides:

- **Mutual Authentication**: Both peers verify each other's Ed25519 identities
- **Forward Secrecy**: X25519 ephemeral keys ensure past sessions can't be decrypted
- **Encryption**: ChaCha20-Poly1305 authenticated encryption for all messages

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                      Application Layer                        │
│  PaykitInteractiveManager, PaykitReceipt, PaykitNoiseMessage │
├──────────────────────────────────────────────────────────────┤
│                      Channel Abstraction                      │
│         PubkyNoiseChannel<S> implements PaykitNoiseChannel   │
├──────────────────────────────────────────────────────────────┤
│                      Noise Protocol Layer                     │
│     NoiseClient / NoiseServer + RingKeyProvider trait        │
├──────────────────────────────────────────────────────────────┤
│                      Transport Layer                          │
│             TCP, WebSocket, or any AsyncRead+AsyncWrite      │
└──────────────────────────────────────────────────────────────┘
```

## Quick Start

### Client Side (Payer)

```rust
use paykit_interactive::transport::PubkyNoiseChannel;
use pubky_noise::{NoiseClient, RingKeyProvider};
use tokio::net::TcpStream;
use std::sync::Arc;

// 1. Implement RingKeyProvider for your key management system
struct MyKeyProvider { /* ... */ }
impl RingKeyProvider for MyKeyProvider {
    fn derive_device_x25519(&self, kid: &str, device_id: &[u8], epoch: u32) 
        -> Result<[u8; 32], pubky_noise::NoiseError> {
        // Derive X25519 key from your identity
        Ok(pubky_noise::kdf::derive_x25519_for_device_epoch(&self.seed, device_id, 0))
    }
    
    fn ed25519_pubkey(&self, kid: &str) -> Result<[u8; 32], pubky_noise::NoiseError> {
        // Return your Ed25519 public key
        Ok(self.keypair.public_key().as_bytes().clone())
    }
    
    fn sign_ed25519(&self, kid: &str, msg: &[u8]) -> Result<[u8; 64], pubky_noise::NoiseError> {
        // Sign with your Ed25519 private key
        Ok(self.keypair.sign(msg).to_bytes())
    }
}

// 2. Create Noise client
let ring = Arc::new(MyKeyProvider::new(/* ... */));
let client = NoiseClient::<MyKeyProvider, ()>::new_direct(
    "my_key_id",
    b"device_identifier",
    ring,
);

// 3. Connect to server
let stream = TcpStream::connect("payee.example.com:9000").await?;
let server_static_pub: [u8; 32] = /* server's X25519 public key */;

let mut channel = PubkyNoiseChannel::connect(&client, stream, &server_static_pub).await?;

// 4. Exchange encrypted messages
channel.send(PaykitNoiseMessage::RequestReceipt { /* ... */ }).await?;
let response = channel.recv().await?;
```

### Server Side (Payee)

```rust
use paykit_interactive::transport::PubkyNoiseChannel;
use pubky_noise::{NoiseServer, RingKeyProvider};
use tokio::net::TcpListener;
use std::sync::Arc;

// 1. Create Noise server
let ring = Arc::new(MyKeyProvider::new(/* ... */));
let server = Arc::new(NoiseServer::<MyKeyProvider, ()>::new_direct(
    "server_key_id",
    b"server_device_id",
    ring,
));

// 2. Accept connections
let listener = TcpListener::bind("0.0.0.0:9000").await?;

loop {
    let (stream, addr) = listener.accept().await?;
    let server = server.clone();
    
    tokio::spawn(async move {
        // 3. Accept connection with Noise handshake
        let (mut channel, client_identity) = PubkyNoiseChannel::accept(&server, stream)
            .await
            .expect("Handshake failed");
        
        // 4. Verify client identity (optional additional checks)
        println!("Accepted connection from: {:?}", client_identity.ed25519_pub);
        
        // 5. Handle encrypted messages
        let msg = channel.recv().await?;
        // Process payment request...
        channel.send(PaykitNoiseMessage::ConfirmReceipt { /* ... */ }).await?;
    });
}
```

## Noise_IK Handshake Flow

The handshake completes in 2 round-trips (4 messages total):

```
Client                                        Server
   |                                             |
   |  -> e, es, s, ss [IdentityPayload]          |  Step 1: Client sends ephemeral key,
   |         (first_msg)                         |          static key, and identity
   |                                             |
   |              <- e, ee, se                   |  Step 2: Server responds with
   |                (response)                   |          ephemeral key
   |                                             |
   |  [Both derive transport keys]               |  Step 3: Both complete handshake
   |                                             |
   |  ============ ENCRYPTED CHANNEL =========== |
   |                                             |
   |  -> PaykitNoiseMessage (encrypted)          |  Step 4+: Application messages
   |  <- PaykitNoiseMessage (encrypted)          |
```

### Message Format

During handshake:
- Messages are sent/received as raw bytes (no length prefix)
- The `connect()` and `accept()` methods handle this automatically

After handshake:
- Messages are length-prefixed (4 bytes, big-endian)
- Ciphertext includes ChaCha20-Poly1305 authentication tag

## Key Derivation

The `RingKeyProvider` trait abstracts key management. The library uses:

1. **Ed25519** for identity binding and signatures
2. **X25519** for Diffie-Hellman key exchange

X25519 keys are derived from Ed25519 seeds using HKDF:

```rust
// Internal key derivation (for reference)
fn derive_x25519_for_device_epoch(seed: &[u8; 32], device_id: &[u8], epoch: u32) -> [u8; 32] {
    // HKDF with device_id and epoch bound into the info parameter
    // Returns X25519 private key
}
```

**Note**: Key rotation should be handled at the application level using device IDs, key identifiers, or full identity rotation.

## Security Considerations

### Rate Limiting

The Noise handshake is computationally cheap for attackers. Use the built-in rate limiter:

```rust
use paykit_interactive::rate_limit::{HandshakeRateLimiter, RateLimitConfig};
use std::sync::Arc;

// Create rate limiter with default config (10 attempts per 60 seconds per IP)
let limiter = HandshakeRateLimiter::new_shared(RateLimitConfig::default());

// Or use predefined configs:
// let limiter = HandshakeRateLimiter::new_shared(RateLimitConfig::strict());  // 3/min
// let limiter = HandshakeRateLimiter::new_shared(RateLimitConfig::relaxed()); // 100/min

// In your accept loop:
let (stream, addr) = listener.accept().await?;
if !limiter.check_and_record(addr.ip()) {
    eprintln!("Rate limit exceeded for {}", addr.ip());
    continue; // Drop connection
}

// Proceed with Noise handshake
let (channel, identity) = PubkyNoiseChannel::accept(&server, stream).await?;

// Optional: reset limits after successful authentication
limiter.reset(addr.ip());
```

Configuration options:

```rust
let config = RateLimitConfig {
    max_attempts_per_ip: 10,           // Max handshakes per IP in window
    window: Duration::from_secs(60),   // Time window
    max_tracked_ips: 10_000,           // Memory limit
};
```

### Client Identity Verification

After accepting a connection, verify the client's identity:

```rust
let (channel, identity) = PubkyNoiseChannel::accept(&server, stream).await?;

// Verify client is who they claim to be
if !is_authorized_client(&identity.ed25519_pub) {
    return Err("Unauthorized client");
}

// Optional: Check server_hint matches expected routing
if let Some(hint) = &identity.server_hint {
    if hint != expected_hint {
        return Err("Unexpected routing hint");
    }
}
```

## WebSocket Transport

For browser-based clients, use WebSocket transport:

```rust
// See paykit-demo-web for full example
use paykit_demo_web::websocket_transport::WebSocketNoiseChannel;

let channel = WebSocketNoiseChannel::connect(&client, ws_url, &server_pk).await?;
```

## Troubleshooting

### Handshake Failures

1. **"Handshake build failed"**: Check that the server's static public key is correct
2. **"Identity verification failed"**: Ensure the client's Ed25519 signature is valid
3. **"Invalid peer key"**: The peer sent an all-zero public key (potential attack)

### Decryption Failures

1. **"Decryption failed"**: Check nonce synchronization (messages must be sent in order)
2. **"Read failed"**: Connection dropped or corrupted

## API Reference

### `PubkyNoiseChannel<S>`

```rust
impl<S: AsyncRead + AsyncWrite + Unpin + Send> PubkyNoiseChannel<S> {
    /// Create channel from existing NoiseLink
    pub fn new(stream: S, link: NoiseLink) -> Self;
    
    /// Client-side: connect to server
    pub async fn connect<R: RingKeyProvider>(
        client: &NoiseClient<R, ()>,
        stream: S,
        server_static_pub: &[u8; 32],
    ) -> Result<Self>;
    
    /// Server-side: accept incoming connection
    pub async fn accept<R: RingKeyProvider>(
        server: &NoiseServer<R, ()>,
        stream: S,
    ) -> Result<(Self, IdentityPayload)>;
}

impl<S> PaykitNoiseChannel for PubkyNoiseChannel<S> {
    /// Send encrypted message
    async fn send(&mut self, msg: PaykitNoiseMessage) -> Result<()>;
    
    /// Receive encrypted message
    async fn recv(&mut self) -> Result<PaykitNoiseMessage>;
}
```

### `IdentityPayload`

Returned by `accept()`, contains:

```rust
pub struct IdentityPayload {
    pub ed25519_pub: [u8; 32],      // Client's Ed25519 public key
    pub noise_x25519_pub: [u8; 32], // Client's X25519 public key (for this session)
    pub sig: [u8; 64],              // Ed25519 signature over binding message
    pub server_hint: Option<String>, // Optional routing hint
}
```

## See Also

- [pubky-noise README](../../pubky-noise-main/README.md) - Low-level Noise protocol implementation
- [paykit-interactive README](../paykit-interactive/README.md) - Payment message types
- [ARCHITECTURE.md](ARCHITECTURE.md) - Overall Paykit architecture

