# Paykit iOS Demo

A comprehensive iOS demo application showcasing all Paykit features including auto-pay, subscriptions, and payment requests.

## Features

### Payment Methods
- View available payment methods (Lightning, On-Chain)
- Health status monitoring for each method
- Endpoint validation
- Smart method selection with customizable strategies

### Subscriptions
- Create and manage subscriptions
- Proration calculator for upgrades/downgrades
- Multiple billing frequencies (daily, weekly, monthly, yearly)
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
- Security settings (Face ID, background lock)
- Notification preferences
- Key management
- Developer options for testing

## Project Structure

```
PaykitDemo/
├── PaykitDemoApp.swift          # App entry point and global state
├── Models/
│   └── AutoPayModels.swift      # Auto-pay data models
├── ViewModels/
│   └── AutoPayViewModel.swift   # Auto-pay business logic
└── Views/
    ├── ContentView.swift        # Main tab navigation
    ├── PaymentMethodsView.swift # Payment methods UI
    ├── SubscriptionsView.swift  # Subscriptions UI
    ├── AutoPayView.swift        # Auto-pay settings UI
    ├── PaymentRequestsView.swift # Payment requests UI
    └── SettingsView.swift       # App settings UI
```

## Requirements

- iOS 16.0+
- Xcode 15.0+
- Swift 5.9+

## Setup

### 1. Generate Paykit Bindings

First, build the Paykit mobile library and generate Swift bindings:

```bash
cd paykit-mobile

# Build the library
cargo build --release

# Generate Swift bindings
uniffi-bindgen generate \
    --library target/release/libpaykit_mobile.dylib \
    -l swift \
    -o swift
```

### 2. Add Files to Xcode Project

1. Create a new iOS project in Xcode
2. Copy all files from `PaykitDemo/` to your project
3. Copy generated files:
   - `swift/paykit_mobile.swift`
   - `swift/paykit_mobileFFI.h`
   - `swift/KeychainStorage.swift`
4. Add the compiled library (`libpaykit_mobile.a` or `.dylib`)

### 3. Configure Build Settings

In your Xcode project:

1. Add library to "Link Binary with Libraries"
2. Set "Enable Modules" to YES
3. Add library search paths if needed

### 4. Run the Demo

Build and run on a simulator or device.

## Auto-Pay Flow

The auto-pay system works as follows:

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

## Spending Limits

### Global Limits
- Daily limit applies to all auto-payments combined
- Resets at midnight local time
- Visual progress bar shows usage

### Per-Peer Limits
- Set specific limits for individual peers
- Supports hourly, daily, weekly, monthly periods
- Automatically resets when period expires

### Auto-Pay Rules
- Define conditions for automatic approval
- Filter by maximum amount
- Filter by payment method
- Filter by specific peers

## Security Considerations

1. **Keychain Storage**: All sensitive data stored in iOS Keychain
2. **Biometric Auth**: Optional Face ID requirement for payments
3. **Spending Limits**: Multiple layers of protection
4. **Notifications**: Alert on all auto-payments and limit warnings

## Testing

The demo includes mock data for testing:
- Sample peer spending limits
- Sample auto-pay rules
- Sample recent payments

For real testing, configure testnet mode in Settings.

## License

MIT
