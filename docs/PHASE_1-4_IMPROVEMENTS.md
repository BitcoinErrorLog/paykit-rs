# Architectural Improvements: Phases 1-4 Implementation Summary

This document summarizes the architectural hardening implemented in Phases 1-4. These improvements should be integrated into the main [BITKIT_PAYKIT_INTEGRATION_MASTERGUIDE.md](BITKIT_PAYKIT_INTEGRATION_MASTERGUIDE.md).

## Phase 1: Ring-Only Identity Model

### Overview
Ed25519 master keys are now owned exclusively by Pubky Ring. Bitkit never generates, stores, or has access to these secrets.

### Implementation

**iOS: [`Bitkit/PaykitIntegration/KeyManager.swift`](../../bitkit-ios/Bitkit/PaykitIntegration/KeyManager.swift)**
- Removed: `generateNewIdentity()`, `storeIdentity()`, `getSecretKeyHex()`, `getSecretKeyBytes()`
- Added: `storePublicKey()`, `setCurrentEpoch()`, cache-only X25519 keypair storage
- Security comment documenting Ring ownership

**Android: [`app/src/main/java/to/bitkit/paykit/KeyManager.kt`](../../bitkit-android/app/src/main/java/to/bitkit/paykit/KeyManager.kt)**
- Removed: `generateNewIdentity()`, `getSecretKeyHex()`, `getSecretKeyBytes()`
- Added: `setCurrentEpoch()` for rotation support

### Key Management Improvements

**Cache Miss Recovery:**
- **iOS**: Added `PubkyRingIntegration.getOrRefreshKeypair()` - automatically requests from Ring when cache is empty
- **Android**: Same method in `PubkyRingIntegration.kt`

**Key Rotation:**
- **iOS**: Added `NoisePaymentService.checkKeyRotation(forceRotation:)` - checks if epoch 1 is available and rotates
- **Android**: Same method with suspend support

**Usage:**
```swift
// iOS
let keypair = try await PubkyRingIntegration.shared.getOrRefreshKeypair(deviceId: deviceId, epoch: 0)
if NoisePaymentService.shared.checkKeyRotation(forceRotation: true) {
    // Rotated to epoch 1
}
```

```kotlin
// Android
val keypair = pubkyRingIntegration.getOrRefreshKeypair(deviceId, 0u)
if (noisePaymentService.checkKeyRotation(forceRotation = true)) {
    // Rotated to epoch 1
}
```

---

## Phase 2: Secure Handoff Protocol

### Overview
Session secrets and noise keys are no longer passed in callback URLs. Instead, they're stored on the homeserver at an unguessable path and fetched by Bitkit.

### Protocol Flow

```
┌─────────┐                    ┌──────────┐                  ┌──────────┐
│  Bitkit │                    │   Ring   │                  │Homeserver│
└────┬────┘                    └────┬─────┘                  └────┬─────┘
     │                              │                             │
     │ 1. Generate ephemeral key    │                             │
     │    (optional, not used yet)  │                             │
     │                              │                             │
     │ 2. paykit-connect deep link  │                             │
     │─────────────────────────────>│                             │
     │                              │                             │
     │                              │ 3. Generate request_id      │
     │                              │    (256-bit random)         │
     │                              │                             │
     │                              │ 4. PUT /v0/handoff/{id}     │
     │                              │────────────────────────────>│
     │                              │    {session, keys, expires} │
     │                              │                             │
     │ 5. callback with request_id  │                             │
     │<─────────────────────────────│                             │
     │   (NO SECRETS IN URL!)       │                             │
     │                              │                             │
     │ 6. GET /v0/handoff/{id}      │                             │
     │─────────────────────────────────────────────────────────────>│
     │                              │                             │
     │ 7. Payload (JSON)            │                             │
     │<─────────────────────────────────────────────────────────────│
     │                              │                             │
     │ 8. DELETE /v0/handoff/{id}   │                             │
     │─────────────────────────────────────────────────────────────>│
     │                              │                             │
```

### Implementation

**Ring: [`src/utils/actions/paykitConnectAction.ts`](../../pubky-ring/src/utils/actions/paykitConnectAction.ts)**
```typescript
const requestId = generateRequestId(); // 256-bit random
const handoffPath = `pubky://${pubky}/pub/paykit.app/v0/handoff/${requestId}`;

