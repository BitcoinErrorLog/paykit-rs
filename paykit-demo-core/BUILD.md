# Paykit Demo Core - Build Instructions

**Crate**: `paykit-demo-core`  
**Description**: Shared business logic for Paykit demo applications  
**Type**: Library (no binary)

---

## Prerequisites

### Required

- **Rust 1.70.0+** via Rustup
- **Cargo** (comes with Rust)
- **paykit-lib** (workspace dependency)
- **paykit-interactive** (workspace dependency)
- **paykit-subscriptions** (workspace dependency)

### Installation

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Verify
rustc --version
cargo --version
```

---

## Quick Build

```bash
# From workspace root
cd paykit-rs
cargo build -p paykit-demo-core

# Or from this directory
cd paykit-demo-core
cargo build

# Run tests
cargo test --lib
```

---

## Dependencies

### Workspace Dependencies

- **paykit-lib**: Core Paykit types and transport
- **paykit-interactive**: Interactive payment protocol
- **paykit-subscriptions**: Subscription management and recurring payments

### External Dependencies

- `pubky` (0.6.0-rc.6) - Pubky SDK
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `anyhow` - Error handling
- `pkarr` - Keypair generation
- `uuid` - Unique identifiers

All external dependencies are automatically downloaded by Cargo.

---

## Building

### Development Build

```bash
cargo build -p paykit-demo-core
```

**Output**: `target/debug/libpaykit_demo_core.rlib`

### Release Build

```bash
cargo build -p paykit-demo-core --release
```

**Output**: `target/release/libpaykit_demo_core.rlib`

---

## Testing

### Run All Tests (25+ tests)

```bash
cargo test -p paykit-demo-core --lib
```

**Test Coverage**:
- ✅ Identity generation and management (3 tests)
- ✅ X25519 key derivation (1 test)
- ✅ Contact storage (1 test)
- ✅ Subscription management (4 tests)
- ✅ Noise client/server helpers (8 tests)
- ✅ Integration tests (3 test files)
- ✅ Property-based tests (6 tests)

**Note**: 3 tests in `session.rs` require network access (pubky-testnet) and will fail in sandboxed environments.

### Run with Output

```bash
cargo test -p paykit-demo-core -- --nocapture
```

---

## Usage in Other Projects

### As a Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
paykit-demo-core = { path = "../paykit-demo-core" }
```

### In Code

```rust
use paykit_demo_core::{
    Identity,
    IdentityManager,
    DirectoryClient,
    PaymentCoordinator,
    SubscriptionCoordinator,
    DemoStorage,
    Contact,
    PaymentMethod,
    Receipt,
};

// Generate identity
let identity = Identity::generate()?;
let pubky_uri = identity.pubky_uri();

// Create directory client
let client = DirectoryClient::new("https://demo.httprelay.io");
let methods = client.query_methods(&public_key).await?;

// Payment coordination
let coordinator = PaymentCoordinator::new(storage, receipt_generator);
let receipt = coordinator.initiate_payment(channel, payer, payee, method, amount, currency).await?;

// Subscription management
let sub_coordinator = SubscriptionCoordinator::new(storage_path)?;
let subscription = sub_coordinator.create_subscription(
    subscriber, provider, 1000, "SAT".to_string(),
    frequency, method, description
)?;
```

---

## Project Structure

```
paykit-demo-core/
├── Cargo.toml          # Package metadata
├── BUILD.md            # This file
├── PAYKIT_DEMO_CORE_AUDIT_REPORT.md  # Audit report
├── src/
│   ├── lib.rs         # Module exports
│   ├── identity.rs    # Identity management (Ed25519 + X25519)
│   ├── directory.rs   # Directory client (Pubky queries)
│   ├── payment.rs     # Payment coordination
│   ├── subscription.rs # Subscription management
│   ├── noise_client.rs # Noise client helpers
│   ├── noise_server.rs # Noise server helpers
│   ├── session.rs     # Session management
│   ├── storage.rs     # File-based storage
│   └── models.rs      # Data models (Contact, PaymentMethod, Receipt)
└── tests/
    ├── test_directory_operations.rs  # Directory integration tests
    ├── test_subscription_flow.rs     # Subscription integration tests
    └── property_tests.rs             # Property-based tests
```

---

## Key Features

### Identity Management

Ed25519 keypair generation with X25519 derivation:

```rust
use paykit_demo_core::{Identity, IdentityManager};

// Generate new identity
let identity = Identity::new()?;

// With nickname
let identity = Identity::new_with_nickname("Alice")?;

// Get public key
let pubkey = identity.public_key();  // Ed25519 public key
let pubky_uri = identity.pubky_uri();  // pubky://...

// Derive X25519 for Noise
let x25519_keypair = identity.x25519_keypair()?;

// Manage multiple identities
let manager = IdentityManager::new(base_path)?;
manager.save_identity("alice", &identity)?;
let loaded = manager.load_identity("alice")?;
```

### Directory Client

Query payment methods from Pubky homeservers:

```rust
use paykit_demo_core::DirectoryClient;

let client = DirectoryClient::new("https://demo.httprelay.io");

// Query all payment methods
let methods = client.query_payment_methods(&public_key).await?;

// Query specific method
let method = client.query_payment_method(&public_key, &method_id).await?;

// List contacts
let contacts = client.list_contacts(&public_key).await?;
```

### Payment Coordination

