//
//  PaykitDemoApp.swift
//  PaykitDemo
//
//  Paykit iOS Demo Application
//  Demonstrates all Paykit features including auto-pay
//

import SwiftUI

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
}
