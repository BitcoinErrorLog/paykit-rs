# Paykit Specification vs Implementation Analysis

**Date**: 2026-01-02  
**Author**: Claude Opus 4.5  
**Spec Reference**: `Paykit.pdf`  
**Codebase**: `paykit-rs/`, `bitkit-android/`, `bitkit-ios/`, `pubky-ring/`

---

## Executive Summary

The Paykit implementation has **diverged significantly** from the PDF specification in several fundamental ways. Most drifts are **pragmatic improvements**, but some create **interoperability gaps** and **cross-implementation inconsistencies** that require resolution.

### Critical Issues (Highest Priority)

| Issue | Severity | Recommendation |
|-------|----------|----------------|
| Bitkit vs paykit-rs disagree on request paths & encryption | **Critical** | Align on single canonical model |
| No fallback execution loop | **High** | Add `execute_with_fallbacks` API |
| iOS uses first-method-only, Android uses selection | **High** | Unify selection behavior |

### Beneficial Drifts (Keep)

| Drift | Why It's Better |
|-------|-----------------|
| Per-method files vs single JSON list | Atomic updates, no merge conflicts |
| Short method IDs vs URL identifiers | FFI-friendly, plugin dispatch works |
| Inline endpoint payloads vs URL indirection | Fewer round-trips, more reliable |
| Sealed Blob encryption vs "private URL" | Cryptographic security > obscurity |

---

## 1. Supported Payments List Schema

### PDF Specification

> "Read request to Paykit Routing Network with public key returns a Supported Payments List stored at default public path. This is an **array of objects** with one key being 'method' whose value is URL to Payment Method and another key is 'endpoint' with value being Payment Endpoint URL."

```json
[
  {
    "method": "paykit.standards.com/p2pkh",
    "endpoint": "payee-paykit-server.com/bitcoin/p2pkh"
  },
  {
    "method": "paykit.standards.com/lightning",
    "endpoint": "payee-paykit-server.com/bitcoin/bolt11"
  }
]
```

### Implementation Reality

Per-method files under `/pub/paykit.app/v0/{method_id}` with endpoint payload as file content:

```rust
// paykit-lib/src/transport/pubky/mod.rs
pub const PAYKIT_PATH_PREFIX: &str = "/pub/paykit.app/v0/";
// Convention: /pub/paykit.app/v0/{method_id} -> payload is the payment endpoint
```

Discovery works by **listing the directory** and reading each file:

```rust
// paykit-lib/src/transport/pubky/unauthenticated_transport.rs
async fn fetch_supported_payments(&self, payee: &PublicKey) -> Result<SupportedPayments> {
    let addr = format!("pubky{payee}{PAYKIT_PATH_PREFIX}");
    let entries = self.list_entries(addr, "list supported payments").await?;

    let mut map = HashMap::new();
    for resource in entries {
        let method = resource.path.as_str().rsplit('/').next()...;
        if let Some(payload) = self.fetch_text(resource.to_string(), &label).await? {
            map.insert(MethodId(method), EndpointData(payload));
        }
    }
    Ok(SupportedPayments { entries: map })
}
```

### Analysis

| Aspect | PDF | Implementation | Verdict |
|--------|-----|----------------|---------|
| Atomicity | Single file = atomic snapshot | Directory = N reads, possible mixed state | **Spec better** |
| Updateability | Must rewrite entire list | Update one method independently | **Impl better** |
| Merge conflicts | Possible with concurrent writers | None (per-file) | **Impl better** |
| Round-trips | 1 (fetch list) | 1 + N (list + N gets) | **Spec better** |
| Interoperability | Standard JSON schema | Custom directory convention | **Spec better** |

### Recommendation

**Keep per-method files**, but add an **optional snapshot file** at `/pub/paykit.app/v0/supported.json` for PDF-compatible clients:

```json
{
  "version": 1,
  "methods": [
    { "method_id": "lightning", "endpoint": "lnbc1..." },
    { "method_id": "onchain", "endpoint": "bc1q..." }
  ],
  "generated_at": 1704200000
}
```

Update the spec to document both storage models.

---

