# Paykit Demo CLI - Build Instructions

**Crate**: `paykit-demo-cli`  
**Description**: Command-line demonstration of Paykit protocol  
**Type**: Binary application  
**Binary Name**: `paykit-demo`

---

## Prerequisites

### Required

- **Rust 1.70.0+** via Rustup
- **Cargo** (comes with Rust)
- **All workspace dependencies** (paykit-lib, paykit-subscriptions, etc.)

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

## Quick Start

```bash
# From workspace root
cd paykit-rs-master
cargo build -p paykit-demo-cli

# Run
./target/debug/paykit-demo --help

# Or build and run in one command
cargo run -p paykit-demo-cli -- --help
```

---

## Building

### Development Build

```bash
cargo build -p paykit-demo-cli
```

**Output**: `target/debug/paykit-demo`

### Release Build (Optimized)

```bash
cargo build -p paykit-demo-cli --release
```

**Output**: `target/release/paykit-demo`

### Install Locally

```bash
# Install to ~/.cargo/bin/
cargo install --path paykit-demo-cli

# Now you can run from anywhere
paykit-demo --help
```

---

## Dependencies

### Workspace Dependencies

- **paykit-demo-core**: Shared demo logic
- **paykit-lib**: Core Paykit library
- **paykit-subscriptions**: Subscription protocol

### External Dependencies

- `clap` (4.x) - Command-line argument parsing
- `tokio` - Async runtime
- `console` - Terminal styling
- `dialoguer` - Interactive prompts
- `indicatif` - Progress bars
- `colored` - Color output
- `qrcode` - QR code generation
- `dirs` - User directories
- `chrono` - Date/time operations
- `rand` - Random number generation

All external dependencies are automatically downloaded by Cargo.

---

## Usage

### Getting Help

```bash
paykit-demo --help

# Help for specific command
paykit-demo setup --help
paykit-demo subscriptions --help
```

### Basic Workflow

```bash
# 1. Create identity
paykit-demo setup --name "Alice"

# 2. Show current identity
paykit-demo whoami

# 3. Publish payment methods (requires Pubky session)
paykit-demo publish --method lightning --endpoint "lnurl1..."

# 4. Discover peer's methods
paykit-demo discover --uri "pubky://..."

# 5. Create payment request
paykit-demo subscriptions send-request \
    --recipient "pubky://..." \
    --amount 1000 \
    --currency SAT \
    --description "Test payment"

# 6. List requests
paykit-demo subscriptions list-requests

# 7. Propose subscription
paykit-demo subscriptions propose \
    --recipient "pubky://..." \
    --amount 1000 \
    --currency SAT \
    --frequency monthly \
    --description "Monthly subscription"

# 8. List subscriptions
paykit-demo subscriptions list

# 9. Enable auto-pay
paykit-demo subscriptions enable-autopay \
    --subscription "sub_..." \
    --max-amount 1000

# 10. Set spending limit
paykit-demo subscriptions set-limit \
    --peer "pubky://..." \
    --limit 5000 \
    --period monthly
```

---

## Project Structure

```
paykit-demo-cli/
├── Cargo.toml            # Package metadata
├── BUILD.md              # This file
├── README.md             # Project overview
└── src/
    ├── main.rs          # Entry point
    ├── ui/
    │   └── mod.rs       # Terminal UI utilities
    └── commands/
        ├── mod.rs       # Command exports
        ├── setup.rs     # Identity setup
        ├── whoami.rs    # Show identity
        ├── list.rs      # List identities
        ├── switch.rs    # Switch identity
        ├── publish.rs   # Publish methods
        ├── discover.rs  # Discover methods
        ├── contacts.rs  # Manage contacts
        ├── pay.rs       # Initiate payment
        ├── receive.rs   # Receive payment
        ├── receipts.rs  # View receipts
        └── subscriptions.rs  # Subscription commands
```

---

## Commands

### Identity Management

- `setup` - Create new identity
- `whoami` - Show current identity
- `list` - List all identities
- `switch` - Switch to different identity

### Directory Operations

- `publish` - Publish payment methods
- `discover` - Query payment methods

### Contacts

- `contacts add` - Add contact
- `contacts list` - List contacts
- `contacts remove` - Remove contact

### Payments

- `pay` - Initiate payment
- `receive` - Start payment receiver
- `receipts` - View payment receipts

### Subscriptions (Phase 2 & 3)

#### Payment Requests
- `subscriptions send-request` - Send payment request
- `subscriptions list-requests` - List requests
- `subscriptions show-request` - Show request details
- `subscriptions respond` - Respond to request

#### Subscription Agreements
- `subscriptions propose` - Propose subscription
- `subscriptions accept` - Accept subscription
- `subscriptions list` - List subscriptions
- `subscriptions show` - Show subscription details

#### Auto-Pay (Phase 3)
- `subscriptions enable-autopay` - Enable auto-pay
- `subscriptions disable-autopay` - Disable auto-pay
- `subscriptions autopay-status` - Show auto-pay status
- `subscriptions set-limit` - Set spending limit
- `subscriptions show-limits` - Show spending limits

---

## Storage

### Default Storage Location

- **macOS**: `~/Library/Application Support/paykit-demo/`
- **Linux**: `~/.local/share/paykit-demo/`
- **Custom**: Set `PAYKIT_DEMO_DIR` environment variable

### Storage Structure

