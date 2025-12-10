# Paykit Mobile

Mobile FFI bindings for Paykit, enabling integration with iOS (Swift) and Android (Kotlin) applications.

## Overview

This crate provides UniFFI-based bindings that expose Paykit functionality to mobile platforms:

- **Payment Method Selection**: Automatic selection of optimal payment methods
- **Subscription Management**: Create, modify, and manage subscriptions
- **Payment Requests**: Create and track payment requests
- **Receipts**: Generate and manage payment receipts
- **Health Monitoring**: Check payment method availability
- **Proration**: Calculate prorated billing for subscription changes

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

# Generate Swift bindings
uniffi-bindgen generate --library target/release/libpaykit_mobile.dylib -l swift -o swift

# Generate Kotlin bindings
uniffi-bindgen generate --library target/release/libpaykit_mobile.dylib -l kotlin -o kotlin
```

## iOS Integration

### 1. Add the Generated Files

Copy the generated files to your Xcode project:
- `swift/paykit_mobile.swift` - Swift bindings
- Copy the compiled library (`.a` or `.dylib`)

### 2. Configure Build Settings

In Xcode, add the library to your target's "Link Binary with Libraries" phase.

### 3. Usage Example

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
    endpoint: "bc1q..."
)

// Select optimal method
let selection = try! client.selectMethod(
    supportedMethods: [
        PaymentMethod(methodId: "lightning", endpoint: "lnbc..."),
        PaymentMethod(methodId: "onchain", endpoint: "bc1q...")
    ],
    amountSats: 10000,
    preferences: nil
)
print("Selected: \(selection.primaryMethod)")

// Calculate proration for upgrade
let proration = try! client.calculateProration(
    currentAmountSats: 1000,
    newAmountSats: 2000,
    periodStart: Int64(Date().timeIntervalSince1970) - 86400 * 15,
    periodEnd: Int64(Date().timeIntervalSince1970) + 86400 * 15,
    changeDate: Int64(Date().timeIntervalSince1970)
)
print("Net charge: \(proration.netSats) sats")
```

## Android Integration

### 1. Add Dependencies

Add to your `build.gradle`:

```kotlin
dependencies {
    // AndroidX Security for EncryptedSharedPreferences
    implementation("androidx.security:security-crypto:1.1.0-alpha06")
    
    // Kotlin coroutines (if using async extensions)
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")
}
```

### 2. Add the Generated Files

Copy the generated files to your Android project:
- `kotlin/uniffi/paykit_mobile/paykit_mobile.kt` - Kotlin bindings
- `kotlin/EncryptedPreferencesStorage.kt` - Secure storage implementation
- Copy the compiled library (`.so` for your target architecture)

### 3. Configure build.gradle

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

### 4. Secure Storage Setup

```kotlin
import com.paykit.storage.EncryptedPreferencesStorage
import com.paykit.storage.asPaykitStorage

// Create secure storage (in Application.onCreate or similar)
val storage = EncryptedPreferencesStorage.create(
    context = applicationContext,
    fileName = "paykit_secure_prefs"
)

// For high-security apps, use biometric protection
val biometricStorage = EncryptedPreferencesStorage.createWithBiometrics(
    context = applicationContext,
    fileName = "paykit_biometric_prefs"
)

// Use with Paykit
val paykitStorage = storage.asPaykitStorage()
```

### 5. Usage Example

```kotlin
import uniffi.paykit_mobile.*
import com.paykit.storage.EncryptedPreferencesStorage
import com.paykit.storage.asPaykitStorage

// Create secure storage
val storage = EncryptedPreferencesStorage.create(context)
val paykitStorage = storage.asPaykitStorage()

// Create the client with secure storage
val client = PaykitClient(storage = paykitStorage)

// List available payment methods
val methods = client.listMethods()
println("Available methods: $methods")

// Validate an endpoint
val isValid = client.validateEndpoint(
    methodId = "onchain",
    endpoint = "bc1q..."
)

// Select optimal method
val selection = client.selectMethod(
    supportedMethods = listOf(
        PaymentMethod("lightning", "lnbc..."),
        PaymentMethod("onchain", "bc1q...")
    ),
    amountSats = 10000u,
    preferences = null
)
println("Selected: ${selection.primaryMethod}")

// Create a subscription
val subscription = client.createSubscription(
    subscriber = "pubky://subscriber...",
    provider = "pubky://provider...",
    terms = SubscriptionTerms(
        amountSats = 1000,
        currency = "SAT",
        frequency = PaymentFrequency.Monthly(dayOfMonth = 1u),
        methodId = "lightning",
        description = "Monthly subscription"
    )
)
println("Created subscription: ${subscription.subscriptionId}")

// Using coroutine extensions for async operations
lifecycleScope.launch {
    storage.storeAsync("private_key", keyBytes)
    val key = storage.retrieveAsync("private_key")
}
```

### Android Security Features

The `EncryptedPreferencesStorage` provides:

- **AES-256-GCM** encryption for values
- **AES-256-SIV** encryption for keys
- **Hardware-backed keystore** when available
- **StrongBox** support on compatible devices
- **Biometric authentication** option for sensitive data
- **Coroutine extensions** for async operations

## API Reference

### PaykitClient

The main entry point for all Paykit operations.

#### Methods

| Method | Description |
|--------|-------------|
| `listMethods()` | Get all registered payment methods |
| `validateEndpoint(methodId, endpoint)` | Validate an endpoint for a method |
| `selectMethod(methods, amount, prefs)` | Select optimal payment method |
| `checkHealth()` | Check health of all methods |
| `getHealthStatus(methodId)` | Get health of specific method |
| `isMethodUsable(methodId)` | Check if method is usable |
| `getPaymentStatus(receiptId)` | Get payment status |
| `getInProgressPayments()` | Get all pending payments |
| `createSubscription(...)` | Create a new subscription |
| `calculateProration(...)` | Calculate prorated billing |
| `daysRemainingInPeriod(periodEnd)` | Get days until period ends |
| `createPaymentRequest(...)` | Create a payment request |
| `createReceipt(...)` | Create a receipt |

### Types

- `PaymentMethod` - A method ID and endpoint pair
- `Amount` - Value and currency
- `SelectionPreferences` - Strategy and constraints
- `SubscriptionTerms` - Subscription configuration
- `PaymentFrequency` - Daily, Weekly, Monthly, Yearly, Custom
- `ProrationResult` - Credit, charge, and net amounts

## Async Operations

For long-running operations, use the async bridge module:

```swift
// Swift example with completion handler
client.fetchEndpointsAsync(pubkey: "...") { result in
    switch result {
    case .success(let endpoints):
        // Handle endpoints
    case .failure(let error):
        // Handle error
    }
}
```

## Thread Safety

All types are thread-safe. The `PaykitClient` manages its own Tokio runtime internally and can be used from any thread.

## License

MIT
