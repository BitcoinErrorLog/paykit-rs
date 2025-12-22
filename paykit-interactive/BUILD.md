# Paykit Interactive - Build Instructions

**Crate**: `paykit-interactive`  
**Description**: Interactive payment protocol over Noise channels  
**Type**: Library (no binary)

---

## Prerequisites

### Required

- **Rust 1.70.0+** via Rustup
- **Cargo** (comes with Rust)
- **paykit-lib** (workspace dependency)
- **pubky-noise** (workspace dependency)

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
cargo build -p paykit-interactive

# Or from this directory
cd paykit-interactive
cargo build

# Run tests
cargo test --lib
```

---

## Dependencies

### Workspace Dependencies

- **paykit-lib**: Core Paykit types and transport
- **pubky-noise**: Noise protocol implementation

These are automatically available when building from the workspace.

### External Dependencies

- `pubky` (0.6.0-rc.6) - Pubky SDK
- `tokio` - Async runtime
- `serde` - Serialization
- `anyhow` - Error handling

All external dependencies are automatically downloaded by Cargo.

---

## Building

### Development Build

```bash
cargo build -p paykit-interactive
```

**Output**: `target/debug/libpaykit_interactive.rlib`

### Release Build

```bash
cargo build -p paykit-interactive --release
```

**Output**: `target/release/libpaykit_interactive.rlib`

### With Features

```bash
# Build with all features
cargo build -p paykit-interactive --all-features

# Build without default features
cargo build -p paykit-interactive --no-default-features
```

---

## Testing

### Run All Tests

```bash
cargo test -p paykit-interactive --lib
```

**Note**: Currently this crate has 0 unit tests defined. Integration tests are in the `tests/` directory.

### Run Integration Tests

```bash
# Run all integration tests
cargo test -p paykit-interactive

# Run specific integration test
cargo test -p paykit-interactive --test integration_noise
cargo test -p paykit-interactive --test manager_tests
```

### Run Examples

```bash
# Run the complete payment flow example
cargo run -p paykit-interactive --example complete_payment_flow
```

---

## Usage in Other Projects

### As a Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
paykit-interactive = { path = "../paykit-interactive" }
```

### In Code

```rust
use paykit_interactive::{
    PaykitInteractiveManager,
    PaykitNoiseChannel,
    PaykitReceipt,
    PaykitStorage,
    ReceiptGenerator,
};

// Create a manager
let manager = PaykitInteractiveManager::new(storage, receipt_generator);

// Initiate payment
let receipt = manager.initiate_payment(&mut channel, provisional_receipt).await?;
```

---

## Project Structure

```
paykit-interactive/
├── Cargo.toml          # Package metadata
├── BUILD.md            # This file
├── README.md           # Project overview
├── src/
│   ├── lib.rs         # Main library entry point
│   ├── manager.rs     # PaykitInteractiveManager
│   ├── storage.rs     # Storage trait
│   └── transport.rs   # Noise channel abstraction
├── examples/
│   └── complete_payment_flow.rs  # Full payment example
└── tests/
    ├── integration_noise.rs      # Noise integration tests
    ├── manager_tests.rs          # Manager tests
    ├── mock_implementations.rs   # Test mocks
    └── serialization.rs          # Serialization tests
```

---

## Key Features

### Interactive Payment Protocol

Implements the Paykit interactive payment flow:

1. **Payment Initiation**: Payer sends provisional receipt
2. **Receipt Generation**: Payee generates actual receipt (invoice/address)
3. **Receipt Confirmation**: Payer confirms receipt
4. **Payment Execution**: Payment happens off-protocol
5. **Completion**: Final status exchange

### Noise Protocol Integration

Built on top of `pubky-noise` for secure P2P communication:

```rust
use paykit_interactive::PaykitNoiseChannel;

// Implement for your Noise transport
impl PaykitNoiseChannel for MyNoiseChannel {
    async fn send(&mut self, message: PaykitNoiseMessage) -> Result<()> {
        // Send over Noise
    }
    
    async fn recv(&mut self) -> Result<PaykitNoiseMessage> {
        // Receive over Noise
    }
}
```

