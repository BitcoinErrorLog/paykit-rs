# Paykit Mobile Demo - Integration Guide

This guide explains how to use the Paykit Mobile Demo as a reference for implementing Paykit in production mobile applications like Bitkit.

## Overview

The demo app now includes feature parity with Bitkit's Paykit integration:

- **Payment Executor Pattern**: Abstract protocol for Bitcoin/Lightning payments
- **Pubky Ring Authentication**: Same-device, cross-device (QR), and manual auth flows
- **Contact Discovery**: Health indicators and profile import from Pubky follows
- **Dashboard**: Stats cards, quick actions, and connection status
- **Deep Link Handling**: Full support for `paykit://` and `paykitdemo://` URIs
- **Background Processing**: Patterns for subscription checks and payment sync

## Architecture

```
┌─────────────────────────────────────────────────┐
│              Views (SwiftUI)                    │
│  - DashboardView, ContactDiscoveryView, etc.    │
└─────────────────┬───────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────┐
│              ViewModels                          │
│  - DashboardViewModel, ContactDiscoveryViewModel│
└─────────────────┬───────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────┐
│              Services                            │
│  - PaymentService, DirectoryService              │
│  - PubkyRingBridge, NoisePaymentService          │
└─────────────────┬───────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────┐
│              Executors                           │
│  - MockBitcoinExecutor, MockLightningExecutor    │
│  - (In production: Real wallet executors)        │
└─────────────────────────────────────────────────┘
```

## Key Components

### 1. Payment Executor Pattern

The executor pattern abstracts payment operations, allowing easy swapping between mock (demo) and real (production) implementations.

**Files:**
- `Executors/PaymentExecutorProtocol.swift` - Protocol definitions
- `Executors/MockBitcoinExecutor.swift` - Mock on-chain executor
- `Executors/MockLightningExecutor.swift` - Mock Lightning executor
- `Executors/PaymentService.swift` - High-level payment coordination

**Usage:**

```swift
// Demo app uses mock executors
let bitcoinExecutor = MockBitcoinExecutor(configuration: .default)
let lightningExecutor = MockLightningExecutor(configuration: .default)
let paymentService = PaymentService(
    bitcoinExecutor: bitcoinExecutor,
    lightningExecutor: lightningExecutor
)

// Production app would use real wallet executors
let bitcoinExecutor = BitkitBitcoinExecutor(lightningService: lightningService)
let lightningExecutor = BitkitLightningExecutor(lightningService: lightningService)
```

**Implementing for Production:**

```swift
public final class MyWalletBitcoinExecutor: BitcoinExecutorProtocol {
    private let wallet: MyBitcoinWallet
    
    public func sendToAddress(
        address: String,
        amountSats: UInt64,
        feeRate: Double?
    ) async throws -> BitcoinTxResult {
        let txid = try await wallet.send(to: address, amount: amountSats)
        return BitcoinTxResult(txid: txid, ...)
    }
    
    // ... other methods
}
```

### 2. Pubky Ring Authentication

Three authentication methods are supported:

1. **Same Device**: Direct URL scheme communication
2. **Cross Device (QR)**: Generate QR code, scan from device with Pubky Ring
3. **Manual Entry**: Enter pubkey and session secret manually

**Files:**
- `Services/PubkyRingBridge.swift` - Bridge for Pubky Ring communication
- `Views/PubkyRingAuthView.swift` - Authentication UI

**URL Schemes:**

```
// Request session from Pubky Ring
pubkyring://session?callback=paykitdemo://session

// Session callback
paykitdemo://session?pubky=z6mk...&session_secret=...

// Cross-device auth URL
https://pubky.app/auth?request_id=xxx&callback_scheme=paykitdemo
```

**Usage:**

```swift
// Check if Pubky Ring is installed
if PubkyRingBridge.shared.isPubkyRingInstalled {
    let session = try await bridge.requestSession()
} else {
    // Use cross-device flow
    let request = bridge.generateCrossDeviceRequest()
    // Display request.qrCodeImage
    let session = try await bridge.pollForCrossDeviceSession(requestId: request.requestId)
}
```

### 3. Contact Discovery with Health Indicators

Discover contacts from Pubky follows directory with payment method health status.

**Files:**
- `Views/ContactDiscoveryView.swift` - Discovery UI with filters and health
- `Models/DiscoveredContact` - Contact model with health tracking

**Features:**
- Search by name or pubkey
- Filter by payment method (Lightning, On-chain, Noise)
- Health indicators per endpoint
- Overall health status (All healthy, Partial, Unreachable)
- One-tap import to contacts

### 4. Dashboard with Connection Status

Enhanced dashboard showing:
- Pubky Ring connection status card
- Wallet balance breakdown (On-chain + Lightning)
- Quick access cards (Auto-Pay, Subscriptions, Requests, Discover)
- Setup checklist for new users
- Directory status
- Recent activity
- Quick action buttons (Send, Receive, Scan)

### 5. Deep Link Handling

Full support for navigation via URLs:

