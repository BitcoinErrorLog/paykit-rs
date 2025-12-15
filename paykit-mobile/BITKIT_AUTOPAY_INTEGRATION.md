# Bitkit Autopay Integration Guide

This guide explains how to integrate payment request waking with autopay evaluation into Bitkit iOS and Android applications.

## Overview

The autopay integration allows Bitkit to:
1. Receive payment requests via deep links or push notifications
2. Automatically evaluate if the request meets autopay requirements
3. Execute payments automatically when criteria are met
4. Show manual approval UI when autopay is not applicable

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Payment Request Received                    │
│         (Deep Link / Push Notification)                 │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│         PaymentRequestService.handleIncomingRequest()   │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│         AutopayEvaluator.evaluate()                     │
│  • Check if autopay is enabled                          │
│  • Check global daily limit                             │
│  • Check peer-specific limit                            │
│  • Check auto-pay rules                                 │
└────────────────────┬────────────────────────────────────┘
                     │
         ┌───────────┴───────────┐
         │                       │
    Approved              Needs Approval / Denied
         │                       │
         ▼                       ▼
┌──────────────────┐   ┌──────────────────────┐
│ Execute Payment  │   │ Show Manual Approval │
│ Automatically    │   │ UI                   │
└──────────────────┘   └──────────────────────┘
```

## Phase 1 Implementation

### 1. Deep Link/URL Scheme Handlers

#### iOS Implementation

**Step 1: Register URL Scheme**

Add to your `Info.plist`:

```xml
<key>CFBundleURLTypes</key>
<array>
    <dict>
        <key>CFBundleURLSchemes</key>
        <array>
            <string>bitkit</string>
        </array>
        <key>CFBundleURLName</key>
        <string>com.bitkit.app</string>
    </dict>
</array>
```

**Step 2: Handle URL in AppDelegate/SceneDelegate**

```swift
import UIKit
import PaykitMobile

class AppDelegate: UIResponder, UIApplicationDelegate {
    
    func application(
        _ app: UIApplication,
        open url: URL,
        options: [UIApplication.OpenURLOptionsKey : Any] = [:]
    ) -> Bool {
        guard url.scheme == "bitkit" else { return false }
        
        if url.host == "payment-request" {
            handlePaymentRequest(url: url)
            return true
        }
        
        return false
    }
    
    private func handlePaymentRequest(url: URL) {
        guard let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
              let queryItems = components.queryItems else {
            return
        }
        
        let requestId = queryItems.first(where: { $0.name == "requestId" })?.value
        let fromPubkey = queryItems.first(where: { $0.name == "from" })?.value
        let amountSats = queryItems.first(where: { $0.name == "amount" })?.value.flatMap { Int64($0) }
        let methodId = queryItems.first(where: { $0.name == "method" })?.value
        
        guard let requestId = requestId, let fromPubkey = fromPubkey else {
            return
        }
        
        // Initialize services
        let paykitClient = PaykitClient.new() // or use your existing instance
        let autopayViewModel = AutoPayViewModel() // Your autopay view model
        let paymentRequestService = PaymentRequestService(
            paykitClient: paykitClient,
            autopayEvaluator: autopayViewModel
        )
        
        // Handle the request
        paymentRequestService.handleIncomingRequest(
            requestId: requestId,
            fromPubkey: fromPubkey
        ) { result in
            switch result {
            case .success(let processingResult):
                switch processingResult {
                case .autoPaid(let paymentResult):
                    // Payment was automatically executed
                    print("Auto-paid: \(paymentResult.receiptId ?? "unknown")")
                    // Show notification to user
                    
                case .needsApproval(let request):
                    // Show manual approval UI
                    DispatchQueue.main.async {
                        self.showPaymentApprovalUI(request: request)
                    }
                    
                case .denied(let reason):
                    // Show denial message
                    print("Payment denied: \(reason)")
                    
                case .error(let error):
                    // Handle error
                    print("Error: \(error)")
                }
                
            case .failure(let error):
                print("Failed to process request: \(error)")
            }
        }
    }
    