## 2. Payment Method Identifiers

### PDF Specification

Method identifiers are **URLs** pointing to method specifications:

> Example: `"method": "paykit.standards.com/p2pkh"`

### Implementation Reality

Method IDs are **opaque short strings**:

```rust
// paykit-lib/src/lib.rs
pub struct MethodId(pub String);

impl MethodId {
    pub const ONCHAIN: &'static str = "onchain";
    pub const LIGHTNING: &'static str = "lightning";
}
```

### Analysis

| Aspect | PDF (URLs) | Implementation (strings) |
|--------|-----------|--------------------------|
| Global uniqueness | Guaranteed (DNS-based) | Collision possible |
| Self-documenting | URL = spec location | Requires external registry |
| FFI friendliness | URL parsing overhead | Simple strings work everywhere |
| Plugin dispatch | String extraction needed | Direct key lookup |

### Recommendation

**Keep short IDs** as the primary key. Add optional `method_spec_uri` field for standards-level interoperability:

```rust
pub struct MethodInfo {
    pub id: MethodId,           // "lightning"
    pub spec_uri: Option<String>, // "https://paykit.standards.com/ln-btc"
}
```

Publish a **method registry document** so the ecosystem converges on naming.

---

## 3. Endpoint Representation

### PDF Specification

> "Payment Endpoint corresponds to the specific payee owned credentials/reference..."
> 
> The `endpoint` field contains a **URL** that returns UTF-8 payment data when fetched.

### Implementation Reality

Endpoint content is stored **inline** as file content—no URL indirection:

```rust
// paykit-mobile/src/lib.rs
pub struct PaymentMethod {
    pub method_id: String,
    pub endpoint: String,  // Direct payload, not a URL
}
```

Example: `/pub/paykit.app/v0/lightning` contains `lnbc1pj...` directly.

### Analysis

| Aspect | PDF (URL) | Implementation (inline) |
|--------|-----------|------------------------|
| Latency | 2 fetches (list → endpoint URL → content) | 1 fetch (list) + 1 read per method |
| Reliability | Depends on endpoint server uptime | Data is co-located |
| Dynamism | Endpoint can generate fresh data | Static until updated |
| Extensibility | Endpoint can return rich schemas | Schema must be spec'd per method |

### Recommendation

**Keep inline payloads** as default. Define **well-known schemas per method** to avoid "opaque string" becoming a trap:

```json
// /pub/paykit.app/v0/lightning
{
  "type": "bolt11",
  "value": "lnbc1pj...",
  "expires_at": 1704300000,
  "amount_sats": 10000
}
```

If dynamic endpoints are needed, allow `endpoint_url` as an optional redirect.

---

## 4. Private Payment Method Lists

### PDF Specification

> "Private Payment Method Lists...are:
> - Optionally encrypted
> - Only readable via the corresponding URL
> - Designed to maximize privacy"
>
> "The payee grants access to created data to whoever possesses access URL"

The PDF implies **security by URL obscurity** with optional encryption.

### Implementation Reality

No "private list by URL" abstraction exists. Privacy is handled via:

1. **Noise Protocol channels** for interactive payments
2. **Sealed Blob v1 encryption** for stored sensitive data
3. **Private endpoint manager** for per-peer local storage

```rust
// paykit-lib/src/private_endpoints/mod.rs
pub struct PrivateEndpointManager<S: PrivateEndpointStore> {
    store: Arc<S>,
    default_policy: EndpointPolicy,
}

// Endpoints are stored locally, encrypted, and managed per-peer
pub async fn store_endpoint(
    &self,
    peer: PublicKey,
    method_id: MethodId,
    endpoint: EndpointData,
    expires_at: Option<i64>,
) -> Result<()>
```

### Analysis

| Aspect | PDF (URL access) | Implementation (encrypted) |
|--------|------------------|---------------------------|
| Security model | Obscurity (URL = secret) | Cryptography (must have key) |
| Key distribution | Share URL out-of-band | Share noise pubkey |
| Access revocation | Change URL (breaks links) | Rotate keys, re-encrypt |
| Auditability | URL logging exposes access | Only ciphertext visible |

