# Demo Apps Comprehensive Review & Feature Parity Plan

## Executive Summary

This document provides a comprehensive review of all Paykit demo applications (CLI, Web, iOS, Android), ensures all READMEs are current and optimal, compares feature deltas across demos, identifies mock implementations that should be made real, and creates a plan to achieve rich feature parity and demo capability.

**Review Date:** 2024  
**Status:** Complete Review - Ready for Implementation

---

## 1. README Review & Status

### 1.1 CLI Demo README (`paykit-demo-cli/README.md`)

**Status:** ✅ **Current and Comprehensive**

**Strengths:**
- Detailed feature status table with clear "Real" vs "Mock" indicators
- Comprehensive command reference
- Good security considerations section
- Clear architecture diagram
- Well-organized with proper sections

**Recommendations:**
- ✅ Already includes roadmap section
- ✅ Already documents mock vs real implementations clearly
- Minor: Could add more examples for advanced features (auto-pay, rotation)

**Action Items:**
- [ ] Add more advanced usage examples
- [ ] Update status table if any features change

### 1.2 Web Demo README (`paykit-demo-web/README.md`)

**Status:** ✅ **Current and Comprehensive**

**Strengths:**
- Excellent feature documentation
- Clear directory publishing modes explanation
- Comprehensive API reference links
- Good architecture documentation
- Detailed testing guide references

**Recommendations:**
- ✅ Already documents mock vs real modes clearly
- ✅ Already includes comprehensive feature documentation
- Minor: Could add more visual examples

**Action Items:**
- [ ] Add visual examples/screenshots if possible
- [ ] Update status table if any features change

### 1.3 iOS Demo README (`paykit-mobile/ios-demo/README.md`)

**Status:** ✅ **Current and Comprehensive**

**Strengths:**
- Clear feature status table
- Good security model documentation
- Comprehensive feature descriptions
- Clear roadmap section

**Recommendations:**
- ✅ Already documents real vs mock implementations
- ✅ Already includes comprehensive feature list
- Minor: Could add more code examples

**Action Items:**
- [ ] Add more Swift code examples
- [ ] Update status table if any features change

### 1.4 Android Demo README (`paykit-mobile/android-demo/README.md`)

**Status:** ✅ **Current and Comprehensive**

**Strengths:**
- Clear feature status table
- Good security model documentation
- Comprehensive feature descriptions
- Clear project structure

**Recommendations:**
- ✅ Already documents real vs mock implementations
- ✅ Already includes comprehensive feature list
- Minor: Could add more Kotlin code examples

**Action Items:**
- [ ] Add more Kotlin code examples
- [ ] Update status table if any features change

### 1.5 Mobile Library README (`paykit-mobile/README.md`)

**Status:** ✅ **Current and Comprehensive**

**Strengths:**
- Excellent API documentation
- Good examples for both Swift and Kotlin
- Clear type reference
- Good integration guide

**Recommendations:**
- ✅ Already comprehensive
- No changes needed

---

## 2. Feature Comparison Matrix

### 2.1 Core Features

| Feature | CLI | Web | iOS | Android | paykit-lib | Notes |
|---------|-----|-----|-----|---------|------------|-------|
| **Identity Management** |
| Ed25519 keypair generation | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| Multiple identities | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| Identity switching | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| Key backup/restore | ✅ | ✅ | ✅ | ✅ | N/A | All complete (CLI/Web: encrypted, Mobile: Keychain/EncryptedSharedPrefs) |
| **Directory Operations** |
| Publish payment methods | ✅ | ⚠️ | ⚠️ | ⚠️ | ✅ | CLI: Real, Web: Mock default (Direct/Proxy available), Mobile: Mock default (real available) |
| Discover payment methods | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| Remove payment endpoints | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| Contact discovery (follows) | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| **Contact Management** |
| Add/remove contacts | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| Contact search | ❌ | ✅ | ✅ | ✅ | N/A | CLI missing |
| Contact notes | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| Import from follows | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| **Payment Methods** |
| Configure methods | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| Method validation | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| Health monitoring | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| Method selection | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| Priority ordering | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| **Payments** |
| Noise protocol handshake | ✅ | ⚠️ | ❌ | ❌ | N/A | CLI: Real, Web: Partial (WebSocket), Mobile: Not implemented |
| Payment coordination | ✅ | ⚠️ | ❌ | ❌ | N/A | CLI: Real, Web: Partial, Mobile: Not implemented |
| Receipt exchange | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| Real payment execution (LND) | ✅* | ❌ | ❌ | ❌ | ✅ | CLI only (with wallet config) |
| Real payment execution (Esplora) | ✅* | ❌ | ❌ | ❌ | ✅ | CLI only (with wallet config) |
| **Receipts** |
| Receipt storage | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| Receipt filtering | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| Receipt statistics | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| Receipt export | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| **Subscriptions** |
| Payment requests | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| Subscription agreements | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| Proration calculator | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| **Auto-Pay** |
| Enable/disable auto-pay | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| Global settings | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| Per-peer limits | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| Auto-pay rules | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| Recent payments | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| **Dashboard** |
| Overview statistics | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| Recent activity | ✅ | ✅ | ✅ | ✅ | N/A | All complete |
| Setup checklist | ❌ | ✅ | ❌ | ❌ | N/A | Web only |
| **Other Features** |
| QR code scanning | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| URI parsing | ✅ | ✅ | ✅ | ✅ | ✅ | All complete |
| Payment proof/verification | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ✅ | Partial (custom proofs not verified) |
| Private endpoints | ✅ | ✅ | ✅ | ✅ | ✅ | All complete (CLI has rotation) |
| Endpoint rotation | ✅ | ❌ | ❌ | ❌ | ✅ | CLI only |
| Secure storage (OS keychain) | ⚠️ | ❌ | ✅ | ✅ | ✅ | CLI: Partial (migration available), Mobile: Complete |

