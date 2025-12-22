# Paykit Subscriptions - Build Instructions

**Crate**: `paykit-subscriptions`  
**Description**: P2P subscription protocol with auto-pay and spending limits  
**Type**: Library (no binary)  
**Version**: 0.2.0 (after security fixes)

---

## Prerequisites

### Required

- **Rust 1.70.0+** via Rustup
- **Cargo** (comes with Rust)
- **paykit-lib** (workspace dependency)
- **paykit-interactive** (workspace dependency)

### Installation

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Verify
rustc --version  # Should be 1.70.0 or higher
cargo --version
```

---

## Quick Build

```bash
# From workspace root
cd paykit-rs
cargo build -p paykit-subscriptions

# Or from this directory
cd paykit-subscriptions
cargo build

# Run tests (44 tests)
cargo test --lib
```

---

## Dependencies

### Workspace Dependencies

- **paykit-lib**: Core Paykit types
- **paykit-interactive**: Interactive payment protocol

### External Dependencies

- `pubky` (0.6.0-rc.6) - Pubky SDK
- `tokio` - Async runtime (full features)
- `serde` / `serde_json` - Serialization
- `postcard` - Deterministic binary serialization
- `ed25519-dalek` - Ed25519 signatures
- `rust_decimal` - Safe financial arithmetic
- `chrono` - Date/time operations
- `sha2` - Hashing
- `zeroize` - Secure memory handling
- `subtle` - Constant-time operations
- `fs2` - File locking for atomic operations

All external dependencies are automatically downloaded by Cargo.

---

## Building

### Development Build

```bash
cargo build -p paykit-subscriptions
```

**Output**: `target/debug/libpaykit_subscriptions.rlib`

### Release Build

```bash
cargo build -p paykit-subscriptions --release
```

**Output**: `target/release/libpaykit_subscriptions.rlib`

---

## Testing

### Run All Tests (44 tests)

```bash
cargo test -p paykit-subscriptions --lib
```

**Test Coverage**:
- ✅ Amount type safety (4 tests)
- ✅ Signature creation/verification (7 tests)
- ✅ Nonce store/replay protection (7 tests)
- ✅ Payment requests (3 tests)
- ✅ Subscriptions (4 tests)
- ✅ Auto-pay rules (3 tests)
- ✅ Spending limits (2 tests + atomic tests)
- ✅ Manager operations (3 tests)
- ✅ Storage operations (3 tests)
- ✅ Monitor (2 tests)

### Run Specific Test Suites

```bash
# Test Amount type
cargo test -p paykit-subscriptions amount

# Test cryptographic signatures
cargo test -p paykit-subscriptions signing

# Test nonce store (replay protection)
cargo test -p paykit-subscriptions nonce_store

# Test auto-pay
cargo test -p paykit-subscriptions autopay

# Test spending limits
cargo test -p paykit-subscriptions spending_limit
```

### Run with Output

```bash
cargo test -p paykit-subscriptions -- --nocapture
```

---

## Usage in Other Projects

### As a Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
paykit-subscriptions = { path = "../paykit-subscriptions" }
```

### In Code

```rust
use paykit_subscriptions::{
    Amount,
    PaymentRequest,
    Subscription,
    SubscriptionTerms,
    PaymentFrequency,
    AutoPayRule,
    PeerSpendingLimit,
    SubscriptionManager,
    FileSubscriptionStorage,
};

// Create a payment request
let amount = Amount::from_sats(1000);
let request = PaymentRequest::new(from, to, amount, "SAT".to_string(), method);

// Create subscription terms
let terms = SubscriptionTerms::new(
    Amount::from_sats(1000),
    "SAT".to_string(),
    PaymentFrequency::Monthly { day_of_month: 1 },
    method,
    "Monthly subscription".to_string(),
);

// Create auto-pay rule
let rule = AutoPayRule::new(subscription_id, peer, method)
    .with_max_payment_amount(Amount::from_sats(1000))
    .with_max_period_amount(Amount::from_sats(5000), "monthly".to_string());
```

---

## Project Structure

```
paykit-subscriptions/
├── Cargo.toml                # Package metadata
├── BUILD.md                  # This file
├── src/
│   ├── lib.rs               # Module exports
│   ├── amount.rs            # Safe financial arithmetic
│   ├── request.rs           # Payment requests
│   ├── subscription.rs      # Subscription agreements
│   ├── signing.rs           # Cryptographic signatures
│   ├── nonce_store.rs       # Replay attack prevention
│   ├── autopay.rs           # Auto-pay rules and limits
│   ├── storage.rs           # File-based storage
│   ├── manager.rs           # Subscription manager
│   └── monitor.rs           # Payment due detection
└── tests/
    ├── phase2_integration.rs  # Integration tests
    └── phase3_autopay.rs      # Auto-pay tests
```

---

## Key Features

### Safe Financial Arithmetic

Uses `rust_decimal::Decimal` to prevent floating-point errors:

```rust
use paykit_subscriptions::Amount;

let amount1 = Amount::from_sats(1000);
let amount2 = Amount::from_sats(500);

// Checked operations
let sum = amount1.checked_add(&amount2)?;      // Ok(1500)
let diff = amount1.checked_sub(&amount2)?;     // Ok(500)

// Saturating operations
let max = amount1.saturating_add(&amount2);    // 1500 (no overflow)

// Comparisons
assert!(amount1 > amount2);
assert!(amount2.is_within_limit(&amount1));
```

### Cryptographic Signatures (Ed25519 only)

Deterministic signatures with replay protection:

