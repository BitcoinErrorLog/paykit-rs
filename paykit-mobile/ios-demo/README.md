# Paykit iOS Demo

A comprehensive iOS demo application showcasing Paykit features including key management, auto-pay, subscriptions, and payment requests.

## Current Status

> **Demo Application**: This is a demonstration app. Some features use real implementations while others use sample data for UI demonstration.

| Feature | Status | Notes |
|---------|--------|-------|
| Dashboard | **Real** | Overview with stats, recent activity, quick actions |
| Key Management | **Real** | Ed25519/X25519 via Rust FFI, Keychain storage |
| Key Backup/Restore | **Real** | Argon2 + AES-GCM encrypted exports |
| Contacts | **Real** | Keychain-backed contact storage |
| Receipts | **Real** | Payment history with search and filtering |
| Payment Methods | **Real** | Lists methods via PaykitClient FFI, validates endpoints |
| Health Monitoring | **Real** | Real health checks via PaykitClient.checkHealth() |
| Method Selection | **Real** | Smart method selection with strategy options |
| Subscriptions | **Real** | Keychain-backed subscription storage |
| Auto-Pay | **Real** | Keychain-backed settings, limits, and rules |
| Payment Requests | UI Only | Sample data, not persisted |
| Directory Operations | **Mock** | DirectoryService with mock transport (real Pubky integration pending) |
| Noise Payments | Not Implemented | Requires WebSocket/TCP transport |

## Features

### Dashboard (Real)

The dashboard provides an overview of your payment activity:

- **Stats Cards**: Total sent/received, contact count, pending transactions
- **Recent Activity**: Latest receipts with status indicators
- **Quick Actions**: Send, Receive, and Scan buttons
- **Pull to Refresh**: Update all statistics

### Receipts (Real)

Full payment history management with Keychain persistence:

- **Receipt List**: All payments with direction and status indicators
- **Search**: Find receipts by counterparty, memo, or key
- **Filtering**: Filter by direction (sent/received) or status
- **Detail View**: Full receipt details with transaction info
- **Statistics**: Total sent/received, completed/pending counts

### Key Management (Real)

The demo includes full cryptographic key management via Rust FFI:

- **Ed25519 Identity Keys**: Generate real pkarr-compatible identity keypairs
- **X25519 Device Keys**: Derive Noise protocol encryption keys via HKDF
- **Keychain Storage**: Secure storage using iOS Keychain Services
- **Encrypted Backup**: Export/import with password-protected encryption (Argon2 + AES-GCM)
- **z-base32 Format**: Public keys displayed in pkarr-compatible format

Key files:
- `KeyManager.swift` - High-level key management API
- `KeychainStorage.swift` - iOS Keychain wrapper
- `PaykitMobile.swift` - UniFFI generated bindings

### Payment Methods (Real)

Full payment method management via Rust FFI:

- **Method Listing**: Real-time list from `PaykitClient.listMethods()`
- **Health Monitoring**: Live health checks via `PaykitClient.checkHealth()`
- **Endpoint Validation**: Validate addresses/invoices via `PaykitClient.validateEndpoint()`
- **Smart Selection**: Strategy-based method selection (Balanced, Cost, Speed, Privacy)
- **Usability Check**: Verify method availability via `PaykitClient.isMethodUsable()`

### Subscriptions (Real)

Subscription management with Keychain persistence:

- **Create Subscriptions**: Set provider, amount, frequency, and method
- **Proration Calculator**: Calculate charges when upgrading/downgrading
- **Multiple Frequencies**: Daily, weekly, monthly, yearly billing
- **Active Tracking**: Toggle subscriptions active/paused
- **Keychain Storage**: Persistent subscription data via SubscriptionStorage

### Auto-Pay (Real)

Auto-pay settings management with Keychain persistence:

- **Enable/Disable**: Toggle auto-pay globally
- **Global Daily Limits**: Set spending caps per day
- **Per-Peer Limits**: Individual limits with usage tracking
- **Auto-Pay Rules**: Custom conditions for automatic approval
- **Recent Payments**: In-memory history (could be extended to Keychain)
- **Keychain Storage**: Settings, limits, and rules persist via AutoPayStorage

### Payment Requests (UI Demo)

- Create payment requests with optional expiry
- Accept/decline incoming requests
- Request history with status tracking

### Contacts (NEW - Real Implementation)
- Add and manage payment recipients
- Persistent storage using iOS Keychain
- Search contacts by name or public key
- Copy public keys to clipboard
- Payment history tracking per contact
- Notes and metadata support

### Settings

- Network selection (Mainnet/Testnet/Regtest)
- Security settings (Face ID, background lock)
- Notification preferences
- **Key management** (real implementation)
- Developer options for testing

## Project Structure

```
PaykitDemo/
├── PaykitDemoApp.swift          # App entry point and global state
├── KeyManager.swift             # Real key management (Rust FFI)
├── KeychainStorage.swift        # iOS Keychain wrapper
├── PaykitMobile.swift           # UniFFI generated Swift bindings
├── PaykitMobileFFI.h            # C header for FFI
├── Models/
│   ├── AutoPayModels.swift      # Auto-pay data models
│   ├── Contact.swift            # Contact data model
│   └── Receipt.swift            # Receipt data model
├── Storage/
│   ├── ContactStorage.swift     # Keychain-backed contact storage
│   └── ReceiptStorage.swift     # Keychain-backed receipt storage
├── ViewModels/
│   └── AutoPayViewModel.swift   # Auto-pay business logic (sample data)
└── Views/
    ├── ContentView.swift        # Main tab navigation
    ├── DashboardView.swift      # Dashboard with stats and activity
    ├── PaymentMethodsView.swift # Payment methods UI (static)
    ├── ContactsView.swift       # Contact management
    ├── ReceiptsView.swift       # Receipt history with filtering
    ├── SubscriptionsView.swift  # Subscriptions UI (sample data)
    ├── AutoPayView.swift        # Auto-pay settings UI (sample data)
    ├── PaymentRequestsView.swift # Payment requests UI (sample data)
    └── SettingsView.swift       # Settings with real key management
```