const payload = {
  version: 1,
  pubky,
  session_secret: sessionSecret,
  capabilities,
  device_id: deviceId,
  noise_keypairs: noiseKeypairs,
  created_at: Date.now(),
  expires_at: Date.now() + 5 * 60 * 1000, // 5 minutes
};

await put(handoffPath, payload, ed25519SecretKey);

// Callback with ONLY pubky and request_id (no secrets!)
const callbackUrl = `${params.callback}?mode=secure_handoff&pubkey=${pubky}&request_id=${requestId}`;
```

**Bitkit iOS: [`Services/PubkyRingBridge.swift`](../../bitkit-ios/Bitkit/PaykitIntegration/Services/PubkyRingBridge.swift)**
```swift
private func fetchSecureHandoffPayload(pubkey: String, requestId: String) async throws -> PaykitSetupResult {
    let handoffUri = "pubky://\(pubkey)/pub/paykit.app/v0/handoff/\(requestId)"
    let data = try await PubkySDKService.shared.get(uri: handoffUri)
    let payload = try JSONDecoder().decode(SecureHandoffPayload.self, from: data)
    
    // Delete payload immediately after fetch
    try? await deleteHandoffFile(path: handoffUri, session: payload.sessionSecret)
    
    return buildSetupResult(from: payload)
}
```

**Bitkit Android: [`services/PubkyRingBridge.kt`](../../bitkit-android/app/src/main/java/to/bitkit/paykit/services/PubkyRingBridge.kt)**
```kotlin
private suspend fun fetchSecureHandoffPayload(pubkey: String, requestId: String): PaykitSetupResult {
    val handoffUri = "pubky://$pubkey/pub/paykit.app/v0/handoff/$requestId"
    val result = uniffi.pubkycore.get(handoffUri)
    val payload = json.decodeFromString<SecureHandoffPayload>(result[1])
    
    // Note: Cleanup documented - handled by TTL or Ring deletion
    
    return buildSetupResult(payload)
}
```

### Security Benefits
- **No secrets in URLs**: Protects against URL logging/leaks
- **Unguessable path**: 256-bit random request_id makes brute-force infeasible
- **Time-limited**: 5-minute expiry window
- **Defense in depth**: Bitkit deletes after fetch (iOS), TTL enforcement (homeserver)

---

## Phase 3: Private Push Relay Service

### Overview
Push notification tokens are stored server-side instead of being published publicly. This prevents DoS attacks and improves privacy.

### Architecture

See full design in [PUSH_RELAY_DESIGN.md](PUSH_RELAY_DESIGN.md).

**Key URLs:**
- Production: `https://push.paykit.app/v1`
- Staging: `https://push-staging.paykit.app/v1`

### API Endpoints

#### POST /register
Register device token with the relay:
```json
{
  "platform": "ios" | "android",
  "token": "<apns_or_fcm_token>",
  "capabilities": ["wake", "payment_received"]
}
```

**Authentication**: Ed25519 signature via Ring
```
X-Pubky-Signature: <hex_signature>
X-Pubky-Timestamp: <unix_timestamp>
X-Pubky-Pubkey: <z32_pubkey>
```

#### POST /wake
Send wake notification to recipient:
```json
{
  "recipient_pubkey": "<recipient_z32>",
  "wake_type": "noise_connect",
  "sender_pubkey": "<sender_z32>",
  "nonce": "<random_hex>"
}
```

### Implementation

**iOS: [`Services/PushRelayService.swift`](../../bitkit-ios/Bitkit/PaykitIntegration/Services/PushRelayService.swift)**
```swift
// Register on app launch
if PushRelayService.shared.isEnabled {
    let token = await getAPNsToken()
    try await PushRelayService.shared.register(token: token)
}

// Send wake notification before Noise connect
try await PushRelayService.shared.wake(
    recipientPubkey: recipientPubkey,
    wakeType: .noiseConnect
)
```

**Android: [`services/PushRelayService.kt`](../../bitkit-android/app/src/main/java/to/bitkit/paykit/services/PushRelayService.kt)**
```kotlin
// Register on app launch
if (PushRelayService.isEnabled()) {
    val token = getFCMToken()
    pushRelayService.register(token)
}

// Send wake notification
pushRelayService.wake(
    recipientPubkey = recipientPubkey,
    wakeType = WakeType.NOISE_CONNECT
)
```

