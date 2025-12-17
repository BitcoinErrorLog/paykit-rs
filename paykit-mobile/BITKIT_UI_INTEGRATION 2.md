# Bitkit UI Integration Guide

This guide explains how to integrate the ported UI components from the demo apps into Bitkit iOS and Android applications.

## Overview

Phase 2 provides reusable UI components for the core Paykit features:
- Dashboard - Overview with stats and recent activity
- Send Payment - Payment form with method selection
- Receive Payment - Server mode for receiving payments
- Contacts - Contact management
- Receipts - Payment history with search/filter
- Payment Methods - Method listing with health monitoring

## Architecture

All UI components follow a consistent pattern:

```
View (SwiftUI/Compose)
    ↓
ViewModel (Business Logic)
    ↓
Storage Protocol (Bitkit Implementation)
    ↓
PaykitClient (FFI)
```

## iOS Integration

### Component Location

All iOS components are in: `paykit-mobile/swift/BitkitUI/`

- `DashboardView.swift` - Dashboard screen
- `PaymentView.swift` - Send payment screen
- `ReceivePaymentView.swift` - Receive payment screen
- `ContactsView.swift` - Contacts management
- `ReceiptsView.swift` - Receipts history
- `PaymentMethodsView.swift` - Payment methods
- `NavigationExample.swift` - Navigation structure example

### Integration Steps

1. **Copy Components to Your Project**

```bash
# Copy BitkitUI directory to your Bitkit iOS project
cp -r paykit-mobile/swift/BitkitUI/ bitkit-ios/Sources/BitkitApp/UI/
```

2. **Implement Storage Protocols**

Create implementations of the storage protocols:

```swift
import PaykitMobile

class BitkitReceiptStorage: ReceiptStorageProtocol {
    func recentReceipts(limit: Int) -> [Receipt] {
        // Load from Bitkit's storage
    }
    
    func totalSent() -> UInt64 {
        // Calculate from stored receipts
    }
    
    func totalReceived() -> UInt64 {
        // Calculate from stored receipts
    }
    
    func pendingCount() -> Int {
        // Count pending receipts
    }
}

class BitkitContactStorage: ContactStorageProtocol {
    func listContacts() -> [Contact] {
        // Load from Bitkit's contact storage
    }
}
```

3. **Set Up Navigation**

Use the `NavigationExample.swift` as a template:

```swift
import SwiftUI
import PaykitMobile

@main
struct BitkitApp: App {
    var body: some Scene {
        WindowGroup {
            BitkitMainNavigationView()
        }
    }
}
```

4. **Apply Bitkit Styling**

Create a styling extension:

```swift
extension View {
    func bitkitStyle() -> some View {
        self
            .foregroundColor(.bitkitPrimary)
            .font(.bitkitBody)
            // Apply Bitkit design tokens
    }
}
```

## Android Integration

### Component Location

All Android components are in: `paykit-mobile/kotlin/BitkitUI/`

- `DashboardScreen.kt` - Dashboard screen
- `PaymentScreen.kt` - Send payment screen
- `ContactsScreen.kt` - Contacts management
- `ReceiptsScreen.kt` - Receipts history
- `PaymentMethodsScreen.kt` - Payment methods
- `NavigationExample.kt` - Navigation structure example

### Integration Steps

1. **Copy Components to Your Project**

```bash
# Copy BitkitUI directory to your Bitkit Android project
cp -r paykit-mobile/kotlin/BitkitUI/ bitkit-android/app/src/main/java/com/bitkit/ui/
```

2. **Implement Storage Protocols**

Create implementations:

```kotlin
class BitkitReceiptStorage: ReceiptStorageProtocol {
    override fun recentReceipts(limit: Int): List<Receipt> {
        // Load from Bitkit's storage
    }
    
    override fun totalSent(): Long {
        // Calculate from stored receipts
    }
    
    override fun totalReceived(): Long {
        // Calculate from stored receipts
    }
    
    override fun pendingCount(): Int {
        // Count pending receipts
    }
}
```

3. **Set Up Navigation**

Use `NavigationExample.kt` as a template:

```kotlin
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        val paykitClient = PaykitClient.new()
        val contactStorage = BitkitContactStorage()
        val receiptStorage = BitkitReceiptStorage()
        
        setContent {
            BitkitTheme {
                BitkitMainNavigation(
                    paykitClient = paykitClient,
                    contactStorage = contactStorage,
                    receiptStorage = receiptStorage
                )
            }
        }
    }
}
```

4. **Apply Bitkit Theme**

Create a Compose theme:

```kotlin
@Composable
fun BitkitTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = BitkitColorScheme,
        typography = BitkitTypography,
        content = content
    )
}
```

## Styling Integration

### Design Tokens

Bitkit should provide design tokens that components can use:

**iOS:**
- Colors: Primary, Secondary, Background, etc.
- Typography: Headline, Body, Caption styles
- Spacing: Consistent padding/margin values
- Corner Radius: Card and button radius values

**Android:**
- Material 3 Color Scheme
- Typography scale
- Shape system
- Spacing system

### Styling Approach

1. **Create Style Extensions/Modifiers**

**iOS:**
```swift
extension View {
    func bitkitCard() -> some View {
        self
            .padding()
            .background(Color.bitkitCardBackground)
            .cornerRadius(12)
    }
}
```

**Android:**
```kotlin
@Composable
fun BitkitCard(
    modifier: Modifier = Modifier,
    content: @Composable ColumnScope.() -> Unit
) {
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(
            containerColor = BitkitColorScheme.surface
        )
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            content = content
        )
    }
}
```

