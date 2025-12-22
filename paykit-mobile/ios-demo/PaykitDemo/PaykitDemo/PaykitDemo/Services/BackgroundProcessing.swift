//
//  BackgroundProcessing.swift
//  PaykitDemo
//
//  Demonstrates background processing patterns for Paykit mobile apps.
//  This file shows how to handle subscription checks and payment processing
//  when the app is in the background.
//
//  iOS uses BGTaskScheduler for background tasks.
//  Android uses WorkManager (see Kotlin counterpart).
//

import Foundation
import BackgroundTasks

// MARK: - Background Task Identifiers

enum BackgroundTaskIdentifier {
    static let subscriptionCheck = "com.paykit.demo.subscription.check"
    static let paymentSync = "com.paykit.demo.payment.sync"
    static let directoryRefresh = "com.paykit.demo.directory.refresh"
}

// MARK: - Background Task Manager

/// Manages background task registration and execution for Paykit operations.
///
/// ## Usage in Production
///
/// 1. Add task identifiers to Info.plist:
/// ```xml
/// <key>BGTaskSchedulerPermittedIdentifiers</key>
/// <array>
///     <string>com.paykit.demo.subscription.check</string>
///     <string>com.paykit.demo.payment.sync</string>
///     <string>com.paykit.demo.directory.refresh</string>
/// </array>
/// ```
///
/// 2. Register tasks in AppDelegate:
/// ```swift
/// BGTaskScheduler.shared.register(
///     forTaskWithIdentifier: BackgroundTaskIdentifier.subscriptionCheck,
///     using: nil
/// ) { task in
///     BackgroundTaskManager.shared.handleSubscriptionCheck(task as! BGProcessingTask)
/// }
/// ```
///
/// 3. Schedule tasks when app enters background:
/// ```swift
/// func applicationDidEnterBackground(_ application: UIApplication) {
///     BackgroundTaskManager.shared.scheduleSubscriptionCheck()
/// }
/// ```
public final class BackgroundTaskManager {
    
    public static let shared = BackgroundTaskManager()
    
    private let keyManager = KeyManager()
    
    private init() {}
    
    // MARK: - Task Registration
    
    /// Register all background tasks with the system.
    /// Call this in `application(_:didFinishLaunchingWithOptions:)`.
    public func registerBackgroundTasks() {
        // Subscription check task (runs periodically)
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: BackgroundTaskIdentifier.subscriptionCheck,
            using: nil
        ) { [weak self] task in
            self?.handleSubscriptionCheck(task as! BGProcessingTask)
        }
        
