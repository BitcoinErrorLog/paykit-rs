# Paykit Workspace - Complete Build Guide

**Platform**: macOS, Linux  
**Last Updated**: November 20, 2025

---

## Quick Start

### Prerequisites Check

```bash
# Check if you have the right Rust setup
which rustc
# ✅ Should be: /Users/USERNAME/.cargo/bin/rustc (Rustup)
# ❌ NOT: /opt/homebrew/bin/rustc (Homebrew - won't work for WASM)

# Check versions
rustc --version  # Should be 1.70.0 or higher
cargo --version
```

### One-Command Build

```bash
# Build entire workspace
cd paykit-rs-master
cargo build --workspace

# Run all tests
cargo test --workspace --lib
```

---

## System Requirements

### Required

- **Rust 1.70.0+** via Rustup (NOT Homebrew)
- **Cargo** (comes with Rust)
- **OpenSSL** (usually pre-installed on macOS/Linux)

### Optional (for demos)

- **Python 3** - For web demo development server
- **wasm-pack** - For building web demo
- **Node.js/npm** - For web demo npm scripts

---

## Installation

### 1. Install Rust via Rustup

**⚠️ IMPORTANT**: Use Rustup, NOT Homebrew's Rust!

```bash
# Install Rustup (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow prompts (choose default installation)

# Restart terminal or source cargo env
source "$HOME/.cargo/env"

# Verify
rustc --version
which rustc  # Should show ~/.cargo/bin/rustc
```

**If you have Homebrew Rust installed**:
```bash
# Remove it first
brew uninstall rust

# Then install Rustup as above
```

### 2. Install System Dependencies

#### macOS

```bash
# Most dependencies are pre-installed
# Verify OpenSSL
brew list openssl || brew install openssl

# For web demo (optional)
brew install python3  # Usually pre-installed
cargo install wasm-pack  # For WASM builds
```

#### Linux (Ubuntu/Debian)

```bash
sudo apt update
sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    python3
```

#### Linux (Fedora/RHEL)

```bash
sudo dnf install -y \
    gcc \
    openssl-devel \
    pkg-config \
    python3
```

### 3. Clone Repository

```bash
# Clone (if not already done)
git clone <repository-url> paykit-rs-master
cd paykit-rs-master
```

---

## Building

### Workspace Build

```bash
# Build all crates in workspace
cargo build --workspace

# Build in release mode (optimized)
cargo build --workspace --release

# Build specific crate
cargo build -p paykit-lib
cargo build -p paykit-demo-cli
```

### Individual Projects

Each project has its own BUILD.md:

- **paykit-lib**: [paykit-lib/BUILD.md](./paykit-lib/BUILD.md)
- **paykit-interactive**: [paykit-interactive/BUILD.md](./paykit-interactive/BUILD.md)
- **paykit-subscriptions**: [paykit-subscriptions/BUILD.md](./paykit-subscriptions/BUILD.md)
- **paykit-demo-cli**: [paykit-demo-cli/BUILD.md](./paykit-demo-cli/BUILD.md)
- **paykit-demo-core**: [paykit-demo-core/BUILD.md](./paykit-demo-core/BUILD.md)
- **paykit-demo-web**: [paykit-demo-web/BUILD_INSTRUCTIONS.md](./paykit-demo-web/BUILD_INSTRUCTIONS.md)

### Special: Web Demo (WASM)

The web demo requires additional setup:

```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack

# Build web demo
cd paykit-demo-web
wasm-pack build --target web --out-dir www/pkg
```

See [paykit-demo-web/START_HERE.md](./paykit-demo-web/START_HERE.md) for complete web demo instructions.

---

## Testing

### Run All Tests

```bash
# Run all library tests
cargo test --workspace --lib

# Run all tests (including integration)
cargo test --workspace

# Run tests for specific crate
cargo test -p paykit-subscriptions
```

### Run Specific Tests

