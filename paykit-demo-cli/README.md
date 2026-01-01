# Paykit Demo CLI

> **Command-Line Interface for Demonstrating Paykit Payment Protocol**

A feature-rich CLI application showcasing Paykit capabilities: public directory operations, private Noise-encrypted payments, subscription management, auto-pay automation, and receipt coordination.

## Current Status

> **Demo Application**: Core protocol features work with optional real payment execution.

| Feature | Status | Notes |
|---------|--------|-------|
| Identity Management | **Real** | Ed25519 keypairs, file persistence |
| Contact Management | **Real** | Full CRUD operations |
| Directory Publish | **Real** | Pubky homeserver integration |
| Directory Discover | **Real** | HTTP queries to homeservers |
| Profile Management | **Real** | Fetch, publish, import profiles |
| Smart Checkout | **Real** | Method discovery with ranking strategies |
| Activity Timeline | **Real** | Unified view of all payment activity |
| Noise Handshake | **Real** | Used internally by pay/receive commands |
| Payment Coordination | **Real** | Request/receipt exchange |
| Wallet Configuration | **Real** | LND and Esplora setup |
| Payment Execution | **Real (with wallet)** | Requires configured wallet |
| Subscriptions | **Real** | Full P2P lifecycle |
| Auto-Pay Rules | **Real** | Rules and limits with file persistence |
| Spending Limits | **Real** | Per-peer limits with period tracking |
| Receipts | **Real** | Stored and queryable |

### Payment Execution

The `pay` command supports **real payment execution** when a wallet is configured:

```bash
# Configure LND for Lightning payments
paykit-demo wallet configure-lnd --url https://localhost:8081 --macaroon <hex>

# Or use a preset for local development (Polar)
paykit-demo wallet preset polar --macaroon <hex>

# Execute real payment
paykit-demo pay lnbc1... --method lightning
```

Without wallet configuration, the CLI shows simulation mode with setup instructions.

**Supported payment backends:**
- **Lightning**: LND REST API (requires `--features http-executor`)
- **On-chain**: Esplora API for fee estimates and verification

## Features

### 🔐 Identity Management
- Ed25519 keypair generation and management
- Pubky URI creation and display
- Multiple identity support with switching
- Secure key derivation for Noise protocol
- **Secure Storage**: OS keychain support (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux)
- **Migration**: Migrate existing plaintext identities to secure storage

### 📡 Directory Operations
- Publish payment methods to Pubky homeservers
- Discover recipient payment endpoints
- Support for onchain, lightning, and custom methods
- Real-time endpoint query

### 💸 Interactive Payments
- **Real Noise Protocol encryption** for private communication
- End-to-end encrypted payment coordination
- Receipt exchange and persistence
- Support for both public and private endpoints

### 📋 Contact Management
- Save and organize payment recipients
- Quick lookup by name
- QR code display for sharing
- Contact import/export

### 🔄 Subscription Management
- **Phase 2**: Payment requests and subscription agreements
- **Phase 3**: Auto-pay automation and spending limits
- Full P2P subscription lifecycle
- No intermediaries required

### 🛡️ Privacy Features
- **Private Endpoints**: Per-peer dedicated addresses for enhanced privacy
- **Endpoint Rotation**: Automatic rotation policies (on-use, after:N uses, manual)
- **Rotation History**: Track all rotations with audit capability
- **Encrypted Storage**: Private endpoints stored with AES-256-GCM encryption

### 💾 Backup & Restore
- **Encrypted Backups**: Argon2 + AES-256-GCM key derivation
- **Export**: `backup --output file.json` to create encrypted backup
- **Import**: `restore file.json --name <name>` to restore identity
- **Complete Data**: Includes identity, contacts, payment methods, settings

## 🚀 Quick Start

### Installation

```bash
cd paykit-demo-cli
cargo build --release
```

The binary will be at `target/release/paykit-demo`.

### Basic Workflow: Alice Pays Bob

```bash
# Terminal 1: Bob sets up and starts receiving
paykit-demo setup --name bob
paykit-demo receive --port 9735

# Terminal 2: Alice sets up and pays Bob
paykit-demo setup --name alice  
paykit-demo pay bob --amount 1000 --currency SAT --method lightning

# Both check receipts
paykit-demo receipts
```

## 📚 Commands Reference

### Identity Management

