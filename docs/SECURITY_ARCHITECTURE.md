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
| X25519 Epoch 0 | 30-90 days | Bitkit cache | To Epoch 1 |
| X25519 Epoch 1 | 30-90 days | Bitkit cache | Request new epochs |
| Session Secret | Until revoked | Bitkit secure storage | On 401/403 |

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

**Attack Scenario**: URL logging/leaks expose session secrets

**Example Attack Vectors**:
- Web server access logs
- Browser history
- Deep link analytics
- URL sharing/forwarding

### Mitigation: Encrypted Handoff

**Old (Insecure)**:
```
bitkit://paykit-setup?
  session_secret=abc123...&
  noise_secret_key_0=def456...
```
☠️ **Secrets exposed in URL!**

**New (Secure)**:
```
bitkit://paykit-setup?
  mode=secure_handoff&
  pubky=8um71us...&
  request_id=f3a7b2c...
```
✅ **Only request ID in URL - secrets on homeserver**

### Security Properties

1. **Unguessable Path**: 256-bit random `request_id` = 2^256 possible paths
2. **Time-Limited**: 5-minute TTL in `expires_at` field
3. **Single-Use**: Deleted immediately after successful fetch
4. **Access Control**: Requires knowing exact `request_id` (no directory listing)

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
**Acceptable**: Request ID alone is useless without homeserver access + 5-minute window

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
- [ ] Push tokens not published publicly (deprecated methods removed)
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