        // Payment sync task
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: BackgroundTaskIdentifier.paymentSync,
            using: nil
        ) { [weak self] task in
            self?.handlePaymentSync(task as! BGProcessingTask)
        }
        
        // Directory refresh task
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: BackgroundTaskIdentifier.directoryRefresh,
            using: nil
        ) { [weak self] task in
            self?.handleDirectoryRefresh(task as! BGAppRefreshTask)
        }
        
        print("[BackgroundTaskManager] Registered background tasks")
    }
    
    // MARK: - Task Scheduling
    
    /// Schedule subscription check to run in background.
    /// Checks for due subscriptions and processes auto-pay.
    public func scheduleSubscriptionCheck() {
        let request = BGProcessingTaskRequest(identifier: BackgroundTaskIdentifier.subscriptionCheck)
        request.requiresNetworkConnectivity = true
        request.requiresExternalPower = false
        
        // Schedule to run in the next 15 minutes to 1 hour
        request.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60)
        
        do {
            try BGTaskScheduler.shared.submit(request)
            print("[BackgroundTaskManager] Scheduled subscription check")
        } catch {
            print("[BackgroundTaskManager] Failed to schedule subscription check: \(error)")
        }
    }
    
    /// Schedule payment sync to run in background.
    /// Syncs payment receipts and pending transactions.
    public func schedulePaymentSync() {
        let request = BGProcessingTaskRequest(identifier: BackgroundTaskIdentifier.paymentSync)
        request.requiresNetworkConnectivity = true
        request.requiresExternalPower = false
        request.earliestBeginDate = Date(timeIntervalSinceNow: 5 * 60)
        
        do {
            try BGTaskScheduler.shared.submit(request)
            print("[BackgroundTaskManager] Scheduled payment sync")
        } catch {
            print("[BackgroundTaskManager] Failed to schedule payment sync: \(error)")
        }
    }
    
    /// Schedule directory refresh to run periodically.
    /// Updates contact health status and discovers new contacts.
    public func scheduleDirectoryRefresh() {
        let request = BGAppRefreshTaskRequest(identifier: BackgroundTaskIdentifier.directoryRefresh)
        request.earliestBeginDate = Date(timeIntervalSinceNow: 60 * 60) // 1 hour
        
        do {
            try BGTaskScheduler.shared.submit(request)
            print("[BackgroundTaskManager] Scheduled directory refresh")
        } catch {
            print("[BackgroundTaskManager] Failed to schedule directory refresh: \(error)")
        }
    }
    
    // MARK: - Task Handlers
    
    /// Handle subscription check task.
    private func handleSubscriptionCheck(_ task: BGProcessingTask) {
        print("[BackgroundTaskManager] Starting subscription check")
        
        // Schedule next check
        scheduleSubscriptionCheck()
        
        // Create a task to process subscriptions
        let processingTask = Task {
            do {
                try await processSubscriptions()
                task.setTaskCompleted(success: true)
            } catch {
                print("[BackgroundTaskManager] Subscription check failed: \(error)")
                task.setTaskCompleted(success: false)
            }
        }
        
        // Handle cancellation
        task.expirationHandler = {
            processingTask.cancel()
        }
    }
    
    /// Handle payment sync task.
    private func handlePaymentSync(_ task: BGProcessingTask) {
        print("[BackgroundTaskManager] Starting payment sync")
        
        let processingTask = Task {
            do {
                try await syncPayments()
                task.setTaskCompleted(success: true)
            } catch {
                print("[BackgroundTaskManager] Payment sync failed: \(error)")
                task.setTaskCompleted(success: false)
            }
        }
        
        task.expirationHandler = {
            processingTask.cancel()
        }
    }
    
    /// Handle directory refresh task.
    private func handleDirectoryRefresh(_ task: BGAppRefreshTask) {
        print("[BackgroundTaskManager] Starting directory refresh")
        
        // Schedule next refresh
        scheduleDirectoryRefresh()
        
        let processingTask = Task {
            do {
                try await refreshDirectory()
                task.setTaskCompleted(success: true)
            } catch {
                print("[BackgroundTaskManager] Directory refresh failed: \(error)")
                task.setTaskCompleted(success: false)
            }
        }
        
        task.expirationHandler = {
            processingTask.cancel()
        }
    }
    
    // MARK: - Processing Logic
    
    /// Process due subscriptions and trigger auto-pay if enabled.
    private func processSubscriptions() async throws {
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        let subscriptionStorage = SubscriptionStorage(identityName: identityName)
        let autoPayStorage = AutoPayStorage(identityName: identityName)
        
        // Get auto-pay settings
        let settings = autoPayStorage.getSettings()
        guard settings.isEnabled else {
            print("[BackgroundTaskManager] Auto-pay disabled, skipping subscription processing")
            return
        }
        
        // Get due subscriptions
        let subscriptions = subscriptionStorage.listSubscriptions()
        let now = Date()
        
        for subscription in subscriptions {
            // Check if subscription is due
            if let nextPayment = subscription.nextPaymentDate, nextPayment <= now {
                // Check spending limits
                let dailyLimit = settings.maxDailyAmountSats ?? UInt64.max
                let perPeerLimit = settings.maxPerPeerAmountSats ?? UInt64.max
                
                if subscription.amountSats <= dailyLimit && subscription.amountSats <= perPeerLimit {
                    print("[BackgroundTaskManager] Processing subscription payment: \(subscription.id)")
                    
                    // In production, this would trigger actual payment
                    // try await paymentService.paySubscription(subscription)
                    
                    // Update subscription state
                    // subscriptionStorage.markPaymentProcessed(subscription.id)
                }
            }
        }
        
        print("[BackgroundTaskManager] Subscription processing complete")
    }
    
    /// Sync payment receipts with remote storage.
    private func syncPayments() async throws {
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        let receiptStorage = ReceiptStorage(identityName: identityName)
        
        // Get pending receipts that need sync
        let pendingReceipts = receiptStorage.listReceipts().filter { $0.status == .pending }
        
        for receipt in pendingReceipts {
            // Check payment status
            // In production, this would query the actual payment network
            print("[BackgroundTaskManager] Checking receipt status: \(receipt.id)")
        }
        
        print("[BackgroundTaskManager] Payment sync complete")
    }
    
    /// Refresh directory contacts and health status.
    private func refreshDirectory() async throws {
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        let contactStorage = ContactStorage(identityName: identityName)
        
        // Get contacts and refresh health status
        let contacts = contactStorage.listContacts()
        
        for contact in contacts {
            // In production, this would check endpoint health
            print("[BackgroundTaskManager] Refreshing contact health: \(contact.name)")
        }
        
        print("[BackgroundTaskManager] Directory refresh complete")
    }
}

// MARK: - Subscription Worker Example (Pattern for Android)