```
~/Library/Application Support/paykit-demo/
├── identities/
│   ├── alice.json
│   └── bob.json
├── current_identity.txt
├── contacts.json
├── receipts/
│   └── receipt_*.json
└── subscriptions/
    ├── requests/
    ├── subscriptions/
    ├── signed_subscriptions/
    ├── autopay_rules/
    └── peer_limits/
```

---

## Configuration

### Environment Variables

```bash
# Custom storage directory
export PAYKIT_DEMO_DIR="$HOME/my-paykit-data"

# Verbose logging
paykit-demo --verbose <command>

# Custom storage per command
paykit-demo --storage-dir /path/to/dir <command>
```

---

## Development

### Running from Source

```bash
# Run without installing
cargo run -p paykit-demo-cli -- setup --name "Test"

# Run with verbose output
cargo run -p paykit-demo-cli -- --verbose whoami

# Run specific command
cargo run -p paykit-demo-cli -- subscriptions list
```

### Code Quality

```bash
# Format code
cargo fmt --package paykit-demo-cli

# Lint code
cargo clippy -p paykit-demo-cli --all-targets

# Check without building
cargo check -p paykit-demo-cli
```

### Testing

```bash
# Run tests (if any)
cargo test -p paykit-demo-cli

# Run integration tests
cargo test -p paykit-demo-cli --test '*'
```

---

## Troubleshooting

### Error: "command not found: paykit-demo"

**Problem**: Binary not in PATH

**Solution**:
```bash
# Option 1: Use cargo run
cargo run -p paykit-demo-cli -- --help

# Option 2: Use full path
./target/debug/paykit-demo --help

# Option 3: Install to cargo bin
cargo install --path paykit-demo-cli
```

### Error: "No such file or directory" for storage

**Problem**: Storage directory doesn't exist

**Solution**: The CLI creates it automatically, but you can create manually:
```bash
mkdir -p "$HOME/Library/Application Support/paykit-demo"
```

### Error: Build fails with "could not find `paykit_subscriptions`"

**Problem**: Building outside workspace

**Solution**: Build from workspace root:
```bash
cd paykit-rs-master
cargo build -p paykit-demo-cli
```

### Warning: "unused import" or "unused variable"

**Status**: Known warnings in some command modules

**To fix**: Run `cargo fix`:
```bash
cargo fix -p paykit-demo-cli --bin paykit-demo --allow-dirty
```

---

## Terminal UI Features

### Color Output

Uses `colored` crate for visual feedback:

- 🟢 **Green**: Success messages
- 🔴 **Red**: Error messages
- 🟡 **Yellow**: Warnings
- 🔵 **Blue**: Info messages

### Progress Indicators

Uses `indicatif` for operations:

- Spinners for async operations
- Progress bars for multi-step processes

### Interactive Prompts

Uses `dialoguer` for user input:

- Text input with validation
- Confirmations
- Selection menus

---

## Examples

### Complete Workflow

```bash
# Setup Alice
paykit-demo setup --name "Alice"

# Show identity
paykit-demo whoami
# Output: Name: Alice
#         Pubky URI: pubky://...

# Create subscription request
paykit-demo subscriptions send-request \
    --recipient "pubky://y4euc825..." \
    --amount 1000 \
    --currency SAT \
    --description "Monthly subscription"

# List requests
paykit-demo subscriptions list-requests

# Propose subscription
paykit-demo subscriptions propose \
    --recipient "pubky://y4euc825..." \
    --amount 1000 \
    --currency SAT \
    --frequency monthly:1 \
    --description "Monthly payment"

# Enable auto-pay with limits
paykit-demo subscriptions enable-autopay \
    --subscription "sub_..." \
    --max-amount 1000

paykit-demo subscriptions set-limit \
    --peer "pubky://y4euc825..." \
    --limit 5000 \
    --period monthly
```

---

## Performance

### Build Time

- **Debug build**: ~60-90 seconds (first build)
- **Release build**: ~90-180 seconds (first build)
- **Incremental**: ~5-15 seconds (after changes)

### Binary Size

- **Debug**: ~15MB (unoptimized with debug symbols)
- **Release**: ~5MB (optimized, stripped)

### Startup Time

- **Debug**: ~50-100ms
- **Release**: ~20-30ms

---

## Release Builds

### Optimized Build

```bash
cargo build -p paykit-demo-cli --release
```

### Stripped Binary (Smaller Size)

```bash
cargo build -p paykit-demo-cli --release
strip target/release/paykit-demo

# Check size
ls -lh target/release/paykit-demo
```

### Distribution

```bash
# Create distributable
cd target/release
tar -czf paykit-demo-macos.tar.gz paykit-demo
```

---

## Related Documentation

- **Workspace BUILD.md**: [../BUILD.md](../BUILD.md)
- **Project README**: [README.md](./README.md)
- **Subscriptions Complete**: [../SUBSCRIPTIONS_COMPLETE_REPORT.md](../SUBSCRIPTIONS_COMPLETE_REPORT.md)
- **Demo Core BUILD.md**: [../paykit-demo-core/BUILD.md](../paykit-demo-core/BUILD.md)

---

## Quick Reference

```bash
# Build
cargo build -p paykit-demo-cli

# Run
cargo run -p paykit-demo-cli -- --help

# Install
cargo install --path paykit-demo-cli

# Common commands
paykit-demo setup --name "Alice"
paykit-demo whoami
paykit-demo subscriptions list
paykit-demo subscriptions list-requests

# Format & Lint
cargo fmt --package paykit-demo-cli
cargo clippy -p paykit-demo-cli --all-targets
```

---

**For workspace-level build instructions, see [../BUILD.md](../BUILD.md)**

