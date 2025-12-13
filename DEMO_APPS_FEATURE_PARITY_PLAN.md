# Demo Apps Feature Parity & Gap Analysis Plan

## Executive Summary

This document provides a comprehensive review of all Paykit demo applications (CLI, Web, iOS, Android) and creates a plan to achieve feature parity and replace mock implementations with real functionality.

**Key Findings:**
- **CLI Demo**: Most complete, but has simulation modes for Noise payments and incomplete Pubky publishing
- **Web Demo**: Good feature set, but uses mock publishing by default and incomplete WebSocket Noise transport
- **Mobile (iOS/Android)**: Real implementations for most features, but missing Noise payments and using mock directory transport by default
- **Common Gaps**: Noise protocol payments, real Pubky directory publishing, health monitoring, method selection strategies

---

## 1. Feature Comparison Matrix

| Feature | CLI | Web | iOS | Android | paykit-lib | Status |
|---------|-----|-----|-----|---------|------------|--------|
| **Identity Management** |
| Ed25519 keypair generation | ✅ | ✅ | ✅ | ✅ | N/A | Complete |
| Multiple identities | ✅ | ✅ | ❌ | ❌ | N/A | Partial |
| Identity switching | ✅ | ✅ | ❌ | ❌ | N/A | Partial |
| Key backup/restore | ❌ | ❌ | ✅ | ✅ | N/A | Partial |
| **Directory Operations** |
| Publish payment methods | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ✅ | Mock/Partial |
| Discover payment methods | ✅ | ✅ | ✅ | ✅ | ✅ | Complete |
| Remove payment endpoints | ✅ | ✅ | ✅ | ✅ | ✅ | Complete |
| Contact discovery (follows) | ❌ | ❌ | ❌ | ❌ | ✅ | Missing |
| **Contact Management** |
| Add/remove contacts | ✅ | ✅ | ✅ | ✅ | N/A | Complete |
| Contact search | ❌ | ✅ | ❌ | ❌ | N/A | Partial |
| Contact notes | ✅ | ✅ | ✅ | ✅ | N/A | Complete |
| Import from follows | ❌ | ✅ | ❌ | ❌ | N/A | Partial |
| **Payment Methods** |
| Configure methods | ✅ | ✅ | ✅ | ✅ | N/A | Complete |
| Method validation | ❌ | ❌ | ✅ | ✅ | ✅ | Partial |
| Health monitoring | ❌ | ❌ | ✅ | ✅ | ✅ | Partial |
| Method selection | ❌ | ❌ | ✅ | ✅ | ✅ | Partial |
| Priority ordering | ✅ | ✅ | ❌ | ❌ | N/A | Partial |
| **Payments** |
| Noise protocol handshake | ⚠️ | ⚠️ | ❌ | ❌ | N/A | Partial/None |
| Payment coordination | ⚠️ | ⚠️ | ❌ | ❌ | N/A | Partial/None |
| Receipt exchange | ⚠️ | ⚠️ | ❌ | ❌ | N/A | Partial/None |
| Real payment execution (LND) | ✅* | ❌ | ❌ | ❌ | ✅ | Partial |
| Real payment execution (Esplora) | ✅* | ❌ | ❌ | ❌ | ✅ | Partial |
| **Receipts** |
| Receipt storage | ✅ | ✅ | ✅ | ✅ | N/A | Complete |
| Receipt filtering | ✅ | ✅ | ✅ | ✅ | N/A | Complete |
| Receipt statistics | ✅ | ✅ | ✅ | ✅ | N/A | Complete |
| Receipt export | ✅ | ✅ | ❌ | ❌ | N/A | Partial |
| **Subscriptions** |
| Payment requests | ✅ | ✅ | ✅ | ✅ | ✅ | Complete |
| Subscription agreements | ✅ | ✅ | ✅ | ✅ | ✅ | Complete |
| Proration calculator | ❌ | ❌ | ✅ | ✅ | ✅ | Partial |
| **Auto-Pay** |
| Enable/disable auto-pay | ✅ | ✅ | ✅ | ✅ | ✅ | Complete |
| Global settings | ✅ | ✅ | ✅ | ✅ | ✅ | Complete |
| Per-peer limits | ✅ | ✅ | ✅ | ✅ | ✅ | Complete |
| Auto-pay rules | ✅ | ✅ | ✅ | ✅ | ✅ | Complete |
| Recent payments | ✅ | ✅ | ❌ | ❌ | N/A | Partial |
| **Dashboard** |
| Overview statistics | ❌ | ✅ | ✅ | ✅ | N/A | Partial |
| Recent activity | ❌ | ✅ | ✅ | ✅ | N/A | Partial |
| Setup checklist | ❌ | ✅ | ❌ | ❌ | N/A | Partial |
| **Other Features** |
| QR code scanning | ❌ | ❌ | ❌ | ❌ | ✅ | Missing |
| URI parsing | ❌ | ❌ | ✅ | ✅ | ✅ | Partial |
| Payment proof/verification | ❌ | ❌ | ❌ | ❌ | ✅ | Missing |
| Private endpoints | ❌ | ❌ | ❌ | ❌ | ✅ | Missing |
| Key rotation | ❌ | ❌ | ❌ | ❌ | ✅ | Missing |
| Secure storage (OS keychain) | ❌ | ❌ | ✅ | ✅ | ✅ | Partial |

