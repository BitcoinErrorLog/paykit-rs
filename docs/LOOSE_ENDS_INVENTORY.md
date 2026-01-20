# Paykit-rs Loose Ends Inventory

**Created**: January 2026  
**Purpose**: Comprehensive inventory and triage of TODO/placeholder/spec-drift items for PUBKY_CRYPTO_SPEC v2.5 alignment

---

## Triage Categories

| Category | Description |
|----------|-------------|
| **IMPLEMENT** | Must implement as part of spec alignment |
| **DEPRECATE** | Remove or feature-gate; not compliant with spec |
| **OUT-OF-SCOPE** | Intentionally not implemented; documented as app-layer concern |
| **DEFERRED** | Low priority; implement in future release |

---

## 1. Critical: Cryptographic Primitives

### 1.1 AAD Format Mismatch

**Location**: `paykit-lib/src/protocol/aad.rs`

**Current**:
```rust
// String-based AAD format
"paykit:v0:{purpose}:{owner_z32}:{path}:{id}"
```

**CRYPTO_SPEC v2.5 Requirement** (Section 7.5):
```
aad = aad_prefix || owner_peerid_bytes || canonical_path_bytes || header_bytes
// Where aad_prefix = "pubky-envelope/v2:" (18 bytes)
```

**Triage**: **IMPLEMENT**  
**Action**: Add binary AAD builder alongside legacy string AAD; migrate all encrypted objects to binary AAD.

---

### 1.2 ContextId Derivation Mismatch

**Location**: `paykit-lib/src/protocol/scope.rs`

**Current**:
```rust
// Deterministic derivation from peer pubkeys
context_id(pubkey_a, pubkey_b) = SHA256("paykit:v0:context:" + first_z32 + ":" + second_z32)
```

**CRYPTO_SPEC v2.5 Requirement** (Section 7.7):
- 32 random bytes chosen by thread initiator
- NOT derived from peer keys
- PairContextId (from sorted keys) is for diagnostics ONLY

**Triage**: **IMPLEMENT**  
**Action**: 
1. Rename current `context_id()` → `pair_context_id()` for diagnostics only
2. Add `random_context_id()` for thread-level ContextId
3. Update storage paths to use random ContextId
4. Implement bounded discovery without PairContextId in paths

---

### 1.3 Sealed Blob Format (SB2)

**Location**: `paykit-subscriptions/src/discovery.rs` (imports from `pubky_noise::sealed_blob`)

**Current**:
- Uses JSON envelope: `{"v": 2, "epk": "...", "nonce": "...", "ct": "..."}`
- Via `sealed_blob_encrypt()` / `sealed_blob_decrypt()` from pubky-noise

**CRYPTO_SPEC v2.5 Requirement** (Section 7.2):
- Binary wire format: `SB2` magic + version + header_len (u16 BE) + CBOR header + ciphertext
- Deterministic CBOR header with integer keys (0-11)
- Resource bounds: header_len <= 2048, max 16 keys, depth <= 2

**Triage**: **IMPLEMENT**  
**Action**: 
1. Implement SB2 binary format in pubky-noise (or consume when available)
2. Keep legacy JSON envelope read support for migration
3. Update paykit-rs to use SB2 for all new encrypted objects

---

### 1.4 InboxKey/TransportKey Separation

**Locations**: 
- `paykit-mobile/src/noise_ffi.rs` (publishes/discovers `/pub/paykit.app/v0/noise`)
- `paykit-subscriptions/src/discovery.rs` (encrypts to `recipient_noise_pk`)

**Current**:
- Stored delivery encrypts to Noise endpoint (TransportKey)
- `/pub/paykit.app/v0/noise` is used for both purposes

**CRYPTO_SPEC v2.5 Requirement** (Section 4.7):
- **InboxKey**: X25519 for Sealed Blob stored delivery ONLY
- **TransportKey**: X25519 for Noise sessions ONLY
- Keys MUST be distinct

**Triage**: **IMPLEMENT**  
**Action**:
1. Add InboxKey derivation and publication
2. Update stored delivery to use InboxKey
3. Keep `/pub/paykit.app/v0/noise` as legacy transport-only

---

### 1.5 inbox_kid Header Field

**Location**: Not currently implemented