### Recommendation

**Do not implement URL-as-access-control.** The PDF's model is fundamentally weaker than what we have.

**Update the spec** to describe:
- Noise endpoint discovery at `/pub/paykit.app/v0/noise`
- Sealed Blob v1 format for encrypted storage
- Per-peer private endpoint exchange over encrypted channels

---

## 5. Payment Method Selection & Fallback Loop

### PDF Specification

> "5. The payer attempts to execute a payment
> 6. In case of failure - repeats from step 3 until the list from step 2 is empty."

The spec mandates an **automatic fallback loop**: try methods in order until one succeeds.

### Implementation Reality

Selection returns `primary_method` + `fallbacks`, but **execution is single-shot**:

```rust
// paykit-lib/src/selection/selector.rs
pub struct SelectionResult {
    pub primary: MethodId,
    pub fallbacks: Vec<MethodId>,
    pub score: f64,
    pub reason: String,
}

pub fn all_methods(&self) -> Vec<MethodId> {
    let mut methods = vec![self.primary.clone()];
    methods.extend(self.fallbacks.clone());
    methods
}
```

But PaykitMobile's `execute_payment` only runs **one method**:

```rust
// paykit-mobile/src/lib.rs
pub fn execute_payment(
    &self,
    method_id: String,  // Single method
    endpoint: String,
    amount_sats: u64,
    metadata_json: Option<String>,
) -> Result<PaymentExecutionResult>
```

Bitkit iOS uses **first discovered method only** (no selection at all for Paykit URIs):

```swift
// bitkit-ios/.../PaykitPaymentService.swift
let methods = try await directoryService.discoverPaymentMethods(for: pubkey)
guard let firstMethod = methods.first else { throw ... }
let result = try client.executePayment(methodId: firstMethod.methodId, ...)
```

Bitkit Android uses selection but still no fallback loop:

```kotlin
// bitkit-android/.../PaykitPaymentService.kt
val selectedMethod = selectOptimalMethod(pubkey, amount)
val result = client.executePayment(methodId = selectedMethod.methodId, ...)
```

### Analysis

| Aspect | PDF (fallback loop) | Implementation (single-shot) |
|--------|--------------------|-----------------------------|
| Success rate | Higher (tries alternatives) | Lower (fails on first error) |
| UX | "Payment succeeded" more often | "Payment failed" more often |
| Complexity | Loop logic needed | Simple dispatch |
| Double-spend risk | Must be careful per error type | None |

### Recommendation

**Follow the spec.** Add `execute_with_fallbacks` to `paykit-mobile`:

```rust
pub fn execute_with_fallbacks(
    &self,
    methods: Vec<(String, String)>,  // (method_id, endpoint) pairs
    amount_sats: u64,
    metadata_json: Option<String>,
) -> Result<PaymentExecutionResult> {
    for (method_id, endpoint) in methods {
        match self.execute_payment(method_id, endpoint, amount_sats, metadata_json.clone()) {
            Ok(result) if result.success => return Ok(result),
            Ok(result) if is_retryable_error(&result.error) => continue,
            other => return other,
        }
    }
    Err(PaykitMobileError::Transport { msg: "All methods exhausted".into() })
}
```

Define **retryable vs non-retryable errors** to avoid double-spend risks.

---

## 6. Paykit Daemon

### PDF Specification

> "The Paykit Daemon is a **stateful component** that keeps track of sent and received payments, provides a unified API for various payment operations, and includes advanced logic for payment prioritization and subscription management."

Listed features:
- Payments to pubkey/URL with fallback
- Payment requests
- Subscription management (push/pull)
- Accounting API
- Event notifications

### Implementation Reality

**No daemon crate exists.** Daemon-like behavior is embedded in apps:

| Component | Location | Behavior |
|-----------|----------|----------|
| `PaykitPollingWorker` | Bitkit Android | Periodic request discovery |
| `PaykitPollingService` | Bitkit iOS | Background refresh polling |
| `PaymentRequestService` | Both platforms | Request handling + autopay |
| `paykit-subscriptions` | paykit-rs | Subscription primitives |

