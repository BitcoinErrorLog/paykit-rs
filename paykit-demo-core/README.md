# Paykit Demo Core

Shared business logic library for all Paykit demo applications (CLI, Web, Desktop).

This crate provides a unified abstraction layer that sits between the protocol crates (`paykit-lib`, `paykit-interactive`, `paykit-subscriptions`) and the demo applications, enabling code reuse across different platforms.

## Purpose

`paykit-demo-core` provides:

- **Identity Management**: Ed25519/X25519 keypair generation and management
- **Directory Operations**: Wrapper around `paykit-lib` for payment method discovery
- **Payment Coordination**: Integration with `paykit-interactive` for payment flows
- **Subscription Management**: Integration with `paykit-subscriptions` for recurring payments
- **Storage Abstraction**: File-based storage for demo applications
- **Contact Management**: Address book functionality
- **Noise Protocol Helpers**: Client and server helpers for encrypted channels

## Architecture

This library sits between the protocol crates and demo applications:

```
Demo Apps (CLI, Web, Desktop)
      ↓
paykit-demo-core (this crate)
      ↓
Protocol Layer:
  - paykit-lib (directory & transport)
  - paykit-interactive (payments)
  - paykit-subscriptions (recurring payments)
  - pubky-noise (encryption)
```

## Key Modules

### Identity Management
- `Identity`: Represents a user identity with Ed25519/X25519 keypairs
- `IdentityManager`: Manages multiple identities with file-based storage

### Directory Operations
- `DirectoryClient`: Wrapper around `paykit-lib` for discovering payment methods

### Payment Coordination
- `PaymentCoordinator`: Manages payment flows using `paykit-interactive`
- `DemoPaykitStorage`: Storage implementation for receipts and private endpoints
- `DemoReceiptGenerator`: Receipt generation for demo purposes

### Subscription Management
- Integration with `paykit-subscriptions` for subscription agreements and auto-pay

### Storage
- File-based storage for identities, contacts, payment methods, and receipts
- Platform-agnostic storage traits for future WASM/localStorage support

### Noise Protocol
- `NoiseClientHelper`: Client-side Noise protocol helper (IK pattern)
- `NoiseServerHelper`: Server-side Noise protocol helper (multi-pattern)
- `NoiseRawClientHelper`: Raw-key Noise helpers for cold key scenarios
- `AcceptedConnection`: Enum representing pattern-specific connection info

### pkarr Discovery
- `pkarr_discovery`: Module for discovering/publishing X25519 keys via pkarr
- Cold key support: Publish X25519 keys once, keep Ed25519 cold

## Noise Pattern Support

This crate supports multiple Noise patterns for different security/usability tradeoffs:

| Pattern | Client Auth | Server Auth | Bidirectional | Use Case |
|---------|-------------|-------------|---------------|----------|
| **IK** | Ed25519 at handshake | X25519 static | ✅ Yes | Standard authenticated payments |
| **IK-raw** | Via pkarr lookup | X25519 static | ✅ Yes | Cold key scenarios (Bitkit) |
| **N** | Anonymous | X25519 static | ❌ **ONE-WAY** | Anonymous donations (fire-and-forget) |
| **NN** | Post-handshake attestation | Post-handshake attestation | ✅ Yes | Ephemeral sessions |
| **XX** | TOFU (learned) | TOFU (learned) | ✅ Yes | Trust-on-first-use |

### IK-raw Trust Model

IK-raw is designed for **cold key architectures** where Ed25519 keys are kept offline:

1. **One-time setup**: Derive X25519 from Ed25519, publish to pkarr with Ed25519 signature
2. **Runtime**: Use derived X25519 key for Noise handshakes (Ed25519 stays cold)
3. **Verification**: Receiver looks up sender's pkarr record to verify X25519 key binding

**Important**: Without pkarr verification, IK-raw connections are effectively anonymous. The receiver MUST verify the client's X25519 key against their pkarr record to establish identity.

```rust
use paykit_demo_core::pkarr_discovery;

// Cold key setup (one-time, requires Ed25519)
let (x25519_sk, x25519_pk) = pkarr_discovery::derive_noise_keypair(&ed25519_sk, "device-id");
pkarr_discovery::publish_noise_key(&session, &ed25519_sk, &x25519_pk, "device-id").await?;

// Runtime (Ed25519 stays cold)
let channel = NoiseRawClientHelper::connect_ik_raw(&x25519_sk, host, &server_pk).await?;
```

### N Pattern Warning

⚠️ **N pattern is ONE-WAY only.** The client can send encrypted messages to the server, but the server cannot send encrypted responses back. Use N only for fire-and-forget scenarios like anonymous donations.

For bidirectional anonymous communication, use NN pattern with post-handshake attestation.

## Usage

### Identity Management

```rust
use paykit_demo_core::{Identity, IdentityManager};

// Create identity manager
let manager = IdentityManager::new(storage_path)?;

// Create new identity
let identity = manager.create_identity("alice")?;

// List all identities
let identities = manager.list_identities()?;

// Get current identity
let current = manager.get_current_identity()?;
```

### Directory Operations

```rust
use paykit_demo_core::DirectoryClient;

let client = DirectoryClient::new(public_storage);
let methods = client.discover_payment_methods(&peer_pubkey).await?;
```

### Payment Coordination

```rust
use paykit_demo_core::PaymentCoordinator;

let coordinator = PaymentCoordinator::new(storage, receipt_generator);
// Use coordinator for payment flows
```

## Related Components

This crate is used by:

- **[paykit-demo-cli](../paykit-demo-cli/README.md)** - Command-line demo application
- **[paykit-demo-web](../paykit-demo-web/README.md)** - WebAssembly browser demo

This crate depends on:

- **[paykit-lib](../paykit-lib/README.md)** - Core library for directory operations
- **[paykit-interactive](../paykit-interactive/README.md)** - Interactive payment protocol
- **[paykit-subscriptions](../paykit-subscriptions/README.md)** - Subscription management

## Security Warning

⚠️ **This is demo code** for development and testing purposes only.

Key security considerations:
- Private keys stored in plaintext JSON files
- No encryption at rest
- No OS-level secure storage
- Simplified error handling

**For production use**, implement:
- Proper key management (HSMs, secure enclaves)
- Encryption at rest
- OS-level secure storage
- Comprehensive error handling
- Audit logging

## Testing

Run the test suite:

```bash
cd paykit-demo-core
cargo test
```

The crate includes:
- Unit tests for all modules
- Property-based tests
- Integration tests for directory operations
- Subscription flow tests

## Documentation

- [Build Instructions](BUILD.md)
- [Repository Root README](../README.md)

## License

See root [LICENSE](../LICENSE) file for details.

