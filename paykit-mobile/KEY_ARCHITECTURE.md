# Key Architecture: Cold Pkarr, Hot Noise

This document describes the key management architecture used in Paykit mobile apps
for secure, privacy-preserving payments over the Noise protocol.

## Overview

Paykit uses a split-key architecture:

- **Ed25519 (pkarr) keys**: "Cold" identity keys managed by a separate app (Pubky Ring)
- **X25519 (Noise) keys**: "Hot" encryption keys derived on-demand and cached locally

This separation provides:

- **Security**: Identity keys are not stored in the payment app
- **Privacy**: Rotation of Noise keys without changing identity
- **Flexibility**: Multiple devices can derive unique keys from the same identity

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Key Management Architecture                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                         Pubky Ring App                             │  │
│  │                      (Remote Key Manager)                          │  │
│  ├───────────────────────────────────────────────────────────────────┤  │
│  │                                                                    │  │
│  │   ┌──────────────────┐                                            │  │
│  │   │  Ed25519 Seed    │  ← "Cold" Storage                          │  │
│  │   │  (32 bytes)      │    - Secure enclave / Keychain             │  │
│  │   └────────┬─────────┘    - Never leaves Ring app                 │  │
│  │            │                                                       │  │
│  │            │ HKDF-SHA512                                          │  │
│  │            │ + device_id                                          │  │
│  │            │ + epoch                                              │  │
│  │            ▼                                                       │  │
│  │   ┌──────────────────┐                                            │  │
│  │   │  X25519 Keypair  │                                            │  │
│  │   │  (sk + pk)       │                                            │  │
│  │   └────────┬─────────┘                                            │  │
│  │            │                                                       │  │
│  └────────────┼──────────────────────────────────────────────────────┘  │
│               │                                                          │
│               │  URL Scheme / Intent (secure IPC)                       │
│               │  X25519 keypair transferred                             │
│               ▼                                                          │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                         Paykit App                                 │  │
│  │                      (Payment Client)                              │  │
│  ├───────────────────────────────────────────────────────────────────┤  │
│  │                                                                    │  │
│  │   ┌──────────────────┐                                            │  │
│  │   │  NoiseKeyCache   │  ← "Hot" Storage                           │  │
│  │   │  X25519 Keypairs │    - In-memory + Keychain                  │  │
│  │   │  per device/epoch│    - Auto-expires                          │  │
│  │   └────────┬─────────┘                                            │  │
│  │            │                                                       │  │
│  │            │ Used for Noise handshake                             │  │
│  │            ▼                                                       │  │
│  │   ┌──────────────────┐                                            │  │
│  │   │  FfiNoiseManager │  ← Noise Protocol                          │  │
│  │   │  Sessions        │    - IK handshake                          │  │
│  │   └──────────────────┘    - Encrypt/Decrypt                       │  │
│  │                                                                    │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## Key Types

### Ed25519 Identity Keys

- **Purpose**: Long-term identity, DHT record signing, homeserver sessions
- **Size**: 32-byte seed → 64-byte keypair
- **Storage**: Secure enclave / Keychain in Pubky Ring
- **Exposure**: Never leaves Pubky Ring; only used for derivation/signing
- **Lifetime**: Permanent (or until identity rotation)

### X25519 Noise Keys

- **Purpose**: Noise protocol encryption for payments
- **Size**: 32-byte secret key, 32-byte public key
- **Storage**: In-memory cache + Keychain backup in Paykit app
- **Exposure**: Stored locally in Paykit, used for each Noise session
- **Lifetime**: Per-epoch (rotated periodically for privacy)

## Key Derivation

X25519 keys are derived from Ed25519 seeds using HKDF-SHA512:

```
input:
  - ed25519_seed: 32 bytes
  - device_id: variable length bytes
  - epoch: u32

derive_x25519_for_device_epoch(seed, device_id, epoch):
    info = b"noise-key" || device_id || epoch.to_be_bytes()
    prk = HKDF-Extract(salt=nil, ikm=seed)
    x25519_sk = HKDF-Expand(prk, info, len=32)
    x25519_pk = x25519_base_point_multiply(x25519_sk)
    return (x25519_sk, x25519_pk)
```

### Implementation (pubky-noise)

```rust
// Rust (pubky-noise/src/kdf.rs)
pub fn derive_x25519_for_device_epoch(
    seed: &[u8; 32],
    device_id: &[u8],
    epoch: u32,
) -> [u8; 32] {
    let hk = Hkdf::<Sha512>::new(None, seed);
    let mut info = Vec::with_capacity(device_id.len() + 4);
    info.extend_from_slice(device_id);
    info.extend_from_slice(&epoch.to_be_bytes());
    
    let mut x25519_secret = [0u8; 32];
    hk.expand(&info, &mut x25519_secret).expect("valid length");
    x25519_secret
}
```

### FFI Bindings

```swift
// Swift (pubky-noise FFI)
let keypair = deriveKeypair(
    seed: seedData,
    deviceId: deviceIdData,
    epoch: currentEpoch
)
// keypair.secretKeyHex, keypair.publicKeyHex
```

```kotlin
// Kotlin (pubky-noise FFI)
val keypair = deriveKeypair(
    seed = seedBytes,
    deviceId = deviceIdBytes,
    epoch = currentEpoch
)
// keypair.secretKeyHex, keypair.publicKeyHex
```

## Components

### Pubky Ring (Remote)

Manages identity keys and provides derivation:

