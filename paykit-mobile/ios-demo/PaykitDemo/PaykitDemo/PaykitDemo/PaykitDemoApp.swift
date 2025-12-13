//
//  PaykitDemoApp.swift
//  PaykitDemo
//
//  Paykit iOS Demo Application
//  Demonstrates all Paykit features including auto-pay
//

import SwiftUI
import Combine

@main
struct PaykitDemoApp: App {
    @StateObject private var appState = AppState()
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
        }
    }
}

/// Global application state
class AppState: ObservableObject {
    @Published var paykitClient: PaykitClientWrapper
    @Published var isInitialized = false
    @Published var errorMessage: String?
    
    init() {
        do {
            self.paykitClient = try PaykitClientWrapper()
            self.isInitialized = true
        } catch {
            self.paykitClient = PaykitClientWrapper.placeholder()
            self.errorMessage = "Failed to initialize Paykit: \(error.localizedDescription)"
        }
    }
}

/// Wrapper around the FFI PaykitClient
class PaykitClientWrapper: ObservableObject {
    private var client: PaykitClient?
    private let storage: KeychainStorage
    
    init() throws {
        self.storage = KeychainStorage(serviceIdentifier: "com.paykit.demo")
        self.client = try PaykitClient()
    }
    
    /// Placeholder for error states
    static func placeholder() -> PaykitClientWrapper {
        let wrapper = try! PaykitClientWrapper()
        wrapper.client = nil
        return wrapper
    }
    
    var isAvailable: Bool {
        client != nil
    }
    
    // MARK: - Payment Methods
    
    func listMethods() -> [String] {
        client?.listMethods() ?? []
    }
    
    func validateEndpoint(methodId: String, endpoint: String) -> Bool {
        (try? client?.validateEndpoint(methodId: methodId, endpoint: endpoint)) ?? false
    }
    
    func selectMethod(
        methods: [PaymentMethod],
        amountSats: UInt64,
        preferences: SelectionPreferences?
    ) -> SelectionResult? {
        try? client?.selectMethod(
            supportedMethods: methods,
            amountSats: amountSats,
            preferences: preferences
        )
    }
    
    // MARK: - Health
    
    func checkHealth() -> [HealthCheckResult] {
        client?.checkHealth() ?? []
    }
    
    func getHealthStatus(methodId: String) -> HealthStatus? {
        client?.getHealthStatus(methodId: methodId)
    }
    
    func isMethodUsable(methodId: String) -> Bool {
        client?.isMethodUsable(methodId: methodId) ?? false
    }
    
    // MARK: - Subscriptions
    
    func createSubscription(
        subscriber: String,
        provider: String,
        terms: SubscriptionTerms
    ) -> Subscription? {
        try? client?.createSubscription(
            subscriber: subscriber,
            provider: provider,
            terms: terms
        )
    }
    
    func calculateProration(
        currentAmountSats: Int64,
        newAmountSats: Int64,
        periodStart: Int64,
        periodEnd: Int64,
        changeDate: Int64
    ) -> ProrationResult? {
        try? client?.calculateProration(
            currentAmountSats: currentAmountSats,
            newAmountSats: newAmountSats,
            periodStart: periodStart,
            periodEnd: periodEnd,
            changeDate: changeDate
        )
    }
    
    func daysRemainingInPeriod(periodEnd: Int64) -> UInt32 {
        client?.daysRemainingInPeriod(periodEnd: periodEnd) ?? 0
    }
    
    // MARK: - Payment Requests
    
    func createPaymentRequest(
        fromPubkey: String,
        toPubkey: String,
        amountSats: Int64,
        currency: String,
        methodId: String,
        description: String,
        expiresInSecs: UInt64?
    ) -> PaymentRequest? {
        try? client?.createPaymentRequest(
            fromPubkey: fromPubkey,
            toPubkey: toPubkey,
            amountSats: amountSats,
            currency: currency,
            methodId: methodId,
            description: description,
            expiresInSecs: expiresInSecs
        )
    }
    
    // MARK: - Receipts
    
    func createReceipt(
        payer: String,
        payee: String,
        methodId: String,
        amount: String?,
        currency: String?
    ) -> Receipt? {
        try? client?.createReceipt(
            payer: payer,
            payee: payee,
            methodId: methodId,
            amount: amount,
            currency: currency
        )
    }
    
    func getPaymentStatus(receiptId: String) -> PaymentStatusInfo? {
        client?.getPaymentStatus(receiptId: receiptId)
    }
    
    func getInProgressPayments() -> [PaymentStatusInfo] {
        client?.getInProgressPayments() ?? []
    }
    
    // MARK: - QR Scanning
    
    func parseScannedQR(data: String) -> ScannedUri? {
        try? client?.parseScannedQr(scannedData: data)
    }
    