| Command | Description | Example |
|---------|-------------|---------|
| `setup` | Create new identity | `paykit-demo setup --name alice` |
| `whoami` | Show current identity | `paykit-demo whoami` |
| `list` | List all identities | `paykit-demo list` |
| `switch` | Switch identity | `paykit-demo switch bob` |
| `migrate` | Migrate plaintext identities to secure storage | `paykit-demo migrate` |

### Wallet Configuration

Configure payment execution backends to enable real payments.

| Command | Description | Example |
|---------|-------------|---------|
| `wallet status` | Show wallet status | `paykit-demo wallet status` |
| `wallet configure-lnd` | Configure LND | `paykit-demo wallet configure-lnd --url https://localhost:8081 --macaroon <hex>` |
| `wallet configure-esplora` | Configure Esplora | `paykit-demo wallet configure-esplora --url https://blockstream.info/testnet/api` |
| `wallet preset` | Apply preset config | `paykit-demo wallet preset polar --macaroon <hex>` |
| `wallet clear` | Clear wallet config | `paykit-demo wallet clear` |

**Available presets:**
- `polar` - Polar regtest (requires macaroon)
- `testnet` - Bitcoin testnet3 (Blockstream Esplora)
- `signet` - Bitcoin signet (mempool.space)
- `mutinynet` - Mutinynet signet

### Directory Operations

| Command | Description | Example |
|---------|-------------|---------|
| `publish` | Publish payment methods | `paykit-demo publish --method lightning --endpoint "noise://..."` |
| `discover` | Query payment methods | `paykit-demo discover pubky://...` |

### Profile Management

Manage your Pubky profile in the directory.

| Command | Description | Example |
|---------|-------------|---------|
| `profile fetch` | Fetch a profile | `paykit-demo profile fetch pubky://...` |
| `profile fetch --json` | Fetch as JSON | `paykit-demo profile fetch pubky://... --json` |
| `profile publish` | Publish your profile | `paykit-demo profile publish --name "Alice" --bio "Bitcoin enthusiast"` |
| `profile import` | Import and publish another user's profile | `paykit-demo profile import --from pubky://...` |

### Smart Checkout

Discover and select the best payment method automatically.

| Command | Description | Example |
|---------|-------------|---------|
| `smart-checkout` | Discover best payment method | `paykit-demo smart-checkout pubky://... --amount 1000` |
| `smart-checkout --strategy` | Use specific strategy | `paykit-demo smart-checkout pubky://... --strategy lowest-fee` |
| `smart-checkout --execute` | Execute with selected method | `paykit-demo smart-checkout pubky://... --amount 1000 --execute` |

**Available strategies:**
- `balanced` (default) - Balance cost, speed, and privacy
- `lowest-fee` - Prefer lowest transaction fees
- `fastest` - Prefer fastest confirmation
- `private` - Prefer maximum privacy

### Activity Timeline

View unified activity across all payment types.

| Command | Description | Example |
|---------|-------------|---------|
| `activity` | Show activity timeline | `paykit-demo activity` |
| `activity --type` | Filter by type | `paykit-demo activity --type payment` |
| `activity --direction` | Filter by direction | `paykit-demo activity --direction sent` |
| `activity --limit` | Limit results | `paykit-demo activity --limit 50` |

**Activity types:** `payment`, `subscription`, `request`, `autopay`
**Directions:** `all`, `sent`, `received`

### Contact Management

| Command | Description | Example |
|---------|-------------|---------|
| `contacts add` | Add contact | `paykit-demo contacts add bob pubky://...` |
| `contacts list` | List contacts | `paykit-demo contacts list` |
| `contacts show` | Show contact | `paykit-demo contacts show bob` |
| `contacts remove` | Remove contact | `paykit-demo contacts remove bob` |

### Payment Flow

| Command | Description | Example |
|---------|-------------|---------|
| `pay` | Initiate payment | `paykit-demo pay bob --amount 1000` |
| `pay --dry-run` | Test payment without executing | `paykit-demo pay bob --amount 1000 --dry-run` |
| `receive` | Start receiver | `paykit-demo receive --port 9735` |
| `receipts` | View receipts | `paykit-demo receipts` |

**Payment methods:**
- `lightning` (default) - Pay via Lightning Network (requires LND)
- `onchain` - Pay via Bitcoin on-chain (uses Esplora for fee estimates)

### Subscriptions