**Legend:**
- ✅ = Fully implemented
- ⚠️ = Partially implemented or mock/default
- ❌ = Not implemented
- ✅* = Requires wallet configuration
- N/A = Not applicable (feature not in library)

### 2.2 Feature Delta Summary

**CLI Advantages:**
- ✅ Real Noise protocol payments (TCP-based)
- ✅ Real payment execution (LND/Esplora with wallet)
- ✅ Endpoint rotation management
- ✅ Contact search (missing - should add)

**Web Advantages:**
- ✅ Setup checklist
- ✅ Contact search
- ✅ Multiple publishing modes (Mock/Direct/Proxy)

**Mobile Advantages:**
- ✅ Native secure storage (Keychain/EncryptedSharedPreferences)
- ✅ Better UX for mobile workflows
- ✅ Camera-based QR scanning

**Common Gaps:**
- ⚠️ Web/Mobile: Noise payments not fully implemented (Web: Partial WebSocket, Mobile: Not implemented)
- ⚠️ Web/Mobile: Directory publishing defaults to mock (real available but not default)
- ⚠️ All: Payment proof verification for custom proof types incomplete

---

## 3. Mock Implementations Analysis

### 3.1 CLI Demo

#### A. Payment Execution (Simulation Mode)
**Current State:**
- Shows "SIMULATION MODE" message when wallet not configured
- Falls back to simulation when direct invoice payment not possible

**Location:** `paykit-demo-cli/src/commands/pay.rs:805-808`

**Status:** ✅ **Acceptable** - This is intentional fallback behavior, not a mock. Real execution available with wallet config.

**Action:** None needed - this is correct behavior

#### B. Custom Proof Verification
**Current State:**
- Shows warning: "Custom proof type - verification not implemented"

**Location:** `paykit-demo-cli/src/commands/receipts.rs:199`

**Status:** ⚠️ **Should Improve** - Should implement custom proof verification

**Action:** Add custom proof verification support

### 3.2 Web Demo

#### A. Directory Publishing (Mock Mode Default)
**Current State:**
- Default mode is "Mock" - saves to localStorage only
- Supports Direct and Proxy modes but not used by default
- Mock mode doesn't actually publish to homeserver

**Location:** `paykit-demo-web/src/directory.rs:11-54`, `payment_methods.rs:15-27`

**Status:** ⚠️ **Should Change Default** - Should default to Direct mode with clear UI indication

**Action:** 
- Change default to Direct mode
- Add clear UI indicator for publishing mode
- Make it easy to switch modes

#### B. WebSocket Noise Transport
**Current State:**
- WebSocket transport exists but payment client/server incomplete
- Payment coordination partially implemented

**Location:** `paykit-demo-web/src/websocket_transport.rs`

**Status:** ⚠️ **Should Complete** - WebSocket Noise transport exists but needs completion

**Action:** Complete WebSocket-based payment flows

### 3.3 Mobile Demos

#### A. Directory Transport (Mock Default)
**Current State:**
- Default mode uses `UnauthenticatedTransportFFI::newMock()`
- Real Pubky transport available but not default
- Mock mode doesn't actually publish/discover

**Location:**
- iOS: `paykit-mobile/ios-demo/PaykitDemo/.../PaykitDemoApp.swift`
- Android: `paykit-mobile/android-demo/.../PaykitClientWrapper.kt`

