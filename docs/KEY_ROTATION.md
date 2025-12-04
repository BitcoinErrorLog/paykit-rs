# Key Rotation & Revocation Guide

This guide explains how to rotate X25519 Noise keys and revoke compromised keys in the Pubky ecosystem.

## Overview

In the cold key architecture:
- **Ed25519 keys** are kept cold (offline, rarely accessed)
- **X25519 keys** are derived and published to pkarr for Noise sessions
- **Key rotation** changes the X25519 key without changing the Ed25519 identity

## When to Rotate Keys

### Scheduled Rotation
- **Recommended interval**: 90 days for production
- **Conservative interval**: 30 days for high-security environments
- **Maximum interval**: 365 days (enforced via pkarr timestamp)

### Incident-Based Rotation
Rotate immediately if:
- ✅ X25519 key is suspected compromised
- ✅ Device is lost or stolen
- ✅ Session logs show suspicious activity
- ✅ Routine security audit recommends it

### When NOT to Rotate
- ❌ Ed25519 key is compromised (requires new identity, not rotation)
- ❌ Just for routine maintenance (only rotate on schedule or incident)

## Rotation Procedure

### 1. Derive New X25519 Key

**Change the derivation context** to generate a new key:

```rust
use pubky_noise::kdf;
use zeroize::Zeroizing;

// Old key (existing)
let old_context = b"device-2024-q1";
let old_x25519_sk = Zeroizing::new(kdf::derive_x25519_static(&ed25519_sk, old_context));

// New key (rotated)
let new_context = b"device-2024-q2";  // Changed context
let new_x25519_sk = Zeroizing::new(kdf::derive_x25519_static(&ed25519_sk, new_context));
let new_x25519_pk = kdf::x25519_pk_from_sk(&new_x25519_sk);
```

**Alternative: Increment device ID**
```rust
// Old: "device-v1"
// New: "device-v2"
let new_device_id = format!("{}-v{}", base_device_id, version + 1);
```

### 2. Sign New Key Binding (Cold Operation)

```rust
use pubky_noise::pkarr_helpers;

// This requires Ed25519 access (cold signing)
let signature = pkarr_helpers::sign_pkarr_key_binding(
    &ed25519_sk,
    &new_x25519_pk,
    new_context.as_bytes(),
);

// Get current timestamp
let timestamp = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs();

// Format for publication
let txt_value = pkarr_helpers::format_x25519_for_pkarr_with_timestamp(
    &new_x25519_pk,
    Some(&signature),
    timestamp,
);
```

### 3. Publish to pkarr

```rust
use paykit_demo_core::pkarr_discovery;

// Publish new key to pubky storage
pkarr_discovery::publish_noise_key(
    &session,
    &ed25519_sk,
    &new_x25519_pk,
    &new_device_id,
).await?;

// Ed25519 key can now be stored cold again
```

### 4. Update Local Configuration

```rust
// Update stored X25519 secret key
secure_storage.set("x25519_sk", &new_x25519_sk).await?;
secure_storage.set("device_id", &new_device_id).await?;

// Clear the old key from memory
drop(old_x25519_sk);  // Zeroizing will clear memory
```

### 5. Gradual Rollout

**Option A: Immediate cutover**
```rust
// Publish new key only
// Old sessions will fail, forcing reconnection with new key
```

**Option B: Dual publication (recommended)**
```rust
// Publish both old and new keys during transition period
publish_noise_key(&session, &ed25519_sk, &old_x25519_pk, "device-v1").await?;
publish_noise_key(&session, &ed25519_sk, &new_x25519_pk, "device-v2").await?;

// After 7 days, remove old key
// This allows existing sessions to complete gracefully
```

## Backward Compatibility

### During Rotation

**Old sessions continue to work:**
- Existing Noise sessions use the old key
- New connections use the new key
- No disruption to active connections

**Peers discover the new key:**
- pkarr lookup returns the new key
- Cached keys are invalidated (timestamp mismatch)
- Automatic upgrade on next connection

### Migration Period

**Recommended: 7-day overlap**
1. Day 0: Publish new key alongside old key
2. Days 1-7: Both keys valid, new connections use new key
3. Day 7: Remove old key from pkarr

