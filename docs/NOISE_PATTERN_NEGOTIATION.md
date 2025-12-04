# Noise Pattern Negotiation Protocol

This document describes the pattern negotiation protocol used between Paykit clients and servers when establishing Noise connections.

## Overview

When connecting to a pattern-aware server (e.g., `NoiseServerHelper::run_pattern_aware_server`), clients send a single-byte pattern identifier before the Noise handshake message. This allows servers to accept multiple patterns on a single port.

## Pattern Bytes

| Byte | Pattern | Client Auth | Server Auth | Use Case |
|------|---------|-------------|-------------|----------|
| `0x00` | IK | Ed25519 signature | X25519 static | Standard authenticated payments |
| `0x01` | IK-raw | Via pkarr | X25519 static | Cold key scenarios |
| `0x02` | N | Anonymous | X25519 static | Anonymous donations |
| `0x03` | NN | Ephemeral | Ephemeral | Post-handshake attestation |
| `0x04` | XX | TOFU (learned) | TOFU (learned) | Trust-on-first-use |

> **Note:** The pattern-aware server (`NoiseServerHelper::run_pattern_server`) is
> currently used for IK-raw, N, NN, and XX. IK connections continue to use the
> legacy `run_server` helper and therefore do **not** send a pattern byte.

## Wire Format

### Pattern-Aware Protocol

```
[1 byte: pattern] [4 bytes: length BE] [N bytes: handshake msg]
```

1. **Pattern byte** - Identifies which Noise pattern to use
2. **Length prefix** - Big-endian 32-bit length of the handshake message
3. **Handshake message** - Pattern-specific Noise handshake data

### Legacy Protocol (IK only)

```
[4 bytes: length BE] [N bytes: handshake msg]
```

Legacy servers using `NoiseServerHelper::run_server` only accept IK pattern and don't expect a pattern byte.

## Client Implementation

### Using paykit-demo-core

```rust
use paykit_demo_core::{NoiseClientHelper, NoisePattern, NoiseRawClientHelper};

async fn connect_with_pattern(
    host: &str,
    pattern: NoisePattern,
) -> Result<PubkyNoiseChannel<TcpStream>> {
    match pattern {
        NoisePattern::IK => {
            // IK uses the legacy server (no pattern byte).
            NoiseClientHelper::connect_to_recipient(&identity, host, &pk).await
        }
        NoisePattern::IKRaw => {
            let x25519_sk = NoiseRawClientHelper::derive_x25519_key(&seed, b"device");
            NoiseRawClientHelper::connect_ik_raw_with_negotiation(&x25519_sk, host, &pk).await
        }
        NoisePattern::N => {
            NoiseRawClientHelper::connect_anonymous_with_negotiation(host, &pk).await
        }
        NoisePattern::NN => {
            let (channel, server_ephemeral) =
                NoiseRawClientHelper::connect_ephemeral_with_negotiation(host).await?;
            // Application-specific attestation logic goes here using server_ephemeral.
            Ok(channel)
        }
    }
}
```

### Using paykit-interactive (library)

```rust
use paykit_interactive::{PubkyNoiseChannel, NoisePattern, Zeroizing};
use tokio::net::TcpStream;

// For cold key scenarios
let x25519_sk = Zeroizing::new(derive_x25519_key(&seed, b"device"));
let stream = TcpStream::connect(host).await?;
let channel = PubkyNoiseChannel::connect_ik_raw_with_negotiation(&x25519_sk, stream, &server_pk).await?;

// For anonymous connections
let stream = TcpStream::connect(host).await?;
let channel = PubkyNoiseChannel::connect_anonymous_with_negotiation(stream, &server_pk).await?;

// For ephemeral connections (returns server's ephemeral for attestation)
let stream = TcpStream::connect(host).await?;
let (channel, server_ephemeral) =
    PubkyNoiseChannel::connect_ephemeral_with_negotiation(stream).await?;
```

## Server Implementation

### Pattern-Aware Server

```rust
use paykit_demo_core::{NoiseServerHelper, NoisePattern, AcceptedConnection};

NoiseServerHelper::run_pattern_server(
    &identity,
    "0.0.0.0:9735",
    |connection| async {
        match connection {
            AcceptedConnection::IK { channel, client_identity } => {
                // Client authenticated via Ed25519 signature
            }
            AcceptedConnection::IKRaw { channel, client_x25519_pk } => {
                // Verify client via pkarr lookup
            }
            AcceptedConnection::N { channel } => {
                // Anonymous client
            }
            AcceptedConnection::NN { channel, client_ephemeral } => {
                // Implement post-handshake attestation
            }
            AcceptedConnection::XX { channel, client_static_pk } => {
                // Trust-on-first-use: cache client_static_pk for future
            }
        }
        Ok(())
    },
).await
```

## Security Considerations

### Pattern Selection

- **IK**: Full authentication, use for standard payments
- **IK-raw**: Suitable when Ed25519 keys are cold; requires pkarr verification
- **N**: Client anonymity; verify server via pkarr
- **NN**: No authentication; **MUST** implement post-handshake attestation
- **XX**: Trust-on-first-use; cache static keys after first contact for future verification

### Post-Handshake Attestation (NN pattern)

When using NN pattern, the server should sign a challenge with their Ed25519 key:

```rust
let (channel, server_ephemeral) = PubkyNoiseChannel::connect_ephemeral(stream).await?;

// Server sends signed attestation
let attestation = channel.recv().await?;
// Verify: attestation = Sign(ed25519_sk, server_ephemeral || client_ephemeral)
verify_attestation(&attestation, &expected_server_pk)?;
```

## Related Documentation

- [Pattern Selection Guide](PATTERN_SELECTION.md)
- [Bitkit Integration](BITKIT_INTEGRATION.md)
- [pubky-noise Cold Key Architecture](../../pubky-noise/docs/COLD_KEY_ARCHITECTURE.md)

