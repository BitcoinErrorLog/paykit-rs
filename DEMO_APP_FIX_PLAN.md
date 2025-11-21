# Demo App Fix Plan - Phased Approach with Testing

## Overview

This plan fixes demo apps to work with `paykit-subscriptions` v0.2.0 while ensuring:
- ✅ No breaking changes to `paykit-lib` or `pubky-noise`
- ✅ Thorough testing between each phase
- ✅ Safe rollback points if issues arise
- ✅ Incremental progress with verification

## Critical Constraints

### ❌ DO NOT MODIFY (Without Permission)
- `paykit-lib/` - Core payment library
- `pubky-noise/` - Noise protocol implementation  
- `paykit-interactive/` - Interactive payment protocol
- Any `pubky` or `pkarr` dependencies

### ✅ SAFE TO MODIFY
- `paykit-demo-cli/` - CLI demo application
- `paykit-demo-web/` - Web demo application
- `paykit-demo-core/` - Shared demo logic
- Test files in demo apps

## Pre-Flight Verification

Before starting, verify baseline:
```bash
# Core library must pass
cd paykit-subscriptions
cargo test --lib
# Expected: 44 tests passed ✅

# Document current demo app state
cd ..
cargo build --workspace 2>&1 | grep "error\[E" | wc -l
# Expected: ~25-30 errors (to be fixed) ⚠️
```

## Phase 1: Fix paykit-demo-core (Foundation)

**Goal**: Update shared business logic that other demos depend on  
**Estimated Time**: 1 hour  
**Risk Level**: LOW (isolated changes)

### Changes Required

#### 1.1: Add Amount Import
**File**: `paykit-demo-core/src/lib.rs` or main module
```rust
// Add to imports
use paykit_subscriptions::Amount;
```

#### 1.2: Update Helper Functions
**Files**: Any amount-handling utilities

**OLD**:
```rust
fn format_amount(amount: &str, currency: &str) -> String {
    format!("{} {}", amount, currency)
}
```

**NEW**:
```rust
fn format_amount(amount: &Amount, currency: &str) -> String {
    format!("{} {}", amount, currency)
}
```

#### 1.3: Update Storage/State Structures
If demo-core has state management with amounts:

**OLD**:
```rust
struct PaymentState {
    amount: String,
}
```

**NEW**:
```rust
struct PaymentState {
    amount: Amount,
}
```

### Phase 1 Testing

**Test 1**: Compile demo-core alone
```bash
cd paykit-demo-core
cargo build
# Expected: ✅ SUCCESS or fewer errors than before
```

**Test 2**: Run demo-core tests
```bash
cargo test
# Expected: ✅ All tests pass
```

**Test 3**: Verify dependencies unaffected
```bash
cd ../paykit-lib
cargo test
# Expected: ✅ Still passing (no changes made)

cd ../paykit-interactive  
cargo test
# Expected: ✅ Still passing (no changes made)
```

**Checkpoint**: If any test fails, STOP and analyze before proceeding.

---

## Phase 2: Fix paykit-demo-cli (Part A: Types)

**Goal**: Update type declarations and imports  
**Estimated Time**: 30 minutes  
**Risk Level**: LOW (compilation will catch errors)

### Changes Required

#### 2.1: Update Imports
**File**: `paykit-demo-cli/src/commands/subscriptions.rs`

**ADD**:
```rust
use paykit_subscriptions::Amount;
```

**REMOVE**:
```rust
use paykit_subscriptions::signing::{KeyType, SigningKeyInfo};
// These types no longer exist
```

#### 2.2: Update Function Signatures
Find all functions that take/return amounts:

**OLD**:
```rust
fn create_request(amount: String, currency: String) -> PaymentRequest
```

**NEW**:
```rust
fn create_request(amount: Amount, currency: String) -> PaymentRequest
```

### Phase 2A Testing

**Test 1**: Compilation check
```bash
cd paykit-demo-cli
cargo build 2>&1 | grep "error\[E" | wc -l
# Expected: ⬇️ Fewer errors than before
```

**Test 2**: Core library still intact
```bash
cd ../paykit-subscriptions
cargo test --lib
# Expected: ✅ 44 tests still passing
```

**Checkpoint**: Errors should be decreasing. If errors increase, review changes.

---

## Phase 3: Fix paykit-demo-cli (Part B: Amount Conversions)

**Goal**: Convert all String amounts to Amount type  
**Estimated Time**: 1 hour  
**Risk Level**: MEDIUM (many changes, but mechanical)

### Changes Required

#### 3.1: PaymentRequest Creation
**Pattern to find**: `PaymentRequest::new(`

