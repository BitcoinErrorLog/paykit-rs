# Paykit Cryptographic Primitives

SPDX-License-Identifier: CC0-1.0 OR MIT

This document specifies the cryptographic primitives used by the Paykit protocol.

## Key Types

| Purpose | Algorithm | Key Size | Notes |
|---------|-----------|----------|-------|
| Identity | Ed25519 | 32 bytes (public), 32 bytes (secret) | RFC 8032 |
| Key Exchange | X25519 | 32 bytes | RFC 7748 |
| Noise AEAD | ChaCha20-Poly1305 | 32-byte key, 12-byte nonce | RFC 8439 |
| Sealed Blob AEAD (v2) | XChaCha20-Poly1305 | 32-byte key, 24-byte nonce | Extended-nonce variant |
| Sealed Blob AEAD (v1) | ChaCha20-Poly1305 | 32-byte key, 12-byte nonce | Legacy, decrypt-only |
| Hash (Noise) | BLAKE2s | 32 bytes | RFC 7693 |
| Hash (Signatures) | SHA-256 | 32 bytes | FIPS 180-4 |
| Key Derivation | HKDF-SHA256 | - | RFC 5869 |

## Identity Encoding

Public keys in Pubky URIs use **z-base-32** encoding:

* **Alphabet**: `ybndrfg8ejkmcpqxot1uwisza345h769`
* **Output length**: 52 characters for 32-byte keys
* **Case-insensitive**: Implementations MUST normalize to lowercase
* **No padding**: Trailing `=` characters are omitted

Example (all-zeros key → all-'y' encoding):
```
Public key (hex): 0000000000000000000000000000000000000000000000000000000000000000
Public key (z-base-32): yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy
Pubky URI: pubky://yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy
```

Note: The all-zeros vector is trivially verifiable. For arbitrary keys, use the `pubky` crate's z-base-32 implementation as the reference.

## Noise Protocol Cipher Suites

### Noise_XX_25519_ChaChaPoly_BLAKE2s

Used for first-contact scenarios (Trust-On-First-Use):

```
-> e
<- e, ee, s, es
-> s, se
```

* **DH**: X25519
* **Cipher**: ChaCha20-Poly1305 (RFC 8439)
* **Hash**: BLAKE2s (RFC 7693)

### Noise_IK_25519_ChaChaPoly_BLAKE2s

Used when initiator knows responder's static public key:

```
-> e, es, s, ss
<- e, ee, se
```

* **DH**: X25519
* **Cipher**: ChaCha20-Poly1305 (RFC 8439)
* **Hash**: BLAKE2s (RFC 7693)

## ContextId Derivation

ContextId provides a stable, symmetric identifier for a peer pair, used in storage paths for routing and correlation.

**Formula**:
```
context_id = hex(SHA256("paykit:v0:context:" || first_z32 || ":" || second_z32))
```

Where:
- `first_z32`, `second_z32` = normalized pubkeys sorted lexicographically
- Normalization: strip `pubky://` and `pk:` prefixes, lowercase, trim
- Result: 64-character lowercase hex string
- Symmetric: `context_id(A, B) == context_id(B, A)`

## Sealed Blob Envelope (Async Messaging)

