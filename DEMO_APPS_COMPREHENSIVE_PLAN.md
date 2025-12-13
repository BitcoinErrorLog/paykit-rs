# Paykit Demo Apps Comprehensive Review & Feature Parity Plan

**Date**: December 2024  
**Status**: Phase 5 Complete - Mobile Payment Requests

---

## Executive Summary

This document provides a thorough review of all Paykit demo applications (CLI, Web, iOS, Android), compares their features, identifies mock vs real implementations, and creates a comprehensive plan for achieving feature parity and optimal demo capability.

### Implementation Progress

- **Phase 1**: ✅ Complete - iOS Payment Methods, Health Monitoring, and Method Selection now use real FFI
- **Phase 2**: ✅ Complete - Android Payment Methods, Health Monitoring, and Method Selection now use real FFI
- **Phase 3**: ✅ Complete - Mobile Directory Operations now support configurable mock/callback transport
- **Phase 4**: ✅ Complete - Web Real Publishing with Mock/Direct/Proxy modes
- **Phase 5**: ✅ Complete - Mobile Payment Requests with persistent storage
- **Phase 6**: Pending - Documentation & Final Verification

### Key Findings

1. **CLI Demo**: Most complete, with real implementations for most features
2. **Web Demo**: Good feature coverage but uses mock publishing for directory operations
3. **iOS Demo**: ✅ Now has real FFI integration for Payment Methods, Health, and Selection
4. **Android Demo**: ✅ Now has real FFI integration for Payment Methods, Health, and Selection
5. **Library Features**: Many features from paykit-lib, paykit-interactive, and paykit-subscriptions are not fully utilized

---

## 1. Current State Analysis

### 1.1 CLI Demo (`paykit-demo-cli`)

**Status**: ✅ **Most Complete**

| Feature | Implementation | Notes |
|---------|---------------|-------|
| Identity Management | ✅ Real | Ed25519 keypairs, file persistence |
| Contact Management | ✅ Real | Full CRUD operations |
| Directory Publish | ✅ Real | Pubky homeserver integration |
| Directory Discover | ✅ Real | HTTP queries to homeservers |
| Noise Handshake | ✅ Real | TCP-based encrypted channel |
| Payment Coordination | ✅ Real | Request/receipt exchange |
| Wallet Configuration | ✅ Real | LND and Esplora setup |
| Payment Execution | ✅ Real (with wallet) | Requires configured wallet |
| Subscriptions | ✅ Real | Full P2P lifecycle |
| Auto-Pay Rules | ✅ Real | Rules and limits with file persistence |
| Spending Limits | ✅ Real | Per-peer limits with period tracking |
| Receipts | ✅ Real | Stored and queryable |

**README Status**: ✅ **Current and comprehensive**

**Gaps**:
- None significant - this is the reference implementation

---

### 1.2 Web Demo (`paykit-demo-web`)

**Status**: ⚠️ **Mostly Real with Mock Publishing**

| Feature | Implementation | Notes |
|---------|---------------|-------|
| Identity Management | ✅ Real | Ed25519 keypairs, localStorage persistence |
| Contact Management | ✅ Real | Full CRUD, localStorage persistence |
| Receipt Management | ✅ Real | Full history with filtering, localStorage |
| Dashboard | ✅ Real | Statistics from real stored data |
| Noise Payments | ✅ Real | WebSocket-based encrypted payments |
| Payment Methods | ✅ Real | Configured locally with real publishing options |
| Directory Publish | ✅ Configurable | Mock, Direct, or Proxy modes |
| Directory Discover | ✅ Real | HTTP queries to homeservers |
| Subscriptions | ✅ Real | Full P2P lifecycle, localStorage |
| Auto-Pay | ✅ Real | Rules and limits, localStorage |
| Spending Limits | ✅ Real | Per-peer limits with period reset |

**README Status**: ✅ **Current and comprehensive**