### Analysis

The PDF's daemon vision is **server-oriented** (always-on, SQLite DB, LND connectivity). The implementation is **mobile-first** (ephemeral, in-app, polling-based).

### Recommendation

**Do not build a daemon for Bitkit's use case.** Document that:

1. `paykit-subscriptions` + mobile services = daemon-equivalent for mobile
2. A standalone daemon is an **optional deployment profile** for merchants/servers
3. Update the spec to reflect this reality

---

## 7. Cross-Implementation Consistency (Critical)

### Payment Request Paths

| Implementation | Path Format | Encryption |
|----------------|-------------|------------|
| `paykit-subscriptions` | `/pub/paykit.app/v0/requests/{request_id}` | Sealed Blob v1 |
| Bitkit Android | `/pub/paykit.app/v0/requests/{recipient}/{id}` | Plaintext JSON |
| Bitkit iOS | `/pub/paykit.app/v0/requests/{recipient}/{id}` | Plaintext JSON |

```rust
// paykit-subscriptions/src/discovery.rs
pub const PAYKIT_REQUESTS_PATH: &str = "/pub/paykit.app/v0/requests/";
let envelope = sealed_blob_encrypt(recipient_noise_pk, &plaintext, &aad, Some("request"))?;
```

```kotlin
// bitkit-android/.../DirectoryService.kt
fun paymentRequestPath(recipientPubkey: String, requestId: String): String =
    "/pub/paykit.app/v0/requests/$recipientPubkey/$requestId"
val result = adapter.put(path, requestJson)  // Plaintext!
```

### Subscription Proposal Paths

| Implementation | Path Format |
|----------------|-------------|
| `paykit-subscriptions` | `/pub/paykit.app/subscriptions/proposals/{provider}/{id}` (no `/v0/`!) |
| Bitkit Android/iOS | `/pub/paykit.app/v0/subscriptions/proposals/{recipient}/{id}` |

### Critical Issue

**These implementations cannot interoperate.** A request published by `paykit-subscriptions` will not be discovered by Bitkit, and vice versa.

### Recommendation

**Immediate action required:**

1. Decide on **one canonical path format**:
   - Option A: `/pub/paykit.app/v0/requests/{request_id}` (sender-storage model)
   - Option B: `/pub/paykit.app/v0/requests/{recipient}/{request_id}` (recipient-inbox model)

2. Decide on **encryption policy**:
   - Sealed Blob v1 should be **mandatory** for payment requests (they contain sensitive data)

3. Update all implementations to match.

---

## 8. Capability URLs ("Access URL grants read/write")

### PDF Specification

The PDF describes a **capability-based access model**:

> "Read access to locations can be public or restricted and granted with URL"
> 
> "Write access to locations can be granted to non-owners and granted with URL"
> 
> "Access levels can be changed without changing the path component of the URL"

This implies:
- URL = capability token (possession grants access)
- Single URL can embed read or write permissions
- Access revocation is possible without changing paths

### Implementation Reality

The transport traits model **binary authentication**, not capability URLs:

```rust
// paykit-lib/src/transport/traits.rs
pub trait AuthenticatedTransport: Send + Sync {
    async fn put(&self, path: &str, data: &str) -> Result<()>;
    async fn delete(&self, path: &str) -> Result<()>;
}

pub trait UnauthenticatedTransportRead: Send + Sync {
    async fn get(&self, uri: &str) -> Result<Option<Vec<u8>>>;
    async fn list(&self, uri: &str) -> Result<Vec<DirEntry>>;
}
```

Current model:
- **Authenticated** = session cookie → full write access to own paths
- **Unauthenticated** = public read of any pubky:// URI
- No middle ground (delegation, scoped writes, capability tokens)

### Analysis

| Aspect | PDF (capability URLs) | Implementation (binary auth) |
|--------|----------------------|------------------------------|
| Delegation | Share URL = share access | Not possible |
| Granularity | Per-path, per-operation | All-or-nothing |
| Revocation | Change URL secret | Re-auth entire session |
| Use cases | Third-party writes, shared inboxes | Single-owner storage only |

