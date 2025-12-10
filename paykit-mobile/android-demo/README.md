# Paykit Android Demo

A comprehensive Android demo application showcasing all Paykit features including auto-pay, subscriptions, and payment requests.

## Features

### Payment Methods
- View available payment methods (Lightning, On-Chain)
- Health status monitoring for each method
- Endpoint validation
- Smart method selection with customizable strategies

### Subscriptions
- Create and manage subscriptions
- Proration calculator for upgrades/downgrades
- Multiple billing frequencies (daily, weekly, monthly)
- Active subscription tracking

### Auto-Pay
- Enable/disable auto-pay globally
- Set global daily spending limits
- Per-peer spending limits with usage tracking
- Custom auto-pay rules with conditions
- Recent auto-payment history
- Visual spending limit progress bars

### Payment Requests
- Create payment requests with optional expiry
- Accept/decline incoming requests
- Request history with status tracking
- Support for Lightning and On-Chain

### Settings
- Network selection (Mainnet/Testnet/Regtest)
- Security settings (biometric, background lock)
- Notification preferences
- Developer options for testing

## Project Structure

```
android-demo/
├── app/
│   ├── build.gradle.kts        # App build configuration
│   └── src/main/
│       ├── AndroidManifest.xml
│       └── java/com/paykit/demo/
│           ├── PaykitDemoApp.kt      # Application class
│           ├── MainActivity.kt       # Main activity with navigation
│           ├── ui/
│           │   ├── AutoPayScreen.kt      # Auto-pay settings UI
│           │   ├── PaymentMethodsScreen.kt # Methods UI
│           │   ├── SubscriptionsScreen.kt  # Subscriptions UI
│           │   ├── PaymentRequestsScreen.kt # Requests UI
│           │   ├── SettingsScreen.kt       # Settings UI
│           │   └── theme/
│           │       └── Theme.kt          # Material 3 theme
│           └── viewmodel/
│               └── AutoPayViewModel.kt   # Auto-pay logic
├── build.gradle.kts            # Root build configuration
└── settings.gradle.kts         # Project settings
```

## Requirements

- Android SDK 26+ (Android 8.0 Oreo)
- Kotlin 1.9+
- Android Studio Hedgehog (2023.1.1) or later

## Dependencies

The demo uses:
- **Jetpack Compose** - Modern UI toolkit
- **Material 3** - Latest Material Design
- **Navigation Compose** - Type-safe navigation
- **EncryptedSharedPreferences** - Secure storage
- **Kotlin Coroutines** - Async operations

## Setup

### 1. Copy Storage Implementations

Copy the Kotlin storage files from `paykit-mobile/kotlin/` to your project:

```
kotlin/
├── EncryptedPreferencesStorage.kt  # Base secure storage
├── AutoPayStorage.kt               # Auto-pay specific storage
└── PrivateEndpointStorage.kt       # Private endpoint storage
```

### 2. Build Paykit Mobile Library

Build the Paykit mobile library for Android:

```bash
cd paykit-mobile

# Build for Android targets
cargo ndk -t armeabi-v7a -t arm64-v8a -t x86_64 -o android-demo/app/src/main/jniLibs build --release

# Generate Kotlin bindings
uniffi-bindgen generate \
    --library target/release/libpaykit_mobile.so \
    -l kotlin \
    -o kotlin
```

### 3. Add Paykit Bindings

Copy the generated bindings to your project:

```
app/src/main/java/uniffi/paykit_mobile/paykit_mobile.kt
```

### 4. Configure build.gradle.kts

Ensure the native libraries are included:

```kotlin
android {
    sourceSets {
        main {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }
}
```

### 5. Build and Run

```bash
./gradlew assembleDebug
./gradlew installDebug
```

## Auto-Pay Flow

The auto-pay system works as follows:

```
Payment Request Received
         │
         ▼
   Is Auto-Pay Enabled?
         │
    ┌────┴────┐
    No        Yes
    │          │
    ▼          ▼
  Needs    Check Global
  Manual   Daily Limit
  Approval      │
           ┌────┴────┐
         Exceeded  Within Limit
           │          │
           ▼          ▼
         Deny    Check Peer Limit
                      │
                 ┌────┴────┐
               Exceeded  Within Limit
                 │          │
                 ▼          ▼
               Deny    Check Rules
                            │
                      ┌─────┴─────┐
                  No Match    Match Found
                      │          │
                      ▼          ▼
                   Needs    Auto-Approve
                   Manual
                   Approval
```

## Security Features

### Encrypted Storage
- All sensitive data stored using `EncryptedSharedPreferences`
- AES-256-GCM encryption for values
- AES-256-SIV encryption for keys
- Hardware-backed keystore when available

### Biometric Protection
- Optional biometric requirement for payments
- StrongBox support on compatible devices
- Configurable threshold for biometric prompts

### Spending Limits
- Global daily limits with automatic reset
- Per-peer limits with configurable periods
- Real-time usage tracking

## Testing

The demo includes mock data for testing without real payments:
- Sample peer spending limits
- Sample auto-pay rules
- Sample recent payments

For real testing, enable testnet mode in Settings.

## License

MIT