```bash
# Run tests matching a pattern
cargo test signing

# Run a specific test
cargo test test_sign_and_verify_ed25519

# Run with output
cargo test -- --nocapture
```

### Test Coverage

```bash
# Install cargo-tarpaulin (once)
cargo install cargo-tarpaulin

# Run with coverage
cargo tarpaulin --workspace --lib --out Html
# Opens coverage report in browser
```

---

## Running Demos

### CLI Demo

```bash
cd paykit-demo-cli

# Build
cargo build

# Run
cargo run -- --help

# Example: Create identity
cargo run -- setup --name "Alice"
```

See [paykit-demo-cli/BUILD.md](./paykit-demo-cli/BUILD.md) for complete CLI instructions.

### Web Demo

```bash
cd paykit-demo-web

# Build WASM
wasm-pack build --target web --out-dir www/pkg

# Serve
python3 -m http.server 8080 -d www

# Open browser
open http://localhost:8080
```

See [paykit-demo-web/START_HERE.md](./paykit-demo-web/START_HERE.md) for complete web demo instructions.

---

## Workspace Structure

```
paykit-rs-master/
├── paykit-lib/              # Core Paykit library
├── paykit-interactive/      # Interactive payment protocol
├── paykit-subscriptions/    # Subscription protocol
├── paykit-demo-cli/         # CLI demo application
├── paykit-demo-core/        # Shared demo logic
├── paykit-demo-web/         # Web/WASM demo
└── Cargo.toml              # Workspace configuration
```

### Dependencies Between Crates

```
paykit-lib (core)
    ↓
paykit-interactive
    ↓
paykit-subscriptions
    ↓
paykit-demo-core → paykit-demo-cli
                → paykit-demo-web
```

---

## Troubleshooting

### Error: "can't find crate for `core`" (WASM build)

**Problem**: WASM target not installed or using Homebrew Rust

**Solution**:
```bash
# If using Homebrew Rust, switch to Rustup first
brew uninstall rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown
```

### Error: "failed to run custom build command for `openssl-sys`"

**Problem**: OpenSSL development headers not found

**Solution (macOS)**:
```bash
brew install openssl
export OPENSSL_DIR=$(brew --prefix openssl)
cargo build
```

**Solution (Linux)**:
```bash
sudo apt install libssl-dev pkg-config
cargo build
```

### Error: "linker `cc` not found"

**Problem**: C compiler not installed

**Solution (macOS)**:
```bash
xcode-select --install
```

**Solution (Linux)**:
```bash
sudo apt install build-essential
```

### Build is Slow

**Solution**: Use release mode or parallel builds
```bash
# Release mode (faster runtime, slower compile)
cargo build --release

# Parallel builds (use all CPU cores)
cargo build -j $(nproc)  # Linux
cargo build -j $(sysctl -n hw.ncpu)  # macOS
```

### Out of Disk Space

**Solution**: Clean build artifacts
```bash
# Clean all build artifacts
cargo clean

# Clean specific package
cargo clean -p paykit-subscriptions

# Check disk usage
du -sh target/
```

---

## Development Workflow

### 1. Make Changes

```bash
# Edit files in src/
vim paykit-lib/src/lib.rs
```

### 2. Check Code

```bash
# Format code
cargo fmt --all

# Lint code
cargo clippy --all-targets --all-features

# Check without building
cargo check
```

### 3. Build

```bash
cargo build
```

### 4. Test

```bash
cargo test
```

### 5. Run

```bash
cargo run -p paykit-demo-cli -- --help
```

---

## Common Commands

```bash
# Build everything
cargo build --workspace

# Test everything
cargo test --workspace --lib

# Format all code
cargo fmt --all

# Lint all code
cargo clippy --all-targets --all-features

# Clean build artifacts
cargo clean

# Update dependencies
cargo update

# Check for outdated dependencies
cargo outdated

# Generate documentation
cargo doc --workspace --no-deps --open
```

---

## Environment Variables

### Optional Configuration

