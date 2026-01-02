# Paykit.pdf vs Implementation Drift Report

**Date**: 2026-01-02  
**Reference spec**: `Paykit.pdf` (provided)  

## Scope

Compared the Paykit spec against these codebases/areas:

- `paykit-rs/` (notably `paykit-lib/`, `paykit-subscriptions/`, `paykit-mobile/`)
- Bitkit Android (`bitkit-android/app/src/main/java/to/bitkit/paykit/…`)
- Bitkit iOS (`bitkit-ios/Bitkit/PaykitIntegration/…`)
- Pubky Ring (`pubky-ring/src/utils/actions/paykitConnectAction.ts`, `pubky-ring/src/utils/PubkyNoiseModule.ts`)

This report focuses on **protocol/schema/path/behavior drift** between the PDF and current implementations.

## Biggest protocol drifts (spec → code)

- **Supported Payments List storage + schema**
  - **PDF**: a single file at a conventional public path containing a JSON array of `{ method, endpoint }`.
  - **Code**: a directory of per-method files under `/pub/paykit.app/v0/{method_id}`; supported payments is derived by listing + reading.

- **Payment method identifiers**
  - **PDF**: `method` values shown as URLs (e.g., `paykit.standards.com/lightning`).
  - **Code**: `method_id` is an opaque short string (e.g., `lightning`, `onchain`).

- **Endpoint representation**
  - **PDF**: `endpoint` is a URL to a “payment endpoint” resource that then returns UTF‑8 payment data.
  - **Code**: directory file content is typically the endpoint payload itself (invoice/address/JSON/etc). No extra endpoint URL indirection is required by the library.

- **Private lists via access URL**
  - **PDF**: “private Supported Payment Method Lists” shared by URL, optionally encrypted.
  - **Code**: does not model “private lists” as first-class objects; privacy is handled via **Noise** and/or **sealed-blob encrypted payloads** for certain flows.

- **Default fallback loop**
  - **PDF**: explicitly retries alternative methods until the list is exhausted.
  - **Code**: method selection can produce fallbacks, but Bitkit/PaykitMobile generally execute one selected method and do not automatically loop through fallbacks on failure.

- **Daemon**
  - **PDF**: describes a stateful Paykit Daemon as a key deliverable.
  - **Code**: no daemon crate exists in `paykit-rs`; Bitkit implements polling/autopay in-app (daemon-like behavior) instead.

## 1) Supported Payments List: single list file (PDF) vs directory of method files (code)

**PDF expectation**: resolving a pubkey returns a Supported Payments List stored at a default public path as a JSON array.

**Code reality**: supported payments is modeled as “list `/pub/paykit.app/v0/` and treat each file name as a method ID; file contents are endpoint payloads”.

### Evidence

`paykit-lib` explicitly defines v0 as per-method files:

```rust
// paykit-lib/src/transport/pubky/mod.rs
/// Conventional prefix for Paykit data hosted on Pubky storage.
/// `v0` means that the paykit conventions is to store data on pubky as following:
///  - /pub/paykit.app/v0/{method_id} -> with payload being the payment endpoint
pub const PAYKIT_PATH_PREFIX: &str = "/pub/paykit.app/v0/";
```

`paykit-lib` reads supported payments by listing + reading the directory:

```rust
// paykit-lib/src/transport/pubky/unauthenticated_transport.rs
async fn fetch_supported_payments(&self, payee: &PublicKey) -> Result<SupportedPayments> {
    let addr = format!("pubky{payee}{PAYKIT_PATH_PREFIX}");
    let entries = self.list_entries(addr, "list supported payments").await?;

    let mut map = HashMap::new();
    for resource in entries {
        if resource.path.as_str().ends_with('/') {
            continue;
        }

        let method = resource
            .path
            .as_str()
            .rsplit('/')
            .next()
            .filter(|segment| !segment.is_empty())
            .ok_or_else(|| {
                PaykitError::Transport("invalid resource returned for supported payment entry".into())
            })?
            .to_string();

        let label = format!("fetch endpoint {}", method);
        if let Some(payload) = self.fetch_text(resource.to_string(), &label).await? {
            map.insert(MethodId(method), EndpointData(payload));
        }
    }

    Ok(SupportedPayments { entries: map })
}
```