**OLD**:
```rust
PaymentRequest::new(
    from,
    to,
    amount.to_string(),      // ❌
    currency.to_string(),
    method,
)
```

**NEW**:
```rust
PaymentRequest::new(
    from,
    to,
    Amount::from_sats(amount),  // ✅
    currency.to_string(),
    method,
)
```

**Locations** (~5 occurrences):
- Line ~49: `send-request` command
- Line ~308: Subscription creation
- Similar patterns in other commands

#### 3.2: SubscriptionTerms Creation
**Pattern to find**: `SubscriptionTerms::new(`

**OLD**:
```rust
SubscriptionTerms::new(
    amount.to_string(),      // ❌
    currency.to_string(),
    frequency,
    method,
    description,
)
```

**NEW**:
```rust
SubscriptionTerms::new(
    Amount::from_sats(amount),  // ✅
    currency.to_string(),
    frequency,
    method,
    description,
)
```

**Locations** (~3 occurrences):
- Line ~308: propose-subscription command
- Similar patterns in accept/create flows

#### 3.3: PeerSpendingLimit Creation
**Pattern to find**: `PeerSpendingLimit::new(`

**OLD**:
```rust
PeerSpendingLimit::new(
    peer,
    limit.to_string(),   // ❌
    period.to_string(),
)
```

**NEW**:
```rust
PeerSpendingLimit::new(
    peer,
    Amount::from_sats(limit),  // ✅
    period.to_string(),
)
```

**Locations** (~1 occurrence):
- Line ~701: set-limit command

### Phase 3 Testing

**Test 1**: Compilation progress check
```bash
cd paykit-demo-cli
cargo build 2>&1 | grep "error\[E" | wc -l
# Expected: ⬇️ ~10-15 errors remaining (down from ~25)
```

**Test 2**: Type checking
```bash
cargo check
# Expected: Only display-related errors remain
```

**Test 3**: Dependencies unchanged
```bash
cd ../paykit-lib && cargo test
cd ../paykit-interactive && cargo test  
cd ../paykit-subscriptions && cargo test --lib
# Expected: ✅ All still passing
```

**Checkpoint**: Should have ~50% fewer errors. Main remaining errors: display/printing.

---

## Phase 4: Fix paykit-demo-cli (Part C: Display Conversions)

**Goal**: Add .to_string() for Amount display  
**Estimated Time**: 45 minutes  
**Risk Level**: LOW (pure display logic)

### Changes Required

#### 4.1: UI Display Calls
**Pattern to find**: `ui::key_value(.*, &.*amount.*)`

**OLD**:
```rust
ui::key_value("Amount", &request.amount);           // ❌ Amount not &str
ui::key_value("Current Spent", &limit.current_spent); // ❌
ui::key_value("Remaining", &limit.remaining_limit());  // ❌
```

**NEW**:
```rust
ui::key_value("Amount", &request.amount.to_string());              // ✅
ui::key_value("Current Spent", &limit.current_spent.to_string()); // ✅
ui::key_value("Remaining", &limit.remaining_limit().to_string()); // ✅
```

**Locations** (~15-20 occurrences):
- Line ~115: list-requests display
- Line ~156: show-request display
- Line ~211: request details
- Line ~363: subscription display
- Line ~429: list-subscriptions
- Line ~463: subscription details
- Line ~502: active subscriptions
- Line ~731-732: spending limits

#### 4.2: Format Strings
**Pattern to find**: `format!(.* {} .*request.amount.*)`

**OLD**:
```rust
format!("{} {}", request.amount, request.currency)  // ❌ Works but inconsistent
```

**NEW**:
```rust
format!("{} {}", request.amount.to_string(), request.currency)  // ✅ Explicit
// OR
format!("{} {}", request.amount, request.currency)  // ✅ Also works (Amount has Display)
```

**Locations** (~10 occurrences):
- Various display formatters

### Phase 4 Testing

**Test 1**: Full compilation
```bash
cd paykit-demo-cli
cargo build
# Expected: ✅ SUCCESS
```

**Test 2**: Run all CLI tests
```bash
cargo test
# Expected: ✅ All tests pass
```

**Test 3**: Smoke test CLI commands
```bash
cargo run -- --help
# Expected: ✅ Help displays

cargo run -- list-requests 2>/dev/null || true
# Expected: ✅ Runs without panic (may show "no requests")
```

**Test 4**: Full workspace still works
```bash
cd ..
cargo test -p paykit-subscriptions --lib
cargo test -p paykit-lib --lib  
cargo test -p paykit-interactive --lib
# Expected: ✅ All passing
```

