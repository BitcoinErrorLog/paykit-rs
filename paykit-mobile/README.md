# Paykit Mobile

Mobile FFI bindings for Paykit, enabling integration with iOS (Swift) and Android (Kotlin) applications.

## Overview

This crate provides UniFFI-based bindings that expose Paykit functionality to mobile platforms:

- **Directory Operations**: Publish and discover payment endpoints
- **Payment Method Selection**: Automatic selection of optimal payment methods
- **Subscription Management**: Create, modify, and manage subscriptions
- **Contact Management**: Add, remove, and sync contacts
- **Interactive Protocol**: Message building and receipt storage for Noise channels
- **Health Monitoring**: Check payment method availability
- **QR Code Scanning**: Parse Paykit URIs from scanned QR codes

## Building

### Prerequisites

- Rust 1.70+
- UniFFI 0.25+
- For iOS: Xcode, Swift 5.5+
- For Android: Android NDK, Kotlin 1.8+

### Build the Library

```bash
# Debug build
cargo build -p paykit-mobile

# Release build
cargo build --release -p paykit-mobile
```

### Generate Bindings

```bash
# Install uniffi-bindgen CLI (if not installed)
cargo install uniffi-bindgen-cli@0.25

# Generate bindings
./paykit-mobile/generate-bindings.sh

# Or manually:
# Generate Swift bindings
uniffi-bindgen generate --library target/release/libpaykit_mobile.dylib -l swift -o paykit-mobile/swift

# Generate Kotlin bindings
uniffi-bindgen generate --library target/release/libpaykit_mobile.dylib -l kotlin -o paykit-mobile/kotlin
```

## Core APIs

### PaykitClient

The main entry point for Paykit operations.

```swift
// Swift
let client = try! PaykitClient()

// List payment methods
let methods = client.listMethods()

// Validate endpoint
let isValid = try! client.validateEndpoint(methodId: "onchain", endpoint: "bc1q...")

// Select optimal method
let selection = try! client.selectMethod(
    supportedMethods: methods,
    amountSats: 10000,
    preferences: nil
)
```

```kotlin
// Kotlin
val client = PaykitClient()

// List payment methods
val methods = client.listMethods()

// Validate endpoint
val isValid = client.validateEndpoint(methodId = "onchain", endpoint = "bc1q...")

// Select optimal method
val selection = client.selectMethod(
    supportedMethods = methods,
    amountSats = 10000u,
    preferences = null
)
```

### Directory Operations

Publish and discover payment endpoints using transport wrappers.

```swift
// Swift
// Create transports
let authTransport = AuthenticatedTransportFFI.newMock(ownerPubkey: myPubkey)
let unauthTransport = UnauthenticatedTransportFFI.fromAuthenticated(auth: authTransport)

// Publish payment endpoint
try! client.publishPaymentEndpoint(
    transport: authTransport,
    methodId: "lightning",
    endpointData: "lnbc1..."
)

// Discover payment methods
let methods = try! client.fetchSupportedPayments(
    transport: unauthTransport,
    ownerPubkey: peerPubkey
)

// Fetch specific endpoint
let endpoint = try! client.fetchPaymentEndpoint(
    transport: unauthTransport,
    ownerPubkey: peerPubkey,
    methodId: "lightning"
)

// Remove endpoint
try! client.removePaymentEndpointFromDirectory(
    transport: authTransport,
    methodId: "lightning"
)
```

```kotlin
// Kotlin
// Create transports
val authTransport = AuthenticatedTransportFFI.newMock(myPubkey)
val unauthTransport = UnauthenticatedTransportFFI.fromAuthenticated(authTransport)

// Publish payment endpoint
client.publishPaymentEndpoint(
    transport = authTransport,
    methodId = "lightning",
    endpointData = "lnbc1..."
)

// Discover payment methods
val methods = client.fetchSupportedPayments(
    transport = unauthTransport,
    ownerPubkey = peerPubkey
)
```

### Contact Management

Manage contacts locally and sync with remote storage.

```swift
// Swift
// Add contacts
try! client.addContact(transport: authTransport, contactPubkey: "peer1")
try! client.addContact(transport: authTransport, contactPubkey: "peer2")

// List contacts
let contacts = try! client.listContacts(transport: authTransport)

// Remove contact
try! client.removeContact(transport: authTransport, contactPubkey: "peer1")

// Local contact cache
let cache = ContactCacheFFI()
cache.add(pubkey: "peer1")
cache.addWithName(pubkey: "peer2", name: "Alice")

// Sync with remote
let result = cache.sync(remotePubkeys: remoteContacts)
print("Added: \(result.added), Total: \(result.total)")
```

### Interactive Protocol

Build messages for Noise channel communication.

```swift
// Swift
let builder = PaykitMessageBuilder()

// Create endpoint offer
let offerJson = try! builder.createEndpointOffer(
    methodId: "lightning",
    endpoint: "lnbc1..."
)
// Send offerJson over Noise channel

// Parse received message
let parsed = try! builder.parseMessage(messageJson: receivedJson)
switch parsed {
case .offerPrivateEndpoint(let offer):
    print("Received offer: \(offer.methodId) -> \(offer.endpoint)")
case .requestReceipt(let request):
    print("Receipt requested: \(request.receiptId)")
case .ack:
    print("Acknowledged")
case .error(let error):
    print("Error: \(error.code) - \(error.message)")
}

// Store receipts
let store = ReceiptStore()
store.saveReceipt(receipt)
let receipts = store.listReceipts()
```

