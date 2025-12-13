# Paykit Android Demo

A comprehensive Android demo application showcasing Paykit features including key management, auto-pay, subscriptions, and payment requests.

## Current Status

> **Demo Application**: This is a demonstration app. Some features use real implementations while others use sample data for UI demonstration.

| Feature | Status | Notes |
|---------|--------|-------|
| Dashboard | **Real** | Overview with stats, recent activity, quick actions |
| Key Management | **Real** | Ed25519/X25519 via Rust FFI, EncryptedSharedPreferences |
| Key Backup/Restore | **Real** | Argon2 + AES-GCM encrypted exports |
| Contacts | **Real** | EncryptedSharedPreferences-backed contact storage |
| Receipts | **Real** | Payment history with search and filtering |
| Payment Methods | **Real** | Lists methods via PaykitClient FFI, validates endpoints |
| Health Monitoring | **Real** | Real health checks via PaykitClient.checkHealth() |
| Method Selection | **Real** | Smart method selection with strategy options |
| Subscriptions | **Real** | EncryptedSharedPreferences-backed subscription storage |
| Auto-Pay | **Real** | EncryptedSharedPreferences-backed settings, limits, and rules |
| Payment Requests | UI Only | Sample data, not persisted |
| Directory Operations | **Configurable** | DirectoryService supports mock or callback-based Pubky transport |
| Noise Payments | Not Implemented | Requires WebSocket/TCP transport |

## Features

### Dashboard (Real)

The dashboard provides an overview of your payment activity:

- **Stats Cards**: Total sent/received, contact count, pending transactions
- **Recent Activity**: Latest receipts with status indicators
- **Quick Actions**: Send, Receive, and Scan buttons
- **Material 3 Design**: Modern UI with card-based layout

### Receipts (Real)

Full payment history management with EncryptedSharedPreferences persistence:

- **Receipt List**: All payments with direction and status indicators
- **Search**: Find receipts by counterparty, memo, or key
- **Filter Sheet**: Filter by direction (sent/received) or status
- **Delete Support**: Remove individual receipts
- **Statistics**: Total sent/received, completed/pending counts

### Key Management (Real)

The demo includes full cryptographic key management via Rust FFI:

- **Ed25519 Identity Keys**: Generate real pkarr-compatible identity keypairs
- **X25519 Device Keys**: Derive Noise protocol encryption keys via HKDF
- **EncryptedSharedPreferences**: Secure storage with AES-256-GCM
- **Encrypted Backup**: Export/import with password-protected encryption (Argon2 + AES-GCM)
- **z-base32 Format**: Public keys displayed in pkarr-compatible format

Key files:
- `KeyManager.kt` - High-level key management API
- `paykit_mobile.kt` - UniFFI generated Kotlin bindings

### Payment Methods (Real)

Full payment method management via Rust FFI:

- **Method Listing**: Real-time list from `PaykitClient.listMethods()`
- **Health Monitoring**: Live health checks via `PaykitClient.checkHealth()`
- **Endpoint Validation**: Validate addresses/invoices via `PaykitClient.validateEndpoint()`
- **Smart Selection**: Strategy-based method selection (Balanced, Cost, Speed, Privacy)
- **Usability Check**: Verify method availability via `PaykitClient.isMethodUsable()`

### Subscriptions (Real)

Subscription management with EncryptedSharedPreferences persistence:

- **Create Subscriptions**: Set provider, amount, frequency, and method
- **Proration Calculator**: Calculate charges when upgrading/downgrading
- **Multiple Frequencies**: Daily, weekly, monthly billing
- **Active Tracking**: Toggle subscriptions active/paused
- **Secure Storage**: Persistent subscription data via SubscriptionStorage

### Auto-Pay (Real)

Auto-pay settings management with EncryptedSharedPreferences persistence:

- **Enable/Disable**: Toggle auto-pay globally
- **Global Daily Limits**: Set spending caps per day
- **Per-Peer Limits**: Individual limits with usage tracking
- **Auto-Pay Rules**: Custom conditions for automatic approval
- **Secure Storage**: Settings, limits, and rules persist via AutoPayStorage

### Payment Requests (UI Demo)

- Create payment requests with optional expiry
- Accept/decline incoming requests
- Request history with status tracking

### Contacts (NEW - Real Implementation)
- Add and manage payment recipients
- Persistent storage using EncryptedSharedPreferences
- Search contacts by name or public key
- Copy public keys to clipboard
- Payment history tracking per contact
- Notes and metadata support

### Directory Operations (Configurable Transport)

The DirectoryService supports both mock and real Pubky transport:

```kotlin
// Development/Testing mode (default)
val service = DirectoryService(DirectoryTransportMode.Mock)

// Production mode with real Pubky SDK
val pubkyCallback = MyPubkyStorageCallback(pubkyClient)
val service = DirectoryService(DirectoryTransportMode.Callback(pubkyCallback))

// Fetch contacts and payment endpoints
val contacts = service.fetchKnownContacts("pk...")
val endpoint = service.fetchPaymentEndpoint("pk...", "lightning")
val methods = service.fetchSupportedPayments("pk...")
```

To enable real Pubky integration, implement `PubkyUnauthenticatedStorageCallback`:

```kotlin
class MyPubkyStorage(
    private val pubkyClient: PubkyClient
) : PubkyUnauthenticatedStorageCallback {
    
    override fun get(ownerPubkey: String, path: String): StorageGetResult {
        return try {
            val content = pubkyClient.publicGet(ownerPubkey, path)
            StorageGetResult(success = true, content = content, error = null)
        } catch (e: Exception) {
            StorageGetResult(success = false, content = null, error = e.message)
        }
    }
    
    override fun list(ownerPubkey: String, prefix: String): StorageListResult {
        return try {
            val entries = pubkyClient.publicList(ownerPubkey, prefix)
            StorageListResult(success = true, entries = entries, error = null)
        } catch (e: Exception) {
            StorageListResult(success = false, entries = emptyList(), error = e.message)
        }
    }
}
```

### Settings

- Network selection (Mainnet/Testnet/Regtest)
- Security settings (biometric, background lock)
- Notification preferences
- **Key management** (real implementation)
- Developer options for testing

## Project Structure

```
android-demo/
├── app/
│   ├── build.gradle.kts          # App build configuration
│   └── src/main/
│       ├── AndroidManifest.xml
│       ├── jniLibs/              # Native libraries (.so files)
│       │   ├── arm64-v8a/
│       │   └── x86_64/
│       └── java/com/paykit/
│           ├── demo/
│           │   ├── PaykitDemoApp.kt      # Application class
│           │   ├── MainActivity.kt       # Main activity with navigation
│           │   ├── model/
│           │   │   ├── Contact.kt        # Contact data model
│           │   │   └── Receipt.kt        # Receipt data model
│           │   ├── storage/
│           │   │   ├── ContactStorage.kt # Encrypted contact storage
│           │   │   └── ReceiptStorage.kt # Encrypted receipt storage
│           │   ├── ui/
│           │   │   ├── DashboardScreen.kt    # Dashboard with stats
│           │   │   ├── AutoPayScreen.kt      # Auto-pay settings UI
│           │   │   ├── PaymentMethodsScreen.kt # Methods UI (static)
│           │   │   ├── ContactsScreen.kt     # Contact management
│           │   │   ├── ReceiptsScreen.kt     # Receipt history
│           │   │   ├── SubscriptionsScreen.kt  # Subscriptions UI
│           │   │   ├── PaymentRequestsScreen.kt # Requests UI
│           │   │   ├── SettingsScreen.kt       # Settings with key management
│           │   │   └── theme/
│           │   │       └── Theme.kt          # Material 3 theme
│           │   └── viewmodel/
│           │       └── AutoPayViewModel.kt   # Auto-pay logic (stub)
│           └── mobile/
│               ├── KeyManager.kt         # Real key management
│               └── paykit_mobile.kt      # UniFFI bindings
├── build.gradle.kts            # Root build configuration
└── settings.gradle.kts         # Project settings
```

## Requirements

- Android SDK 26+ (Android 8.0 Oreo)
- Kotlin 1.9+
- Android Studio Hedgehog (2023.1.1) or later
- Rust toolchain (for building paykit-mobile)

## Dependencies

The demo uses:
- **Jetpack Compose** - Modern UI toolkit
- **Material 3** - Latest Material Design
- **Navigation Compose** - Type-safe navigation
- **EncryptedSharedPreferences** - Secure storage (androidx.security:security-crypto)
- **JNA** - Java Native Access for UniFFI
- **Kotlin Coroutines** - Async operations

## Setup

### 1. Build Native Libraries

```bash
cd paykit-rs-master/paykit-mobile

# Install cargo-ndk if needed
cargo install cargo-ndk

# Build for Android targets
cargo ndk -t arm64-v8a -t x86_64 \
    -o android-demo/app/src/main/jniLibs \
    build --release

# Generate Kotlin bindings
uniffi-bindgen generate \
    --library target/release/libpaykit_mobile.dylib \
    --language kotlin \
    --out-dir kotlin/generated
```

### 2. Copy Bindings

```bash
# Copy generated bindings
cp kotlin/generated/com/paykit/mobile/paykit_mobile.kt \
   android-demo/app/src/main/java/com/paykit/mobile/

# Copy KeyManager
cp kotlin/KeyManager.kt \
   android-demo/app/src/main/java/com/paykit/mobile/
```