**Checkpoint**: CLI should compile and run. Core libraries untouched.

---

## Phase 5: Fix paykit-demo-cli (Part D: Signature API)

**Goal**: Update signature creation/verification  
**Estimated Time**: 30 minutes  
**Risk Level**: MEDIUM (cryptographic operations)

### Changes Required

#### 5.1: Remove SigningKeyInfo Usage
**Pattern to find**: `SignedSubscription::new(`

**OLD**:
```rust
let signed = SignedSubscription::new(
    subscription,
    proposer_sig,
    acceptor_sig,
    SigningKeyInfo {                    // ❌ Removed
        subscriber_key_type: KeyType::Ed25519,
        provider_key_type: KeyType::Ed25519,
    },
);
```

**NEW**:
```rust
let signed = SignedSubscription::new(
    subscription,
    proposer_sig,
    acceptor_sig,
    // SigningKeyInfo removed from API ✅
);
```

**Locations** (~2-3 occurrences):
- Line ~379: accept-subscription command

#### 5.2: Update Signature Verification
**Pattern to find**: `verify_ed25519_signatures()`

**OLD**:
```rust
if signed.verify_ed25519_signatures()? {  // ❌ Method renamed
    // Valid
}
```

**NEW**:
```rust
if signed.verify_signatures()? {  // ✅ Simplified name
    // Valid
}
```

**Locations** (~1-2 occurrences):
- Subscription verification flows

#### 5.3: Update Signing Calls (If Any)
**Pattern to find**: `sign_subscription(`

**OLD**:
```rust
let sig = signing::sign_subscription(&sub, Some(&keypair), None)?;
```

**NEW**:
```rust
let nonce = rand::random::<[u8; 32]>();
let sig = signing::sign_subscription_ed25519(
    &sub,
    &keypair,
    &nonce,
    3600 * 24 * 7, // 7 days
)?;
```

**Note**: Check if demo-cli actually calls this directly. It may only use higher-level APIs.

### Phase 5 Testing

**Test 1**: Compilation
```bash
cd paykit-demo-cli
cargo build
# Expected: ✅ SUCCESS
```

**Test 2**: Signature tests
```bash
cargo test subscription
# Expected: ✅ Subscription-related tests pass
```

**Test 3**: Integration check
```bash
# Test if we can still interact with subscriptions
cargo run -- list-subscriptions 2>/dev/null || true
# Expected: ✅ Runs without error
```

**Checkpoint**: All CLI commands compile and signature operations work.

---

## Phase 6: Fix paykit-demo-web (WASM Bindings)

**Goal**: Update web demo for new APIs  
**Estimated Time**: 1-2 hours  
**Risk Level**: MEDIUM (WASM interop is tricky)

### Changes Required

#### 6.1: Update WASM Exports
**File**: `paykit-demo-web/src/lib.rs`

**Pattern**: Amount handling in WASM exports

**OLD**:
```rust
#[wasm_bindgen]
pub fn create_payment_request(
    from: String,
    to: String,
    amount: String,  // ❌
    currency: String,
) -> Result<JsValue, JsValue>
```

**NEW**:
```rust
#[wasm_bindgen]
pub fn create_payment_request(
    from: String,
    to: String,
    amount_sats: i64,  // ✅ Pass as number, convert internally
    currency: String,
) -> Result<JsValue, JsValue> {
    let amount = Amount::from_sats(amount_sats);
    // ... rest of function
}
```

#### 6.2: Amount Serialization for JS
**Strategy**: Convert Amount to/from strings at WASM boundary

```rust
// For outputs to JS:
#[wasm_bindgen]
pub struct PaymentRequestJS {
    pub from: String,
    pub to: String,
    pub amount: String,  // Serialize Amount as string for JS
    pub currency: String,
}

impl From<PaymentRequest> for PaymentRequestJS {
    fn from(req: PaymentRequest) -> Self {
        Self {
            from: req.from.to_string(),
            to: req.to.to_string(),
            amount: req.amount.to_string(),  // ✅
            currency: req.currency,
        }
    }
}
```

#### 6.3: Update JavaScript Side (If Needed)
**File**: `paykit-demo-web/www/` or similar

May need to update JS code to pass amounts as numbers instead of strings.

### Phase 6 Testing

**Test 1**: WASM compilation
```bash
cd paykit-demo-web
cargo build --target wasm32-unknown-unknown
# Expected: ✅ SUCCESS
```

**Test 2**: wasm-pack build
```bash
wasm-pack build --target web
# Expected: ✅ Generates pkg/ directory
```

**Test 3**: Check JS bindings
```bash
ls pkg/*.js pkg/*.d.ts
# Expected: ✅ TypeScript definitions look correct
```