### Subscription Management

```swift
// Swift
let subscription = try! client.createSubscription(
    subscriber: subscriberPubkey,
    provider: providerPubkey,
    terms: SubscriptionTerms(
        amountSats: 1000,
        currency: "SAT",
        frequency: .monthly(dayOfMonth: 1),
        methodId: "lightning",
        description: "Monthly subscription"
    )
)

// Calculate proration
let proration = try! client.calculateProration(
    currentAmountSats: 1000,
    newAmountSats: 2000,
    periodStart: periodStart,
    periodEnd: periodEnd,
    changeDate: changeDate
)
```

### QR Code Scanning

```swift
// Swift
// Check if scanned data is a Paykit URI
if client.isPaykitQr(scannedData: qrCode) {
    let result = try! client.parseScannedQr(scannedData: qrCode)
    switch result.uriType {
    case .pubky:
        let pubkey = result.publicKey!
        // Start payment flow
    case .invoice:
        let method = result.methodId!
        let data = result.data!
        // Process invoice
    case .paymentRequest:
        let requestId = result.requestId!
        // Handle payment request
    }
}
```

## Type Reference

### Core Types

| Type | Description |
|------|-------------|
| `PaykitClient` | Main client for all Paykit operations |
| `AuthenticatedTransportFFI` | Write access for publishing endpoints |
| `UnauthenticatedTransportFFI` | Read access for discovering endpoints |
| `PaymentMethod` | Method ID and endpoint pair |
| `Amount` | Value and currency |

### Directory Types

| Type | Description |
|------|-------------|
| `DirectoryOperationsAsync` | Async directory operations manager |
| `ContactCacheFFI` | Local contact cache with sync |
| `CachedContactFFI` | Cached contact with metadata |
| `SyncResultFFI` | Result of contact sync operation |

### Interactive Types

| Type | Description |
|------|-------------|
| `PaykitMessageBuilder` | Create protocol messages |
| `ReceiptStore` | Store receipts and private endpoints |
| `ReceiptRequest` | Receipt request details |
| `PrivateEndpointOffer` | Private endpoint offer |
| `ParsedMessage` | Parsed protocol message |
| `PaykitMessageType` | Message type enum |

### Selection Types

| Type | Description |
|------|-------------|
| `SelectionStrategy` | Balanced, CostOptimized, SpeedOptimized, PrivacyOptimized |
| `SelectionPreferences` | Strategy and constraints |
| `SelectionResult` | Primary and fallback methods |

### Subscription Types

| Type | Description |
|------|-------------|
| `SubscriptionTerms` | Subscription configuration |
| `PaymentFrequency` | Daily, Weekly, Monthly, Yearly, Custom |
| `ProrationResult` | Credit, charge, and net amounts |

### Error Types

| Error | Description |
|-------|-------------|
| `Transport` | Network/I/O errors |
| `Validation` | Invalid input/format |
| `NotFound` | Resource not found |
| `Serialization` | JSON errors |
| `NetworkTimeout` | Connection timeout |
| `AuthenticationError` | Auth failed |
| `SessionError` | Session expired/invalid |
| `RateLimitError` | Rate limit exceeded |
| `PermissionDenied` | Access denied |

## Thread Safety

All types are thread-safe. The `PaykitClient` manages its own Tokio runtime internally and can be used from any thread.

## Secure Storage

### iOS (Keychain)

Use the provided `KeychainStorage.swift` adapter:

```swift
let keychain = KeychainStorage(serviceIdentifier: Bundle.main.bundleIdentifier!)
keychain.store(key: "private_key", value: keyData)
```

### Android (EncryptedSharedPreferences)

Use the provided `EncryptedPreferencesStorage.kt` adapter:

```kotlin
val storage = EncryptedPreferencesStorage.create(context)
storage.store("private_key", keyBytes)
```

## Integration with pubky-noise

For encrypted channel communication, use the `pubky-noise` FFI bindings:

```swift
// 1. Establish Noise connection using pubky-noise
let noiseManager = FfiNoiseManager.newClient(config: config, seed: seed, kid: kid, deviceId: deviceId)
let result = noiseManager.initiateConnection(serverPk: peerPubkey, hint: nil)

// 2. Complete handshake
let sessionId = noiseManager.completeConnection(sessionId: result.sessionId, serverResponse: response)

// 3. Use PaykitMessageBuilder to create messages
let builder = PaykitMessageBuilder()
let offerJson = builder.createEndpointOffer(methodId: "lightning", endpoint: invoice)

// 4. Encrypt and send via Noise channel
let encrypted = noiseManager.encrypt(sessionId: sessionId, plaintext: offerJson.data(using: .utf8)!)
// Send encrypted data...

// 5. Receive and decrypt
let decrypted = noiseManager.decrypt(sessionId: sessionId, ciphertext: receivedData)
let parsed = builder.parseMessage(messageJson: String(data: decrypted, encoding: .utf8)!)
```

## Demo Apps

Complete demo applications are available:

- **iOS Demo**: `ios-demo/` - SwiftUI app with full Paykit features
- **Android Demo**: `android-demo/` - Jetpack Compose app with Material 3

## License

MIT