For asynchronous encrypted messaging (e.g., payment requests stored on homeservers), Paykit uses the Sealed Blob format. All blobs use **ephemeral-static ECDH** (sender generates ephemeral keypair, encrypts to recipient's static public key).

### Version 2 (Current)

#### Envelope Structure

```json
{
  "v": 2,
  "epk": "<base64url-encoded ephemeral public key, 32 bytes>",
  "nonce": "<base64url-encoded nonce, 24 bytes>",
  "ct": "<base64url-encoded ciphertext + 16-byte tag>"
}
```

Optional fields:
* `kid`: Key identifier (first 8 bytes of SHA-256(recipient_pk), hex-encoded)
* `purpose`: Human-readable hint (`"handoff"`, `"request"`, `"proposal"`)

#### Key Derivation (v2)

```
shared_secret = X25519(ephemeral_sk, recipient_pk)
salt          = ephemeral_pk || recipient_pk  (64 bytes)
info          = b"pubky-envelope/v2"
key           = HKDF-SHA256(salt, shared_secret, info, 32)
```

#### Encryption Process (v2)

1. Generate ephemeral X25519 keypair: `(ephemeral_sk, ephemeral_pk)`
2. Compute: `shared_secret = X25519(ephemeral_sk, recipient_pk)`
3. Derive key via HKDF-SHA256:
   - Salt: `ephemeral_pk || recipient_pk` (64 bytes)
   - IKM: `shared_secret`
   - Info: `"pubky-envelope/v2"`
   - Output: 32 bytes
4. Generate random 24-byte nonce
5. Encrypt with XChaCha20-Poly1305:
   - Key: derived key
   - Nonce: 24 bytes
   - AAD: `"paykit:v0:{purpose}:{owner_z32}:{path}:{id}"` (binds ciphertext to storage owner and location)
   - Plaintext: message bytes
6. Encode envelope as JSON with base64url (no padding)

**AAD Format (Normative)**:
```
aad = "paykit:v0:" || purpose || ":" || owner_z32 || ":" || path || ":" || id
```

Where:
- `purpose`: Object type (`request`, `subscription_proposal`, `ack_request`, `handoff`)
- `owner_z32`: Normalized z-base-32 pubkey of the storage owner
- `path`: Full storage path
- `id`: Object identifier

### Version 1 (Legacy, Decrypt-Only)

Implementations MUST support v1 decryption for backward compatibility.

#### Envelope Structure (v1)

```json
{
  "v": 1,
  "epk": "<base64url-encoded ephemeral public key, 32 bytes>",
  "nonce": "<base64url-encoded nonce, 12 bytes>",
  "ct": "<base64url-encoded ciphertext + 16-byte tag>"
}
```

#### Key Derivation (v1)

```
shared_secret = X25519(ephemeral_sk, recipient_pk)
salt          = ephemeral_pk || recipient_pk  (64 bytes)
info          = b"paykit-sealed-blob-v1"
key           = HKDF-SHA256(salt, shared_secret, info, 32)
```

#### Differences from v2

| Aspect | v1 | v2 |
|--------|----|----|
| AEAD | ChaCha20-Poly1305 | XChaCha20-Poly1305 |
| Nonce size | 12 bytes | 24 bytes |
| HKDF info | `paykit-sealed-blob-v1` | `pubky-envelope/v2` |

### Decryption Process (Both Versions)

1. Parse envelope JSON, extract `v`, `epk`, `nonce`, `ct`
2. Verify version: `v == 1` or `v == 2`
3. Decode base64url fields (no padding)
4. Validate sizes:
   - `epk`: 32 bytes
   - `nonce`: 12 bytes (v1) or 24 bytes (v2)
5. Compute: `shared_secret = X25519(recipient_sk, epk)`
6. Derive key via HKDF-SHA256:
   - Salt: `epk || recipient_pk` (64 bytes)
   - IKM: `shared_secret`
   - Info: `paykit-sealed-blob-v1` (v1) or `pubky-envelope/v2` (v2)
7. Decrypt with appropriate AEAD:
   - v1: ChaCha20-Poly1305
   - v2: XChaCha20-Poly1305
8. Return plaintext

### Encoding

* **Base64url**: RFC 4648 §5, without padding (`=`)
* **JSON**: UTF-8, compact (no extra whitespace in production)

## Subscription Signatures

### Domain Separation

Subscription signatures use domain: `PAYKIT_SUBSCRIPTION_V2`

### Signing Process

1. Serialize subscription data using deterministic binary encoding (postcard)
2. Construct message: `domain || serialized_data || nonce || timestamp || expiration`
3. Hash with SHA-256 to produce 32-byte digest
4. Sign digest with Ed25519 secret key

### Signature Structure

```json
{
  "signature": "<base64url-encoded 64-byte Ed25519 signature>",
  "public_key": "<32-byte public key, hex or base64url>",
  "nonce": "<32-byte random nonce>",
  "timestamp": 1704067200,
  "expires_at": 1706745600
}
```

### Verification

1. Check `expires_at > current_time` (fail fast on expired signatures)
2. Check `nonce` not in replay database
3. Reconstruct message using same deterministic serialization
4. Verify Ed25519 signature
5. Store `nonce` in replay database

## Key Derivation

### Ed25519 to X25519 Conversion

For Noise handshakes, Ed25519 identity keys are converted to X25519:

```
x25519_secret = clamp(SHA-512(ed25519_secret)[0:32])
x25519_public = X25519(x25519_secret, basepoint)
```

Or equivalently, use `crypto_sign_ed25519_sk_to_curve25519` from libsodium.

## Security Requirements

### Nonce Handling

* Sealed Blob nonces: MUST be random (24 bytes v2, 12 bytes v1), NEVER reused with same key
* Ephemeral key per blob ensures nonce uniqueness even with random generation
* Subscription nonces: MUST be random 32 bytes, MUST be tracked for replay protection

### Key Storage

* Secret keys MUST be stored in secure hardware when available
* Secret keys MUST be zeroized after use
* Key derivation secrets MUST NOT be logged

### Timing Attacks

* Signature verification MUST use constant-time comparison
* AEAD decryption failures MUST NOT leak timing information

## Encrypted ACK Protocol

ACKs confirm receipt of async messages, enabling reliable delivery without active connections.

**ACK Path**: `/pub/paykit.app/v0/acks/{object_type}/{context_id}/{msg_id}`

**ACK AAD**: `paykit:v0:ack_{object_type}:{ack_writer_z32}:{path}:{msg_id}`

**Encryption**: Sealed Blob v2 to original sender's X25519 key

**ACK Payload**:
```json
{
  "acked": true,
  "acked_at": 1704672000,
  "msg_id": "req_001",
  "object_type": "request"
}
```

**Lifecycle**:
1. Receiver decrypts and accepts message (payment request or subscription proposal)
2. Receiver discovers sender's Noise X25519 pubkey via their noise endpoint
3. Receiver encrypts ACK payload to sender's X25519 key
4. Receiver writes encrypted ACK to their own storage
5. Sender polls receiver's ACK directory
6. Sender decrypts ACK with their own Noise secret key
7. Sender stops resending after ACK or expiration

## References

* [RFC 8032: Ed25519](https://datatracker.ietf.org/doc/html/rfc8032)
* [RFC 7748: X25519](https://datatracker.ietf.org/doc/html/rfc7748)
* [RFC 8439: ChaCha20-Poly1305](https://datatracker.ietf.org/doc/html/rfc8439)
* [RFC 7693: BLAKE2](https://datatracker.ietf.org/doc/html/rfc7693)
* [RFC 5869: HKDF](https://datatracker.ietf.org/doc/html/rfc5869)
* [Noise Protocol Framework](https://noiseprotocol.org/noise.html)
* [z-base-32](https://philzimmermann.com/docs/human-oriented-base-32-encoding.txt)
* [XChaCha20-Poly1305](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-xchacha)
* [Paykit Sealed Blob Specification](../SEALED_BLOB_SPEC.md)