    private func showPaymentApprovalUI(request: PaymentRequest) {
        // Navigate to payment approval screen
        // This should be implemented in your app's navigation system
    }
}
```

**URL Format:**
```
bitkit://payment-request?requestId={id}&from={pubkey}&amount={sats}&method={methodId}
```

#### Android Implementation

**Step 1: Register Intent Filter**

Add to your `AndroidManifest.xml`:

```xml
<activity
    android:name=".MainActivity"
    android:exported="true">
    <intent-filter>
        <action android:name="android.intent.action.MAIN" />
        <category android:name="android.intent.category.LAUNCHER" />
    </intent-filter>
    
    <!-- Payment request handler -->
    <intent-filter>
        <action android:name="android.intent.action.VIEW" />
        <category android:name="android.intent.category.DEFAULT" />
        <category android:name="android.intent.category.BROWSABLE" />
        <data android:scheme="bitkit" android:host="payment-request" />
    </intent-filter>
</activity>
```

**Step 2: Handle Intent in MainActivity**

```kotlin
import android.content.Intent
import android.net.Uri
import androidx.lifecycle.lifecycleScope
import com.paykit.mobile.*
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        // Handle incoming intent
        handleIntent(intent)
    }
    
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleIntent(intent)
    }
    
    private fun handleIntent(intent: Intent?) {
        val data: Uri? = intent?.data
        if (data?.scheme == "bitkit" && data.host == "payment-request") {
            val requestId = data.getQueryParameter("requestId")
            val fromPubkey = data.getQueryParameter("from")
            val amountSats = data.getQueryParameter("amount")?.toLongOrNull()
            val methodId = data.getQueryParameter("method")
            
            if (requestId != null && fromPubkey != null) {
                handlePaymentRequest(requestId, fromPubkey)
            }
        }
    }
    
    private fun handlePaymentRequest(requestId: String, fromPubkey: String) {
        lifecycleScope.launch {
            // Initialize services
            val paykitClient = PaykitClient.new() // or use your existing instance
            val autopayViewModel = AutoPayViewModel(application) // Your autopay view model
            val paymentRequestService = PaymentRequestService(
                paykitClient = paykitClient,
                autopayEvaluator = autopayViewModel
            )
            
            // Handle the request
            paymentRequestService.handleIncomingRequest(requestId, fromPubkey)
                .onSuccess { result ->
                    when (result) {
                        is PaymentRequestProcessingResult.AutoPaid -> {
                            // Payment was automatically executed
                            println("Auto-paid: ${result.paymentResult.receiptId}")
                            // Show notification to user
                        }
                        is PaymentRequestProcessingResult.NeedsApproval -> {
                            // Show manual approval UI
                            showPaymentApprovalUI(result.request)
                        }
                        is PaymentRequestProcessingResult.Denied -> {
                            // Show denial message
                            println("Payment denied: ${result.reason}")
                        }
                        is PaymentRequestProcessingResult.Error -> {
                            // Handle error
                            println("Error: ${result.error}")
                        }
                    }
                }
                .onFailure { error ->
                    println("Failed to process request: $error")
                }
        }
    }
    
    private fun showPaymentApprovalUI(request: PaymentRequest) {
        // Navigate to payment approval screen
        // This should be implemented in your app's navigation system
    }
}
```

### 2. Payment Request Service Integration

The `PaymentRequestService` is provided in:
- **iOS**: `paykit-mobile/swift/PaymentRequestService.swift`
- **Android**: `paykit-mobile/kotlin/PaymentRequestService.kt`

**Key Methods:**

- `handleIncomingRequest(requestId:fromPubkey:completion:)` - Process an incoming payment request
- `evaluateAutopay(peerPubkey:amount:methodId:)` - Evaluate autopay requirements
- `executePayment(request:)` - Execute a payment request

**Important:** You must implement the `fetchPaymentRequest` method to retrieve full payment request details from your storage/network.

### 3. Autopay Evaluator Integration

The autopay evaluation logic is provided through the `AutopayEvaluator` protocol/interface.

**iOS:** The `AutoPayViewModel` from the demo app already conforms to `AutopayEvaluator`. You can use it directly or create your own implementation.

**Android:** Implement the `AutopayEvaluator` interface in your `AutoPayViewModel`:

```kotlin
class AutoPayViewModel(private val app: Application) : AndroidViewModel(app), AutopayEvaluator {
    
    override fun evaluate(peerPubkey: String, amount: Long, methodId: String): AutopayEvaluationResult {
        val result = shouldAutoApprove(peerPubkey, amount, methodId)
        
        return when (result) {
            is AutoApprovalResult.Approved -> {
                AutopayEvaluationResult.Approved(result.ruleId, result.ruleName)
            }
            is AutoApprovalResult.Denied -> {
                AutopayEvaluationResult.Denied(result.reason)
            }
            is AutoApprovalResult.NeedsApproval -> {
                AutopayEvaluationResult.NeedsApproval
            }
        }
    }
    