**Legend:**
- ✅ = Fully implemented
- ⚠️ = Partially implemented or mock
- ❌ = Not implemented
- ✅* = Requires wallet configuration
- N/A = Not applicable (feature not in library)

---

## 2. Mock Implementations Analysis

### 2.1 CLI Demo - Mock/Partial Implementations

#### A. Directory Publishing (`publish` command)
**Current State:**
- Shows warning: "Full Pubky session publishing not yet fully implemented"
- Only displays what would be published, doesn't actually publish

**Location:** `paykit-demo-cli/src/commands/publish.rs:59-64`

**Required Changes:**
- Implement full Pubky session creation using `pubky::PubkySession`
- Use `PubkyAuthenticatedTransport` from `paykit-lib`
- Call `set_payment_endpoint()` with real session

**Priority:** High

#### B. Noise Payment Receiver (`receive` command)
**Current State:**
- Shows warning: "Full Noise server implementation pending"
- Only simulates receiver, doesn't actually listen

**Location:** `paykit-demo-cli/src/commands/receive.rs:23-31`

**Required Changes:**
- Integrate `paykit-interactive` manager
- Use `pubky-noise` for Noise protocol server
- Implement TCP server listening on specified port
- Handle incoming payment requests and generate receipts

**Priority:** High

#### C. Noise Payment Initiator (`pay` command)
**Current State:**
- Supports direct invoice payment when wallet configured
- Shows warning: "Full Noise negotiation not yet implemented"
- Falls back to simulation mode without wallet

**Location:** `paykit-demo-cli/src/commands/pay.rs:139-141`

**Required Changes:**
- Integrate `paykit-interactive` for payment coordination
- Use `pubky-noise` for Noise protocol client
- Implement full payment flow: discover → connect → negotiate → execute → receipt

**Priority:** High

### 2.2 Web Demo - Mock/Partial Implementations

#### A. Directory Publishing (Default Mode)
**Current State:**
- Default mode is "Mock" - saves to localStorage only
- Supports Direct and Proxy modes but not used by default
- Mock mode doesn't actually publish to homeserver

**Location:** `paykit-demo-web/src/directory.rs:11-54`, `payment_methods.rs:15-27`

**Required Changes:**
- Change default to Direct mode (with CORS proxy fallback)
- Implement real Pubky session creation in WASM
- Use `PubkyAuthenticatedTransport` for real publishing
- Add UI toggle for publishing mode selection

**Priority:** High

#### B. WebSocket Noise Transport
**Current State:**
- `WasmPaymentClient.pay()` returns error: "Payment client not yet implemented - Phase 4"
- `WasmPaymentServer.listen()` returns error: "Payment server not yet implemented - Phase 4"

**Location:** `paykit-demo-web/src/websocket_transport.rs:319-350`

**Required Changes:**
- Implement WebSocket-based Noise protocol client
- Implement WebSocket relay server integration
- Complete payment coordination flow
- Integrate with `paykit-interactive` for receipt exchange

**Priority:** High

### 2.3 Mobile Demos - Mock/Partial Implementations

