# Paykit Sealed Blob v1 Specification

This document specifies the encrypted envelope format used for storing secret-bearing data on Pubky homeservers. Since all data under `/pub/` is publicly readable, any sensitive payload must be encrypted before storage.

## Table of Contents

1. [Overview](#overview)
2. [Cryptographic Primitives](#cryptographic-primitives)
3. [Envelope Format](#envelope-format)
4. [AAD Construction](#aad-construction)
5. [Operations](#operations)
6. [Error Handling](#error-handling)
7. [Versioning](#versioning)
8. [Security Considerations](#security-considerations)

---

## Overview

The Paykit Sealed Blob is an authenticated encryption envelope that allows:

1. **Handoff payloads**: Ring encrypts session secrets and Noise keys for Bitkit
2. **Payment requests**: Sender encrypts request for recipient's Noise public key
3. **Subscription proposals**: Sender encrypts proposal for recipient's Noise public key

All blobs use **ephemeral-static ECDH** (sender generates ephemeral keypair, encrypts to recipient's static public key).

---

## Cryptographic Primitives

| Primitive | Algorithm | Library |
|-----------|-----------|---------|
| Key Agreement | X25519 ECDH | `x25519-dalek` |
| Key Derivation | HKDF-SHA256 | `hkdf` crate |
| AEAD | ChaCha20-Poly1305 | `chacha20poly1305` crate |
| Nonce | 12 bytes random | `rand` crate |

### Key Derivation

```
shared_secret = X25519(sender_ephemeral_sk, recipient_static_pk)
salt          = sender_ephemeral_pk || recipient_static_pk  (64 bytes)
info          = b"paykit-sealed-blob-v1"
key           = HKDF-SHA256(salt, shared_secret, info, 32)
```

### Nonce Generation

- 12 bytes from cryptographically secure RNG
- Never reused for the same key (ephemeral key ensures uniqueness)

---

## Envelope Format

The envelope is a JSON object with the following fields:

```json
{
  "v": 1,
  "epk": "<base64url-encoded sender ephemeral public key, 32 bytes>",
  "nonce": "<base64url-encoded nonce, 12 bytes>",
  "ct": "<base64url-encoded ciphertext + 16-byte Poly1305 tag>"
}
```

### Field Definitions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `v` | integer | Yes | Version number. Must be `1` for this spec. |
| `epk` | string | Yes | Sender's ephemeral X25519 public key, base64url-encoded (no padding). |
| `nonce` | string | Yes | 12-byte nonce, base64url-encoded (no padding). |
| `ct` | string | Yes | Ciphertext concatenated with 16-byte Poly1305 authentication tag, base64url-encoded (no padding). |

### Optional Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `kid` | string | No | Key identifier: first 8 bytes of SHA-256(recipient_pk), hex-encoded. Allows recipient to select correct decryption key when multiple keys are cached. |
| `purpose` | string | No | Human-readable purpose hint: `"handoff"`, `"request"`, `"proposal"`. Informational only; not authenticated. |

### Encoding Rules

- **Base64url**: RFC 4648 §5, without padding (`=`)
- **JSON**: UTF-8 encoded, no BOM, compact (no extra whitespace in production)
- **Key order**: Fields should appear in order: `v`, `epk`, `nonce`, `ct`, then optional fields

### Size Limits

| Component | Maximum Size |
|-----------|--------------|
| Plaintext | 64 KiB |
| Envelope JSON | 100 KiB |

---

## AAD Construction

**Associated Authenticated Data (AAD)** binds the ciphertext to its storage context, preventing blob relocation attacks.

### AAD Format

```
aad = <purpose>:<owner_pubkey>:<path>
```

Where:
- `purpose`: One of `handoff`, `request`, `proposal`
- `owner_pubkey`: The z32-encoded Ed25519 public key of the homeserver owner where the blob is stored
- `path`: The full storage path (e.g., `/pub/paykit.app/v0/handoff/abc123`)

### AAD Examples

**Handoff** (Ring → Bitkit):
```
handoff:8um71us...xyz:/pub/paykit.app/v0/handoff/f3a7b2c1d4e5
```

**Payment Request** (Alice → Bob):
```
request:o1j5bz6...abc:/pub/paykit.app/v0/requests/o1j5bz6...abc/req_123
```

**Subscription Proposal** (Service → User):
```
proposal:8um71us...xyz:/pub/paykit.app/v0/subscriptions/proposals/8um71us...xyz/prop_456
```

### AAD Validation

On decryption:
1. Recipient reconstructs AAD from known context (purpose, owner, path)
2. Decryption with wrong AAD fails with authentication error
3. This prevents an attacker from copying a blob to a different path

---

## Operations

### Encrypt (Seal)

**Inputs**:
- `recipient_pk`: Recipient's X25519 public key (32 bytes)
- `plaintext`: Data to encrypt (≤64 KiB)
- `aad`: Associated data string (see AAD Construction)

**Algorithm**:
```
1. Generate ephemeral X25519 keypair: (epk, esk)
2. Compute shared_secret = X25519(esk, recipient_pk)
3. Derive key via HKDF:
   salt = epk || recipient_pk
   key = HKDF-SHA256(salt, shared_secret, b"paykit-sealed-blob-v1", 32)
4. Generate random 12-byte nonce
5. Encrypt: ct = ChaCha20-Poly1305.seal(key, nonce, plaintext, aad)
6. Zeroize: esk, shared_secret, key
7. Return Envelope { v: 1, epk, nonce, ct }
```

**Output**: JSON-encoded envelope

### Decrypt (Open)

**Inputs**:
- `recipient_sk`: Recipient's X25519 secret key (32 bytes)
- `envelope`: JSON-encoded envelope
- `aad`: Associated data string (must match encryption)

**Algorithm**:
```
1. Parse envelope JSON
2. Verify v == 1
3. Decode epk, nonce, ct from base64url
4. Compute shared_secret = X25519(recipient_sk, epk)
5. Derive key via HKDF:
   salt = epk || recipient_pk  (where recipient_pk = X25519_public_from_secret(recipient_sk))
   key = HKDF-SHA256(salt, shared_secret, b"paykit-sealed-blob-v1", 32)
6. Decrypt: plaintext = ChaCha20-Poly1305.open(key, nonce, ct, aad)
7. Zeroize: shared_secret, key
8. Return plaintext
```

**Output**: Decrypted plaintext or error

---

## Error Handling

### Error Codes

| Error | Code | Description |
|-------|------|-------------|
| `UNSUPPORTED_VERSION` | E001 | Envelope `v` field is not `1` |
| `MALFORMED_ENVELOPE` | E002 | JSON parsing failed or required fields missing |
| `INVALID_BASE64` | E003 | Base64url decoding failed for epk/nonce/ct |
| `INVALID_KEY_SIZE` | E004 | epk is not 32 bytes after decoding |
| `INVALID_NONCE_SIZE` | E005 | nonce is not 12 bytes after decoding |
| `DECRYPTION_FAILED` | E006 | ChaCha20-Poly1305 authentication failed |
| `PLAINTEXT_TOO_LARGE` | E007 | Plaintext exceeds 64 KiB limit |

### Error Behavior

1. **Never reveal reason for decryption failure** beyond "decryption failed"
   - Wrong key, wrong AAD, and tampered ciphertext all return `DECRYPTION_FAILED`
   - This prevents oracle attacks

2. **Parse errors are distinct** from decryption errors
   - Malformed JSON or invalid base64 can be reported specifically
   - These reveal no secret information

3. **Version errors allow graceful upgrade**
   - If `v > 1`, return `UNSUPPORTED_VERSION` with the version number
   - Allows clients to prompt for app update

---

## Versioning

### Current Version: 1

This specification is version 1. The `v` field in the envelope indicates the version.

### Future Versions

When a new version is introduced:
1. Increment `v` field
2. Document changes in this spec
3. Clients should attempt v1 decryption as fallback during migration
4. Old clients return `UNSUPPORTED_VERSION` for unknown versions

### Breaking vs Non-Breaking Changes

**Breaking** (requires version bump):
- Changing AEAD algorithm
- Changing key derivation
- Changing AAD format
- Changing field encodings

**Non-Breaking** (same version):
- Adding optional fields
- Increasing size limits
- Adding new `purpose` values

---

## Security Considerations

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Passive eavesdropper | X25519 ECDH + ChaCha20-Poly1305 encryption |
| Active tampering | Poly1305 authentication tag |
| Blob relocation | AAD binding to path and owner |
| Key reuse attacks | Ephemeral sender key per blob |
| Timing attacks | Constant-time comparison in Poly1305 verification |

### Key Management

1. **Ephemeral keys**: Sender generates fresh X25519 keypair per blob; zeroize after use
2. **Recipient keys**: Static X25519 key (derived from Ed25519 in Ring, or published Noise endpoint key)
3. **Key rotation**: Use `kid` field to help recipients select among multiple cached keys

### Handoff-Specific Security

For handoff blobs (`purpose: "handoff"`):

1. **Ephemeral recipient key**: Bitkit generates one-time X25519 keypair for receiving handoff
2. **Time-limited**: Handoff payloads include `expires_at` in plaintext; Bitkit rejects expired
3. **Single-use**: Bitkit deletes remote blob immediately after successful decryption
4. **No legacy fallback**: Ring rejects handoff requests without `ephemeralPk` parameter

### Request/Proposal Security

For payment requests and subscription proposals:

1. **Recipient Noise key**: Encrypt to recipient's published Noise endpoint public key
2. **Discovery**: Sender fetches recipient's Noise endpoint from `/pub/paykit.app/v0/noise`
3. **Epoch handling**: Try current epoch key first, fall back to previous epoch if `kid` matches
4. **Legacy migration**: Readers accept plaintext during transition (write-encrypted only)

### Memory Safety

1. **Zeroize secrets**: ephemeral secret key, shared secret, derived key must be zeroized after use
2. **No logging**: Never log plaintext, keys, or decrypted content
3. **Secure storage**: Recipient secret keys stored in platform keychain/keystore

---

## Implementation Reference

### Rust (pubky-noise)

```rust
// Seal
pub fn sealed_blob_encrypt(
    recipient_pk: &[u8; 32],
    plaintext: &[u8],
    aad: &str,
) -> Result<String, SealedBlobError>;

// Open
pub fn sealed_blob_decrypt(
    recipient_sk: &[u8; 32],
    envelope_json: &str,
    aad: &str,
) -> Result<Vec<u8>, SealedBlobError>;

// Key generation
pub fn x25519_generate_keypair() -> ([u8; 32], [u8; 32]); // (secret, public)
pub fn x25519_public_from_secret(secret: &[u8; 32]) -> [u8; 32];
```

### Swift (via UniFFI)

```swift
func sealedBlobEncrypt(recipientPk: Data, plaintext: Data, aad: String) throws -> String
func sealedBlobDecrypt(recipientSk: Data, envelopeJson: String, aad: String) throws -> Data
func x25519GenerateKeypair() -> (secret: Data, publicKey: Data)
func x25519PublicFromSecret(secret: Data) -> Data
```

### Kotlin (via UniFFI)

```kotlin
fun sealedBlobEncrypt(recipientPk: ByteArray, plaintext: ByteArray, aad: String): String
fun sealedBlobDecrypt(recipientSk: ByteArray, envelopeJson: String, aad: String): ByteArray
fun x25519GenerateKeypair(): Pair<ByteArray, ByteArray> // (secret, public)
fun x25519PublicFromSecret(secret: ByteArray): ByteArray
```

---

## Test Vectors

### Vector 1: Basic Encryption

**Inputs**:
```
recipient_sk (hex): 0x77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a
recipient_pk (hex): 0x8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a
plaintext (utf8): "hello world"
aad: "handoff:testpubkey123:/pub/paykit.app/v0/handoff/abc"
ephemeral_sk (hex): 0x5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb
nonce (hex): 0x000000000000000000000001
```

**Expected Envelope** (field values, not full JSON due to ephemeral key):
```
v: 1
epk: base64url of 0xde9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f
nonce: base64url of 0x000000000000000000000001
ct: <authenticated ciphertext>
```

Note: Actual test vectors with computed ciphertext will be added during implementation.

---

**Document Version**: 1.0  
**Last Updated**: January 2, 2026  
**Status**: Specification - Implementation Required