## Requirements

- iOS 16.0+
- Xcode 15.0+
- Swift 5.9+
- Rust toolchain (for building paykit-mobile)

## Setup

### 1. Build Paykit Mobile Library

```bash
cd paykit-rs-master/paykit-mobile

# Add iOS simulator target
rustup target add aarch64-apple-ios-sim

# Build for iOS Simulator (Apple Silicon)
cargo build --release --target aarch64-apple-ios-sim

# Generate Swift bindings
uniffi-bindgen generate \
    --library target/release/libpaykit_mobile.dylib \
    --language swift \
    --out-dir swift/generated
```

### 2. Open Xcode Project

```bash
cd ios-demo/PaykitDemo/PaykitDemo/PaykitDemo.xcodeproj
open .
```

### 3. Configure Build Settings

The project should be pre-configured, but verify:

1. **Library Search Paths**: Points to `target/aarch64-apple-ios-sim/release`
2. **Header Search Paths**: Includes project directory for FFI header
3. **Bridging Header**: Set to `PaykitDemo-Bridging-Header.h`

### 4. Build and Run

1. Select an iOS Simulator (e.g., iPhone 15 Pro)
2. Press Cmd+R to build and run

## Security Model

### Keychain Storage

All sensitive data is stored using iOS Keychain Services:

- **Access Control**: `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`
- **No iCloud Sync**: Keys stay on device
- **Encrypted at Rest**: Hardware-backed encryption

### Key Backup Security

Exported backups use:

- **Key Derivation**: Argon2id with random salt
- **Encryption**: AES-256-GCM
- **Integrity**: Authenticated encryption prevents tampering

### What's NOT Secure (Demo Limitations)

- Recent auto-payments stored in-memory only
- No biometric authentication enforced
- Sample data visible in production builds

## Auto-Pay Flow

The auto-pay system architecture (UI demonstration only):

```
Payment Request Received
         ↓
   Is Auto-Pay Enabled?
         ↓
   ┌─────┴─────┐
   No          Yes
   ↓            ↓
Needs      Check Global
Manual     Daily Limit
Approval        ↓
          ┌─────┴─────┐
        Exceeded    Within Limit
          ↓            ↓
        Deny      Check Peer Limit
                       ↓
                 ┌─────┴─────┐
               Exceeded    Within Limit
                 ↓            ↓
               Deny      Check Rules
                              ↓
                        ┌─────┴─────┐
                    No Match    Match Found
                        ↓            ↓
                    Needs      Auto-Approve
                    Manual
                    Approval
```

## Using KeyManager

```swift
import SwiftUI

// Initialize KeyManager
let keyManager = KeyManager()

// Check for existing identity
if keyManager.hasIdentity {
    print("Public Key: \(keyManager.publicKeyZ32)")
}

// Generate new identity
do {
    let keypair = try keyManager.generateNewIdentity()
    print("New key: \(keypair.publicKeyZ32)")
} catch {
    print("Error: \(error)")
}

// Export encrypted backup
do {
    let backup = try keyManager.exportBackup(password: "my-secure-password")
    let json = try keyManager.backupToString(backup)
    // Save json to file or share
} catch {
    print("Export failed: \(error)")
}

// Import from backup
do {
    let backup = try keyManager.backupFromString(jsonString)
    let keypair = try keyManager.importBackup(backup, password: "my-secure-password")
} catch {
    print("Import failed: \(error)")
}
```

## Testing

### Real Features

Test the key management features:
1. Go to Settings → Key Management
2. Generate a new keypair
3. Export with password
4. Delete and re-import

### Demo Features

The following use sample data for UI demonstration:
- Subscriptions: Shows sample subscriptions
- Auto-Pay: Shows sample rules and limits
- Payment Requests: Shows sample requests
- Directory Operations: Uses mock transport (not connected to real Pubky homeserver)

## Roadmap

Completed improvements:
- ✅ **Contacts**: Contact management with Keychain persistence
- ✅ **Dashboard**: Overview with statistics and recent activity
- ✅ **Receipts**: Payment history with search and filtering
- ✅ **Payment Methods**: Real FFI integration with PaykitClient
- ✅ **Health Monitoring**: Real health checks via PaykitClient.checkHealth()
- ✅ **Method Selection**: Smart method selection with strategy options

Planned improvements:
1. **Directory Lookup**: Fetch contacts from Pubky directory (replace mock transport)
2. **Payment Request Persistence**: Store payment requests in Keychain
3. **Noise Integration**: Real encrypted payments via Noise protocol

## Related Documentation

- [BUILD_AND_TEST.md](./BUILD_AND_TEST.md) - Detailed build instructions
- [QUICK_START.md](./QUICK_START.md) - Quick setup guide
- [paykit-mobile README](../README.md) - FFI library documentation

## License

MIT