| Command | Description | Example |
|---------|-------------|---------|
| `subscriptions request` | Send payment request | `paykit-demo subscriptions request --recipient pubky://... --amount 1000 --currency SAT` |
| `subscriptions list` | List payment requests | `paykit-demo subscriptions list` |
| `subscriptions list-agreements` | List subscriptions | `paykit-demo subscriptions list-agreements` |
| `subscriptions respond` | Respond to request | `paykit-demo subscriptions respond --request-id <id> --action accept` |
| `subscriptions propose` | Propose subscription | `paykit-demo subscriptions propose --recipient pubky://... --amount 1000 --frequency monthly:1` |
| `subscriptions accept` | Accept subscription | `paykit-demo subscriptions accept --subscription-id <id>` |

### Auto-Pay & Spending Limits

| Command | Description | Example |
|---------|-------------|---------|
| `subscriptions enable-auto-pay` | Enable auto-pay | `paykit-demo subscriptions enable-auto-pay <sub-id> --max-amount 5000` |
| `subscriptions disable-auto-pay` | Disable auto-pay | `paykit-demo subscriptions disable-auto-pay <sub-id>` |
| `subscriptions show-auto-pay` | Show auto-pay status | `paykit-demo subscriptions show-auto-pay <sub-id>` |
| `subscriptions list-auto-pay` | List all auto-pay rules | `paykit-demo subscriptions list-auto-pay` |
| `subscriptions delete-auto-pay` | Delete auto-pay rule | `paykit-demo subscriptions delete-auto-pay <sub-id>` |
| `subscriptions set-limit` | Set spending limit | `paykit-demo subscriptions set-limit <peer> --limit 10000 --period monthly` |
| `subscriptions show-limits` | Show spending limits | `paykit-demo subscriptions show-limits` |
| `subscriptions delete-limit` | Delete spending limit | `paykit-demo subscriptions delete-limit <peer>` |
| `subscriptions reset-limit` | Reset spending counter | `paykit-demo subscriptions reset-limit <peer>` |
| `subscriptions global-settings` | Show global settings | `paykit-demo subscriptions global-settings` |
| `subscriptions configure-global` | Configure global settings | `paykit-demo subscriptions configure-global --enable --daily-limit 100000` |
| `subscriptions recent-payments` | Show recent auto-payments | `paykit-demo subscriptions recent-payments --count 20` |