**Key Improvement** (Phase 4):
- **Directory Publishing**: Now supports three modes: Mock (localStorage), Direct (CORS-enabled homeserver), and Proxy (via CORS proxy). Real publishing is now possible with proper configuration.

**Remaining Gaps**:
1. Payment execution (no wallet integration - WebSocket transport only)

---

### 1.3 iOS Demo (`paykit-mobile/ios-demo`)

**Status**: ⚠️ **UI Complete, FFI Integration Minimal**

| Feature | Implementation | Notes |
|---------|---------------|-------|
| Dashboard | ✅ Real | Overview with stats, recent activity |
| Key Management | ✅ Real | Ed25519/X25519 via Rust FFI, Keychain |
| Key Backup/Restore | ✅ Real | Argon2 + AES-GCM encrypted exports |
| Contacts | ✅ Real | Keychain-backed contact storage |
| Receipts | ✅ Real | Payment history with search/filtering |
| Payment Methods | ❌ UI Only | Static list, not connected to PaykitClient |
| Health Monitoring | ❌ UI Only | Displays mock "Healthy" status |
| Subscriptions | ✅ Real | Keychain-backed subscription storage |
| Auto-Pay | ✅ Real | Keychain-backed settings, limits, rules |
| Payment Requests | ✅ Real | Keychain-backed storage with FFI integration |
| Directory Operations | ✅ Configurable | DirectoryService supports mock or callback transport |
| Noise Payments | ❌ Not Implemented | Requires WebSocket/TCP transport |

**README Status**: ✅ **Current and accurate**

**Critical Gaps** (Resolved):
1. ✅ `PaykitClient` now used from UI for Payment Methods, Health, Selection
2. ✅ `list_methods()`, `validate_endpoint()`, `select_method()`, `check_health()` now called
3. ✅ Directory transport now configurable for real Pubky integration
4. ✅ Payment requests now persisted to Keychain via `PaymentRequestStorage`
4. Directory operations use mock transport instead of real Pubky integration
5. Payment method UI shows static data instead of real FFI calls

---

### 1.4 Android Demo (`paykit-mobile/android-demo`)

**Status**: ⚠️ **UI Complete, FFI Integration Minimal**

| Feature | Implementation | Notes |
|---------|---------------|-------|
| Dashboard | ✅ Real | Overview with stats, recent activity |
| Key Management | ✅ Real | Ed25519/X25519 via Rust FFI, EncryptedSharedPreferences |
| Key Backup/Restore | ✅ Real | Argon2 + AES-GCM encrypted exports |
| Contacts | ✅ Real | EncryptedSharedPreferences-backed storage |
| Receipts | ✅ Real | Payment history with search/filtering |
| Payment Methods | ❌ UI Only | Static list, not connected to PaykitClient |
| Health Monitoring | ❌ UI Only | Displays mock "Healthy" status |
| Subscriptions | ✅ Real | EncryptedSharedPreferences-backed storage |
| Auto-Pay | ✅ Real | EncryptedSharedPreferences-backed settings |
| Payment Requests | ✅ Real | EncryptedSharedPreferences storage with FFI integration |
| Directory Operations | ✅ Configurable | DirectoryService supports mock or callback transport |
| Noise Payments | ❌ Not Implemented | Requires WebSocket/TCP transport |

**README Status**: ✅ **Current and accurate**

**Critical Gaps** (Resolved):
1. ✅ `PaykitClient` now used from UI for Payment Methods, Health, Selection
2. ✅ `listMethods()`, `validateEndpoint()`, `selectMethod()`, `checkHealth()` now called
3. ✅ Directory transport now configurable for real Pubky integration
4. ✅ Payment requests now persisted to EncryptedSharedPreferences via `PaymentRequestStorage`
4. Directory operations use mock transport instead of real Pubky integration
5. Payment method UI shows static data instead of real FFI calls

---

## 2. Feature Comparison Matrix

