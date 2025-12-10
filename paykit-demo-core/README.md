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
- `NoiseClientHelper`: Client-side Noise protocol helper
- `NoiseServerHelper`: Server-side Noise protocol helper

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