    // Your existing shouldAutoApprove method
    private fun shouldAutoApprove(peerPubkey: String, amount: Long, methodId: String): AutoApprovalResult {
        // Implementation from demo app
    }
}
```

### 4. Background Processing

#### iOS Background Processing

Use `BGTaskScheduler` for background processing:

```swift
import BackgroundTasks

class BackgroundPaymentProcessor {
    static let taskIdentifier = "com.bitkit.process-payment-requests"
    
    static func registerBackgroundTask() {
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: taskIdentifier,
            using: nil
        ) { task in
            self.handleBackgroundPaymentProcessing(task: task as! BGProcessingTask)
        }
    }
    
    static func scheduleBackgroundProcessing() {
        let request = BGProcessingTaskRequest(identifier: taskIdentifier)
        request.requiresNetworkConnectivity = true
        request.requiresExternalPower = false
        request.earliestBeginDate = Date(timeIntervalSinceNow: 1 * 60) // 1 minute
        
        try? BGTaskScheduler.shared.submit(request)
    }
    
    private static func handleBackgroundPaymentProcessing(task: BGProcessingTask) {
        // Process pending payment requests
        // This should fetch pending requests and process them
        
        task.expirationHandler = {
            task.setTaskCompleted(success: false)
        }
        
        // Process requests...
        task.setTaskCompleted(success: true)
    }
}
```

Register in `AppDelegate`:

```swift
func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
    BackgroundPaymentProcessor.registerBackgroundTask()
    return true
}
```

#### Android Background Processing

Use `WorkManager` for background processing:

```kotlin
import androidx.work.*

class PaymentRequestWorker(
    context: Context,
    params: WorkerParameters
) : CoroutineWorker(context, params) {
    
    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        try {
            // Process pending payment requests
            // This should fetch pending requests and process them
            
            Result.success()
        } catch (e: Exception) {
            Result.retry()
        }
    }
    
    companion object {
        fun enqueue(context: Context) {
            val request = PeriodicWorkRequestBuilder<PaymentRequestWorker>(
                15, TimeUnit.MINUTES
            ).build()
            
            WorkManager.getInstance(context).enqueue(request)
        }
    }
}
```

### 5. Implementing Payment Request Fetching

You must implement the `fetchPaymentRequest` method in `PaymentRequestService`. This should fetch payment request details from your storage or network.

**Example Implementation:**

```swift
// iOS
private func fetchPaymentRequest(requestId: String, fromPubkey: String) async throws -> PaymentRequest {
    // Option 1: Fetch from local storage
    if let stored = try? paymentRequestStorage.get(requestId: requestId) {
        return stored
    }
    
    // Option 2: Fetch from Paykit network
    // This would use PaykitClient or your transport layer
    // to fetch the request from the requester's directory
    
    throw PaymentRequestError.notFound
}
```

```kotlin
// Android
private suspend fun fetchPaymentRequest(requestId: String, fromPubkey: String): PaymentRequest {
    // Option 1: Fetch from local storage
    paymentRequestStorage.get(requestId)?.let { return it }
    
    // Option 2: Fetch from Paykit network
    // This would use PaykitClient or your transport layer
    // to fetch the request from the requester's directory
    
    throw IllegalStateException("Payment request not found")
}
```

## Testing

### Test Deep Link Handling

**iOS:**
```bash
xcrun simctl openurl booted "bitkit://payment-request?requestId=test123&from=pk_test&amount=1000&method=lightning"
```

**Android:**
```bash
adb shell am start -a android.intent.action.VIEW -d "bitkit://payment-request?requestId=test123&from=pk_test&amount=1000&method=lightning"
```

### Test Autopay Evaluation

Create test cases for:
- Autopay enabled/disabled
- Global daily limit checks
- Peer-specific limit checks
- Auto-pay rule matching
- Payment execution

## Next Steps

1. Implement payment request fetching from your storage/network
2. Integrate with your notification system (if using push notifications)
3. Create UI for manual payment approval
4. Add logging and analytics
5. Test end-to-end flows

## Related Documentation

- [Bitkit Integration Guide](./BITKIT_INTEGRATION_GUIDE.md)
- [Auto-Pay Models](../ios-demo/PaykitDemo/PaykitDemo/PaykitDemo/Models/AutoPayModels.swift)
- [AutoPayViewModel](../ios-demo/PaykitDemo/PaykitDemo/PaykitDemo/ViewModels/AutoPayViewModel.swift)
