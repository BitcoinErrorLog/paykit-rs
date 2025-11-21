# Paykit

> A flexible payment protocol built on Pubky for discovering and coordinating payments across multiple methods (Bitcoin onchain, Lightning, and more).

## Overview

Paykit enables payment method discovery and interactive payment flows using:
- **Public Directory**: Discover payment methods via Pubky homeservers
- **Private Channels**: Negotiate payments over encrypted Noise Protocol channels
- **Receipt Exchange**: Cryptographic proof of payment coordination

## Project Structure

```
paykit-rs-master/
├── paykit-lib/              # Core library (public directory, transport traits)
├── paykit-interactive/      # Interactive payment protocol (Noise + receipts)
├── paykit-demo-core/        # Shared demo application logic
└── paykit-demo-cli/         # Command-line demo application
```

## Quick Start

### Try the CLI Demo

The fastest way to experience Paykit:

```bash
cd paykit-demo-cli
cargo build --release

# Create an identity
./target/release/paykit-demo setup --name alice

# View your Pubky URI
./target/release/paykit-demo whoami

# Discover someone's payment methods
./target/release/paykit-demo discover pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo
```

See the [CLI README](paykit-demo-cli/README.md) for complete documentation.

## Components

### paykit-lib

Core library providing:
- Payment method directory operations
- Transport trait abstractions
- Pubky homeserver integration
- Public endpoint management

**Key APIs**:
```rust
use paykit_lib::{
    AuthenticatedTransport,
    UnauthenticatedTransportRead,
    PubkyAuthenticatedTransport,
    PubkyUnauthenticatedTransport,
    MethodId,
    EndpointData,
};

// Publish payment methods
let transport = PubkyAuthenticatedTransport::new(session);
transport.upsert_payment_endpoint(&method_id, &endpoint_data).await?;

// Query payment methods
let transport = PubkyUnauthenticatedTransport::new(storage);
let methods = transport.fetch_supported_payments(&public_key).await?;
```

### paykit-interactive

Interactive payment protocol using:
- Pubky Noise for encrypted channels
- Receipt negotiation
- Private endpoint exchange
- Payment coordination

**Key APIs**:
```rust
use paykit_interactive::{
    PaykitInteractiveManager,
    PaykitNoiseChannel,
    PaykitReceipt,
};

// Initiate payment
let manager = PaykitInteractiveManager::new(storage, receipt_generator);
let receipt = manager.initiate_payment(&mut channel, provisional_receipt).await?;

// Handle payment request
let response = manager.handle_message(msg, &payer, &payee).await?;
```

### paykit-demo-core

Shared business logic for demo applications:
- Identity management (Ed25519/X25519 keypairs)
- Directory client wrapper
- Payment coordinator
- File-based storage
- Contact management

### paykit-demo-cli

Full-featured command-line demonstration:
- Identity setup and management
- Payment method publishing
- Directory discovery
- Contact management
- Payment simulation
- Receipt viewing

## Installation

### Prerequisites

- Rust 1.70+ (2021 edition)
- Cargo

### Build All Components

```bash
git clone <repo-url>
cd paykit-rs-master
cargo build --release
```

### Build Individual Components

```bash
# Core library
cd paykit-lib
cargo build

# Interactive protocol
cd paykit-interactive
cargo build

# CLI demo
cd paykit-demo-cli
cargo build --release
```

## Usage Examples

### Using paykit-lib

```rust
use paykit_lib::{MethodId, EndpointData, PubkyUnauthenticatedTransport};
use pubky::PublicStorage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create transport
    let storage = PublicStorage::new()?;
    let transport = PubkyUnauthenticatedTransport::new(storage);
    
    // Query payment methods
    let public_key = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".parse()?;
    let methods = transport.fetch_supported_payments(&public_key).await?;
    
    for (method_id, endpoint) in methods.entries {
        println!("{}: {}", method_id.0, endpoint.0);
    }
    
    Ok(())
}
```

### Using the CLI

