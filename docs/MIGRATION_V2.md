# Migration Guide: v1.x to v2.0

This guide covers the breaking changes and migration steps for upgrading from paykit-lib v1.x to v2.0.

## Overview

The v2.0 release includes critical security fixes and correctness improvements. All downstream consumers (bitkit-android, bitkit-ios, pubky-ring) should upgrade as soon as possible.

## Breaking Changes Summary

| Component | Change | Impact |
|-----------|--------|--------|
| `paykit-lib` | TLS certificate validation behavior | May reject previously-accepted invalid certs |
| `paykit-interactive` | Message size limits enforced | Large messages now rejected |
| `paykit-subscriptions` | `NonceStorage` trait required for persistent nonces | Implement trait for your storage backend |
| `paykit-mobile` | Version bump | Regenerate FFI bindings |

---

## paykit-lib Changes

### TLS Certificate Validation

**Old Behavior (v1.x - INSECURE)**:
```rust
// When tls_cert_pem was provided, ALL certificate validation was bypassed
let config = LndConfig::new("https://lnd.example.com", "macaroon")
    .with_tls_cert(cert_pem); // This DISABLED validation entirely!
```

**New Behavior (v2.0 - SECURE)**:
```rust
// Certificate is added as a trusted root, validation is ENFORCED
let config = LndConfig::new("https://lnd.example.com", "macaroon")
    .with_tls_cert(cert_pem); // Certificate properly validated
```

**Migration**:
1. Ensure your TLS certificate PEM is valid and matches your LND server
2. The certificate must be in PEM format (begins with `-----BEGIN CERTIFICATE-----`)
3. If you were relying on bypassed validation, fix your certificate setup

**Testing**:
```rust
// Invalid PEM now returns error during LndExecutor::new()
let result = LndExecutor::new(config);
assert!(result.is_err()); // Will fail for invalid PEM
```

### Preimage Verification

The `LightningExecutor::verify_preimage()` default implementation now uses real SHA256 hashing. If you were relying on the placeholder XOR-based implementation (unlikely), your tests may need updating.

---

## paykit-interactive Changes

### Message Size Limits

**New Constants**:
```rust
// Default maximum message size (1 MB)
pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 1024 * 1024;

// Alias for DEFAULT_MAX_MESSAGE_SIZE
pub const MAX_MESSAGE_SIZE: usize = DEFAULT_MAX_MESSAGE_SIZE;

// Maximum handshake message size (64 KB)
pub const MAX_HANDSHAKE_SIZE: usize = 65536;
```

**New Behavior**:
- Messages exceeding `MAX_MESSAGE_SIZE` are rejected with `InteractiveError::Transport`
- Handshake messages exceeding `MAX_HANDSHAKE_SIZE` are rejected
- Size validation happens BEFORE memory allocation (DoS protection)

**Customizing the Limit**:
```rust
let channel = PubkyNoiseChannel::connect(client, stream, server_pk)
    .await?
    .with_max_message_size(2 * 1024 * 1024)?; // 2 MB
```

**Migration**:
- If your use case requires messages > 1 MB, use `with_max_message_size()`
- Consider chunking large payloads instead of increasing the limit

---

## paykit-subscriptions Changes

### NonceStorage Trait

**Old Behavior (v1.x)**:
```rust
// NonceStore was internal-only, in-memory, lost on restart
let store = NonceStore::new();
store.check_and_mark(&nonce, expires_at)?;
```

**New Behavior (v2.0)**:
```rust
// NonceStorage trait for persistent implementations
pub trait NonceStorage: Send + Sync {
    fn check_and_mark(&self, nonce: &[u8; 32], expires_at: i64) -> Result<bool>;
    fn is_used(&self, nonce: &[u8; 32]) -> Result<bool>;
    fn cleanup_expired(&self, before: i64) -> Result<()>;
    fn count(&self) -> Result<usize>;
}
```

**Available Implementations**:

1. **In-Memory (Testing Only)**:
```rust
use paykit_subscriptions::NonceStore;
let store = NonceStore::new();
```

