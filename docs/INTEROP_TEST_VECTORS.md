# Paykit v0 Interop Test Vectors

This document provides test vectors that **must** match across all Paykit client implementations (Rust, Kotlin, Swift, TypeScript).

---

## ContextId Derivation (Current)

ContextId creates symmetric peer-pair directories in storage paths.

### Algorithm

```
context_id = hex(sha256("paykit:v0:context:" + first_z32 + ":" + second_z32))
```

Where:
- `first_z32` and `second_z32` are normalized pubkeys sorted lexicographically
- Normalization: trim, strip `pubky://` or `pk:` prefix, lowercase

**Key property**: `context_id(A, B) == context_id(B, A)` (symmetric)

### Test Vectors

| Pubkey A | Pubkey B | Expected ContextId (hex) |
|----------|----------|-------------------------|
| `yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy` | `yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy` | `9d88e67d72ad84aff9c61bd356da55c802febcfd2f86c9ca239a1a9d6e8db576` |
| `ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u` | `8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo` | `762732a6bd789d03abd23de709ab0990593217566d098381d50fac87f0c58c74` |
| `pk:ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u` | `pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo` | `762732a6bd789d03abd23de709ab0990593217566d098381d50fac87f0c58c74` (same after normalization) |

**Verification command (Unix)**:
```bash
echo -n 'paykit:v0:context:8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo:ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u' | shasum -a 256
# Output: 762732a6bd789d03abd23de709ab0990593217566d098381d50fac87f0c58c74
```

### Rust Reference

```rust
use paykit_lib::protocol::context_id;

let ctx = context_id(
    "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
    "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
).unwrap();
assert_eq!(ctx.len(), 64); // 64 hex chars

// Symmetric property
let ctx_reversed = context_id(
    "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
    "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u"
).unwrap();
assert_eq!(ctx, ctx_reversed);
```

### Kotlin Reference

```kotlin
import java.security.MessageDigest

fun contextId(pubkeyA: String, pubkeyB: String): String {
    val normA = normalizePubkeyZ32(pubkeyA)
    val normB = normalizePubkeyZ32(pubkeyB)
    val (first, second) = if (normA <= normB) Pair(normA, normB) else Pair(normB, normA)
    val preimage = "paykit:v0:context:$first:$second"
    val hash = MessageDigest.getInstance("SHA-256").digest(preimage.toByteArray(Charsets.UTF_8))
    return hash.joinToString("") { "%02x".format(it) }
}
```

### Swift Reference

```swift
import CryptoKit

func contextId(_ pubkeyA: String, _ pubkeyB: String) throws -> String {
    let normA = try normalizePubkeyZ32(pubkeyA)
    let normB = try normalizePubkeyZ32(pubkeyB)
    let (first, second) = normA <= normB ? (normA, normB) : (normB, normA)
    let preimage = "paykit:v0:context:\(first):\(second)"
    let hash = SHA256.hash(data: Data(preimage.utf8))
    return hash.map { String(format: "%02x", $0) }.joined()
}
```

---

## Legacy Scope Derivation (Deprecated)

The legacy `recipient_scope` is retained for backward compatibility but **should not be used for new code**.

### Algorithm

```
scope = hex(sha256(utf8(normalize(pubkey_z32))))
```

### Test Vectors

| Input | Expected Output |
|-------|-----------------|
| `ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u` | `55340b54f918470e1f025a80bb3347934fad3f57189eef303d620e65468cde80` |
| `8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo` | `04dc3323da61313c6f5404cf7921af2432ef867afe6cc4c32553858b8ac07f12` |
| `pk:8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo` | `04dc3323da61313c6f5404cf7921af2432ef867afe6cc4c32553858b8ac07f12` |
| `YBNDRFG8EJKMCPQXOT1UWISZA345H769YBNDRFG8EJKMCPQXOT1U` | `55340b54f918470e1f025a80bb3347934fad3f57189eef303d620e65468cde80` |

---

## Path Formats (ContextId-based)

### Payment Request

```
/pub/paykit.app/v0/requests/{context_id}/{request_id}
```

### Subscription Proposal

```
/pub/paykit.app/v0/subscriptions/proposals/{context_id}/{proposal_id}
```

### Encrypted ACK

```
/pub/paykit.app/v0/acks/{object_type}/{context_id}/{msg_id}
```

---

## AAD (Additional Authenticated Data) Formats

AAD binds the ciphertext to its storage context and owner. All Sealed Blob v2 encryption must use these exact formats.

### Format Pattern (Owner-Bound)

```
paykit:v0:{purpose}:{owner_z32}:{path}:{id}
```

### Payment Request AAD

```
paykit:v0:request:{owner_z32}:{path}:{request_id}
```

### Subscription Proposal AAD

```
paykit:v0:subscription_proposal:{owner_z32}:{path}:{proposal_id}
```

### Encrypted ACK AAD

```
paykit:v0:ack_{object_type}:{ack_writer_z32}:{path}:{msg_id}
```

### Secure Handoff AAD

```
paykit:v0:handoff:{owner_z32}:{path}:{request_id}
```

---

## AAD Test Vectors

These test vectors verify that AAD construction matches across implementations.

### Test Case 1: Payment Request AAD

