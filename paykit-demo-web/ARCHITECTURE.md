# Paykit Demo Web - Architecture

## Overview

Paykit Demo Web is a browser-based demonstration of the Paykit payment protocol, built with Rust and WebAssembly. This document describes the architecture, design decisions, and technical implementation details.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Browser (JavaScript)                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │   index.html │  │    app.js    │  │  styles.css  │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────────┘  │
└─────────┼──────────────────┼────────────────────────────┘
          │                  │
          │                  │ wasm-bindgen
          │                  │
┌─────────▼──────────────────▼────────────────────────────┐
│              WebAssembly Module (Rust)                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  Identity    │  │   Contacts   │  │   Methods    │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │   Receipts   │  │  Dashboard   │  │ Subscriptions│  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  Directory   │  │   Storage    │  │  WebSocket   │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└─────────┬──────────────────┬────────────────────────────┘
          │                  │
          │                  │
┌─────────▼──────────────────▼────────────────────────────┐
│              Browser APIs                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ localStorage │  │  WebSocket   │  │   Fetch API  │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Identity Management (`src/identity.rs`)

**Purpose**: Ed25519 keypair generation and management

**Key Features**:
- Random keypair generation
- Nickname support
- JSON serialization/deserialization
- Pubky URI generation

**Storage**: BrowserStorage (localStorage)

**Dependencies**: `paykit_lib::PublicKey`, `ed25519_dalek`

### 2. Contact Management (`src/contacts.rs`)

**Purpose**: Address book for Pubky peers

**Key Features**:
- CRUD operations
- Search functionality
- Payment history tracking
- Notes and metadata

**Storage**: WasmContactStorage (localStorage)

**Schema**: `paykit_contact:{pubkey}` → JSON

### 3. Payment Methods (`src/payment_methods.rs`)

**Purpose**: Configure payment endpoints

**Key Features**:
- Multiple method types (Lightning, Onchain, Custom)
- Priority ordering
- Preferred method selection
- Public/private visibility
- Mock publishing (demo limitation)

**Storage**: WasmPaymentMethodStorage (localStorage)

**Schema**: `paykit_payment_method:{method_id}` → JSON

### 4. Receipt Management (`src/payment.rs`)

**Purpose**: Transaction history and filtering

**Key Features**:
- Receipt storage and retrieval
- Filter by direction (sent/received)
- Filter by method
- Filter by contact
- Statistics calculation
- JSON export

**Storage**: WasmReceiptStorage (localStorage)

**Schema**: `paykit_receipts:{receipt_id}` → JSON

### 5. Dashboard (`src/dashboard.rs`)

**Purpose**: Unified overview and statistics

**Key Features**:
- Statistics aggregation from all features
- Setup progress tracking
- Recent activity feed
- Setup completion validation

**Dependencies**: All storage modules

**No Storage**: Aggregates data from other modules

### 6. Directory Client (`src/directory.rs`)

**Purpose**: Query payment methods from Pubky homeservers

**Key Features**:
- HTTP-based method discovery
- Pubky homeserver integration
- Unauthenticated read operations

**Protocol**: HTTP GET to `/pub/paykit.app/v0/{method_id}`

**Dependencies**: `paykit_lib::UnauthenticatedTransportRead`

### 7. Subscriptions (`src/subscriptions.rs`)

**Purpose**: Recurring payment agreements

**Key Features**:
- Subscription creation and management
- Signed subscription storage
- Active subscription tracking
- Frequency parsing (daily, weekly, monthly, yearly, custom)

**Storage**: WasmSubscriptionAgreementStorage (localStorage)

### 8. WebSocket Transport (`src/websocket_transport.rs`)

**Purpose**: Noise protocol over WebSocket

**Key Features**:
- WebSocket wrapper for Noise IK handshake
- Encrypted message send/receive
- Length-prefixed messages
- Event-driven architecture

**Protocol**: Noise_IK over WebSocket binary frames

**Dependencies**: `pubky_noise::NoiseLink`, `web_sys::WebSocket`

### 9. Browser Storage (`src/storage.rs`)

