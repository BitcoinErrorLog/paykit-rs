# Audit Issues Tracking

**Generated:** November 21, 2025  
**Audit Report:** See `PAYKIT_SECURITY_AUDIT_REPORT.md`

---

## Critical Issues (MUST FIX) 🔴

### ✅ NONE

Excellent! Zero critical security vulnerabilities found.

---

## High Issues (SHOULD FIX) 🟠

### ✅ NONE

No high severity issues requiring immediate attention.

---

## Medium Issues (DOCUMENT & PLAN) 🟡

### ISSUE-M001: Incomplete Implementation in SubscriptionManager

- **Severity:** MEDIUM
- **Found:** 2025-11-21
- **Component:** paykit-subscriptions
- **File:** `paykit-subscriptions/src/manager.rs:128`
- **Description:** TODO comment indicates incomplete Pubky directory listing functionality
  ```rust
  // TODO: Implement full Pubky directory listing and fetching
  ```
- **Impact:** May prevent discovery of all subscriptions for a given peer
- **Recommendation:** Complete implementation or document limitation in public API
- **Status:** OPEN
- **Assigned:** TBD
- **Target Date:** Before v1.0 release
- **Priority:** P2 (Document workaround acceptable for MVP)

---

### ISSUE-M002: Unwrap Usage Documentation

- **Severity:** MEDIUM (Code Quality)
- **Found:** 2025-11-21
- **Component:** paykit-subscriptions  
- **File:** `paykit-subscriptions/src/nonce_store.rs:117,126`
- **Description:** Two `.expect()` calls for Mutex poisoning lack safety documentation
  ```rust
  .expect("NonceStore lock poisoned");
  ```
- **Impact:** Low - Mutex poisoning is unrecoverable, expect() is appropriate
- **Recommendation:** Add safety comment:
  ```rust
  // SAFETY: Mutex poisoning is unrecoverable state, panic is appropriate
  .expect("NonceStore lock poisoned - unrecoverable error");
  ```
- **Status:** OPEN
- **Assigned:** TBD
- **Target Date:** Next sprint
- **Priority:** P3 (Documentation improvement)

---

### ISSUE-M003: Format Drift

- **Severity:** MEDIUM (Code Quality)
- **Found:** 2025-11-21
- **Component:** Workspace-wide
- **Description:** `cargo fmt --check` failed
- **Impact:** Code consistency
- **Recommendation:** Run `cargo fmt --all`
- **Status:** OPEN
- **Assigned:** TBD
- **Target Date:** Before release
- **Priority:** P1 (Trivial fix, must do before merge)

---

## Low Issues (NICE TO HAVE) 🟢

### ISSUE-L001: Demo Test Failures

- **Severity:** LOW (Demo Only)
- **Found:** 2025-11-21
- **Component:** paykit-demo-cli
- **File:** `paykit-demo-cli/tests/e2e_payment_flow.rs`
- **Description:** 2 test failures:
  - `test_noise_handshake_between_payer_and_receiver`
  - `test_multiple_concurrent_payment_requests`
- **Impact:** None on production libraries
- **Recommendation:** Fix or mark as `#[ignore]` if environment-dependent
- **Status:** DOCUMENTED
- **Assigned:** TBD
- **Target Date:** Backlog
- **Priority:** P4 (Demo code outside audit scope)

---

### ISSUE-L002: Integration Test Environment Issue

- **Severity:** LOW
- **Found:** 2025-11-21
- **Component:** paykit-lib
- **File:** `paykit-lib/tests/pubky_sdk_compliance.rs:335`
- **Description:** `test_unauthenticated_transport_404_handling` HTTP transport error
- **Impact:** May be environment-specific
- **Recommendation:** Investigate test environment; consider mock server
- **Status:** OPEN
- **Assigned:** TBD
- **Target Date:** Next sprint
- **Priority:** P3

---

### ISSUE-L003: Deprecated Function Usage

- **Severity:** LOW
- **Found:** 2025-11-21
- **Component:** paykit-interactive, paykit-demo-core
- **Description:** Using deprecated `pubky_noise::datalink_adapter::server_accept_ik`
- **Impact:** May break in future pubky-noise versions
- **Recommendation:** Migrate to 3-step handshake functions
- **Status:** OPEN
- **Assigned:** TBD
- **Target Date:** Before pubky-noise v1.0
- **Priority:** P3

---

### ISSUE-L004: Unused Variables in Tests

- **Severity:** LOW (Code Quality)
- **Found:** 2025-11-21
- **Component:** Various test files
- **Description:** Compiler warnings for unused variables
- **Impact:** None - test code only
- **Recommendation:** Prefix with underscore or use `#[allow(unused_variables)]`
- **Status:** DOCUMENTED
- **Assigned:** TBD
- **Target Date:** Backlog
- **Priority:** P4

---

### ISSUE-L005: Doc Link Warning

- **Severity:** LOW (Documentation)
- **Found:** 2025-11-21
- **Component:** paykit-demo-cli
- **File:** `paykit-demo-cli/src/main.rs:182`
- **Description:** Broken intra-doc link to `:DAY`
- **Impact:** Documentation quality
- **Recommendation:** Escape brackets: `monthly\[:DAY\]`
- **Status:** OPEN
- **Assigned:** TBD
- **Target Date:** Backlog
- **Priority:** P4 (Demo code)

---

## Summary

| Severity | Count | Open | Closed |
|----------|-------|------|--------|
| 🔴 CRITICAL | 0 | 0 | 0 |
| 🟠 HIGH | 0 | 0 | 0 |
| 🟡 MEDIUM | 3 | 3 | 0 |
| 🟢 LOW | 5 | 4 | 0 |
| **TOTAL** | **8** | **7** | **0** |

---

## Action Items for Release

### Must Do (Blocking Release)

1. ✅ None - No blocking issues

### Should Do (Before Release)

1. [ ] **ISSUE-M003:** Run `cargo fmt --all`
2. [ ] **ISSUE-M001:** Document TODO limitation OR complete implementation
3. [ ] **ISSUE-M002:** Add safety comment to Mutex expect calls

### Nice to Have (Next Sprint)

4. [ ] **ISSUE-L002:** Fix integration test environment issue
5. [ ] **ISSUE-L003:** Migrate from deprecated pubky-noise functions
6. [ ] Add property-based tests (proptest)
7. [ ] Add nonce store concurrency tests
8. [ ] Install and run cargo-audit
9. [ ] Add clippy deny rules for unwrap/expect

---

## Status Updates

**Last Updated:** 2025-11-21  
**Next Review:** Before v1.0 release OR 2026-05-21