### Recommendation

For Bitkit's current use case (single-user wallet), **binary auth is sufficient**.

If multi-party writes are needed later (e.g., shared subscription inboxes, merchant delegation), consider:

```rust
pub trait CapabilityTransport: Send + Sync {
    /// Write using a delegation token (not full session)
    async fn put_with_capability(&self, path: &str, data: &str, cap_token: &str) -> Result<()>;
    
    /// Read a capability-protected resource
    async fn get_with_capability(&self, uri: &str, cap_token: &str) -> Result<Option<Vec<u8>>>;
}
```

**Update the spec** to clarify that capability URLs are an **optional extension**, not a baseline requirement.

---

## 9. "Send Arbitrary Data to URL" API

### PDF Specification

> "Send arbitrary UTF-8 data to the Paykit Routing Network at a given URL"

This is described as a **primitive** the Paykit Library should expose, enabling:
- Payment requests (payee → payer)
- Memos / notes
- Custom application data

### Implementation Reality

**No generic "send to URL" API exists.** Instead, purpose-built modules handle specific data types:

| Data Type | Module | Path Pattern |
|-----------|--------|--------------|
| Payment requests | `paykit-subscriptions/discovery.rs` | `/pub/paykit.app/v0/requests/{id}` |
| Subscription proposals | `paykit-subscriptions/manager.rs` | `/pub/paykit.app/subscriptions/proposals/{provider}/{id}` |
| Handoff payloads | Bitkit + Pubky Ring | `/pub/paykit.app/v0/handoff/{id}` |
| Noise endpoints | `paykit-lib` | `/pub/paykit.app/v0/noise` |

Each module encrypts and formats data differently—there's no shared abstraction.

### Analysis

| Aspect | PDF (generic API) | Implementation (per-type modules) |
|--------|-------------------|----------------------------------|
| Flexibility | Any data, any path | Fixed schemas per use case |
| Type safety | None (raw bytes) | Strong (Rust types + serde) |
| Encryption | Caller's responsibility | Built into each module |
| Discoverability | Caller must know path | Modules define conventions |

### Recommendation

**Keep purpose-built modules** for security and type safety. A raw "send bytes to URL" API would likely lead to:
- Plaintext sensitive data (encryption forgotten)
- Path collisions
- Schema drift

If extensibility is needed, define a **registry of data types**:

```rust
pub enum PaykitDataType {
    PaymentRequest,
    SubscriptionProposal,
    Handoff,
    Custom { type_id: String },
}

pub fn publish_data(
    transport: &impl AuthenticatedTransport,
    data_type: PaykitDataType,
    payload: &[u8],
    encryption: EncryptionPolicy,
) -> Result<String>  // Returns published URL
```

**Update the spec** to describe the typed-module approach rather than raw URL writes.

---

## 10. Secure Handoff (Code-Only Feature)

The PDF does not describe cross-device authentication. The implementation adds a sophisticated handoff system:

```typescript
// pubky-ring/src/utils/actions/paykitConnectAction.ts
const storagePath = `/pub/paykit.app/v0/handoff/${requestId}`;
const aad = `handoff:${pubky}:${storagePath}`;
encryptedEnvelope = await sealedBlobEncrypt(ephemeralPk, payloadHex, aad, 'handoff');
```

```kotlin
// bitkit-android/.../SecureHandoffHandler.kt
val plaintextBytes = com.pubky.noise.sealedBlobDecrypt(secretKeyBytes, envelopeJson, aad)
```

```swift
// bitkit-ios/.../SecureHandoffHandler.swift
let plaintextData = try sealedBlobDecrypt(recipientSk: secretKeyData, envelopeJson: envelopeJson, aad: aad)
```

### Recommendation

**Document this in the spec.** Secure handoff is a well-designed security feature that should be standardized.

---

## 11. What Aligns Reasonably Well

Not everything has drifted. These areas match the PDF's intent:

| Spec Concept | Implementation | Notes |
|--------------|----------------|-------|
| **Pubky as routing network** | `pubky://` URIs throughout | DHT-backed storage as specified |
| **Multiple payment methods per payee** | `SupportedPayments` map | Any number of methods supported |
| **Paykit Library is stateless** | `paykit-lib` has no DB | Functions accept transport, return results |
| **Noise Protocol for interactive payments** | `paykit-interactive`, `pubky-noise` | Encrypted channels for real-time flows |
| **Method-agnostic core** | Plugin architecture in `paykit-mobile` | Methods register handlers |
| **Epoch-based key rotation** | `NoiseKeypairManager` (Android/iOS) | Epochs 0 and 1 derived from seed |
| **HKDF for noise_seed derivation** | `paykitConnectAction.ts` | `noise_seed = HKDF-SHA256(ed25519_sk, "paykit-noise-seed")` |
| **Subscription primitives** | `paykit-subscriptions` crate | Proposals, status tracking |

### Aligned Security Properties

The implementation correctly implements these PDF-implied security requirements:

1. **Authenticated writes only to own paths** — session-bound storage
2. **Sealed Blob AAD includes path** — prevents relocation attacks
3. **Ephemeral keys for handoff** — forward secrecy for cross-device auth
4. **Ed25519 identity, X25519 encryption** — standard key separation

---

## Summary Matrix

| Spec Item | Drift | Better? | Action |
|-----------|-------|---------|--------|
| Single JSON list | Per-method files | Yes | Update spec, add snapshot option |
| Method URLs | Short string IDs | Yes | Keep IDs, add optional URI |
| Endpoint URLs | Inline payloads | Yes | Keep inline, define schemas |
| Private list URLs | Sealed Blob encryption | Yes | Update spec to encryption model |
| Fallback loop | Single-shot execution | **No** | Add `execute_with_fallbacks` |
| Daemon | In-app services | Neutral | Document as deployment option |
| Request paths | **Implementations disagree** | **Broken** | Align immediately |
| Capability URLs | Binary auth only | Neutral | Document as optional extension |
| Generic "send to URL" | Typed modules | Yes | Keep modules, update spec |
| Secure handoff | Not in spec | **Addition** | Document in spec |

### Omissions in This Report vs `PAYKIT_PDF_DRIFT_REPORT.md`

This report was generated after an initial drift analysis. The first report included items that were initially omitted from this version:

| Originally Omitted | Now Covered In |
|--------------------|----------------|
| "Access URL grants read/write" (capability tokens) | Section 8 |
| "Send arbitrary data to URL" API | Section 9 |
| "What aligns reasonably well" (positive findings) | Section 11 |
| Hybrid delegation design alternatives | Section 8 (Recommendation) |

Both reports reach the **same conclusions** on all shared topics. The new report adds:
- Executive summary with severity ratings
- Per-section analysis tables
- Proposed code snippets for recommendations

---

## Appendix: File References

| Concept | Key Files |
|---------|-----------|
| Path conventions | `paykit-lib/src/transport/pubky/mod.rs` |
| Supported payments | `paykit-lib/src/transport/pubky/unauthenticated_transport.rs` |
| Transport traits | `paykit-lib/src/transport/traits.rs` |
| Method selection | `paykit-lib/src/selection/selector.rs` |
| Private endpoints | `paykit-lib/src/private_endpoints/mod.rs` |
| Request publishing | `paykit-subscriptions/src/discovery.rs` |
| Subscription proposals | `paykit-subscriptions/src/manager.rs` |
| Android requests | `bitkit-android/.../DirectoryService.kt` |
| iOS requests | `bitkit-ios/.../DirectoryService.swift` |
| Secure handoff (Ring) | `pubky-ring/src/utils/actions/paykitConnectAction.ts` |
| Android handoff | `bitkit-android/.../SecureHandoffHandler.kt` |
| iOS handoff | `bitkit-ios/.../SecureHandoffHandler.swift` |
| Android polling | `bitkit-android/.../PaykitPollingWorker.kt` |
| iOS polling | `bitkit-ios/.../PaykitPollingService.swift` |
| FFI bindings | `paykit-mobile/src/lib.rs` |
| Noise key derivation | `pubky-ring/src/utils/actions/paykitConnectAction.ts` |