**Purpose**: localStorage abstraction

**Key Features**:
- Identity persistence
- Current identity tracking
- Clear all functionality

**Schema**: `paykit_identity:{name}` → JSON

## Data Flow

### Identity Creation Flow

```
User Input (nickname)
    ↓
Identity.withNickname()
    ↓
Ed25519 Keypair Generation
    ↓
BrowserStorage.saveIdentity()
    ↓
localStorage: paykit_identity:{name}
```

### Payment Method Configuration Flow

```
User Input (method type, endpoint, settings)
    ↓
WasmPaymentMethodConfig::new()
    ↓
WasmPaymentMethodStorage.save_method()
    ↓
localStorage: paykit_payment_method:{method_id}
    ↓
(Optional) mock_publish() → localStorage marker
```

### Receipt Storage Flow

```
Payment Coordination
    ↓
Receipt Generation
    ↓
WasmReceiptStorage.save_receipt()
    ↓
localStorage: paykit_receipts:{receipt_id}
    ↓
(Optional) Contact.update_payment_history()
```

### Dashboard Statistics Flow

```
WasmDashboard.get_overview_stats()
    ↓
    ├─→ WasmContactStorage.list_contacts()
    ├─→ WasmPaymentMethodStorage.list_methods()
    ├─→ WasmReceiptStorage.get_statistics()
    └─→ WasmSubscriptionAgreementStorage.list_all_subscriptions()
    ↓
Aggregate Results
    ↓
Return JavaScript Object
```

## Storage Architecture

### localStorage Schema

```
paykit_identity:{name}              → Identity JSON
paykit_current_identity              → Current identity name
paykit_contact:{pubkey}              → Contact JSON
paykit_payment_method:{method_id}    → Method JSON
paykit_receipts:{receipt_id}         → Receipt JSON
paykit_subscription:{id}             → Subscription JSON
paykit_signed_subscription:{id}      → Signed Subscription JSON
paykit_request:{id}                  → Request JSON
paykit_mock_publish_status           → Mock publish timestamp
```

### Storage Limits

- **Browser Limit**: 5-10MB (browser-dependent)
- **Per Item**: ~200-400 bytes average
- **Capacity**: ~17,000-34,000 items total
- **Quota Detection**: Errors handled gracefully

### Storage Patterns

**Key Naming**: `{prefix}:{identifier}`

**Benefits**:
- Easy iteration (prefix search)
- Namespace isolation
- Clear organization

**Trade-offs**:
- Manual iteration required (no native indexing)
- O(n) list operations

## WebAssembly Architecture

### Module Structure

```
paykit-demo-web (WASM crate)
├── src/
│   ├── lib.rs              # Module exports
│   ├── identity.rs         # Identity WASM bindings
│   ├── contacts.rs         # Contact WASM bindings
│   ├── payment_methods.rs  # Method WASM bindings
│   ├── payment.rs          # Receipt + Payment WASM bindings
│   ├── dashboard.rs        # Dashboard WASM bindings
│   ├── directory.rs        # Directory WASM bindings
│   ├── subscriptions.rs    # Subscription WASM bindings
│   ├── storage.rs          # Storage WASM bindings
│   ├── websocket_transport.rs # WebSocket WASM bindings
│   ├── types.rs            # Core types (WASM-compatible)
│   └── utils.rs            # Utilities
└── tests/                  # Integration tests
```

### WASM Bindings Pattern

```rust
#[wasm_bindgen]
pub struct WasmFeature {
    inner: Feature,
}

#[wasm_bindgen]
impl WasmFeature {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { /* ... */ }
    
    #[wasm_bindgen]
    pub async fn do_something(&self) -> Result<(), JsValue> { /* ... */ }
}
```

### JavaScript Interop

**Rust → JavaScript**:
- `JsValue` for complex types
- `String` for text
- `bool`, `u32`, `i64` for primitives
- `Vec<JsValue>` for arrays

**JavaScript → Rust**:
- `String` from JS strings
- `Option<T>` from JS null/undefined
- `Result<T, JsValue>` for error handling