### 2.1 Core Features

| Feature | CLI | Web | iOS | Android |
|---------|-----|-----|-----|---------|
| **Identity Management** |
| Ed25519 Key Generation | ✅ | ✅ | ✅ | ✅ |
| X25519 Device Key | ✅ | ❌ | ✅ | ✅ |
| Secure Storage | 📁 File | ⚠️ localStorage | ✅ Keychain | ✅ EncryptedPrefs |
| Key Export/Import | ✅ | ✅ | ✅ | ✅ |
| Multiple Identities | ✅ | ✅ | ❌ | ❌ |
| **Directory Operations** |
| Publish Endpoints | ✅ Real | ❌ Mock | ⚠️ Configurable | ⚠️ Configurable |
| Discover Methods | ✅ | ✅ | ⚠️ Configurable | ⚠️ Configurable |
| Remove Endpoints | ✅ | ❌ | ❌ | ❌ |
| Fetch Known Contacts | ✅ | ✅ | ❌ | ❌ |
| **Payment Operations** |
| Noise Handshake | ✅ TCP | ✅ WebSocket | ❌ | ❌ |
| Payment Coordination | ✅ | ✅ | ❌ | ❌ |
| Receipt Exchange | ✅ | ✅ | ❌ | ❌ |
| Payment Execution | ✅ (with wallet) | ❌ | ❌ | ❌ |
| **Subscriptions** |
| Create/Manage | ✅ | ✅ | ✅ (storage) | ✅ (storage) |
| Payment Requests | ✅ | ✅ | ❌ (UI only) | ❌ (UI only) |
| Auto-Pay Rules | ✅ | ✅ | ✅ | ✅ |
| Spending Limits | ✅ | ✅ | ✅ | ✅ |
| **Contact Management** |
| Add/Remove | ✅ | ✅ | ✅ | ✅ |
| Search/Filter | ✅ | ✅ | ✅ | ✅ |
| Payment History | ✅ | ✅ | ✅ | ✅ |
| **Payment Methods** |
| List Methods | ✅ | ✅ | ❌ (static) | ❌ (static) |
| Validate Endpoint | ✅ | ✅ | ❌ | ❌ |
| Select Method | ✅ | ✅ | ❌ | ❌ |
| Health Monitoring | ✅ | ✅ | ❌ (mock) | ❌ (mock) |

---

## 3. Mock vs Real Implementation Analysis

### 3.1 Mock Implementations That Should Be Real

#### Web Demo

1. **Directory Publishing** (`paykit-demo-web/src/directory.rs`)
   - **Current**: ✅ Configurable Mock/Direct/Proxy modes
   - **Should Be**: Real HTTP PUT to Pubky homeserver
   - **Status**: ✅ Complete - DirectoryClient supports all three modes
   - **Priority**: ✅ Complete

#### iOS Demo

1. **Payment Methods UI** (`PaymentMethodsView.swift`)
   - **Current**: Static hardcoded list
   - **Should Be**: Call `PaykitClient.list_methods()` from FFI
   - **Priority**: High

2. **Health Monitoring** (`PaymentMethodsView.swift`)
   - **Current**: Always shows "Healthy" status
   - **Should Be**: Call `PaykitClient.check_health()` from FFI
   - **Priority**: Medium

3. **Directory Operations** (`DirectoryService.swift` or equivalent)
   - **Current**: ✅ Configurable mock/callback transport
   - **Should Be**: Real Pubky transport integration (implement callback)
   - **Priority**: Medium (transport ready, need Pubky SDK integration)

4. **Payment Method Selection**
   - **Current**: ✅ Real FFI integration
   - **Should Be**: Call `PaykitClient.select_method()` from FFI
   - **Priority**: ✅ Complete

#### Android Demo

1. **Payment Methods UI** (`PaymentMethodsScreen.kt`)
   - **Current**: ✅ Real FFI integration
   - **Should Be**: Call `PaykitClient.listMethods()` from FFI
   - **Priority**: ✅ Complete