```bash
# Setup identity
paykit-demo setup --name alice

# Add contact
paykit-demo contacts add bob pubky://...

# Discover payment methods
paykit-demo discover pubky://...

# View receipts
paykit-demo receipts
```

See [CLI README](paykit-demo-cli/README.md) for complete examples.

## Development Status

### ✅ Completed

**Phase 0: Protocol Testing**
- Integration tests for Noise protocol handshakes
- Pubky SDK compliance tests
- Fixed pubky-noise compilation issues

**Phase 1: Core Library (`paykit-demo-core`)**
- Identity management with Ed25519/X25519
- Directory operations wrapper
- Payment flow coordination
- File-based storage
- Data models

**Phase 2: CLI Demo (`paykit-demo-cli`)**
- Complete command structure
- All core commands implemented
- Rich terminal UI with colors and QR codes
- Contact management
- Comprehensive documentation

### 🚧 In Progress

- Full Noise protocol integration for live payments
- Pubky session creation API
- Real-time payment negotiation

### 📋 Planned

- Web WASM demo (browser-based showcase)
- Desktop Electron app (end-user application)
- Multi-signature support
- Hardware wallet integration

See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for detailed progress tracking.

## Architecture

### Payment Discovery Flow

```
User A                    Pubky Homeserver              User B
  |                              |                         |
  |--Publish Methods------------>|                         |
  |  (onchain, lightning)        |                         |
  |                              |                         |
  |                              |<---Query Methods--------|
  |                              |                         |
  |                              |----Return Methods------>|
```

### Interactive Payment Flow

```
Payer                    Noise Channel                 Payee
  |                              |                         |
  |--Connect (Noise_IK)----------|------------------------>|
  |                              |                         |
  |--RequestReceipt--------------|------------------------>|
  |  (provisional)               |                         |
  |                              |                         |
  |<-ConfirmReceipt--------------|-------------------------|
  |  (with invoice)              |                         |
  |                              |                         |
  |--Execute Payment (off-protocol)                        |
```

## Testing

```bash
# Run all tests
cargo test --all --all-features

# Test specific component
cd paykit-lib && cargo test
cd paykit-interactive && cargo test

# Test with network access (for integration tests)
cargo test --test pubky_sdk_compliance -- --test-threads=1
```

## Documentation

- [CLI User Guide](paykit-demo-cli/README.md)
- [Implementation Status](IMPLEMENTATION_STATUS.md)
- [Paykit Roadmap](PAYKIT_ROADMAP.md)
- [Noise Integration Review](../NOISE_INTEGRATION_REVIEW.md)

## Security Considerations

**For Production Use**:
1. Store private keys in secure enclaves/HSMs
2. Implement proper session management and authentication
3. Add rate limiting and DDoS protection
4. Verify payment proofs cryptographically
5. Use TLS for all network communication
6. Implement key rotation policies
7. Add audit logging

**Demo Limitations**:
- Keys stored in plain JSON files
- Simplified error handling
- No rate limiting
- Simulation mode for some operations

## Contributing

This is a demonstration/reference implementation. Contributions welcome:

1. Follow Rust 2021 conventions
2. Run `cargo fmt` and `cargo clippy`
3. Add tests for new functionality
4. Update documentation

See [repository guidelines](./_RULES.md) for detailed conventions.

## Dependencies

**Core**:
- `pubky` 0.6.0-rc.6 - Pubky protocol SDK
- `pubky-noise` - Noise protocol implementation
- `tokio` - Async runtime
- `serde` - Serialization

**CLI**:
- `clap` - Command-line parsing
- `colored` - Terminal colors
- `dialoguer` - Interactive prompts
- `qrcode` - QR code generation

## License

MIT

## Related Projects

- [Pubky](https://pubky.org) - Decentralized identity and data protocol
- [Pubky Noise](../pubky-noise-main/) - Noise Protocol implementation
- [Bitkit](https://bitkit.to) - Reference wallet implementation

## Contact

For questions or support, please open an issue in the repository.

---

**Built with ❤️ for the decentralized web**