## Browser Compatibility

### Required APIs

- **WebAssembly**: Core WASM support
- **localStorage**: Persistent storage
- **WebSocket**: Network communication (for payments)
- **Fetch API**: HTTP requests (for directory)
- **JSON**: Native JSON support

### Browser Support

| Browser | Minimum Version | Status |
|---------|----------------|--------|
| Chrome | 90+ | ✅ Full support |
| Firefox | 88+ | ✅ Full support |
| Safari | 14+ | ✅ Full support |
| Edge | 90+ | ✅ Full support |
| Opera | 76+ | ✅ Full support |

### Mobile Support

- **iOS Safari**: 14+ ✅
- **Chrome Android**: 90+ ✅
- **Samsung Internet**: 15+ ✅

## Security Architecture

### Current Implementation (Demo)

**⚠️ NOT Production-Ready**:

1. **No Encryption**: All data stored in plaintext
2. **No Authentication**: No capability tokens
3. **No Access Control**: localStorage accessible to any script
4. **No Server Validation**: Client-side only
5. **Mock Publishing**: No real homeserver integration

### Production Requirements

1. **Encryption at Rest**: Encrypt sensitive data before storage
2. **Capability Tokens**: Implement Pubky authentication
3. **CSP Headers**: Content Security Policy
4. **Input Validation**: Server-side validation
5. **Rate Limiting**: Protect against abuse
6. **HTTPS Only**: Enforce secure connections

## Performance Considerations

### WASM Module Size

- **Uncompressed**: ~2-3MB
- **Compressed (gzip)**: ~200-300KB
- **Optimized**: `wasm-opt` reduces by ~30%

### Load Times

- **Initial Load**: <2 seconds
- **Module Init**: <100ms
- **Tab Switch**: <50ms
- **Storage Operations**: <10ms (localStorage)

### Optimization Strategies

1. **Code Splitting**: Not applicable (single WASM module)
2. **Lazy Loading**: Load features on demand
3. **Caching**: Browser caches WASM module
4. **Compression**: gzip/brotli on server

## Error Handling

### Error Propagation

```
Rust Error
    ↓
Result<T, E>
    ↓
JsValue (via wasm-bindgen)
    ↓
JavaScript Error/Promise Rejection
```

### Error Types

- **Validation Errors**: Invalid input (pubkey, etc.)
- **Storage Errors**: localStorage unavailable/quota exceeded
- **Network Errors**: Fetch/WebSocket failures
- **Serialization Errors**: JSON parse failures

### Error Handling Pattern

```rust
pub async fn operation() -> Result<(), JsValue> {
    let result = inner_operation()
        .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
    Ok(())
}
```

## Testing Architecture

### Test Organization

```
tests/
├── contact_lifecycle.rs          # Contact tests
├── payment_method_management.rs   # Method tests
├── receipt_management.rs         # Receipt tests
├── dashboard.rs                  # Dashboard tests
├── edge_cases.rs                 # Edge case tests
├── cross_feature_integration.rs  # Integration tests
├── payment_flow.rs               # Payment tests
├── subscription_lifecycle.rs      # Subscription tests
└── storage_persistence.rs        # Storage tests
```

### Test Execution

- **Environment**: Browser (Chrome/Firefox)
- **Framework**: `wasm-bindgen-test`
- **Async**: All storage tests are async
- **Isolation**: Tests clean up after themselves

## Deployment Architecture

### Build Process

```
Rust Source Code
    ↓
cargo build --target wasm32-unknown-unknown
    ↓
wasm-pack build --target web
    ↓
WASM + JavaScript Bindings
    ↓
www/pkg/
    ├── paykit_demo_web_bg.wasm
    ├── paykit_demo_web.js
    └── paykit_demo_web.d.ts
```

### Deployment Targets

1. **Static Hosting**: Netlify, Vercel, GitHub Pages
2. **CDN**: Cloudflare Pages, AWS S3 + CloudFront
3. **Self-Hosted**: Any static file server

### Requirements