Note: PaykitMobile FFI uses the same directory convention (via callback `list` + `get`) and expects method entries like `"/pub/paykit.app/v0/lightning"` (see `paykit-mobile/src/transport_ffi.rs`).

### Impact vs PDF

Any consumer expecting to fetch and parse a single JSON “Supported Payments List” file per the PDF will not interoperate with this implementation without a compatibility layer.

## 2) Method identifiers: URL (PDF) vs opaque string IDs (code)

**PDF expectation**: list entries use `method` URLs (examples like `paykit.standards.com/lightning`, `paykit.standards.com/p2pkh`).

**Code reality**: method IDs are plain strings; `paykit-lib` even defines well-known IDs `"onchain"` and `"lightning"`.

### Evidence

```rust
// paykit-lib/src/lib.rs
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MethodId(pub String);

impl MethodId {
    /// Well-known method ID for on-chain Bitcoin payments.
    pub const ONCHAIN: &'static str = "onchain";

    /// Well-known method ID for Lightning payments.
    pub const LIGHTNING: &'static str = "lightning";
}
```

### Impact vs PDF

The PDF’s implied “method registry by URL” does not exist at the data layer in current code.

## 3) Endpoint semantics: endpoint URL indirection (PDF) vs direct payload (code)

**PDF expectation**: each entry includes an `endpoint` URL; that endpoint URL returns UTF‑8 data (address/invoice/etc).

**Code reality**: the directory entry file content is typically the payload (UTF‑8 string). Consumers do not need to fetch a second resource unless the payload itself is a URL by convention.

### Evidence

PaykitMobile directory output is `PaymentMethod { method_id, endpoint }` where `endpoint` is a string payload:

```rust
// paykit-mobile/src/lib.rs
#[derive(Clone, Debug, uniffi::Record)]
pub struct PaymentMethod {
    pub method_id: String,
    pub endpoint: String,
}
```

### Impact vs PDF

If you want the PDF’s two-step “list → endpoint URL → endpoint content” flow, you’d have to store a URL string as the per-method file content and add app logic to follow it. The library does not enforce that model.

## 4) Private Supported Payment Lists by access URL (PDF) are not implemented as described

**PDF expectation**: “Private Payment Method Lists” are stored at private locations, optionally encrypted, and shared by giving the payer an access URL to the list. Known-peer flow starts from that URL.

**Code reality**:

- No “private supported payments list” object exists in `paykit-lib` / `paykit-mobile` analogous to “fetch private list by URL”.
- Private coordination is implemented via:
  - Noise endpoint discovery at `/pub/paykit.app/v0/noise` (PaykitMobile FFI), and/or
  - sealed-blob encryption for certain payloads (handoff, requests/proposals in some paths), and/or
  - app-level “private endpoints” exchanged over Noise and stored locally (not as a per-peer “private list URL”).

### Impact vs PDF

This is a design-level mismatch: PDF’s private-discovery object is a shared URL; current code’s private coordination object is an encrypted channel and/or encrypted blob stored under public paths.

## 5) Default selection + fallback loop (PDF) is not what Bitkit/PaykitMobile do today

**PDF expectation**: payer tries a selected method; on failure tries the next compatible method; repeats until exhausted.

**Code reality**:

- PaykitMobile exposes selection returning `primary_method` + `fallback_methods`.
- Payment execution is per-method; there is no built-in “execute with fallback loop”.

### Evidence

PaykitMobile executes a single method:

```rust
// paykit-mobile/src/lib.rs
pub fn execute_payment(
    &self,
    method_id: String,
    endpoint: String,
    amount_sats: u64,
    metadata_json: Option<String>,
) -> Result<PaymentExecutionResult> {
    let plugin = self
        .registry
        .read()
        .unwrap()
        .get(&paykit_lib::MethodId(method_id.clone()))
        .ok_or(PaykitMobileError::NotFound {
            msg: format!("Payment method not registered: {}", method_id),
        })?;

    let endpoint_data = paykit_lib::EndpointData(endpoint.clone());
    let amount = paykit_lib::methods::Amount::sats(amount_sats);

    // ...
    let execution = self.runtime.block_on(async {
        plugin
            .execute_payment(&endpoint_data, &amount, &metadata)
            .await
    })?;
    // ...
}
```

Bitkit Android selects a method then executes once (no retry loop across fallbacks):

```kotlin
// bitkit-android/app/src/main/java/to/bitkit/paykit/services/PaykitPaymentService.kt
val selectedMethod = selectOptimalMethod(pubkey, amount)
val result = client.executePayment(
    methodId = selectedMethod.methodId,
    endpoint = selectedMethod.endpoint,
    amountSats = amount,
    metadataJson = null,
)
```

Bitkit iOS paykit URI path uses the **first** discovered method and does not use selection or retries:

```swift
// bitkit-ios/Bitkit/PaykitIntegration/Services/PaykitPaymentService.swift
let methods = try await directoryService.discoverPaymentMethods(for: pubkey)
guard let firstMethod = methods.first else {
    throw PaykitPaymentError.invalidRecipient("No payment methods found for \(pubkey)")
}

let result = try client.executePayment(
    methodId: firstMethod.methodId,
    endpoint: firstMethod.endpoint,
    amountSats: amount,
    metadataJson: nil
)
```

### Impact vs PDF

The PDF’s specified “try-next-on-failure until empty” behavior is not implemented in current Bitkit Paykit-send paths.

## 6) “Access URL grants read/write” (PDF) doesn’t map cleanly to current transport abstractions

**PDF expectation**: the routing network grants read/write access via URLs, and access levels can change without changing the path.

**Code reality**:

- `paykit-lib`’s transport traits are keyed by `(owner_pubkey, path)` for reads and session-based authenticated `put/get/delete` for writes.
- There is no first-class model of “capability URLs” at the library layer.

### Impact vs PDF

The spec’s “URL as capability token” design isn’t represented directly in the current trait/API model.

## 7) Paykit Daemon (PDF) does not exist in `paykit-rs`

**PDF expectation**: a daemon is a key component.

**Code reality**:

- `paykit-rs` includes library + interactive + subscriptions + mobile bindings + demos, but no daemon crate.

Closest equivalents today:

- Bitkit Android: `PaykitPollingWorker` + `PaymentRequestService` + autopay logic provide daemon-like behavior inside the app.
- Bitkit iOS: `PaykitPollingService` does similar.

This is application-specific, not a standalone daemon component.

## 8) Pubky Ring + secure handoff: added architecture not in Paykit.pdf

**PDF** doesn’t cover “Ring-only identity + secure handoff of sessions/Noise keys”.

**Code** adds a major layer:

- Pubky Ring `paykit-connect` encrypts a handoff payload and stores it at `/pub/paykit.app/v0/handoff/{request_id}`, returning only `{pubky, request_id}` to Bitkit.

### Evidence

- `pubky-ring/src/utils/actions/paykitConnectAction.ts`: requires `ephemeralPk`, encrypts payload with Sealed Blob v1, stores at `/pub/paykit.app/v0/handoff/{request_id}` with AAD `handoff:{pubky}:{storagePath}`.
- `bitkit-android/app/src/main/java/to/bitkit/paykit/services/SecureHandoffHandler.kt`: detects sealed blobs and decrypts using `com.pubky.noise.sealedBlobDecrypt(...)` with the same AAD construction.
- `bitkit-ios/Bitkit/PaykitIntegration/Services/SecureHandoffHandler.swift`: decrypts sealed blobs and persists `noise_seed` for local epoch derivation.

```typescript
// pubky-ring/src/utils/actions/paykitConnectAction.ts
// SECURITY: ephemeralPk is REQUIRED for secure handoff
// Legacy mode (without encryption) has been removed
if (!ephemeralPk) {
  return err('ephemeralPk is required for secure handoff. Legacy mode is no longer supported.');
}
```