    func isPaykitQR(data: String) -> Bool {
        client?.isPaykitQr(scannedData: data) ?? false
    }
    
    // MARK: - Secure Storage
    
    func storeSecurely(key: String, data: Data) throws {
        try storage.store(key: key, data: data)
    }
    
    func retrieveSecurely(key: String) throws -> Data? {
        try storage.retrieve(key: key)
    }
    
    func deleteSecurely(key: String) throws {
        try storage.delete(key: key)
    }
    
    // MARK: - Directory Operations
    
    /// Create a directory service for fetching contacts and payment endpoints
    func createDirectoryService() -> DirectoryService {
        DirectoryService()
    }
}

// MARK: - Directory Service Configuration

/// Configuration for directory service transport
enum DirectoryTransportMode {
    /// Use mock transport (for development/testing)
    case mock
    /// Use callback-based transport with Pubky SDK
    case callback(PubkyUnauthenticatedStorageCallback)
}

// MARK: - Directory Service

/// Service for interacting with the Pubky directory
/// Provides access to contacts and payment endpoint discovery
///
/// ## Usage
///
/// ### Development/Testing (Mock Transport)
/// ```swift
/// let service = DirectoryService(mode: .mock)
/// ```
///
/// ### Production (Real Pubky Transport)
/// ```swift
/// let pubkyCallback = MyPubkyStorageCallback(pubkyClient: myPubkyClient)
/// let service = DirectoryService(mode: .callback(pubkyCallback))
/// ```
class DirectoryService {
    private let directoryOps: DirectoryOperationsAsync
    private let unauthTransport: UnauthenticatedTransportFfi
    
    /// Whether this service is using mock transport
    let isMockMode: Bool
    
    /// Initialize with mock transport (default for demo)
    convenience init() {
        self.init(mode: .mock)
    }
    
    /// Initialize with specified transport mode
    /// - Parameter mode: Transport mode (mock or callback)
    init(mode: DirectoryTransportMode) {
        switch mode {
        case .mock:
            self.unauthTransport = UnauthenticatedTransportFfi.newMock()
            self.isMockMode = true
        case .callback(let callback):
            self.unauthTransport = UnauthenticatedTransportFfi.fromCallback(callback: callback)
            self.isMockMode = false
        }
        self.directoryOps = try! createDirectoryOperationsAsync()
    }
    
    /// Fetch known contacts from a user's Pubky directory
    /// - Parameter ownerPubkey: The public key of the owner (z-base32 format)
    /// - Returns: Array of contact public keys
    func fetchKnownContacts(ownerPubkey: String) async throws -> [String] {
        try directoryOps.fetchKnownContacts(transport: unauthTransport, ownerPubkey: ownerPubkey)
    }
    
    /// Fetch payment endpoint for a specific method
    /// - Parameters:
    ///   - ownerPubkey: The public key of the payee
    ///   - methodId: The payment method ID (e.g., "lightning", "onchain")
    /// - Returns: The endpoint data if found, nil otherwise
    func fetchPaymentEndpoint(ownerPubkey: String, methodId: String) async throws -> String? {
        try directoryOps.fetchPaymentEndpoint(transport: unauthTransport, ownerPubkey: ownerPubkey, methodId: methodId)
    }
    
    /// Fetch all supported payment methods for a payee
    /// - Parameter ownerPubkey: The public key of the payee
    /// - Returns: Array of payment methods supported by the payee
    func fetchSupportedPayments(ownerPubkey: String) async throws -> [PaymentMethod] {
        try directoryOps.fetchSupportedPayments(transport: unauthTransport, ownerPubkey: ownerPubkey)
    }
}

// MARK: - Example Pubky Storage Callback

/// Example implementation of PubkyUnauthenticatedStorageCallback
/// 
/// Implement this protocol to integrate with the real Pubky SDK.
/// This example shows the interface - you need to replace the implementation
/// with actual Pubky SDK calls.
///
/// ```swift
/// class MyPubkyStorage: PubkyUnauthenticatedStorageCallback {
///     let pubkyClient: PubkyClient // Your Pubky SDK client
///     
///     func get(ownerPubkey: String, path: String) -> StorageGetResult {
///         do {
///             let content = try pubkyClient.publicGet(owner: ownerPubkey, path: path)
///             return StorageGetResult(success: true, content: content, error: nil)
///         } catch {
///             return StorageGetResult(success: false, content: nil, error: error.localizedDescription)
///         }
///     }
///     
///     func list(ownerPubkey: String, prefix: String) -> StorageListResult {
///         do {
///             let entries = try pubkyClient.publicList(owner: ownerPubkey, prefix: prefix)
///             return StorageListResult(success: true, entries: entries, error: nil)
///         } catch {
///             return StorageListResult(success: false, entries: [], error: error.localizedDescription)
///         }
///     }
/// }
/// ```