#### A. Directory Transport (Default Mock Mode)
**Current State:**
- Default mode uses `UnauthenticatedTransportFFI::newMock()`
- Real Pubky transport available but not default
- Mock mode doesn't actually publish/discover

**Location:**
- iOS: `paykit-mobile/ios-demo/PaykitDemo/PaykitDemo/PaykitDemoApp.swift:216-260`
- Android: `paykit-mobile/android-demo/app/src/main/java/com/paykit/demo/PaykitClientWrapper.kt:229-272`

**Required Changes:**
- Change default to real Pubky transport
- Provide clear UI indication of mock vs real mode
- Add settings toggle for transport mode

**Priority:** Medium

#### B. Noise Payments
**Current State:**
- Not implemented - requires WebSocket/TCP transport
- Status: "Not Implemented" in README

**Location:** Both iOS and Android READMEs

**Required Changes:**
- Integrate `pubky-noise` FFI bindings
- Implement TCP/WebSocket transport layer
- Add payment coordination UI
- Integrate with `paykit-interactive` manager

**Priority:** High

---

## 3. Missing Features from paykit-lib

### 3.1 Features Available but Not Used in Demos

#### A. Health Monitoring (`paykit-lib::health`)
**Available:** ✅
**Used in Demos:** Only iOS/Android mobile apps
**Should Add To:** CLI, Web

**Implementation:**
- `paykit-lib::health::check_payment_method_health()`
- Monitor endpoint availability
- Check method usability

**Priority:** Medium

#### B. Method Selection (`paykit-lib::selection`)
**Available:** ✅
**Used in Demos:** Only iOS/Android mobile apps
**Should Add To:** CLI, Web

**Implementation:**
- `paykit-lib::selection::select_payment_method()`
- Strategy-based selection (Balanced, Cost, Speed, Privacy)
- Automatic fallback handling

**Priority:** Medium

#### C. Contact Discovery (`paykit-lib::get_known_contacts`)
**Available:** ✅
**Used in Demos:** None
**Should Add To:** All demos

**Implementation:**
- Query Pubky follows directory
- Auto-import contacts from follows
- Sync with local contact list

**Priority:** Medium

#### D. QR Code Scanning (`paykit-lib::uri`)
**Available:** ✅ (`parse_uri`, `PaykitUri`)
**Used in Demos:** None (mentioned but not implemented)
**Should Add To:** All demos

**Implementation:**
- Parse `paykit://` URIs
- Parse `pubky://` URIs
- Handle payment requests, invoices, subscriptions

**Priority:** Medium

#### E. Payment Proof/Verification (`paykit-interactive::proof`)
**Available:** ✅
**Used in Demos:** None
**Should Add To:** All demos

**Implementation:**
- `PaymentProof` generation
- `ProofVerifier` for receipt validation
- Proof registry for different proof types

**Priority:** Low

#### F. Private Endpoints (`paykit-lib::private_endpoints`)
**Available:** ✅
**Used in Demos:** None
**Should Add To:** All demos

**Implementation:**
- Encrypted endpoint storage
- Private endpoint negotiation
- Endpoint rotation

**Priority:** Low

#### G. Key Rotation (`paykit-lib::rotation`)
**Available:** ✅
**Used in Demos:** None
**Should Add To:** All demos

**Implementation:**
- Rotation manager
- Policy-based rotation
- Migration support

**Priority:** Low

#### H. Secure Storage (`paykit-lib::secure_storage`)
**Available:** ✅ (platform-specific)
**Used in Demos:** Only mobile apps
**Should Add To:** CLI (desktop), Web (IndexedDB/encrypted)

**Implementation:**
- iOS: Keychain (already used in mobile)
- Android: EncryptedSharedPreferences (already used)
- Desktop: OS keychain (CLI should use)
- Web: Encrypted IndexedDB

**Priority:** High (for CLI), Medium (for Web)

---

## 4. Feature Parity Plan

### Phase 1: Critical Mock Replacements (High Priority)

#### 1.1 Real Directory Publishing
**Target:** CLI, Web, Mobile (all)
**Effort:** Medium
**Dependencies:** Pubky SDK integration