2. **Health Monitoring** (`PaymentMethodsScreen.kt`)
   - **Current**: ✅ Real FFI integration
   - **Should Be**: Call `PaykitClient.checkHealth()` from FFI
   - **Priority**: ✅ Complete

3. **Directory Operations** (`DirectoryService.kt` or equivalent)
   - **Current**: ✅ Configurable mock/callback transport
   - **Should Be**: Real Pubky transport integration (implement callback)
   - **Priority**: Medium (transport ready, need Pubky SDK integration)

4. **Payment Method Selection**
   - **Current**: ✅ Real FFI integration
   - **Should Be**: Call `PaykitClient.selectMethod()` from FFI
   - **Priority**: ✅ Complete

---

### 3.2 Features That Are Appropriately Mocked (Demo Limitations)

These are acceptable for demo purposes:

1. **Payment Execution** (Web, Mobile)
   - Cannot execute real payments without wallet integration
   - Demo limitation is appropriate

2. **Noise Payments** (Mobile)
   - Requires WebSocket/TCP transport which is complex on mobile
   - Demo limitation is acceptable

3. **Key Storage** (Web)
   - localStorage is appropriate for browser demo
   - Production would use secure storage

---

## 4. Library Features Not Fully Utilized

### 4.1 From `paykit-lib`

| Feature | CLI | Web | iOS | Android | Status |
|---------|-----|-----|-----|---------|--------|
| `set_payment_endpoint` | ✅ | ⚠️ Mock | ❌ | ❌ | Mobile: Not used |
| `remove_payment_endpoint` | ✅ | ❌ | ❌ | ❌ | Web/Mobile: Missing |
| `get_payment_list` | ✅ | ✅ | ❌ | ❌ | Mobile: Not used |
| `get_payment_endpoint` | ✅ | ✅ | ❌ | ❌ | Mobile: Not used |
| `get_known_contacts` | ✅ | ✅ | ❌ | ❌ | Mobile: Not used |
| Payment Executors (LND/Esplora) | ✅ | ❌ | ❌ | ❌ | Web/Mobile: Not used |
| Health Monitoring | ✅ | ✅ | ❌ | ❌ | Mobile: Not used |
| Method Selection | ✅ | ✅ | ❌ | ❌ | Mobile: Not used |

### 4.2 From `paykit-interactive`

| Feature | CLI | Web | iOS | Android | Status |
|---------|-----|-----|-----|---------|--------|
| `PaykitNoiseChannel` | ✅ TCP | ✅ WebSocket | ❌ | ❌ | Mobile: Not implemented |
| `PaykitReceipt` | ✅ | ✅ | ✅ (storage) | ✅ (storage) | Mobile: Storage only |
| `PaykitInteractiveManager` | ⚠️ Partial | ⚠️ Partial | ❌ | ❌ | Not fully utilized |
| Receipt Generation | ✅ | ✅ | ❌ | ❌ | Mobile: Not used |
| Rate Limiting | ✅ | ❌ | ❌ | ❌ | Web/Mobile: Not used |

### 4.3 From `paykit-subscriptions`

| Feature | CLI | Web | iOS | Android | Status |
|---------|-----|-----|-----|---------|--------|
| `Subscription` | ✅ | ✅ | ✅ (storage) | ✅ (storage) | Mobile: Storage only |
| `PaymentRequest` | ✅ | ✅ | ❌ (UI only) | ❌ (UI only) | Mobile: UI only |
| `AutoPayRule` | ✅ | ✅ | ✅ | ✅ | All: Complete |
| `PeerSpendingLimit` | ✅ | ✅ | ✅ | ✅ | All: Complete |
| Signing/Verification | ✅ | ❌ | ❌ | ❌ | Web/Mobile: Not used |
| Proration Calculator | ✅ | ✅ | ✅ | ✅ | All: Complete |