**CRYPTO_SPEC v2.5 Requirement** (Section 7.2):
- `inbox_kid` = `first_16_bytes(SHA256(inbox_x25519_pub))`
- REQUIRED field in SB2 header (key 3)
- Unknown `inbox_kid` MUST be rejected WITHOUT calling Ring derivation

**Triage**: **IMPLEMENT**  
**Action**: Add `inbox_kid` computation and validation in sealed blob handling.

---

### 1.6 Signature in Sealed Blob Headers

**Location**: Not currently implemented

**CRYPTO_SPEC v2.5 Requirement** (Section 7.2.1):
- `sig` field (key 10) is Ed25519 signature
- Computed over BLAKE3("pubky-envelope-sig/v2" || aad || header_no_sig || ciphertext)
- REQUIRED for Paykit purposes (request, proposal, ack)

**Triage**: **IMPLEMENT**  
**Action**: Add signature computation and verification to sealed blob handling.

---

## 2. Critical: Protocol Features

### 2.1 Encrypted ACK Protocol

**Location**: `docs/PAYKIT_PROTOCOL_V0.md` (line 63)

**Current**: "Encrypted ACK (specified, not yet implemented)"

**CRYPTO_SPEC v2.5 Requirement** (Section 7.9):
- ACKs are Sealed Blob v2 encrypted
- Encrypted to original sender's InboxKey
- ACK payload: `{ "acked_msg_id": "...", "error_code": 0, "error_text": null }`
- Storage path: `/pub/paykit.app/v0/acks/{object_type}/{context_id_z32}/{acked_msg_id}`

**Triage**: **IMPLEMENT**  
**Action**: 
1. Implement ACK creation and encryption
2. Add ACK polling and decryption
3. Implement resend defaults (1m/2m/4m/8m/16m)

---

### 2.2 KeyBinding Discovery

**Location**: Not currently implemented

**CRYPTO_SPEC v2.5 Requirement** (Section 6.8.1):
- KeyBinding object via PKARR with:
  - `inbox_keys[]` array
  - `transport_keys[]` array
  - `app_keys[]` array (optional)
  - Root signature

**Triage**: **IMPLEMENT**  
**Action**: Add KeyBinding schema types, PKARR fetch, and signature verification.

---

### 2.3 AppCert Delegation

**Location**: Not currently implemented

**DELEGATION_SPEC v0.2 Requirement**:
- AppCert (root-signed certificate) binding AppKey, TransportKey, InboxKey
- `cert_id` in Sealed Blob headers for delegated signing
- SignedContent envelope for proof-of-authorship

**Triage**: **IMPLEMENT**  
**Action**: Add AppCert schema, parsing, and delegated signature verification.

---

## 3. Critical: Security

### 3.1 Generic "sign arbitrary bytes" API

**Location**: `paykit-mobile/src/keys.rs` (line 173)

**Current**:
```rust
#[uniffi::export]
pub fn sign_message(secret_key_hex: String, message: Vec<u8>) -> Result<String>
```

**CRYPTO_SPEC v2.5 Requirement** (Section 5.3.5):
- "Ring MUST NOT expose a generic 'sign arbitrary bytes' API"
- All signing operations MUST be typed and scoped to specific protocol transcripts
- Prohibited patterns: `sign(arbitrary_bytes)`, `sign_message(user_visible_string)`, `sign_challenge(challenge_bytes)`

**Triage**: **DEPRECATE**  
**Action**: Remove or feature-gate `sign_message()` and replace with typed signing surfaces.

---

### 3.2 Memory Zeroization

**Location**: `docs/SECURITY_ARCHITECTURE.md` (lines 628, 753, 758)

**Current**: TODOs for memory zeroization

**CRYPTO_SPEC v2.5 Requirement** (Section 11.1):
- Use `zeroize` crate for secret buffers
- Wrap secrets in `Zeroizing<[u8; 32]>`
- Minimize copies of key material

**Triage**: **IMPLEMENT**  
**Action**: Add zeroization for all secret material in paykit-mobile and paykit-lib.

---

### 3.3 Placeholder SHA256 in Preimage Verification

**Location**: `docs/SECURITY_ARCHITECTURE.md` (line 919)

**Current**: "Placeholder SHA256 in preimage verification" marked as high severity

**Triage**: **IMPLEMENT**  
**Action**: Replace placeholder with proper implementation per CRYPTO_SPEC.

---

## 4. Medium: Code Quality

### 4.1 Placeholder Message Sends

**Location**: `paykit-subscriptions/src/manager.rs` (lines 129, 179, 230)