**Tasks:**
- [ ] CLI: Complete `publish` command with real Pubky session
- [ ] Web: Change default to Direct mode, implement real publishing
- [ ] Mobile: Change default transport from mock to real
- [ ] All: Add UI indicators for mock vs real mode
- [ ] All: Add error handling for publishing failures

**Files to Modify:**
- `paykit-demo-cli/src/commands/publish.rs`
- `paykit-demo-web/src/directory.rs`
- `paykit-demo-web/src/payment_methods.rs`
- `paykit-mobile/ios-demo/PaykitDemo/.../PaykitDemoApp.swift`
- `paykit-mobile/android-demo/.../PaykitClientWrapper.kt`

#### 1.2 Noise Protocol Payments
**Target:** CLI, Web, Mobile (all)
**Effort:** High
**Dependencies:** `pubky-noise`, `paykit-interactive`

**Tasks:**
- [ ] CLI: Complete `receive` command with Noise server
- [ ] CLI: Complete `pay` command with Noise client negotiation
- [ ] Web: Implement WebSocket Noise transport
- [ ] Web: Complete payment client/server
- [ ] Mobile: Integrate `pubky-noise` FFI
- [ ] Mobile: Add TCP/WebSocket transport layer
- [ ] All: Integrate `paykit-interactive` manager
- [ ] All: Implement receipt exchange

**Files to Modify:**
- `paykit-demo-cli/src/commands/receive.rs`
- `paykit-demo-cli/src/commands/pay.rs`
- `paykit-demo-web/src/websocket_transport.rs`
- `paykit-demo-web/src/payment.rs`
- `paykit-mobile/src/transport_ffi.rs` (new)
- `paykit-mobile/ios-demo/.../Views/` (payment UI)
- `paykit-mobile/android-demo/.../ui/` (payment UI)

### Phase 2: Feature Additions (Medium Priority)

#### 2.1 Health Monitoring & Method Selection
**Target:** CLI, Web
**Effort:** Low
**Dependencies:** `paykit-lib::health`, `paykit-lib::selection`

**Tasks:**
- [ ] CLI: Add `wallet health` command
- [ ] CLI: Add method selection to `pay` command
- [ ] Web: Add health status to payment methods UI
- [ ] Web: Add method selection dropdown
- [ ] Both: Display method health indicators

**Files to Modify:**
- `paykit-demo-cli/src/commands/wallet.rs` (add health subcommand)
- `paykit-demo-cli/src/commands/pay.rs` (add selection)
- `paykit-demo-web/src/payment_methods.rs`
- `paykit-demo-web/www/app.js`

#### 2.2 Contact Discovery
**Target:** All demos
**Effort:** Medium
**Dependencies:** `paykit-lib::get_known_contacts`

**Tasks:**
- [ ] CLI: Add `contacts discover` command
- [ ] Web: Add "Import from Follows" button
- [ ] Mobile: Add contact discovery feature
- [ ] All: Sync discovered contacts with local list

**Files to Modify:**
- `paykit-demo-cli/src/commands/contacts.rs` (add discover)
- `paykit-demo-web/src/contacts.rs`
- `paykit-demo-web/www/app.js`
- `paykit-mobile/ios-demo/.../Views/ContactsView.swift`
- `paykit-mobile/android-demo/.../ui/ContactsScreen.kt`

#### 2.3 QR Code Scanning
**Target:** All demos
**Effort:** Medium
**Dependencies:** `paykit-lib::uri`, camera access

**Tasks:**
- [ ] CLI: Add QR code display (terminal-friendly)
- [ ] Web: Add camera-based QR scanner
- [ ] Mobile: Integrate camera QR scanner
- [ ] All: Parse and handle scanned URIs

**Files to Modify:**
- `paykit-demo-cli/src/commands/` (add qr subcommand)
- `paykit-demo-web/www/app.js` (add scanner)
- `paykit-mobile/ios-demo/.../Views/` (scanner view)
- `paykit-mobile/android-demo/.../ui/` (scanner screen)

### Phase 3: Feature Parity (Medium-Low Priority)

#### 3.1 Dashboard Features
**Target:** CLI, Mobile
**Effort:** Low
**Dependencies:** None

**Tasks:**
- [ ] CLI: Add `dashboard` command with statistics
- [ ] Mobile: Ensure dashboard matches web feature set
- [ ] All: Standardize dashboard metrics