### 3. Patch Kotlin Bindings

The generated bindings have a known issue with `message` property conflicts. Run:

```bash
python3 << 'EOF'
import re
with open('android-demo/app/src/main/java/com/paykit/mobile/paykit_mobile.kt', 'r') as f:
    content = f.read()
content = re.sub(r'val `message`: String', 'val errorMessage: String', content)
content = re.sub(r'get\(\) = "message=\$\{ `message` \}"', 'get() = "message=${ errorMessage }"', content)
with open('android-demo/app/src/main/java/com/paykit/mobile/paykit_mobile.kt', 'w') as f:
    f.write(content)
print("Patched successfully")
EOF
```

### 4. Build and Run

```bash
cd android-demo
./gradlew assembleDebug
./gradlew installDebug
```

Or open in Android Studio and run.

## Security Model

### EncryptedSharedPreferences

All sensitive data is stored using Android's EncryptedSharedPreferences:

- **Key Encryption**: AES-256-SIV
- **Value Encryption**: AES-256-GCM
- **Master Key**: Hardware-backed keystore when available

### Key Backup Security

Exported backups use:

- **Key Derivation**: Argon2id with random salt
- **Encryption**: AES-256-GCM
- **Integrity**: Authenticated encryption prevents tampering

### What's NOT Secure (Demo Limitations)

- No biometric authentication enforced
- Sample data visible in production builds

## Using KeyManager

```kotlin
import com.paykit.mobile.KeyManager

class MyActivity : ComponentActivity() {
    private lateinit var keyManager: KeyManager
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        // Initialize KeyManager with context
        keyManager = KeyManager(applicationContext)
        
        // Check for existing identity
        if (keyManager.hasIdentity.value) {
            Log.d("Paykit", "Public Key: ${keyManager.publicKeyZ32.value}")
        }
        
        // Generate new identity
        try {
            val keypair = keyManager.generateNewIdentity()
            Log.d("Paykit", "New key: ${keypair.publicKeyZ32}")
        } catch (e: Exception) {
            Log.e("Paykit", "Error: ${e.message}")
        }
        
        // Export encrypted backup
        try {
            val backup = keyManager.exportBackup("my-secure-password")
            val json = keyManager.backupToString(backup)
            // Save json to file or share
        } catch (e: Exception) {
            Log.e("Paykit", "Export failed: ${e.message}")
        }
        
        // Import from backup
        try {
            val backup = keyManager.backupFromString(jsonString)
            val keypair = keyManager.importBackup(backup, "my-secure-password")
        } catch (e: Exception) {
            Log.e("Paykit", "Import failed: ${e.message}")
        }
    }
}
```

## Auto-Pay Flow

The auto-pay system architecture (UI demonstration only):

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

## Testing

### Real Features

Test the key management features:
1. Go to Settings → Manage Keys
2. Generate a new keypair (or one is created automatically)
3. Export with password
4. Import from backup

### Demo Features

The following use sample data for UI demonstration:
- Subscriptions: Empty state initially
- Auto-Pay: Basic toggle only
- Payment Requests: Sample data
- Directory Operations: Uses mock transport by default (configurable for real Pubky integration)

## Roadmap

Completed improvements:
- ✅ **Contacts**: Contact management with EncryptedSharedPreferences
- ✅ **Dashboard**: Overview with statistics and recent activity
- ✅ **Receipts**: Payment history with search and filtering
- ✅ **Payment Methods**: Real FFI integration with PaykitClient
- ✅ **Health Monitoring**: Real health checks via PaykitClient.checkHealth()
- ✅ **Method Selection**: Smart method selection with strategy options
- ✅ **Directory Transport**: Configurable mock/callback transport for Pubky integration

Planned improvements:
1. **Pubky SDK Integration**: Implement `PubkyUnauthenticatedStorageCallback` with real Pubky SDK
2. **Payment Request Persistence**: Store payment requests in EncryptedSharedPreferences
3. **Noise Integration**: Real encrypted payments via Noise protocol

## Troubleshooting

### Native Library Not Found

```
java.lang.UnsatisfiedLinkError: dlopen failed
```

Ensure native libraries are in the correct location:
```
app/src/main/jniLibs/
├── arm64-v8a/
│   └── libpaykit_mobile.so
└── x86_64/
    └── libpaykit_mobile.so
```

### JNA Issues

If you see JNA-related errors, ensure the dependency is correct:
```kotlin
implementation("net.java.dev.jna:jna:5.14.0@aar")
```

### Kotlin Binding Errors

If you see "Conflicting declarations" errors, re-run the patching script above.

## Related Documentation

- [paykit-mobile README](../README.md) - FFI library documentation
- [iOS Demo README](../ios-demo/README.md) - iOS equivalent

## License

MIT
