# Paykit Testing & Audit Plan

> **Production-Ready Audit Framework for Paykit Rust Libraries**
> 
> This document provides a comprehensive, systematic approach to auditing and testing the Paykit codebase. It is designed to be used before releases, between development phases, and during security reviews.

**Document Version:** 1.0  
**Last Updated:** 2025-11-20  
**Applicable To:** paykit-lib, paykit-interactive, paykit-subscriptions

---

## Table of Contents

1. [Introduction & Purpose](#1-introduction--purpose)
2. [Quick Reference Checklist](#2-quick-reference-checklist)
3. [Issue Severity Classification](#3-issue-severity-classification--stop-criteria)
4. [Stage 1: Threat Model & Architecture Review](#stage-1-threat-model--architecture-review)
5. [Stage 2: Cryptography Audit (Zero Tolerance)](#stage-2-cryptography-audit-zero-tolerance)
6. [Stage 3: Rust Safety & Correctness](#stage-3-rust-safety--correctness)
7. [Stage 4: Testing Requirements (Non-Negotiable)](#stage-4-testing-requirements-non-negotiable)
8. [Stage 5: Documentation & Commenting](#stage-5-documentation--commenting)
9. [Stage 6: Build & CI Verification](#stage-6-build--ci-verification)
10. [Stage 7: Final Audit Report](#stage-7-final-audit-report)
11. [Code Completeness Checks](#code-completeness-checks)
12. [Stack-Specific Considerations](#stack-specific-considerations)

---

## 1. Introduction & Purpose

### 1.1 Scope

This audit plan covers the following **production crates**:

- **`paykit-lib/`** - Core library with transport abstractions for payment method discovery
- **`paykit-interactive/`** - Interactive payment protocol using Pubky Noise for encrypted channels
- **`paykit-subscriptions/`** - P2P subscriptions with Ed25519 signatures and replay protection

**Excluded from audit:** Demo applications (`paykit-demo-cli`, `paykit-demo-web`, `paykit-demo-core`)

### 1.2 When to Use This Plan

- **Pre-Release:** Before tagging any release version
- **Post-Feature:** After completing major features (especially crypto/financial)
- **Security Reviews:** Periodic security audits
- **Code Reviews:** Deep reviews of pull requests touching security-critical code
- **Handoffs:** Before transferring project ownership

### 1.3 How to Use This Plan

**Quick Sweep (30-60 minutes):**
- Run Section 2 (Quick Reference Checklist)
- Review automated command outputs
- Document any CRITICAL or HIGH severity issues

**Deep Audit (4-8 hours):**
- Execute all 7 stages sequentially
- **STOP at each stage if CRITICAL issues found**
- Document findings using templates in Section 10
- Sign off on each stage before proceeding

**Between-Phase Review (1-2 hours):**
- Focus on Stages 2, 3, and 4 (crypto, safety, testing)
- Verify no regressions in security-critical areas

---

## 2. Quick Reference Checklist

### 2.1 One-Page Rapid Audit

Run these commands from the repository root:

```bash
# 1. Build verification
cargo build --workspace --locked --all-targets --all-features
cargo build --workspace --release --locked

# 2. Static analysis
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check

# 3. Test execution
cargo test --workspace --all-features

# 4. Security audit
cargo audit

# 5. Documentation build
cargo doc --workspace --no-deps

# 6. Code completeness
grep -r "TODO\|FIXME\|todo!\|unimplemented!" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src
```

### 2.2 Pass/Fail Criteria

| Check | Pass Criteria | Action if Failed |
|-------|---------------|------------------|
| Build | All builds succeed | STOP - fix compilation errors |
| Clippy | Zero warnings with `-D warnings` | STOP if security-related, else document |
| Tests | All tests pass | STOP - investigate failures |
| Cargo Audit | No HIGH/CRITICAL vulnerabilities | STOP - update dependencies |
| Documentation | Builds without warnings | MEDIUM - fix before release |
| TODOs | Zero in `src/` of production crates | HIGH - complete or remove |

---

## 3. Issue Severity Classification & Stop Criteria

### 3.1 Severity Levels

#### 🔴 CRITICAL (MUST STOP)

**Characteristics:**
- Cryptographic vulnerabilities (weak primitives, nonce reuse, timing attacks)
- Memory safety issues (unjustified `unsafe` code, use-after-free)
- Financial arithmetic bugs (overflow, underflow, precision loss in `Amount`)
- Replay attack vectors (missing nonce checks, no expiration)
- Authentication bypass (signature validation failures)

**Action:** ⛔ **STOP IMMEDIATELY. Do not proceed to next stage until fixed and verified.**

**Examples:**
```rust
// CRITICAL: Nonce reuse
let nonce = [0u8; 32]; // ❌ Hardcoded nonce

// CRITICAL: Integer overflow
let amount = a + b; // ❌ No overflow check

// CRITICAL: Weak crypto
use md5::Md5; // ❌ Banned primitive
```

#### 🟠 HIGH (STOP RECOMMENDED)

**Characteristics:**
- Missing tests for security-critical code
- `unwrap()`/`panic!()` in production paths
- Missing input validation on external data
- Improper error handling exposing sensitive data
- Incomplete documentation for security features

**Action:** 🛑 **STOP and fix, or document explicit acceptance of risk with mitigation plan.**

**Examples:**
```rust
// HIGH: Unwrap in production
let key = get_key().unwrap(); // ❌ Can panic

// HIGH: Missing validation
pub fn process(data: Vec<u8>) { // ❌ No length check
    // ...
}
```

#### 🟡 MEDIUM (DOCUMENT & CONTINUE)

**Characteristics:**
- Code quality issues (clippy warnings)
- Non-critical missing tests
- Documentation gaps in non-security code
- Minor performance issues

**Action:** 📝 **Document findings, create remediation tickets, continue audit.**

#### 🟢 LOW (DOCUMENT ONLY)

**Characteristics:**
- Style inconsistencies
- Optimization opportunities
- Nice-to-have features

**Action:** 📋 **Document for future consideration, continue audit.**

### 3.2 Stage Completion Checklist

After **each stage**, verify:

- [ ] All commands executed successfully
- [ ] All issues classified by severity
- [ ] All CRITICAL and HIGH issues addressed or explicitly accepted
- [ ] Findings documented in audit report (use template in Stage 7)
- [ ] Sign-off recorded before proceeding

**⚠️ NEVER proceed to the next stage with unresolved CRITICAL issues.**

---

## Stage 1: Threat Model & Architecture Review

### 1.1 Stop/Go Criteria

**🛑 STOP Criteria:**
- Architecture violates security principles (not stateless, improper abstraction)
- Missing threat model for critical component
- Insecure design patterns identified (e.g., storing secrets in memory unnecessarily)

**✅ GO Criteria:**
- All components have documented threat models
- Architecture follows separation of concerns
- Trait-based dependency injection properly implemented
- No fundamental security design flaws

### 1.2 Component Threat Models

#### Paykit-Lib Threat Model

**Purpose:** Payment method discovery via Pubky homeservers

**Threat Actors:**
1. **Network Attacker** - Intercepts discovery requests
2. **Malicious Homeserver** - Serves fake payment endpoints
3. **Compromised Application** - Misuses transport API

**Key Threats:**
- T1: Attacker serves fraudulent payment endpoints
- T2: Privacy leakage (who queries whom)
- T3: Denial of service (homeserver unavailable)

**Mitigations:**
- ✅ Transport layer abstraction (TLS handled by Pubky SDK)
- ✅ Stateless design (no secrets stored)
- ✅ Unauthenticated reads (public discovery)
- ✅ Authenticated writes (requires valid session)
- ⚠️ Application MUST verify payment methods out-of-band

**Residual Risks:**
- MEDIUM: Malicious homeserver can lie about endpoints
- LOW: Privacy leakage at network layer (Pubky responsibility)

#### Paykit-Interactive Threat Model

**Purpose:** Encrypted payment negotiation over Noise Protocol

**Threat Actors:**
1. **Network Attacker** - MITM attempts, eavesdropping
2. **Malicious Peer** - Protocol violations, DoS
3. **Compromised Device** - Key extraction

**Key Threats:**
- T1: Man-in-the-middle attack on handshake
- T2: Replay of payment receipts
- T3: Resource exhaustion (connection flooding)
- T4: Key material extraction from memory

**Mitigations:**
- ✅ Noise IK pattern (mutual authentication)
- ✅ Ed25519 identity binding
- ✅ ChaCha20-Poly1305 AEAD encryption
- ✅ Delegates to `pubky-noise` (vetted implementation)
- ✅ Receipt includes timestamps (application should check)
- ⚠️ Application MUST implement rate limiting
- ⚠️ Application MUST verify receipt uniqueness

**Residual Risks:**
- MEDIUM: DoS possible without application-level rate limits
- LOW: Memory dump attack (OS-level protection required)

#### Paykit-Subscriptions Threat Model

**Purpose:** Cryptographically signed recurring payment agreements

**Threat Actors:**
1. **Malicious Subscriber** - Forges signatures, replays old subscriptions
2. **Malicious Provider** - Claims unbounded amounts
3. **Network Attacker** - Modifies subscription terms in transit
4. **Integer Overflow Attacker** - Crafts amounts causing arithmetic errors

**Key Threats:**
- T1: Signature forgery (fake subscriptions)
- T2: Replay attack (reuse old subscription)
- T3: Nonce reuse (breaks replay protection)
- T4: Integer overflow in amount calculations
- T5: Timing attack on signature verification
- T6: Spending limit bypass (race conditions)

**Mitigations:**
- ✅ Ed25519 digital signatures (256-bit security)
- ✅ Deterministic serialization (postcard)
- ✅ Domain separation (`PAYKIT_SUBSCRIPTION_V2`)
- ✅ Replay protection (nonce + timestamp + expiration)
- ✅ `Amount` type with checked arithmetic (no overflow)
- ✅ `subtle` crate for constant-time operations
- ✅ `NonceStore` for tracking used nonces
- ⚠️ Application MUST persist nonce store across restarts
- ⚠️ Application MUST implement atomic spending limit checks

**Residual Risks:**
- LOW: Cryptographic break (requires breaking Ed25519)
- MEDIUM: Nonce store not persisted (replay after restart)
- MEDIUM: Spending limit race conditions (if not atomic)

### 1.3 Architecture Assessment

**Core Principles:**

1. **Stateless Libraries** ✅
   - No global state
   - No persistent connections
   - Caller manages sessions

2. **Trait-Based Dependency Injection** ✅
   - `AuthenticatedTransport` / `UnauthenticatedTransportRead`
   - Testable with mock implementations
   - No concrete SDK coupling in library core

3. **Feature Gating** ✅
   - `pubky` feature optional in paykit-lib
   - Platform-specific dependencies (WASM vs native)

4. **Error Propagation** ✅
   - No panics in library code
   - Detailed error types
   - Transport errors wrapped

**Verification Commands:**

```bash
# Check for global state (should be minimal)
grep -r "static mut\|lazy_static\|once_cell" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src

# Verify trait boundaries
grep -r "impl.*Transport" --include="*.rs" paykit-lib/src
```

### 1.4 FFI Boundary Safety (Future)

**Status:** No FFI currently implemented (WASM support excluded from workspace)

**Future Considerations:**
- UniFFI for mobile (Android/iOS)
- wasm-bindgen for web
- Memory management across FFI boundary
- Error handling without panics

### 1.5 Stage 1 Sign-Off

- [ ] All threat models documented and reviewed
- [ ] Architecture follows security principles
- [ ] No fundamental design flaws identified
- [ ] Residual risks are acceptable

**Auditor:** ________________  **Date:** __________

---

## Stage 2: Cryptography Audit (Zero Tolerance)

### 2.1 Stop/Go Criteria

**🛑 STOP Criteria:**
- Weak or banned cryptographic primitives detected
- Nonce reuse possible
- Non-constant-time operations on secrets
- Missing or improper domain separation
- Integer overflow in financial calculations

**✅ GO Criteria:**
- All crypto uses vetted libraries (ed25519-dalek, pubky-noise)
- Nonces are cryptographically random and unique
- Deterministic serialization verified
- Constant-time operations for sensitive comparisons
- Amount type prevents all overflow/underflow

### 2.2 Paykit-Subscriptions Cryptography (CRITICAL)

#### 2.2.1 Ed25519 Signature Implementation

**File:** `paykit-subscriptions/src/signing.rs`

**Critical Functions:**
- `sign_subscription_ed25519()` - Creates signatures
- `verify_signature_ed25519()` - Verifies signatures
- `hash_subscription_canonical()` - Deterministic hashing

**Verification Checklist:**

- [ ] **Deterministic Serialization**
  ```bash
  # Verify postcard usage (deterministic)
  grep -n "postcard::to_allocvec" paykit-subscriptions/src/signing.rs
  # Should appear in hash_subscription_canonical()
  ```

- [ ] **Domain Separation**
  ```bash
  # Verify domain constant present
  grep -n "SUBSCRIPTION_DOMAIN\|PAYKIT_SUBSCRIPTION_V2" \
    paykit-subscriptions/src/signing.rs
  # Should be included in SignaturePayload
  ```

- [ ] **Nonce Randomness**
  ```rust
  // ✅ CORRECT: Cryptographically random nonce
  use rand::RngCore;
  let mut nonce = [0u8; 32];
  rand::thread_rng().fill_bytes(&mut nonce);
  
  // ❌ CRITICAL: Never use deterministic nonce
  let nonce = [0u8; 32]; // Hardcoded
  let nonce = hash(data); // Predictable
  ```

- [ ] **Replay Protection**
  ```bash
  # Verify Signature struct includes replay protection fields
  grep -A 10 "pub struct Signature" paykit-subscriptions/src/signing.rs
  # Must have: nonce, timestamp, expires_at
  ```

- [ ] **Expiration Check**
  ```bash
  # Verify expiration checked BEFORE crypto verification
  grep -B 5 -A 5 "expires_at" paykit-subscriptions/src/signing.rs
  # Should check "now > signature.expires_at" early
  ```

- [ ] **No X25519 Signing** (removed in v0.2)
  ```bash
  # Should return 0
  grep -r "x25519.*sign\|X25519.*Sign" --include="*.rs" \
    paykit-subscriptions/src
  ```

#### 2.2.2 Nonce Store Implementation

**File:** `paykit-subscriptions/src/nonce_store.rs`

**Critical Requirements:**
- Prevent nonce reuse (replay protection)
- Thread-safe (atomic operations)
- Persistent across restarts (storage required)

**Verification Checklist:**

- [ ] **Atomic Operations**
  ```bash
  # Check for proper synchronization
  grep -n "Arc\|Mutex\|RwLock" paykit-subscriptions/src/nonce_store.rs
  ```

- [ ] **Persistence Warning**
  ```bash
  # Must document that applications need to persist store
  grep -n "persist\|storage\|restart" paykit-subscriptions/src/nonce_store.rs
  ```

- [ ] **Expiration Cleanup**
  ```bash
  # Check if old nonces are cleaned up
  grep -n "cleanup\|prune\|expire" paykit-subscriptions/src/nonce_store.rs
  ```

#### 2.2.3 Amount Type Safety

**File:** `paykit-subscriptions/src/amount.rs`

**Critical Requirements:**
- No integer overflow/underflow
- No floating-point for monetary values
- Deterministic serialization
- Precision preservation

**Verification Checklist:**

- [ ] **Checked Arithmetic**
  ```bash
  # All operations must use checked_* methods
  grep -n "checked_add\|checked_sub\|checked_mul" \
    paykit-subscriptions/src/amount.rs
  
  # Should NOT find unchecked operations
  grep -n "fn add\|fn sub\|fn mul" paykit-subscriptions/src/amount.rs | \
    grep -v "checked"
  ```

- [ ] **No Floating Point**
  ```bash
  # Should return 0
  grep -r "f32\|f64\|float" --include="*.rs" \
    paykit-subscriptions/src/amount.rs paykit-subscriptions/src/subscription.rs
  ```

- [ ] **Uses rust_decimal**
  ```bash
  # Verify rust_decimal is used (arbitrary precision)
  grep -n "rust_decimal\|Decimal" paykit-subscriptions/src/amount.rs
  ```

### 2.3 Paykit-Interactive Cryptography

**Status:** Delegates to `pubky-noise` crate (vetted separately)

**Verification Checklist:**

- [ ] **Noise Protocol Version**
  ```bash
  # Check dependency version
  grep "pubky-noise" paykit-interactive/Cargo.toml
  # Verify it's from known good path/version
  ```

- [ ] **Handshake Pattern**
  ```bash
  # Verify IK pattern usage
  grep -n "client_start_ik\|server_accept_ik" \
    paykit-interactive/src/*.rs paykit-interactive/tests/*.rs
  ```

- [ ] **Receipt Security**
  ```bash
  # Verify receipts include timestamps
  grep -n "timestamp" paykit-interactive/src/*.rs
  # Application must validate uniqueness
  ```

### 2.4 Banned Primitives Detection

Run these commands to detect weak cryptography:

```bash
# Search for weak/banned primitives (MUST return 0)
echo "Searching for banned crypto primitives..."

# MD5 (broken)
grep -ri "\\bmd5\\b\|use.*md5" --include="*.rs" \
  --exclude-dir=target --exclude-dir=paykit-demo* \
  paykit-lib/ paykit-interactive/ paykit-subscriptions/ | wc -l

# SHA-1 (broken for signatures)
grep -ri "\\bsha1\\b\|use.*sha1[^_]" --include="*.rs" \
  --exclude-dir=target --exclude-dir=paykit-demo* \
  paykit-lib/ paykit-interactive/ paykit-subscriptions/ | wc -l

# RC4 (broken)
grep -ri "\\brc4\\b" --include="*.rs" \
  --exclude-dir=target --exclude-dir=paykit-demo* \
  paykit-lib/ paykit-interactive/ paykit-subscriptions/ | wc -l

# DES (broken)
grep -ri "\\bdes[^c]" --include="*.rs" \
  --exclude-dir=target --exclude-dir=paykit-demo* \
  paykit-lib/ paykit-interactive/ paykit-subscriptions/ | wc -l

echo "All counts must be 0. Any non-zero is CRITICAL."
```

### 2.5 Constant-Time Operations

**Critical Areas:**

```bash
# Verify subtle crate usage for constant-time comparisons
grep -n "use subtle" paykit-subscriptions/src/*.rs

# Check for timing-sensitive comparisons
grep -n "ConstantTimeEq\|ct_eq" paykit-subscriptions/src/*.rs
```

### 2.6 Key Management Review

**Verification Checklist:**

- [ ] **No Hardcoded Keys**
  ```bash
  # Should return 0
  grep -r "secret.*=.*\[.*\]\|private.*key.*=" --include="*.rs" \
    paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
    grep -v "test"
  ```

- [ ] **Zeroization**
  ```bash
  # Check for Zeroizing wrapper usage
  grep -r "Zeroizing\|zeroize" --include="*.rs" \
    paykit-subscriptions/Cargo.toml paykit-subscriptions/src/
  ```

- [ ] **No Key Logging**
  ```bash
  # Check that keys are never logged
  grep -r "debug!.*key\|info!.*key\|println!.*key" --include="*.rs" \
    paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
    grep -v "public"
  ```

### 2.7 Stage 2 Sign-Off

- [ ] All cryptographic implementations reviewed
- [ ] No banned primitives detected
- [ ] Nonce management is secure
- [ ] Amount type prevents overflow/underflow
- [ ] Replay protection properly implemented
- [ ] Constant-time operations used where required

**Auditor:** ________________  **Date:** __________

---

## Stage 3: Rust Safety & Correctness

### 3.1 Stop/Go Criteria

**🛑 STOP Criteria:**
- Unjustified `unsafe` blocks in paykit code
- `unwrap()`/`panic!()` in production paths
- Send/Sync violations for async code
- Improper interior mutability

**✅ GO Criteria:**
- Zero `unsafe` in paykit code (delegates to vetted libs)
- All errors properly propagated
- Async code is cancellation-safe
- Interior mutability properly synchronized

### 3.2 Unsafe Code Audit

**Expectation:** Zero `unsafe` blocks in paykit production code

```bash
echo "=== Unsafe Block Count ==="
# Should return 0 for paykit code
unsafe_count=$(grep -r "unsafe" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
  grep -v "test" | wc -l)

echo "Unsafe blocks in production code: $unsafe_count"

if [ "$unsafe_count" -gt 0 ]; then
  echo "❌ CRITICAL: Found unsafe blocks - must justify each one"
  grep -rn "unsafe" --include="*.rs" \
    paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
    grep -v "test"
else
  echo "✅ PASS: No unsafe blocks found"
fi
```

**If `unsafe` Found:** Each block MUST have:
1. Comment explaining why `unsafe` is necessary
2. Invariants that make it safe
3. Reference to Rust documentation
4. Review by second engineer

### 3.3 Panic Safety Audit

**Critical:** Production code must not panic on user input

```bash
echo "=== Panic/Unwrap/Expect Audit ==="

# Find unwrap/expect/panic in production code
panic_count=$(grep -r "\.unwrap()\|\.expect(\|panic!\|todo!\|unimplemented!" \
  --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
  grep -v "test" | grep -v "example" | wc -l)

echo "Panic-prone calls in production: $panic_count"

if [ "$panic_count" -gt 0 ]; then
  echo "⚠️  HIGH: Found panic-prone code"
  grep -rn "\.unwrap()\|\.expect(\|panic!" --include="*.rs" \
    paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
    grep -v "test" | grep -v "example"
  echo "Each instance must be justified or replaced with proper error handling"
else
  echo "✅ PASS: No panic-prone code found"
fi
```

**Acceptable Uses of `unwrap()`/`expect()`:**
- After explicit length check (e.g., `if len == 64 { array.try_into().unwrap() }`)
- In test code only
- On infallible operations (must document why)

### 3.4 Error Propagation Review

**Verification:**

```bash
# Check that Result types are used consistently
grep -rn "pub fn" --include="*.rs" paykit-lib/src \
  paykit-interactive/src paykit-subscriptions/src | \
  grep -v "Result\|test\|example" | head -20

echo "Review above functions - should most return Result?"
```

### 3.5 Send/Sync Correctness

**For Async Code:**

```bash
# Find async functions and check their safety
grep -rn "async fn" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src

# Check for potential Send/Sync issues
grep -rn "Arc\|Mutex\|RwLock\|Rc\|RefCell" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src
```

**Checklist:**
- [ ] `Arc` used for shared state (not `Rc`)
- [ ] `Mutex`/`RwLock` used for interior mutability (not `RefCell`)
- [ ] No `Rc<RefCell<T>>` in async code
- [ ] Async traits properly handled

### 3.6 Interior Mutability Review

**Files to Check:**
- `paykit-subscriptions/src/nonce_store.rs` - Must use Arc<Mutex<_>>
- `paykit-subscriptions/src/manager.rs` - Spending limit synchronization

```bash
# Review interior mutability patterns
echo "=== Interior Mutability Patterns ==="
grep -rn "Arc<Mutex\|Arc<RwLock" --include="*.rs" \
  paykit-subscriptions/src/

# Check for problematic patterns
echo "=== Checking for Rc/RefCell (bad in async) ==="
grep -rn "Rc<\|RefCell<" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
  grep -v "test"
```

### 3.7 Drop Order and Leak Freedom

**Verification:**

- [ ] No circular `Arc` references
- [ ] Resources cleaned up on Drop
- [ ] Async cancellation doesn't leak resources

```bash
# Look for potential circular references
grep -rn "Arc.*Arc\|Rc.*Rc" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src
```

### 3.8 Async Cancellation Safety

**For paykit-interactive** (uses async):

- [ ] `.await` points are cancellation-safe
- [ ] No partial updates left on cancellation
- [ ] Timeouts don't corrupt state

```bash
# Review timeout implementations
grep -rn "timeout\|Timeout" --include="*.rs" \
  paykit-interactive/src/
```

### 3.9 Stage 3 Sign-Off

- [ ] Zero unsafe blocks (or all justified)
- [ ] No unwrap/panic in production paths (or all justified)
- [ ] Send/Sync correct for async code
- [ ] Interior mutability properly synchronized
- [ ] Drop order correct, no leaks
- [ ] Async cancellation safe

**Auditor:** ________________  **Date:** __________

---

## Stage 4: Testing Requirements (Non-Negotiable)

### 4.1 Stop/Go Criteria

**🛑 STOP Criteria:**
- Security-critical code lacks tests
- Cryptographic functions missing negative test cases
- Determinism tests missing for serialization
- Test coverage below 80% for critical modules

**✅ GO Criteria:**
- All public APIs have integration tests
- Crypto functions have positive + negative tests
- Serialization tested for determinism
- Error paths tested

### 4.2 Test Execution

Run complete test suite:

```bash
echo "=== Running Full Test Suite ==="

# All tests
cargo test --workspace --all-features

# Individual crates
echo "--- Testing paykit-lib ---"
cd paykit-lib && cargo test && cd ..

echo "--- Testing paykit-interactive ---"
cd paykit-interactive && cargo test && cd ..

echo "--- Testing paykit-subscriptions ---"
cd paykit-subscriptions && cargo test && cd ..

# Integration tests
echo "--- Running Integration Tests ---"
cargo test --test pubky_sdk_compliance -- --test-threads=1

# Doc tests
echo "--- Running Doc Tests ---"
cargo test --doc --workspace
```

### 4.3 Test Coverage Requirements

#### Paykit-Lib Tests

**Status: ✅ Complete**

- [x] Pubky SDK compliance tests (`tests/pubky_sdk_compliance.rs`)
- [x] Round-trip tests for all CRUD operations
- [x] Error handling for missing/invalid data
- [x] Contact discovery tests

**Verification:**

```bash
# Count test functions
echo "paykit-lib test count:"
grep -r "#\[test\]\|#\[tokio::test\]" --include="*.rs" \
  paykit-lib/tests/ paykit-lib/src/ | wc -l
```

#### Paykit-Subscriptions Tests

**Status: 🟡 Partial**

- [x] Signature creation and verification
- [x] Deterministic hashing
- [x] Expired signature rejection
- [x] Corrupted signature detection
- [x] Modified subscription detection
- [x] Different nonces produce different signatures
- [ ] **TODO:** Property-based tests (proptest) for Amount arithmetic
- [ ] **TODO:** Nonce store concurrency tests
- [ ] **TODO:** Atomic spending limit tests

**Verification:**

```bash
# Review existing tests
echo "paykit-subscriptions test count:"
grep -r "#\[test\]\|#\[tokio::test\]" --include="*.rs" \
  paykit-subscriptions/src/ paykit-subscriptions/tests/ | wc -l

# Check for property tests (should add these)
grep -r "proptest\|quickcheck" paykit-subscriptions/src/ \
  paykit-subscriptions/tests/
```

**Required Additional Tests:**

1. **Property-Based Tests for Amount**
   ```rust
   // TODO: Add to paykit-subscriptions/tests/property_tests.rs
   #[cfg(test)]
   mod property_tests {
       use proptest::prelude::*;
       use paykit_subscriptions::Amount;
       
       proptest! {
           #[test]
           fn amount_add_commutative(a in 0u64..1_000_000, b in 0u64..1_000_000) {
               let amt_a = Amount::from_sats(a);
               let amt_b = Amount::from_sats(b);
               prop_assert_eq!(
                   amt_a.checked_add(&amt_b),
                   amt_b.checked_add(&amt_a)
               );
           }
           
           #[test]
           fn amount_no_overflow(a in 0u64..u64::MAX/2, b in 0u64..u64::MAX/2) {
               let amt_a = Amount::from_sats(a);
               let amt_b = Amount::from_sats(b);
               prop_assert!(amt_a.checked_add(&amt_b).is_ok());
           }
       }
   }
   ```

2. **Nonce Store Concurrency Tests**
   ```rust
   // TODO: Add to paykit-subscriptions/tests/nonce_store_concurrent.rs
   #[tokio::test]
   async fn nonce_store_concurrent_inserts() {
       // Test parallel nonce insertions
       // Verify no nonce accepted twice
   }
   ```

3. **Atomic Spending Limit Tests**
   ```rust
   // TODO: Add to paykit-subscriptions/tests/spending_limits.rs
   #[tokio::test]
   async fn spending_limit_race_condition() {
       // Simulate concurrent payment attempts
       // Verify limit not exceeded
   }
   ```

#### Paykit-Interactive Tests

**Status: 🟡 Partial**

- [x] Noise handshake tests
- [x] Receipt serialization tests
- [x] Message encryption/decryption tests
- [ ] **TODO:** Integration tests with mock transport
- [ ] **TODO:** Timeout handling tests

**Verification:**

```bash
echo "paykit-interactive test count:"
grep -r "#\[test\]\|#\[tokio::test\]" --include="*.rs" \
  paykit-interactive/src/ paykit-interactive/tests/ | wc -l
```

### 4.4 Negative Test Cases

**Critical:** Security functions MUST have negative tests

```bash
# Check for negative test patterns
echo "=== Checking for Negative Tests ==="
grep -rn "test.*invalid\|test.*fail\|test.*reject\|test.*error" \
  --include="*.rs" paykit-subscriptions/src/ paykit-subscriptions/tests/ | \
  wc -l

echo "Should find multiple negative test cases"
```

**Required Negative Tests:**

- [x] Expired signatures rejected
- [x] Corrupted signatures rejected
- [x] Modified data fails verification
- [ ] Nonce reuse rejected
- [ ] Amount overflow rejected
- [ ] Invalid public keys rejected

### 4.5 Determinism Tests

**Critical for signatures:**

```bash
# Verify determinism tests exist
grep -rn "deterministic\|test.*hash.*same" --include="*.rs" \
  paykit-subscriptions/tests/ paykit-subscriptions/src/
```

**Example from `signing.rs`:**

```rust
#[test]
fn test_signature_hash_is_deterministic() {
    let sub1 = create_test_subscription();
    let sub2 = sub1.clone();
    let nonce = [42u8; 32];
    let timestamp = 1000i64;
    let expires_at = 2000i64;
    
    let hash1 = hash_subscription_canonical(&sub1, &nonce, timestamp, expires_at).unwrap();
    let hash2 = hash_subscription_canonical(&sub2, &nonce, timestamp, expires_at).unwrap();
    
    assert_eq!(hash1, hash2, "Hash must be deterministic");
}
```

### 4.6 Test Naming Conventions

**Follow pattern:** `test_<feature>_<case>()`

Examples:
- ✅ `test_sign_and_verify_ed25519()`
- ✅ `test_expired_signature_rejected()`
- ✅ `test_amount_overflow_detected()`
- ❌ `test1()` - not descriptive
- ❌ `test_feature()` - missing case

### 4.7 Stage 4 Sign-Off

- [ ] All tests pass
- [ ] Security-critical code has tests
- [ ] Negative test cases present
- [ ] Determinism tested for crypto
- [ ] Property tests planned/implemented (if TODO, ticket created)
- [ ] Concurrency tests planned/implemented (if TODO, ticket created)

**Test Results:**
- Total tests run: __________
- Tests passed: __________
- Tests failed: __________
- Coverage: __________%

**Auditor:** ________________  **Date:** __________

---

## Stage 5: Documentation & Commenting

### 5.1 Stop/Go Criteria

**🛑 STOP Criteria:**
- Security-critical functions lack documentation
- Cryptographic operations don't cite sources
- Public APIs missing doc comments
- Examples don't compile

**✅ GO Criteria:**
- All public APIs documented with `///`
- Security preconditions clearly stated
- Examples compile and run (doctests pass)
- Module-level docs explain purpose

### 5.2 Documentation Build

```bash
echo "=== Building Documentation ==="

# Build docs (must be warning-free for production)
cargo doc --workspace --no-deps --document-private-items 2>&1 | tee doc-build.log

# Check for warnings
warning_count=$(grep -c "warning:" doc-build.log || echo "0")
echo "Documentation warnings: $warning_count"

if [ "$warning_count" -gt 0 ]; then
  echo "⚠️  MEDIUM: Fix documentation warnings before release"
else
  echo "✅ PASS: Documentation builds cleanly"
fi

# Run doctests
echo "=== Running Doc Tests ==="
cargo test --doc --workspace
```

### 5.3 Public API Documentation

**All public items must have `///` docs:**

```bash
# Find public items without docs
echo "=== Checking Public API Documentation ==="

# This is a heuristic - manual review still required
grep -rn "^pub fn\|^pub struct\|^pub enum\|^pub trait" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
  head -20

echo "Manually verify each has /// doc comment"
```

**Documentation Standards:**

Each public function should have:

```rust
/// Brief one-line description.
///
/// # Purpose
/// Longer explanation of what this does and why.
///
/// # Security
/// Any security-relevant preconditions or guarantees.
///
/// # Arguments
/// * `param` - Description
///
/// # Returns
/// What is returned and under what conditions.
///
/// # Errors
/// What errors can occur and when.
///
/// # Examples
/// ```
/// use paykit_lib::*;
/// // Working example
/// ```
pub fn function_name() -> Result<T> { ... }
```

### 5.4 Security-Sensitive Documentation

**Critical areas requiring detailed docs:**

#### Nonce Management (paykit-subscriptions)

```bash
# Verify nonce documentation
grep -B 5 -A 10 "nonce" paykit-subscriptions/src/signing.rs | \
  grep -A 10 "///"
```

**Must document:**
- Nonces must be cryptographically random
- Nonces must never be reused
- Application must persist nonce store
- Replay protection requirements

#### Replay Protection

**Must document:**
- How expiration times work
- Timestamp validation requirements
- Clock skew considerations

#### Financial Operations

```bash
# Verify Amount documentation
grep -B 5 "pub struct Amount" paykit-subscriptions/src/amount.rs
```

**Must document:**
- Checked arithmetic always used
- Overflow handling
- Precision guarantees
- Serialization format

### 5.5 Cryptographic Citations

**All crypto code must cite sources:**

```bash
# Check for citations in crypto-heavy files
echo "=== Checking Crypto Citations ==="

for file in paykit-subscriptions/src/signing.rs \
            paykit-subscriptions/src/amount.rs; do
  echo "--- $file ---"
  grep -i "RFC\|NIST\|standard\|specification\|reference" "$file" || \
    echo "⚠️  Missing citations"
done
```

**Expected citations:**
- Ed25519: RFC 8032
- Postcard: Pubky SDK standard for deterministic serialization
- Noise Protocol: Noise Protocol Framework specification

### 5.6 Examples Compilation

**All code examples must compile:**

```bash
# Doctests verify this automatically
cargo test --doc --workspace

# Check for no_run examples (should be rare)
grep -rn "```.*no_run" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src
```

### 5.7 Module-Level Documentation

**Each module should have `//!` docs:**

```bash
# Check for module docs
echo "=== Module Documentation ==="
grep -rn "^//!" --include="*.rs" paykit-lib/src paykit-interactive/src \
  paykit-subscriptions/src | grep "lib.rs\|mod.rs"
```

### 5.8 SECURITY.md Document

**Should exist at workspace root or crate level:**

```bash
ls -la SECURITY.md */SECURITY.md 2>/dev/null || \
  echo "⚠️  Consider adding SECURITY.md with threat models"
```

**Should contain:**
- Threat models from Stage 1
- Known limitations
- Security reporting process
- Mitigations and residual risks

### 5.9 Stage 5 Sign-Off

- [ ] Documentation builds without warnings
- [ ] All doc tests pass
- [ ] Public APIs have `///` documentation
- [ ] Security-sensitive functions document preconditions
- [ ] Cryptographic operations cite sources
- [ ] Examples compile and run
- [ ] Module-level docs present
- [ ] Threat models documented

**Auditor:** ________________  **Date:** __________

---

## Stage 6: Build & CI Verification

### 6.1 Stop/Go Criteria

**🛑 STOP Criteria:**
- Build fails
- Clippy errors with `-D warnings`
- Format check fails
- Cargo audit shows HIGH/CRITICAL vulnerabilities

**✅ GO Criteria:**
- All builds succeed (debug + release)
- Zero clippy warnings
- Code properly formatted
- No known vulnerabilities

### 6.2 Clean Build Verification

```bash
#!/bin/bash
set -e

echo "======================================="
echo "=== Paykit Build Verification ==="
echo "======================================="
echo "Started at $(date)"
echo ""

# Stage 1: Clean build
echo "===== Stage 1: Clean Build ====="
echo "Cleaning workspace..."
cargo clean

echo "Building workspace (debug, all features)..."
cargo build --workspace --locked --all-targets --all-features

echo "Building workspace (release)..."
cargo build --workspace --release --locked

echo "✅ Stage 1 Complete"
echo ""

# Stage 2: Static analysis
echo "===== Stage 2: Static Analysis ====="
echo "Running clippy..."
if cargo clippy --workspace --all-targets --all-features -- -D warnings; then
  echo "✅ Clippy passed"
else
  echo "❌ Clippy warnings found - must fix"
  exit 1
fi

echo "Checking formatting..."
if cargo fmt --all -- --check; then
  echo "✅ Format check passed"
else
  echo "❌ Format drift detected"
  echo "Run: cargo fmt --all"
  exit 1
fi

echo "✅ Stage 2 Complete"
echo ""

# Stage 3: Security audit
echo "===== Stage 3: Security Audit ====="
echo "Running cargo audit..."
if cargo audit; then
  echo "✅ No known vulnerabilities"
else
  echo "⚠️  Vulnerabilities found or cargo-audit not installed"
  echo "Install with: cargo install cargo-audit"
fi

echo "✅ Stage 3 Complete"
echo ""

echo "======================================="
echo "=== Build Verification Complete ==="
echo "======================================="
```

Save as `build-verify.sh` and run:

```bash
chmod +x build-verify.sh
./build-verify.sh
```

### 6.3 Cargo.toml Configuration Review

**Workspace Root** (`Cargo.toml`):

```bash
# Verify resolver 2
grep 'resolver = "2"' Cargo.toml
```

**Per-Crate** (`paykit-*/Cargo.toml`):

```bash
# Verify edition 2021
for toml in paykit-lib/Cargo.toml paykit-interactive/Cargo.toml \
            paykit-subscriptions/Cargo.toml; do
  echo "--- $toml ---"
  grep 'edition = "2021"' "$toml"
done
```

**Feature Gates:**

```bash
# Verify paykit-lib has optional pubky feature
grep -A 5 "\[features\]" paykit-lib/Cargo.toml

# Check default features
grep "default = " paykit-lib/Cargo.toml
```

**Platform-Specific Dependencies:**

```bash
# Verify WASM vs native splits in paykit-subscriptions
grep -A 10 "target\.'cfg" paykit-subscriptions/Cargo.toml
```

### 6.4 Clippy Configuration

**Recommended `.clippy.toml` (optional):**

```toml
# Deny clippy warnings in CI
warn-on-all-wildcard-imports = true
```

**Run with:**

```bash
cargo clippy --workspace --all-targets --all-features -- \
  -D warnings \
  -D clippy::unwrap_used \
  -D clippy::expect_used \
  -D clippy::panic
```

### 6.5 Format Configuration

**`.rustfmt.toml` (if present):**

```bash
cat .rustfmt.toml
```

**Verify format:**

```bash
cargo fmt --all -- --check

# To fix formatting:
# cargo fmt --all
```

### 6.6 Platform-Specific Builds

**Native Build (x86_64-unknown-linux-gnu):**

```bash
# Should succeed
cargo build --package paykit-lib --target x86_64-unknown-linux-gnu
cargo build --package paykit-interactive --target x86_64-unknown-linux-gnu
cargo build --package paykit-subscriptions --target x86_64-unknown-linux-gnu
```

**WASM Build (future):**

```bash
# Currently excluded from workspace (see FINAL_SWEEP_REPORT.md)
# When enabled:
# cargo build --package paykit-lib --target wasm32-unknown-unknown
```

### 6.7 Dependency Audit

```bash
echo "=== Dependency Audit ==="

# Install cargo-audit if needed
# cargo install cargo-audit

# Run audit
cargo audit

# Check for outdated dependencies
cargo outdated || echo "cargo-outdated not installed"
```

**Response to Vulnerabilities:**

- **CRITICAL:** Update immediately, test, release patch
- **HIGH:** Update within 1 week
- **MEDIUM:** Update in next minor release
- **LOW:** Update when convenient

### 6.8 Stage 6 Sign-Off

- [ ] Clean build succeeds (debug + release)
- [ ] Clippy passes with `-D warnings`
- [ ] Code properly formatted
- [ ] Cargo audit shows no HIGH/CRITICAL issues
- [ ] Cargo.toml configurations correct
- [ ] Platform-specific builds work

**Build Info:**
- Rust version: __________
- Build time: __________
- Binary sizes: __________

**Auditor:** ________________  **Date:** __________

---

## Stage 7: Final Audit Report

### 7.1 Audit Report Template

Use this template to document findings:

```markdown
# Paykit Security Audit Report

## Executive Summary

- **Audit Date:** YYYY-MM-DD
- **Auditor:** [Name/Organization]
- **Scope:** paykit-lib v0.0.1, paykit-interactive v0.1.0, paykit-subscriptions v0.2.0
- **Commit Hash:** [git rev-parse HEAD]
- **Audit Duration:** X hours
- **Methodology:** 7-stage systematic audit per TESTING_AND_AUDIT_PLAN.md

## Overall Assessment

**Status:** [PASS / CONDITIONAL PASS / FAIL]

**Summary:** [Brief 2-3 sentence summary of findings]

## Threat Model Summary

| Threat Actor | Risk Level | Mitigations | Residual Risk |
|--------------|------------|-------------|---------------|
| Network Attacker | LOW | TLS, Noise Protocol | Minimal |
| Malicious Peer | MEDIUM | Validation, rate limiting (app) | DoS possible |
| Cryptanalytic | LOW | Modern algorithms, 256-bit security | Quantum threat >10y |
| Replay Attacks | LOW | Nonce + timestamp + expiration | Requires nonce persistence |
| Integer Overflow | LOW | Checked arithmetic in Amount | None |

## Critical Issues

### [Issue ID]: [Title]

- **Severity:** CRITICAL / HIGH / MEDIUM / LOW
- **Component:** paykit-[lib/interactive/subscriptions]
- **File:** path/to/file.rs:line
- **Description:** Detailed description of the issue
- **Impact:** What could go wrong
- **Recommendation:** How to fix
- **Status:** OPEN / FIXED / ACCEPTED

[Repeat for each issue]

## Stage Results

### Stage 1: Threat Model & Architecture
- **Status:** ✅ PASS / ❌ FAIL / ⚠️ CONDITIONAL
- **Findings:** [Summary]
- **Issues:** [List issue IDs]

### Stage 2: Cryptography Audit
- **Status:** ✅ PASS / ❌ FAIL / ⚠️ CONDITIONAL
- **Findings:** [Summary]
- **Issues:** [List issue IDs]

### Stage 3: Rust Safety & Correctness
- **Status:** ✅ PASS / ❌ FAIL / ⚠️ CONDITIONAL
- **Findings:** [Summary]
- **Issues:** [List issue IDs]

### Stage 4: Testing Requirements
- **Status:** ✅ PASS / ❌ FAIL / ⚠️ CONDITIONAL
- **Test Count:** X tests, Y passed, Z failed
- **Coverage:** X%
- **Issues:** [List issue IDs]

### Stage 5: Documentation & Commenting
- **Status:** ✅ PASS / ❌ FAIL / ⚠️ CONDITIONAL
- **Findings:** [Summary]
- **Issues:** [List issue IDs]

### Stage 6: Build & CI Verification
- **Status:** ✅ PASS / ❌ FAIL / ⚠️ CONDITIONAL
- **Build Time:** X seconds
- **Clippy Warnings:** 0
- **Issues:** [List issue IDs]

## Verification Checklist

- [ ] All 7 stages completed
- [ ] All tests pass
- [ ] Zero unsafe blocks in audited code
- [ ] Zero clippy warnings
- [ ] Crypto primitives reviewed and approved
- [ ] Documentation complete and accurate
- [ ] No CRITICAL issues remain unresolved
- [ ] No HIGH issues remain unresolved or unaccepted

## Recommendations

### Immediate (Before Release)
1. [Action item]
2. [Action item]

### Short-Term (Next Sprint)
1. [Action item]
2. [Action item]

### Long-Term (Roadmap)
1. [Action item]
2. [Action item]

## Acceptance Criteria Met

- [ ] All CRITICAL issues resolved
- [ ] All HIGH issues resolved or explicitly accepted with mitigation
- [ ] Test coverage ≥80% for critical modules
- [ ] Documentation complete for public APIs
- [ ] Build succeeds on all target platforms

## Sign-Off

**Auditor:** ________________  
**Date:** __________  
**Status:** PASS / CONDITIONAL PASS / FAIL

**Notes:** [Any additional context]

---

**Attachment:** Command outputs from audit (logs, test results, etc.)
```

### 7.2 Issue Tracking Template

Create `AUDIT_ISSUES.md` to track findings:

```markdown
# Audit Issues Tracking

## Critical Issues (MUST FIX)

### ISSUE-001: [Title]
- **Severity:** CRITICAL
- **Found:** YYYY-MM-DD
- **Component:** paykit-subscriptions
- **File:** src/signing.rs:42
- **Description:** [Details]
- **Status:** OPEN
- **Assigned:** [Name]
- **Target Date:** YYYY-MM-DD

[Repeat for each critical issue]

## High Issues (SHOULD FIX)

[Same format]

## Medium Issues (DOCUMENT & PLAN)

[Same format]

## Low Issues (NICE TO HAVE)

[Same format]
```

### 7.3 Stage 7 Sign-Off

- [ ] Audit report completed using template
- [ ] All issues documented and tracked
- [ ] Recommendations provided
- [ ] Final status determined (PASS/CONDITIONAL/FAIL)
- [ ] Report delivered to stakeholders

**Auditor:** ________________  **Date:** __________

---

## Code Completeness Checks

### Quick Commands

Run from repository root:

```bash
#!/bin/bash
echo "======================================="
echo "=== Code Completeness Checks ==="
echo "======================================="

# TODO/FIXME detection in production code
echo "=== TODOs/FIXMEs in Production Code ==="
todo_count=$(grep -r "TODO\|FIXME\|PLACEHOLDER\|todo!\|unimplemented!" \
  --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
  wc -l)
echo "Count: $todo_count"
if [ "$todo_count" -gt 0 ]; then
  echo "⚠️  Found incomplete code:"
  grep -rn "TODO\|FIXME\|PLACEHOLDER\|todo!\|unimplemented!" \
    --include="*.rs" \
    paykit-lib/src paykit-interactive/src paykit-subscriptions/src
fi
echo ""

# Ignored tests detection
echo "=== Ignored Tests ==="
ignored_count=$(grep -r "#\[ignore\]" --include="*.rs" \
  paykit-lib/ paykit-interactive/ paykit-subscriptions/ | wc -l)
echo "Count: $ignored_count"
if [ "$ignored_count" -gt 0 ]; then
  echo "⚠️  Found ignored tests:"
  grep -rn "#\[ignore\]" --include="*.rs" \
    paykit-lib/ paykit-interactive/ paykit-subscriptions/
fi
echo ""

# Unwrap/expect in production code
echo "=== Unwrap/Expect/Panic in Production ==="
panic_count=$(grep -r "\.unwrap()\|\.expect(\|panic!" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
  grep -v "test" | wc -l)
echo "Count: $panic_count"
if [ "$panic_count" -gt 0 ]; then
  echo "⚠️  Found panic-prone code:"
  grep -rn "\.unwrap()\|\.expect(\|panic!" --include="*.rs" \
    paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
    grep -v "test"
fi
echo ""

# Debug/println! in production code
echo "=== Debug Print Statements ==="
debug_count=$(grep -r "println!\|dbg!\|eprintln!" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
  grep -v "test" | wc -l)
echo "Count: $debug_count"
if [ "$debug_count" -gt 0 ]; then
  echo "⚠️  Found debug prints:"
  grep -rn "println!\|dbg!\|eprintln!" --include="*.rs" \
    paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
    grep -v "test"
fi
echo ""

echo "======================================="
echo "Summary:"
echo "  TODOs/FIXMEs: $todo_count"
echo "  Ignored Tests: $ignored_count"
echo "  Panic-prone: $panic_count"
echo "  Debug prints: $debug_count"
echo "======================================="
```

Save as `check-completeness.sh` and run:

```bash
chmod +x check-completeness.sh
./check-completeness.sh
```

---

## Stack-Specific Considerations

### Pubky Homeserver Integration

**Checklist:**

- [ ] **Path Constants Used**
  ```bash
  # Verify constants are used, not hardcoded strings
  grep -rn "PAYKIT_PATH_PREFIX\|PUBKY_FOLLOWS_PATH" paykit-lib/src/
  
  # Should NOT find hardcoded paths
  grep -rn '"/pub/paykit\|/pub/pubky"' paykit-lib/src/ | grep -v "const"
  ```

- [ ] **404 Handling**
  ```bash
  # Verify 404s treated as empty (not errors)
  grep -rn "404\|NotFound" paykit-lib/src/transport/
  ```

- [ ] **Authenticated vs Unauthenticated**
  ```bash
  # Verify proper trait usage
  grep -rn "AuthenticatedTransport\|UnauthenticatedTransportRead" \
    paykit-lib/src/
  ```

- [ ] **Session Management Documented**
  ```bash
  # Check that session creation is documented as caller responsibility
  grep -rn "session.*responsibility\|caller.*session" paykit-lib/src/
  ```

### Noise Protocol Integration

**Checklist:**

- [ ] **IK Pattern Usage**
  ```bash
  # Verify handshake pattern
  grep -rn "client_start_ik\|server_accept_ik\|Noise_IK" \
    paykit-interactive/src/ paykit-interactive/tests/
  ```

- [ ] **Identity Payload Exchange**
  ```bash
  grep -rn "identity.*payload\|IdentityPayload" paykit-interactive/src/
  ```

- [ ] **Key Provider Implementation**
  ```bash
  grep -rn "RingKeyProvider\|KeyProvider" paykit-interactive/
  ```

- [ ] **Message Encryption/Decryption**
  ```bash
  grep -rn "encrypt\|decrypt\|NoiseLink" paykit-interactive/src/
  ```

- [ ] **Zero Shared Secret Detection**
  ```bash
  # Should be handled by pubky-noise, verify in tests
  grep -rn "all.*zero\|zero.*secret" paykit-interactive/tests/
  ```

### Financial Operations

**Checklist:**

- [ ] **Amount Type Always Used**
  ```bash
  # Should NOT find raw u64/i64 for amounts in subscription code
  grep -rn "amount.*:.*u64\|amount.*:.*i64" \
    paykit-subscriptions/src/subscription.rs \
    paykit-subscriptions/src/request.rs | \
    grep -v "Amount"
  ```

- [ ] **No Floating Point for Money**
  ```bash
  # Should return 0
  grep -r "f32\|f64\|float" --include="*.rs" \
    paykit-subscriptions/src/ | \
    grep -v "test" | \
    grep -v "comment" | \
    wc -l
  ```

- [ ] **Serialization Preserves Precision**
  ```bash
  # Verify Amount serialization uses rust_decimal
  grep -rn "Serialize\|Deserialize" paykit-subscriptions/src/amount.rs
  ```

- [ ] **Overflow/Underflow Properly Handled**
  ```bash
  # All arithmetic should use checked_* methods
  grep -rn "checked_add\|checked_sub\|checked_mul" \
    paykit-subscriptions/src/amount.rs
  ```

- [ ] **Currency Units Documented**
  ```bash
  # Verify Amount docs explain satoshi vs BTC vs other units
  grep -B 5 -A 10 "pub struct Amount" paykit-subscriptions/src/amount.rs
  ```

### Concurrency & Async

**Checklist:**

- [ ] **Arc for Shared State**
  ```bash
  # Verify Arc used (not Rc)
  grep -rn "Arc<" paykit-subscriptions/src/
  grep -rn "Rc<" paykit-subscriptions/src/ | grep -v "test"
  ```

- [ ] **Async Trait Correctness**
  ```bash
  # Verify #[async_trait] used
  grep -rn "async_trait\|#\[async_trait\]" \
    paykit-lib/src paykit-interactive/src paykit-subscriptions/src
  ```

- [ ] **No Blocking in Async**
  ```bash
  # Check for blocking operations
  grep -rn "std::thread::sleep\|blocking" \
    paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
    grep -v "test"
  ```

- [ ] **Error Propagation Across Await**
  ```bash
  # Verify Result types and ? operator usage
  grep -rn "\.await?" paykit-lib/src paykit-interactive/src \
    paykit-subscriptions/src | head -10
  ```

- [ ] **Timeout Handling**
  ```bash
  # Verify timeout feature and implementation
  grep -rn "timeout\|Timeout" paykit-interactive/src/ paykit-interactive/Cargo.toml
  ```

---

## Appendix: Automated Audit Script

### Full Audit Automation

Create `audit-paykit.sh` based on existing `audit-full.sh`:

```bash
#!/bin/bash
set -e

echo "======================================="
echo "=== Paykit Production Crates Audit ==="
echo "======================================="
echo "Started at $(date)"
echo ""
echo "Scope: paykit-lib, paykit-interactive, paykit-subscriptions"
echo "Excluded: paykit-demo-*"
echo ""

# Stage 1: Build verification
echo "===== Stage 1: Build Verification ====="
echo "Clean build..."
cargo clean

echo "Building workspace (debug)..."
cargo build --workspace --locked --all-targets --all-features

echo "Building workspace (release)..."
cargo build --workspace --release --locked

echo "✅ Stage 1 Complete"
echo ""

# Stage 2: Static analysis
echo "===== Stage 2: Static Analysis ====="
echo "Running clippy..."
cargo clippy --workspace --all-targets --all-features -- -D warnings || \
  echo "⚠️  Clippy warnings found"

echo "Checking formatting..."
cargo fmt --all -- --check || echo "⚠️  Format drift detected"

echo "✅ Stage 2 Complete"
echo ""

# Stage 3: Testing
echo "===== Stage 3: Running Test Suite ====="
echo "Running all tests..."
cargo test --workspace --all-features -- --nocapture || \
  echo "⚠️  Some tests failed"

echo "Running integration tests..."
cargo test --test pubky_sdk_compliance -- --test-threads=1 || \
  echo "⚠️  Integration tests failed"

echo "✅ Stage 3 Complete"
echo ""

# Stage 4: Documentation
echo "===== Stage 4: Documentation Build ====="
echo "Building documentation..."
cargo doc --workspace --no-deps --document-private-items

echo "Running doctests..."
cargo test --doc --workspace || echo "⚠️  Some doctests failed"

echo "✅ Stage 4 Complete"
echo ""

# Stage 5: Security audit
echo "===== Stage 5: Security Audit ====="
echo "Running cargo audit..."
cargo audit || echo "⚠️  Vulnerabilities found or cargo-audit not installed"

echo "✅ Stage 5 Complete"
echo ""

# Stage 6: Code completeness
echo "===== Stage 6: Code Completeness Check ====="
echo ""

echo "Unsafe blocks in production code:"
unsafe_count=$(grep -r "unsafe" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
  wc -l || echo "0")
echo "  Count: $unsafe_count (should be 0)"

echo ""
echo "Unwraps/panics in production code:"
panic_count=$(grep -r "\.unwrap()\|\.expect(\|panic!" --include="*.rs" \
  paykit-lib/src paykit-interactive/src paykit-subscriptions/src | \
  grep -v "test" | wc -l || echo "0")
echo "  Count: $panic_count (should be 0)"

echo ""
echo "TODOs/FIXMEs in source:"
todo_count=$(grep -r "TODO\|FIXME\|PLACEHOLDER\|todo!\|unimplemented!" \
  --include="*.rs" paykit-lib/src paykit-interactive/src \
  paykit-subscriptions/src | wc -l || echo "0")
echo "  Count: $todo_count (should be 0)"

echo ""
echo "Ignored tests:"
ignored_count=$(grep -r "#\[ignore\]" --include="*.rs" \
  paykit-lib/ paykit-interactive/ paykit-subscriptions/ | \
  wc -l || echo "0")
echo "  Count: $ignored_count (should be 0)"

echo ""
echo "Banned crypto primitives (md5, sha1, rc4, des):"
banned_count=$(grep -ri "\\bmd5\\b\|\\bsha1\\b\|\\brc4\\b\|\\bdes[^c]" \
  --include="*.rs" --exclude-dir=target \
  paykit-lib/ paykit-interactive/ paykit-subscriptions/ | \
  wc -l || echo "0")
echo "  Count: $banned_count (should be 0)"

echo ""
echo "✅ Stage 6 Complete"
echo ""

# Stage 7: Crypto-specific checks
echo "===== Stage 7: Cryptography Checks ====="

echo "Checking nonce randomness..."
grep -rn "rand::thread_rng\|OsRng" paykit-subscriptions/src/signing.rs || \
  echo "⚠️  Verify nonce generation is cryptographically random"

echo "Checking deterministic serialization..."
grep -rn "postcard" paykit-subscriptions/src/signing.rs || \
  echo "⚠️  Verify deterministic serialization"

echo "Checking Amount checked arithmetic..."
grep -rn "checked_add\|checked_sub\|checked_mul" \
  paykit-subscriptions/src/amount.rs || \
  echo "⚠️  Verify Amount uses checked arithmetic"

echo "✅ Stage 7 Complete"
echo ""

echo "======================================="
echo "=== Audit Complete at $(date) ==="
echo "======================================="
echo ""
echo "Summary of Issues:"
echo "  Unsafe blocks: $unsafe_count"
echo "  Unwraps/panics: $panic_count"
echo "  TODOs/FIXMEs: $todo_count"
echo "  Ignored tests: $ignored_count"
echo "  Banned crypto: $banned_count"
echo ""
echo "Next Steps:"
echo "1. Review any non-zero counts above"
echo "2. Complete manual review per TESTING_AND_AUDIT_PLAN.md"
echo "3. Fill out audit report template (Stage 7)"
echo "4. Sign off on each stage"
echo ""
```

Save as `/Users/johncarvalho/Library/Mobile Documents/com~apple~CloudDocs/vibes/paykit-rs-master/audit-paykit.sh`

---

## Quick Reference Card

### Pre-Release Checklist

```
□ Run: ./audit-paykit.sh
□ Run: ./check-completeness.sh  
□ All tests pass
□ Zero clippy warnings
□ Zero unsafe blocks (or justified)
□ Zero unwrap/panic in production
□ Documentation complete
□ Cargo audit clean
□ All stages signed off
```

### Critical Commands

```bash
# Quick audit (5 min)
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo audit

# Deep audit (1 hour)
./audit-paykit.sh

# Completeness check
./check-completeness.sh
```

### Issue Severity Quick Reference

- 🔴 **CRITICAL**: Stop immediately (crypto bugs, memory safety, overflow)
- 🟠 **HIGH**: Stop and fix (missing tests, unwraps, validation)
- 🟡 **MEDIUM**: Document and continue (clippy, minor gaps)
- 🟢 **LOW**: Document only (style, nice-to-haves)

---

**End of Document**

**Version:** 1.0  
**Maintained by:** Paykit Core Team  
**Last Audit:** [To be filled]  
**Next Audit Due:** [To be filled]