```bash
# Custom storage directory for demos
export PAYKIT_DEMO_DIR="$HOME/my-paykit-data"

# Rust compiler flags (optimize for native CPU)
export RUSTFLAGS="-C target-cpu=native"

# Parallel build jobs
export CARGO_BUILD_JOBS=8
```

---

## Platform-Specific Notes

### macOS

#### Apple Silicon (M1/M2/M3)

Everything works natively! No special configuration needed.

#### Intel Macs

Works perfectly. Use the same instructions.

#### Homebrew Rust Issue

If you installed Rust via Homebrew (`brew install rust`), you MUST uninstall it and use Rustup instead. Homebrew Rust doesn't support WASM targets.

```bash
# Check if you have Homebrew Rust
which rustc
# If shows /opt/homebrew/bin/rustc, you need to switch

brew uninstall rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Linux

#### Ubuntu/Debian

```bash
# Install dependencies
sudo apt update
sudo apt install build-essential pkg-config libssl-dev
```

#### Arch Linux

```bash
sudo pacman -S base-devel openssl
```

#### WSL (Windows Subsystem for Linux)

Works perfectly! Follow Linux instructions.

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build and Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Build
        run: cargo build --workspace
      - name: Test
        run: cargo test --workspace --lib
```

---

## Component-Specific Build Instructions

Each component has detailed build documentation:

- **[paykit-lib/BUILD.md](paykit-lib/BUILD.md)** - Core library build instructions
- **[paykit-interactive/BUILD.md](paykit-interactive/BUILD.md)** - Interactive protocol build (if exists)
- **[paykit-subscriptions/BUILD.md](paykit-subscriptions/BUILD.md)** - Subscriptions crate build
- **[paykit-demo-core/BUILD.md](paykit-demo-core/BUILD.md)** - Shared demo logic build
- **[paykit-demo-cli/BUILD.md](paykit-demo-cli/BUILD.md)** - CLI demo build instructions
- **[paykit-demo-web/BUILD_INSTRUCTIONS.md](paykit-demo-web/BUILD_INSTRUCTIONS.md)** - Web demo WASM build instructions

## Next Steps

After building successfully:

1. **Read component-specific BUILD.md files** listed above
2. **Try the demos**: Start with `paykit-demo-cli` or `paykit-demo-web`
3. **Read the code**: Check out `paykit-lib/src/lib.rs`
4. **Run tests**: `cargo test --workspace --lib`
5. **Explore examples**: Check `paykit-interactive/examples/`

---

## Getting Help

### Documentation

- Workspace README: [README.md](./README.md)
- Component READMEs:
  - [paykit-lib/README.md](paykit-lib/README.md)
  - [paykit-interactive/README.md](paykit-interactive/README.md)
  - [paykit-subscriptions/README.md](paykit-subscriptions/README.md)
  - [paykit-demo-cli/README.md](paykit-demo-cli/README.md)
  - [paykit-demo-web/README.md](paykit-demo-web/README.md)
- Security docs: [SECURITY.md](./SECURITY.md)
- Deployment: [DEPLOYMENT.md](./DEPLOYMENT.md)

### Common Issues

1. **Homebrew Rust**: Must use Rustup (not Homebrew's Rust). See Prerequisites section above.
2. **WASM build fails**: Need WASM target installed: `rustup target add wasm32-unknown-unknown`. See [paykit-demo-web/BUILD_INSTRUCTIONS.md](paykit-demo-web/BUILD_INSTRUCTIONS.md)
3. **OpenSSL errors**: Install libssl-dev (Linux) or ensure OpenSSL is available (macOS). See Troubleshooting section above.

---

## Quick Reference

```bash
# Complete setup from scratch
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
cd paykit-rs-master
cargo build --workspace
cargo test --workspace --lib

# Web demo additional setup
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
cd paykit-demo-web
wasm-pack build --target web --out-dir www/pkg
python3 -m http.server 8080 -d www
```

---

**Happy Building! 🚀**

