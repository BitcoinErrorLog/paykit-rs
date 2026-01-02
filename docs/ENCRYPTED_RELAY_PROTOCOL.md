# Encrypted Relay Handoff Protocol

**Version**: 1.0  
**Date**: January 2026  
**Status**: Active

## Overview

This document specifies the encrypted relay handoff protocol used for cross-device authentication between Bitkit and Pubky Ring. The protocol ensures that no Paykit secrets (session secrets, Noise private keys, noise_seed) are ever transmitted in plaintext via callback URLs.

## Security Requirements

1. **No secrets in callback URLs**: Callback URLs contain only reference IDs
2. **Secrets encrypted at rest**: Relay stores only encrypted sealed blobs
3. **Forward secrecy**: Ephemeral X25519 keypairs ensure session compromise doesn't expose past handoffs
4. **Short TTL**: Encrypted payloads expire within 5 minutes
5. **Single-use**: Payloads are deleted after successful fetch

## Protocol Variants

### Variant A: Same-Device Secure Handoff (Homeserver)

Used when Bitkit and Ring are on the same mobile device.

**Storage Location**: User's homeserver at `/pub/paykit.app/v0/handoff/{requestId}`

**Flow**:
1. Bitkit generates ephemeral X25519 keypair
2. Bitkit sends `pubkyring://paykit-connect?deviceId=...&callback=...&ephemeralPk=<hex>`
3. Ring signs in, derives session + noise keys + noise_seed
4. Ring encrypts payload using `sealedBlobEncrypt(ephemeralPk, payload, aad, "handoff")`
5. Ring stores encrypted envelope on homeserver
6. Ring calls back: `bitkit://paykit-setup?pubky=...&request_id=...&mode=secure_handoff`
7. Bitkit fetches, decrypts with ephemeral secret key, persists, deletes blob

**AAD Format (Paykit v0)**: `paykit:v0:handoff:{pubky}:{storagePath}:{requestId}`

### Variant B: Cross-Device Secure Relay (pubkyauth)

Used when Bitkit and Ring are on different devices (e.g., desktop browser + mobile phone).

**Storage Location**: HTTP relay at `{relayUrl}/{channelId}`

**Flow**:
1. Bitkit generates 32-byte `client_secret` and computes `channelId = hash(client_secret)`
2. Bitkit subscribes to relay at `channelId`
3. Bitkit displays QR: `pubkyauth:///?relay=...&caps=...&secret=<base64url(client_secret)>`
4. Ring scans QR, shows consent form, user approves
5. Ring creates AuthToken, encrypts with `client_secret`
6. Ring sends encrypted token to `relay/{channelId}`
7. Relay forwards to Bitkit
8. Bitkit decrypts, presents AuthToken to homeserver for session

**Note**: This flow is already secure because the `client_secret` is exchanged via physical QR scan and the token is encrypted before relay transmission.

### Variant C: Cross-Device Sealed Blob Relay (Bitkit custom)

Used when Bitkit wants full Paykit setup (session + noise keys + noise_seed) via cross-device QR, using the same encryption model as Variant A but with HTTP relay storage.

**Storage Location**: HTTP relay at `{sessionRelayUrl}/{requestId}`

**Flow**:
1. Bitkit generates ephemeral X25519 keypair
2. Bitkit displays QR: `https://pubky.app/auth?request_id=...&ephemeralPk=...&relay_url=...`
3. Ring/web scans QR, signs in, derives session + noise keys + noise_seed
4. Ring encrypts payload using `sealedBlobEncrypt(ephemeralPk, payload, aad, "handoff")`
5. Ring PUTs encrypted envelope to `{relay_url}/{request_id}`
6. Bitkit polls relay, receives encrypted blob
7. Bitkit decrypts with ephemeral secret key, persists, zeroizes ephemeral key

**AAD Format (Paykit v0)**: `paykit:v0:relay:session:{requestId}`

**SECURITY**: The relay only stores encrypted blobs. Bitkit rejects plaintext responses.

## Payload Structure

### Handoff Payload (before encryption)

```json
{
  "version": 1,
  "pubky": "8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty",
  "session_secret": "...",
  "capabilities": ["read", "write"],
  "device_id": "bitkit-device-uuid",
  "noise_keypairs": [
    { "epoch": 0, "public_key": "...", "secret_key": "..." },
    { "epoch": 1, "public_key": "...", "secret_key": "..." }
  ],
  "noise_seed": "...",
  "created_at": 1735830000000,
  "expires_at": 1735830300000
}
```

### Sealed Blob v1 Envelope (stored)

```json
{
  "v": 1,
  "epk": "<base64url sender ephemeral public key>",
  "nonce": "<base64url 12-byte nonce>",
  "ct": "<base64url ciphertext + tag>",
  "purpose": "handoff"
}
```

## Deprecated Flows (DISABLED)

The following legacy flows are **disabled** for security reasons:

### Legacy Plaintext Callback (REJECTED)

```
bitkit://paykit-cross-session?pubky=...&session_secret=...
```

**Status**: ❌ REJECTED - Handler logs error and throws exception

**Reason**: Secrets exposed in URL (system logs, URL handlers, clipboard)

### Legacy Plaintext Homeserver Storage (REJECTED)

```json
PUT /pub/paykit.app/v0/handoff/{requestId}
{
  "session_secret": "...",
  "noise_keypairs": [{"secret_key": "..."}]
}
```

**Status**: ❌ REJECTED - Bitkit requires `is_sealed_blob()` check to pass

**Reason**: Secrets readable by anyone who knows the path

## Security Properties

| Property | Same-Device Handoff | Cross-Device Relay |
|----------|--------------------|--------------------|
| Secrets encrypted | ✅ Sealed Blob v1 | ✅ client_secret |
| No URL secrets | ✅ reference only | ✅ reference only |
| Forward secrecy | ✅ ephemeral X25519 | ⚠️ per-QR secret |
| TTL enforcement | ✅ 5 minutes | ✅ 45 seconds |
| Single-use deletion | ✅ Bitkit deletes | ✅ Relay forwards once |

## Implementation Checklist

- [x] Ring: Encrypt same-device handoff with Sealed Blob v1
- [x] Bitkit: Reject plaintext homeserver handoff payloads
- [x] Bitkit: Reject plaintext cross-device callback URLs
- [x] Cross-device relay: Already uses client_secret encryption
- [x] Documentation: Updated SECURITY_ARCHITECTURE.md

