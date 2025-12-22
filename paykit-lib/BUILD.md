# Paykit Library - Build Instructions

**Crate**: `paykit-lib`  
**Description**: Core Paykit protocol library  
**Type**: Library (no binary)

---

## Prerequisites

### Required

- **Rust 1.70.0+** via Rustup
- **Cargo** (comes with Rust)
- **OpenSSL** (for cryptographic operations)

### Installation

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Verify
rustc --version  # Should be 1.70.0 or higher
cargo --version
```

---

## Quick Build

```bash
# From workspace root
cd paykit-rs
cargo build -p paykit-lib

# Or from this directory
cd paykit-lib
cargo build

# Run tests
cargo test --lib
```

---

## Features

This crate has optional features that enable different functionality:

### Available Features

| Feature | Default | Description |
|---------|---------|-------------|
| `pubky` | Yes | Enables Pubky transport integration for directory operations |
| `http-executor` | No | Enables real HTTP clients for LND and Esplora executors |
| `file-storage` | No | Enables encrypted file-based storage |
| `tracing` | No | Enables tracing instrumentation for debugging |
| `test-utils` | No | Enables test utilities for integration testing |
| `integration-tests` | No | Enables Pubky integration tests (requires network) |

### Build with Specific Features

```bash
# Default features (includes pubky)
cargo build

# No default features (minimal build)
cargo build --no-default-features

# Enable HTTP executors for real LND/Esplora connections
cargo build --features http-executor

# Enable all features for development
cargo build --all-features

# WASM-compatible build (no http-executor)
cargo build --target wasm32-unknown-unknown --no-default-features --features pubky
```

### Feature Details

#### `http-executor`

This feature enables the real HTTP client implementations in the executor modules:

- **LndExecutor**: Full REST API client for LND nodes
- **EsploraExecutor**: Full HTTP client for Esplora/mempool.space APIs

Without this feature, executor methods return `Unimplemented` errors. This is useful for WASM targets where `reqwest` is not available.

```bash
# Build with executors enabled
cargo build -p paykit-lib --features http-executor

# Run executor tests
cargo test -p paykit-lib --features http-executor --test executor_integration
```

#### `file-storage`

Enables AES-GCM encrypted storage with Argon2 key derivation:

```bash
cargo build -p paykit-lib --features file-storage
```

---

## Dependencies

### System Dependencies

#### macOS

```bash
# OpenSSL (usually pre-installed)
brew list openssl || brew install openssl

# If build fails, set OpenSSL path:
export OPENSSL_DIR=$(brew --prefix openssl)
```

#### Linux (Ubuntu/Debian)

```bash
sudo apt update
sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev
```

#### Linux (Fedora/RHEL)

```bash
sudo dnf install -y \
    gcc \
    openssl-devel \
    pkg-config
```

### Rust Dependencies

All Rust dependencies are managed by Cargo and will be downloaded automatically:

- `pubky` (0.6.0-rc.6) - Pubky SDK for directory operations
- `serde` - Serialization framework
- `thiserror` - Error handling
- `anyhow` - Error context
- `url` - URL parsing

---

## Building

### Development Build

```bash
cargo build -p paykit-lib
```

**Output**: `target/debug/libpaykit_lib.rlib`

### Release Build

```bash
cargo build -p paykit-lib --release
```

**Output**: `target/release/libpaykit_lib.rlib`

### Documentation

```bash
# Generate docs
cargo doc -p paykit-lib --no-deps

# Generate and open in browser
cargo doc -p paykit-lib --no-deps --open
```

---

## Testing

### Run All Tests

```bash
cargo test -p paykit-lib --lib
```

**Note**: Some tests require file system access and may fail in sandboxed environments.

### Run Specific Tests

```bash
# Test endpoint operations
cargo test -p paykit-lib endpoint

# Test with output
cargo test -p paykit-lib -- --nocapture

# Run a specific test
cargo test -p paykit-lib test_endpoint_round_trip_and_update
```

### Integration Tests

```bash
# Run integration tests (if any)
cargo test -p paykit-lib --test pubky_sdk_compliance
```

---

## Usage in Other Projects

### As a Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
paykit-lib = { path = "../paykit-lib", features = ["pubky"] }
```

### In Code

