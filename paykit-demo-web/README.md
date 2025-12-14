# Paykit Demo Web

**Browser Application for Demonstrating Paykit Payment Protocol**

A WebAssembly application demonstrating Paykit capabilities in the browser: identity management, directory discovery, subscription management, auto-pay automation, and real-time encrypted payments via WebSocket-based Noise protocol.

## Current Status

> **Demo Application**: Most features are fully implemented. Directory publishing now supports multiple modes.

| Feature | Status | Notes |
|---------|--------|-------|
| Identity Management | **Real** | Ed25519 keypairs, localStorage persistence |
| Contact Management | **Real** | Full CRUD, localStorage persistence |
| Receipt Management | **Real** | Full history with filtering, localStorage |
| Dashboard | **Real** | Statistics from real stored data |
| Noise Payments | **Real** | WebSocket-based encrypted payments |
| Payment Methods | **Real** | Configured locally with real publishing options |
| Directory Publish | **Configurable** | Mock, Direct, or Proxy modes |
| Directory Discover | **Real** | HTTP queries to homeservers |
| Subscriptions | **Real** | Full P2P lifecycle, localStorage |
| Auto-Pay | **Real** | Rules and limits, localStorage |
| Spending Limits | **Real** | Per-peer limits with period reset |

### Directory Publishing Modes

The `DirectoryClient` now supports three publishing modes to work around browser CORS restrictions:

| Mode | Description | Use Case |
|------|-------------|----------|
| **Mock** | Saves to localStorage only (default) | Development/testing |
| **Direct** | HTTP PUT directly to homeserver | Homeserver with CORS headers |
| **Proxy** | Routes through a CORS proxy | Production without CORS headers |

#### Usage Examples

```javascript
// Mock mode (default) - for development
const mockClient = new DirectoryClient("https://homeserver.example.com");
await mockClient.publishEndpoint(publicKey, "lightning", "lnurl1...", null);
// Result: Saved to localStorage only

// Direct mode - requires CORS-enabled homeserver
const directClient = DirectoryClient.withDirectAccess("https://cors-enabled.example.com");
await directClient.publishEndpoint(publicKey, "lightning", "lnurl1...", authToken);
// Result: Real HTTP PUT to homeserver

// Proxy mode - uses a CORS proxy
const proxyClient = DirectoryClient.withProxy(
    "https://homeserver.example.com",
    "https://cors-proxy.example.com"
);
await proxyClient.publishEndpoint(publicKey, "lightning", "lnurl1...", authToken);
// Result: Request routed through proxy to homeserver
```

#### Bulk Publishing

```javascript
const storage = new WasmPaymentMethodStorage();
const client = DirectoryClient.withProxy(homeserver, proxy);

// Publish all public methods at once
const result = await storage.publishToDirectory(client, publicKey, authToken);
console.log(result); // "Published 3 method(s) successfully, 0 failed."

// Remove all published methods
await storage.unpublishFromDirectory(client, publicKey, authToken);
```

## Features

### 🏠 Dashboard
- Unified overview of all Paykit features
- Real-time statistics (contacts, methods, receipts, subscriptions)
- Setup progress tracker with visual checklist
- Quick action buttons for common tasks
- Recent activity feed
- Getting started guide for new users
- See [DASHBOARD.md](./DASHBOARD.md) for details

### 🔐 Identity Management
- Ed25519 keypair generation and management
- Pubky URI creation and display
- Multiple identity support with switching
- Browser localStorage persistence
- Import/export capabilities
- **Security Settings**: Password-based encrypted storage (IndexedDB + SubtleCrypto)
- Password strength meter and secure key derivation

### 👥 Contact Management
- Address book for Pubky peers
- Add, edit, delete contacts
- Search contacts by name
- Payment history tracking per contact
- Notes and metadata
- Import contacts from Pubky follows
- See [CONTACTS_FEATURE.md](./CONTACTS_FEATURE.md) for details

### 💳 Payment Methods
- Configure your payment endpoints (Lightning, Onchain, Custom)
- Priority ordering and preferred method selection
- Public/private visibility controls
- Multiple publishing modes (Mock, Direct, Proxy)
- **Endpoint Rotation**: Configurable rotation policies for privacy
- Auto-rotation after payments (on-use, after:N uses, manual)
- See [PAYMENT_METHODS.md](./PAYMENT_METHODS.md) for details