```
paykitdemo://dashboard          → Dashboard
paykitdemo://send?pubkey=z6mk.. → Send payment to pubkey
paykitdemo://send?amount=1000   → Send payment with amount
paykitdemo://receive            → Receive tab
paykitdemo://contacts           → Contacts tab
paykitdemo://contact?pubkey=... → Contact detail
paykitdemo://discover           → Contact discovery
paykitdemo://settings           → Settings tab
paykitdemo://profile            → Profile settings
paykitdemo://auth               → Pubky Ring auth
paykitdemo://receipt?id=...     → Receipt detail
```

**Implementation:**

```swift
.onOpenURL { url in
    handleDeepLink(url)
}

private func handleDeepLink(_ url: URL) {
    // Handle Pubky Ring callback first
    if PubkyRingBridge.shared.handleCallback(url: url) {
        return
    }
    
    // Parse and navigate
    switch url.host {
    case "send":
        navigationState.handleDeepLink(.send(pubkey: params["pubkey"], amount: params["amount"]))
    // ...
    }
}
```

### 6. Background Processing Patterns

Patterns for handling background tasks on iOS (BGTaskScheduler) and Android (WorkManager).

**Files:**
- `Services/BackgroundProcessing.swift` - iOS background task patterns

**iOS Setup:**

1. Add to Info.plist:
```xml
<key>BGTaskSchedulerPermittedIdentifiers</key>
<array>
    <string>com.paykit.demo.subscription.check</string>
    <string>com.paykit.demo.payment.sync</string>
    <string>com.paykit.demo.directory.refresh</string>
</array>
```

2. Register in AppDelegate:
```swift
BackgroundTaskManager.shared.registerBackgroundTasks()
```

3. Schedule when app enters background:
```swift
func applicationDidEnterBackground(_ application: UIApplication) {
    BackgroundTaskManager.shared.scheduleSubscriptionCheck()
    BackgroundTaskManager.shared.schedulePaymentSync()
}
```

**Android (WorkManager):**

```kotlin
@HiltWorker
class SubscriptionCheckWorker @AssistedInject constructor(
    @Assisted context: Context,
    @Assisted params: WorkerParameters,
    private val subscriptionRepo: SubscriptionRepository
) : CoroutineWorker(context, params) {

    override suspend fun doWork(): Result {
        // Process subscriptions
        return Result.success()
    }

    companion object {
        fun schedule(context: Context) {
            val request = PeriodicWorkRequestBuilder<SubscriptionCheckWorker>(
                15, TimeUnit.MINUTES
            )
                .setConstraints(Constraints.Builder()
                    .setRequiredNetworkType(NetworkType.CONNECTED)
                    .build())
                .build()

            WorkManager.getInstance(context)
                .enqueueUniquePeriodicWork("subscription_check", KEEP, request)
        }
    }
}
```

## Comparison with Bitkit

| Feature | Bitkit | Demo | Notes |
|---------|--------|------|-------|
| Payment Executors | Real LDK | Mock | Same protocol, swap implementations |
| Pubky Ring Auth | Full | Full | Same-device, QR, manual |
| Contact Discovery | Directory | Mock data | Same UI patterns |
| Health Indicators | Real checks | Simulated | Same visual patterns |
| Dashboard | Full | Simplified | Key components present |
| Deep Links | Full | Full | Same routing patterns |
| Background Tasks | Production | Patterns | Ready for production use |

## Migration to Production

1. **Replace Mock Executors**: Implement `BitcoinExecutorProtocol` and `LightningExecutorProtocol` with your wallet SDK

2. **Configure URL Schemes**: Update Info.plist with your app's URL scheme

3. **Enable Real Directory**: Replace mock `DirectoryService` with Pubky SDK integration

4. **Register Background Tasks**: Add task identifiers to Info.plist and implement in AppDelegate

5. **Configure Push Notifications**: Implement `PushPaymentHandler` patterns for incoming payment requests

## Testing

The demo includes mock implementations that simulate:
- Payment delays (configurable)
- Success/failure rates (configurable)
- Balance management
- Receipt generation

Configure mock behavior:

```swift
// Always succeed, instant
let config = MockLightningExecutor.Configuration(
    delayRange: 0...0,
    successRate: 1.0
)

// Realistic delays, occasional failures
let config = MockLightningExecutor.Configuration(
    delayRange: 0.5...2.0,
    successRate: 0.95
)
```

## File Structure

```
PaykitDemo/
├── Executors/
│   ├── PaymentExecutorProtocol.swift
│   ├── MockBitcoinExecutor.swift
│   ├── MockLightningExecutor.swift
│   └── PaymentService.swift
├── Services/
│   ├── PubkyRingBridge.swift
│   ├── BackgroundProcessing.swift
│   ├── DirectoryService.swift
│   └── NoisePaymentService.swift
├── Views/
│   ├── ContentView.swift (with deep link handling)
│   ├── DashboardView.swift (with connection card)
│   ├── ContactDiscoveryView.swift (with health)
│   ├── PubkyRingAuthView.swift (3 auth methods)
│   └── ...
├── ViewModels/
│   └── ...
├── Models/
│   └── ...
└── Storage/
    └── ...
```

