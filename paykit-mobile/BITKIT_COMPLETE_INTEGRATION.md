# Bitkit Complete Integration Guide

This guide covers the complete integration of Paykit features into Bitkit iOS and Android applications, including all phases of the integration plan.

## Overview

This integration provides:
1. **Payment Request Waking with Autopay** - Automatic payment processing when requests arrive
2. **Complete UI Components** - All Paykit features exposed in Bitkit UI
3. **Service Integration** - Noise payments, Directory operations, Pubky Ring integration

## Phase 1: Payment Request Waking with Autopay ✅

### Components Provided

- **PaymentRequestService** (Swift/Kotlin) - Handles incoming payment requests
- **AutopayEvaluator Protocol** - Interface for autopay evaluation
- **BitkitAutoPayViewModel** - Complete autopay logic implementation

### Integration Steps

1. **Implement Deep Link Handlers** (see `BITKIT_AUTOPAY_INTEGRATION.md`)
2. **Implement Payment Request Fetching**
3. **Implement Endpoint Resolution**
4. **Connect to PaymentRequestService**

## Phase 2: Core UI Components ✅

### iOS Components (SwiftUI)

Located in `swift/BitkitUI/`:
- `DashboardView.swift`
- `PaymentView.swift`
- `ReceivePaymentView.swift`
- `ContactsView.swift`
- `ReceiptsView.swift`
- `PaymentMethodsView.swift`
- `NavigationExample.swift`

### Android Components (Jetpack Compose)

Located in `kotlin/BitkitUI/`:
- `DashboardScreen.kt`
- `PaymentScreen.kt`
- `ContactsScreen.kt`
- `ReceiptsScreen.kt`
- `PaymentMethodsScreen.kt`
- `NavigationExample.kt`

### Integration Steps

1. **Copy Components** to Bitkit project
2. **Implement Storage Protocols**
3. **Apply Bitkit Styling**
4. **Set Up Navigation**

See `BITKIT_UI_INTEGRATION.md` for detailed instructions.

## Phase 3: Advanced Features ✅

### iOS Components

- `SubscriptionsView.swift` - Subscription management
- `AutoPayView.swift` - Auto-pay settings UI
- `PaymentRequestsView.swift` - Payment request management
- `QRScannerView.swift` - QR code scanning
- `SettingsView.swift` - App settings
- `IdentityListView.swift` - Identity management

### Android Components

- `SubscriptionsScreen.kt` - Subscription management
- `AutoPayScreen.kt` - Auto-pay settings UI
- `PaymentRequestsScreen.kt` - Payment request management
- `QRScannerScreen.kt` - QR code scanning
- `SettingsScreen.kt` - App settings
- `IdentityListScreen.kt` - Identity management

### AutoPayViewModel Integration

**iOS**: `swift/BitkitUI/AutoPayViewModel.swift`
- Complete autopay logic
- Conforms to `AutopayEvaluator` protocol
- Uses `AutoPayStorageProtocol` for persistence

**Android**: `kotlin/BitkitUI/AutoPayViewModel.kt`
- Complete autopay logic
- Implements `AutopayEvaluator` interface
- Uses `AutoPayStorageProtocol` for persistence

## Phase 4: Service Integration

### Directory Service

**iOS**: `swift/BitkitUI/Services/DirectoryService.swift`
- Endpoint discovery
- Contact discovery
- Payment method discovery

**Integration**: Bitkit must implement methods using their Pubky SDK:
```swift
// Bitkit implementation example
func discoverNoiseEndpoint(recipientPubkey: String) async throws -> NoiseEndpointInfo? {
    // Use Bitkit's Pubky SDK to read directory
    let transport = BitkitPubkyTransport() // Your transport
    let endpoint = try await transport.readNoiseEndpoint(for: recipientPubkey)
    return endpoint
}
```

### Pubky Ring Integration

**iOS**: `swift/BitkitUI/Services/PubkyRingIntegration.swift`
- Key derivation requests
- URL scheme handling

**Integration**: Bitkit must implement URL scheme communication:
```swift
// In AppDelegate
func application(_ app: UIApplication, open url: URL, options: [UIApplication.OpenURLOptionsKey : Any] = [:]) -> Bool {
    if url.scheme == "bitkit" && url.host == "keypair-derived" {
        // Handle Pubky Ring callback
        BitkitPubkyRingIntegration.shared.handleCallback(url: url)
        return true
    }
    return false
}
```

### Noise Payment Service

**iOS**: `swift/BitkitUI/Services/NoisePaymentService.swift`
- Noise protocol payment coordination
- Server mode listening

**Integration**: Bitkit must implement:
1. Noise handshake using `FfiNoiseManager` from pubky-noise
2. Network connection handling
3. Message encryption/decryption

## Storage Protocol Implementation

Bitkit must implement all storage protocols:

### iOS Storage Protocols

```swift
class BitkitReceiptStorage: ReceiptStorageProtocol {
    func recentReceipts(limit: Int) -> [Receipt] {
        // Load from Bitkit storage
    }
    // ... implement other methods
}

class BitkitContactStorage: ContactStorageProtocol {
    func listContacts() -> [Contact] {
        // Load from Bitkit storage
    }
}

class BitkitAutoPayStorage: AutoPayStorageProtocol {
    func getSettings() -> AutoPaySettings {
        // Load from Bitkit storage
    }
    // ... implement other methods
}
```

