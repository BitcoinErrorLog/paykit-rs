# Paykit Demo Feature Parity Implementation Plan

This document outlines the concrete steps to bring all Paykit demo apps to feature parity with rich, real functionality.

---

## Quick Reference: Current State

| Demo | Real | Mock | Parity Score |
|------|------|------|--------------|
| CLI | 85% | 15% | 🟢 Reference |
| Web | 75% | 25% | 🟢 Good |
| iOS | 30% | 70% | 🔴 Needs Work |
| Android | 30% | 70% | 🔴 Needs Work |

---

## Phase 1: Foundation (5 days)

### Task 1.1: Update Mobile READMEs ✏️
**Effort**: 0.5 days | **Priority**: P0

- [ ] iOS README: Add "Real vs Mock Features" section at top
- [ ] iOS README: Update setup instructions for current build flow
- [ ] iOS README: Document KeyManager.swift usage
- [ ] Android README: Add "Real vs Mock Features" section at top
- [ ] Android README: Document KeyManager.kt usage
- [ ] Both: Add "Security Model" section explaining Keychain/EncryptedPrefs

### Task 1.2: Wire PaykitClient to Payment Methods ⚡
**Effort**: 1 day | **Priority**: P0

**iOS Changes:**
```swift
// PaymentMethodsView.swift
- Replace static `methods` list with:
  let methods = try paykitClient.listMethods()
  
- Replace static `healthResults` with:
  let health = try paykitClient.checkHealth()
  
- Wire "Validate" button to:
  let valid = try paykitClient.validateEndpoint(methodId, endpoint)
  
- Wire method selection to:
  let result = try paykitClient.selectMethod(methods, amount, prefs)
```

**Android Changes:**
```kotlin
// PaymentMethodsScreen.kt
- Replace static methods list with FFI call
- Replace static healthResults with FFI call
- Wire validation to FFI
- Wire selection to FFI
```

### Task 1.3: Add Contact Management to Mobile 👥
**Effort**: 2 days | **Priority**: P1

**New Files:**
- iOS: `ContactsView.swift`, `ContactsViewModel.swift`, `Contact.swift`
- Android: `ContactsScreen.kt`, `ContactsViewModel.kt`, `Contact.kt`

**Features:**
- [ ] List contacts from secure storage
- [ ] Add contact with name and pubkey
- [ ] Delete contact
- [ ] Search/filter contacts
- [ ] Copy pubkey to clipboard
- [ ] Show contact's payment methods (via directory)

**Navigation:**
- Add "Contacts" tab between "Methods" and "Subscriptions"

### Task 1.4: Initialize PaykitClient Properly 🔧
**Effort**: 0.5 days | **Priority**: P0

**iOS:**
```swift
// PaykitDemoApp.swift
- Replace placeholder() with real PaykitClient initialization
- Handle initialization errors gracefully
- Display error state in UI if client fails
```

**Android:**
```kotlin
// PaykitDemoApp.kt
- Initialize PaykitClient in Application.onCreate()
- Provide via DI or companion object
- Handle native library loading errors
```

---

## Phase 2: Storage & Persistence (5 days)

### Task 2.1: Subscription Storage 📋
**Effort**: 2 days | **Priority**: P1

**New Files:**
- iOS: `SubscriptionStorage.swift`
- Android: `SubscriptionStorage.kt`

**Implementation:**
```
Storage Key: "paykit.subscriptions"
Format: JSON array of subscription objects

Operations:
- saveSubscription(sub: Subscription)
- listSubscriptions() -> [Subscription]
- getSubscription(id: String) -> Subscription?
- deleteSubscription(id: String)
- updateSubscription(sub: Subscription)
```

**UI Wire-up:**
- Replace `loadSampleSubscriptions()` with real storage load
- "Create" saves to storage
- "Delete" removes from storage
- List refreshes from storage

### Task 2.2: Auto-Pay Storage 🤖
**Effort**: 1.5 days | **Priority**: P1

**Storage Keys:**
- `paykit.autopay.enabled` - Boolean
- `paykit.autopay.dailyLimit` - Long
- `paykit.autopay.usedToday` - Long
- `paykit.autopay.rules` - JSON array
- `paykit.autopay.peerLimits` - JSON object (peer -> limit)

**Implementation:**
- Persist all auto-pay settings to Keychain/EncryptedPrefs
- Load on app start
- Update on every change
- Reset usage at midnight (or on demand)

### Task 2.3: Receipt Storage 🧾
**Effort**: 1.5 days | **Priority**: P2

**New Files:**
- iOS: `ReceiptStorage.swift`
- Android: `ReceiptStorage.kt`

**Implementation:**
```
Storage Key: "paykit.receipts"
Format: JSON array of receipt objects

Operations:
- saveReceipt(receipt: Receipt)
- listReceipts() -> [Receipt]
- getReceipt(id: String) -> Receipt?
- deleteReceipt(id: String)
- filterByDirection(sent/received)
- filterByMethod(methodId)
- exportAsJson() -> String
```

