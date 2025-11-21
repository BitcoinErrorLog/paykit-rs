# Paykit Demo CLI - Quick Start Guide

> **5-minute guide to get started with Paykit Demo CLI**

This guide provides step-by-step instructions for common workflows. For complete documentation, see [README.md](./README.md).

## Installation

```bash
cd paykit-demo-cli
cargo build --release
```

The binary will be at `target/release/paykit-demo`

## Basic Usage

### 1. Setup Your Identity

```bash
# Create a new identity (or load existing)
./paykit-demo setup --name Alice

# View your identity
./paykit-demo whoami
```

### 2. Publish Payment Methods

```bash
# Publish a lightning endpoint
./paykit-demo publish \
  --lightning "lnbc1000..." \
  --homeserver "pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"

# Publish multiple methods
./paykit-demo publish \
  --onchain "bc1q..." \
  --lightning "lnbc..." \
  --homeserver "pubky://..."
```

### 3. Discover Payment Methods

```bash
# Discover by pubky URI
./paykit-demo discover pubky://[recipient-public-key]

# Discover with verbose output
./paykit-demo discover --verbose pubky://[recipient-public-key]
```

### 4. Subscriptions

#### Send a Payment Request

```bash
./paykit-demo subscriptions request \
  --recipient pubky://[recipient-key] \
  --amount 1000 \
  --currency SAT \
  --description "Coffee payment"
```

#### List Requests

```bash
# List all requests
./paykit-demo subscriptions list

# List sent requests
./paykit-demo subscriptions list --direction sent

# List received requests
./paykit-demo subscriptions list --direction received
```

#### Respond to a Request

```bash
./paykit-demo subscriptions respond \
  --request-id [id] \
  --accept
```

#### Propose a Subscription

```bash
./paykit-demo subscriptions propose \
  --recipient pubky://[recipient-key] \
  --amount 1000 \
  --currency SAT \
  --frequency monthly:1 \
  --description "Monthly subscription"
```

#### Accept a Subscription

```bash
./paykit-demo subscriptions accept \
  --subscription-id [id]
```

#### Enable Auto-Pay

```bash
./paykit-demo subscriptions enable-auto-pay \
  --subscription-id [subscription-id] \
  --max-amount 5000
```

#### Set Spending Limits

```bash
./paykit-demo subscriptions set-limit \
  --peer pubky://[peer-key] \
  --limit 10000 \
  --period monthly
```

#### Show Spending Limits

```bash
./paykit-demo subscriptions show-limits
```

## Testing

### Run All Tests

```bash
# Library tests
cargo test --workspace --lib

# Integration tests
cargo test --workspace --test '*'

# Specific test suite
cargo test --test pubky_compliance
cargo test --test noise_integration
```

### With Verbose Output

```bash
RUST_LOG=debug cargo test -- --nocapture
```

## Logging

Enable structured logging with the `--verbose` flag:

```bash
# Debug a specific command
./paykit-demo --verbose publish --lightning "lnbc..."

# Or set environment variable
RUST_LOG=paykit_demo_cli=debug ./paykit-demo publish ...
```

### Log Levels
- `error`: Failures only
- `warn`: Potential issues
- `info`: High-level operations (default with --verbose)
- `debug`: Detailed execution flow
- `trace`: Very detailed (all subsystems)

## Examples

### Complete Workflow: Alice → Bob

```bash
# Alice sets up
./paykit-demo setup --name Alice
ALICE_KEY=$(./paykit-demo whoami --format pubkey)

# Bob sets up
./paykit-demo setup --name Bob  
BOB_KEY=$(./paykit-demo whoami --format pubkey)

# Bob publishes his payment method
./paykit-demo publish \
  --lightning "lnbc1000..." \
  --homeserver "pubky://..."

# Alice discovers Bob's methods
./paykit-demo discover "pubky://$BOB_KEY"

# Alice sends a payment request to Bob
./paykit-demo subscriptions request \
  --recipient "pubky://$BOB_KEY" \
  --amount 1000 \
  --currency SAT \
  --description "Coffee"

# Bob lists and accepts the request
./paykit-demo subscriptions list --direction received
./paykit-demo subscriptions respond --request-id [...] --accept
```

### Subscription Workflow

```bash
# Alice proposes a monthly subscription to Bob
./paykit-demo subscriptions propose \
  --recipient "pubky://$BOB_KEY" \
  --amount 10000 \
  --currency SAT \
  --frequency monthly:1 \
  --description "Premium membership"

# Bob accepts the subscription
./paykit-demo subscriptions list
./paykit-demo subscriptions accept --subscription-id [...]

# Alice enables auto-pay for Bob's subscription
./paykit-demo subscriptions enable-auto-pay \
  --subscription-id [subscription-id] \
  --max-amount 15000

# Alice sets spending limits
./paykit-demo subscriptions set-limit \
  --peer "pubky://$BOB_KEY" \
  --limit 20000 \
  --period monthly
```

## Storage

All data is stored in:
- **macOS**: `~/Library/Application Support/paykit-demo/`
- **Linux**: `~/.local/share/paykit-demo/`
- **Windows**: `%APPDATA%\paykit-demo\`
- **Custom**: Set `PAYKIT_DEMO_DIR` environment variable

### Storage Structure

```
paykit-demo/
├── identities/           # Ed25519 keypairs (JSON)
│   ├── alice.json
│   └── bob.json
├── data/
│   ├── data.json        # Contacts and receipts
│   └── subscriptions/   # Subscription data
│       ├── requests/     # Payment requests
│       ├── subscriptions/ # Subscription agreements
│       ├── autopay_rules/ # Auto-pay rules
│       └── peer_limits/  # Spending limits
└── .current_identity    # Active identity marker
```

## Configuration

### Homeserver

The CLI needs a Pubky homeserver for directory operations. You can:

1. **Use a public homeserver** (when available)
2. **Run a local testnet** (for development):
   ```bash
   # In a separate terminal
   cargo run --bin pubky-testnet
   ```

### Custom Data Directory

```bash
export PAYKIT_DEMO_DIR=/path/to/data
./paykit-demo setup --name Alice
```

## Troubleshooting

### "No identity found"
Run `./paykit-demo setup --name YourName` first.

### "Failed to connect to homeserver"
- Check the homeserver URL format (must be `pubky://[public-key]`)
- Verify the homeserver is running
- Check network connectivity

### "Method not found"
Ensure the recipient has published their payment methods with `publish` command.

### Test Failures
```bash
# Clean and rebuild
cargo clean
cargo build --release

# Run tests with detailed output
RUST_LOG=debug cargo test -- --nocapture
```

## Development

### Run from Source

```bash
cargo run -- setup --name Dev
cargo run -- --verbose whoami
```

### Watch Mode

```bash
cargo watch -x check -x test
```

### Format and Lint

```bash
cargo fmt
cargo clippy --all-targets
```

## Help

For detailed command help:

```bash
# General help
./paykit-demo --help

# Command-specific help
./paykit-demo publish --help
./paykit-demo subscriptions --help
./paykit-demo subscriptions send-request --help
```

## Support

- **Documentation**: See [README.md](./README.md) for complete command reference
- **Testing**: See [TESTING.md](./TESTING.md) for testing guide
- **Troubleshooting**: See [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) for common issues
- **Architecture**: See `paykit-demo-core/` for implementation details