**Status:** ⚠️ **Should Change Default** - Should default to real transport with clear indication

**Action:**
- Change default to real Pubky transport
- Add clear UI indication of transport mode
- Add settings toggle for transport mode

#### B. Noise Payments
**Current State:**
- Not implemented - requires WebSocket/TCP transport
- Status: "Not Implemented" in README

**Location:** Both iOS and Android READMEs

**Status:** ❌ **Should Implement** - Core feature missing

**Action:** 
- Integrate `pubky-noise` FFI bindings
- Implement TCP/WebSocket transport layer
- Add payment coordination UI

---

## 4. Missing Features from paykit-lib

### 4.1 Features Available but Not Used in All Demos

#### A. Endpoint Rotation (`paykit-lib::rotation`)
**Available:** ✅
**Used in Demos:** CLI only
**Should Add To:** Web, Mobile

**Implementation:**
- Rotation manager
- Policy-based rotation
- Migration support

**Priority:** Medium

#### B. Payment Proof Verification (Custom Types)
**Available:** ✅ (partial)
**Used in Demos:** All (but custom proofs not verified)
**Should Add To:** All

**Implementation:**
- Custom proof type verification
- Proof registry extension
- Verification UI/commands

**Priority:** Low

#### C. Secure Storage (CLI)
**Available:** ✅
**Used in Demos:** Mobile only (CLI has migration but not default)
**Should Add To:** CLI (make default)

**Implementation:**
- CLI: Use OS keychain by default
- Migrate existing plaintext identities
- Secure credential storage

**Priority:** Medium

---

## 5. Comprehensive Feature Parity Plan

### Phase 1: Critical Mock Replacements (High Priority)

#### 1.1 Web: Change Directory Publishing Default
**Target:** Web Demo
**Effort:** Low
**Dependencies:** None
**Status:** ⚠️ Pending

**Tasks:**
- [ ] Change default from Mock to Direct mode
- [ ] Add clear UI indicator for publishing mode
- [ ] Add settings toggle for mode selection
- [ ] Update documentation

**Files to Modify:**
- `paykit-demo-web/src/directory.rs`
- `paykit-demo-web/src/payment_methods.rs`
- `paykit-demo-web/www/app.js`
- `paykit-demo-web/README.md`

**Priority:** High

#### 1.2 Mobile: Change Directory Transport Default
**Target:** iOS, Android
**Effort:** Low
**Dependencies:** Pubky SDK integration
**Status:** ⚠️ Pending

**Tasks:**
- [ ] Change default from mock to real transport
- [ ] Add clear UI indication of transport mode
- [ ] Add settings toggle for transport mode
- [ ] Update documentation

**Files to Modify:**
- `paykit-mobile/ios-demo/.../PaykitDemoApp.swift`
- `paykit-mobile/android-demo/.../PaykitClientWrapper.kt`
- `paykit-mobile/ios-demo/README.md`
- `paykit-mobile/android-demo/README.md`

**Priority:** High

#### 1.3 Web: Complete WebSocket Noise Transport
**Target:** Web Demo
**Effort:** High
**Dependencies:** WebSocket infrastructure
**Status:** ⚠️ Pending

**Tasks:**
- [ ] Complete WebSocket-based Noise protocol client
- [ ] Complete WebSocket relay server integration
- [ ] Complete payment coordination flow
- [ ] Integrate with `paykit-interactive` for receipt exchange
- [ ] Add payment UI flows

**Files to Modify:**
- `paykit-demo-web/src/websocket_transport.rs`
- `paykit-demo-web/src/payment.rs`
- `paykit-demo-web/www/app.js`

**Priority:** High

#### 1.4 Mobile: Implement Noise Payments
**Target:** iOS, Android
**Effort:** High
**Dependencies:** `pubky-noise` FFI bindings
**Status:** ❌ Not Started

**Tasks:**
- [ ] Integrate `pubky-noise` FFI bindings
- [ ] Implement TCP/WebSocket transport layer
- [ ] Add payment coordination UI
- [ ] Integrate with `paykit-interactive` manager
- [ ] Test end-to-end payment flows

**Files to Modify:**
- `paykit-mobile/src/transport_ffi.rs` (new)
- `paykit-mobile/ios-demo/.../Views/` (payment UI)
- `paykit-mobile/android-demo/.../ui/` (payment UI)

**Priority:** High

### Phase 2: Feature Additions (Medium Priority)

#### 2.1 CLI: Add Contact Search
**Target:** CLI Demo
**Effort:** Low
**Dependencies:** None
**Status:** ❌ Not Started