---

## Phase 3: Dashboard & UX (3 days)

### Task 3.1: Dashboard Overview 📊
**Effort**: 2 days | **Priority**: P2

**New Files:**
- iOS: `DashboardView.swift`, `DashboardViewModel.swift`
- Android: `DashboardScreen.kt`, `DashboardViewModel.kt`

**Features:**
- [ ] Contact count
- [ ] Payment method count
- [ ] Receipt count (sent/received)
- [ ] Active subscription count
- [ ] Recent activity feed (last 10 events)
- [ ] Quick action buttons
- [ ] Setup progress checklist

**Make Dashboard the first tab** (before Payment Methods)

### Task 3.2: Real Payment Requests 💸
**Effort**: 1 day | **Priority**: P2

**Changes:**
- Replace `loadSampleData()` in PaymentRequestsView with real storage
- Store payment requests in Keychain/EncryptedPrefs
- Implement create/accept/decline with persistence
- Add expiration handling

---

## Phase 4: Advanced Integration (10 days)

### Task 4.1: Directory Publishing 📡
**Effort**: 3 days | **Priority**: P2

**Requires:**
- Transport FFI properly exposed
- User has authenticated Pubky session

**Implementation:**
1. Add "Publish" button to Payment Methods screen
2. Show publish dialog with endpoint input
3. Call FFI `publish_payment_endpoint(transport, methodId, endpoint)`
4. Display success/error feedback
5. Add "Published" indicator on methods

**Challenges:**
- Need authenticated transport (requires Pubky session management)
- May need to stub with mock initially

### Task 4.2: Noise Protocol Integration 🔐
**Effort**: 7 days | **Priority**: P3

**This is the most complex integration:**

1. **FFI Exposure** (2 days)
   - Expose NoiseClient/NoiseServer through UniFFI
   - Handle handshake state in FFI layer
   - Manage encrypted message send/receive

2. **Mobile Transport** (2 days)
   - Implement WebSocket transport for mobile
   - Handle connection lifecycle
   - Manage reconnection

3. **Payment Flow** (3 days)
   - Implement send payment UI
   - Implement receive payment (background service?)
   - Receipt exchange and storage

**Recommendation:** Defer this to Phase 5 or separate project

---

## Phase 5: Polish & Verification (3 days)

### Task 5.1: Feature Parity Test 🧪
**Effort**: 1 day | **Priority**: P1

Create manual test checklist:
- [ ] All demos can generate identity
- [ ] All demos can export/import backup
- [ ] All demos show real payment methods
- [ ] All demos persist subscriptions
- [ ] All demos persist auto-pay settings
- [ ] All demos have contact management
- [ ] All demos have dashboard

### Task 5.2: Documentation Sync 📝
**Effort**: 1 day | **Priority**: P1

- [ ] Update all READMEs with current features
- [ ] Create DEMO_COMPARISON.md (feature matrix)
- [ ] Update CHANGELOG.md
- [ ] Create video walkthrough of each demo

### Task 5.3: CLI & Web Gaps 🔄
**Effort**: 1 day | **Priority**: P2

**CLI:**
- [ ] Add "simulation mode" warnings more prominently
- [ ] Consider adding real payment execution if feasible

**Web:**
- [ ] Consider real Pubky homeserver publishing
- [ ] Document WebSocket relay server requirements

---

## Implementation Schedule

```
Week 1: Phase 1 (Foundation)
├── Day 1-2: Update READMEs, Wire PaykitClient
├── Day 3-4: Add Contact Management
└── Day 5: Initialize PaykitClient properly

Week 2: Phase 2 (Storage)
├── Day 1-2: Subscription Storage
├── Day 3: Auto-Pay Storage
└── Day 4-5: Receipt Storage

Week 3: Phase 3 (UX)
├── Day 1-2: Dashboard
└── Day 3: Real Payment Requests

Week 4-5: Phase 4 (Advanced)
├── Week 4: Directory Publishing
└── Week 5: Noise Protocol (stretch goal)

Week 6: Phase 5 (Polish)
├── Day 1: Feature Parity Test
├── Day 2: Documentation Sync
└── Day 3: Final fixes
```

---

## Success Metrics

After implementation, each demo should:

| Metric | Target |
|--------|--------|
| Real features | > 80% |
| Mock features | < 20% |
| Core features working | 100% |
| Documented accurately | 100% |
| Feature parity | 90%+ |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Noise FFI complexity | High | High | Defer to Phase 5, focus on storage first |
| Pubky auth integration | Medium | Medium | Use mock transport initially |
| Platform-specific bugs | Medium | Low | Thorough testing on both platforms |
| Scope creep | High | Medium | Strict phase gates |

---

## Next Steps

1. **Immediate**: Approve this plan
2. **Today**: Start Task 1.1 (Update READMEs)
3. **This Week**: Complete Phase 1
4. **Review**: After each phase, review and adjust

---

*Created: December 2024*
*Owner: Engineering Team*