**Files to Modify:**
- `paykit-demo-cli/src/commands/dashboard.rs` (new)
- `paykit-mobile/ios-demo/.../Views/DashboardView.swift`
- `paykit-mobile/android-demo/.../ui/DashboardScreen.kt`

#### 3.2 Receipt Export
**Target:** Mobile
**Effort:** Low
**Dependencies:** None

**Tasks:**
- [ ] iOS: Add receipt export (JSON, CSV)
- [ ] Android: Add receipt export (JSON, CSV)
- [ ] Both: Share functionality

**Files to Modify:**
- `paykit-mobile/ios-demo/.../Views/ReceiptsView.swift`
- `paykit-mobile/android-demo/.../ui/ReceiptsScreen.kt`

#### 3.3 Multiple Identities
**Target:** Mobile
**Effort:** Medium
**Dependencies:** None

**Tasks:**
- [ ] iOS: Add identity switching
- [ ] Android: Add identity switching
- [ ] Both: Identity management UI

**Files to Modify:**
- `paykit-mobile/ios-demo/.../Views/SettingsView.swift`
- `paykit-mobile/android-demo/.../ui/SettingsScreen.kt`

#### 3.4 Proration Calculator
**Target:** CLI, Web
**Effort:** Low
**Dependencies:** `paykit-subscriptions::proration`

**Tasks:**
- [ ] CLI: Add proration to subscription commands
- [ ] Web: Add proration UI for subscription changes
- [ ] Both: Display proration details

**Files to Modify:**
- `paykit-demo-cli/src/commands/subscriptions.rs`
- `paykit-demo-web/src/subscriptions.rs`
- `paykit-demo-web/www/app.js`

### Phase 4: Advanced Features (Low Priority)

#### 4.1 Payment Proof/Verification
**Target:** All demos
**Effort:** Medium
**Dependencies:** `paykit-interactive::proof`

**Tasks:**
- [ ] All: Generate payment proofs
- [ ] All: Verify received proofs
- [ ] All: Display proof status in receipts

#### 4.2 Private Endpoints
**Target:** All demos
**Effort:** Medium
**Dependencies:** `paykit-lib::private_endpoints`

**Tasks:**
- [ ] All: Support private endpoint negotiation
- [ ] All: Encrypted endpoint storage
- [ ] All: Endpoint rotation

#### 4.3 Key Rotation
**Target:** All demos
**Effort:** High
**Dependencies:** `paykit-lib::rotation`

**Tasks:**
- [ ] All: Implement rotation policies
- [ ] All: Migration support
- [ ] All: Rotation UI/commands

#### 4.4 Secure Storage (CLI)
**Target:** CLI
**Effort:** Medium
**Dependencies:** `paykit-lib::secure_storage`

**Tasks:**
- [ ] CLI: Use OS keychain instead of plaintext files
- [ ] CLI: Encrypt identity storage
- [ ] CLI: Secure credential storage

---

## 5. Implementation Priority Matrix

### High Priority (Do First)
1. **Real Directory Publishing** - Core functionality, currently mock
2. **Noise Protocol Payments** - Core payment flow, currently incomplete
3. **Secure Storage (CLI)** - Security critical, currently plaintext

### Medium Priority (Do Next)
4. **Health Monitoring & Method Selection** - UX improvement
5. **Contact Discovery** - Feature completeness
6. **QR Code Scanning** - User convenience
7. **Dashboard Features** - Feature parity

### Low Priority (Nice to Have)
8. **Payment Proof/Verification** - Advanced feature
9. **Private Endpoints** - Advanced feature
10. **Key Rotation** - Advanced feature

---

## 6. Testing Strategy

### 6.1 Unit Tests
- [ ] Add tests for real directory publishing
- [ ] Add tests for Noise protocol integration
- [ ] Add tests for health monitoring
- [ ] Add tests for method selection

### 6.2 Integration Tests
- [ ] E2E test: Publish → Discover → Pay flow
- [ ] E2E test: Noise handshake → Payment → Receipt
- [ ] E2E test: Subscription → Auto-pay → Payment
- [ ] Cross-platform compatibility tests

