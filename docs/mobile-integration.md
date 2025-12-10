# Mobile Integration Guide

This guide provides step-by-step instructions for integrating Paykit into iOS and Android applications.

## Table of Contents

- [Prerequisites](#prerequisites)
- [iOS Integration](#ios-integration)
- [Android Integration](#android-integration)
- [Usage Examples](#usage-examples)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Tools

- **Rust**: 1.70+ with cargo
- **UniFFI**: 0.25+ (`cargo install uniffi-bindgen-cli@0.25`)
- **iOS**: Xcode 14+, Swift 5.5+
- **Android**: Android Studio, Kotlin 1.8+, NDK

### Build the Library

```bash
# From the workspace root
cargo build --release -p paykit-mobile
```

This generates:
- `target/release/libpaykit_mobile.dylib` (macOS)
- `target/release/libpaykit_mobile.a` (static library)
- `target/release/libpaykit_mobile.so` (Linux/Android)

## iOS Integration

### Step 1: Generate Swift Bindings

```bash
cd paykit-mobile
uniffi-bindgen generate \
    --library ../target/release/libpaykit_mobile.dylib \
    -l swift \
    -o swift/
```

This creates `swift/paykit_mobile.swift` with all FFI bindings.

### Step 2: Add to Xcode Project

1. **Add the Swift file**:
   - Drag `swift/paykit_mobile.swift` into your Xcode project
   - Ensure "Copy items if needed" is checked

2. **Link the library**:
   - Go to your target's "Build Phases"
   - Under "Link Binary with Libraries", add:
     - `libpaykit_mobile.a` (static library)
     - Or configure dynamic linking for `.dylib`

3. **Configure Build Settings**:
   - Add library search path: `$(PROJECT_DIR)/../target/release`
   - Set "Other Linker Flags": `-L$(PROJECT_DIR)/../target/release -lpaykit_mobile`

### Step 3: Use in Swift

```swift
import Foundation

// Create the client
let client = try! PaykitClient()

// List available payment methods
let methods = client.listMethods()
print("Available methods: \(methods)")

// Validate an endpoint
let isValid = try! client.validateEndpoint(
    methodId: "onchain",
    endpoint: "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
)

// Select optimal method
let selection = try! client.selectMethod(
    supportedMethods: [
        PaymentMethod(methodId: "lightning", endpoint: "lnbc1..."),
        PaymentMethod(methodId: "onchain", endpoint: "bc1q...")
    ],
    amountSats: 10000,
    preferences: nil
)
print("Selected: \(selection.primaryMethod)")

// Parse scanned QR code
if let scanned = try? client.parseScannedQr(scannedData: "pubky://abc123...") {
    if scanned.uriType == .pubky {
        print("Public key: \(scanned.publicKey!)")
    }
}
```

### Step 4: Secure Storage (iOS Keychain)

Use the provided `KeychainStorage` adapter:

```swift
import Foundation

// Create Keychain storage
let keychain = KeychainStorage(
    serviceIdentifier: Bundle.main.bundleIdentifier ?? "com.example.app"
)

// Use with Paykit
let storage = keychain.asPaykitStorage()

// Store private data
try? storage.store(key: "private_key", value: keyData)

// Retrieve
if let data = try? storage.retrieve(key: "private_key") {
    // Use the data
}
```

## Android Integration

### Step 1: Generate Kotlin Bindings

```bash
cd paykit-mobile
uniffi-bindgen generate \
    --library ../target/release/libpaykit_mobile.so \
    -l kotlin \
    -o kotlin/
```

This creates Kotlin bindings in `kotlin/uniffi/paykit_mobile/`.

### Step 2: Add to Android Project

1. **Copy Kotlin files**:
   - Copy `kotlin/uniffi/paykit_mobile/` to `app/src/main/java/`

2. **Add native library**:
   - Create `app/src/main/jniLibs/` directory
   - Copy `libpaykit_mobile.so` for your target architecture:
     - `jniLibs/arm64-v8a/libpaykit_mobile.so`
     - `jniLibs/armeabi-v7a/libpaykit_mobile.so`
     - `jniLibs/x86_64/libpaykit_mobile.so`

3. **Update build.gradle**:
   ```kotlin
   android {
       // ... existing config ...
       
       sourceSets {
           main {
               jniLibs.srcDirs("src/main/jniLibs")
           }
       }
   }
   ```

### Step 3: Use in Kotlin

```kotlin
import uniffi.paykit_mobile.*

// Create the client
val client = PaykitClient()

// List available payment methods
val methods = client.listMethods()
println("Available methods: $methods")

// Validate an endpoint
val isValid = client.validateEndpoint(
    methodId = "onchain",
    endpoint = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
)

// Select optimal method
val selection = client.selectMethod(
    supportedMethods = listOf(
        PaymentMethod("lightning", "lnbc1..."),
        PaymentMethod("onchain", "bc1q...")
    ),
    amountSats = 10000u,
    preferences = null
)
println("Selected: ${selection.primaryMethod}")

// Parse scanned QR code
val scanned = client.parseScannedQr("pubky://abc123...")
when (scanned.uriType) {
    UriType.PUBKY -> println("Public key: ${scanned.publicKey}")
    UriType.INVOICE -> println("Invoice: ${scanned.data}")
    else -> {}
}
```

### Step 4: Secure Storage (Android EncryptedSharedPreferences)

Create a Kotlin adapter:

```kotlin
import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

class EncryptedStorageAdapter(context: Context) {
    private val masterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()
    
    private val prefs = EncryptedSharedPreferences.create(
        context,
        "paykit_secure_prefs",
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )
    
    fun store(key: String, value: ByteArray) {
        prefs.edit().putString(key, value.toString(Charsets.UTF_8)).apply()
    }
    
    fun retrieve(key: String): ByteArray? {
        return prefs.getString(key, null)?.toByteArray(Charsets.UTF_8)
    }
    
    fun delete(key: String) {
        prefs.edit().remove(key).apply()
    }
}
```

## Usage Examples

### Creating a Payment Request

**Swift:**
```swift
let request = try! client.createPaymentRequest(
    fromPubkey: myPubkey,
    toPubkey: recipientPubkey,
    amountSats: 1000,
    currency: "SAT",
    methodId: "lightning",
    description: "Payment for services",
    expiresInSecs: 3600
)
```

**Kotlin:**
```kotlin
val request = client.createPaymentRequest(
    fromPubkey = myPubkey,
    toPubkey = recipientPubkey,
    amountSats = 1000,
    currency = "SAT",
    methodId = "lightning",
    description = "Payment for services",
    expiresInSecs = 3600u
)
```

### Managing Subscriptions

**Swift:**
```swift
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

// Calculate proration for upgrade
let proration = try! client.calculateProration(
    currentAmountSats: 1000,
    newAmountSats: 2000,
    periodStart: periodStart,
    periodEnd: periodEnd,
    changeDate: changeDate
)
```

**Kotlin:**
```kotlin
val subscription = client.createSubscription(
    subscriber = subscriberPubkey,
    provider = providerPubkey,
    terms = SubscriptionTerms(
        amountSats = 1000,
        currency = "SAT",
        frequency = PaymentFrequency.Monthly(dayOfMonth = 1u),
        methodId = "lightning",
        description = "Monthly subscription"
    )
)

// Calculate proration
val proration = client.calculateProration(
    currentAmountSats = 1000,
    newAmountSats = 2000,
    periodStart = periodStart,
    periodEnd = periodEnd,
    changeDate = changeDate
)
```

### QR Code Scanning

**Swift:**
```swift
// After scanning QR code
let scannedData = qrCodeString

// Check if it's a Paykit URI
if client.isPaykitQr(scannedData: scannedData) {
    // Parse it
    if let result = try? client.parseScannedQr(scannedData: scannedData) {
        switch result.uriType {
        case .pubky:
            // Start payment flow with public key
            let pubkey = result.publicKey!
        case .invoice:
            // Process invoice
            let method = result.methodId!
            let data = result.data!
        case .paymentRequest:
            // Handle payment request
            let requestId = result.requestId!
        case .unknown:
            break
        }
    }
}
```

**Kotlin:**
```kotlin
// After scanning QR code
val scannedData = qrCodeString

// Check if it's a Paykit URI
if (client.isPaykitQr(scannedData)) {
    // Parse it
    val result = client.parseScannedQr(scannedData)
    when (result.uriType) {
        UriType.PUBKY -> {
            // Start payment flow
            val pubkey = result.publicKey
        }
        UriType.INVOICE -> {
            // Process invoice
            val method = result.methodId
            val data = result.data
        }
        UriType.PAYMENT_REQUEST -> {
            // Handle payment request
            val requestId = result.requestId
        }
        UriType.UNKNOWN -> {}
    }
}
```

## Troubleshooting

### iOS Issues

**Problem**: Library not found
- **Solution**: Ensure library search paths are correctly configured in Build Settings

**Problem**: Swift bindings not found
- **Solution**: Verify `paykit_mobile.swift` is added to the target and included in compilation

**Problem**: Keychain access denied
- **Solution**: Add Keychain Sharing capability in Xcode and configure access groups

### Android Issues

**Problem**: UnsatisfiedLinkError
- **Solution**: Ensure `.so` files are in the correct `jniLibs` directory structure

**Problem**: Kotlin bindings not found
- **Solution**: Verify package structure matches `uniffi.paykit_mobile.*`

**Problem**: EncryptedSharedPreferences not available
- **Solution**: Add dependency: `implementation "androidx.security:security-crypto:1.1.0-alpha06"`

### General Issues

**Problem**: Bindings generation fails
- **Solution**: Ensure UniFFI CLI is installed: `cargo install uniffi-bindgen-cli@0.25`

**Problem**: Runtime errors
- **Solution**: Check that all required dependencies are linked and native libraries are accessible

## Next Steps

- Review the [API Reference](../target/doc/paykit_mobile/index.html)
- See [Examples](../paykit-lib/examples/) for complete code samples
- Check [BIP Compliance](./bip-compliance.md) for specification details
