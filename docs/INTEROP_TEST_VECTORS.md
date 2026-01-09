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

| Pubkey A | Pubkey B | Expected ContextId |
|----------|----------|-------------------|
| `ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u` | `8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo` | See implementation |
| `pk:ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u` | `pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo` | Same as above (normalization) |

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

## Implementation Checklist

When implementing Paykit protocol in a new language:

1. [ ] Implement `normalize_pubkey_z32` with trim, strip `pubky://` and `pk:` prefixes, lowercase
2. [ ] Implement `context_id` with symmetric peer-pair derivation
3. [ ] Verify all ContextId test vectors pass (including symmetry)
4. [ ] Implement path builders using ContextId
5. [ ] Implement owner-bound AAD builders
6. [ ] (Optional) Implement legacy `recipient_scope` for backward compatibility