### 🧾 Receipt Management
- View all payment transaction history
- Filter by direction (sent/received), method, or contact
- Statistics dashboard (total, sent, received)
- Export receipts as JSON
- Delete individual receipts or clear all
- Local persistence in browser storage
- See [RECEIPTS.md](./RECEIPTS.md) for details

### 📡 Directory Operations
- Publish payment methods to Pubky homeservers
- Discover recipient payment endpoints
- Support for onchain, lightning, and custom methods
- Real-time endpoint queries via HTTP

### 💸 Interactive Payments
- **Real WebSocket-based Noise Protocol** encryption
- End-to-end encrypted payment coordination
- Automatic endpoint discovery via directory
- Receipt exchange and persistence
- Support for both public and private endpoints
- Real-time payment status updates
- Full error handling and user feedback
- See [PAYMENTS.md](./PAYMENTS.md) for complete guide

### 📋 Subscription Management
- **Phase 2**: Payment requests and subscription agreements
- **Phase 3**: Auto-pay automation and spending limits
- Full P2P subscription lifecycle
- No intermediaries required
- Browser-based storage

### 🔄 Auto-Pay Features
- Enable auto-pay for subscriptions with configurable rules
- Set maximum payment amounts per subscription
- Require manual confirmation before each payment (optional)
- Enable/disable auto-pay rules dynamically
- View and manage all auto-pay configurations

### 💰 Spending Limits (Allowances)
- Set spending limits per peer (daily, weekly, monthly)
- Track current spending against limits
- Visual progress indicators
- Automatic period reset
- Prevent exceeding configured limits

## 📚 Documentation

### Complete Guides

- **[API Reference](./API_REFERENCE.md)** - Complete API documentation for all WASM bindings
- **[Architecture](./ARCHITECTURE.md)** - System architecture and design decisions
- **[Deployment](./DEPLOYMENT.md)** - Build and deployment instructions
- **[Testing Guide](./TESTING.md)** - Comprehensive testing documentation

### Feature Documentation

- **[Payment Methods](./PAYMENT_METHODS.md)** - Payment method configuration guide
- **[Receipt Management](./RECEIPTS.md)** - Receipt viewing and filtering guide
- **[Dashboard](./DASHBOARD.md)** - Dashboard overview and usage
- **[Contacts Feature](./CONTACTS_FEATURE.md)** - Contact management guide
- **[Subscriptions](./SUBSCRIPTION_IMPLEMENTATION.md)** - Subscription management guide

### Quick Start

- **[START_HERE.md](./START_HERE.md)** - Quick setup guide
- **[BUILD_INSTRUCTIONS.md](./BUILD_INSTRUCTIONS.md)** - Detailed build guide
- **[TESTING.md](./TESTING.md)** - How to run tests

### Project History

- **[CHANGELOG.md](./CHANGELOG.md)** - Complete implementation history and version changes

## 🚀 Quick Start

### Prerequisites

You must use **Rustup**, not Homebrew Rust:

```bash
# Verify you have Rustup
which rustc  # Should show ~/.cargo/bin/rustc

# If you have Homebrew Rust, remove it first:
brew uninstall rust

# Install Rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack
```

### Building

```bash
cd paykit-demo-web

# Build for development
wasm-pack build --target web --out-dir www/pkg

# OR build for production (optimized)
wasm-pack build --target web --release --out-dir www/pkg
```

### Running

```bash
# Start development server
python3 -m http.server 8080 -d www

# Open in browser
open http://localhost:8080
```

## 📚 Architecture

### WebSocket Noise Transport

Unlike the CLI which uses TCP directly, the web demo uses WebSocket as the transport layer for Noise protocol:

```
Browser                    WebSocket Server              TCP Endpoint
  |                              |                              |
  |-- WebSocket Connection ----->|                              |
  |                              |-- TCP Connection ----------->|
  |<-- Noise IK Handshake ------>|<-- Noise IK Handshake ----->|
  |<-- Encrypted Messages ------>|<-- Encrypted Messages ------>|
```

