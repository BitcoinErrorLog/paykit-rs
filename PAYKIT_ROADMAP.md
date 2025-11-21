# Paykit Roadmap & Integration Plan

This document outlines the plan to advance Paykit from a public directory tool to a full interactive payment protocol using Pubky Noise, replacing the legacy mock implementation in the original Bitkit.

## 🟢 Phase 1: Public Directory & Rotation (Part A)

**Goal**: Replicate original Bitkit's "Payment Profile" features using real Pubky infrastructure, including automatic endpoint rotation.

### Status
- `paykit-lib`: **Complete** (implements public directory traits & adapters).
- `bitkit-core`: **Complete** (module structure created, rotation logic implemented).
- Mobile Apps: **Pending** UI/UX.

### Action Items
1.  **Method Standardization**:
    - Formalize method IDs:
        - `"onchain"`: Bitcoin address string.
        - `"lightning"`: Zero-amount BOLT11 invoice.
2.  **Endpoint Rotation Logic (`bitkit-core`)**:
    - ✅ Implement a background monitor that checks usage.
    - ✅ Expose `paykit_check_rotation_needed(pubkey)` via FFI.
3.  **bitkit-core Integration**:
    - ✅ Add `paykit-lib` dependency.
    - ✅ Create `src/paykit/mod.rs` wrapper.
    - ✅ Expose async functions for manual management.
    - ✅ Expose `paykit_rotate_endpoints()` (implemented as check).
4.  **Mobile Integration**:
    - **Profile UI**:
        - Display "Payment Profile" QR code (Pubky URI).
        - Toggles for "Enable Public On-chain" and "Enable Public Lightning".
    - **Scan Flow**:
        - Scanning a Pubky QR automatically queries `paykit_get_supported_methods_for_key`.

---

## 🟡 Phase 2: Interactive Layer Foundation (Part B)

**Goal**: Scaffolding the new `paykit-interactive` crate to support private endpoints and receipts.

### Status
- `paykit-interactive`: **Scaffolding Complete**.
- Types `PaykitReceipt` and `PaykitNoiseMessage` implemented.

### Action Items
1.  **Refine Data Structures**:
    - Ensure `PaykitReceipt` JSON schema handles both on-chain (txid) and lightning (preimage) proofs.
2.  **Dependency wiring**:
    - ✅ Depend on `paykit-lib`.
    - ✅ Depend on `pubky-noise`.

---

## 🟢 Phase 3: Pubky Noise Integration

**Goal**: Connect `paykit-interactive` with `pubky-noise` to enable real encrypted communication.

### Status
- **COMPLETE**: All interactive layer components implemented and tested.
- `PaykitNoiseChannel` trait defined & implemented.
- `PaykitInteractiveManager` implemented (State Machine).
- `PaykitStorage` & `ReceiptGenerator` traits defined.
- SQLite storage implementation in `bitkit-core`.
- FFI wrappers for interactive payment flow implemented in `bitkit-core`.
- Comprehensive test suite with mock implementations.
- Complete end-to-end example provided.
- Timeout handling added (30s for receipt negotiation).

### Deliverables
1.  **Integrate `pubky-noise`**:
    - ✅ Use `pubky-noise` for the underlying secure channel.
    - ✅ Implement `PaykitNoiseChannel::connect(payer, payee)`.
    - ✅ Verified Noise_IK handshake semantics (1-RTT pattern).
    - ✅ Updated documentation to explain handshake flow.
2.  **Implement Logic**:
    - ✅ **Private Endpoint Store**: Defined `PaykitStorage` trait and `OfferPrivateEndpoint` handler.
    - ✅ **Interactive Flow**: Implemented `PaykitInteractiveManager` state machine.
    - ✅ **Timeout Logic**: 30-second timeout for receipt negotiation (configurable via feature flag).
    - ✅ **Error Handling**: Proper error messages for all failure cases.
3.  **Storage**:
    - ✅ Defined `PaykitStorage` trait.
    - ✅ Implemented `BitkitPaykitStorage` in `bitkit-core/src/modules/paykit/storage.rs`.
    - ✅ SQLite tables for receipts and private endpoints.
4.  **Testing**:
    - ✅ Mock implementations for `PaykitStorage`, `ReceiptGenerator`, and `PaykitNoiseChannel`.
    - ✅ Integration tests for complete payment flow.
    - ✅ Tests for error cases (wrong payee, receipt ID mismatch, timeout).
    - ✅ Example: `paykit-interactive/examples/complete_payment_flow.rs`.
5.  **FFI & Mobile Integration**:
    - ✅ FFI Types (`PaykitReceiptFfi`) in `bitkit-core`.
    - ✅ FFI Facade (`PaykitInteractive`) in `bitkit-core`.
    - ✅ TCP + Noise channel management in FFI layer.
    - ✅ Build scripts (`build_ios.sh`, `build_android.sh`) validated/updated.
    - ✅ `BINDINGS_INSTRUCTIONS.md` created for mobile team.

---

## 🟣 Phase 4: Checkout & Receipts UI

**Goal**: User-facing checkout experiences in Bitkit, moving beyond simple public addresses.

### Status
- **Backend (bitkit-core)**: ✅ Complete (Smart checkout logic, FFI exports, Scanner integration).
- **Frontend Integration**: 🚧 Pending Mobile Implementation (Integration guides provided).

### Action Items
1.  **Smart Checkout Flow**:
    - ✅ **Step 1**: Resolve contact/QR. (Implemented in `Scanner` with `PubkyPayment`)
    - ✅ **Step 2**: Check **Private Offer** (preferred). (Implemented in `paykit_smart_checkout` with storage wiring)
    - ✅ **Step 3**: Fallback to **Public Directory**. (Implemented fallback logic)
    - ✅ **Step 4**: Payment & Receipt. (FFI structures ready)
2.  **Receipts History**:
    - 🚧 Transaction history linking delegated to mobile teams (see integration guides).

---

## Architecture

```mermaid
graph TD
    A[Bitkit App] --> B[bitkit-core]
    B --> C[paykit-lib]
    B --> D[paykit-interactive]
    B --> H[Rotation Monitor]
    C --> E[Public Directory (Pubky)]
    D --> F[pubky-noise]
    F --> G[Private Peer Channel]
    H -- Watches --> I[Wallet State]
    H -- Updates --> C
```
