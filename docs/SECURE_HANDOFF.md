# Secure Handoff Protocol

> **Version**: 1.0  
> **Last Updated**: January 2, 2026  
> **Status**: Production

This document specifies the secure handoff protocol between Pubky Ring and Bitkit for provisioning identity sessions and Noise keypairs.

**Related Documents**:
- [SEALED_BLOB_V1_SPEC.md](SEALED_BLOB_V1_SPEC.md) - Encryption envelope format
- [ENCRYPTED_RELAY_PROTOCOL.md](ENCRYPTED_RELAY_PROTOCOL.md) - Cross-device relay protocol
- [PAYKIT_PROTOCOL_V0.md](PAYKIT_PROTOCOL_V0.md) - Paykit v0 specification

---

## Table of Contents

1. [Overview](#1-overview)
2. [Security Goals](#2-security-goals)
3. [Protocol Flow](#3-protocol-flow)
4. [Request Format](#4-request-format)
5. [Payload Format](#5-payload-format)
6. [AAD Construction](#6-aad-construction)
7. [Implementation Details](#7-implementation-details)
8. [Error Handling](#8-error-handling)
9. [Security Considerations](#9-security-considerations)

---

## 1. Overview

Secure Handoff enables Bitkit to receive:
- Pubky session credentials (for homeserver authentication)
- Noise keypairs (for encrypted payment channels)
- Noise seed (for local key derivation)

All secret material is encrypted using Sealed Blob v1 before storage, ensuring secrets are never exposed in URLs or readable on public storage.

### Actors

| Actor | Role |
|-------|------|
| **Bitkit** | Payment wallet requesting credentials |
| **Ring** | Identity manager holding Ed25519 master key |
| **Homeserver** | Storage for encrypted handoff payload |

---

## 2. Security Goals

| Goal | How Achieved |
|------|--------------|
| No secrets in URLs | Only `request_id` in callback, not session/keys |
| Encrypted at rest | Sealed Blob v1 encryption on homeserver |
| Forward secrecy | Ephemeral X25519 keypair per handoff |
| Time-limited exposure | 5-minute expiration on handoff payload |
| Single-use | Bitkit deletes payload after fetch |
| No legacy fallback | Plaintext payloads are rejected |

---

## 3. Protocol Flow

### Step 1: Bitkit Initiates Request

Bitkit generates an ephemeral X25519 keypair and opens Ring:

```
pubkyring://paykit-connect
  ?deviceId={device_id}
  &callback={urlencoded bitkit://paykit-setup}
  &ephemeralPk={hex_encoded_x25519_public_key}
  &includeEpoch1=true
```

### Step 2: Ring Processes Request

Ring:
1. Authenticates with homeserver (if needed)
2. Derives Noise keypairs from Ed25519 seed
3. Constructs handoff payload (JSON)
4. Generates random 256-bit `request_id`
5. Encrypts payload using Sealed Blob v1:
   - Recipient: Bitkit's `ephemeralPk`
   - AAD: `paykit:v0:handoff:{pubky}:{path}:{request_id}`
6. Stores encrypted envelope at `/pub/paykit.app/v0/handoff/{request_id}`

### Step 3: Ring Returns to Bitkit

Ring opens Bitkit with callback:

```
bitkit://paykit-setup
  ?mode=secure_handoff
  &pubky={z32_pubkey}
  &request_id={256bit_hex}
```

**No secrets in the URL.**

### Step 4: Bitkit Fetches and Decrypts

Bitkit:
1. Constructs storage path: `pubky://{pubky}/pub/paykit.app/v0/handoff/{request_id}`
2. Fetches encrypted envelope
3. Verifies it's a Sealed Blob (`v`, `epk`, `nonce`, `ct` fields present)
4. Constructs AAD matching Ring's encryption
5. Decrypts using ephemeral secret key
6. Parses handoff payload
7. **Immediately deletes** the remote handoff file
8. Zeroizes ephemeral secret key
9. Caches session and Noise keys locally

### Sequence Diagram

```
┌──────────┐                    ┌──────────┐                    ┌────────────┐
│  Bitkit  │                    │   Ring   │                    │ Homeserver │
└────┬─────┘                    └────┬─────┘                    └─────┬──────┘
     │                               │                                │
     │  Generate ephemeral keypair   │                                │
     │ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─  │                                │
     │                               │                                │
     │  pubkyring://paykit-connect   │                                │
     │  + deviceId + callback        │                                │
     │  + ephemeralPk                │                                │
     │ ─────────────────────────────>│                                │
     │                               │                                │
     │                               │  Sign in / get session         │
     │                               │ ───────────────────────────────>
     │                               │                                │
     │                               │  Derive Noise keys             │
     │                               │ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │
     │                               │                                │
     │                               │  Encrypt payload to ephemeralPk│
     │                               │ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │
     │                               │                                │
     │                               │  PUT /pub/paykit.app/v0/       │
     │                               │       handoff/{request_id}     │
     │                               │ ───────────────────────────────>
     │                               │                                │
     │  bitkit://paykit-setup        │                                │
     │  + mode=secure_handoff        │                                │
     │  + pubky + request_id         │                                │
     │ <─────────────────────────────│                                │
     │                               │                                │
     │  GET /pub/paykit.app/v0/      │                                │
     │       handoff/{request_id}    │                                │
     │ ────────────────────────────────────────────────────────────────>
     │                               │                                │
     │  <encrypted envelope>         │                                │
     │ <────────────────────────────────────────────────────────────────
     │                               │                                │
     │  Decrypt with ephemeral SK    │                                │
     │ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─  │                                │
     │                               │                                │
     │  DELETE /pub/paykit.app/v0/   │                                │
     │         handoff/{request_id}  │                                │
     │ ────────────────────────────────────────────────────────────────>
     │                               │                                │
     │  Cache session + keys         │                                │
     │ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─  │                                │
     │                               │                                │
```

---

## 4. Request Format

### URL Scheme

```
pubkyring://paykit-connect
  ?deviceId={device_id}
  &callback={urlencoded_callback}
  &ephemeralPk={hex_public_key}
  [&includeEpoch1={true|false}]
```

### Parameters

| Parameter | Required | Description |
|-----------|----------|-------------|
| `deviceId` | Yes | Unique device identifier (hex string) |
| `callback` | Yes | URL-encoded callback URL (e.g., `bitkit://paykit-setup`) |
| `ephemeralPk` | **Yes** | Hex-encoded X25519 public key (64 hex chars) |
| `includeEpoch1` | No | Include epoch 1 keypair (default: `true`) |

### Security: `ephemeralPk` is Required

**Ring MUST reject requests without `ephemeralPk`.**

This ensures:
- All handoff payloads are encrypted
- No plaintext secrets on homeserver
- No legacy unencrypted handoffs

---

## 5. Payload Format

### Handoff Payload (JSON, before encryption)

```json
{
  "version": 1,
  "pubky": "8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty",
  "session_secret": "TVQB9B07VD...",
  "capabilities": ["read", "write"],
  "device_id": "a1b2c3d4e5f6...",
  "noise_keypairs": [
    {
      "epoch": 0,
      "public_key": "abcd1234...",
      "secret_key": "efgh5678..."
    },
    {
      "epoch": 1,
      "public_key": "ijkl9012...",
      "secret_key": "mnop3456..."
    }
  ],
  "noise_seed": "qrst7890...",
  "created_at": 1704153600000,
  "expires_at": 1704153900000
}
```

### Field Definitions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | integer | Yes | Payload version (must be `1`) |
| `pubky` | string | Yes | z-base-32 encoded Ed25519 public key |
| `session_secret` | string | Yes | Homeserver session secret |
| `capabilities` | array | Yes | Session capabilities |
| `device_id` | string | Yes | Device ID used for key derivation |
| `noise_keypairs` | array | Yes | Array of epoch-indexed keypairs |
| `noise_seed` | string | No | 32-byte seed for local key derivation (hex) |
| `created_at` | integer | Yes | Unix timestamp (ms) of creation |
| `expires_at` | integer | Yes | Unix timestamp (ms) of expiration |

### Noise Keypair Object

| Field | Type | Description |
|-------|------|-------------|
| `epoch` | integer | Key rotation epoch (0, 1, 2, ...) |
| `public_key` | string | X25519 public key (64 hex chars) |
| `secret_key` | string | X25519 secret key (64 hex chars) |

---

## 6. AAD Construction

The AAD binds the encrypted blob to its storage context.

### Format

```
paykit:v0:handoff:{owner_pubkey}:{storage_path}:{request_id}
```

### Example

```
paykit:v0:handoff:8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty:/pub/paykit.app/v0/handoff/f3a7b2c1d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1:f3a7b2c1d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1
```

---

## 7. Implementation Details

### Bitkit (iOS)

**Files:**
- `bitkit-ios/Bitkit/PaykitIntegration/Services/PubkyRingBridge.swift`
- `bitkit-ios/Bitkit/PaykitIntegration/Services/SecureHandoffHandler.swift`

**Key Methods:**
```swift
// Generate ephemeral keypair
let (ephemeralSk, ephemeralPk) = X25519.generateKeypair()

// Build request URL
let url = "pubkyring://paykit-connect?deviceId=\(deviceId)&callback=\(encodedCallback)&ephemeralPk=\(ephemeralPk.hexString)"

// Handle callback
func handleSecureHandoff(pubky: String, requestId: String) async throws {
    let path = "/pub/paykit.app/v0/handoff/\(requestId)"
    let envelope = try await pubkySDK.publicGet(pubky: pubky, path: path)
    
    guard isSealedBlob(envelope) else {
        throw SecureHandoffError.plaintextRejected
    }
    
    let aad = "paykit:v0:handoff:\(pubky):\(path):\(requestId)"
    let payload = try sealedBlobDecrypt(ephemeralSk, envelope, aad)
    
    // Delete immediately
    try await pubkySDK.sessionDelete(pubky: pubky, path: path)
    
    // Cache credentials
    try cacheSession(payload)
    try cacheNoiseKeys(payload)
}
```

### Bitkit (Android)

**Files:**
- `bitkit-android/app/src/main/java/to/bitkit/paykit/services/PubkyRingBridge.kt`
- `bitkit-android/app/src/main/java/to/bitkit/paykit/services/SecureHandoffHandler.kt`

**Key Methods:**
```kotlin
// Generate ephemeral keypair
val (ephemeralSk, ephemeralPk) = X25519.generateKeypair()

// Build request URL
val url = "pubkyring://paykit-connect?deviceId=$deviceId&callback=$encodedCallback&ephemeralPk=${ephemeralPk.toHex()}"

// Handle callback
suspend fun handleSecureHandoff(pubky: String, requestId: String) {
    val path = "/pub/paykit.app/v0/handoff/$requestId"
    val envelope = pubkySDK.publicGet(pubky, path)
    
    require(isSealedBlob(envelope)) { "Plaintext handoffs rejected" }
    
    val aad = "paykit:v0:handoff:$pubky:$path:$requestId"
    val payload = sealedBlobDecrypt(ephemeralSk, envelope, aad)
    
    // Delete immediately
    pubkySDK.sessionDelete(pubky, path)
    
    // Cache credentials
    cacheSession(payload)
    cacheNoiseKeys(payload)
}
```

### Ring (TypeScript)

**File:** `pubky-ring/src/utils/actions/paykitConnectAction.ts`

```typescript
export const handlePaykitConnectAction = async (
    data: PaykitConnectActionData,
    context: ActionContext
) => {
    const { pubky, dispatch } = context;
    const { deviceId, callback, ephemeralPk, includeEpoch1 = true } = data.params;

    // SECURITY: ephemeralPk is REQUIRED
    if (!ephemeralPk) {
        throw new Error('ephemeralPk required for secure handoff');
    }

    // Get session and derive keys...
    const payload = buildPayload(session, noiseKeypairs, deviceId);
    
    // Generate random request ID
    const requestId = crypto.randomBytes(32).toString('hex');
    
    // Build AAD
    const storagePath = `/pub/paykit.app/v0/handoff/${requestId}`;
    const aad = `paykit:v0:handoff:${pubky}:${storagePath}:${requestId}`;
    
    // Encrypt using Sealed Blob v1
    const envelope = await sealedBlobEncrypt(
        Buffer.from(ephemeralPk, 'hex'),
        JSON.stringify(payload),
        aad
    );
    
    // Store on homeserver
    await put(`pubky://${pubky}${storagePath}`, envelope);
    
    // Return to Bitkit (no secrets in URL!)
    const callbackUrl = `${callback}?mode=secure_handoff&pubky=${pubky}&request_id=${requestId}`;
    await Linking.openURL(callbackUrl);
};
```

---

## 8. Error Handling

### Bitkit Errors

| Error | Code | Handling |
|-------|------|----------|
| Missing `ephemeralPk` in response | E001 | Reject handoff, prompt user to update Ring |
| Plaintext payload received | E002 | Reject, log security warning |
| Expired payload | E003 | Reject, prompt user to retry |
| Decryption failed | E004 | Reject, prompt user to retry |
| Fetch failed | E005 | Retry with backoff, then prompt user |

### Ring Errors

| Error | Code | Handling |
|-------|------|----------|
| Missing `ephemeralPk` in request | E101 | Reject, do not return secrets |
| Session creation failed | E102 | Display error to user |
| Key derivation failed | E103 | Display error to user |
| Storage failed | E104 | Display error, prompt retry |

---

## 9. Security Considerations

### Threat Mitigations

| Threat | Mitigation |
|--------|------------|
| URL logging exposing secrets | Secrets never in URLs |
| Homeserver operator reading secrets | Sealed Blob v1 encryption |
| Replay attack | Unique `request_id` per handoff |
| Blob relocation | AAD binding to path and owner |
| Key compromise | Ephemeral X25519 per handoff |
| Stale handoffs | 5-minute expiration + immediate deletion |

### Invariants

1. **No secrets in callback URLs**: Only `mode`, `pubky`, `request_id`
2. **Plaintext rejected**: Bitkit's `isSealedBlob()` check is mandatory
3. **Ephemeral key required**: Ring rejects requests without `ephemeralPk`
4. **Immediate deletion**: Bitkit deletes handoff after successful decrypt
5. **Zeroization**: Ephemeral secret key zeroized after use

### Audit Checklist

- [ ] `ephemeralPk` parameter is validated (not empty, correct length)
- [ ] Plaintext detection returns `false` for all non-sealed-blob formats
- [ ] AAD construction matches between Ring and Bitkit exactly
- [ ] Handoff file is deleted after successful decryption
- [ ] Ephemeral secret key is zeroized after decryption
- [ ] Expiration timestamp is validated before use
- [ ] Session and keys are stored in platform keychain/keystore

---

*This specification is maintained in the [BitcoinErrorLog/paykit-rs](https://github.com/BitcoinErrorLog/paykit-rs) repository.*