**Test 4**: Core libraries still intact
```bash
cd ../paykit-subscriptions && cargo test --lib
# Expected: ✅ 44 tests passing
```

**Checkpoint**: WASM builds successfully. May need browser testing.

---

## Phase 7: Integration Testing

**Goal**: Verify all components work together  
**Estimated Time**: 1 hour  
**Risk Level**: LOW (validation only)

### Test Suite

#### 7.1: Workspace Build
```bash
cd /path/to/paykit-rs-master
cargo build --workspace
# Expected: ✅ SUCCESS (no errors)
```

#### 7.2: Workspace Tests
```bash
cargo test --workspace --lib
# Expected: ✅ All library tests pass
```

#### 7.3: Individual Package Tests
```bash
# Core library (most critical)
cargo test -p paykit-subscriptions --lib
# Expected: ✅ 44/44 tests pass

# Demo core
cargo test -p paykit-demo-core
# Expected: ✅ All tests pass

# Demo CLI  
cargo test -p paykit-demo-cli
# Expected: ✅ All tests pass
```

#### 7.4: CLI Smoke Tests
```bash
cd paykit-demo-cli

# Help command
cargo run -- --help
# Expected: ✅ Displays help

# Setup command
cargo run -- setup --name "Test User"
# Expected: ✅ Creates identity

# List commands
cargo run -- list-requests
cargo run -- list-subscriptions  
# Expected: ✅ Runs without error (may be empty)
```

#### 7.5: Dependency Verification (CRITICAL)
```bash
# Verify we didn't break anything we shouldn't have
cd ../paykit-lib
git diff src/
# Expected: ✅ No changes

cd ../paykit-interactive
git diff src/
# Expected: ✅ No changes

cd ../pubky-noise
git diff src/  
# Expected: ✅ No changes
```

---

## Phase 8: Documentation & Cleanup

**Goal**: Document changes and update examples  
**Estimated Time**: 30 minutes  
**Risk Level**: NONE (docs only)

### Tasks

1. Update CLI README with new examples
2. Add migration notes to demo-core docs
3. Update web demo documentation
4. Add comments for tricky Amount conversions
5. Document any WASM-specific patterns

---

## Success Criteria

### ✅ Completion Checklist

- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace --lib` passes all tests  
- [ ] `paykit-subscriptions` still has 44/44 tests passing
- [ ] `paykit-lib` unchanged (git diff shows no changes)
- [ ] `paykit-interactive` unchanged
- [ ] `pubky-noise` unchanged
- [ ] CLI commands run without panic
- [ ] WASM builds successfully
- [ ] Documentation updated

### ⚠️ Rollback Points

If at any phase:
- Core library tests fail → STOP, revert Phase N changes
- Dependencies show unexpected changes → STOP, review git diff
- Errors increase instead of decrease → STOP, review approach

---

## Estimated Timeline

| Phase | Description | Time | Cumulative |
|-------|-------------|------|------------|
| 0 | Pre-flight checks | 15 min | 15 min |
| 1 | Fix demo-core | 1 hour | 1h 15m |
| 2 | CLI types | 30 min | 1h 45m |
| 3 | CLI amounts | 1 hour | 2h 45m |
| 4 | CLI display | 45 min | 3h 30m |
| 5 | CLI signatures | 30 min | 4h |
| 6 | Web WASM | 1-2 hours | 5-6h |
| 7 | Integration tests | 1 hour | 6-7h |
| 8 | Documentation | 30 min | 6.5-7.5h |

**Total**: 6.5-7.5 hours (with testing)

---

## Risk Mitigation

### Low Risk Changes
- Adding `.to_string()` to Amount values
- Updating type signatures
- Removing unused imports

### Medium Risk Changes  
- Signature API updates (test thoroughly)
- WASM bindings (verify JS interop)

### High Risk Changes
- None (we're not modifying core libraries)

### Emergency Rollback
```bash
# If something goes wrong
git status
git diff > my_changes.patch  # Save work
git checkout -- path/to/broken/file  # Revert specific file
# Or
git stash  # Stash all changes
# Then re-apply incrementally
```

---

## Notes for Implementation

1. **Work incrementally**: Complete each phase fully before moving to next
2. **Test frequently**: Run tests after every 5-10 changes
3. **Commit often**: Small commits make rollback easier
4. **Don't skip verification**: The dependency checks are critical
5. **Ask before modifying**: If you're unsure about a file, ASK

---

**Plan Created**: 2025-11-20  
**Status**: Ready for Implementation  
**Next Step**: Begin Phase 0 (Pre-flight checks)