### Ed25519 Signing via Ring

Push relay requests require Ed25519 signatures. Since Bitkit doesn't have the secret key, it requests signatures from Ring:

**Ring: [`actions/signMessageAction.ts`](../../pubky-ring/src/utils/actions/signMessageAction.ts)**
```typescript
// Deep link: pubkyring://sign-message?message={message}&callback={url}
const signature = await PubkyNoise.signEd25519(secretKey, messageBytes);
// Returns: bitkit://signature-result?signature={hex}&pubkey={z32}
```

**iOS:**
```swift
let signature = try await PubkyRingBridge.shared.requestSignature(message: message)
```

**Android:**
```kotlin
val signature = pubkyRingBridge.requestSignature(context, message)
```

### Migration from Public Publishing

**Deprecated Methods (DO NOT USE):**
- `DirectoryService.publishPushNotificationEndpoint()` - publishes tokens publicly
- `DirectoryService.discoverPushNotificationEndpoint()` - reads public tokens

These methods are marked `@available(*, deprecated)` (iOS) and `@Deprecated` (Android).

---

## Phase 4: Type-Safe Homeserver Identifiers

### Overview
Introduced typed wrappers to prevent confusion between pubkeys and URLs.

### Types

**iOS: [`Types/HomeserverTypes.swift`](../../bitkit-ios/Bitkit/PaykitIntegration/Types/HomeserverTypes.swift)**  
**Android: [`types/HomeserverTypes.kt`](../../bitkit-android/app/src/main/java/to/bitkit/paykit/types/HomeserverTypes.kt)**

```swift
// iOS
struct HomeserverPubkey {
    let value: String        // z32 pubkey (without pk: prefix)
    var withPrefix: String   // with pk: prefix
    var isValid: Bool        // format validation
}

struct HomeserverURL {
    let value: String        // https://homeserver.pubky.app
    var isValid: Bool
    func urlForPath(owner: String, path: String) -> URL?
}

struct OwnerPubkey {
    let value: String        // z32 pubkey
    var withPrefix: String
    var isValid: Bool
}

struct SessionSecret {
    let hexValue: String
    var bytes: Data?
    // Redacts when printed: SessionSecret(***)
}
```

### HomeserverResolver

Centralized pubkey→URL resolution with caching:

**iOS:**
```swift
let pubkey = HomeserverPubkey("8um71us3fyw6h...")
let url = HomeserverResolver.shared.resolve(pubkey: pubkey)
// Returns: HomeserverURL("https://homeserver.pubky.app")

// Override for testing
HomeserverResolver.shared.overrideURL = HomeserverURL("https://test.example.com")

// Add custom mapping
HomeserverResolver.shared.addMapping(
    pubkey: HomeserverPubkey("abc..."),
    url: HomeserverURL("https://custom.homeserver.com")
)
```

**Android:**
```kotlin
val pubkey = HomeserverPubkey("8um71us3fyw6h...")
val url = HomeserverResolver.resolve(pubkey)

// Override and custom mappings work the same way
```

### Known Homeservers

Default mappings included:
- `8um71us3fyw6h...` → `https://homeserver.pubky.app` (production)
- `ufibwbmed6jeq...` → `https://staging.homeserver.pubky.app` (staging)

### Adoption in Services

**DirectoryService:**
- Changed `homeserverBaseURL: String?` → `homeserverURL: HomeserverURL?`
- Changed `ownerPubkey: String?` → `ownerPubkey: OwnerPubkey?`

**PubkyStorageAdapter:**
- Constructors accept `HomeserverURL?`
- Internal methods convert to `String` using `.value` property

**Usage:**
```swift
// iOS
directoryService.configurePubkyTransport(
    homeserverURL: HomeserverResolver.shared.resolve(pubkey: myHomeserver)
)
```

```kotlin
// Android  
directoryService.configurePubkyTransport(
    homeserverURL = HomeserverResolver.resolve(myHomeserver)
)
```

---

## Integration Checklist Updates

### Section 16.4: Pubky Ring Bridge

**Add to checklist:**
- [ ] `PubkyRingBridge.requestSignature()` works for Ed25519 signing
- [ ] `pubkyring://sign-message` deep link handler implemented
- [ ] Signature returned via `signature-result` callback
- [ ] Secure handoff mode works (no secrets in callback URL)
- [ ] Handoff payload deleted from homeserver after fetch

