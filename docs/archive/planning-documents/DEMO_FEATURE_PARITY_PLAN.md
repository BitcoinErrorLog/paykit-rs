# Paykit Demo Apps: Feature Parity Analysis & Implementation Plan

> **Comprehensive review of CLI, Web, and Mobile demo applications**
> Generated: December 2024

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Feature Comparison Matrix](#feature-comparison-matrix)
3. [README Accuracy Audit](#readme-accuracy-audit)
4. [Mock vs Real Implementation Analysis](#mock-vs-real-implementation-analysis)
5. [Library Features Not Exposed in Demos](#library-features-not-exposed-in-demos)
6. [Gap Analysis by Demo App](#gap-analysis-by-demo-app)
7. [Implementation Plan](#implementation-plan)
8. [Priority Recommendations](#priority-recommendations)

---

## Executive Summary

### Current State Overview

| Demo App | Overall Completeness | Production Readiness | README Accuracy |
|----------|---------------------|---------------------|-----------------|
| **CLI** | 85% | Demo-ready | ✅ Accurate |
| **Web** | 80% | Demo-ready | ✅ Accurate |
| **iOS** | 45% | Partial demo | ⚠️ Minor updates needed |
| **Android** | 40% | Partial demo | ⚠️ Minor updates needed |

### Key Findings

1. **Payment Execution Gap**: All demos simulate payment execution. The protocol coordination works, but actual wallet integration (LND, Bitcoin Core) is not connected.

2. **Mobile Demos Lag Behind**: iOS/Android demos have real key management but most features are UI-only with sample data.

3. **Library Features Underutilized**: `paykit-lib` has real LND/Esplora executors, health monitoring, and payment selection that aren't wired to any demos.

4. **Directory Publishing Mock**: Web demo saves methods to localStorage only; CLI and Mobile don't have real homeserver publishing tested.

---

## Feature Comparison Matrix

### Identity & Key Management

| Feature | CLI | Web | iOS | Android |
|---------|-----|-----|-----|---------|
| Ed25519 keypair generation | ✅ Real | ✅ Real | ✅ Real (FFI) | ✅ Real (FFI) |
| X25519 key derivation | ✅ Real | ✅ Real | ✅ Real (FFI) | ✅ Real (FFI) |
| Key persistence | ✅ File JSON | ✅ localStorage | ✅ Keychain | ✅ EncryptedPrefs |
| Key backup/export | ❌ Missing | ❌ Missing | ✅ Argon2+AES | ✅ Argon2+AES |
| Key import/restore | ❌ Missing | ❌ Missing | ✅ Real | ✅ Real |
| Multiple identities | ✅ Real | ✅ Real | ❌ Single only | ❌ Single only |
| Identity switching | ✅ Real | ❌ Missing | ❌ N/A | ❌ N/A |

### Contact Management

| Feature | CLI | Web | iOS | Android |
|---------|-----|-----|-----|---------|
| Add contacts | ✅ Real | ✅ Real | ✅ Real | ✅ Real |
| List contacts | ✅ Real | ✅ Real | ✅ Real | ✅ Real |
| Remove contacts | ✅ Real | ✅ Real | ✅ Real | ✅ Real |
| Contact search | ❌ Missing | ✅ Real | ✅ Real | ✅ Real |
| Contact notes | ✅ Real | ✅ Real | ✅ Real | ✅ Real |
| Import from follows | ❌ Missing | ⚠️ Partial | ❌ Missing | ❌ Missing |
| Contact payment history | ❌ Missing | ✅ Real | ✅ Real | ✅ Real |

### Directory Operations

| Feature | CLI | Web | iOS | Android |
|---------|-----|-----|-----|---------|
| Discover payment methods | ✅ Real (HTTP) | ✅ Real (HTTP) | ❌ Not impl. | ❌ Not impl. |
| Publish payment endpoint | ⚠️ Untested | ❌ Mock only | ❌ Not impl. | ❌ Not impl. |
| Remove payment endpoint | ❌ Missing | ❌ Mock only | ❌ Not impl. | ❌ Not impl. |
| Fetch known contacts (follows) | ⚠️ Partial | ⚠️ Partial | ❌ Not impl. | ❌ Not impl. |

### Payment Methods

| Feature | CLI | Web | iOS | Android |
|---------|-----|-----|-----|---------|
| Configure methods | ⚠️ CLI args only | ✅ Real (localStorage) | ❌ Static list | ❌ Static list |
| Priority ordering | ❌ Missing | ✅ Real | ❌ UI only | ❌ UI only |
| Public/private visibility | ⚠️ Partial | ✅ Real | ❌ UI only | ❌ UI only |
| Preferred method selection | ❌ Missing | ✅ Real | ❌ UI only | ❌ UI only |
| Method validation | ❌ Missing | ❌ Missing | ❌ Missing | ❌ Missing |
| Health monitoring | ❌ Missing | ❌ Missing | ❌ UI mock | ❌ UI mock |

### Interactive Payments (Noise Protocol)

| Feature | CLI | Web | iOS | Android |
|---------|-----|-----|-----|---------|
| Noise handshake (IK pattern) | ✅ Real (TCP) | ✅ Real (WebSocket) | ❌ Not impl. | ❌ Not impl. |
| Private endpoint exchange | ✅ Real | ✅ Real | ❌ Not impl. | ❌ Not impl. |
| Receipt request/confirm | ✅ Real | ✅ Real | ❌ Not impl. | ❌ Not impl. |
| Receipt storage | ✅ Real (file) | ✅ Real (localStorage) | ✅ Real (Keychain) | ✅ Real (EncryptedPrefs) |
| **Payment execution** | ❌ Simulation | ❌ Simulation | ❌ Not impl. | ❌ Not impl. |
| Payment receiver mode | ✅ Real (TCP) | ⚠️ Needs relay | ❌ Not impl. | ❌ Not impl. |

### Subscription Management

| Feature | CLI | Web | iOS | Android |
|---------|-----|-----|-----|---------|
| Create payment request | ✅ Real | ✅ Real | ❌ UI mock | ❌ UI mock |
| List payment requests | ✅ Real | ✅ Real | ❌ UI mock | ❌ UI mock |
| Respond to requests | ✅ Real | ✅ Real | ❌ UI mock | ❌ UI mock |
| Propose subscription | ✅ Real | ✅ Real | ❌ UI mock | ❌ UI mock |
| Accept subscription | ✅ Real | ✅ Real | ❌ UI mock | ❌ UI mock |
| Subscription persistence | ✅ File | ✅ localStorage | ❌ None | ❌ None |
| Proration calculation | ❌ Missing | ❌ Missing | ✅ Real (FFI) | ✅ Real (FFI) |

### Auto-Pay & Spending Limits

| Feature | CLI | Web | iOS | Android |
|---------|-----|-----|-----|---------|
| Enable/disable auto-pay | ✅ Real | ✅ Real | ❌ UI mock | ❌ UI mock |
| Max amount per payment | ✅ Real | ✅ Real | ❌ UI mock | ❌ UI mock |
| Require confirmation | ✅ Real | ✅ Real | ❌ UI mock | ❌ UI mock |
| Per-peer spending limits | ✅ Real | ✅ Real | ❌ UI mock | ❌ UI mock |
| Global daily limit | ✅ Real | ✅ Real | ❌ UI mock | ❌ UI mock |
| Limit persistence | ✅ File | ✅ localStorage | ❌ None | ❌ None |
| Period reset tracking | ✅ Real | ✅ Real | ❌ UI mock | ❌ UI mock |
| Recent auto-payments | ✅ Real | ✅ Real | ❌ UI mock | ❌ UI mock |

### Dashboard & UI

| Feature | CLI | Web | iOS | Android |
|---------|-----|-----|-----|---------|
| Statistics overview | ❌ N/A (CLI) | ✅ Real | ✅ Real | ✅ Real |
| Recent activity | ❌ N/A (CLI) | ✅ Real | ✅ Real | ✅ Real |
| Quick actions | ❌ N/A (CLI) | ✅ Real | ✅ Real | ✅ Real |
| Setup checklist | ❌ N/A (CLI) | ✅ Real | ❌ Missing | ❌ Missing |

### QR Code Support

| Feature | CLI | Web | iOS | Android |
|---------|-----|-----|-----|---------|
| Display QR for pubkey | ❌ Missing | ❌ Missing | ❌ Missing | ❌ Missing |
| Scan QR codes | ❌ Missing | ❌ Missing | ❌ Missing | ❌ Missing |
| Parse Paykit URIs | ❌ Missing | ❌ Missing | ✅ Real (FFI) | ✅ Real (FFI) |

---

## README Accuracy Audit

### CLI README (`paykit-demo-cli/README.md`)

| Section | Status | Notes |
|---------|--------|-------|
| Feature status table | ✅ Accurate | Correctly shows simulation mode |
| Commands reference | ✅ Accurate | All commands documented |
| Architecture diagram | ✅ Accurate | Shows correct dependencies |
| Testing info | ⚠️ Needs update | Test counts may be outdated |
| Storage structure | ✅ Accurate | Matches implementation |

**Recommended Updates:**
- Verify test count is current (claims 25 tests)
- Add key backup/import to roadmap

### Web README (`paykit-demo-web/README.md`)

| Section | Status | Notes |
|---------|--------|-------|
| Feature status table | ✅ Accurate | Mock publish correctly noted |
| API reference | ✅ Accurate | Comprehensive documentation |
| Architecture | ✅ Accurate | WebSocket transport explained |
| Test counts | ⚠️ Verify | Claims ~103 tests |
| Deployment info | ✅ Accurate | Multiple platforms covered |

**Recommended Updates:**
- Add identity switching to roadmap
- Note that payment execution is coordination only

### iOS README (`paykit-mobile/ios-demo/README.md`)

| Section | Status | Notes |
|---------|--------|-------|
| Feature status table | ✅ Accurate | Clear real vs UI-only split |
| Project structure | ⚠️ Outdated | May not reflect all files |
| Setup instructions | ✅ Accurate | Build steps correct |
| KeyManager usage | ✅ Accurate | Code examples work |
| Auto-pay flow diagram | ⚠️ Misleading | Shows full flow but it's mock |

**Recommended Updates:**
- Update project structure to include new files
- Clarify auto-pay flow diagram is aspirational
- Add more features to roadmap

### Android README (`paykit-mobile/android-demo/README.md`)

| Section | Status | Notes |
|---------|--------|-------|
| Feature status table | ✅ Accurate | Clear real vs UI-only split |
| Project structure | ✅ Accurate | Matches current code |
| Setup instructions | ✅ Accurate | Build steps work |
| Auto-pay flow diagram | ⚠️ Misleading | Shows full flow but it's mock |

**Recommended Updates:**
- Clarify auto-pay flow diagram is aspirational
- Add subscription persistence to roadmap

---

## Mock vs Real Implementation Analysis

### Components That Are Mock/Simulation

#### 1. Payment Execution (All Demos)
**Current:** Coordination messages exchange successfully, but no actual Bitcoin/Lightning payment occurs.
**Should Be Real:** Yes - this is the core value proposition.
**Blocker:** `paykit-lib/executors/lnd.rs` has stub implementation - needs `reqwest` dependency for HTTP calls.

```rust
// Current (stub):
Err(PaykitError::Unimplemented(
    "LND HTTP client not compiled - add reqwest dependency",
))
```

**Fix:** Add `reqwest` feature flag and implement HTTP client.

#### 2. Directory Publishing (Web)
**Current:** `mock_publish()` saves to localStorage only.
**Should Be Real:** Yes - required for P2P discovery.
**Blocker:** CORS restrictions require proxy or homeserver with CORS headers.

**Fix:** 
- Option A: Deploy CORS-enabled homeserver for demos
- Option B: Add proxy endpoint to demo server
- Option C: Document as limitation for browser demos

#### 3. Payment Methods UI (iOS/Android)
**Current:** Static list of 2 hardcoded methods.
**Should Be Real:** Yes - uses existing FFI bindings.
**Blocker:** None - just needs wiring.

**Fix:** Connect UI to `PaykitClient.list_methods()` and `PaykitClient.validate_endpoint()`.

#### 4. Subscriptions & Auto-Pay (iOS/Android)
**Current:** Sample data, not persisted.
**Should Be Real:** Yes - FFI bindings exist for subscription creation.
**Blocker:** None - storage and FFI calls need wiring.

**Fix:** 
- Create SubscriptionStorage using EncryptedPrefs/Keychain
- Wire ViewModels to FFI calls

#### 5. Health Monitoring (All Demos)
**Current:** Not implemented or shows mock "Healthy" status.
**Should Be Real:** Partially - useful for UX but not critical.
**Blocker:** Health checks require network access to payment backends.

**Fix:** 
- Wire to `PaykitClient.check_health()` 
- For demos, can use mock backends or show "Unknown" state

### Components That Should Remain Mock (for Demos)

1. **Actual Bitcoin/Lightning Transactions** - Without testnet/regtest setup, real payments could lose funds
2. **Homeserver Authentication** - Requires user accounts, complex setup
3. **Biometric Authentication** - Good for production but adds demo friction

---

## Library Features Not Exposed in Demos

### paykit-lib Features

| Feature | In Library | In CLI | In Web | In Mobile |
|---------|-----------|--------|--------|-----------|
| `LndExecutor` | ✅ Stub | ❌ | ❌ | ❌ |
| `EsploraExecutor` | ✅ Stub | ❌ | ❌ | ❌ |
| `HealthMonitor` | ✅ Real | ❌ | ❌ | ✅ FFI |
| `PaymentMethodSelector` | ✅ Real | ❌ | ❌ | ✅ FFI |
| `PaymentMethodRegistry` | ✅ Real | ❌ | ❌ | ✅ FFI |
| `PrivateEndpointManager` | ✅ Real | ⚠️ Partial | ⚠️ Partial | ❌ |
| `ProrationCalculator` | ✅ Real | ❌ | ❌ | ✅ FFI |
| Key rotation policies | ✅ Real | ❌ | ❌ | ❌ |
| Connection limits | ✅ Real | ❌ | ❌ | ❌ |
| Rate limiting | ✅ Real | ❌ | ❌ | ❌ |

### paykit-subscriptions Features

| Feature | In Library | In CLI | In Web | In Mobile |
|---------|-----------|--------|--------|-----------|
| Signature verification | ✅ Real | ❌ | ❌ | ❌ |
| Nonce tracking | ✅ Real | ⚠️ | ⚠️ | ❌ |
| Subscription modification | ✅ Real | ❌ | ❌ | ❌ |
| Pause/Resume | ✅ Real | ❌ | ❌ | ❌ |
| Upgrade/Downgrade | ✅ Real | ❌ | ❌ | ❌ |
| Billing date changes | ✅ Real | ❌ | ❌ | ❌ |

### paykit-interactive Features

| Feature | In Library | In CLI | In Web | In Mobile |
|---------|-----------|--------|--------|-----------|
| `PaymentStatusTracker` | ✅ Real | ❌ | ❌ | ✅ FFI |
| Connection pooling | ✅ Real | ❌ | ❌ | ❌ |
| Handshake rate limiting | ✅ Real | ❌ | ❌ | ❌ |
| Metrics collection | ✅ Real | ❌ | ❌ | ❌ |

---

## Gap Analysis by Demo App

### CLI Gaps (15% incomplete)

1. **Key Export/Import** - Users cannot backup or restore keys
2. **Payment Execution** - Shows simulation message instead of real payment
3. **Contact Search** - No search functionality
4. **Payment Method Validation** - No endpoint validation
5. **Health Monitoring** - Not exposed
6. **Subscription Modifications** - Cannot upgrade/downgrade/pause
7. **QR Codes** - Cannot display or scan

### Web Gaps (20% incomplete)

1. **Directory Publishing** - Mock only, not to real homeserver
2. **Payment Execution** - Coordination works, execution simulated
3. **Identity Switching** - Single identity only
4. **Key Backup/Import** - No export/import functionality
5. **Method Validation** - Endpoints not validated
6. **Subscription Modifications** - Missing upgrade/downgrade
7. **QR Codes** - Cannot display or scan

### iOS Gaps (55% incomplete)

1. **Directory Operations** - Not implemented
2. **Noise Payments** - Not implemented (requires TCP/WebSocket)
3. **Payment Methods** - Static UI, not connected to FFI
4. **Subscriptions** - Sample data only
5. **Auto-Pay** - Sample data only
6. **Payment Requests** - Sample data only
7. **Multiple Identities** - Single identity only
8. **QR Display/Scan** - Not implemented

### Android Gaps (60% incomplete)

Same as iOS, plus:
1. **Subscriptions UI** - Shows empty state (iOS has sample data)

---

## Implementation Plan

### Phase 1: Critical Path - Real Payment Coordination (2-3 weeks)

#### 1.1 Enable LND/Esplora Executors in paykit-lib
- [ ] Add `reqwest` dependency behind feature flag
- [ ] Complete LND REST client implementation
- [ ] Complete Esplora client implementation
- [ ] Add integration tests with regtest/testnet

#### 1.2 Wire Payment Execution to CLI Demo
- [ ] Add `--execute` flag to `pay` command
- [ ] Implement wallet configuration commands
- [ ] Connect to LND/Esplora executors
- [ ] Add real payment with testnet

#### 1.3 Wire Payment Execution to Web Demo
- [ ] Add executor configuration in settings
- [ ] Connect WebSocket payment flow to executor
- [ ] Display real transaction status

### Phase 2: Mobile Feature Parity (3-4 weeks)

#### 2.1 Connect Payment Methods UI
- [ ] iOS: Wire PaymentMethodsView to PaykitClient
- [ ] Android: Wire PaymentMethodsScreen to PaykitClient
- [ ] Implement endpoint validation
- [ ] Add method configuration persistence

#### 2.2 Implement Subscriptions
- [ ] Create SubscriptionStorage (Keychain/EncryptedPrefs)
- [ ] Wire subscription creation to FFI
- [ ] Implement subscription list with persistence
- [ ] Add payment request flow

#### 2.3 Implement Auto-Pay
- [ ] Create AutoPayStorage
- [ ] Wire auto-pay rules to FFI
- [ ] Implement spending limit tracking
- [ ] Add period reset logic

#### 2.4 Add Directory Operations (if Pubky transport available)
- [ ] Wire directory discovery to FFI
- [ ] Add contact import from follows
- [ ] (Optional) Add publishing if homeserver accessible

### Phase 3: Cross-Platform Enhancements (2-3 weeks)

#### 3.1 Key Backup/Import
- [ ] CLI: Add `export-key` and `import-key` commands
- [ ] Web: Add key export/import UI
- [ ] Document backup format across platforms

#### 3.2 QR Code Support
- [ ] CLI: Add QR code display for pubkey
- [ ] Web: Add QR display and (optionally) scan via camera
- [ ] iOS/Android: Add QR display and camera scan

#### 3.3 Subscription Modifications
- [ ] All: Add upgrade/downgrade support
- [ ] All: Add pause/resume support
- [ ] All: Add billing date change

### Phase 4: Production Hardening (2 weeks)

#### 4.1 Health Monitoring
- [ ] CLI: Add `health` command
- [ ] Web: Add health status indicators
- [ ] Mobile: Wire health monitoring UI

#### 4.2 Rate Limiting & Connection Limits
- [ ] Expose configuration in demos
- [ ] Add user-facing rate limit indicators

#### 4.3 Error Handling & Recovery
- [ ] Improve error messages across all demos
- [ ] Add retry logic for transient failures
- [ ] Add offline mode handling

---

## Priority Recommendations

### Must Have (Demo Blocking)

1. **Complete LND executor** - Without real payments, demos are incomplete
2. **Wire mobile payment methods** - Core feature, simple fix
3. **Wire mobile subscriptions** - Uses existing FFI, high impact
4. **Fix directory publishing (Web)** - Either real or clearly documented as mock

### Should Have (Polish)

1. **Key backup/import for CLI/Web** - Feature parity with mobile
2. **Health monitoring** - Good UX, shows system status
3. **QR codes** - Expected mobile feature
4. **Contact search (CLI)** - Usability improvement

### Nice to Have (Future)

1. **Subscription modifications** - Power user feature
2. **WebRTC P2P** - Eliminates WebSocket relay need
3. **Offline mode** - Enterprise/production feature
4. **Cross-device sync** - Complex, production feature

---

## Recommended README Updates

### Immediate Updates Needed

1. **All READMEs**: Add section clarifying payment execution is simulation mode
2. **Mobile READMEs**: Remove or clarify auto-pay flow diagrams as aspirational
3. **Web README**: Emphasize directory publishing limitation more prominently

### Template for Status Tables

All READMEs should use consistent status terminology:

| Status | Meaning |
|--------|---------|
| **Real** | Fully functional, persisted, uses real protocols |
| **Real (Mock Backend)** | Protocol works, backend is simulated |
| **Partial** | Some functionality works, some missing |
| **UI Only** | Visual demonstration, not connected to logic |
| **Not Implemented** | No code exists for this feature |

---

## Conclusion

The Paykit demo apps provide solid demonstrations of the protocol layer but lack the "last mile" integration to execute real payments. The highest-impact improvements are:

1. **Completing the LND/Esplora executors** to enable real testnet payments
2. **Wiring mobile UIs to existing FFI bindings** - much of the work is already done in `paykit-mobile`
3. **Standardizing features across platforms** - CLI and Web are ahead, mobile needs catch-up

With 6-8 weeks of focused development, all demos could achieve 90%+ feature parity with real payment execution on testnet.