```swift
// PubkyRingIntegration.swift
protocol PubkyRingProtocol {
    /// Request key derivation from Pubky Ring
    func requestKeyDerivation(
        deviceId: String,
        epoch: UInt32
    ) async throws -> X25519KeypairResult
    
    /// Get stored Ed25519 public key
    func getIdentityPublicKey() async throws -> String
}
```

For demo/testing, a mock service simulates Pubky Ring:

```swift
// MockPubkyRingService.swift
class MockPubkyRingService {
    /// Returns a test Ed25519 seed for demo purposes
    func getEd25519Seed() -> Data
    
    /// Derives X25519 keypair locally (demo only)
    func deriveKeypair(deviceId: String, epoch: UInt32) -> X25519KeypairResult
}
```

### NoiseKeyCache (Local)

Caches derived X25519 keys:

```swift
// NoiseKeyCache.swift
class NoiseKeyCache {
    /// Get cached key or nil if not found/expired
    func getKey(deviceId: String, epoch: UInt32) -> X25519KeypairResult?
    
    /// Store key in cache
    func storeKey(deviceId: String, epoch: UInt32, keypair: X25519KeypairResult)
    
    /// Clear all keys for device
    func clearAllKeys(deviceId: String)
    
    /// Increment epoch (triggers re-derivation)
    func rotateEpoch()
}
```

## Key Flow Examples

### Example 1: First Payment

```
User wants to send payment
         │
         ▼
Check NoiseKeyCache for X25519 keys
         │
    ┌────┴────┐
  Found     Not Found
    │           │
    │           ▼
    │     Request from Pubky Ring
    │           │
    │           ▼
    │     Ring derives X25519 from Ed25519 seed
    │           │
    │           ▼
    │     Return keypair to Paykit
    │           │
    │           ▼
    │     Store in NoiseKeyCache
    │           │
    └─────┬─────┘
          │
          ▼
Use X25519 keys for Noise handshake
          │
          ▼
Complete payment over encrypted channel
```

### Example 2: Key Rotation

```
User triggers key rotation (or automatic schedule)
         │
         ▼
Increment epoch in NoiseKeyCache
         │
         ▼
Clear cached keys for current device
         │
         ▼
Update published Noise endpoint with new pubkey
         │
         ▼
Next payment will derive new X25519 keys
```

### Example 3: Multi-Device

```
Same identity, different devices:

Device A (iPhone):
  device_id = "iPhone_14_abc123"
  epoch = 0
  → derives X25519 keypair A

Device B (Android):
  device_id = "Pixel_7_xyz789"
  epoch = 0
  → derives X25519 keypair B

Both devices can:
- Prove ownership of same identity
- Have unique Noise keys
- Rotate independently
```

## Security Properties

### Separation of Concerns

| Property | Ed25519 (pkarr) | X25519 (Noise) |
|----------|-----------------|----------------|
| Storage | Pubky Ring only | Paykit app |
| Exposure risk | Very low | Medium |
| Compromise impact | Identity loss | Session compromise |
| Rotation | Rare | Frequent |
| Backup | User responsibility | Derived on-demand |

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Paykit app compromised | Only X25519 keys exposed; identity safe |
| Network eavesdropping | Noise encryption prevents |
| Man-in-the-middle | Noise IK provides mutual auth |
| Key reuse tracking | Epoch rotation provides unlinkability |
| Device loss | X25519 keys protected by Keychain |

### Key Hierarchy

```
Ed25519 Master Seed (in Pubky Ring)
    │
    ├── Ed25519 Signing Key (identity, DHT records)
    │
    └── X25519 Derivation Path
            │
            ├── Device A, Epoch 0 → X25519 keypair
            ├── Device A, Epoch 1 → X25519 keypair
            ├── Device B, Epoch 0 → X25519 keypair
            └── ...
```

## Implementation Files

### iOS

| File | Purpose |
|------|---------|
| `Services/PubkyRingIntegration.swift` | Protocol for Ring integration |
| `Services/MockPubkyRingService.swift` | Demo/testing mock |
| `Services/NoiseKeyCache.swift` | X25519 key caching |
| `Services/NoisePaymentService.swift` | Uses keys for payments |

### Android

| File | Purpose |
|------|---------|
| `services/PubkyRingIntegration.kt` | Interface for Ring integration |
| `services/MockPubkyRingService.kt` | Demo/testing mock |
| `services/NoiseKeyCache.kt` | X25519 key caching |
| `services/NoisePaymentService.kt` | Uses keys for payments |

### Rust (pubky-noise)

| File | Purpose |
|------|---------|
| `src/kdf.rs` | Key derivation functions |
| `src/ffi.rs` | FFI bindings for mobile |
| `src/mobile/mod.rs` | FfiNoiseManager |

## Best Practices

1. **Never store Ed25519 seeds in Paykit app**
   - Always delegate to Pubky Ring

2. **Rotate X25519 keys regularly**
   - Increment epoch monthly or after N payments

3. **Use unique device IDs**
   - Combine manufacturer, model, and unique identifier

4. **Clear keys on logout**
   - Remove from cache and Keychain

5. **Validate pubkeys before use**
   - Check format and length

6. **Log sparingly**
   - Never log private keys or seeds

## Related Documentation

- [PAYMENT_FLOW_GUIDE.md](./PAYMENT_FLOW_GUIDE.md) - Payment flow details
- [TESTING_GUIDE.md](./TESTING_GUIDE.md) - Testing documentation
- [pubky-noise README](../../pubky-noise-main/README.md) - Noise library docs

