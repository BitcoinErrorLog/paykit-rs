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
├── paykit-subscriptions/    # Subscription management and auto-pay
├── paykit-demo-core/        # Shared demo application logic
├── paykit-demo-cli/         # Command-line demo application
├── paykit-demo-web/         # WebAssembly browser demo application
└── paykit-mobile/           # Mobile FFI bindings and demo apps
    ├── src/                 # UniFFI bindings (Rust)
    ├── swift/               # iOS Keychain storage adapter
    ├── kotlin/              # Android EncryptedSharedPreferences adapter
    ├── ios-demo/            # Complete iOS demo app (SwiftUI)
    └── android-demo/        # Complete Android demo app (Jetpack Compose)
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

### Try the Web Demo

Run the WebAssembly demo in your browser:

```bash
cd paykit-demo-web
wasm-pack build --target web --out-dir www/pkg
cd www
python3 -m http.server 8080
```

Then open `http://localhost:8080` in your browser. See the [Web Demo README](paykit-demo-web/README.md) for complete documentation.

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

### paykit-subscriptions

Subscription management and automated payments:
- Subscription agreements with cryptographic signatures
- Payment requests with expiration and metadata
- Auto-pay rules with spending limits
- Thread-safe nonce tracking and spending limit enforcement
- Safe arithmetic with overflow protection

**Key APIs**:
```rust
use paykit_subscriptions::{Subscription, SubscriptionTerms, PaymentFrequency, AutoPayRule};

// Create subscription
let terms = SubscriptionTerms::new(amount, currency, PaymentFrequency::Monthly { day_of_month: 1 });
let subscription = Subscription::new(provider_pk, subscriber_pk, terms);

// Configure auto-pay
let rule = AutoPayRule::new(subscription_id, peer_pk, max_amount, period_seconds);
```

See the [Subscriptions README](paykit-subscriptions/README.md) for complete documentation.

### paykit-demo-cli

Full-featured command-line demonstration:
- Identity setup and management
- Payment method publishing
- Directory discovery
- Contact management
- Payment simulation
- Receipt viewing
- Subscription management
- Auto-pay configuration

### paykit-demo-web

WebAssembly browser application:
- Full Paykit functionality in the browser
- Identity management with localStorage persistence
- Contact management and directory discovery
- Payment method configuration
- Subscription and auto-pay management
- Spending limits with visual progress tracking
- Receipt tracking
- Interactive dashboard

See the [Web Demo README](paykit-demo-web/README.md) for complete documentation.

### paykit-mobile

Mobile FFI bindings and demo applications:
- UniFFI-based bindings for iOS (Swift) and Android (Kotlin)
- Platform-native secure storage adapters
- Complete demo apps with all Paykit features

**iOS Demo (SwiftUI)**:
- Keychain-based secure storage
- Auto-pay configuration with spending limits
- Subscription management
- Payment method discovery

**Android Demo (Jetpack Compose)**:
- EncryptedSharedPreferences storage with biometric support
- Material 3 design
- Auto-pay rules and spending tracking
- Full subscription workflow

See the [Mobile README](paykit-mobile/README.md) and [Mobile Integration Guide](docs/mobile-integration.md) for complete documentation.

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

# Subscriptions
cd paykit-subscriptions
cargo build

# CLI demo
cd paykit-demo-cli
cargo build --release

# Web demo (WASM)
cd paykit-demo-web
wasm-pack build --target web --out-dir www/pkg
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

**Core Components**
- `paykit-lib`: Core library with transport traits and public directory operations
- `paykit-interactive`: Interactive payment protocol with Noise encryption and receipts
- `paykit-subscriptions`: Subscription management, payment requests, and auto-pay automation
- `paykit-demo-core`: Shared business logic with SubscriptionCoordinator for demo applications

**Demo Applications**
- `paykit-demo-cli`: Full-featured command-line demo with all Paykit features
- `paykit-demo-web`: Complete WebAssembly browser demo with interactive UI
- `paykit-mobile/ios-demo`: iOS demo app with SwiftUI and Keychain storage
- `paykit-mobile/android-demo`: Android demo app with Jetpack Compose and EncryptedSharedPreferences

**Features**
- Identity management (Ed25519/X25519 keypairs)
- Payment method discovery and publishing
- Contact management
- Subscription agreements and payment requests
- Auto-pay rules with per-payment and per-period limits
- Peer spending limits with daily/weekly/monthly periods
- Atomic spending reservations with commit/rollback
- Receipt exchange and tracking
- Noise protocol encryption for private channels
- Platform-native secure storage (Keychain, EncryptedSharedPreferences)
- Comprehensive test coverage (100+ tests)

**Payment Plugins**
- `OnchainPlugin`: Bitcoin on-chain payment execution with proof generation
- `LightningPlugin`: Lightning Network payments (BOLT11, LNURL)

### 🚧 In Progress

- Full Noise protocol integration for live payments
- Pubky session creation API
- Real-time payment negotiation

### 📋 Planned

- Desktop Electron app (end-user application)
- Multi-signature support
- Hardware wallet integration

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

### Component Documentation
- [paykit-lib](paykit-lib/README.md) - Core library API reference
- [paykit-interactive](paykit-interactive/README.md) - Interactive payment protocol
- [paykit-subscriptions](paykit-subscriptions/README.md) - Subscription management
- [paykit-demo-core](paykit-demo-core/BUILD.md) - Shared demo logic
- [paykit-demo-cli](paykit-demo-cli/README.md) - CLI demo user guide
- [paykit-demo-web](paykit-demo-web/README.md) - Web demo user guide
- [paykit-mobile](paykit-mobile/README.md) - Mobile FFI bindings

### Feature Guides
- [Auto-Pay Guide](docs/autopay-guide.md) - Auto-pay rules and spending limits
- [Mobile Integration](docs/mobile-integration.md) - iOS and Android integration

### Project Documentation
- [Architecture Guide](docs/ARCHITECTURE.md) - System architecture and component relationships
- [Paykit Roadmap](PAYKIT_ROADMAP.md) - Development roadmap and integration plan
- [Security Guide](SECURITY.md) - Security considerations and best practices
- [Deployment Guide](DEPLOYMENT.md) - Deployment instructions
- [Build Instructions](BUILD.md) - Build and development setup
- [Documentation Index](docs/README.md) - Complete documentation index

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
