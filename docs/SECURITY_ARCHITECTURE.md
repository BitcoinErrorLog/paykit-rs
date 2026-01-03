# Paykit Security Architecture

This document describes the security model, threat assumptions, and cryptographic design of the Paykit integration with Bitkit and Pubky Ring.

## Table of Contents

1. [Identity Model](#identity-model)
2. [Key Hierarchy](#key-hierarchy)
3. [Session Management](#session-management)
4. [Secure Handoff Protocol](#secure-handoff-protocol)
5. [Push Relay Security](#push-relay-security)
6. [Threat Model](#threat-model)
7. [Security Assumptions](#security-assumptions)
8. [Attack Surface Analysis](#attack-surface-analysis)

---

## Identity Model

### Ring-Only Ed25519 Ownership

**Design Principle**: Ed25519 master keys **never** leave Pubky Ring.

```
┌──────────────┐                    ┌──────────────┐
│ Pubky Ring   │                    │   Bitkit     │
├──────────────┤                    ├──────────────┤
│              │                    │              │
│ Ed25519      │──[derives]────────>│              │
│ Secret Key   │                    │ X25519 Cache │
│              │                    │ (ephemeral)  │
│ [NEVER       │                    │              │
│  EXPORTED]   │                    │ [NO ED25519  │
│              │                    │  SECRETS]    │
└──────────────┘                    └──────────────┘
        │                                   │
        │                                   │
     [signs]                          [verifies]
        │                                   │
        ▼                                   ▼
  External APIs                      Public Keys Only
```

**Rationale**:
- **Separation of concerns**: Ring manages identity, Bitkit manages payments
- **Reduced attack surface**: Compromise of Bitkit does NOT compromise master key
- **Forward secrecy**: X25519 keys can be rotated without affecting identity

### Trust Boundaries

1. **Ring**: Trusted with Ed25519 secrets, key derivation, signing
2. **Bitkit**: Trusted with cached X25519 keys, session secrets, payment data
3. **Homeserver**: Trusted with public data and encrypted payloads (time-limited)
4. **Push Relay**: Trusted with device tokens and wake routing (not message content)

---

## Key Hierarchy

### Derivation Path

```
Ed25519 Master Key (Ring)
    │
    ├─── derives ───> X25519 Keypair (Epoch 0)
    │                 deviceId: <uuid>
    │                 epoch: 0
    │                 purpose: Noise IK handshake
    │
    ├─── derives ───> X25519 Keypair (Epoch 1)
    │                 deviceId: <uuid>
    │                 epoch: 1
    │                 purpose: Key rotation
    │
    └─── signs ────> API Signatures
                      (push relay, etc.)
```

### Derivation Function

```rust
// In pubky-noise
pub fn derive_device_key(
    identity_secret: &[u8; 32],  // Ed25519 secret
    device_id: &str,              // Device UUID
    epoch: u32,                   // Rotation epoch
) -> Result<X25519Keypair>
```

**Properties**:
- Deterministic: Same inputs always produce same X25519 keypair
- One-way: Cannot reverse to get Ed25519 secret
- Domain-separated: Different `device_id` or `epoch` = different keys

### Key Lifecycle

| Key Type | Lifetime | Storage | Rotation |
|----------|----------|---------|----------|
| Ed25519 | Permanent | Ring only | Never |
| Noise Seed | Permanent per device | Bitkit secure storage | On device reset |
| X25519 Epoch 0 | 30-90 days | Bitkit cache | To Epoch 1 |
| X25519 Epoch 1 | 30-90 days | Bitkit cache | To Epoch 2 (local derivation) |
| Session Secret | Until revoked | Bitkit secure storage | On 401/403 |
| Ephemeral Handoff Key | Minutes | Bitkit temp storage | Zeroized after handoff |

### Noise Seed for Local Epoch Derivation

**Problem**: Bitkit needs to derive future Noise epochs without repeatedly calling Ring.

**Solution**: Ring derives a per-device `noise_seed` and includes it in the encrypted handoff payload.

```
noise_seed = HKDF-SHA256(
    salt = "paykit-noise-seed-v1",
    ikm = Ed25519_secret,
    info = device_id,
    len = 32
)
```

**Properties**:
- **Domain-separated**: Cannot be used for signing or any non-Noise purpose
- **Device-specific**: Different `device_id` = different `noise_seed`
- **One-way**: Cannot reverse to get Ed25519 secret

**Epoch Derivation** (local, no Ring needed):

```
epoch_key = derive_epoch_keypair(noise_seed, epoch_number)
```

This uses the existing `pubky-noise` KDF with `noise_seed` as input instead of Ed25519 secret.

---

## Session Management

### Session Creation

1. **Ring authenticates** with homeserver (Ed25519 signature)
2. **Homeserver issues** session secret (opaque token)
3. **Ring provides** session to Bitkit via callback
4. **Bitkit stores** session in:
   - iOS: Keychain (`PaykitKeychainStorage`)
   - Android: EncryptedSharedPreferences

### Session Properties

```swift
struct PubkySession {
    let pubkey: String           // Owner's Ed25519 pubkey (z32)
    let sessionSecret: String    // Opaque token from homeserver
    let capabilities: [String]   // Granted permissions
    let createdAt: Date          // When session was created
    let expiresAt: Date?         // Optional expiry (nil = no expiry)
}
```

### Expiry Semantics

**Current Implementation**:
- Sessions do NOT expire automatically
- Active until revoked via Pkarr or by homeserver
- Re-authentication triggered by 401/403 responses

**Future Enhancement**:
- Add `isExpired` and `needsRefresh` computed properties
- Proactive renewal 7 days before expiry
- Automatic re-auth flow on 401/403

### Revocation

Sessions can be revoked by:
1. **Homeserver operator**: Immediate invalidation
2. **Pkarr record update**: Removes homeserver authorization
3. **User action in Ring**: Explicit session termination

---

## Secure Handoff Protocol

### Threat Model

**Attack Scenario**: URL logging/leaks expose session secrets, OR public homeserver storage exposes secrets

**Example Attack Vectors**:
- Web server access logs
- Browser history
- Deep link analytics
- URL sharing/forwarding
- **Public `/pub/` storage on Pubky homeserver** (anyone can read)

### Mitigation: Encrypted Handoff with Sealed Blob

All secret-bearing data stored on `/pub/` paths **must** be encrypted using the **Paykit Sealed Blob v1** format. See [SEALED_BLOB_V1_SPEC.md](./SEALED_BLOB_V1_SPEC.md) for full specification.

**Old (Insecure v1)**:
```
bitkit://paykit-setup?
  session_secret=abc123...&
  noise_secret_key_0=def456...
```
☠️ **Secrets exposed in URL!**

**Old (Insecure v2 - plaintext on homeserver)**:
```
PUT /pub/paykit.app/v0/handoff/abc123
{
  "session_secret": "...",
  "noise_keypairs": [{"secret_key": "..."}]
}
```
☠️ **Secrets readable by anyone who knows the path!**

**New (Secure v3 - encrypted sealed blob)**:
```
bitkit://paykit-setup?
  mode=secure_handoff&
  pubky=8um71us...&
  request_id=f3a7b2c...
```
✅ **Only request ID in URL**

```
PUT /pub/paykit.app/v0/handoff/abc123
{
  "v": 1,
  "epk": "<base64url>",
  "nonce": "<base64url>",
  "ct": "<encrypted session + noise keys + noise_seed>"
}
```
✅ **Secrets encrypted to Bitkit's ephemeral X25519 public key**

### Ephemeral Key Exchange

1. **Bitkit generates ephemeral X25519 keypair** before calling Ring
2. **Bitkit includes `ephemeralPk`** in Ring request URL
3. **Ring encrypts handoff payload** to Bitkit's ephemeral public key
4. **Ring stores only encrypted envelope** on homeserver
5. **Bitkit fetches and decrypts** using ephemeral secret key
6. **Bitkit zeroizes ephemeral secret** after successful decryption

### Security Properties

1. **Unguessable Path**: 256-bit random `request_id` = 2^256 possible paths
2. **Time-Limited**: 5-minute TTL in `expires_at` field (inside encrypted payload)
3. **Single-Use**: Deleted immediately after successful fetch
4. **Access Control**: Requires knowing exact `request_id` (no directory listing)
5. **Encrypted at Rest**: Even if path is discovered, ciphertext is useless without Bitkit's ephemeral secret key
6. **AAD Binding**: Ciphertext is bound to storage path; cannot be relocated

### Attack Analysis

**Brute Force**:
- Probability of guessing `request_id`: 1 / 2^256 = negligible
- Rate limiting on homeserver prevents enumeration
- 5-minute window limits exposure even if path leaks

**Man-in-the-Middle**:
- HTTPS encrypts homeserver communication
- Bitkit verifies TLS certificates
- Homeserver URL resolution via trusted `HomeserverResolver`

**Replay Attacks**:
- Deleted after fetch prevents reuse
- `created_at` and `expires_at` prevent time-manipulation
- `request_id` is unique per request

**Public Storage Attack** (NEW):
- Even if attacker discovers `request_id`, they only get encrypted ciphertext
- Decryption requires Bitkit's ephemeral secret key (never transmitted)
- AAD binding prevents attacker from copying blob to different path

---

## Encrypted Payment Requests & Proposals

### Threat Model

**Attack Scenario**: Payment requests and subscription proposals stored in plaintext on public `/pub/` paths expose:
- Payment amounts and currencies
- Sender/recipient relationships
- Payment method details (invoices, addresses)

### Mitigation: Sealed Blob Encryption

All payment requests and subscription data are encrypted using the **Paykit Sealed Blob v1** format before storage.

**Implementation Status**: ✅ Complete
- Payment requests: Encrypted to recipient's Noise endpoint public key
- Subscription proposals: Encrypted to subscriber's Noise endpoint public key
- Subscription agreements: Encrypted separately to both parties
- Subscription cancellations: Encrypted separately to both parties

**Storage Paths** (encrypted blobs):
- Payment requests: `/pub/paykit.app/v0/requests/{scope}/{requestId}` (scope = sha256(normalized_pubkey))
- Subscription proposals: `/pub/paykit.app/v0/subscriptions/proposals/{scope}/{proposalId}`
- Subscription agreements: `/pub/paykit.app/v0/subscriptions/agreements/{party}/{subscriptionId}`
- Subscription cancellations: `/pub/paykit.app/v0/subscriptions/cancellations/{party}/{subscriptionId}`
- Secure handoff: `/pub/paykit.app/v0/handoff/{requestId}`

**Prerequisite**: Recipients must have a Noise endpoint published at `/pub/paykit.app/v0/noise`.
Apps publish this via `DirectoryService.publishNoiseEndpoint()` (iOS/Android).

### Canonical AAD Formats (Paykit v0 Protocol)

All AAD strings follow the format: `paykit:v0:{purpose}:{...context...}:{id}`

| Object Type | AAD Format | Example |
|-------------|------------|---------|
| Payment Request | `paykit:v0:request:{path}:{requestId}` | `paykit:v0:request:/pub/paykit.app/v0/requests/{scope}/abc:abc` |
| Subscription Proposal | `paykit:v0:subscription_proposal:{path}:{proposalId}` | `paykit:v0:subscription_proposal:/pub/.../prop1:prop1` |
| Subscription Agreement | `paykit:v0:subscription_agreement:{path}:{subscriptionId}` | `paykit:v0:subscription_agreement:/pub/.../sub1:sub1` |
| Subscription Cancellation | `paykit:v0:subscription_cancellation:{path}:{subscriptionId}` | `paykit:v0:subscription_cancellation:/pub/.../sub1:sub1` |
| Secure Handoff | `paykit:v0:handoff:{ownerPubkey}:{path}:{requestId}` | `paykit:v0:handoff:8um71u...:/pub/.../req1:req1` |
| Cross-device Relay Session | `paykit:v0:relay:session:{requestId}` | `paykit:v0:relay:session:abc123` |

### Recipient Scope Hash

Payment requests and subscription proposals use a scope-based directory structure:
- `scope = sha256(normalized_pubkey).hex()`
- Normalized pubkey: lowercase, z32 alphabet, 52 chars, no `pk:` prefix

This provides:
- **Privacy**: Pubkey not exposed in path
- **Determinism**: Same pubkey always maps to same scope
- **Collision resistance**: SHA-256 provides sufficient entropy

### Encryption Flow (Sender)

1. Discover recipient's Noise endpoint public key from `/pub/paykit.app/v0/noise`
2. Compute recipient scope: `sha256(normalize(recipient_pubkey)).hex()`
3. Construct canonical AAD from purpose, path, and ID
4. Encrypt request JSON using `pubky_noise::sealed_blob::sealed_blob_encrypt()`
5. Store encrypted envelope at scope-based path

### Decryption Flow (Recipient)

1. Fetch envelope from storage path
2. Decrypt using locally-cached Noise secret key
3. Reconstruct canonical AAD from purpose, path, and ID
4. Parse decrypted JSON

### Implementation Status (January 2026)

All storage and discovery functions are complete:

| Component | Status |
|-----------|--------|
| `publish_payment_request` | ✅ Encrypted |
| `discover_request` | ✅ Decrypts |
| `store_subscription_proposal` | ✅ Encrypted |
| `store_signed_subscription` | ✅ Encrypted |
| `store_subscription_cancellation` | ✅ Encrypted |
| `discover_subscription_proposals` | ✅ Decrypts |
| `discover_subscription_agreements` | ✅ Decrypts |
| `discover_subscription_cancellations` | ✅ Decrypts |

### App Integration

Both Bitkit iOS and Android correctly implement Noise endpoint publishing:

**Android** (`DirectoryService.kt`):
```kotlin
suspend fun publishNoiseEndpoint(host: String, port: Int, noisePubkey: String, metadata: String? = null)
```

**iOS** (`DirectoryService.swift`):
```swift
func publishNoiseEndpoint(host: String, port: UInt16, noisePubkey: String, metadata: String?) async throws
```

Error handling for missing endpoints is implemented via `NoisePaymentError.EndpointNotFound` (Android) and `NoisePaymentError.endpointNotFound` (iOS).

---

## Push Relay Security

### Threat Model

**Attack Scenario**: Public push tokens enable DoS

**Without Relay**:
1. Attacker reads push token from `/pub/paykit.app/v0/push`
2. Attacker spams APNs/FCM with notifications
3. User's device is overwhelmed or battery drained

### Mitigation: Server-Side Token Storage

**Architecture**:
```
┌─────────┐         ┌─────────────┐         ┌──────────┐
│ Sender  │────┬───>│ Push Relay  │───────>│ Recipient│
└─────────┘    │    └─────────────┘         └──────────┘
               │            │
          [signed]      [validates]
          request        signature
                             │
                        [rate limits]
                             │
                        [forwards to]
                             │
                        APNs/FCM
```

### Security Properties

1. **Token Confidentiality**: Tokens never published publicly
2. **Sender Authentication**: Ed25519 signature required on `/wake` requests
3. **Rate Limiting**:
   - Per-sender: 10 wake/min, 100 wake/hour
   - Per-recipient: 100 wake/hour
   - Global: Infrastructure protection
4. **Replay Prevention**: Nonce required in each wake request

### Signature Scheme

**Message Construction**:
```
<method>:<path>:<timestamp>:<body_hash>
```

**Example**:
```
POST:/wake:1703245689:a3f7b2c1d4e5f6...
```

**Verification**:
1. Check timestamp freshness (<5 minutes old)
2. Reconstruct message from request
3. Verify Ed25519 signature against sender pubkey
4. Reject if signature invalid

### Privacy Considerations

**What Relay Learns**:
- Sender pubkey → recipient pubkey mappings (necessary for routing)
- Wake notification frequency per user pair
- Platform types (iOS/Android)

**What Relay Does NOT Learn**:
- Message content (encrypted end-to-end)
- Payment amounts or details
- User identities beyond pubkeys

**Future Privacy Enhancement**: Anonymous wake via onion routing

### Audit Conclusion (January 2026)

**Status**: ✅ **SECURE** - Push tokens are NOT stored in public `/pub/` paths.

**Implementation Status**:

| Component | Status | Notes |
|-----------|--------|-------|
| `PushRelayService` (iOS) | ✅ Implemented | Server-side token storage, Ed25519 auth |
| `PushRelayService` (Android) | ✅ Implemented | Server-side token storage, Ed25519 auth |

**Verified**:
- No code publishes push tokens to public homeserver paths
- `PushRelayService` is the sole mechanism for push token registration
- Documentation correctly describes relay as the secure alternative

---

## Cross-Device Relay Authentication (pubkyauth)

### Overview

The cross-device relay flow (`pubkyauth://` scheme) enables authentication from one device (e.g., desktop browser) using another device (e.g., mobile phone with Ring). This is documented in [`pubky-core/docs/AUTH.md`](https://github.com/pubky/pubky-core/blob/main/docs/AUTH.md).

### Flow Summary

```
┌──────────────┐   (1) QR   ┌─────────────┐   (4) encrypted   ┌───────────┐
│  3rd Party   │───────────>│   Ring      │─────────────────>│   HTTP    │
│     App      │<───────────│ (Scanner)   │                   │   Relay   │
└──────────────┘   (6) sig  └─────────────┘                   └───────────┘
       │                                                            │
       │ (2) subscribe                                              │
       │<───────────────────────────────────────────────────────────│
       │                          (5) forward                       │
       │<───────────────────────────────────────────────────────────│
```

1. **3rd Party App** generates `client_secret` (32 bytes), subscribes to relay at `channel_id = hash(client_secret)`
2. App displays QR code: `pubkyauth:///?relay=...&caps=...&secret=<base64url(client_secret)>`
3. Ring scans QR, shows consent form, user approves
4. Ring signs `AuthToken`, encrypts with `client_secret`, sends to `relay + channel_id`
5. Relay forwards encrypted token to app
6. App decrypts, extracts `pubky`, presents `AuthToken` to homeserver for session

### Security Properties

| Property | Status | Notes |
|----------|--------|-------|
| **Token Confidentiality** | ✅ Strong | Encrypted with `client_secret` before relay transmission |
| **Ephemeral Transport** | ✅ Strong | Relay only forwards once; no persistent storage |
| **Short-Lived Token** | ✅ Strong | 45-second validity window prevents replay |
| **Mutual Knowledge** | ✅ Strong | Only QR scanner and displayer know `client_secret` |

### Threat Model Comparison

| Attack Vector | Homeserver Storage | Cross-Device Relay |
|---------------|--------------------|--------------------|
| Public path enumeration | ❌ Vulnerable (fixed) | N/A (no public storage) |
| Persistent data exposure | ❌ Vulnerable (fixed) | ✅ Ephemeral only |
| Man-in-the-middle relay | N/A | ✅ Encrypted with client_secret |
| QR code interception | N/A | ⚠️ Requires physical proximity |
| Replay attack | N/A | ✅ 45-second TTL + unique ID |

### Security Analysis

**Why Cross-Device Relay is NOT Vulnerable to the Homeserver Plaintext Issue**:

1. **Ephemeral vs Persistent**: Relay data exists only during transmission; homeserver data was stored indefinitely at guessable paths
2. **Encrypted by Default**: AuthToken is always encrypted with `client_secret` before relay; homeserver used plaintext (now fixed)
3. **Shared Secret via QR**: The `client_secret` is exchanged via physical QR scan, requiring attacker presence
4. **No Predictable Paths**: Relay uses `hash(client_secret)` as channel ID; homeserver paths were predictable

**Remaining Attack Surface**:

| Attack | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| QR photo/screenshot | Low | High | User vigilance; short-lived QR codes |
| Shoulder surfing | Low | High | Physical security awareness |
| Malicious relay operator | Medium | Low | Token encrypted; relay only sees ciphertext |
| Client_secret brute force | Very Low | High | 256-bit secret; computationally infeasible |

### Recommendations

1. **No Changes Required**: The cross-device relay flow already provides strong security properties
2. **Future Enhancement**: Consider time-limited QR codes (regenerate every 30 seconds)
3. **User Education**: Advise users to scan QR codes in private settings
4. **Relay Trust**: Document which relays are trusted; consider self-hosted relay option

### Integration with Paykit

The cross-device relay flow is **separate** from the Paykit secure handoff:

- **Cross-Device Relay**: Used for browser/desktop authentication via mobile Ring
- **Paykit Secure Handoff**: Used for direct mobile-to-mobile Bitkit↔Ring communication

Both flows now use encryption:
- Cross-device relay: `client_secret` encryption (always did)
- Paykit handoff: Ephemeral X25519 encryption (newly added)

---

## Threat Model

### Assumed Attackers

1. **Network Eavesdropper**: Passive monitoring of network traffic
2. **Malicious App**: Other apps on same device trying to steal secrets
3. **Compromised Homeserver**: Malicious homeserver operator
4. **Relay Operator**: Curious or malicious push relay operator

### Security Goals

| Goal | Mechanism |
|------|-----------|
| Identity confidentiality | Ed25519 secrets never leave Ring |
| Session confidentiality | HTTPS + secure storage |
| Forward secrecy | X25519 ephemeral keys |
| Authenticity | Ed25519 signatures |
| Integrity | Hash verification in signatures |
| Availability | Rate limiting + DoS protection |

### Out of Scope

- **Device compromise**: If Ring or Bitkit's secure storage is compromised, all bets are off
- **Homeserver compromise**: Malicious homeserver can MitM its own users
- **Side-channel attacks**: Timing attacks, power analysis, etc.
- **Social engineering**: Phishing, impersonation, etc.

---

## Security Assumptions

### Trust Assumptions

1. **Pubky Ring is secure**:
   - Properly stores Ed25519 secrets in platform keychain
   - Correctly implements `derive_device_key` from pubky-noise
   - Does not leak secrets via logs, crashes, or side channels

2. **Platform Keychains are secure**:
   - iOS Keychain protects data at rest
   - Android EncryptedSharedPreferences provides equivalent security
   - Biometric protection available when configured

3. **Homeservers are honest-but-curious**:
   - Will not tamper with public data
   - May observe access patterns
   - Will honor session revocations

4. **Network is hostile**:
   - TLS prevents eavesdropping
   - Certificate pinning not assumed (relies on system PKI)

### Cryptographic Assumptions

1. **Ed25519 is secure**: 128-bit security level, no known breaks
2. **X25519 is secure**: Diffie-Hellman over Curve25519, equivalent security to Ed25519
3. **Noise Protocol is secure**: IK handshake provides mutual authentication + forward secrecy
4. **SHA-256 is collision-resistant**: Used for body hashing in signatures

---

## Attack Surface Analysis

### Bitkit Attack Surface

| Component | Risk | Mitigation |
|-----------|------|------------|
| X25519 cache | High | Secure storage, automatic expiry, no Ed25519 secrets |
| Session secrets | High | Secure storage, re-auth on 401/403, session revocation |
| Push device token | Medium | Server-side storage (relay), not published publicly |
| Deep link handling | Medium | Input validation, type safety, secure parsing |
| HTTP transport | Low | HTTPS only, certificate validation, timeout enforcement |

### Ring Attack Surface

| Component | Risk | Mitigation |
|-----------|------|------------|
| Ed25519 secret | Critical | Platform keychain, never exported, memory zeroization (TODO) |
| Derivation logic | High | Uses audited pubky-noise library, deterministic |
| Deep link parsing | Medium | Input validation, error handling |
| Callback construction | Low | URL encoding, signature verification by recipient |

### Homeserver Attack Surface

| Component | Risk | Mitigation |
|-----------|------|------------|
| Public data tampering | Medium | Signature verification on critical data |
| Session hijacking | Low | Session secrets are opaque, TLS protects transmission |
| Handoff payload access | Low | Unguessable path, 5-minute TTL, immediate deletion |

### Push Relay Attack Surface

| Component | Risk | Mitigation |
|-----------|------|------------|
| Token database | High | Encryption at rest, access logging, automatic expiry |
| Wake request flooding | Medium | Rate limiting per-sender and per-recipient |
| Relay operator snooping | Low | End-to-end message encryption (relay only sees pubkeys) |

---

## Cryptographic Protocols

### Noise IK Handshake

```
Initiator (Payer)                  Responder (Payee)
    │                                      │
    │ Has: responder's static pubkey       │ Has: own static keypair
    │                                      │
    │ ──── Handshake Message ────────────> │
    │   e, es, s, ss                       │
    │   [encrypted to responder pubkey]    │
    │                                      │
    │ <─── Handshake Response ──────────── │
    │   e, ee, se                          │
    │                                      │
    │ ──── Encrypted Payment Request ────> │
    │                                      │
    │ <─── Encrypted Payment Response ──── │
    │                                      │
```

**Security Properties**:
- **Mutual authentication**: Both parties prove key ownership
- **Forward secrecy**: Ephemeral keys mean past sessions can't be decrypted
- **Replay protection**: Handshake state prevents message replay

### Homeserver Authentication

**Session Creation**:
```
1. Ring constructs auth message: "<homeserver_pubkey>:<timestamp>:<capabilities>"
2. Ring signs with Ed25519: signature = Ed25519.sign(secret_key, message)
3. Ring sends signed request to homeserver
4. Homeserver verifies signature against Ed25519 public key
5. Homeserver issues session token (opaque)
```

**Session Usage**:
```
Cookie: session=<opaque_session_token>
```

**Security Notes**:
- Session tokens are opaque (homeserver implementation-defined)
- Bitkit treats them as opaque blobs
- No assumption about token format or entropy

---

## Rate Limiting Strategy

### Noise Handshakes (Client-Side)

**Implementation**: `NoiseConnectionRateLimiter` in `NoisePaymentService`

```swift
class NoiseConnectionRateLimiter {
    private var handshakeAttempts: [String: [Date]] = [:]
    private let maxAttemptsPerMinute = 10
    
    func checkRateLimit(for recipientPubkey: String) throws {
        let now = Date()
        let oneMinuteAgo = now.addingTimeInterval(-60)
        
        handshakeAttempts[recipientPubkey]?.removeAll { $0 < oneMinuteAgo }
        
        let recent = handshakeAttempts[recipientPubkey]?.count ?? 0
        guard recent < maxAttemptsPerMinute else {
            throw NoisePaymentError.rateLimited("Too many handshake attempts")
        }
        
        handshakeAttempts[recipientPubkey, default: []].append(now)
    }
}
```

**Prevents**: Accidental DoS from buggy retry logic

### Push Relay (Server-Side)

| Limit Type | Threshold | Window | Action |
|------------|-----------|--------|--------|
| Per-sender | 10 requests | 1 minute | HTTP 429 |
| Per-sender | 100 requests | 1 hour | HTTP 429 |
| Per-recipient | 100 requests | 1 hour | HTTP 429 |
| Global | Infrastructure-dependent | N/A | Load balancing |

**Prevents**:
- Single attacker spamming many victims (per-sender limit)
- Targeted attacks on specific user (per-recipient limit)
- Infrastructure overload (global limit)

---

## Secret Memory Handling

### Current Implementation

**iOS**:
- Secrets stored as `String` in memory (not ideal)
- Keychain provides at-rest protection
- TODO: Implement zeroization on dealloc

**Android**:
- Secrets stored as `String` in memory (not ideal)
- EncryptedSharedPreferences for at-rest protection
- TODO: Use `ByteArray` and explicit zeroization

### Future Enhancement

```swift
// iOS
class SecureSecret {
    private var data: Data
    
    init(_ value: Data) {
        self.data = value
    }
    
    deinit {
        // Zero out memory before dealloc
        data.withUnsafeMutableBytes { bytes in
            memset(bytes.baseAddress, 0, bytes.count)
        }
    }
}
```

```kotlin
// Android
class SecureSecret(private var data: ByteArray) {
    fun use(block: (ByteArray) -> Unit) {
        block(data)
    }
    
    fun destroy() {
        data.fill(0)
    }
}
```

---

## Threat Scenarios & Mitigations

### Scenario 1: URL Logging Leak

**Attack**: Deep link URLs logged to analytics/crash reports

**Old Risk**: Session secrets exposed in callback URLs  
**Mitigation**: Secure handoff protocol - only `request_id` in URL

**Residual Risk**: `request_id` still logged  
**Acceptable**: Request ID alone reveals only encrypted ciphertext; decryption requires Bitkit's ephemeral secret key

### Scenario 1b: Public Homeserver Storage

**Attack**: Attacker reads `/pub/` paths on homeserver (publicly accessible)

**Old Risk**: Handoff payloads, payment requests, and subscription proposals contained plaintext secrets  
**Mitigation**: All secret-bearing data encrypted using Paykit Sealed Blob v1

**Residual Risk**: Metadata exposure (path structure reveals sender/recipient pubkeys)  
**Acceptable**: Pubkeys are already public; no secret data exposed

### Scenario 2: Push Token DoS

**Attack**: Attacker reads public push token, spams notifications

**Old Risk**: Tokens published to `/pub/paykit.app/v0/push`  
**Mitigation**: Push relay with server-side token storage

**Residual Risk**: Relay operator could abuse tokens  
**Acceptable**: Relay is trusted infrastructure (same trust as APNs/FCM)

### Scenario 3: Bitkit Compromise

**Attack**: Malware on device extracts secrets from Bitkit

**Impact**:
- X25519 keys stolen → Can impersonate for current epoch
- Session secrets stolen → Can access homeserver until revoked
- Ed25519 secret safe → Identity NOT compromised

**Mitigation**:
- Key rotation limits X25519 exposure window
- Session revocation via Ring
- Ed25519 stays in Ring (separate app with separate sandbox)

**Recovery**:
1. User revokes sessions in Ring
2. User rotates X25519 keys (new epoch)
3. Identity remains intact

### Scenario 4: Man-in-the-Middle

**Attack**: Network attacker intercepts homeserver communication

**Mitigation**:
- HTTPS with certificate validation
- Signature verification on critical data
- Noise protocol provides end-to-end encryption for payments

**Residual Risk**: Compromised CA or platform PKI  
**Acceptable**: Out of scope (system-level trust assumption)

### Scenario 5: Malicious Homeserver

**Attack**: Homeserver operator tampers with user data

**Mitigations**:
- Public data is public (tampering detectable via signatures)
- Private data encrypted client-side before upload
- Can switch homeservers (data portability)

**Residual Risk**: Homeserver sees access patterns  
**Acceptable**: Homeserver is chosen by user (implicit trust)

---

## Security Checklist for Production

### Code Audit

- [ ] No Ed25519 secrets in Bitkit codebase (grep for "secret.*key", "derive.*ed25519")
- [ ] All Ring requests use deep links (no local key operations)
- [ ] Session secrets not logged (check Logger calls)
- [x] Push tokens not published publicly (public directory methods removed)
- [ ] TLS certificate validation enabled
- [ ] Input validation on all deep link parameters

### Configuration Audit

- [ ] ProGuard/R8 rules preserve security-critical classes
- [ ] Network security config allows only HTTPS
- [ ] Debug logging disabled in production builds
- [ ] Homeserver URLs configured correctly
- [ ] Push relay URL configured correctly

### Runtime Audit

- [ ] Keychain/EncryptedSharedPreferences used for all secrets
- [ ] Memory zeroization on secret deallocation (future)
- [ ] Rate limiting active on Noise connections
- [ ] Session expiry handled correctly
- [ ] Key rotation triggers when epoch 1 available

### Infrastructure Audit

- [ ] Push relay deployed with proper APNs/FCM credentials
- [ ] Rate limiting configured at relay level
- [ ] Token encryption at rest on relay
- [ ] Homeserver honors TTL for handoff payloads
- [ ] Monitoring/alerting for security events

---

## Future Security Enhancements

### Short Term (Next Release)

1. **Memory Zeroization**: Zero out secrets on deallocation
2. **Session Expiry Tracking**: Add `isExpired` and proactive renewal
3. **Rate Limiting**: Add `NoiseConnectionRateLimiter` to production code
4. **401/403 Re-auth Flow**: Automatic session refresh on auth failures

### Medium Term (3-6 Months)

1. **Certificate Pinning**: Pin homeserver and relay certificates
2. **Encrypted Relay Registration**: Hide pubkey→token mapping from relay
3. **Anonymous Wake**: Onion routing for sender anonymity
4. **Hardware Security**: Use Secure Enclave (iOS) / StrongBox (Android) for X25519 keys

### Long Term (6-12 Months)

1. **Zero-Knowledge Proofs**: Prove relay authorization without revealing pubkey
2. **Threshold Signatures**: Split Ed25519 key across multiple devices
3. **Homomorphic Wake Tokens**: Wake without revealing recipient to relay
4. **Decentralized Relay Network**: No single point of trust/failure

---

## Security Contacts

For security issues, contact:
- **Paykit**: [security contact TBD]
- **Pubky**: [security contact TBD]
- **Bitkit**: [security contact TBD]

---

**Document Version**: 1.0  
**Last Updated**: December 22, 2025  
**Status**: Reference Implementation - Audit Required Before Production

