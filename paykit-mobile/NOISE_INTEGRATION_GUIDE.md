# Mobile Noise Protocol Integration Guide

This guide documents the integration of the Noise protocol into the Paykit mobile demos for secure interactive payments.

## Overview

The Noise protocol provides authenticated encryption for peer-to-peer payment negotiations. The integration includes:

- **iOS**: Swift/SwiftUI `PaymentView` using `FfiNoiseManager`
- **Android**: Kotlin/Compose `PaymentScreen` using `FfiNoiseManager`

## iOS Integration

### Prerequisites

1. **XCFramework**: The `PubkyNoise.xcframework` has been generated and placed at:
   ```
   paykit-mobile/ios-demo/PaykitDemo/PaykitDemo/PubkyNoise.xcframework
   ```

2. **Swift Bindings**: The `PubkyNoise.swift` file is at:
   ```
   paykit-mobile/ios-demo/PaykitDemo/PaykitDemo/PaykitDemo/PubkyNoise.swift
   ```

3. **FFI Header**: The `PubkyNoiseFFI.h` header is at:
   ```
   paykit-mobile/ios-demo/PaykitDemo/PaykitDemo/PaykitDemo/PubkyNoiseFFI.h
   ```

### Setup Steps

1. **Add XCFramework to Xcode Project**:
   - Open `PaykitDemo.xcodeproj` in Xcode
   - Select the project in the navigator
   - Select the "PaykitDemo" target
   - Go to "General" → "Frameworks, Libraries, and Embedded Content"
   - Click "+" and select "Add Other..." → "Add Files..."
   - Navigate to `PubkyNoise.xcframework` and add it
   - Set "Embed" to "Embed & Sign"

2. **Update Bridging Header** (already done):
   The `PaykitDemo-Bridging-Header.h` has been updated to include:
   ```c
   #import "PubkyNoiseFFI.h"
   ```

3. **Link Security Framework**:
   The `Network.framework` is used for TCP connections. It should be automatically linked.

### Usage

The `PaymentView.swift` provides a complete payment flow:

```swift
// In DashboardView or wherever you want to trigger payment
@State private var showingPaymentView = false

Button("Send Payment") {
    showingPaymentView = true
}
.sheet(isPresented: $showingPaymentView) {
    PaymentView()
}
```

### Key Components

1. **PaymentViewModel**: Manages the payment flow state
2. **FfiNoiseManager**: Rust FFI wrapper for Noise protocol
3. **Network.framework**: TCP connections via `NWConnection`

## Android Integration

### Prerequisites

1. **Native Libraries**: The `libpubky_noise.so` files are at:
   ```
   paykit-mobile/android-demo/app/src/main/jniLibs/arm64-v8a/libpubky_noise.so
   paykit-mobile/android-demo/app/src/main/jniLibs/x86_64/libpubky_noise.so
   ```

2. **Kotlin Bindings**: The `pubky_noise.kt` file is at:
   ```
   paykit-mobile/android-demo/app/src/main/java/com/pubky/noise/pubky_noise.kt
   ```

### Setup Steps

1. **Add JNA Dependency** (if not already present):
   In `app/build.gradle.kts`:
   ```kotlin
   dependencies {
       implementation("net.java.dev.jna:jna:5.14.0@aar")
   }
   ```

2. **Load Native Library**:
   The UniFFI bindings automatically load the library.

### Usage

The `PaymentScreen.kt` provides a complete payment flow:

```kotlin
// In DashboardScreen or wherever you want to trigger payment
var showPaymentScreen by remember { mutableStateOf(false) }

Button(onClick = { showPaymentScreen = true }) {
    Text("Send Payment")
}

if (showPaymentScreen) {
    PaymentScreen(
        keyManager = keyManager,
        onPaymentComplete = {
            showPaymentScreen = false
        }
    )
}
```

### Key Components

1. **PaymentScreen composable**: UI for payment flow
2. **FfiNoiseManager**: UniFFI wrapper for Noise protocol
3. **Socket**: Standard Java TCP connections

## Payment Flow

Both iOS and Android follow the same payment flow:

1. **Resolve Recipient**: Parse `pubky://` URI or lookup contact by name
2. **Discover Endpoint**: Query Pubky directory for `noise://` endpoint
3. **Connect**: Establish TCP connection to the noise endpoint
4. **Handshake**: Perform Noise IK handshake
   - Create `FfiNoiseManager` with client seed
   - Call `initiateConnection()` to get first message
   - Send first message, receive server response
   - Call `completeConnection()` to finalize
5. **Send Request**: Encrypt and send payment request
6. **Receive Confirmation**: Decrypt and process receipt

## Noise Protocol API

### Creating a Client Manager

```swift
// iOS
let config = FfiMobileConfig(
    autoReconnect: false,
    maxReconnectAttempts: 0,
    reconnectDelayMs: 0,
    batterySaver: false,
    chunkSize: 32768
)
let manager = try FfiNoiseManager.newClient(
    config: config,
    clientSeed: seed,  // 32-byte secret key
    clientKid: "paykit-ios",
    deviceId: deviceIdData
)
```

```kotlin
// Android
val config = FfiMobileConfig(
    autoReconnect = false,
    maxReconnectAttempts = 0u,
    reconnectDelayMs = 0u,
    batterySaver = false,
    chunkSize = 32768u
)
val manager = FfiNoiseManager.newClient(
    config = config,
    clientSeed = seed,  // 32-byte secret key
    clientKid = "paykit-android",
    deviceId = deviceId
)
```

### Handshake

```swift
// iOS
let result = try manager.initiateConnection(serverPk: serverPublicKey, hint: nil)
// Send result.firstMessage to server
// Receive serverResponse
let sessionId = try manager.completeConnection(
    sessionId: result.sessionId,
    serverResponse: serverResponse
)
```

```kotlin
// Android
val result = manager.initiateConnection(serverPk, null)
// Send result.firstMessage to server
// Receive serverResponse
val sessionId = manager.completeConnection(result.sessionId, serverResponse)
```

### Encrypt/Decrypt

```swift
// iOS
let ciphertext = try manager.encrypt(sessionId: sessionId, plaintext: plaintext)
let plaintext = try manager.decrypt(sessionId: sessionId, ciphertext: ciphertext)
```

```kotlin
// Android
val ciphertext = manager.encrypt(sessionId, plaintext)
val plaintext = manager.decrypt(sessionId, ciphertext)
```

### Cleanup

```swift
// iOS
manager.removeSession(sessionId: sessionId)
```

```kotlin
// Android
manager.removeSession(sessionId)
```

## Current Limitations

1. **Endpoint Discovery**: The `discoverNoiseEndpoint()` function currently returns `null` as it requires Pubky directory integration. To test the payment flow, recipients must publish their `noise://` endpoints.

2. **Server Mode**: The current implementation is client-only. For receiving payments, the app would need to run as a Noise server with a published endpoint.

3. **Message Format**: The payment request/receipt format is simplified. Production use should integrate with the full `paykit-interactive` message format.

## Testing

To test the Noise payment flow:

1. Set up a Noise server (can use the example from `pubky-noise-main/examples/server_example.rs`)
2. Publish the server's `noise://host:port@pubkey` endpoint
3. Enter the endpoint in the recipient field
4. The payment flow will connect, handshake, and exchange messages

## Future Work

- [ ] Integrate with Pubky directory for automatic endpoint discovery
- [ ] Add server mode for receiving payments
- [ ] Implement full `paykit-interactive` message protocol
- [ ] Add QR code scanning for noise endpoints
- [ ] Add connection retry and resilience

