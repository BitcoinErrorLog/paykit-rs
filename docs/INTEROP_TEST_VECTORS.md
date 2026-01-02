# Paykit v0 Interop Test Vectors

This document provides test vectors that **must** match across all Paykit client implementations (Rust, Kotlin, Swift, TypeScript).

## Scope Derivation

The `scope` is used to create per-recipient directories in storage paths.

### Algorithm

```
scope = hex(sha256(utf8(normalize(pubkey_z32))))
```

Where `normalize(pubkey_z32)` performs:
1. Trim whitespace
2. Strip `pk:` prefix if present
3. Lowercase

### Test Vectors

| Input | Expected Output |
|-------|-----------------|
| `ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u` | `55340b54f918470e1f025a80bb3347934fad3f57189eef303d620e65468cde80` |
| `8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo` | `04dc3323da61313c6f5404cf7921af2432ef867afe6cc4c32553858b8ac07f12` |
| `pk:8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo` | `04dc3323da61313c6f5404cf7921af2432ef867afe6cc4c32553858b8ac07f12` |
| `YBNDRFG8EJKMCPQXOT1UWISZA345H769YBNDRFG8EJKMCPQXOT1U` | `55340b54f918470e1f025a80bb3347934fad3f57189eef303d620e65468cde80` |

### Rust Reference

```rust
use paykit_lib::protocol::recipient_scope;

let scope = recipient_scope("ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u").unwrap();
assert_eq!(scope, "55340b54f918470e1f025a80bb3347934fad3f57189eef303d620e65468cde80");
```

### Kotlin Reference

```kotlin
import java.security.MessageDigest

fun recipientScope(pubkeyZ32: String): String {
    val normalized = pubkeyZ32.trim()
        .removePrefix("pk:")
        .lowercase()
    val hash = MessageDigest.getInstance("SHA-256").digest(normalized.toByteArray(Charsets.UTF_8))
    return hash.joinToString("") { "%02x".format(it) }
}
```

### Swift Reference

```swift
import CryptoKit

func recipientScope(_ pubkeyZ32: String) -> String {
    var normalized = pubkeyZ32.trimmingCharacters(in: .whitespaces)
    if normalized.hasPrefix("pk:") {
        normalized = String(normalized.dropFirst(3))
    }
    normalized = normalized.lowercased()
    let hash = SHA256.hash(data: Data(normalized.utf8))
    return hash.map { String(format: "%02x", $0) }.joined()
}
```

---

## Path Formats

### Payment Request

```
/pub/paykit.app/v0/requests/{recipient_scope}/{request_id}
```

Example with `recipient_scope` = `55340b54f918470e1f025a80bb3347934fad3f57189eef303d620e65468cde80` and `request_id` = `abc123`:

```
/pub/paykit.app/v0/requests/55340b54f918470e1f025a80bb3347934fad3f57189eef303d620e65468cde80/abc123
```

### Subscription Proposal

```
/pub/paykit.app/v0/subscriptions/proposals/{subscriber_scope}/{proposal_id}
```

---

## AAD (Additional Authenticated Data) Formats

AAD binds the ciphertext to its storage context. All Sealed Blob v1 encryption must use these exact formats.

### Payment Request

```
paykit:v0:request:{path}:{request_id}
```

Example:

```
paykit:v0:request:/pub/paykit.app/v0/requests/55340b54f918470e1f025a80bb3347934fad3f57189eef303d620e65468cde80/abc123:abc123
```

### Subscription Proposal

```
paykit:v0:subscription_proposal:{path}:{proposal_id}
```

### Secure Handoff

```
paykit:v0:handoff:{owner_pubkey_z32}:{path}:{request_id}
```

---

## Validation Rules

### Pubkey Normalization

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

When implementing scope derivation in a new language:

1. [ ] Implement `normalize_pubkey_z32` with trim, strip prefix, lowercase
2. [ ] Implement `recipient_scope` with SHA-256 and hex encoding
3. [ ] Verify all test vectors pass
4. [ ] Implement path builders using the scope
5. [ ] Implement AAD builders using the canonical format