**Tasks:**
- [ ] Add search functionality to `contacts list` command
- [ ] Add `contacts search` subcommand
- [ ] Support search by name, pubkey, notes

**Files to Modify:**
- `paykit-demo-cli/src/commands/contacts.rs`

**Priority:** Medium

#### 2.2 All: Complete Payment Proof Verification
**Target:** All Demos
**Effort:** Medium
**Dependencies:** `paykit-interactive::proof`
**Status:** ⚠️ Partial

**Tasks:**
- [ ] Implement custom proof type verification
- [ ] Add proof registry extension support
- [ ] Add verification UI/commands
- [ ] Update receipt display to show proof status

**Files to Modify:**
- `paykit-demo-cli/src/commands/receipts.rs`
- `paykit-demo-web/src/payment.rs`
- `paykit-mobile/ios-demo/.../Views/ReceiptsView.swift`
- `paykit-mobile/android-demo/.../ui/ReceiptsScreen.kt`

**Priority:** Low

#### 2.3 Web & Mobile: Add Endpoint Rotation
**Target:** Web, Mobile
**Effort:** Medium
**Dependencies:** `paykit-lib::rotation`
**Status:** ❌ Not Started

**Tasks:**
- [ ] Add rotation management UI
- [ ] Add rotation policy configuration
- [ ] Add rotation history display
- [ ] Integrate with payment method management

**Files to Modify:**
- `paykit-demo-web/src/payment_methods.rs`
- `paykit-demo-web/www/app.js`
- `paykit-mobile/ios-demo/.../Views/PaymentMethodsView.swift`
- `paykit-mobile/android-demo/.../ui/PaymentMethodsScreen.kt`

**Priority:** Medium

#### 2.4 CLI: Make Secure Storage Default
**Target:** CLI Demo
**Effort:** Medium
**Dependencies:** `paykit-lib::secure_storage`
**Status:** ⚠️ Partial (migration available)

**Tasks:**
- [ ] Make OS keychain default for new identities
- [ ] Improve migration UX
- [ ] Add secure credential storage
- [ ] Update documentation

**Files to Modify:**
- `paykit-demo-cli/src/commands/setup.rs`
- `paykit-demo-cli/src/commands/migrate.rs`
- `paykit-demo-cli/README.md`

**Priority:** Medium

#### 2.5 CLI & Mobile: Add Setup Checklist
**Target:** CLI, Mobile
**Effort:** Low
**Dependencies:** None
**Status:** ❌ Not Started

**Tasks:**
- [ ] Add setup progress tracking
- [ ] Add setup checklist UI/command
- [ ] Guide users through initial setup

**Files to Modify:**
- `paykit-demo-cli/src/commands/dashboard.rs`
- `paykit-mobile/ios-demo/.../Views/DashboardView.swift`
- `paykit-mobile/android-demo/.../ui/DashboardScreen.kt`

**Priority:** Low

### Phase 3: Polish & Documentation (Low Priority)

#### 3.1 All: Update READMEs
**Target:** All Demos
**Effort:** Low
**Dependencies:** None
**Status:** ✅ Mostly Complete

**Tasks:**
- [ ] Review and update all READMEs after Phase 1 & 2
- [ ] Remove "mock" disclaimers where appropriate
- [ ] Add new feature documentation
- [ ] Update status tables

**Priority:** Low

#### 3.2 All: Add More Examples
**Target:** All Demos
**Effort:** Low
**Dependencies:** None
**Status:** ⚠️ Partial

**Tasks:**
- [ ] Add more advanced usage examples
- [ ] Add code examples for all platforms
- [ ] Add visual examples where possible

**Priority:** Low

---

## 6. Implementation Priority Matrix

### High Priority (Do First)
1. **Web: Change Directory Publishing Default** - Core functionality, currently mock default
2. **Mobile: Change Directory Transport Default** - Core functionality, currently mock default
3. **Web: Complete WebSocket Noise Transport** - Core payment flow, currently incomplete
4. **Mobile: Implement Noise Payments** - Core payment flow, currently missing

### Medium Priority (Do Next)
5. **CLI: Add Contact Search** - Feature completeness
6. **Web & Mobile: Add Endpoint Rotation** - Feature parity with CLI
7. **CLI: Make Secure Storage Default** - Security improvement
8. **All: Complete Payment Proof Verification** - Feature completeness

### Low Priority (Nice to Have)
9. **CLI & Mobile: Add Setup Checklist** - UX improvement
10. **All: Update READMEs** - Documentation
11. **All: Add More Examples** - Documentation

---

## 7. Testing Strategy

