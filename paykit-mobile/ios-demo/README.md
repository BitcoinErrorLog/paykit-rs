# Paykit iOS Demo

A comprehensive iOS demo application showcasing Paykit features including key management, auto-pay, subscriptions, and payment requests.

## Current Status

> **Demo Application**: This is a demonstration app. Some features use real implementations while others use sample data for UI demonstration.

| Feature | Status | Notes |
|---------|--------|-------|
| Dashboard | **Real** | Overview with stats, recent activity, quick actions |
| Key Management | **Real** | Ed25519/X25519 via Rust FFI, Keychain storage |
| Key Backup/Restore | **Real** | Argon2 + AES-GCM encrypted exports |
| Contacts | **Real** | Keychain-backed contact storage, identity-scoped |
| Contact Discovery | **Real** | Discover contacts from Pubky follows directory |
| Receipts | **Real** | FFI-backed creation, Keychain storage, search/filtering, identity-scoped |
| Payment Methods | **Real** | Lists methods via PaykitClient FFI, validates endpoints |
| Health Monitoring | **Real** | Real health checks via PaykitClient.checkHealth() |
| Method Selection | **Real** | Smart method selection with strategy options |
| Subscriptions | **Real** | Keychain-backed subscription storage, identity-scoped |
| Auto-Pay | **Real** | Keychain-backed settings, limits, and rules, identity-scoped |
| Payment Requests | **Real** | Keychain-backed storage with FFI integration, identity-scoped |
| QR Scanner | **Real** | AVFoundation-based QR code scanning with Paykit URI parsing |
| Multiple Identities | **Real** | Create, switch, and manage multiple identities |
| Directory Operations | **Configurable** | DirectoryService supports mock or callback-based Pubky transport |
| Noise Payments | **Real** | Send/receive payments over encrypted Noise protocol channels |

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

### Contacts (Real Implementation)
- Add and manage payment recipients
- Persistent storage using iOS Keychain (identity-scoped)
- Search contacts by name or public key
- Copy public keys to clipboard
- Payment history tracking per contact
- Notes and metadata support
- **Contact Discovery**: Discover contacts from Pubky follows directory
  - Fetch known contacts from any public key's follows list
  - View supported payment methods for discovered contacts
  - Import selected contacts with one tap
  - Automatically filters out contacts you already have

### Noise Payments (Real Implementation)

Send and receive payments over encrypted Noise protocol channels:

- **Key Architecture**: "Cold pkarr, hot noise" - Ed25519 keys managed by Pubky Ring, X25519 keys cached locally
- **Secure Channel**: Full Noise_IK handshake over TCP
- **Receipt Exchange**: Cryptographic receipts for payment proof
- **Server Mode**: Receive payments by listening for incoming connections
- **Private Endpoints**: Fresh payment addresses exchanged over encrypted channel

Key files:
- `Services/NoisePaymentService.swift` - Core Noise payment coordination
- `Services/NoiseKeyCache.swift` - Secure X25519 key caching
- `Services/PubkyRingIntegration.swift` - Integration with remote key manager
- `Services/MockPubkyRingService.swift` - Demo/testing key derivation
- `Services/DirectoryService.swift` - Endpoint discovery and publishing
- `ViewModels/NoisePaymentViewModel.swift` - Payment flow state management
- `Views/ReceivePaymentView.swift` - Server mode UI

#### Sending Payments