2. **Replace Default Styling**

Update components to use Bitkit styles:

- Replace `Color(.systemGray6)` with `Color.bitkitCardBackground`
- Replace default fonts with Bitkit typography
- Replace default spacing with Bitkit spacing tokens
- Replace default corner radius with Bitkit values

## Navigation Structure

### iOS (SwiftUI)

```swift
TabView {
    BitkitDashboardView(...)
        .tabItem { Label("Dashboard", systemImage: "house.fill") }
    
    BitkitPaymentView(...)
        .tabItem { Label("Send", systemImage: "arrow.up.circle.fill") }
    
    BitkitReceivePaymentView(...)
        .tabItem { Label("Receive", systemImage: "arrow.down.circle.fill") }
    
    BitkitContactsView(...)
        .tabItem { Label("Contacts", systemImage: "person.2.fill") }
    
    BitkitReceiptsView(...)
        .tabItem { Label("Receipts", systemImage: "doc.text.fill") }
}
```

### Android (Jetpack Compose)

```kotlin
Scaffold(
    bottomBar = {
        NavigationBar {
            // Dashboard, Send, Receive, Contacts, Receipts tabs
        }
    }
) { padding ->
    NavHost(
        navController = navController,
        startDestination = "dashboard"
    ) {
        composable("dashboard") { /* Dashboard */ }
        composable("send") { /* Send */ }
        composable("receive") { /* Receive */ }
        composable("contacts") { /* Contacts */ }
        composable("receipts") { /* Receipts */ }
    }
}
```

## Component Customization

### Dashboard

**Customizable Elements:**
- Stat card colors and icons
- Quick action buttons
- Recent activity row styling
- Quick access cards

**Bitkit Should:**
- Replace stat card colors with brand colors
- Customize icons to match design system
- Adjust spacing and layout to match app design

### Payment View

**Customizable Elements:**
- Form field styling
- Currency picker appearance
- Payment method selector
- Send button styling

**Bitkit Should:**
- Apply Bitkit form field styles
- Customize button appearance
- Add validation feedback styling

### Contacts View

**Customizable Elements:**
- Contact row layout
- Add contact sheet styling
- Empty state appearance
- Search bar styling

**Bitkit Should:**
- Match contact row to Bitkit contact design
- Customize empty states
- Apply Bitkit search bar styles

### Receipts View

**Customizable Elements:**
- Stat box styling
- Filter chip appearance
- Receipt row layout
- Empty state

**Bitkit Should:**
- Customize stat boxes to match dashboard
- Apply Bitkit filter styles
- Match receipt rows to transaction list design

### Payment Methods View

**Customizable Elements:**
- Method row layout
- Health indicator styling
- Empty state

**Bitkit Should:**
- Customize health indicators
- Match method rows to settings list design

## Data Flow

### Loading Data

```swift
// iOS
viewModel.loadDashboard(
    receiptStorage: bitkitReceiptStorage,
    contactStorage: bitkitContactStorage,
    autoPayStorage: bitkitAutoPayStorage
)
```

```kotlin
// Android
viewModel.loadDashboard(
    receiptStorage = bitkitReceiptStorage,
    contactStorage = bitkitContactStorage,
    autoPayStorage = bitkitAutoPayStorage
)
```

### Handling Actions

All components use callback closures/lambdas for navigation:

```swift
// iOS
BitkitDashboardView(
    viewModel: viewModel,
    onSendPayment: { /* Navigate to send */ },
    onReceivePayment: { /* Navigate to receive */ }
)
```

```kotlin
// Android
BitkitDashboardScreen(
    viewModel = viewModel,
    onSendPayment = { /* Navigate to send */ },
    onReceivePayment = { /* Navigate to receive */ }
)
```

## Testing

### Component Testing

Test each component independently:

```swift
// iOS
func testDashboardView() {
    let viewModel = BitkitDashboardViewModel(paykitClient: mockClient)
    let view = BitkitDashboardView(viewModel: viewModel)
    // Test view rendering
}
```

```kotlin
// Android
@Test
fun testDashboardScreen() {
    val viewModel = BitkitDashboardViewModel(mockClient)
    composeTestRule.setContent {
        BitkitDashboardScreen(viewModel = viewModel)
    }
    // Test UI elements
}
```

### Integration Testing

Test navigation flows:

1. Dashboard → Send Payment
2. Dashboard → Receive Payment
3. Dashboard → Contacts
4. Dashboard → Receipts
5. Send Payment → Receipts (after payment)

## Migration Checklist

- [ ] Copy UI components to Bitkit project
- [ ] Implement storage protocols
- [ ] Set up navigation structure
- [ ] Create Bitkit styling extensions/modifiers
- [ ] Replace default styling with Bitkit styles
- [ ] Test all navigation flows
- [ ] Verify data loading from Bitkit storage
- [ ] Test component interactions
- [ ] Apply Bitkit design tokens consistently
- [ ] Test on different screen sizes
- [ ] Verify accessibility

## Next Steps

After Phase 2 integration:
1. Port remaining screens (Subscriptions, Auto-Pay, Settings)
2. Add advanced features (QR Scanner, Contact Discovery)
3. Integrate with Bitkit's notification system
4. Add analytics and logging
5. Performance optimization

## Related Documentation

- [Bitkit Autopay Integration](./BITKIT_AUTOPAY_INTEGRATION.md)
- [Bitkit Integration Guide](./BITKIT_INTEGRATION_GUIDE.md)
- [Demo Apps README](../ios-demo/README.md)