2. **File-Based (CLI/Demo)**:
```rust
use paykit_subscriptions::FileNonceStorage;
let storage = FileNonceStorage::in_directory(path)?;
```

3. **Android (SharedPreferences)**:
```kotlin
@Singleton
class NonceStorage @Inject constructor(context: Context) {
    suspend fun checkAndMark(nonce: String, expiresAt: Long): Boolean
    suspend fun isUsed(nonce: String): Boolean
    suspend fun cleanupExpired(before: Long): Int
    suspend fun count(): Int
}
```

4. **iOS (UserDefaults)**:
```swift
final class NonceStorage {
    func checkAndMark(nonce: String, expiresAt: Int64) -> Bool
    func isUsed(nonce: String) -> Bool
    func cleanupExpired(before: Int64) -> Int
    func count() -> Int
}
```

**Migration**:
1. Choose appropriate storage backend for your platform
2. Implement the `NonceStorage` trait or use a provided implementation
3. Ensure nonces persist across app restarts
4. Call `cleanup_expired()` periodically (e.g., on app startup)

### SpendingGuard

**New RAII Type for Panic-Safe Spending**:
```rust
use paykit_subscriptions::SpendingGuard;

// Create guard with reservation token
let guard = SpendingGuard::new(storage, token);

// Do payment work that might panic
match execute_payment().await {
    Ok(_) => {
        // Explicitly commit on success
        guard.commit().await?;
    }
    Err(e) => {
        // Guard auto-rolls-back on drop
        return Err(e);
    }
}
```

**Migration**:
- Replace manual `commit_spending`/`rollback_spending` with `SpendingGuard`
- Ensures no spending limit leaks on panic or early return

---

## Downstream Updates

### bitkit-android

1. **Regenerate UniFFI Bindings**:
```bash
cd paykit-mobile
cargo run --features=bindgen-cli --bin=generate-bindings
```

2. **Add NonceStorage**:
```kotlin
// Already implemented in to.bitkit.paykit.storage.NonceStorage
@Singleton
class NonceStorage @Inject constructor(context: Context) { ... }
```

3. **Update Dependencies**:
```gradle
// Update paykit-mobile version in build.gradle
```

### bitkit-ios

1. **Regenerate UniFFI Bindings**:
```bash
cd paykit-mobile
cargo run --features=bindgen-cli --bin=generate-bindings
```

2. **Add NonceStorage**:
```swift
// Already implemented in PaykitIntegration/Storage/NonceStorage.swift
final class NonceStorage { ... }
```

3. **Update Framework Version**:
- Rebuild `PaykitMobile.xcframework` with new version

### pubky-ring

No changes required - pubky-ring uses pubky-noise directly and does not depend on the affected paykit crates.

---

## Testing Your Migration

### Rust Tests

```bash
cd paykit-rs

# Run all tests
cargo test

# Run specific crate tests
cargo test -p paykit-lib
cargo test -p paykit-interactive
cargo test -p paykit-subscriptions
```

### Android Tests

```bash
cd bitkit-android
./gradlew testDevDebugUnitTest --tests NonceStorageTest
```

### iOS Tests

```bash
cd bitkit-ios
xcodebuild test -scheme Bitkit -only-testing:BitkitTests/NonceStorageTests
```

---

## Version Compatibility Matrix

| paykit-lib | paykit-interactive | paykit-subscriptions | paykit-mobile |
|------------|-------------------|---------------------|---------------|
| 1.0.x | 0.1.x | 0.2.x | 0.1.x |
| **2.0.0** | **0.2.0** | **0.3.0** | **0.2.0** |

All crates in a row are compatible with each other. Do not mix versions across rows.

---

## Rollback Procedure

If you encounter issues after upgrading:

1. Revert to previous Cargo.lock
2. Run `cargo update -p paykit-lib --precise 1.0.0`
3. Report the issue at https://github.com/synonymdev/paykit-rs/issues

---

## Security Contact

For security-related questions about this migration:
- security@synonym.to

---

**Document Version**: 1.0  
**Last Updated**: January 5, 2026

