# Paykit Sealed Blob Specification

> **Canonical Reference**: This specification implements the Sealed Blob format defined in
> [PUBKY_CRYPTO_SPEC v2.5](https://github.com/pubky/pubky-core/blob/main/docs/PUBKY_CRYPTO_SPEC.md)
> Section 7.2 (Sealed Blob v2 / SB2). The PUBKY_CRYPTO_SPEC is the authoritative source for:
> - Binary wire format (magic bytes, version, CBOR header, ciphertext)
> - Deterministic CBOR header encoding with integer keys
> - AAD construction: `prefix || owner_peerid || canonical_path || header_bytes`
> - inbox_kid derivation for key selection
> - Signature construction and AppKey delegation via cert_id
>
> This document describes Paykit-specific usage and integration patterns.

This document specifies the encrypted envelope format used for storing secret-bearing data on Pubky homeservers. Since all data under `/pub/` is publicly readable, any sensitive payload must be encrypted before storage.

**Current Format**: SB2 binary wire format (XChaCha20-Poly1305, CBOR header)  
**Legacy Format**: JSON envelope (v1/v2) - decryption only, deprecated for new implementations

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

### Version 2 (Current)

| Primitive | Algorithm | Library |
|-----------|-----------|---------|
| Key Agreement | X25519 ECDH | `x25519-dalek` |
| Key Derivation | HKDF-SHA256 | `hkdf` crate |
| AEAD | XChaCha20-Poly1305 | `chacha20poly1305` crate |
| Nonce | 24 bytes random | `rand` crate |

### Version 1 (Legacy, Decryption Only)

| Primitive | Algorithm |
|-----------|-----------|
| AEAD | ChaCha20-Poly1305 |
| Nonce | 12 bytes random |

### Key Derivation

**v2**:
```
shared_secret = X25519(sender_ephemeral_sk, recipient_static_pk)
salt          = sender_ephemeral_pk || recipient_static_pk  (64 bytes)
info          = b"pubky-envelope/v2"
key           = HKDF-SHA256(salt, shared_secret, info, 32)
```

**v1** (legacy):
```
info          = b"paykit-sealed-blob-v1"
```

### Nonce Generation

- **v2**: 24 bytes from cryptographically secure RNG (XChaCha20 extended nonce)
- **v1** (legacy): 12 bytes random
- Never reused for the same key (ephemeral key ensures uniqueness)

---

## Envelope Format

### SB2 Binary Wire Format (Current)

Per CRYPTO_SPEC Section 7.2, the SB2 binary format is:

```
Wire Format:
  magic: 0x53 0x42 0x32 ("SB2", 3 bytes)
  version: u8 (0x02)
  header_len: u16 (big-endian, MUST be <= 2048 bytes)
  header_bytes: [u8; header_len] (deterministic CBOR)
  ciphertext: [u8] (remainder, includes 16-byte Poly1305 tag)
```

### CBOR Header Fields (Integer Keys)

The header is a deterministic CBOR map using integer keys for compactness:

| Key | Field Name | Type | Required | Description |
|-----|------------|------|----------|-------------|
| 0 | `context_id` | bytes(32) | REQUIRED (Paykit) | Thread identifier, raw bytes |
| 1 | `created_at` | uint | RECOMMENDED | Unix timestamp (seconds) |
| 2 | `expires_at` | uint | REQUIRED (Paykit) | Expiration for requests/proposals |
| 3 | `inbox_kid` | bytes(16) | **REQUIRED** | Key identifier for recipient InboxKey |
| 4 | `msg_id` | text | REQUIRED (Paykit) | Idempotency key, ASCII, max 128 chars |
| 5 | `nonce` | bytes(24) | **REQUIRED** | XChaCha20-Poly1305 nonce (random per message) |
| 6 | `purpose` | text | Optional | Hint: `"request"`, `"proposal"`, `"ack"`, `"handoff"` |
| 7 | `recipient_peerid` | bytes(32) | **REQUIRED** | Recipient's Ed25519 public key |
| 8 | `sender_ephemeral_pub` | bytes(32) | **REQUIRED** | Sender's ephemeral X25519 public key for DH |
| 9 | `sender_peerid` | bytes(32) | **REQUIRED** | Sender's Ed25519 public key |
| 10 | `sig` | bytes(64) | REQUIRED (Paykit) | Ed25519 signature for sender authenticity |
| 11 | `cert_id` | bytes(16) | Optional | AppCert identifier; if present, `sig` uses AppKey |

### inbox_kid Derivation

```
inbox_kid = first_16_bytes(SHA256(recipient_inbox_x25519_pub))
```

The `inbox_kid` enables O(1) key selection. Unknown `inbox_kid` MUST be rejected immediately WITHOUT calling Ring derivation (DoS prevention).

### Resource Bounds (DoS Prevention)

| Limit | Value | Rationale |
|-------|-------|-----------|
| `header_len` | MUST be <= 2048 bytes | Prevents memory exhaustion |
| `msg_id` length | MUST be <= 128 characters | Bounds path lengths |
| CBOR nesting depth | MUST be <= 2 | Prevents parsing complexity |
| CBOR top-level keys | MUST be <= 16 | Bounds field count |
| Indefinite-length CBOR | PROHIBITED | Determinism requirement |

### Legacy JSON Envelope (Deprecated)

For backward compatibility, decryption MAY support legacy JSON format:

```json
{
  "v": 2,
  "epk": "<base64url ephemeral public key, 32 bytes>",
  "nonce": "<base64url nonce, 24 bytes>",
  "ct": "<base64url ciphertext + 16-byte tag>"
}
```

**New implementations MUST use binary SB2 format for writing.**

### Size Limits

| Component | Maximum Size |
|-----------|--------------|
| Plaintext | 64 KiB |
| Header | 2 KiB |
| Total blob | ~66 KiB |

---

## AAD Construction

**Associated Authenticated Data (AAD)** binds the ciphertext to its storage context, preventing blob relocation attacks.

### Binary AAD Format (Normative)

Per CRYPTO_SPEC Section 7.5, AAD is constructed as binary concatenation (no delimiters):

```
aad = aad_prefix || owner_peerid_bytes || canonical_path_bytes || header_bytes
```

Where:
- `aad_prefix`: ASCII bytes `"pubky-envelope/v2:"` (18 bytes, includes colon)
- `owner_peerid_bytes`: Raw 32-byte Ed25519 public key of storage owner
- `canonical_path_bytes`: UTF-8 bytes of canonical storage path
- `header_bytes`: Deterministic CBOR serialization of header (without signature for signing)

**No delimiters between components.** Fields are concatenated directly:
- aad_prefix: 18 bytes (fixed)
- owner_peerid_bytes: 32 bytes (fixed)
- canonical_path_bytes: variable length
- header_bytes: variable length (self-delimiting CBOR)

### Storage Owner

The peer who writes the object to their homeserver storage:

| Object Type | Storage Owner |
|-------------|---------------|
| Payment request | Sender |
| Subscription proposal | Provider |
| ACK | Receiver |
| Handoff | Ring user |

### Path Canonicalization

Per CRYPTO_SPEC Section 7.12.5:

| Rule | Description |
|------|-------------|
| Encoding | UTF-8 bytes, no BOM |
| Leading slash | REQUIRED (must start with `/`) |
| Trailing slash | PROHIBITED (except root `"/"`) |
| Duplicate slashes | PROHIBITED (no `//`) |
| Character set | ASCII alphanumeric + `/-_.` only |
| Max length | 1024 bytes |

### AAD Validation

On decryption:
1. Recipient reconstructs AAD from known context (owner, path, header)
2. Decryption with wrong AAD fails with authentication error
3. This prevents an attacker from copying a blob to a different path

### Legacy String AAD (Deprecated)

For backward compatibility only:
```
aad = <purpose>:<owner_z32>:<path>
```

**New implementations MUST use binary AAD format.**

---

## Operations

### Encrypt (Seal) — SB2 Binary

**Inputs**:
- `recipient_inbox_pk`: Recipient's InboxKey X25519 public key (32 bytes)
- `recipient_peerid`: Recipient's Ed25519 public key (32 bytes)
- `sender_peerid`: Sender's Ed25519 public key (32 bytes)
- `plaintext`: Data to encrypt (≤64 KiB)
- `owner_peerid`: Storage owner's Ed25519 public key (32 bytes)
- `path`: Canonical storage path
- `context_id`: Thread identifier (32 bytes)
- `msg_id`: Idempotency key (text)

**Algorithm**:
```
1. Generate ephemeral X25519 keypair: (epk, esk)
2. Compute shared_secret = X25519(esk, recipient_inbox_pk)
3. Derive key via HKDF:
   salt = epk || recipient_inbox_pk
   key = HKDF-SHA256(salt, shared_secret, b"pubky-envelope/v2", 32)
4. Generate random 24-byte nonce
5. Compute inbox_kid = first_16_bytes(SHA256(recipient_inbox_pk))
6. Build CBOR header (deterministic encoding, integer keys):
   { 0: context_id, 3: inbox_kid, 4: msg_id, 5: nonce,
     7: recipient_peerid, 8: epk, 9: sender_peerid, ... }
7. Construct AAD = "pubky-envelope/v2:" || owner_peerid || path || header_bytes
8. Encrypt: ct = XChaCha20-Poly1305.seal(key, nonce, plaintext, aad)
9. Sign (optional): compute sig over header + ciphertext (CRYPTO_SPEC Section 7.2.1)
10. Add sig to header (key 10), optionally cert_id (key 11) if using AppKey
11. Re-encode final header with signature
12. Serialize: magic (0x53 0x42 0x32) || version (0x02) || header_len || header || ct
13. Zeroize: esk, shared_secret, key
```

**Output**: Binary SB2 blob

### Decrypt (Open) — SB2 Binary

**Inputs**:
- `recipient_inbox_sk`: Recipient's InboxKey X25519 secret key (32 bytes)
- `blob`: Binary SB2 blob
- `owner_peerid`: Storage owner's Ed25519 public key (32 bytes)
- `path`: Canonical storage path

**Algorithm**:
```
1. Verify magic bytes (0x53 0x42 0x32) and version (0x02)
2. Read header_len (u16 big-endian), reject if > 2048
3. Parse CBOR header, extract fields by integer key
4. Extract inbox_kid (key 3)
5. Look up recipient_inbox_sk by inbox_kid in local keyring
   - If not found: reject immediately (DoS prevention)
6. Extract sender_ephemeral_pub (key 8)
7. Compute shared_secret = X25519(recipient_inbox_sk, sender_ephemeral_pub)
8. Derive key via HKDF:
   salt = sender_ephemeral_pub || recipient_inbox_pk
   key = HKDF-SHA256(salt, shared_secret, b"pubky-envelope/v2", 32)
9. Construct AAD = "pubky-envelope/v2:" || owner_peerid || path || header_no_sig
10. Decrypt: plaintext = XChaCha20-Poly1305.open(key, nonce, ct, aad)
11. If sig (key 10) present:
    - If cert_id (key 11) present: verify via AppKey from AppCert
    - Else: verify sig against sender_peerid (key 9)
12. Zeroize: shared_secret, key
```

**Output**: Decrypted plaintext or error

### Legacy Decrypt (JSON Format)

For backward compatibility, implementations MAY support legacy JSON envelope decryption:

```
1. Check if blob starts with '{' (JSON)
2. Parse JSON envelope
3. Decode base64url fields (epk, nonce, ct)
4. Proceed with key derivation and decryption
```

**This is for decryption only. New writes MUST use binary SB2 format.**

---

## Error Handling

### Error Codes

| Error | Code | Description |
|-------|------|-------------|
| `UNSUPPORTED_VERSION` | E001 | Envelope `v` field is not `1` or `2` |
| `MALFORMED_ENVELOPE` | E002 | JSON parsing failed or required fields missing |
| `INVALID_BASE64` | E003 | Base64url decoding failed for epk/nonce/ct |
| `INVALID_KEY_SIZE` | E004 | epk is not 32 bytes after decoding |
| `INVALID_NONCE_SIZE` | E005 | nonce is not 12 bytes (v1) or 24 bytes (v2) after decoding |
| `DECRYPTION_FAILED` | E006 | AEAD authentication failed |
| `PLAINTEXT_TOO_LARGE` | E007 | Plaintext exceeds 64 KiB limit |

### Error Behavior

1. **Never reveal reason for decryption failure** beyond "decryption failed"
   - Wrong key, wrong AAD, and tampered ciphertext all return `DECRYPTION_FAILED`
   - This prevents oracle attacks

2. **Parse errors are distinct** from decryption errors
   - Malformed JSON or invalid base64 can be reported specifically
   - These reveal no secret information

3. **Version errors allow graceful upgrade**
   - If `v` is not `1` or `2`, return `UNSUPPORTED_VERSION` with the version number
   - Allows clients to prompt for app update

---

## Versioning

### Current Format: SB2 Binary

This specification defines SB2 binary wire format as the current standard. Decryption MAY support legacy JSON formats for backward compatibility.

| Format | Status | Wire Format | AEAD | Nonce | HKDF Info |
|--------|--------|-------------|------|-------|-----------|
| SB2 | **Current** | Binary (magic + CBOR header) | XChaCha20-Poly1305 | 24 bytes | `pubky-envelope/v2` |
| JSON v2 | Legacy (decrypt only) | JSON | XChaCha20-Poly1305 | 24 bytes | `pubky-envelope/v2` |
| JSON v1 | Legacy (decrypt only) | JSON | ChaCha20-Poly1305 | 12 bytes | `paykit-sealed-blob-v1` |

### Format Detection

```
if blob[0:3] == 0x53 0x42 0x32 ("SB2"):
    # Binary SB2 format
    parse_sb2_binary(blob)
elif blob[0] == 0x7B ('{'):
    # Legacy JSON format
    parse_json_envelope(blob)
else:
    return ERROR_UNKNOWN_FORMAT
```

### Version History

- **SB2 Binary** (January 2026): Binary wire format with CBOR header, full header fields, inbox_kid for key selection
- **JSON v2** (January 2026): XChaCha20-Poly1305 with 24-byte nonce (deprecated for new writes)
- **JSON v1** (December 2025): ChaCha20-Poly1305 with 12-byte nonce (deprecated)

### Breaking vs Non-Breaking Changes

**Breaking** (requires new format):
- Changing AEAD algorithm
- Changing key derivation
- Changing AAD format
- Changing header encoding

**Non-Breaking** (same format):
- Adding optional header fields (new integer keys)
- Adding new `purpose` values

---

## Security Considerations

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Passive eavesdropper | X25519 ECDH + XChaCha20-Poly1305 encryption (v2) |
| Active tampering | Poly1305 authentication tag |
| Blob relocation | AAD binding to path and owner |
| Key reuse attacks | Ephemeral sender key per blob |
| Timing attacks | Constant-time comparison in Poly1305 verification |

### Key Management

1. **Ephemeral keys**: Sender generates fresh X25519 keypair per blob; zeroize after use
2. **Recipient InboxKey**: Static X25519 key discovered via KeyBinding `inbox_keys[]`
3. **Key rotation**: Use `inbox_kid` field (key 3) for O(1) key selection among cached keys
4. **Key separation**: InboxKey for Sealed Blob ONLY; TransportKey for Noise ONLY

### Handoff-Specific Security

For handoff blobs (`purpose: "handoff"`):

1. **Ephemeral recipient key**: Bitkit generates one-time X25519 keypair for receiving handoff
2. **Time-limited**: Handoff payloads include `expires_at` in header; Bitkit rejects expired
3. **Single-use**: Bitkit deletes remote blob immediately after successful decryption
4. **No legacy fallback**: Ring rejects handoff requests without `ephemeralPk` parameter

### Request/Proposal Security

For payment requests and subscription proposals:

1. **Recipient InboxKey**: Encrypt to recipient's InboxKey (NOT TransportKey)
2. **Discovery**: Sender fetches recipient's KeyBinding from PKARR, extracts `inbox_keys[]`
3. **inbox_kid derivation**: `first_16_bytes(SHA256(inbox_x25519_pub))`
4. **Key rotation**: Try key matching `inbox_kid` first; retain old keys for 7+ days
4. **Legacy migration**: Readers accept plaintext during transition (write-encrypted only)

### Memory Safety

1. **Zeroize secrets**: ephemeral secret key, shared secret, derived key must be zeroized after use
2. **No logging**: Never log plaintext, keys, or decrypted content
3. **Secure storage**: Recipient secret keys stored in platform keychain/keystore

---

## Implementation Reference

### Rust (pubky-noise + paykit-lib)

SB2 encryption/decryption is handled via the `Sb2` struct in `pubky-noise`:

```rust
use pubky_noise::sealed_blob_v2::Sb2;
use pubky_noise::{sb2_build_aad, sb2_compute_sig_input, ed25519_sign, ed25519_verify};

// Encrypt plaintext to SB2 (with optional cert_id for delegated signing)
let sb2 = Sb2::encrypt_with_cert_id(
    recipient_inbox_pk, plaintext, context_id, msg_id, purpose,
    owner_peerid, sender_peerid, recipient_peerid, canonical_path,
    created_at, expires_at, cert_id,
)?;

// Sign the envelope (after encryption)
let header_no_sig = sb2.header.encode_no_sig();
let aad = sb2_build_aad(owner_peerid, canonical_path, &header_no_sig);
let sig_input = sb2_compute_sig_input(&aad, &header_no_sig, &sb2.ciphertext);
let sig = ed25519_sign(sender_sk, &sig_input)?;
sb2.header.sig = Some(sig);

// Encode to bytes
let blob = sb2.encode();

// Decrypt and verify (paykit-lib provides higher-level API)
use paykit_lib::protocol::{sb2_decrypt_verified, SignatureRequirement};
let (plaintext, metadata) = sb2_decrypt_verified(
    &blob, recipient_inbox_sk, owner_peerid, canonical_path,
    SignatureRequirement::Required, // MUST verify for Paykit messages
    Some(&cert_fetcher), // For delegated signatures (cert_id in header)
)?;
assert!(metadata.signature_verified);
```

Key helpers from `pubky-noise`:

```rust
// inbox_kid derivation
use pubky_noise::Sb2Header;
let kid = Sb2Header::compute_inbox_kid(&inbox_pk); // [u8; 16]

// Key generation
use pubky_noise::{x25519_generate_keypair, x25519_public_from_secret};
let (secret, public) = x25519_generate_keypair();
let public = x25519_public_from_secret(&secret);

// Ed25519 signing/verification (for SB2 signatures)
use pubky_noise::{ed25519_sign, ed25519_verify};
let sig = ed25519_sign(&secret_key, &message)?;
let valid = ed25519_verify(&public_key, &message, &sig);

// Legacy JSON (decryption only - for migration)
use pubky_noise::sealed_blob_decrypt;
let plaintext = sealed_blob_decrypt(&recipient_sk, &json_envelope, aad)?;
```

### Swift (via UniFFI)

```swift
// SB2 with signing (REQUIRED for Paykit messages per PUBKY_CRYPTO_SPEC v2.5)
func sb2EncryptSigned(recipientInboxPk: Data, senderSk: Data, senderPk: Data,
                      plaintext: Data, ownerPeerid: Data, path: String, 
                      contextId: Data, msgId: String, certId: Data?) throws -> Data

// Decrypt with signature verification (REQUIRED for Paykit messages)
func sb2DecryptVerified(recipientInboxSk: Data, blob: Data, 
                        ownerPeerid: Data, path: String,
                        requireSignature: Bool) throws -> (plaintext: Data, metadata: Sb2Metadata)

// Helper functions
func computeInboxKid(inboxPk: Data) -> Data  // [u8; 16]
func x25519GenerateKeypair() -> (secret: Data, publicKey: Data)
```

### Kotlin (via UniFFI)

```kotlin
// SB2 with signing (REQUIRED for Paykit messages per PUBKY_CRYPTO_SPEC v2.5)
fun sb2EncryptSigned(recipientInboxPk: ByteArray, senderSk: ByteArray, senderPk: ByteArray,
                     plaintext: ByteArray, ownerPeerid: ByteArray, path: String,
                     contextId: ByteArray, msgId: String, certId: ByteArray?): ByteArray

// Decrypt with signature verification (REQUIRED for Paykit messages)
fun sb2DecryptVerified(recipientInboxSk: ByteArray, blob: ByteArray,
                       ownerPeerid: ByteArray, path: String,
                       requireSignature: Boolean): Pair<ByteArray, Sb2Metadata>

// Helper functions
fun computeInboxKid(inboxPk: ByteArray): ByteArray  // 16 bytes
fun x25519GenerateKeypair(): Pair<ByteArray, ByteArray> // (secret, public)
```

---

## Test Vectors

### Vector 1: v2 Basic Encryption

**Inputs**:
```
recipient_sk (hex): 0x77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a
recipient_pk (hex): 0x8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a
plaintext (utf8): "hello world"
aad: "handoff:testpubkey123:/pub/paykit.app/v0/handoff/abc"
ephemeral_sk (hex): 0x5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb
nonce (hex): 0x000000000000000000000000000000000000000000000001 (24 bytes)
```

**Expected Envelope** (field values, not full JSON due to ephemeral key):
```
v: 2
epk: base64url of ephemeral public key
nonce: base64url of 24-byte nonce
ct: <authenticated ciphertext>
```

See `pubky-noise/tests/fixtures/test_vectors.json` for cross-language test vectors with frozen ciphertext.

---

**Document Version**: 3.0  
**Last Updated**: January 21, 2026  
**Status**: Aligned with PUBKY_CRYPTO_SPEC v2.5 SB2 Binary Format

