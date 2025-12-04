# Key Caching Strategy for Noise Patterns

This guide explains when and how to cache static keys learned from XX handshakes to optimize future connections.

## Overview

When you connect to a peer for the first time and don't know their static key:
1. Use **XX pattern** (Trust-On-First-Use) to learn their static key during handshake
2. **Cache the learned key** securely
3. Use **IK or IK-raw pattern** for all future connections (faster, no key learning needed)

This upgrade path provides both convenience (no pkarr lookup needed for first contact) and security (subsequent connections are fully authenticated).

## When to Cache

### Always Cache After XX Handshake
```rust
// First connection with unknown peer
let (channel, server_static_pk) = NoiseRawClientHelper::connect_xx(&x25519_sk, host).await?;

// 📝 IMPORTANT: Cache this key!
contact_storage.save_noise_key(&peer_pubkey, &server_static_pk, Instant::now()).await?;
```

### When to Invalidate Cache
- Key age exceeds policy (e.g., 30 days)
- pkarr record shows different key (key rotation detected)
- Connection failures with cached key (try re-discovering)
- Explicit user action (contact removal, key reset)

## What to Cache

### Minimum Required Fields
```rust
struct CachedNoiseKey {
    peer_pubkey: PublicKey,      // Ed25519 identity
    x25519_static: [u8; 32],     // Learned X25519 key
    first_seen: Instant,         // When key was learned
    last_used: Instant,          // Last successful connection
    source: KeySource,           // XX, pkarr, or manual
}

enum KeySource {
    XX,              // Learned from XX handshake
    Pkarr,           // Discovered from pkarr
    Manual,          // Manually configured
}
```

### Optional Metadata
- `connection_count`: Number of successful connections
- `device_id`: For multi-device scenarios
- `notes`: User-added context

## Storage Recommendations

### Mobile Platforms

**iOS (Swift):**
```swift
// Use Keychain for secure storage
let keychain = KeychainWrapper.standard
keychain.set(
    encodedKey,
    forKey: "noise_key_\(peerPubkey)",
    withAccessibility: .whenUnlocked
)
```

**Android (Kotlin):**
```kotlin
// Use EncryptedSharedPreferences
val sharedPrefs = EncryptedSharedPreferences.create(
    context,
    "noise_keys",
    masterKey,
    EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
    EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
)
sharedPrefs.edit().putString("noise_key_$peerPubkey", encodedKey).apply()
```

### Desktop/Server

**Rust:**
```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct NoiseKeyCache {
    keys: HashMap<String, CachedNoiseKey>,
}

impl NoiseKeyCache {
    fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn load_from_file(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }
}
```

## Upgrade Path: XX → IK-raw

### First Contact (XX Pattern)
```rust
use paykit_demo_core::NoiseRawClientHelper;
use zeroize::Zeroizing;

// Connect to unknown peer with XX
let x25519_sk = Zeroizing::new(derive_x25519_key(&seed, b"device"));
let (channel, server_static_pk) = NoiseRawClientHelper::connect_xx(&x25519_sk, host).await?;

// Cache the learned key
cache.save(peer_pubkey, server_static_pk, KeySource::XX).await?;

// Continue with payment...
```

### Subsequent Connections (IK-raw Pattern)
```rust
// Load cached key
let server_pk = cache.get(&peer_pubkey).await?;

// Use IK-raw for faster connection
let channel = NoiseRawClientHelper::connect_ik_raw(&x25519_sk, host, &server_pk).await?;

// Update last_used timestamp
cache.touch(&peer_pubkey).await?;
```

## Validation Strategy

### Before Using Cached Key

**Option 1: Trust cached key (fast)**
```rust
if let Some(cached_key) = cache.get(&peer_pubkey).await? {
    // Use cached key directly
    return connect_ik_raw(&x25519_sk, host, &cached_key).await;
}
```

**Option 2: Verify against pkarr (secure)**
```rust
if let Some(cached_key) = cache.get(&peer_pubkey).await? {
    // Verify cached key matches pkarr
    match discover_noise_key(&storage, &peer_pubkey, "default").await {
        Ok(pkarr_key) if pkarr_key == cached_key => {
            // Cache is valid
            return connect_ik_raw(&x25519_sk, host, &cached_key).await;
        }
        Ok(new_key) => {
            // Key rotated - update cache
            cache.update(&peer_pubkey, &new_key).await?;
            return connect_ik_raw(&x25519_sk, host, &new_key).await;
        }
        Err(_) => {
            // pkarr lookup failed - use cached key anyway
            return connect_ik_raw(&x25519_sk, host, &cached_key).await;
        }
    }
}
```

**Option 3: Periodic validation (balanced)**
```rust
let cached = cache.get(&peer_pubkey).await?;

if cached.needs_validation() {  // e.g., > 7 days since last pkarr check
    // Re-verify via pkarr
    if let Ok(pkarr_key) = discover_noise_key(&storage, &peer_pubkey, "default").await {
        if pkarr_key != cached.x25519_static {
            cache.update(&peer_pubkey, &pkarr_key).await?;
            return connect_ik_raw(&x25519_sk, host, &pkarr_key).await;
        }
    }
}

// Use cached key
connect_ik_raw(&x25519_sk, host, &cached.x25519_static).await
```

## Cache Expiry Policy

### Recommended TTLs