**Inputs:**
- `owner_pubkey_z32` (sender): `8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo`
- `sender_pubkey_z32`: `8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo`
- `recipient_pubkey_z32`: `ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u`
- `request_id`: `req-12345`

**Expected AAD:**
```
paykit:v0:request:8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo:/pub/paykit.app/v0/requests/{context_id}/req-12345:req-12345
```

(Where `{context_id}` is computed from sender + recipient)

### Test Case 2: Secure Handoff AAD

**Inputs:**
- `owner_pubkey_z32`: `ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u`
- `request_id`: `handoff-abc123`

**Expected AAD:**
```
paykit:v0:handoff:ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u:/pub/paykit.app/v0/handoff/handoff-abc123:handoff-abc123
```

---

## Validation Rules

### Pubkey Normalization

- **Prefix stripping**: Remove `pubky://` or `pk:` prefix if present
- **Length**: Normalized pubkey must be exactly 52 characters
- **Alphabet**: Only z-base-32 characters allowed: `ybndrfg8ejkmcpqxot1uwisza345h769`
- **Case**: Must be lowercase after normalization

### Invalid Inputs

| Input | Reason |
|-------|--------|
| `tooshort` | Length ≠ 52 |
| `lbndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u` | Contains 'l' (not in z32) |
| `vbndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u` | Contains 'v' (not in z32) |

---

## Sealed Blob v2 (SB2) Test Vectors

Per PUBKY_CRYPTO_SPEC v2.5 Section 7.5.

### Magic Bytes and Version

```
SB2_MAGIC = 0x53 0x42 0x32 ("SB2")
SB2_VERSION = 0x01
```

### inbox_kid Derivation

```
inbox_kid = first_16_bytes(SHA256(inbox_public_key))
```

| Inbox Public Key (hex) | Expected inbox_kid (hex) |
|----------------------|-------------------------|
| `0000000000000000000000000000000000000000000000000000000000000000` | `66687aadf862bd776c8fc18b8e9f8e20` |
| `0101010101010101010101010101010101010101010101010101010101010101` | `72cd6e8422c407fb6d098690f1130b7d` |
| `ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff` | `af9613760f72635fbdb44a5a0a63c39f` |

**Verification (using sha256sum)**:
```bash
# For all-zeros key
echo -n '0000000000000000000000000000000000000000000000000000000000000000' | xxd -r -p | shasum -a 256
# Output: 66687aadf862bd776c8fc18b8e9f8e20... (first 16 bytes = inbox_kid)
```

### AAD Construction (Binary)

```
aad = aad_prefix || owner_peerid_bytes || canonical_path_bytes || header_bytes
```

Where:
- `aad_prefix = "pubky-envelope/v2:"` (18 bytes)
- `owner_peerid_bytes` = 32-byte Ed25519 public key
- `canonical_path_bytes` = UTF-8 path string
- `header_bytes` = Deterministic CBOR-encoded header (without sig field)

### CBOR Header Integer Keys

Per CRYPTO_SPEC, header fields use integer keys:

| Key | Field | Type |
|-----|-------|------|
| 0 | context_id | bstr(32) |
| 1 | msg_id | tstr |
| 2 | inbox_kid | bstr(16) |
| 3 | sender_peerid | bstr(32) |
| 4 | epk | bstr(32) |
| 5 | nonce | bstr(24) |
| 6 | created_at | uint |
| 7 | expires_at | uint |
| 8 | sig | bstr(64) (optional) |

### Rust Reference

```rust
use pubky_noise::{Sb2, Sb2Header, sb2_build_aad, SB2_MAGIC, SB2_VERSION};

// Check SB2 magic
assert_eq!(&data[0..3], SB2_MAGIC);
assert_eq!(data[3], SB2_VERSION);

// Build AAD
let aad = sb2_build_aad(&owner_peerid, "/pub/paykit.app/v0/requests/test", &header_bytes);
```

---

## Random ContextId (CRYPTO_SPEC v2.5)

New threads use 32 random bytes instead of pair-derived ContextId.

### Generation

```rust
use paykit_lib::protocol::{generate_context_id, generate_context_id_hex};

let ctx_bytes: [u8; 32] = generate_context_id();
let ctx_hex: String = generate_context_id_hex(); // 64 hex chars
```

### Properties

- 32 bytes of cryptographically random data
- Chosen by thread initiator
- NOT derived from pubkeys
- Legacy `pair_context_id()` is deprecated but available for migration

---

## Implementation Checklist

When implementing Paykit protocol in a new language:

1. [ ] Implement `normalize_pubkey_z32` with trim, strip `pubky://` and `pk:` prefixes, lowercase
2. [ ] Implement `context_id` with symmetric peer-pair derivation (legacy)
3. [ ] Implement `generate_context_id` for random 32-byte ContextId (new)
4. [ ] Verify all ContextId test vectors pass (including symmetry for legacy)
5. [ ] Implement path builders using ContextId
6. [ ] Implement binary AAD builders per CRYPTO_SPEC v2.5
7. [ ] Implement SB2 binary format with CBOR header
8. [ ] Implement inbox_kid derivation
9. [ ] (Optional) Implement legacy `recipient_scope` for backward compatibility

