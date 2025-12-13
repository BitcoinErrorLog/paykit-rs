# Paykit iOS Demo

A comprehensive iOS demo application showcasing Paykit features including key management, auto-pay, subscriptions, and payment requests.

## Current Status

> **Demo Application**: This is a demonstration app. Some features use real implementations while others use sample data for UI demonstration.

| Feature | Status | Notes |
|---------|--------|-------|
| Key Management | **Real** | Ed25519/X25519 via Rust FFI, Keychain storage |
| Key Backup/Restore | **Real** | Argon2 + AES-GCM encrypted exports |
| Payment Methods | UI Only | Static list, not connected to PaykitClient |
| Health Monitoring | UI Only | Displays mock "Healthy" status |
| Subscriptions | UI Only | Sample data, not persisted |
| Auto-Pay | UI Only | Sample data, not persisted |
| Payment Requests | UI Only | Sample data, not persisted |
| Directory Operations | Not Implemented | Requires Pubky transport |
| Noise Payments | Not Implemented | Requires WebSocket/TCP transport |

## Features

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

### Payment Methods (UI Demo)

- View available payment methods (Lightning, On-Chain)
- Health status monitoring (mock data)
- Endpoint validation UI
- Smart method selection UI

### Subscriptions (UI Demo)

- Create and manage subscriptions (sample data)
- Proration calculator for upgrades/downgrades
- Multiple billing frequencies (daily, weekly, monthly, yearly)
- Active subscription tracking

### Auto-Pay (UI Demo)

- Enable/disable auto-pay globally
- Set global daily spending limits
- Per-peer spending limits with usage tracking
- Custom auto-pay rules with conditions
- Recent auto-payment history
- Visual spending limit progress bars

### Payment Requests (UI Demo)

- Create payment requests with optional expiry
- Accept/decline incoming requests
- Request history with status tracking

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
│   └── AutoPayModels.swift      # Auto-pay data models
├── ViewModels/
│   └── AutoPayViewModel.swift   # Auto-pay business logic (sample data)
└── Views/
    ├── ContentView.swift        # Main tab navigation
    ├── PaymentMethodsView.swift # Payment methods UI (static)
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

- Subscription/Auto-Pay data uses UserDefaults (not Keychain)
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
- Payment Methods: Shows 2 hardcoded methods
- Subscriptions: Shows sample subscriptions
- Auto-Pay: Shows sample rules and limits
- Payment Requests: Shows sample requests

## Roadmap

Planned improvements to make this a full production demo:

1. **Wire PaykitClient**: Connect Payment Methods UI to real FFI calls
2. **Persist Data**: Store subscriptions/auto-pay in Keychain
3. **Add Contacts**: Contact management with directory lookup
4. **Add Dashboard**: Overview with statistics
5. **Noise Integration**: Real encrypted payments

## Related Documentation

- [BUILD_AND_TEST.md](./BUILD_AND_TEST.md) - Detailed build instructions
- [QUICK_START.md](./QUICK_START.md) - Quick setup guide
- [paykit-mobile README](../README.md) - FFI library documentation

## License

MIT