### Section 16.5: Key Management

**Add to checklist:**
- [ ] Bitkit does NOT store Ed25519 secrets anywhere
- [ ] X25519 keypairs only (derived by Ring, cached by Bitkit)
- [ ] `getOrRefreshKeypair()` auto-recovers from cache miss
- [ ] `checkKeyRotation()` supports epoch 0 → epoch 1 rotation
- [ ] `setCurrentEpoch()` available for manual rotation

### Section 16.6: Push Notifications (NEW)

**Add new section:**
- [ ] `PushRelayService` registered on app launch
- [ ] Deprecated `DirectoryService.publishPushNotificationEndpoint()` NOT called
- [ ] Wake notifications use `PushRelayService.wake()`
- [ ] Signature authentication works via Ring bridge
- [ ] Registration renewed before expiry (check `needsRenewal` property)

---

## Production Configuration

### Environment Variables

**iOS (Xcode scheme or Info.plist):**
```
PUSH_RELAY_URL=https://push.paykit.app/v1
PUSH_RELAY_ENABLED=true
```

**Android (local.properties or BuildConfig):**
```
PUSH_RELAY_URL=https://push.paykit.app/v1
PUSH_RELAY_ENABLED=true
```

### HomeserverResolver Configuration

For custom homeservers, add mappings on app init:
```swift
// iOS
HomeserverResolver.shared.addMapping(
    pubkey: HomeserverPubkey("custom_pubkey"),
    url: HomeserverURL("https://custom.homeserver.com")
)
```

```kotlin
// Android
HomeserverResolver.addMapping(
    HomeserverPubkey("custom_pubkey"),
    HomeserverURL("https://custom.homeserver.com")
)
```

---

## Security Model Updates

### Identity Ownership
- Ed25519 keys: **Ring only** (never exported, never cached)
- X25519 keys: Derived by Ring, **cached by Bitkit** for offline operation
- Session secrets: Created by Ring, **stored by Bitkit** in secure storage

### Signature Flow
All external API requests requiring Ed25519 signatures must:
1. Bitkit constructs message to sign
2. Bitkit requests signature from Ring via `sign-message` deep link
3. Ring signs with Ed25519 secret
4. Ring returns signature via callback
5. Bitkit uses signature in API request

**Never** attempt to sign locally in Bitkit.

### Handoff Security
- Secrets stored at unguessable path (`/pub/paykit.app/v0/handoff/{256-bit-random-id}`)
- 5-minute TTL enforced via `expires_at` field
- Payload deleted immediately after successful fetch (iOS)
- Defense in depth: TTL + immediate deletion

---

## Troubleshooting

### "No keypair cached" errors
**Cause**: Bitkit's X25519 cache was cleared  
**Fix**: Call `getOrRefreshKeypair()` instead of `getCachedKeypair()` - it auto-recovers

### "Placeholder signature" warnings
**Cause**: `PushRelayService` couldn't get signature from Ring  
**Fix**: Ensure Ring is installed and `requestSignature()` method is implemented

### Type mismatch errors (pubkey vs URL)
**Cause**: Passing raw strings where typed values expected  
**Fix**: Wrap in type constructors:
- `HomeserverPubkey("8um71us...")` for pubkeys
- `HomeserverURL("https://...")` for URLs
- `OwnerPubkey("abc123...")` for user pubkeys

---

## Future Work

### Automatic Key Rotation
Currently rotation is manual (`forceRotation: true`). Future improvements:
- Time-based rotation (30 days after epoch 0 creation)
- External rotation signals from Ring
- Automatic background rotation checks

### DNS-Based Homeserver Resolution
`HomeserverResolver.resolve()` currently uses static mappings. Future:
- Query `_pubky.<pubkey>` DNS TXT records
- Fallback to static mappings
- Cache DNS results with appropriate TTL

### Push Relay Server
Design complete ([PUSH_RELAY_DESIGN.md](PUSH_RELAY_DESIGN.md)), server implementation needed:
- APNs/FCM integration
- Rate limiting middleware
- Token encryption at rest
- Monitoring and analytics

---

**Last Updated**: December 22, 2025  
**Phases Covered**: 1 (Identity), 2 (Handoff), 3 (Push Relay), 4 (Type Safety)