| Scenario | Cache TTL | Validation Frequency |
|----------|-----------|---------------------|
| Frequent contacts | 30 days | Weekly |
| Occasional contacts | 14 days | Every connection |
| One-time contacts | 7 days | Every connection |
| High security | No cache | Always use pkarr |

### Expiry Implementation
```rust
impl CachedNoiseKey {
    fn is_expired(&self, max_age: Duration) -> bool {
        self.first_seen.elapsed() > max_age
    }

    fn needs_validation(&self, validation_interval: Duration) -> bool {
        self.last_validated.elapsed() > validation_interval
    }
}
```

## Security Considerations

### TOFU Security Model

**First Connection (XX):**
- ⚠️ Vulnerable to MITM on first contact
- ✅ All subsequent connections are secure (key is cached)
- ✅ Mitigated by verifying key via pkarr after first contact

**Best Practice:**
```rust
// After XX handshake, verify via pkarr
let (channel, learned_key) = connect_xx(&x25519_sk, host).await?;

// Verify the learned key matches pkarr (if available)
if let Ok(pkarr_key) = discover_noise_key(&storage, &peer_pubkey, "default").await {
    if pkarr_key != learned_key {
        return Err("MITM detected: XX key doesn't match pkarr");
    }
}

// Now safe to cache
cache.save(&peer_pubkey, &learned_key).await?;
```

### Key Rotation Detection

```rust
fn detect_key_rotation(cached_key: &[u8; 32], pkarr_key: &[u8; 32]) -> KeyRotationAction {
    if cached_key == pkarr_key {
        KeyRotationAction::NoChange
    } else {
        // Key has rotated - update cache
        KeyRotationAction::UpdateCache(pkarr_key.clone())
    }
}
```

### Cache Poisoning Prevention

- ✅ Only cache keys from successful handshakes
- ✅ Verify keys via pkarr periodically
- ✅ Implement TTL to force refresh
- ✅ Allow manual cache invalidation

## Example: Complete Caching Flow

```rust
use paykit_demo_core::{NoiseRawClientHelper, pkarr_discovery::discover_noise_key};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

async fn connect_with_caching(
    x25519_sk: &Zeroizing<[u8; 32]>,
    peer_pubkey: &PublicKey,
    host: &str,
    cache: &mut KeyCache,
    storage: &PublicStorage,
) -> Result<PubkyNoiseChannel<TcpStream>> {
    // 1. Check cache first
    if let Some(cached) = cache.get(peer_pubkey).await? {
        // 2. Validate if needed
        if !cached.is_expired(Duration::from_secs(30 * 24 * 60 * 60)) {
            if cached.needs_validation(Duration::from_secs(7 * 24 * 60 * 60)) {
                // Periodic validation
                if let Ok(pkarr_key) = discover_noise_key(storage, peer_pubkey, "default").await {
                    if pkarr_key != cached.x25519_static {
                        // Key rotated - update cache
                        cache.update(peer_pubkey, &pkarr_key).await?;
                        return NoiseRawClientHelper::connect_ik_raw(x25519_sk, host, &pkarr_key).await;
                    }
                }
                cache.mark_validated(peer_pubkey).await?;
            }

            // 3. Use cached key (IK-raw pattern)
            cache.touch(peer_pubkey).await?;
            return NoiseRawClientHelper::connect_ik_raw(x25519_sk, host, &cached.x25519_static).await;
        } else {
            // Expired - remove from cache
            cache.remove(peer_pubkey).await?;
        }
    }

    // 4. No valid cache - try pkarr first
    if let Ok(pkarr_key) = discover_noise_key(storage, peer_pubkey, "default").await {
        cache.save(peer_pubkey, &pkarr_key, KeySource::Pkarr).await?;
        return NoiseRawClientHelper::connect_ik_raw(x25519_sk, host, &pkarr_key).await;
    }

    // 5. Fallback to XX (TOFU)
    let (channel, learned_key) = NoiseRawClientHelper::connect_xx(x25519_sk, host).await?;

    // 6. Verify learned key against pkarr (best effort)
    let verified = match discover_noise_key(storage, peer_pubkey, "default").await {
        Ok(pkarr_key) => pkarr_key == learned_key,
        Err(_) => false,  // pkarr unavailable - accept TOFU risk
    };

    // 7. Cache the learned key
    cache.save_with_verification(peer_pubkey, &learned_key, KeySource::XX, verified).await?;

    Ok(channel)
}
```

## Performance Impact

### Without Caching (Always pkarr lookup)
- Connection time: ~500ms (pkarr lookup) + ~50ms (IK-raw handshake) = **~550ms**

### With Caching
- First connection: ~550ms (XX handshake, no pkarr)
- Subsequent: ~50ms (IK-raw with cached key) = **10x faster**

### Cache Hit Rates
- Frequent contacts (daily): 95%+ hit rate
- Occasional contacts (weekly): 70-80% hit rate
- One-time contacts: 0% (but still benefits from TOFU)

## Related Documentation

- [PATTERN_SELECTION.md](PATTERN_SELECTION.md) - Pattern selection guide
- [pubky-noise Cold Key Architecture](../../pubky-noise/docs/COLD_KEY_ARCHITECTURE.md)
- [BITKIT_INTEGRATION.md](BITKIT_INTEGRATION.md) - Bitkit-specific caching

---

**Last Updated**: December 2025

