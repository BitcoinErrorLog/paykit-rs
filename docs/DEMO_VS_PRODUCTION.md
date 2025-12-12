# Demo vs Production Code Boundaries

This document clearly delineates which components are production-ready and which are demonstration/development code.

## Quick Reference

| Crate | Status | Purpose |
|-------|--------|---------|
| `paykit-lib` | ✅ Production-Ready | Core protocol library |
| `paykit-interactive` | ✅ Production-Ready | Interactive payment protocol |
| `paykit-subscriptions` | ✅ Production-Ready | Subscription management |
| `paykit-mobile` | ✅ Production-Ready | Mobile FFI bindings |
| `paykit-demo-cli` | ⚠️ Demo Only | CLI demonstration app |
| `paykit-demo-core` | ⚠️ Demo Only | Shared demo logic |
| `paykit-demo-web` | ⚠️ Demo Only | Web demonstration app |

## Production-Ready Components

### paykit-lib

The core library implementing the Paykit protocol.

**Production Features:**
- Transport trait abstractions (`AuthenticatedTransport`, `UnauthenticatedTransportRead`)
- Payment endpoint serialization/deserialization
- Pubky SDK integration (feature-gated)
- Secure encryption for private endpoints (`file-storage` feature)
- Platform-specific secure storage adapters (Keychain, Keystore, Credential Manager)

**Usage:**
```rust
use paykit_lib::{
    AuthenticatedTransport,
    UnauthenticatedTransportRead,
    PubkyAuthenticatedTransport,
    PubkyUnauthenticatedTransport,
};
```

### paykit-interactive

Interactive payment protocol with Noise encryption.

**Production Features:**
- Full Noise_IK handshake implementation
- End-to-end encrypted payment channels
- Receipt generation and verification
- Rate limiting for handshakes (`HandshakeRateLimiter`)
- Storage abstraction (`PaykitStorage` trait)

**Important:** The storage trait must be implemented with production-grade persistence.

```rust
use paykit_interactive::{
    PaykitInteractiveManager,
    PaykitNoiseChannel,
    PaykitStorage,
    rate_limit::RateLimitConfig,
};
```

### paykit-subscriptions

Subscription and payment request management.

**Production Features:**
- Ed25519 signature generation and verification
- Nonce-based replay protection
- Safe financial arithmetic with `rust_decimal`
- Invoice generation
- Subscription state machine

**Critical Security:** Always use `NonceStore` for replay protection in production.

```rust
use paykit_subscriptions::{
    Subscription,
    PaymentRequest,
    NonceStore,
    sign_subscription,
    verify_signature,
};
```

### paykit-mobile

FFI bindings for iOS and Android.

**Production Features:**
- UniFFI-generated Swift/Kotlin bindings
- Async runtime bridge for mobile
- Platform-specific threading considerations

**FFI Considerations:**
- Never call `block_on()` from an existing Tokio runtime
- Use the provided `AsyncBridge` for async operations
- See `CONCURRENCY.md` for lock handling

## Demo-Only Components

### paykit-demo-cli

⚠️ **NOT FOR PRODUCTION USE**

A command-line demonstration application.

**Demo Limitations:**

| Aspect | Demo Behavior | Production Requirement |
|--------|---------------|------------------------|
| Key Storage | Plaintext JSON files | OS Keychain/Keystore |
| Encryption at Rest | None | AES-256-GCM minimum |
| Session Management | File-based | Secure session tokens |
| Error Handling | Basic `anyhow::Result` | Detailed error types |
| Rate Limiting | None | Required |
| Logging | Minimal | Structured logging |

**Specific Files:**
- `src/commands/*.rs` - Demo CLI commands
- `src/ui/mod.rs` - Terminal UI helpers
- `tests/common/mod.rs` - Test utilities (including `IdentityManager` mock)

### paykit-demo-core

⚠️ **NOT FOR PRODUCTION USE**

Shared logic for demo applications.

**Demo Limitations:**
- `IdentityManager` stores keys insecurely
- `NoiseClientHelper`/`NoiseServerHelper` are convenience wrappers
- Storage implementations are file-based

**Can Be Used As Reference For:**
- Payment flow structure
- Noise channel setup patterns
- Subscription lifecycle

### paykit-demo-web

⚠️ **NOT FOR PRODUCTION USE**

Web-based demonstration application.

**Demo Limitations:**
- Session handling is simplified
- No CSRF protection
- Development-only TLS configuration

## Migration Guide: Demo to Production

### Key Storage

**Demo:**
```rust
// paykit-demo-core/src/identity.rs
pub struct IdentityManager {
    identities_dir: PathBuf,  // Plaintext JSON files
}
```

