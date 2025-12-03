# Production Deployment Guide

This guide covers deploying Paykit-rs with pubky-noise v0.8.0 for production use.

## Overview

Paykit provides encrypted payment communication using the Pubky ecosystem:
- **pubky-noise**: Noise protocol for encrypted channels
- **pkarr**: Decentralized key/metadata resolution
- **paykit-lib**: Payment directory and transport abstraction

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      APPLICATION LAYER                          │
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │  Bitkit     │  │  CLI Demo   │  │  Web Demo               │ │
│  │  (Mobile)   │  │  (Desktop)  │  │  (Browser)              │ │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘ │
│         │                │                      │               │
│         └────────────────┼──────────────────────┘               │
│                          │                                      │
│  ┌───────────────────────▼────────────────────────────────────┐ │
│  │                 paykit-demo-core                           │ │
│  │  - Identity management                                     │ │
│  │  - Session management                                      │ │
│  │  - Payment coordination                                    │ │
│  └───────────────────────┬────────────────────────────────────┘ │
│                          │                                      │
└──────────────────────────┼──────────────────────────────────────┘
                           │
┌──────────────────────────┼──────────────────────────────────────┐
│                          │      PROTOCOL LAYER                  │
│                          │                                      │
│  ┌───────────────────────▼───────────────────────────────────┐  │
│  │                   pubky-noise                             │  │
│  │  - Noise protocol encryption                              │  │
│  │  - Pattern selection (IK, IK-raw, N, NN, XX)             │  │
│  │  - Session management                                     │  │
│  │  - pkarr key helpers                                      │  │
│  └───────────────────────┬───────────────────────────────────┘  │
│                          │                                      │
│  ┌───────────────────────▼───────────────────────────────────┐  │
│  │                   paykit-lib                              │  │
│  │  - Payment directory                                      │  │
│  │  - Transport abstraction                                  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Key Management

### Cold Key Architecture

For production wallets like Bitkit:

1. **Ed25519 Root Identity**
   - Derived from BIP-39 seed phrase
   - Used for pkarr identity
   - Kept cold (offline after setup)

2. **X25519 Session Keys**
   - Derived from Ed25519 using HKDF-SHA512
   - Unique per device (using device ID as context)
   - Stored in secure enclave (Keychain/Keystore)

3. **pkarr Publication**
   - X25519 public key published to pkarr
   - Signed with Ed25519 for binding
   - One-time cold signing operation

### Key Derivation

```rust
use pubky_noise::kdf::derive_x25519_static;
use pubky_noise::pkarr_helpers::{sign_pkarr_key_binding, format_x25519_for_pkarr};

// Derive X25519 from Ed25519 (once, during setup)
let ed25519_sk: [u8; 32] = /* from seed phrase */;
let device_id = "bitkit-mobile-1";
let x25519_sk = derive_x25519_static(&ed25519_sk, device_id.as_bytes());

// Compute public key
use x25519_dalek::{PublicKey, StaticSecret};
let secret = StaticSecret::from(x25519_sk);
let x25519_pk = PublicKey::from(&secret);

// Sign binding for pkarr
let signature = sign_pkarr_key_binding(&ed25519_sk, x25519_pk.as_bytes(), device_id);
let txt_record = format_x25519_for_pkarr(x25519_pk.as_bytes(), Some(&signature));

// Publish to pkarr at _noise.{device_id}
```

### Secure Storage

**iOS:**
```swift
let query: [String: Any] = [
    kSecClass: kSecClassGenericPassword,
    kSecAttrAccount: "x25519-sk",
    kSecValueData: secretKey,
    kSecAttrAccessible: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
]
SecItemAdd(query as CFDictionary, nil)
```

**Android:**
```kotlin
val keyStore = KeyStore.getInstance("AndroidKeyStore")
keyStore.load(null)
val secretKey = SecretKeySpec(x25519Sk, "RAW")
val entry = KeyStore.SecretKeyEntry(secretKey)
keyStore.setEntry("x25519-sk", entry, KeyProtection.Builder(PURPOSE_ENCRYPT or PURPOSE_DECRYPT).build())
```