/// Example of how the equivalent Android WorkManager worker would look.
///
/// ```kotlin
/// @HiltWorker
/// class SubscriptionCheckWorker @AssistedInject constructor(
///     @Assisted context: Context,
///     @Assisted params: WorkerParameters,
///     private val subscriptionRepo: SubscriptionRepository,
///     private val autoPayRepo: AutoPayRepository,
///     private val paymentService: PaymentService
/// ) : CoroutineWorker(context, params) {
///
///     override suspend fun doWork(): Result {
///         return try {
///             val settings = autoPayRepo.getSettings()
///             if (!settings.isEnabled) return Result.success()
///
///             val dueSubscriptions = subscriptionRepo.getDueSubscriptions()
///             for (subscription in dueSubscriptions) {
///                 if (checkSpendingLimits(subscription, settings)) {
///                     paymentService.processSubscription(subscription)
///                 }
///             }
///
///             Result.success()
///         } catch (e: Exception) {
///             Result.retry()
///         }
///     }
///
///     companion object {
///         const val WORK_NAME = "subscription_check"
///
///         fun schedule(context: Context) {
///             val constraints = Constraints.Builder()
///                 .setRequiredNetworkType(NetworkType.CONNECTED)
///                 .build()
///
///             val request = PeriodicWorkRequestBuilder<SubscriptionCheckWorker>(
///                 15, TimeUnit.MINUTES
///             )
///                 .setConstraints(constraints)
///                 .build()
///
///             WorkManager.getInstance(context)
///                 .enqueueUniquePeriodicWork(
///                     WORK_NAME,
///                     ExistingPeriodicWorkPolicy.KEEP,
///                     request
///                 )
///         }
///     }
/// }
/// ```

// MARK: - Push Notification Handler Example

/// Example pattern for handling push notifications that trigger payments.
///
/// ## Usage
///
/// When a push notification arrives indicating a payment request:
/// 1. Parse the notification payload
/// 2. Validate the request against spending limits
/// 3. Process payment if auto-pay is enabled
/// 4. Update local state
///
/// ```swift
/// // In your UNUserNotificationCenterDelegate:
/// func userNotificationCenter(
///     _ center: UNUserNotificationCenter,
///     didReceive response: UNNotificationResponse
/// ) async {
///     let userInfo = response.notification.request.content.userInfo
///
///     if let paymentRequest = parsePaymentRequest(from: userInfo) {
///         await PushPaymentHandler.shared.handlePaymentRequest(paymentRequest)
///     }
/// }
/// ```
public final class PushPaymentHandler {
    public static let shared = PushPaymentHandler()
    
    private init() {}
    
    /// Handle incoming payment request from push notification.
    public func handlePaymentRequest(_ request: IncomingPaymentRequest) async {
        print("[PushPaymentHandler] Received payment request: \(request.id)")
        
        // Check if auto-pay is enabled for this sender
        let keyManager = KeyManager()
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        let autoPayStorage = AutoPayStorage(identityName: identityName)
        
        let settings = autoPayStorage.getSettings()
        guard settings.isEnabled else {
            print("[PushPaymentHandler] Auto-pay disabled, storing for manual review")
            storeForManualReview(request)
            return
        }
        
        // Check spending limits
        guard checkSpendingLimits(request, settings: settings) else {
            print("[PushPaymentHandler] Spending limit exceeded, storing for manual review")
            storeForManualReview(request)
            return
        }
        
        // Check trusted senders
        if settings.requiresTrustedSender && !isTrustedSender(request.senderPubkey, settings: settings) {
            print("[PushPaymentHandler] Sender not trusted, storing for manual review")
            storeForManualReview(request)
            return
        }
        
        // Process payment
        do {
            try await processPayment(request)
            print("[PushPaymentHandler] Payment processed successfully")
        } catch {
            print("[PushPaymentHandler] Payment failed: \(error)")
            storeForManualReview(request)
        }
    }
    
    private func checkSpendingLimits(_ request: IncomingPaymentRequest, settings: AutoPaySettings) -> Bool {
        if let maxAmount = settings.maxPerPaymentAmountSats, request.amountSats > maxAmount {
            return false
        }
        // Additional checks for daily/weekly limits would go here
        return true
    }
    
    private func isTrustedSender(_ pubkey: String, settings: AutoPaySettings) -> Bool {
        settings.trustedSenders?.contains(pubkey) ?? false
    }
    
    private func storeForManualReview(_ request: IncomingPaymentRequest) {
        // Store in local database for user to review
    }
    
    private func processPayment(_ request: IncomingPaymentRequest) async throws {
        // Execute payment using PaymentService
    }
}

/// Represents an incoming payment request from push notification.
public struct IncomingPaymentRequest {
    public let id: String
    public let senderPubkey: String
    public let amountSats: UInt64
    public let methodId: String
    public let invoice: String?
    public let memo: String?
}

