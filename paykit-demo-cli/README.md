# Paykit Demo CLI

> **Production-Quality Command-Line Interface for Demonstrating Paykit Payment Protocol**

A feature-complete CLI application showcasing all Paykit capabilities: public directory operations, private Noise-encrypted payments, subscription management, auto-pay automation, and receipt coordination.

## ✨ Features

### 🔐 Identity Management
- Ed25519 keypair generation and management
- Pubky URI creation and display
- Multiple identity support with switching
- Secure key derivation for Noise protocol

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

### Directory Operations

| Command | Description | Example |
|---------|-------------|---------|
| `publish` | Publish payment methods | `paykit-demo publish --method lightning --endpoint "noise://..."` |
| `discover` | Query payment methods | `paykit-demo discover pubky://...` |

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
| `receive` | Start receiver | `paykit-demo receive --port 9735` |
| `receipts` | View receipts | `paykit-demo receipts` |

### Subscriptions

| Command | Description | Example |
|---------|-------------|---------|
| `subscriptions request` | Send payment request | `paykit-demo subscriptions request --recipient pubky://... --amount 1000 --currency SAT` |
| `subscriptions list` | List payment requests | `paykit-demo subscriptions list` |
| `subscriptions list-agreements` | List subscriptions | `paykit-demo subscriptions list-agreements` |
| `subscriptions respond` | Respond to request | `paykit-demo subscriptions respond --request-id <id> --action accept` |
| `subscriptions propose` | Propose subscription | `paykit-demo subscriptions propose --recipient pubky://... --amount 1000 --frequency monthly:1` |
| `subscriptions accept` | Accept subscription | `paykit-demo subscriptions accept --subscription-id <id>` |
| `subscriptions enable-auto-pay` | Enable auto-pay | `paykit-demo subscriptions enable-auto-pay --subscription-id <id> --max-amount 5000` |
| `subscriptions set-limit` | Set spending limits | `paykit-demo subscriptions set-limit --peer pubky://... --limit 10000 --period monthly` |
| `subscriptions show-limits` | Show spending limits | `paykit-demo subscriptions show-limits` |

For detailed subscription workflows, see [QUICKSTART.md](./QUICKSTART.md#4-subscriptions).

## 🔧 Configuration

### Storage Location

Data is stored in:
- **macOS**: `~/Library/Application Support/paykit-demo/`
- **Linux**: `~/.local/share/paykit-demo/`
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
└── .current_identity    # Active identity marker
```

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

### Not Production-Ready
- Private keys stored in **plaintext JSON files**
- No encryption at rest
- No OS keychain integration
- Simplified error handling

### For Production Use
- Implement secure key storage (Keychain/KeyStore/Credential Manager)
- Add key encryption at rest
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

## 🤝 Contributing

This is a demonstration application. Contributions welcome for:
- Additional test coverage
- Documentation improvements
- Example workflows
- Bug fixes

## 📄 License

MIT

## 🔗 Related Projects

- [Paykit Protocol](../README.md) - Main Paykit documentation
- [Paykit Core Library](../paykit-lib/README.md) - Protocol implementation
- [Pubky Project](https://pubky.org) - Decentralized identity system
- [Noise Protocol](http://www.noiseprotocol.org/) - Encryption framework

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