Wraps paykit-interactive for simplified payment flows:

```rust
use paykit_demo_core::{PaymentCoordinator, DemoReceiptGenerator, DemoPaykitStorage};

let storage = Arc::new(Box::new(DemoPaykitStorage::new()));
let generator = Arc::new(Box::new(DemoReceiptGenerator));

let coordinator = PaymentCoordinator::new(storage, generator);

// Initiate payment
let receipt = coordinator.initiate_payment(
    channel,
    payer,
    payee,
    method,
    Some("1000".to_string()),
    Some("SAT".to_string()),
).await?;

// Handle incoming payment
let receipt = coordinator.handle_payment_request(channel, payer, payee).await?;
```

### Storage

Simple file-based storage:

```rust
use paykit_demo_core::DemoStorage;

let storage = DemoStorage::new(base_path)?;

// Store contact
let contact = Contact::new(public_key, "Alice".to_string());
storage.save_contact(&contact)?;

// Load contacts
let contacts = storage.load_contacts()?;

// Store payment method
let method = PaymentMethod::new("lightning".to_string(), "lnurl1...".to_string(), true);
storage.save_payment_method(&method)?;
```

---

## Models

### Contact

```rust
pub struct Contact {
    pub public_key: PublicKey,
    pub name: String,
    pub notes: Option<String>,
    pub added_at: i64,
}
```

### PaymentMethod

```rust
pub struct PaymentMethod {
    pub method_id: String,      // "lightning", "onchain", etc.
    pub endpoint: String,        // Address/invoice/etc.
    pub is_public: bool,         // Public or private method
    pub created_at: i64,
}
```

### Receipt

```rust
pub struct Receipt {
    pub id: String,
    pub payer: PublicKey,
    pub payee: PublicKey,
    pub method: String,
    pub amount: Option<String>,
    pub currency: Option<String>,
    pub timestamp: i64,
    pub metadata: serde_json::Value,
}
```

---

## Development

### Code Quality

```bash
# Format code
cargo fmt --package paykit-demo-core

# Lint code
cargo clippy -p paykit-demo-core --all-targets

# Check without building
cargo check -p paykit-demo-core
```

### Documentation

```bash
# Generate docs
cargo doc -p paykit-demo-core --no-deps

# Generate and open in browser
cargo doc -p paykit-demo-core --no-deps --open
```

---

## Troubleshooting

### Error: "could not find `paykit_lib`" or "could not find `paykit_interactive`"

**Problem**: Building outside workspace

**Solution**: Build from workspace root:
```bash
cd paykit-rs
cargo build -p paykit-demo-core
```

### Warning: "unused import" in payment.rs

**Status**: Known warnings for conditional imports

**To fix**: Run `cargo fix`:
```bash
cargo fix -p paykit-demo-core --lib --allow-dirty
```

### Test Failures

**Problem**: Tests may fail in sandboxed environments (need file access)

**Solution**: Run with appropriate permissions:
```bash
cargo test -p paykit-demo-core --lib -- --test-threads=1
```

---

## Performance

### Build Time

- **Debug build**: ~40-60 seconds (first build)
- **Release build**: ~60-120 seconds (first build)
- **Incremental**: ~5-10 seconds (after changes)

### Test Time

- **Unit tests (16)**: ~0.01 seconds
- **Integration tests**: Require network access
- **Property tests (6)**: ~0.5 seconds

### Binary Size

- **Debug**: ~600KB (unoptimized)
- **Release**: ~250KB (optimized)

---

## Integration Notes

### Used By

- **paykit-demo-cli**: CLI application
- **paykit-demo-web**: Web/WASM application

Both demos use paykit-demo-core for shared logic:

- Identity generation and management
- Directory queries
- Payment coordination
- Storage operations

### Why Separate Core?

1. **Code Reuse**: Share logic between CLI and Web demos
2. **Consistency**: Same behavior across platforms
3. **Testing**: Test business logic independently
4. **Maintainability**: Changes in one place

---

## API Stability

This library is currently in **active development**. The API may change between versions.

### Current Status

- ✅ Identity management: Stable
- ✅ Directory client: Stable
- ✅ Storage: Stable
- ✅ Payment coordination: Stable
- ✅ Subscription management: Stable (newly added)
- ✅ Noise protocol helpers: Stable

---

## Related Documentation

- **Workspace BUILD.md**: [../BUILD.md](../BUILD.md)
- **CLI Demo BUILD.md**: [../paykit-demo-cli/BUILD.md](../paykit-demo-cli/BUILD.md)
- **Web Demo BUILD**: [../paykit-demo-web/BUILD_INSTRUCTIONS.md](../paykit-demo-web/BUILD_INSTRUCTIONS.md)
- **paykit-lib BUILD.md**: [../paykit-lib/BUILD.md](../paykit-lib/BUILD.md)
- **paykit-interactive BUILD.md**: [../paykit-interactive/BUILD.md](../paykit-interactive/BUILD.md)

---

## Quick Reference

```bash
# Build
cargo build -p paykit-demo-core

# Test
cargo test -p paykit-demo-core --lib

# Docs
cargo doc -p paykit-demo-core --no-deps --open

# Format & Lint
cargo fmt --package paykit-demo-core
cargo clippy -p paykit-demo-core --all-targets
```

---

**For workspace-level build instructions, see [../BUILD.md](../BUILD.md)**