**Production:**
```rust
use paykit_lib::secure_storage::{KeychainAdapter, KeystoreAdapter};

#[cfg(target_os = "macos")]
type SecureStorage = KeychainAdapter;

#[cfg(target_os = "android")]
type SecureStorage = KeystoreAdapter;
```

### Storage Implementation

**Demo:**
```rust
// File-based storage
pub struct FileStorage {
    path: PathBuf,
}
```

**Production:**
```rust
// Implement PaykitStorage trait with your database
impl PaykitStorage for PostgresStorage {
    async fn store_receipt(&self, receipt: &PaykitReceipt) -> Result<()> {
        self.db.execute(
            "INSERT INTO receipts (id, data) VALUES ($1, $2)",
            &[&receipt.id, &serde_json::to_value(receipt)?]
        ).await?;
        Ok(())
    }
    
    async fn get_receipt(&self, id: &str) -> Result<Option<PaykitReceipt>> {
        // ...
    }
}
```

### Error Handling

**Demo:**
```rust
use anyhow::Result;

fn process_payment() -> Result<()> {
    // Simple error propagation
    let result = pay()?;
    Ok(())
}
```

**Production:**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PaymentError {
    #[error("Invoice expired at {expired_at}")]
    InvoiceExpired { invoice_id: String, expired_at: i64 },
    
    #[error("Insufficient funds: needed {needed}, available {available}")]
    InsufficientFunds { needed: u64, available: u64 },
    
    #[error("Rate limit exceeded for {ip}")]
    RateLimited { ip: String },
    
    #[error("Signature verification failed")]
    SignatureInvalid,
}

fn process_payment() -> Result<Receipt, PaymentError> {
    // Specific error types for different failure modes
}
```

### Rate Limiting

**Demo:** None

**Production:**
```rust
use paykit_interactive::rate_limit::{HandshakeRateLimiter, RateLimitConfig};

// Configure for production
let config = RateLimitConfig::strict_with_global();
let limiter = HandshakeRateLimiter::new_shared(config);

// Apply before handshake
if !limiter.check_and_record(peer_ip) {
    return Err(Error::RateLimited);
}
```

### Nonce Management

**Demo:** May not persist nonces across restarts

**Production:**
```rust
use paykit_subscriptions::NonceStore;

// Persist nonces in database
impl NonceStore for PostgresNonceStore {
    fn check_and_mark(&self, nonce: &[u8; 32], expires_at: i64) -> Result<bool> {
        // Database transaction to ensure atomicity
        let tx = self.db.begin()?;
        
        // Check if nonce exists
        if self.exists(&tx, nonce)? {
            return Ok(false);
        }
        
        // Insert nonce
        self.insert(&tx, nonce, expires_at)?;
        tx.commit()?;
        
        Ok(true)
    }
}

// Set up automated cleanup (see NONCE_CLEANUP_GUIDE.md)
```

## Feature Flags

Production code is gated behind features:

```toml
[features]
default = ["pubky"]

# Production features
pubky = ["dep:pubky"]                    # Pubky SDK integration
file-storage = ["dep:aes-gcm", ...]      # Secure file encryption

# Test/development features
test-utils = []                          # Mock implementations
integration-tests = ["pubky"]            # Network tests
pubky_compliance_tests = ["pubky"]       # SDK compliance tests
```

## Disabled Tests

Some tests are intentionally disabled for specific reasons:

### pubky_compliance_tests

**Location:** `paykit-lib/tests/pubky_sdk_compliance.rs`, `paykit-demo-cli/tests/pubky_compliance.rs`

**Status:** Disabled via feature flag

**Reason:** Pubky SDK 0.6.x introduced breaking API changes:
- `PubkyClient` no longer exists
- `generate_keypair()` moved
- `PubkyTestnet` signature changed

**Re-enable When:** Pubky SDK API migration is complete

### integration-tests

**Location:** Various `tests/` directories

**Status:** Disabled by default

**Reason:** Require network access and running homeserver

**Enable With:** `cargo test --features integration-tests`

## Audit Checklist for Production Deployment

Before deploying production code:

- [ ] All demo components (`paykit-demo-*`) are **excluded** from production build
- [ ] Secure key storage is implemented (not file-based)
- [ ] Nonce store is persistent and cleaned up automatically
- [ ] Rate limiting is configured with appropriate limits
- [ ] Error types are specific (not generic `anyhow`)
- [ ] Logging is structured and appropriate for production
- [ ] All `unwrap()`/`expect()` calls reviewed for panic safety
- [ ] Lock poisoning policy followed (see `CONCURRENCY.md`)
- [ ] Financial calculations use `Decimal`, not `f64`
- [ ] Signatures verified before processing payments
- [ ] Nonces checked for replay protection