### Android Storage Protocols

```kotlin
class BitkitReceiptStorage: ReceiptStorageProtocol {
    override fun recentReceipts(limit: Int): List<Receipt> {
        // Load from Bitkit storage
    }
    // ... implement other methods
}

class BitkitAutoPayStorage: AutoPayStorageProtocol {
    override fun getSettings(): AutoPaySettings {
        // Load from Bitkit storage
    }
    // ... implement other methods
}
```

## Complete Integration Checklist

### Phase 1: Autopay ✅
- [x] PaymentRequestService created
- [x] AutopayEvaluator protocol defined
- [x] BitkitAutoPayViewModel ported
- [ ] Deep link handlers implemented in Bitkit
- [ ] Payment request fetching implemented
- [ ] Endpoint resolution implemented
- [ ] Background processing implemented

### Phase 2: Core UI ✅
- [x] Dashboard component
- [x] Send/Receive components
- [x] Contacts component
- [x] Receipts component
- [x] Payment Methods component
- [ ] Storage protocols implemented
- [ ] Bitkit styling applied
- [ ] Navigation structure set up

### Phase 3: Advanced UI ✅
- [x] Subscriptions component
- [x] Auto-Pay settings component
- [x] Payment Requests component
- [x] QR Scanner component
- [x] Settings component
- [x] Identity Management component
- [ ] All components integrated into Bitkit app
- [ ] Bitkit styling applied consistently

### Phase 4: Services ✅
- [x] DirectoryService template created
- [x] PubkyRingIntegration template created
- [x] NoisePaymentService template created
- [ ] DirectoryService implemented with Bitkit Pubky SDK
- [ ] PubkyRingIntegration implemented with URL schemes
- [ ] NoisePaymentService implemented with pubky-noise
- [ ] End-to-end payment flows tested

## Styling Integration

### Design Tokens

Bitkit should provide:
- **Colors**: Primary, Secondary, Background, Error, Success, etc.
- **Typography**: Headline, Body, Caption styles
- **Spacing**: Consistent padding/margin values
- **Corner Radius**: Card and button radius values
- **Icons**: Icon set matching Bitkit design system

### iOS Styling

Create `BitkitStyle.swift`:
```swift
extension View {
    func bitkitCard() -> some View {
        self
            .padding()
            .background(Color.bitkitCardBackground)
            .cornerRadius(BitkitTokens.cornerRadius)
    }
    
    func bitkitButton() -> some View {
        self
            .foregroundColor(.bitkitButtonText)
            .background(Color.bitkitButtonBackground)
            .cornerRadius(BitkitTokens.buttonRadius)
    }
}
```

### Android Styling

Create `BitkitTheme.kt`:
```kotlin
@Composable
fun BitkitTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = BitkitColorScheme,
        typography = BitkitTypography,
        shapes = BitkitShapes,
        content = content
    )
}
```

## Testing Strategy

### Unit Tests

Test each component independently:
- ViewModels with mock storage
- Services with mock dependencies
- Storage implementations

### Integration Tests

Test complete flows:
1. Payment request → Autopay evaluation → Payment execution
2. Dashboard → Send Payment → Receipts
3. Contacts → Add Contact → Send Payment
4. QR Scanner → Parse → Navigate to appropriate flow

### E2E Tests

Test end-to-end scenarios:
- Create subscription → Receive payment → Auto-pay executes
- Scan QR code → Send payment → Receipt generated
- Switch identity → Data isolation verified

## Performance Considerations

1. **Lazy Loading**: Load data on-demand, not all at once
2. **Caching**: Cache frequently accessed data (contacts, methods)
3. **Background Processing**: Use background tasks for payment request processing
4. **Memory Management**: Properly dispose of resources in ViewModels

## Security Considerations

1. **Key Storage**: Never store Ed25519 seeds in Paykit app
2. **Secure Storage**: Use platform secure storage (Keychain/KeyStore)
3. **Identity Isolation**: Ensure data is properly scoped per identity
4. **Error Handling**: Don't leak sensitive information in error messages

## Migration Path

1. **Start with Phase 1**: Implement autopay first (smallest scope)
2. **Add Core UI**: Port Dashboard, Send, Receive (most used features)
3. **Add Advanced UI**: Port remaining screens as needed
4. **Integrate Services**: Add Noise, Directory, PubkyRing last

## Support

- **Documentation**: See individual integration guides
- **Examples**: Demo apps show complete implementations
- **Issues**: File at https://github.com/synonymdev/paykit-rs/issues

## Related Documentation

- [Bitkit Autopay Integration](./BITKIT_AUTOPAY_INTEGRATION.md)
- [Bitkit UI Integration](./BITKIT_UI_INTEGRATION.md)
- [Bitkit Integration Guide](./BITKIT_INTEGRATION_GUIDE.md)
- [Pubky Ring Integration](./PUBKY_RING_INTEGRATION.md)
