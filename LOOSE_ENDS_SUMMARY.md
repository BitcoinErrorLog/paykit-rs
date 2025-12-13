# Loose Ends Summary from Original Review

This document summarizes the remaining gaps and loose ends from the original demo apps feature parity review.

## ✅ Recently Completed (Not Yet Reflected in Plan)

The following items were completed in recent PRs but the plan document may not reflect them:

- ✅ **Phase 3.3: Multiple Identities (Mobile)** - iOS and Android now support multiple identities with full UI
- ✅ **Phase 2.2: Contact Discovery (Mobile)** - iOS and Android now have contact discovery from Pubky follows
- ✅ **Phase 2.3: QR Code Scanning (Mobile)** - iOS and Android now have QR scanners (iOS with camera, Android with manual input)
- ✅ **Phase 4.4: Secure Storage (CLI)** - CLI now supports OS keychain with migration command

## 🔴 High Priority Loose Ends

### 1. Noise Protocol Payments (Web & Mobile)

**Status:** CLI ✅ Complete | Web ❌ Pending | Mobile ❌ Pending

**Web Demo:**
- [ ] Implement WebSocket-based Noise protocol client
- [ ] Implement WebSocket relay server integration
- [ ] Complete payment coordination flow
- [ ] Integrate with `paykit-interactive` for receipt exchange
- **Location:** `paykit-demo-web/src/websocket_transport.rs`
- **Dependencies:** WebSocket infrastructure, relay server

**Mobile Demos (iOS & Android):**
- [ ] Integrate `pubky-noise` FFI bindings
- [ ] Implement TCP/WebSocket transport layer
- [ ] Add payment coordination UI
- [ ] Integrate with `paykit-interactive` manager
- **Dependencies:** FFI bindings for pubky-noise, transport layer implementation

**Priority:** High (core payment flow)

## 🟡 Medium Priority Loose Ends

### 2. Feature Parity Gaps

#### 2.1 Contact Search
- **Status:** Web ✅ | CLI ❌ | Mobile ❌
- **Gap:** CLI and Mobile demos don't have contact search functionality
- **Effort:** Low
- **Files:**
  - `paykit-demo-cli/src/commands/contacts.rs`
  - `paykit-mobile/ios-demo/.../Views/ContactsView.swift`
  - `paykit-mobile/android-demo/.../ui/ContactsScreen.kt`

#### 2.2 Key Backup/Restore
- **Status:** Mobile ✅ | CLI ❌ | Web ❌
- **Gap:** CLI and Web demos don't support encrypted key backup/restore
- **Effort:** Medium
- **Implementation:** Use Argon2 + AES-GCM like mobile apps
- **Files:**
  - `paykit-demo-cli/src/commands/` (new backup/restore commands)
  - `paykit-demo-web/src/` (backup/restore functions)

#### 2.3 Method Validation
- **Status:** Mobile ✅ | CLI ❌ | Web ❌
- **Gap:** CLI and Web don't validate payment method endpoints
- **Effort:** Low
- **Implementation:** Use `PaykitClient.validateEndpoint()` via FFI
- **Files:**
  - `paykit-demo-cli/src/commands/wallet.rs` or `paykit-demo-cli/src/commands/pay.rs`
  - `paykit-demo-web/src/payment_methods.rs`

#### 2.4 Recent Payments (Auto-Pay)
- **Status:** CLI ✅ | Web ✅ | Mobile ❌
- **Gap:** Mobile demos don't show recent auto-payments history
- **Effort:** Low
- **Files:**
  - `paykit-mobile/ios-demo/.../Views/AutoPayView.swift`
  - `paykit-mobile/android-demo/.../ui/AutoPayScreen.kt`

#### 2.5 Setup Checklist
- **Status:** Web ✅ | CLI ❌ | Mobile ❌
- **Gap:** CLI and Mobile don't have setup/onboarding checklist
- **Effort:** Low
- **Files:**
  - `paykit-demo-cli/src/commands/` (new setup-checklist command)
  - `paykit-mobile/ios-demo/.../Views/` (new onboarding view)
  - `paykit-mobile/android-demo/.../ui/` (new onboarding screen)

#### 2.6 Priority Ordering (Payment Methods)
- **Status:** CLI ✅ | Web ✅ | Mobile ❌
- **Gap:** Mobile demos don't allow users to set payment method priority/order
- **Effort:** Low
- **Files:**
  - `paykit-mobile/ios-demo/.../Views/PaymentMethodsView.swift`
  - `paykit-mobile/android-demo/.../ui/PaymentMethodsScreen.kt`