**Current**:
```rust
channel.send(PaykitNoiseMessage::Ack)  // Placeholder
```

**Triage**: **IMPLEMENT**  
**Action**: Replace placeholder sends with real ACK protocol implementation.

---

### 4.2 Placeholder Health Checkers

**Location**: `paykit-lib/src/health/mod.rs`

**Current**: "placeholder implementation" for `OnchainHealthChecker` and `LightningHealthChecker`

**Triage**: **DEFERRED**  
**Action**: Implement real health checks in future release. Document as stub for now.

---

### 4.3 Unimplemented Error Variant

**Location**: `paykit-interactive/src/lib.rs`

**Current**: `Unimplemented` error variant exists

**Triage**: **IMPLEMENT**  
**Action**: Replace `Unimplemented` usages with real implementations or remove the variant.

---

## 5. Medium: Documentation

### 5.1 iOS Approval UI TODO

**Location**: `BITKIT_PAYKIT_INTEGRATION_MASTERGUIDE.md` (line 1843)

**Current**: "iOS currently has a TODO to show an approval UI for `.needsApproval`"

**Triage**: **OUT-OF-SCOPE**  
**Action**: This is a Bitkit iOS app concern, not paykit-rs. Document as app-layer responsibility.

---

### 5.2 Automatic Time-Based Rotation

**Location**: `BITKIT_PAYKIT_INTEGRATION_MASTERGUIDE.md` (line 3181)

**Current**: "Automatic time-based rotation is planned but not yet implemented"

**Triage**: **DEFERRED**  
**Action**: Key rotation is application-layer policy. Document recommended rotation intervals.

---

### 5.3 Paykit PDF Drift

**Location**: `docs/PAYKIT_PDF_DRIFT_REPORT.md`

**Current**: Lists discrepancies between older Paykit PDF spec and current codebase

**Triage**: **IMPLEMENT** (reconcile)  
**Action**: Update drift report to reflect CRYPTO_SPEC v2.5 alignment. Deprecate PDF references.

---

## 6. Low: Test Infrastructure

### 6.1 TODO: SessionManager for Directory Operations

**Location**: `paykit-demo-core/tests/test_directory_operations.rs` (line 9)

**Current**: `// TODO: Implement SessionManager for directory operations`

**Triage**: **DEFERRED**  
**Action**: Demo test infrastructure; implement when needed.

---

### 6.2 Doc Example Placeholders

**Location**: `paykit-subscriptions/src/signing.rs` (lines 144-145)

**Current**:
```rust
/// # let subscription = todo!();
/// # let keypair = todo!();
```

**Triage**: **IMPLEMENT**  
**Action**: Replace doc example placeholders with real examples.

---

### 6.3 Doc Example Placeholders in Methods

**Location**: `paykit-lib/src/methods/mod.rs` (lines 69, 74)

**Current**:
```rust
//!         todo!()
```

**Triage**: **IMPLEMENT**  
**Action**: Replace doc example placeholders with real examples.

---

## 7. Atomicity Integration

### 7.1 Settlement Adapter

**Location**: Not currently implemented

**Atomicity Specification Requirement** (Section 10):
- Settlement messages: `settlement_request`, `settlement_payment_request`, `settlement_proof`
- Integration with Paykit directory for payment method discovery

**Triage**: **IMPLEMENT**  
**Action**: Add settlement message schemas and Paykit-facing adapter.

---

## Summary

| Category | Count |
|----------|-------|
| **IMPLEMENT** | 16 |
| **DEPRECATE** | 1 |
| **OUT-OF-SCOPE** | 1 |
| **DEFERRED** | 3 |

### Priority Order

1. **P0 (Critical Crypto)**:
   - SB2 binary format adoption
   - AAD construction update
   - InboxKey/TransportKey separation
   - inbox_kid implementation
   - Header signature support

2. **P1 (Critical Protocol)**:
   - Encrypted ACK protocol
   - KeyBinding discovery
   - ContextId migration to random
   - Remove generic signing API

3. **P2 (Security Hardening)**:
   - Memory zeroization
   - Placeholder replacement
   - AppCert support

4. **P3 (Documentation)**:
   - Update all docs to reference CRYPTO_SPEC
   - Reconcile drift reports
   - Update test vectors

5. **P4 (Atomicity)**:
   - Settlement adapter
   - Integration docs