### 4.4 From `paykit-mobile` FFI

| Feature | iOS | Android | Status |
|---------|-----|---------|--------|
| `PaykitClient` | ⚠️ Created | ⚠️ Created | Not used from UI |
| `list_methods()` | ❌ | ❌ | Not called |
| `validate_endpoint()` | ❌ | ❌ | Not called |
| `select_method()` | ❌ | ❌ | Not called |
| `check_health()` | ❌ | ❌ | Not called |
| `publish_payment_endpoint()` | ❌ | ❌ | Not called |
| `fetch_supported_payments()` | ❌ | ❌ | Not called |
| `create_subscription()` | ❌ | ❌ | Not called |
| `create_payment_request()` | ❌ | ❌ | Not called |
| Transport Operations | ❌ | ❌ | Not integrated |

---

## 5. Feature Parity Gap Analysis

### Priority 1: Critical Gaps (High Impact)

#### Mobile Demos - FFI Integration

| Gap | iOS | Android | Effort | Impact | Priority |
|-----|-----|---------|--------|--------|----------|
| Wire PaykitClient to UI | ❌ | ❌ | Medium | High | **P1** |
| Real payment method listing | ❌ | ❌ | Low | High | **P1** |
| Real health monitoring | ❌ | ❌ | Low | Medium | **P1** |
| Directory operations (real transport) | ❌ | ❌ | High | High | **P1** |
| Payment method selection | ❌ | ❌ | Medium | Medium | **P1** |

**Estimated Effort**: 2-3 weeks per platform

#### Web Demo - Directory Publishing

| Gap | Status | Effort | Impact | Priority |
|-----|--------|--------|--------|----------|
| Real Pubky homeserver publishing | ❌ Mock | Medium | Medium | **P1** |

**Estimated Effort**: 1 week (requires CORS proxy or homeserver config)

---

### Priority 2: Important Gaps (Medium Impact)

#### Mobile Demos - Missing Features

| Gap | iOS | Android | Effort | Impact | Priority |
|-----|-----|---------|--------|--------|----------|
| Payment request persistence | ❌ | ❌ | Low | Medium | **P2** |
| Receipt generation/verification | ❌ | ❌ | Medium | Medium | **P2** |
| Subscription signing/verification | ❌ | ❌ | Medium | Low | **P2** |
| Noise channel integration | ❌ | ❌ | High | High | **P2** |

**Estimated Effort**: 3-4 weeks per platform

#### Web Demo - Missing Features

| Gap | Status | Effort | Impact | Priority |
|-----|--------|--------|--------|----------|
| Remove payment endpoint | ❌ | Low | Low | **P2** |
| Rate limiting for handshakes | ❌ | Medium | Low | **P2** |

**Estimated Effort**: 1 week

---

### Priority 3: Nice-to-Have (Low Impact)

| Gap | Platforms | Effort | Impact | Priority |
|-----|-----------|--------|--------|----------|
| Multiple identity support (mobile) | iOS, Android | Medium | Low | **P3** |
| Payment execution (web/mobile) | Web, iOS, Android | High | Low | **P3** |
| Advanced method selection strategies | All | Low | Low | **P3** |

---

## 6. Comprehensive Action Plan

### Phase 1: Mobile FFI Integration (Priority 1)

**Goal**: Wire up `PaykitClient` FFI bindings to mobile UI

#### iOS Tasks

1. **Payment Methods Integration**
   - [ ] Update `PaymentMethodsView.swift` to call `PaykitClient.listMethods()`
   - [ ] Replace static list with real FFI data
   - [ ] Add loading states and error handling
   - [ ] **Estimated**: 2 days

2. **Health Monitoring Integration**
   - [ ] Update `PaymentMethodsView.swift` to call `PaykitClient.checkHealth()`
   - [ ] Display real health status from FFI
   - [ ] Add periodic health checks
   - [ ] **Estimated**: 1 day