### Storage Abstraction

Flexible storage backend:

```rust
use paykit_interactive::PaykitStorage;

#[async_trait]
impl PaykitStorage for MyStorage {
    async fn save_receipt(&self, receipt: &PaykitReceipt) -> Result<()> {
        // Store receipt
    }
    
    async fn get_receipt(&self, id: &str) -> Result<Option<PaykitReceipt>> {
        // Retrieve receipt
    }
}
```

---

## Examples

### Complete Payment Flow

See `examples/complete_payment_flow.rs` for a full example:

```bash
cargo run -p paykit-interactive --example complete_payment_flow
```

This demonstrates:
- Setting up Noise channels
- Creating a payment manager
- Initiating payment as payer
- Handling payment as payee
- Receipt generation and confirmation

---

## Development

### Code Quality

```bash
# Format code
cargo fmt --package paykit-interactive

# Lint code
cargo clippy -p paykit-interactive --all-targets

# Check without building
cargo check -p paykit-interactive
```

### Documentation

```bash
# Generate docs
cargo doc -p paykit-interactive --no-deps

# Generate and open in browser
cargo doc -p paykit-interactive --no-deps --open
```

---

## Troubleshooting

### Error: "could not find `paykit_lib`"

**Problem**: Building outside workspace

**Solution**: Build from workspace root:
```bash
cd paykit-rs
cargo build -p paykit-interactive
```

### Error: "could not find `pubky_noise`"

**Problem**: pubky-noise not in workspace

**Solution**: Ensure pubky-noise is properly configured in workspace `Cargo.toml`:
```toml
[workspace]
members = [
    "paykit-interactive",
    # ... other members
]
```

### Warning: "unused import: `paykit_lib::PublicKey`"

**Status**: Known warnings in `transport.rs`

**To fix**: Run `cargo fix`:
```bash
cargo fix -p paykit-interactive --allow-dirty
```

---

## Performance

### Build Time

- **Debug build**: ~40-70 seconds (first build)
- **Release build**: ~70-140 seconds (first build)
- **Incremental**: ~5-10 seconds (after changes)

### Binary Size

- **Debug**: ~600KB (unoptimized)
- **Release**: ~250KB (optimized)

---

## Testing Notes

### Mock Implementations

The `tests/mock_implementations.rs` provides test doubles:

- `MockStorage`: In-memory storage
- `MockReceiptGenerator`: Simple receipt generator
- `MockNoiseChannel`: Simulated Noise channel

Use these for testing without real Noise connections.

### Integration Tests

Integration tests in `tests/` verify:

- Noise protocol integration
- Manager behavior
- Message serialization
- End-to-end payment flows

Run with:
```bash
cargo test -p paykit-interactive
```

---

## API Stability

This library is currently in **active development**. The API may change between versions.

### Current Status

- ✅ Core payment flow: Stable
- ✅ Storage traits: Stable
- ⚠️ Noise integration: May evolve with pubky-noise updates
- 🔄 Additional features: In development

---

## Related Documentation

- **Workspace BUILD.md**: [../BUILD.md](../BUILD.md)
- **Project README**: [README.md](./README.md)
- **paykit-lib BUILD.md**: [../paykit-lib/BUILD.md](../paykit-lib/BUILD.md)
- **pubky-noise**: Check pubky-noise documentation for Noise protocol details

---

## Quick Reference

```bash
# Build
cargo build -p paykit-interactive

# Test
cargo test -p paykit-interactive

# Run example
cargo run -p paykit-interactive --example complete_payment_flow

# Docs
cargo doc -p paykit-interactive --no-deps --open

# Format & Lint
cargo fmt --package paykit-interactive
cargo clippy -p paykit-interactive --all-targets
```

---

**For workspace-level build instructions, see [../BUILD.md](../BUILD.md)**