## Pattern Selection

| Scenario | Pattern | Why |
|----------|---------|-----|
| Payment to known recipient | IK-raw | Authenticated via pkarr |
| Anonymous donation | N | Sender privacy |
| Ephemeral negotiation | NN | Maximum privacy |
| First contact | XX | No prior key exchange |

### Pattern Selection Flowchart

```
┌─────────────────────────────────────────────┐
│  Do you have recipient's X25519 public key? │
└────────────────────┬────────────────────────┘
                     │
         ┌───────────┴───────────┐
         │                       │
         ▼                       ▼
       [Yes]                   [No]
         │                       │
         ▼                       ▼
┌────────────────────┐   ┌────────────────────┐
│ Need sender auth?  │   │   Use XX pattern   │
└─────────┬──────────┘   └────────────────────┘
          │
    ┌─────┴─────┐
    │           │
    ▼           ▼
  [Yes]       [No]
    │           │
    ▼           ▼
┌──────────┐ ┌──────────┐
│ IK-raw   │ │ N        │
└──────────┘ └──────────┘
```

## Error Handling

### Connection Errors

```rust
match manager.initiate_connection_with_pattern(pattern, Some(&sk), Some(&pk)) {
    Ok((session_id, msg)) => {
        // Proceed with handshake
    }
    Err(NoiseError::InvalidKey) => {
        // Invalid key format - check key length
    }
    Err(NoiseError::Handshake(_)) => {
        // Handshake failed - retry or fallback
    }
    Err(NoiseError::Other(msg)) => {
        // Log and report
        log::error!("Noise error: {}", msg);
    }
}
```

### Retry Strategy

```rust
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

for attempt in 0..MAX_RETRIES {
    match connect() {
        Ok(session) => return Ok(session),
        Err(e) if is_transient(&e) => {
            tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * 2u64.pow(attempt))).await;
        }
        Err(e) => return Err(e),
    }
}
```

## Monitoring

### Metrics to Track

- Handshake success rate by pattern
- Encryption/decryption latency
- Session lifetime
- Key derivation time
- pkarr lookup latency

### Logging

```rust
// Enable tracing
use tracing_subscriber::EnvFilter;

tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .init();

// Set log level
// RUST_LOG=pubky_noise=debug,paykit=info
```

## Testing

### Unit Tests (No Network)

Use mock transports:

```rust
use paykit_demo_core::testing::{MockStorage, MockAuthenticatedTransport};

let storage = MockStorage::new();
let transport = MockAuthenticatedTransport::new(storage.clone(), "test-owner");
// ... run tests
```

### Integration Tests

```bash
# Run all tests
cargo test --workspace

# Skip network-dependent tests
cargo test --workspace -- --skip network --skip testnet
```

### E2E Tests

```bash
# With testnet (requires network)
cargo test --workspace --features pubky-testnet

# The test may fail if testnet is unavailable
```

## Deployment Checklist

### Pre-Deployment

- [ ] All tests pass with mocks
- [ ] Key derivation tested with known vectors
- [ ] Pattern selection logic reviewed
- [ ] Error handling covers all cases
- [ ] Logging configured appropriately
- [ ] Metrics endpoints ready

### Security Review

- [ ] Secret keys properly zeroized
- [ ] No key material in logs
- [ ] Secure storage configured
- [ ] TLS for underlying transport
- [ ] pkarr signature verification enabled

### Mobile Specific

- [ ] iOS build tested on device
- [ ] Android build tested on device
- [ ] Background session handling tested
- [ ] Low memory conditions tested
- [ ] Battery impact measured

## Version Compatibility

| pubky-noise | paykit-rs | Snow | Breaking Changes |
|-------------|-----------|------|------------------|
| 0.8.0 | 0.3.0+ | 0.10 | epoch removed, new patterns |
| 0.7.0 | 0.2.0 | 0.9 | Initial release |

## Support

- GitHub Issues: https://github.com/synonymdev/paykit-rs
- Documentation: See `/docs` directory
- Examples: See `paykit-demo-cli/demos/`