1. Navigate to Send tab
2. Enter recipient (pubky:// URI or contact name)
3. Enter amount and select payment method
4. Tap "Send Payment"
5. App discovers recipient's Noise endpoint
6. Establishes encrypted connection
7. Exchanges payment request and receipt

#### Receiving Payments

1. Navigate to Receive tab
2. Tap "Start Listening"
3. Share connection info (QR code or copy)
4. Accept incoming payment requests
5. Receipts stored automatically

### Directory Operations (Configurable Transport)

The DirectoryService supports both mock and real Pubky transport:

```swift
// Development/Testing mode (default)
let service = DirectoryService(mode: .mock)

// Production mode with real Pubky SDK
let pubkyCallback = MyPubkyStorageCallback(pubkyClient: myPubkyClient)
let service = DirectoryService(mode: .callback(pubkyCallback))

// Fetch contacts and payment endpoints
let contacts = try await service.fetchKnownContacts(ownerPubkey: "pk...")
let endpoint = try await service.fetchPaymentEndpoint(ownerPubkey: "pk...", methodId: "lightning")
let methods = try await service.fetchSupportedPayments(ownerPubkey: "pk...")
```

To enable real Pubky integration, implement `PubkyUnauthenticatedStorageCallback`:

```swift
class MyPubkyStorage: PubkyUnauthenticatedStorageCallback {
    let pubkyClient: PubkyClient
    
    func get(ownerPubkey: String, path: String) -> StorageGetResult {
        // Implement using your Pubky SDK
    }
    
    func list(ownerPubkey: String, prefix: String) -> StorageListResult {
        // Implement using your Pubky SDK
    }
}
```

### QR Scanner (Real Implementation)

- **Camera Integration**: AVFoundation-based QR code scanning
- **Paykit URI Parsing**: Automatically detects and parses Paykit URIs
- **Multiple URI Types**: Supports Pubky, Invoice, and Payment Request URIs
- **Result Handling**: Shows parsed information and allows navigation to appropriate flows
- **Permission Handling**: Requests camera permission at runtime
- Accessible from Dashboard quick actions

### Multiple Identities (Real Implementation)

- **Identity Management**: Create, list, switch, and delete identities
- **Identity-Scoped Storage**: All data (contacts, receipts, subscriptions, etc.) is isolated per identity
- **Automatic Migration**: Existing single-identity users are automatically migrated
- **Identity Switching**: Seamlessly switch between identities with automatic data reloading
- **Identity List View**: Full UI for managing all identities
- Accessible from Settings → Manage Identities

### Settings

- Network selection (Mainnet/Testnet/Regtest)
- Security settings (Face ID, background lock)
- Notification preferences
- **Key management** (real implementation)
- **Identity management** (create, switch, delete identities)
- Developer options for testing

## Project Structure

```
PaykitDemo/
├── PaykitDemoApp.swift          # App entry point and global state
├── KeyManager.swift             # Real key management (Rust FFI)
├── KeychainStorage.swift        # iOS Keychain wrapper
├── PaykitMobile.swift           # UniFFI generated Swift bindings
├── PaykitMobileFFI.h            # C header for FFI
├── PubkyNoise.swift             # Noise protocol FFI bindings
├── PubkyNoiseFFI.h              # Noise FFI header
├── Models/
│   ├── AutoPayModels.swift      # Auto-pay data models
│   ├── Contact.swift            # Contact data model
│   └── Receipt.swift            # Receipt data model
├── Services/
│   ├── NoisePaymentService.swift    # Core Noise payment coordination
│   ├── NoiseKeyCache.swift          # X25519 key caching
│   ├── PubkyRingIntegration.swift   # Remote key manager integration
│   ├── MockPubkyRingService.swift   # Demo/testing key derivation
│   └── DirectoryService.swift       # Endpoint discovery/publishing
├── Storage/
│   ├── ContactStorage.swift     # Keychain-backed contact storage
│   └── ReceiptStorage.swift     # Keychain-backed receipt storage
├── ViewModels/
│   ├── AutoPayViewModel.swift   # Auto-pay business logic
│   └── NoisePaymentViewModel.swift  # Payment flow state management
└── Views/
    ├── ContentView.swift        # Main tab navigation (Send/Receive tabs)
    ├── DashboardView.swift      # Dashboard with stats and activity
    ├── PaymentView.swift        # Send payment UI
    ├── ReceivePaymentView.swift # Receive payment UI (server mode)
    ├── PaymentMethodsView.swift # Payment methods UI
    ├── ContactsView.swift       # Contact management
    ├── ReceiptsView.swift       # Receipt history with filtering
    ├── SubscriptionsView.swift  # Subscriptions UI
    ├── AutoPayView.swift        # Auto-pay settings UI
    ├── PaymentRequestsView.swift # Payment requests UI
    └── SettingsView.swift       # Settings with key management
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
- Directory Operations: Uses mock transport by default (configurable for real Pubky integration)

**Real Features**:
- Payment Requests: Persisted to Keychain, created via PaykitClient FFI

## Roadmap

Completed improvements:
- ✅ **Contacts**: Contact management with Keychain persistence
- ✅ **Contact Discovery**: Discover contacts from Pubky follows directory
- ✅ **QR Scanner**: AVFoundation-based QR code scanning with Paykit URI parsing
- ✅ **Multiple Identities**: Full identity management with identity-scoped storage
- ✅ **Dashboard**: Overview with statistics and recent activity
- ✅ **Receipts**: Payment history with search and filtering (identity-scoped)
- ✅ **Payment Methods**: Real FFI integration with PaykitClient
- ✅ **Health Monitoring**: Real health checks via PaykitClient.checkHealth()
- ✅ **Method Selection**: Smart method selection with strategy options
- ✅ **Directory Transport**: Configurable mock/callback transport for Pubky integration
- ✅ **Payment Request Persistence**: Store payment requests in Keychain with FFI integration (identity-scoped)
- ✅ **Receipt Generation**: Create receipts via FFI with Keychain storage (identity-scoped)
- ✅ **Subscription Storage**: Identity-scoped subscription management
- ✅ **Auto-Pay Storage**: Identity-scoped auto-pay settings and rules

Recent additions:
- ✅ **Noise Payments**: Send and receive payments over encrypted Noise protocol channels
- ✅ **Key Architecture**: "Cold pkarr, hot noise" key model with Pubky Ring integration
- ✅ **Server Mode**: Receive payments by listening for incoming connections
- ✅ **Private Endpoint Exchange**: Fresh payment addresses over encrypted channel

Planned improvements:
1. **Pubky SDK Integration**: Implement `PubkyUnauthenticatedStorageCallback` with real Pubky SDK
2. **Real Pubky Ring Integration**: Connect to actual Pubky Ring app for key management
3. **QR Scanner Navigation**: Add navigation flows for scanned QR codes (payment flows, contact addition, etc.)

## Related Documentation

- [BUILD_AND_TEST.md](./BUILD_AND_TEST.md) - Detailed build instructions
- [QUICK_START.md](./QUICK_START.md) - Quick setup guide
- [paykit-mobile README](../README.md) - FFI library documentation

## License

MIT
