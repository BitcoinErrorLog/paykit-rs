# Demo App Migration Guide for v0.2.0

## Status: ⚠️ Demo Apps Need Migration

The security fixes in `paykit-subscriptions` v0.2.0 introduce breaking API changes. The demo apps need updates to work with the new API.

## Required Changes

### 1. Amount Type Migration

**OLD (v0.1)**:
```rust
let amount = "1000".to_string();
let request = PaymentRequest::new(from, to, amount, currency, method);
```

**NEW (v0.2)**:
```rust
use paykit_subscriptions::Amount;

let amount = Amount::from_sats(1000);
let request = PaymentRequest::new(from, to, amount, currency, method);
```

### 2. Remove SigningKeyInfo Usage

**OLD (v0.1)**:
```rust
use paykit_subscriptions::signing::{KeyType, SigningKeyInfo};

let signed = SignedSubscription::new(
    subscription,
    sig1,
    sig2,
    SigningKeyInfo {
        subscriber_key_type: KeyType::Ed25519,
        provider_key_type: KeyType::Ed25519,
    },
);
```

**NEW (v0.2)**:
```rust
// KeyType and SigningKeyInfo removed
let signed = SignedSubscription::new(
    subscription,
    sig1,
    sig2,
);
```

### 3. Update Signature Verification

**OLD (v0.1)**:
```rust
if signed.verify_ed25519_signatures()? {
    // Valid
}
```

**NEW (v0.2)**:
```rust
if signed.verify_signatures()? {
    // Valid
}
```

### 4. Update Signing Calls

**OLD (v0.1)**:
```rust
let sig = signing::sign_subscription(&subscription, Some(&keypair), None)?;
```

**NEW (v0.2)**:
```rust
let nonce = rand::random::<[u8; 32]>();
let sig = signing::sign_subscription_ed25519(
    &subscription,
    &keypair,
    &nonce,
    3600 * 24 * 7, // 7 days lifetime
)?;
```

### 5. Display Amount Values

**OLD (v0.1)**:
```rust
ui::key_value("Amount", &request.amount); // String
```

**NEW (v0.2)**:
```rust
ui::key_value("Amount", &request.amount.to_string()); // Amount -> String
```

### 6. PeerSpendingLimit Changes

**OLD (v0.1)**:
```rust
let limit = limit.remaining_limit(); // Returns String
ui::key_value("Remaining", &limit);
```

**NEW (v0.2)**:
```rust
let limit = limit.remaining_limit(); // Returns Amount
ui::key_value("Remaining", &limit.to_string());
```

## Files Requiring Updates

### paykit-demo-cli
- `src/commands/subscriptions.rs` - Main subscription commands (~30 changes)
- Update all amount handling to use `Amount` type
- Remove `SigningKeyInfo` and `KeyType` usage
- Add `.to_string()` for Amount display

### paykit-demo-web  
- Similar changes needed in WASM bindings
- Update JavaScript interop to handle Amount serialization

### paykit-demo-core
- Update shared business logic for new types
- Ensure all payment/subscription operations use `Amount`

## Quick Fix Commands

For String to Amount conversions in payment requests:
```rust
// Find all: PaymentRequest::new(.., "AMOUNT".to_string(), ..)
// Replace with: PaymentRequest::new(.., Amount::from_sats(AMOUNT), ..)

// Find all: SubscriptionTerms::new("AMOUNT".to_string(), ..)
// Replace with: SubscriptionTerms::new(Amount::from_sats(AMOUNT), ..)
```

For displaying amounts:
```rust
// Find all: ui::key_value("...", &request.amount)
// Replace with: ui::key_value("...", &request.amount.to_string())
```

## Testing After Migration

After making changes, verify:
```bash
# Build all workspace members
cargo build --workspace

# Run all tests
cargo test --workspace

# Test CLI demo
cd paykit-demo-cli
cargo run -- --help

# Test core library
cargo test -p paykit-subscriptions
```

## Estimated Migration Effort

- **paykit-demo-cli**: ~2 hours (30-40 changes)
- **paykit-demo-web**: ~1-2 hours (WASM bindings + JS)
- **paykit-demo-core**: ~1 hour (shared logic)

**Total**: 4-5 hours for complete demo migration

## Migration Priority

1. **HIGH**: `paykit-subscriptions` core library ✅ COMPLETE
2. **MEDIUM**: `paykit-demo-core` - shared business logic
3. **MEDIUM**: `paykit-demo-cli` - command-line interface
4. **LOW**: `paykit-demo-web` - web interface (can be done separately)

## Notes

- The core `paykit-subscriptions` library is **production-ready** and fully tested
- Demo apps are **example/reference implementations** only
- Demo apps can be migrated incrementally
- All security fixes are in the core library, which is complete

## Support

For questions about migration:
- See `IMPLEMENTATION_COMPLETE.md` for core library changes
- See `SECURITY_FIXES_STATUS.md` for security improvements
- Check test files in `paykit-subscriptions/src/` for usage examples