```rust
use paykit_subscriptions::signing;
use rand::RngCore;

// Generate random nonce (MUST be unique)
let mut nonce = [0u8; 32];
rand::thread_rng().fill_bytes(&mut nonce);

// Sign (valid for 7 days)
let signature = signing::sign_subscription_ed25519(
    &subscription,
    &keypair,
    &nonce,
    60 * 60 * 24 * 7,  // lifetime in seconds
)?;

// Verify
let valid = signing::verify_signature_ed25519(&subscription, &signature)?;
```

### Replay Attack Prevention

Nonce tracking to prevent signature reuse:

```rust
use paykit_subscriptions::NonceStore;

let store = NonceStore::new();

// Check and mark nonce as used
if store.check_and_mark(&nonce, expires_at).await? {
    // Fresh nonce - proceed
} else {
    // Replay attack detected - reject
}

// Cleanup expired nonces
store.cleanup_expired(now).await?;
```

### Atomic Spending Limits

File-based locking to prevent race conditions:

```rust
use paykit_subscriptions::{FileSpendingLimit, Amount};

let limit_manager = FileSpendingLimit::new(base_path)?;

// Atomic check-and-reserve
if let Some(token) = limit_manager.check_and_reserve(&peer, &amount).await? {
    // Amount reserved, perform payment
    make_payment()?;
    
    // Commit the spend
    token.commit()?;
} else {
    // Would exceed limit - reject
}
```

---

## Security Features (v0.2.0)

### ✅ Fixed in v0.2.0

1. **Deterministic Serialization**: Uses `postcard` instead of JSON
2. **Replay Protection**: Nonces + timestamps + expiration
3. **Safe Arithmetic**: `Amount` type with `rust_decimal`
4. **Atomic Limits**: File locking for spending limits
5. **Ed25519 Only**: Removed unsafe X25519-derived signatures
6. **Domain Separation**: Cryptographic operations use domain prefixes
7. **Constant-Time**: Uses `subtle` crate where applicable

### Breaking Changes from v0.1

- **Amount Type**: `String` → `Amount`
- **Signature API**: New signature format with nonces
- **Removed Types**: `KeyType`, `SigningKeyInfo`, X25519 support
- **Method Renames**: `verify_ed25519_signatures()` → `verify_signatures()`

See [DEMO_APP_FIX_COMPLETION.md](../DEMO_APP_FIX_COMPLETION.md) for migration guide.

---

## Development

### Code Quality

```bash
# Format code
cargo fmt --package paykit-subscriptions

# Lint code (with strict lints)
cargo clippy -p paykit-subscriptions --all-targets

# Check without building
cargo check -p paykit-subscriptions
```

### Documentation

```bash
# Generate docs
cargo doc -p paykit-subscriptions --no-deps

# Generate and open in browser
cargo doc -p paykit-subscriptions --no-deps --open
```

### Run Property-Based Tests

```bash
# Run proptest tests (for Amount arithmetic)
cargo test -p paykit-subscriptions amount_arithmetic_never_panics
cargo test -p paykit-subscriptions amount_serialization_preserves_value
```

---

## Troubleshooting

### Error: "could not find `paykit_lib`" or "could not find `paykit_interactive`"

**Problem**: Building outside workspace

**Solution**: Build from workspace root:
```bash
cd paykit-rs
cargo build -p paykit-subscriptions
```

### Test Failures Related to File Access

**Problem**: File-based tests need write permissions

**Solution**: Run tests with proper permissions:
```bash
cargo test -p paykit-subscriptions --lib -- --test-threads=1
```

### Warning: "unused import" in storage.rs

**Status**: Known issue with conditional imports

**To fix**: Run `cargo fix`:
```bash
cargo fix -p paykit-subscriptions --lib --allow-dirty
```

---

## Performance

### Build Time

- **Debug build**: ~50-80 seconds (first build)
- **Release build**: ~80-160 seconds (first build)
- **Incremental**: ~5-15 seconds (after changes)

### Test Time

- **All 44 tests**: ~2-3 seconds

### Binary Size

- **Debug**: ~700KB (unoptimized)
- **Release**: ~300KB (optimized with LTO)

---

## Lints and Safety

### Enforced Lints

```toml
[lints.rust]
unsafe_code = "forbid"           # No unsafe code allowed
float_arithmetic = "forbid"      # No floating-point for amounts
```

These are enforced at compile time to maintain security.

---

## API Stability

**Version**: 0.2.0  
**Status**: Stable for current feature set

### Compatibility

- ✅ v0.2.x releases: Backward compatible
- ⚠️ v0.1.x → v0.2.0: Breaking changes (security fixes)
- 🔄 v0.3.0+: May include additional features

---

## Related Documentation

- **Workspace BUILD.md**: [../BUILD.md](../BUILD.md)
- **Migration Guide**: [../DEMO_APP_FIX_COMPLETION.md](../DEMO_APP_FIX_COMPLETION.md)
- **Security Fixes**: [../SECURITY_FIXES_STATUS.md](../SECURITY_FIXES_STATUS.md)
- **paykit-lib BUILD.md**: [../paykit-lib/BUILD.md](../paykit-lib/BUILD.md)
- **paykit-interactive BUILD.md**: [../paykit-interactive/BUILD.md](../paykit-interactive/BUILD.md)

---

## Quick Reference

```bash
# Build
cargo build -p paykit-subscriptions

# Test (44 tests)
cargo test -p paykit-subscriptions --lib

# Test specific module
cargo test -p paykit-subscriptions signing

# Docs
cargo doc -p paykit-subscriptions --no-deps --open

# Format & Lint
cargo fmt --package paykit-subscriptions
cargo clippy -p paykit-subscriptions --all-targets

# Check security lints
cargo clippy -p paykit-subscriptions -- -D warnings
```

---

**For workspace-level build instructions, see [../BUILD.md](../BUILD.md)**