3. **Directory Operations Integration**
   - [x] Create configurable transport (mock/callback)
   - [x] Add `DirectoryTransportMode` enum
   - [x] Document `PubkyUnauthenticatedStorageCallback` interface
   - [x] **Completed**: Phase 3

4. **Payment Method Selection**
   - [x] Add UI for method selection
   - [x] Call `PaykitClient.selectMethod()` from FFI
   - [x] Display selection results
   - [x] **Completed**: Phase 1

**Total iOS Effort**: ✅ Complete (Phases 1 & 3)

#### Android Tasks

1. **Payment Methods Integration**
   - [x] Update `PaymentMethodsScreen.kt` to call `PaykitClient.listMethods()`
   - [x] Replace static list with real FFI data
   - [x] Add loading states and error handling
   - [x] **Completed**: Phase 2

2. **Health Monitoring Integration**
   - [ ] Update `PaymentMethodsScreen.kt` to call `PaykitClient.checkHealth()`
   - [ ] Display real health status from FFI
   - [ ] Add periodic health checks
   - [ ] **Estimated**: 1 day

3. **Directory Operations Integration**
   - [x] Create configurable transport (mock/callback)
   - [x] Add `DirectoryTransportMode` sealed class
   - [x] Document `PubkyUnauthenticatedStorageCallback` interface
   - [x] **Completed**: Phase 3

4. **Payment Method Selection**
   - [x] Add UI for method selection
   - [x] Call `PaykitClient.selectMethod()` from FFI
   - [x] Display selection results
   - [x] **Completed**: Phase 2

**Total Android Effort**: ✅ Complete (Phases 2 & 3)

---

### Phase 2: Web Demo Real Publishing (Priority 1) - ✅ COMPLETE

**Goal**: Replace mock publishing with real Pubky homeserver integration

#### Tasks

1. **CORS Proxy Setup**
   - [x] Create proxy mode in DirectoryClient
   - [x] Document proxy setup in README
   - [x] **Completed**: Phase 4

2. **Real Publishing Implementation**
   - [x] Add configurable publishing modes (Mock, Direct, Proxy)
   - [x] Implement real HTTP PUT via `publishEndpoint()`
   - [x] Add authentication token handling
   - [x] Add error handling with detailed result messages
   - [x] **Completed**: Phase 4

3. **Remove Endpoint Support**
   - [x] Add `removeEndpoint()` functionality in DirectoryClient
   - [x] Add `unpublishFromDirectory()` for bulk removal
   - [x] **Completed**: Phase 4

**Total Web Effort**: ✅ Complete (Phase 4)

---

### Phase 3: Mobile Payment Requests & Receipts (Priority 2)

**Goal**: Add real payment request and receipt functionality to mobile demos

#### iOS Tasks

1. **Payment Request Persistence**
   - [ ] Store payment requests in Keychain
   - [ ] Integrate with `PaykitClient.createPaymentRequest()`
   - [ ] Add request lifecycle management
   - [ ] **Estimated**: 2 days

2. **Receipt Generation**
   - [ ] Integrate receipt generation from FFI
   - [ ] Add receipt verification
   - [ ] Update receipt storage to use real receipts
   - [ ] **Estimated**: 2 days

#### Android Tasks

1. **Payment Request Persistence**
   - [ ] Store payment requests in EncryptedSharedPreferences
   - [ ] Integrate with `PaykitClient.createPaymentRequest()`
   - [ ] Add request lifecycle management
   - [ ] **Estimated**: 2 days

2. **Receipt Generation**
   - [ ] Integrate receipt generation from FFI
   - [ ] Add receipt verification
   - [ ] Update receipt storage to use real receipts
   - [ ] **Estimated**: 2 days

**Total Mobile Effort**: ~8 days (4 per platform)

---

### Phase 4: README Updates

**Goal**: Ensure all READMEs are current and optimal