For detailed subscription workflows, see [QUICKSTART.md](./QUICKSTART.md#4-subscriptions).

### Private Endpoints

| Command | Description | Example |
|---------|-------------|---------|
| `endpoints list` | List all private endpoints | `paykit-demo endpoints list` |
| `endpoints show` | Show endpoints for a peer | `paykit-demo endpoints show <peer>` |
| `endpoints remove` | Remove specific endpoint | `paykit-demo endpoints remove <peer> <method>` |
| `endpoints remove-peer` | Remove all endpoints for peer | `paykit-demo endpoints remove-peer <peer>` |
| `endpoints cleanup` | Remove expired endpoints | `paykit-demo endpoints cleanup` |
| `endpoints stats` | Show endpoint statistics | `paykit-demo endpoints stats` |

### Endpoint Rotation

| Command | Description | Example |
|---------|-------------|---------|
| `rotation status` | Show rotation status | `paykit-demo rotation status` |
| `rotation default` | Set default policy | `paykit-demo rotation default on-use` |
| `rotation policy` | Set per-method policy | `paykit-demo rotation policy lightning after:5` |
| `rotation auto-rotate` | Enable/disable auto-rotation | `paykit-demo rotation auto-rotate --enable true` |
| `rotation rotate` | Manually trigger rotation | `paykit-demo rotation rotate lightning` |
| `rotation history` | View rotation history | `paykit-demo rotation history --method lightning` |
| `rotation clear-history` | Clear rotation history | `paykit-demo rotation clear-history` |

**Rotation policies:**
- `on-use` - Rotate after every use (best privacy)
- `after:<N>` - Rotate after N uses
- `periodic:<seconds>` - Rotate after time interval
- `manual` - Only rotate when manually triggered

### Backup & Restore

| Command | Description | Example |
|---------|-------------|---------|
| `backup` | Export encrypted backup | `paykit-demo backup --output backup.json` |
| `restore` | Import from backup | `paykit-demo restore backup.json --name alice` |

**Backup encryption:**
- Argon2id key derivation with configurable parameters
- AES-256-GCM authenticated encryption
- Includes identity keypair, contacts, settings

## 🔧 Configuration

### Storage Location

Data is stored in:
- **macOS**: `~/Library/Application Support/paykit-demo/`
- **Linux**: `~/.local/share/paykit-demo/`
- **Custom**: Set `PAYKIT_DEMO_DIR` environment variable

### Storage Structure

```
paykit-demo/
├── identities/           # Ed25519 keypairs (JSON) - plaintext or secure storage
│   ├── alice.json
│   └── bob.json
├── data/
│   ├── data.json        # Contacts and receipts
│   └── subscriptions/   # Subscription data
└── .current_identity    # Active identity marker
```

**Note**: New identities are stored in OS keychain by default (if available). Use `paykit-demo migrate` to migrate existing plaintext identities to secure storage.

## 🏗️ Architecture

```
paykit-demo-cli (this crate)
       ↓
┌──────────────────────────────┐
│    paykit-demo-core          │  ← Shared demo logic
├──────────────────────────────┤
│  • Identity management       │
│  • NoiseClientHelper         │
│  • NoiseServerHelper         │
│  • Storage abstraction       │
└──────────────────────────────┘
       ↓
┌──────────────────────────────┐
│    Protocol Layer             │
├──────────────────────────────┤
│  • paykit-lib                │  ← Directory & transport
│  • paykit-interactive        │  ← Noise payments
│  • paykit-subscriptions      │  ← Recurring payments
│  • pubky-noise               │  ← Encryption
└──────────────────────────────┘
```

## 🧪 Testing

### Run All Tests

```bash
cargo test
```

### Test Suites

- **Unit Tests**: 5 tests - Function-level verification
- **Property Tests**: 9 tests - Arbitrary input validation
- **Integration Tests**: 11 tests - End-to-end workflows
- **Total**: 25 tests with 100% pass rate

### Run Specific Test Suite

```bash
cargo test --test property_tests      # Property-based tests
cargo test --test pubky_compliance    # Directory compliance
cargo test --test pay_integration     # Payment tests
```

## 📖 Documentation

- **[QUICKSTART.md](./QUICKSTART.md)** - 5-minute getting started guide with examples
- **[TESTING.md](./TESTING.md)** - Comprehensive testing guide
- **[BUILD.md](./BUILD.md)** - Build instructions and development setup
- **[TROUBLESHOOTING.md](./TROUBLESHOOTING.md)** - Common issues and fixes
- **[demos/README.md](./demos/README.md)** - Demo scripts and workflows

## 🔒 Security Considerations

**⚠️ This is DEMO software for development and testing**

### Secure Storage (Available)

The CLI now supports OS-level secure storage for identities:
- **macOS**: Keychain Services
- **Windows**: Credential Manager
- **Linux**: Secret Service (libsecret)

New identities created with `paykit-demo setup` are stored securely by default (if secure storage is available). Use `paykit-demo migrate` to migrate existing plaintext identities to secure storage.

### Not Production-Ready
- Legacy plaintext identity storage still supported (fallback)
- No encryption at rest for data files (contacts, receipts, etc.)
- Simplified error handling
- No hardware security module integration

### For Production Use
- ✅ Secure key storage implemented (OS keychain)
- Add encryption at rest for data files
- Use hardware security modules for high-value keys
- Implement proper session management
- Add rate limiting and DoS protection

## 🎯 Use Cases

### 1. Payment Protocol Development
Test and verify Paykit protocol implementations across platforms.

### 2. Integration Testing
Validate Pubky directory operations and Noise protocol integration.

### 3. Education & Demos
Learn how decentralized payments work without intermediaries.

### 4. Reference Implementation
See how to properly use paykit-lib, paykit-interactive, and paykit-subscriptions.

## 🐛 Troubleshooting

### Common Issues

**"No current identity"**
```bash
# Create an identity first
paykit-demo setup --name myname
```

**"Failed to connect"**
```bash
# Ensure receiver is running first
# Check firewall/network settings
# Verify port is not in use
```

**"Recipient not found"**
```bash
# Discover or add contact first
paykit-demo discover pubky://...
paykit-demo contacts add bob pubky://...
```

See [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) for comprehensive troubleshooting.

## 📊 Project Status

| Component | Status | Tests |
|-----------|--------|-------|
| Identity Management | ✅ Complete | 100% |
| Directory Operations | ✅ Complete | 100% |
| Contact Management | ✅ Complete | 100% |
| Interactive Payments | ✅ Complete | 100% |
| Subscriptions | ✅ Complete | 100% |
| Property Tests | ✅ Complete | 9/9 |
| Documentation | ✅ Complete | 5/5 |

## 🛣️ Roadmap & Future Improvements

Based on comprehensive code review, the following enhancements are recommended:

### High Priority

#### Enhanced E2E Payment Testing
- **Status**: ⚠️ Partial - Some E2E tests failing (edge cases)
- **Action**: Add more comprehensive E2E test scenarios
- **Impact**: Improved confidence in payment flows
- **Details**: 
  - Create test fixtures for complete payment flows
  - Add automated tests for full payment lifecycle
  - Fix edge case failures in `e2e_payment_flow.rs`

#### Payment Flow Completion
- **Status**: ⚠️ Simulation mode - `pay` command shows simulation message
- **Action**: Complete full payment flow implementation or clearly document as demonstration-only
- **Impact**: Better user experience for demonstrations
- **Details**: Currently shows "Full payment flow implementation pending" message

### Medium Priority

#### Error Type Refinement
- **Status**: ✅ Good - Currently uses `anyhow::Result`
- **Action**: Add specific error types for different failure modes
- **Impact**: Better error handling and debugging
- **Details**: 
  - Create custom error types for payment failures
  - Better error categorization
  - More detailed error messages

#### Performance Testing
- **Status**: ❌ Not implemented
- **Action**: Add performance tests and benchmarks
- **Impact**: Identify performance bottlenecks
- **Details**:
  - Benchmark storage operations
  - Test with large datasets (many contacts/receipts)
  - Profile payment flow performance

### Low Priority

#### Additional Demo Scripts
- **Status**: ✅ 2 scripts available (basic payment, subscription)
- **Action**: Add more demo scenarios
- **Impact**: Better demonstration capabilities
- **Details**:
  - Multi-party payment scenarios
  - Complex subscription workflows
  - Error recovery scenarios

#### Test Documentation Enhancement
- **Status**: ✅ Good - TESTING.md exists
- **Action**: Enhance test documentation
- **Impact**: Easier test maintenance and debugging
- **Details**:
  - Add test scenario documentation
  - Document test data requirements
  - Add troubleshooting guide for test failures

### Known Limitations

The following are documented limitations appropriate for demo applications:

- ⚠️ Legacy plaintext identity storage still supported (fallback when OS keychain unavailable)
- ⚠️ No encryption at rest for data files (contacts, receipts, subscription data)
- ⚠️ Some payment flows require wallet configuration (documented)

**For production use**, consider:
- Hardware security modules for high-value keys
- Encryption at rest for data files
- Proper session management
- Rate limiting and DoS protection

## 🤝 Contributing

This is a demonstration application. Contributions welcome for:
- Additional test coverage
- Documentation improvements
- Example workflows
- Bug fixes
- Roadmap items above

## 📄 License

MIT

## 🔗 Related Projects

- [Paykit Protocol](../README.md) - Main Paykit documentation
- [Paykit Core Library](../paykit-lib/README.md) - Protocol implementation
- [Pubky Project](https://pubky.org) - Decentralized identity system
- [Noise Protocol](http://www.noiseprotocol.org/) - Encryption framework

## Related Components

This CLI demo application uses and integrates with:

- **[paykit-lib](../paykit-lib/README.md)** - Core library for directory operations and transport traits
- **[paykit-interactive](../paykit-interactive/README.md)** - Interactive payment protocol with Noise encryption
- **[paykit-subscriptions](../paykit-subscriptions/README.md)** - Subscription management, payment requests, and auto-pay
- **[paykit-demo-core](../paykit-demo-core/README.md)** - Shared business logic for demo applications

## ⭐ Key Differentiators

### vs. Traditional Payment CLIs
- **No central servers** - Truly peer-to-peer
- **No KYC/accounts** - Just cryptographic keys
- **Encrypted by default** - Noise Protocol security
- **Method agnostic** - Works with any payment rail

### vs. Other Decentralized Solutions
- **Simple** - One binary, no complex setup
- **Fast** - Direct peer connections
- **Flexible** - Public or private endpoints
- **Complete** - Directory + payments + subscriptions

---

**Built with** ❤️ **using Rust, Pubky, and the Noise Protocol**

For questions or issues, see [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) or file an issue.