```rust
use paykit_lib::{MethodId, PublicKey, EndpointData};

// Use the library
let method = MethodId("lightning".to_string());
let endpoint = EndpointData::from_str("lnurl1...")?;
```

---

## Project Structure

```
paykit-lib/
├── Cargo.toml           # Package metadata
├── BUILD.md             # This file
├── README.md            # Project overview
├── src/
│   ├── lib.rs          # Main library entry point
│   └── transport/      # Transport abstraction layer
│       ├── mod.rs
│       ├── traits.rs   # Transport trait definitions
│       └── pubky/      # Pubky transport implementation
│           ├── mod.rs
│           ├── authenticated_transport.rs
│           └── unauthenticated_transport.rs
└── tests/
    └── pubky_sdk_compliance.rs  # Integration tests
```

---

## Key Concepts

### Transport Abstraction

`paykit-lib` uses a transport abstraction to allow different backends:

```rust
pub trait AuthenticatedTransport {
    async fn publish_endpoint(&self, method: &MethodId, data: &EndpointData) -> Result<()>;
    async fn remove_endpoint(&self, method: &MethodId) -> Result<()>;
}

pub trait UnauthenticatedTransportRead {
    async fn fetch_payment_endpoint(&self, method: &MethodId) -> Result<Option<EndpointData>>;
    async fn list_contact_pubkeys(&self) -> Result<Vec<PublicKey>>;
}
```

### Pubky Integration

The default `pubky` feature provides implementations that work with the Pubky SDK:

```rust
use paykit_lib::transport::pubky::{
    PubkyAuthenticatedTransport,
    PubkyUnauthenticatedTransport,
};

// For authenticated operations (requires session)
let transport = PubkyAuthenticatedTransport::new(session);

// For read-only operations
let transport = PubkyUnauthenticatedTransport::new();
```

---

## Development

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy --all-targets --all-features

# Check without building
cargo check -p paykit-lib
```

### Benchmarks

```bash
# Run benchmarks (if any)
cargo bench -p paykit-lib
```

---

## Troubleshooting

### Error: "failed to run custom build command for `openssl-sys`"

**Problem**: OpenSSL not found

**macOS**:
```bash
brew install openssl
export OPENSSL_DIR=$(brew --prefix openssl)
cargo clean
cargo build -p paykit-lib
```

**Linux**:
```bash
sudo apt install libssl-dev pkg-config
cargo clean
cargo build -p paykit-lib
```

### Error: "linker `cc` not found"

**Problem**: C compiler not installed

**macOS**:
```bash
xcode-select --install
```

**Linux**:
```bash
sudo apt install build-essential
```

### Warning: "unexpected `cfg` condition value: `tracing`"

**Status**: Expected warning. The `tracing` feature is prepared but not yet fully implemented.

**To suppress**: Ignore for now, or add to `Cargo.toml`:
```toml
[features]
tracing = []  # Enable when implemented
```

### Tests Fail with "Operation not permitted"

**Problem**: Tests need file system access

**Solution**: Run with appropriate permissions:
```bash
# macOS/Linux
cargo test -p paykit-lib --lib -- --test-threads=1
```

---

## Performance

### Build Time

- **Debug build**: ~30-60 seconds (first build)
- **Release build**: ~60-120 seconds (first build)
- **Incremental**: ~5-10 seconds (after changes)

### Binary Size

- **Debug**: ~500KB (unoptimized)
- **Release**: ~200KB (optimized)

---

## API Stability

This library is currently in **active development**. The API may change between versions.

### Semantic Versioning

- **0.x.y**: Breaking changes may occur in minor versions
- **1.0.0+**: Follows strict semver

---

## Related Documentation

- **Workspace BUILD.md**: [../BUILD.md](../BUILD.md)
- **Project README**: [README.md](./README.md)
- **Transport Documentation**: See `src/transport/traits.rs`
- **Paykit Protocol**: [../README.md](../README.md)

---

## Quick Reference

```bash
# Build
cargo build -p paykit-lib

# Test
cargo test -p paykit-lib --lib

# Docs
cargo doc -p paykit-lib --no-deps --open

# Format & Lint
cargo fmt
cargo clippy --all-targets --all-features

# Clean
cargo clean -p paykit-lib
```

---

**For workspace-level build instructions, see [../BUILD.md](../BUILD.md)**