#### Tasks

1. **CLI Demo README**
   - [x] Already comprehensive and current
   - [ ] Minor: Add note about payment execution requirements

2. **Web Demo README**
   - [x] Already comprehensive and current
   - [ ] Update: Document CORS proxy requirement for real publishing

3. **iOS Demo README**
   - [x] Already current
   - [ ] Update: Add section on FFI integration status
   - [ ] Update: Document which features use real vs mock implementations

4. **Android Demo README**
   - [x] Already current
   - [ ] Update: Add section on FFI integration status
   - [ ] Update: Document which features use real vs mock implementations

5. **Mobile FFI README**
   - [x] Already comprehensive
   - [ ] Update: Add examples of real usage vs current mock usage

**Total README Effort**: ~2 days

---

## 7. Implementation Recommendations

### 7.1 Mobile FFI Integration Pattern

**Recommended Approach**:

```swift
// iOS Example
class PaymentMethodsViewModel: ObservableObject {
    private let paykitClient: PaykitClient
    
    @Published var methods: [PaymentMethod] = []
    @Published var healthStatus: [String: HealthStatus] = [:]
    
    init() {
        self.paykitClient = PaykitClient()
        loadMethods()
        checkHealth()
    }
    
    func loadMethods() {
        let methods = paykitClient.listMethods()
        // Convert to UI models
        self.methods = methods.map { methodId in
            PaymentMethod(id: methodId, health: healthStatus[methodId])
        }
    }
    
    func checkHealth() {
        let results = paykitClient.checkHealth()
        for result in results {
            healthStatus[result.methodId] = result.status
        }
    }
}
```

### 7.2 Directory Transport Integration

**Recommended Approach**:

1. Create a Pubky session wrapper that implements `AuthenticatedTransportFFI`
2. Use real Pubky SDK for authenticated operations
3. Use `PubkyUnauthenticatedTransport` for read operations
4. Handle errors gracefully with user-friendly messages

### 7.3 Web Publishing Solution

**Recommended Approach**:

1. **Option A**: Use a CORS proxy service (e.g., `https://cors-anywhere.herokuapp.com/`)
2. **Option B**: Configure Pubky homeserver with CORS headers
3. **Option C**: Use a backend proxy endpoint

**Recommendation**: Option B (configure homeserver) for production demos, Option A for quick testing

---

## 8. Testing Strategy

### 8.1 Mobile FFI Integration Tests

1. **Unit Tests**
   - Test FFI binding calls
   - Test data conversion (FFI types to UI models)
   - Test error handling

2. **Integration Tests**
   - Test real Pubky transport integration
   - Test payment method discovery
   - Test health monitoring

3. **UI Tests**
   - Test loading states
   - Test error displays
   - Test real data display

### 8.2 Web Publishing Tests

1. **Unit Tests**
   - Test HTTP PUT to homeserver
   - Test authentication handling
   - Test error scenarios

2. **Integration Tests**
   - Test end-to-end publishing flow
   - Test discovery after publishing
   - Test CORS handling

---

## 9. Success Metrics

### Phase 1 Success Criteria

- [ ] iOS: Payment methods UI shows real data from FFI
- [ ] iOS: Health monitoring shows real status
- [ ] iOS: Directory operations use real transport
- [ ] Android: Payment methods UI shows real data from FFI
- [ ] Android: Health monitoring shows real status
- [ ] Android: Directory operations use real transport
- [ ] Web: Real publishing to Pubky homeserver works

### Phase 2 Success Criteria

- [ ] Mobile: Payment requests are persisted and functional
- [ ] Mobile: Receipt generation works end-to-end
- [ ] All: READMEs updated with current status

### Overall Success Criteria

- [ ] All demos have feature parity for core functionality
- [ ] All mock implementations are either real or appropriately documented
- [ ] All READMEs are current and accurate
- [ ] All library features are utilized where appropriate