```typescript
// pubky-ring/src/utils/actions/paykitConnectAction.ts
const storagePath = `/pub/paykit.app/v0/handoff/${requestId}`;
const aad = `handoff:${pubky}:${storagePath}`;
// ...
encryptedEnvelope = await sealedBlobEncrypt(ephemeralPk, payloadHex, aad, 'handoff');
const handoffPath = `pubky://${pubky}/pub/paykit.app/v0/handoff/${requestId}`;
await put(handoffPath, JSON.parse(encryptedEnvelope), ed25519SecretKey);
```

```kotlin
// bitkit-android/app/src/main/java/to/bitkit/paykit/services/SecureHandoffHandler.kt
if (isSealedBlob(payloadJson)) {
    return decryptHandoffEnvelope(payloadJson, pubkey, requestId, ephemeralSecretKey)
}
// Legacy: try direct JSON decode (for pre-encryption payloads during migration)
```

```swift
// bitkit-ios/Bitkit/PaykitIntegration/Services/SecureHandoffHandler.swift
let storagePath = "/pub/paykit.app/v0/handoff/\(requestId)"
let aad = "handoff:\(pubkey):\(storagePath)"
let plaintextData = try sealedBlobDecrypt(
    recipientSk: secretKeyData,
    envelopeJson: envelopeJson,
    aad: aad
)
```

### Impact vs PDF

PDF’s “private lists optionally encrypted via URL” is effectively replaced by sealed-blob encrypted public storage + unguessable IDs. Bitkit Android and iOS both include Sealed Blob v1 decryption paths, and both still contain legacy plaintext fallback handling.

## 9) Payment Requests / “send arbitrary data to URL” (PDF) vs current implementation

**PDF expectation**: includes a general “send arbitrary data to URL” API for e.g. payment requests or memos; private URL flows are central.

**Code reality**:

- Payment requests exist, but not as a general “arbitrary data to URL” abstraction.
- There is drift between `paykit-rs` vs Bitkit on request storage path and encryption conventions.

### Evidence: `paykit-subscriptions` request publishing path vs Bitkit request paths

`paykit-subscriptions` uses `/pub/paykit.app/v0/requests/{request_id}` (no `{recipientPubkey}` nesting) and encrypts using sealed-blob:

```rust
// paykit-subscriptions/src/discovery.rs
pub const PAYKIT_REQUESTS_PATH: &str = "/pub/paykit.app/v0/requests/";

let path = format!("{}{}", PAYKIT_REQUESTS_PATH, request.request_id);
let aad = format!("{}:{}:request", path, request.request_id);
let envelope = sealed_blob_encrypt(recipient_noise_pk, &plaintext, &aad, Some("request"))?;
transport.put(&path, &envelope).await?;
```

Bitkit Android uses `/pub/paykit.app/v0/requests/{recipientPubkey}/{requestId}` and stores plaintext JSON:

```kotlin
// bitkit-android/app/src/main/java/to/bitkit/paykit/services/DirectoryService.kt
fun paymentRequestPath(recipientPubkey: String, requestId: String): String =
    "/pub/paykit.app/v0/requests/$recipientPubkey/$requestId"