- **MIME Types**: `application/wasm` for `.wasm` files
- **HTTPS**: Required for WebSocket secure connections
- **CORS**: If API server is separate domain

## Limitations and Trade-offs

### Browser Limitations

1. **localStorage Size**: 5-10MB limit
2. **No Background Processing**: Single-threaded execution
3. **No File System**: Can't write files
4. **No Direct TCP**: Must use WebSocket for network
5. **No Native Threading**: Async only

### Design Trade-offs

1. **localStorage vs IndexedDB**: Chose localStorage for simplicity
2. **WASM vs Pure JS**: Chose WASM for Rust code reuse
3. **Single Module vs Code Splitting**: Single module for simplicity
4. **Mock Publishing**: Demo limitation, not production-ready

## Future Architecture Enhancements

### Potential Improvements

1. **IndexedDB**: Larger storage capacity
2. **Service Workers**: Offline support
3. **WebRTC**: True P2P connections
4. **Web Crypto API**: Client-side encryption
5. **WebAssembly Threads**: Parallel processing
6. **Code Splitting**: Lazy load features
7. **Progressive Web App**: Installable app

## Dependencies

### Rust Dependencies

```toml
[dependencies]
wasm-bindgen = "0.2"      # WASM bindings
web-sys = "0.3"           # Browser APIs
js-sys = "0.3"            # JavaScript interop
serde = { version = "1", features = ["derive"] }
serde_json = "1"
paykit-lib = { path = "../paykit-lib" }
pubky-noise = { path = "../../pubky-noise" }
```

### JavaScript Dependencies

- **None**: Pure ES6 modules, no npm dependencies
- **Browser APIs**: Native Web APIs only

## Module Communication

### Internal Communication

All modules communicate through:
- **Shared Storage**: localStorage (implicit)
- **JavaScript State**: Global variables in `app.js`
- **Event-Driven**: Tab switches trigger updates

### External Communication

- **Pubky Homeservers**: HTTP GET (directory queries)
- **WebSocket Servers**: WebSocket (payment coordination)
- **No Direct P2P**: Requires relay server

## Data Persistence

### Persistence Strategy

- **localStorage**: Primary storage
- **No Server Sync**: Local-only
- **No Backup**: User responsibility
- **No Encryption**: Plaintext storage

### Data Lifecycle

1. **Create**: User action → WASM → localStorage
2. **Read**: localStorage → WASM → JavaScript
3. **Update**: User action → WASM → localStorage (overwrite)
4. **Delete**: User action → WASM → localStorage.removeItem()

## Concurrency Model

### Single-Threaded Execution

- **Main Thread**: All WASM execution
- **Async Operations**: Futures-based (browser event loop)
- **No Threading**: WebAssembly threads not used
- **Non-Blocking**: Async/await prevents blocking

### Async Patterns

```rust
pub async fn operation() -> Result<(), JsValue> {
    // Async localStorage access
    let storage = get_local_storage()?;
    // ... operations
    Ok(())
}
```

## Memory Management

### WASM Memory

- **Linear Memory**: Managed by wasm-bindgen
- **Garbage Collection**: JavaScript GC handles JS objects
- **No Manual Management**: Rust ownership + JS GC

### Memory Limits

- **WASM Module**: ~2-3MB
- **Runtime Memory**: Browser-dependent
- **localStorage**: 5-10MB limit

## Build Configuration

### Cargo.toml

```toml
[lib]
crate-type = ["cdylib"]

[dependencies.wasm-bindgen]
version = "0.2"
features = ["serde-serialize"]
```

### wasm-pack Configuration

```bash
wasm-pack build --target web --out-dir www/pkg
```

**Targets**:
- `web`: ES6 modules for browsers
- `bundler`: For webpack/rollup
- `nodejs`: For Node.js

## See Also

- [API Reference](./API_REFERENCE.md) - Complete API documentation
- [Testing Guide](./TESTING.md) - Testing architecture
- [Deployment Guide](./DEPLOYMENT.md) - Deployment details
- [README.md](./README.md) - Project overview

---

**Last Updated**: November 2024  
**Architecture Version**: 1.0