---

## 10. Timeline Estimate

### Phase 1: Mobile FFI Integration
- **Duration**: 3-4 weeks
- **Resources**: 1 developer per platform (iOS/Android)

### Phase 2: Web Real Publishing
- **Duration**: 1 week
- **Resources**: 1 developer

### Phase 3: Mobile Payment Requests & Receipts
- **Duration**: 2 weeks
- **Resources**: 1 developer per platform

### Phase 4: README Updates
- **Duration**: 2 days
- **Resources**: 1 developer

**Total Estimated Timeline**: 6-7 weeks with parallel development

---

## 11. Risk Assessment

### High Risk

1. **Pubky Transport Integration** (Mobile)
   - **Risk**: Complex integration, may require Pubky SDK updates
   - **Mitigation**: Start with mock transport, gradually replace with real

2. **CORS Issues** (Web)
   - **Risk**: Browser security restrictions
   - **Mitigation**: Use proxy or configure homeserver properly

### Medium Risk

1. **FFI Performance** (Mobile)
   - **Risk**: FFI calls may be slow, blocking UI
   - **Mitigation**: Use async/await, show loading states

2. **Error Handling** (All)
   - **Risk**: Complex error scenarios not handled
   - **Mitigation**: Comprehensive error handling, user-friendly messages

---

## 12. Conclusion

The Paykit demo applications are in good shape overall, with the CLI demo serving as the reference implementation. The main gaps are:

1. **Mobile demos** need FFI integration to use real Paykit functionality
2. **Web demo** needs real directory publishing (currently mocked)
3. **All demos** could better utilize library features

The plan outlined above provides a clear path to feature parity and optimal demo capability. The estimated timeline of 6-7 weeks is reasonable with proper resource allocation.

**Next Steps**:
1. Review and approve this plan
2. Prioritize phases based on business needs
3. Assign developers to each phase
4. Begin Phase 1 implementation

---

## Appendix A: Feature Checklist

### CLI Demo
- [x] Identity Management
- [x] Contact Management
- [x] Directory Operations
- [x] Payment Operations
- [x] Subscriptions
- [x] Auto-Pay
- [x] Receipts

### Web Demo
- [x] Identity Management
- [x] Contact Management
- [x] Directory Operations (configurable Mock/Direct/Proxy modes)
- [x] Payment Operations
- [x] Subscriptions
- [x] Auto-Pay
- [x] Receipts

### iOS Demo
- [x] Identity Management
- [x] Contact Management
- [x] Directory Operations (configurable mock/callback transport)
- [ ] Payment Operations (not implemented)
- [x] Subscriptions (storage only)
- [x] Auto-Pay
- [x] Receipts (storage only)

### Android Demo
- [x] Identity Management
- [x] Contact Management
- [x] Directory Operations (configurable mock/callback transport)
- [ ] Payment Operations (not implemented)
- [x] Subscriptions (storage only)
- [x] Auto-Pay
- [x] Receipts (storage only)

---

## Appendix B: Library Feature Utilization

### paykit-lib
- [x] Transport traits (CLI, Web)
- [x] Directory operations (CLI, Web)
- [x] Payment executors (CLI only)
- [x] Health monitoring (CLI, Web)
- [x] Method selection (CLI, Web)

### paykit-interactive
- [x] Noise channels (CLI, Web)
- [x] Receipt generation (CLI, Web)
- [x] Interactive manager (CLI, Web - partial)

### paykit-subscriptions
- [x] Subscriptions (All)
- [x] Payment requests (CLI, Web)
- [x] Auto-pay (All)
- [x] Spending limits (All)
- [x] Signing/verification (CLI only)

### paykit-mobile FFI
- [x] Key management (iOS, Android)
- [ ] PaykitClient (iOS, Android - created but unused)
- [ ] Directory operations (iOS, Android - mock)
- [ ] Payment operations (iOS, Android - not implemented)