### 6.3 Demo Scripts
- [ ] Update CLI demo scripts for real features
- [ ] Create web demo scenarios
- [ ] Create mobile demo scenarios
- [ ] Cross-platform demo scenarios

---

## 7. Documentation Updates

### 7.1 README Updates
- [ ] Update CLI README with real feature status
- [ ] Update Web README with real feature status
- [ ] Update Mobile READMEs with real feature status
- [ ] Remove "mock" and "simulation" disclaimers where appropriate

### 7.2 API Documentation
- [ ] Document real directory publishing APIs
- [ ] Document Noise protocol integration
- [ ] Document health monitoring APIs
- [ ] Document method selection APIs

### 7.3 User Guides
- [ ] Update quickstart guides
- [ ] Add migration guides (mock → real)
- [ ] Add troubleshooting guides for new features

---

## 8. Success Criteria

### Phase 1 Complete When:
- ✅ All directory publishing uses real Pubky sessions
- ✅ All demos support Noise protocol payments
- ✅ CLI uses secure storage (OS keychain)
- ✅ No "mock" or "simulation" warnings in core flows

### Phase 2 Complete When:
- ✅ Health monitoring available in all demos
- ✅ Method selection available in all demos
- ✅ Contact discovery available in all demos
- ✅ QR code scanning available in all demos

### Phase 3 Complete When:
- ✅ Feature parity achieved across all demos
- ✅ All demos have dashboard
- ✅ All demos support receipt export
- ✅ All demos support multiple identities

### Final Success Criteria:
- ✅ All mock implementations replaced with real functionality
- ✅ Feature parity across CLI, Web, iOS, Android
- ✅ All paykit-lib features demonstrated in at least one demo
- ✅ Comprehensive test coverage
- ✅ Complete documentation

---

## 9. Estimated Effort

| Phase | Tasks | Estimated Effort | Dependencies |
|-------|-------|------------------|--------------|
| Phase 1 | 8 tasks | 3-4 weeks | Pubky SDK, pubky-noise |
| Phase 2 | 9 tasks | 2-3 weeks | paykit-lib features |
| Phase 3 | 8 tasks | 1-2 weeks | None |
| Phase 4 | 4 tasks | 2-3 weeks | Advanced features |
| **Total** | **29 tasks** | **8-12 weeks** | Various |

---

## 10. Risk Assessment

### High Risk
- **Noise Protocol Integration**: Complex, requires careful testing
- **WebSocket Transport**: Browser limitations, CORS issues
- **Mobile Transport**: Platform-specific challenges

### Medium Risk
- **Directory Publishing**: Pubky session management complexity
- **Secure Storage**: Platform-specific implementations

### Low Risk
- **Health Monitoring**: Straightforward API integration
- **Method Selection**: Well-defined APIs
- **Contact Discovery**: Simple directory queries

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
- `paykit-demo-cli/src/commands/publish.rs` - Directory publishing
- `paykit-demo-cli/src/commands/receive.rs` - Payment receiver
- `paykit-demo-cli/src/commands/pay.rs` - Payment initiator
- `paykit-demo-cli/src/commands/wallet.rs` - Wallet configuration

### Web Demo Files
- `paykit-demo-web/src/directory.rs` - Directory operations
- `paykit-demo-web/src/payment_methods.rs` - Payment method management
- `paykit-demo-web/src/websocket_transport.rs` - WebSocket transport
- `paykit-demo-web/src/payment.rs` - Payment coordination

### Mobile Demo Files
- `paykit-mobile/ios-demo/PaykitDemo/.../PaykitDemoApp.swift` - iOS app setup
- `paykit-mobile/android-demo/.../PaykitClientWrapper.kt` - Android app setup
- `paykit-mobile/src/lib.rs` - FFI bindings

### Library Files
- `paykit-lib/src/lib.rs` - Core library
- `paykit-lib/src/health/mod.rs` - Health monitoring
- `paykit-lib/src/selection/mod.rs` - Method selection
- `paykit-interactive/src/lib.rs` - Interactive payments
- `paykit-interactive/src/manager.rs` - Payment manager

---

**Document Version:** 1.0  
**Last Updated:** 2024  
**Author:** AI Assistant  
**Status:** Draft - Ready for Review