## Revocation

### Emergency Revocation (Compromised Key)

**Add revoked flag to pkarr record:**
```rust
// Extended format with revocation flag
"v=1;k={key};sig={sig};ts={timestamp};revoked=true"
```

**Check before using:**
```rust
fn is_key_revoked(txt_record: &str) -> bool {
    txt_record.contains(";revoked=true")
}

// In discover_noise_key
if is_key_revoked(&txt_record) {
    return Err("Key has been revoked");
}
```

### Revocation Procedure

1. **Publish revoked=true** to pkarr
```rust
// Old key with revocation flag
let txt_value = format!(
    "v=1;k={};sig={};ts={};revoked=true",
    BASE64.encode(old_x25519_pk),
    BASE64.encode(signature),
    timestamp
);

session.storage().put(old_device_path, txt_value).await?;
```

2. **Publish new key** at different device_id
```rust
publish_noise_key(&session, &ed25519_sk, &new_x25519_pk, "device-v2").await?;
```

3. **Notify contacts** (application-level)
```rust
// Send notification via Pubky messaging
notify_contacts("My Noise key has been rotated to device-v2");
```

## Monitoring Key Health

### Key Age Tracking

```rust
#[derive(Serialize, Deserialize)]
struct KeyMetadata {
    created_at: u64,          // When key was derived
    published_at: u64,        // When published to pkarr
    last_rotated: Option<u64>, // Last rotation timestamp
    rotation_count: u32,       // Number of rotations
}

impl KeyMetadata {
    fn age_days(&self) -> u64 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        (now - self.created_at) / (24 * 60 * 60)
    }

    fn should_rotate(&self, policy_days: u64) -> bool {
        self.age_days() >= policy_days
    }
}
```

### Automated Rotation Reminders

```typescript
// React Native example
function checkKeyRotationDue() {
    const keyAge = Date.now() / 1000 - keyCreatedAt;
    const MAX_AGE = 90 * 24 * 60 * 60;  // 90 days
    
    if (keyAge > MAX_AGE) {
        showNotification("Key rotation recommended - key is 90+ days old");
    }
}
```

## Key Rotation Timeline

```
Time    Action                                      Impact
────────────────────────────────────────────────────────────
T+0     Derive new X25519 key (offline)             None
T+1min  Sign binding with Ed25519 (cold access)     None  
T+2min  Publish new key to pkarr                    Peers discover new key
T+1hr   Most peers have discovered new key          New connections use new key
T+1day  Old sessions expire naturally               Minimal disruption
T+7days Remove old key from pkarr                   Full migration complete
```

## Best Practices

1. **Schedule rotations** during low-traffic periods
2. **Test rotation** in staging environment first
3. **Monitor** connection success rates after rotation
4. **Keep Ed25519 cold** - only access for rotation signing
5. **Document rotation** in key metadata
6. **Notify peers** if possible (application-level)
7. **Maintain overlap** period (7 days recommended)

## Security Considerations

### Key Compromise Detection

**Signs of compromise:**
- Unexpected connection attempts
- Sessions from unknown devices
- Decryption failures on valid sessions
- Audit logs show suspicious patterns

**Response:**
1. Immediately rotate to new key
2. Mark old key as revoked
3. Audit all recent sessions
4. Notify affected peers

### Rotation Frequency Tradeoffs

| Frequency | Security | Convenience | Recommended For |
|-----------|----------|-------------|-----------------|
| Weekly | Highest | Lowest | Critical infrastructure |
| Monthly | High | Medium | High-security apps |
| Quarterly (90d) | Good | High | Standard applications ✅ |
| Annually | Minimal | Highest | Low-risk scenarios |

## Related Documentation

- [pkarr_discovery module](../paykit-demo-core/src/pkarr_discovery.rs) - Key publication implementation
- [Cold Key Architecture](../../pubky-noise/docs/COLD_KEY_ARCHITECTURE.md) - Architecture overview
- [KEY_CACHING_STRATEGY.md](KEY_CACHING_STRATEGY.md) - Cache management

---

**Last Updated**: December 2025