### 3. Mobile Directory Transport Default

**Status:** ⚠️ Still defaults to mock mode

- **Current:** Mobile apps default to `UnauthenticatedTransportFFI::newMock()`
- **Required:** Change default to real Pubky transport
- **Priority:** Medium
- **Files:**
  - iOS: `paykit-mobile/ios-demo/.../PaykitDemoApp.swift`
  - Android: `paykit-mobile/android-demo/.../PaykitClientWrapper.kt`

## 🟢 Low Priority Loose Ends (Phase 4 Advanced Features)

### 4. Payment Proof/Verification
- **Status:** ❌ Not implemented in any demo
- **Available:** ✅ `paykit-interactive::proof`
- **Effort:** Medium
- **Tasks:**
  - Generate payment proofs
  - Verify received proofs
  - Display proof status in receipts

### 5. Private Endpoints
- **Status:** ❌ Not implemented in any demo
- **Available:** ✅ `paykit-lib::private_endpoints`
- **Effort:** Medium
- **Tasks:**
  - Support private endpoint negotiation
  - Encrypted endpoint storage
  - Endpoint rotation

### 6. Key Rotation
- **Status:** ❌ Not implemented in any demo
- **Available:** ✅ `paykit-lib::rotation`
- **Effort:** High
- **Tasks:**
  - Implement rotation policies
  - Migration support
  - Rotation UI/commands

### 7. Secure Storage (Web)
- **Status:** ❌ Not implemented
- **Available:** ✅ Can use IndexedDB with encryption
- **Effort:** Medium
- **Tasks:**
  - Encrypted IndexedDB storage
  - Key derivation and encryption
  - Migration from localStorage

## 📋 Testing & Documentation Loose Ends

### 8. Testing
- [ ] Add tests for real directory publishing
- [ ] Add tests for Noise protocol integration
- [ ] Add tests for health monitoring
- [ ] Add tests for method selection
- [ ] E2E test: Publish → Discover → Pay flow
- [ ] E2E test: Noise handshake → Payment → Receipt
- [ ] E2E test: Subscription → Auto-pay → Payment
- [ ] Cross-platform compatibility tests

### 9. Demo Scripts
- [ ] Update CLI demo scripts for real features
- [ ] Create web demo scenarios
- [ ] Create mobile demo scenarios
- [ ] Cross-platform demo scenarios

### 10. Documentation
- [x] Update CLI README with real feature status ✅
- [x] Update Mobile READMEs with real feature status ✅
- [ ] Update Web README with real feature status
- [ ] Remove "mock" and "simulation" disclaimers where appropriate
- [ ] Document real directory publishing APIs
- [ ] Document Noise protocol integration
- [ ] Document health monitoring APIs
- [ ] Document method selection APIs
- [ ] Update quickstart guides
- [ ] Add migration guides (mock → real)
- [ ] Add troubleshooting guides for new features

## 📊 Summary by Priority

### High Priority (Core Functionality)
1. **Noise Protocol Payments** (Web & Mobile) - Core payment flow

### Medium Priority (Feature Parity)
2. **Contact Search** (CLI, Mobile)
3. **Key Backup/Restore** (CLI, Web)
4. **Method Validation** (CLI, Web)
5. **Recent Payments** (Mobile Auto-Pay)
6. **Setup Checklist** (CLI, Mobile)
7. **Priority Ordering** (Mobile Payment Methods)
8. **Mobile Directory Transport Default** (change from mock to real)

### Low Priority (Advanced Features)
9. **Payment Proof/Verification** (All demos)
10. **Private Endpoints** (All demos)
11. **Key Rotation** (All demos)
12. **Secure Storage** (Web)

### Infrastructure
13. **Testing** (comprehensive test coverage)
14. **Demo Scripts** (updated scenarios)
15. **Documentation** (complete API docs and guides)

## 🎯 Recommended Next Steps

1. **Update Plan Document** - Mark Phase 3.3, 2.2 (Mobile), 2.3 (Mobile), and 4.4 as completed
2. **High Priority:** Implement Noise Protocol Payments for Web and Mobile
3. **Medium Priority:** Address feature parity gaps (contact search, key backup, etc.)
4. **Low Priority:** Implement advanced features as needed
5. **Infrastructure:** Add comprehensive testing and update documentation

---

**Last Updated:** 2025-01-XX
**Status:** Active tracking document