val path = PubkyConfig.paymentRequestPath(recipientPubkey, request.id)
val result = adapter.put(path, requestJson)
```

Bitkit iOS follows the same `{recipient}/{requestId}` addressing and parses JSON directly:

```swift
// bitkit-ios/Bitkit/PaykitIntegration/Services/DirectoryService.swift
let path = "/pub/paykit.app/v0/requests/\(recipient)/\(requestId)"
let pubkyUri = "pubky://\(senderPubkey)\(path)"
guard let data = try await PubkySDKService.shared.getData(pubkyUri) else { return nil }
let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
```

Bitkit Android/iOS polling lists `/pub/paykit.app/v0/requests/{ownerPubkey}/` inside the recipient’s own storage:

- Android: `bitkit-android/app/src/main/java/to/bitkit/paykit/services/DirectoryService.kt` (`discoverPendingRequests`)
- iOS: `bitkit-ios/Bitkit/PaykitIntegration/Services/DirectoryService.swift` (`discoverPendingRequests`)

### Additional drift: subscriptions proposal paths and encryption

- `paykit-subscriptions/src/manager.rs`: stores subscription proposals encrypted (Sealed Blob v1) under `/pub/paykit.app/subscriptions/proposals/{provider}/{subscription_id}` (note: not under `/v0/`).
- Bitkit Android/iOS store and list subscription proposals under `/pub/paykit.app/v0/subscriptions/proposals/{recipientPubkey}/{proposalId}` and parse JSON directly.

### Impact vs PDF

Even if you interpret “payment requests” as the PDF’s “send arbitrary data” feature, current code does not implement the PDF’s “share private list URL” mechanism, and request addressing/encryption conventions aren’t aligned across our Rust library vs Bitkit.

## What aligns reasonably well with Paykit.pdf

- Pubky-based routing assumption (pubkey-based discovery and `/pub/` storage paths).
- Multiple payment methods + selection exists (though identifiers and fallback behavior differ).
- Interactive payments exist via Noise (more specific than the PDF, but directionally similar to “optional private messaging / interactive hooks”).

## Notes / follow-up

This report intentionally does not propose changes yet. It’s meant to capture drift accurately.  
If desired, a follow-up document can:

- propose a compatibility layer toward the PDF, or
- update the spec to match current v0 conventions, or
- define a clean “v1” that reconciles list schema, private list semantics, and fallback execution.

## Analysis: are these drifts improvements, and what should we do?

This section is opinionated guidance about whether the current implementation choices are improvements over the PDF, whether we should consider other designs, and where it likely makes sense to “follow the spec” versus update it.

### A) Supported Payments List: single JSON array vs per-method files

- **Assessment**
  - **Improvement in operability**: per-method files allow updating one method without rewriting a whole list and without merge conflicts between writers (important if multiple apps/devices update endpoints).
  - **Regression in atomicity + spec compatibility**: the PDF’s “single list” is easy to fetch and parse as a snapshot, and interoperates with generic clients; directory listing + N reads is more round-trippy and can produce mixed-version views (e.g., partially updated list).

- **Designs to consider**
  - **Hybrid “v0 + snapshot”**: keep per-method files as the source of truth, but additionally publish a single derived snapshot file (e.g., `/pub/paykit.app/v0/supported.json`) for fast reads and PDF-style interoperability.
  - **Manifest + hash/etag**: publish a manifest with method IDs and content hashes so clients can do incremental fetches safely and detect partial updates.

- **Recommendation**
  - **Do not force code back to a single-file model**; the per-method layout is a reasonable base.
  - **Update the spec** (or add a “v0 storage conventions” section) and optionally add a snapshot file for interoperability.

### B) Method identifiers: URL registry vs short string IDs

- **Assessment**
  - **Improvement in simplicity**: short IDs work well for FFI/mobile and plugin registries.
  - **Regression in global namespacing**: URLs avoid collisions and double as documentation/discovery of semantics.

- **Designs to consider**
  - **Namespaced IDs**: e.g., `ln-btc`, `btc-onchain`, or `paykit:ln-btc`.
  - **Dual identifier**: keep a stable `method_id` for storage/plugin dispatch, but define an optional `method_uri` that points to the method spec (PDF-style) for standards-level interoperability.

- **Recommendation**
  - Keep `method_id` as the primary key, but **standardize naming** (and ideally publish a registry document) so the ecosystem converges.
  - If the PDF’s “URL method” is important, add it as optional metadata rather than replacing IDs.

### C) Endpoint URL indirection vs inline endpoint payload

- **Assessment**
  - **Improvement in latency and reliability**: fewer fetches; endpoint content is available immediately.
  - **Regression in extensibility**: endpoint-as-URL allows richer endpoint resources, multiple variants, and future schema evolution without changing directory conventions.

- **Designs to consider**
  - **Structured endpoint payload**: store a small JSON object as the per-method file value (still one fetch) with optional fields like `type`, `value`, `expires_at`, `network`, `features`, `endpoint_url`.
  - **Allow both**: define that endpoint content may be either a direct payload or an HTTP/`pubky://` URL, with clear client behavior (follow URLs only when explicitly allowed).