### 7.1 Unit Tests
- [ ] Add tests for real directory publishing (Web, Mobile)
- [ ] Add tests for WebSocket Noise transport (Web)
- [ ] Add tests for mobile Noise payments (Mobile)
- [ ] Add tests for endpoint rotation (Web, Mobile)
- [ ] Add tests for payment proof verification (All)

### 7.2 Integration Tests
- [ ] E2E test: Publish → Discover → Pay flow (All)
- [ ] E2E test: Noise handshake → Payment → Receipt (Web, Mobile)
- [ ] E2E test: Subscription → Auto-pay → Payment (All)
- [ ] Cross-platform compatibility tests

### 7.3 Demo Scripts
- [ ] Update CLI demo scripts for real features
- [ ] Create web demo scenarios
- [ ] Create mobile demo scenarios
- [ ] Cross-platform demo scenarios

---

## 8. Success Criteria

### Phase 1 Complete When:
- ✅ All directory publishing uses real Pubky sessions (default)
- ✅ Web supports WebSocket Noise payments
- ✅ Mobile supports Noise payments
- ✅ No "mock" defaults in core flows

### Phase 2 Complete When:
- ✅ Contact search available in CLI
- ✅ Endpoint rotation available in Web and Mobile
- ✅ Secure storage default in CLI
- ✅ Payment proof verification complete

### Phase 3 Complete When:
- ✅ All READMEs updated
- ✅ All examples added
- ✅ Documentation complete

### Final Success Criteria:
- ✅ All mock implementations replaced with real functionality (or clearly documented as intentional)
- ✅ Feature parity across all demos (core features)
- ✅ All paykit-lib features demonstrated in at least one demo
- ✅ Core test coverage maintained
- ✅ Complete documentation

---

## 9. Estimated Effort

| Phase | Tasks | Estimated Effort | Dependencies |
|-------|-------|------------------|--------------|
| Phase 1 | 4 tasks | 4-6 weeks | Pubky SDK, pubky-noise, WebSocket infrastructure |
| Phase 2 | 5 tasks | 2-3 weeks | paykit-lib features |
| Phase 3 | 2 tasks | 1 week | None |
| **Total** | **11 tasks** | **7-10 weeks** | Various |

---

## 10. Risk Assessment

### High Risk
- **WebSocket Noise Transport**: Browser limitations, CORS issues, WebSocket relay infrastructure
- **Mobile Noise Payments**: Platform-specific challenges, FFI complexity, transport layer

### Medium Risk
- **Directory Publishing Defaults**: Pubky session management complexity, error handling
- **Secure Storage (CLI)**: Platform-specific implementations, migration complexity

### Low Risk
- **Contact Search**: Straightforward implementation
- **Endpoint Rotation**: Well-defined APIs
- **Payment Proof Verification**: Extension of existing functionality

---

## 11. Next Steps

1. **Review this plan** with team
2. **Prioritize phases** based on business needs
3. **Assign tasks** to developers
4. **Set up tracking** (GitHub issues, project board)
5. **Begin Phase 1** implementation
6. **Regular check-ins** to track progress

---

## Appendix A: File Reference

### CLI Demo Files
- `paykit-demo-cli/src/commands/contacts.rs` - Contact management
- `paykit-demo-cli/src/commands/pay.rs` - Payment execution
- `paykit-demo-cli/src/commands/receipts.rs` - Receipt management
- `paykit-demo-cli/src/commands/setup.rs` - Identity setup
- `paykit-demo-cli/src/commands/dashboard.rs` - Dashboard

### Web Demo Files
- `paykit-demo-web/src/directory.rs` - Directory operations
- `paykit-demo-web/src/payment_methods.rs` - Payment method management
- `paykit-demo-web/src/websocket_transport.rs` - WebSocket transport
- `paykit-demo-web/src/payment.rs` - Payment coordination
- `paykit-demo-web/www/app.js` - Web UI

### Mobile Demo Files
- `paykit-mobile/ios-demo/.../PaykitDemoApp.swift` - iOS app setup
- `paykit-mobile/android-demo/.../PaykitClientWrapper.kt` - Android app setup
- `paykit-mobile/src/lib.rs` - FFI bindings
- `paykit-mobile/src/transport_ffi.rs` - Transport FFI (to be created)

### Library Files
- `paykit-lib/src/lib.rs` - Core library
- `paykit-lib/src/rotation/mod.rs` - Endpoint rotation
- `paykit-interactive/src/lib.rs` - Interactive payments
- `paykit-interactive/src/proof/mod.rs` - Payment proof

---

**Document Version:** 2.0  
**Last Updated:** 2024  
**Status:** Complete Review - Ready for Implementation