**Key Features:**
- Same Noise_IK handshake pattern as TCP version
- Length-prefixed encrypted messages
- Binary WebSocket frames for efficiency
- Automatic reconnection handling

### WASM-Compatible Design

All code is WASM-compatible using:
- `wasm-bindgen` for JavaScript interop
- `web-sys` for browser APIs (WebSocket, localStorage, etc.)
- `wasm-bindgen-futures` for async/await
- `futures-channel` for message queuing
- No `tokio` (uses browser's async runtime)

### Browser Storage

- **localStorage** for:
  - Identities (Ed25519 keypairs)
  - Subscriptions and agreements
  - Payment requests
  - Receipts
  - Auto-pay rules

**Limitations:**
- ~5-10MB storage limit (browser-dependent)
- No encryption at rest (demo purposes only)
- Quota detection with graceful errors

## 📖 API Reference

For complete API documentation, see [API_REFERENCE.md](./API_REFERENCE.md).

### Quick Examples

**Identity Management**:
```javascript
import { Identity, BrowserStorage } from './pkg/paykit_demo_web.js';

const identity = Identity.withNickname("alice");
const storage = new BrowserStorage();
storage.saveIdentity("alice", identity);
```

**Contact Management**:
```javascript
import { WasmContact, WasmContactStorage } from './pkg/paykit_demo_web.js';

const storage = new WasmContactStorage();
const contact = new WasmContact(pubkey, "Alice");
await storage.save_contact(contact);
```

**Payment Methods**:
```javascript
import { WasmPaymentMethodConfig, WasmPaymentMethodStorage } from './pkg/paykit_demo_web.js';

const storage = new WasmPaymentMethodStorage();
const method = new WasmPaymentMethodConfig("lightning", "lnurl...", true, true, 1);
await storage.save_method(method);
```

**Receipt Management**:
```javascript
import { WasmReceiptStorage } from './pkg/paykit_demo_web.js';

const storage = new WasmReceiptStorage();
const receipts = await storage.list_receipts();
const stats = await storage.get_statistics(myPubkey);
```

**Dashboard**:
```javascript
import { WasmDashboard } from './pkg/paykit_demo_web.js';

const dashboard = new WasmDashboard();
const stats = await dashboard.get_overview_stats(myPubkey);
const activity = await dashboard.get_recent_activity(myPubkey, 10);
```

See [API_REFERENCE.md](./API_REFERENCE.md) for complete API documentation.

## 🎯 Usage Examples

### Identity Management

```javascript
import init, { Identity } from './pkg/paykit_demo_web.js';

await init();

// Generate identity
const alice = Identity.withNickname("alice");
console.log("Alice URI:", alice.pubkyUri());

// Export/import
const json = alice.toJSON();
const restored = Identity.fromJSON(json);
```

### Directory Queries

```javascript
import { DirectoryClient } from './pkg/paykit_demo_web.js';

const client = new DirectoryClient("https://demo.httprelay.io");
const methods = await client.queryMethods(recipient_pubkey);

console.log("Available methods:", methods);
```

### Payment Requests

```javascript
import { WasmPaymentRequest } from './pkg/paykit_demo_web.js';

const request = new WasmPaymentRequest(
    alice.publicKey(),
    bob.publicKey(),
    "1000",      // amount in sats
    "SAT",       // currency
    "lightning"  // method
);

request = request
    .with_description("Coffee payment")
    .with_expiration(Date.now() + 3600);
```

### Subscriptions

```javascript
import { WasmSubscription } from './pkg/paykit_demo_web.js';

const subscription = new WasmSubscription(
    subscriber_pubkey,
    provider_pubkey,
    "10000",      // amount
    "SAT",        // currency
    "monthly:1",  // frequency (1st of month)
    "lightning",  // method
    "Netflix subscription"
);
```

### Payment Methods

```javascript
import { WasmPaymentMethodConfig, WasmPaymentMethodStorage } from './pkg/paykit_demo_web.js';

const storage = new WasmPaymentMethodStorage();

// Add a Lightning method
const lightning = new WasmPaymentMethodConfig(
    "lightning",
    "alice@getalby.com",
    true,   // is_public
    true,   // is_preferred
    1       // priority (1 = highest)
);

await storage.save_method(lightning);

// List all methods (sorted by priority)
const methods = await storage.list_methods();

// Mock publish (demo only)
await storage.mock_publish();
```

### Interactive Payments

```javascript
import { WasmPaymentCoordinator } from './pkg/paykit_demo_web.js';

const coordinator = new WasmPaymentCoordinator();

const receipt = await coordinator.initiatePayment(
    payer_identity_json,
    "wss://payment-server.example.com",
    payee_pubkey,
    server_static_key_hex,
    "1000",
    "SAT",
    "lightning"
);

console.log("Payment complete:", receipt);
```

## 🧪 Testing

### Running Tests

```bash
# All tests
wasm-pack test --headless --chrome

# Specific test file
wasm-pack test --headless --chrome --test payment_flow

# With output
wasm-pack test --headless --chrome -- --nocapture
```

### Test Organization

- **Unit tests**: In `src/*.rs` modules with `#[cfg(test)]`
- **Integration tests**: In `tests/*.rs` files
- **Property tests**: Edge cases and validation

**Test Suites:**
- `tests/payment_flow.rs` - Payment workflow tests
- `tests/subscription_lifecycle.rs` - Subscription management tests
- `tests/storage_persistence.rs` - localStorage operations tests

See [TESTING.md](./TESTING.md) for comprehensive testing guide.

## 📖 API Reference

### Core Types

**Identity**
- `new()` - Generate random identity
- `withNickname(name)` - Generate with nickname
- `publicKey()` - Get public key string
- `pubkyUri()` - Get Pubky URI
- `toJSON()` / `fromJSON(json)` - Serialization

**WasmPaymentRequest**
- `new(from, to, amount, currency, method)` - Create request
- `with_description(desc)` - Add description
- `with_expiration(timestamp)` - Add expiration
- Properties: `request_id`, `amount`, `currency`, `from`, `to`

**WasmSubscription**
- `new(subscriber, provider, amount, currency, frequency, method, desc)` - Create subscription
- Properties: `subscription_id`, `amount`, `currency`, `frequency`

**WasmPaymentMethodConfig**
- `new(method_id, endpoint, is_public, is_preferred, priority)` - Create payment method
- Properties: `method_id`, `endpoint`, `is_public`, `is_preferred`, `priority`

**WasmPaymentMethodStorage**
- `save_method(method)` - Save payment method
- `list_methods()` - List all methods (sorted by priority)
- `get_method(method_id)` - Get specific method
- `delete_method(method_id)` - Delete method
- `set_preferred(method_id, preferred)` - Update preferred status
- `update_priority(method_id, priority)` - Update priority
- `get_preferred_methods()` - Get only preferred methods
- `mock_publish()` - Mock publish (demo only)

**WasmReceiptStorage**
- `save_receipt(receipt_id, receipt_json)` - Save receipt
- `get_receipt(receipt_id)` - Get specific receipt
- `list_receipts()` - List all receipts
- `delete_receipt(receipt_id)` - Delete receipt
- `filter_by_direction(direction, pubkey)` - Filter by sent/received
- `filter_by_method(method)` - Filter by payment method
- `filter_by_contact(contact_pubkey, my_pubkey)` - Filter by contact
- `get_statistics(pubkey)` - Get receipt statistics
- `export_as_json()` - Export all receipts as JSON
- `clear_all()` - Clear all receipts

**WasmDashboard**
- `get_overview_stats(pubkey)` - Get comprehensive statistics
- `get_recent_activity(pubkey, limit)` - Get recent transactions
- `get_setup_checklist()` - Get setup progress
- `is_setup_complete()` - Check if setup is complete

**WasmPaymentCoordinator**
- `initiatePayment(...)` - Execute payment flow
- `getReceipts()` - List stored receipts

See [API_REFERENCE.md](./API_REFERENCE.md) for complete API documentation.

## 🏗️ Project Structure

```
paykit-demo-web/
├── src/
│   ├── lib.rs                   # Main WASM entry point
│   ├── types.rs                 # Core Paykit types (WASM-adapted)
│   ├── identity.rs              # Identity management
│   ├── contacts.rs              # Contact management
│   ├── directory.rs             # Directory client
│   ├── storage.rs               # Browser storage
│   ├── payment_methods.rs       # Payment method configuration
│   ├── payment.rs               # Payment coordination & receipts
│   ├── subscriptions.rs         # Subscription management
│   ├── websocket_transport.rs   # WebSocket Noise transport
│   └── utils.rs                 # Utilities
├── tests/
│   ├── contact_lifecycle.rs     # Contact tests
│   ├── payment_method_management.rs # Payment method tests
│   ├── receipt_management.rs    # Receipt tests
│   ├── payment_flow.rs          # Payment tests
│   ├── subscription_lifecycle.rs # Subscription tests
│   └── storage_persistence.rs   # Storage tests
├── www/
│   ├── index.html               # Web UI
│   ├── app.js                   # JavaScript application
│   ├── styles.css               # Styling
│   └── pkg/                     # Generated WASM artifacts
├── Cargo.toml                   # Rust dependencies
└── package.json                 # npm scripts
```

## 🚢 Deployment

### Building for Production

```bash
# Optimized build
wasm-pack build --target web --release --out-dir www/pkg

# Verify bundle size
ls -lh www/pkg/paykit_demo_web_bg.wasm
```

### Hosting

**Recommended Platforms:**
- **Netlify**: Drop `www/` folder
- **Vercel**: Deploy `www/` directory
- **GitHub Pages**: Push `www/` to `gh-pages` branch
- **Cloudflare Pages**: Connect repository, build directory = `www/`

**Requirements:**
- MIME type for `.wasm` files: `application/wasm`
- HTTPS required (for WebSocket secure connections)
- CORS headers if API server is separate domain

See [DEPLOYMENT.md](./DEPLOYMENT.md) for detailed deployment instructions.

## ⚠️ Security Considerations

**Demo Purposes Only:**
- Keys stored in plaintext localStorage
- No encryption at rest
- No secure key derivation for storage
- Not audited for production use

**For Production:**
- Use OS keychain or hardware security modules
- Implement encryption at rest
- Add session management
- Rate limiting and DoS protection
- Security audit required

For production security considerations, see the [Security Considerations](#-security-considerations) section above.

## 🔧 Troubleshooting

### Build Issues

**"can't find crate for `std`"**
→ Using Homebrew Rust; switch to Rustup

**"wasm32-unknown-unknown target not found"**
```bash
rustup target add wasm32-unknown-unknown
```

**"wasm-pack command not found"**
```bash
cargo install wasm-pack
```

### Runtime Issues

**Blank page in browser**
- Ensure serving over HTTP (not `file://`)
- Check browser console for errors
- Verify `pkg/` directory exists

**Import errors**
- Serve from `www/` directory
- Use correct import path: `./pkg/paykit_demo_web.js`

**WebSocket connection fails**
- Check WebSocket server is running
- Verify server URL and port
- Check CORS configuration

See the [Troubleshooting](#-troubleshooting) section above for common issues.

## 📝 Development

### Adding New Features

1. Add Rust code in `src/`
2. Expose via `#[wasm_bindgen]` annotations
3. Update `src/lib.rs` exports
4. Add tests in `tests/` or `#[cfg(test)]`
5. Update JavaScript in `www/app.js`
6. Add UI in `www/index.html`
7. Style in `www/styles.css`
8. Document in feature-specific `.md` file
9. Update [API_REFERENCE.md](./API_REFERENCE.md)

### Code Style

- Follow Rust 2021 edition conventions
- Use `cargo fmt` before committing
- Run `cargo clippy` to catch issues
- Write tests for new functionality
- Document public APIs with `///` comments
- Follow existing patterns in codebase

### Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md) for:
- System architecture
- Component design
- Data flow diagrams
- Storage patterns
- Security considerations

## 🤝 Contributing

See the main [Paykit repository](https://github.com/paykit/paykit-rs) for contribution guidelines.

## 📄 License

This project is part of the Paykit ecosystem. See main repository for license information.

## Related Components

This web demo application uses and integrates with:

- **[paykit-lib](../paykit-lib/README.md)** - Core library for directory operations and transport traits
- **[paykit-subscriptions](../paykit-subscriptions/README.md)** - Subscription management, payment requests, and auto-pay automation
- **[paykit-demo-core](../paykit-demo-core/README.md)** - Shared business logic for demo applications

## 🔗 Links

- [Paykit Demo CLI](../paykit-demo-cli/README.md) - Native command-line demo
- [Paykit Library](../paykit-lib/README.md) - Core protocol library
- [Pubky Protocol](https://pubky.org) - Identity and directory protocol
- [Noise Protocol](http://noiseprotocol.org/) - Encryption protocol

## 📊 Status

**Production-Ready for Demonstration:**
- ✅ Dashboard and overview
- ✅ Identity management
- ✅ Contact management
- ✅ Payment method configuration
- ✅ Receipt management and filtering
- ✅ Directory queries
- ✅ Subscription management
- ✅ Auto-pay automation
- ✅ WebSocket Noise transport
- ✅ Receipt exchange
- ✅ Comprehensive testing (~103 tests)
- ✅ Complete documentation

**Limitations:**
- Demo security model (not for real funds)
- Browser localStorage limits (~5-10MB)
- Mock publishing (methods not actually published to homeserver)
- Requires WebSocket relay server for receiving payments
- No encryption at rest (demo purposes only)

## 🛣️ Roadmap & Future Enhancements

Based on comprehensive code review, the following enhancements are recommended:

### High Priority

#### Enhanced E2E Payment Testing
- **Status**: ⚠️ Partial - Limited automated E2E tests
- **Action**: Add more comprehensive E2E test scenarios
- **Impact**: Improved confidence in payment flows
- **Details**: 
  - Create test fixtures for complete payment flows
  - Add automated tests for WebSocket payment flows
  - Test complete payment lifecycle end-to-end

#### Payment Flow Documentation
- **Status**: ✅ Good - PAYMENTS.md exists
- **Action**: Enhance payment flow documentation
- **Impact**: Better understanding of payment limitations
- **Details**: 
  - Document WebSocket relay server requirements more clearly
  - Add troubleshooting guide for payment failures
  - Document simulation vs real payment flows

### Medium Priority

#### Error Type Refinement
- **Status**: ✅ Good - Currently uses `Result<(), JsValue>`
- **Action**: Add specific error types for different failure modes
- **Impact**: Better error handling and debugging
- **Details**: 
  - Create custom error types for payment failures
  - Better error categorization
  - More detailed error messages in UI

#### Performance Testing
- **Status**: ❌ Not implemented
- **Action**: Add performance tests and benchmarks
- **Impact**: Identify performance bottlenecks
- **Details**:
  - Benchmark localStorage operations
  - Test with large datasets (many contacts/receipts)
  - Profile WASM module performance
  - Test memory usage with many items

#### Real Pubky Homeserver Publishing
- **Status**: ⚠️ Mock publishing only
- **Action**: Implement real homeserver publishing
- **Impact**: Full protocol demonstration
- **Details**: Currently methods are saved locally only, not published to homeserver

### Low Priority

#### Additional Features
- **WebRTC data channels** for true P2P (no relay server needed)
- **Service Worker** for offline support
- **IndexedDB** for larger storage (beyond localStorage limits)
- **Push notifications** for receipts
- **QR code scanner** for URIs
- **Mobile app wrappers** (Capacitor/Tauri)
- **Desktop app** (Tauri)
- **Client-side encryption** for sensitive data
- **Cross-device synchronization**

#### Test Documentation Enhancement
- **Status**: ✅ Excellent - TESTING.md is comprehensive (~800 lines)
- **Action**: Minor enhancements
- **Impact**: Easier test maintenance
- **Details**:
  - Add more test scenario examples
  - Document test data requirements
  - Add troubleshooting guide for common test failures

### Known Limitations

The following are documented limitations appropriate for demo applications:

- ⚠️ Private keys stored in plaintext localStorage (not for production)
- ⚠️ No encryption at rest
- ⚠️ Browser localStorage limits (~5-10MB)
- ⚠️ Mock publishing (methods not actually published to homeserver)
- ⚠️ Requires WebSocket relay server for receiving payments
- ⚠️ No offline support

**For production use**, implement:
- Secure key storage (OS keychain or hardware security modules)
- Encryption at rest
- Proper session management
- Rate limiting and DoS protection
- Security audit required

---

**Built with Rust + WebAssembly | Powered by Pubky + Noise Protocol**