- **Recommendation**
  - Keep “inline payload” as the default (it’s practical), but define a **well-known schema per method** so “opaque string” doesn’t become a long-term interoperability trap.

### D) “Private lists via access URL” vs Noise + Sealed Blob

- **Assessment**
  - **Improvement in security clarity**: “private by URL” is not a security boundary; sealed-blob style encryption is.
  - **Spec gap**: the PDF talks about access URLs but does not fully specify a capability model, key distribution, AAD binding, or rotation.

- **Designs to consider**
  - **Capability URL as encrypted pointer**: a URL can still exist, but it should point to an encrypted blob (or encrypted manifest) and include no secrets; access is enforced cryptographically, not by obscurity.
  - **Per-peer private endpoints**: store private endpoints encrypted to the peer’s Noise public key at a deterministic path (or under an unguessable path shared out-of-band).

- **Recommendation**
  - Prefer updating the spec to align with **Sealed Blob–style encryption semantics** rather than trying to implement “private list URLs” as access control.

### E) Selection + fallback loop behavior

- **Assessment**
  - **Spec is better for UX**: “try next method on failure” improves payment success rates.
  - **Current behavior is simpler but brittle**: a single-method execution fails hard even if another method would work.

- **Designs to consider**
  - **Library-level `execute_with_fallbacks`**: make the fallback loop an explicit API in `paykit-mobile` so apps don’t re-implement it inconsistently.
  - **Policy-driven retries**: only retry on specific error classes (e.g., “invoice expired” vs “insufficient funds”) to avoid unintended double-spend attempts.

- **Recommendation**
  - Here, it’s worth **following the spec** (or at least matching its intent) by adding a standardized fallback execution path at the Paykit layer.

### F) “Access URL grants read/write” vs session-based authenticated transport

- **Assessment**
  - The PDF’s “URL = capability” idea is attractive, but it needs a concrete security model (bearer tokens, macaroons, signed URLs, etc).
  - Current code’s session-based authenticated transport is simpler and fits Pubky sessions, but doesn’t naturally support third-party delegated write access.

- **Designs to consider**
  - **Delegation tokens** for limited write scopes (time-bound, path-bound), which would more closely match the PDF’s intent without overloading URLs themselves.

- **Recommendation**
  - If delegated writes are a real product requirement, expand the spec with an explicit capability/delegation mechanism; otherwise keep the current transport model.

### G) Paykit Daemon vs in-app polling

- **Assessment**
  - In-app polling is pragmatic for mobile, but it’s not a “daemon” in the spec sense (availability, uptime, always-on behavior).

- **Designs to consider**
  - A separate daemon can make sense for server-side merchants or always-on agents, but may be unnecessary for Bitkit’s current product scope.

- **Recommendation**
  - Treat the daemon as an optional deployment profile rather than a mandatory core deliverable; update the spec accordingly if that’s the reality.

### H) Cross-implementation consistency (highest priority)

Independent of what the PDF says, the most important correctness issue is that **our implementations disagree with each other** on paths and encryption conventions for requests/proposals:

- `paykit-subscriptions` uses encrypted sealed-blob payloads and different path prefixes for proposals.
- Bitkit Android/iOS currently parse request/proposal payloads as plaintext JSON and use different directory layouts, and polling appears to assume an “inbox in recipient storage” model.

**Recommendation**: before deciding “follow PDF vs follow code”, decide on a single canonical model for:

- **Where requests live** (sender storage vs recipient inbox vs notifications index)
- **How recipients discover them** (polling notifications vs direct fetch from known sender)
- **Whether encryption is mandatory** (it should be, if secrets/payment details are present)



