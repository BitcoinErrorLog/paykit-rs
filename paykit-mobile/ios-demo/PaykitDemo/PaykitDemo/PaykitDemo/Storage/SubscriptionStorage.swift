//
//  SubscriptionStorage.swift
//  PaykitDemo
//
//  Persistent storage for subscriptions using Keychain.
//

import Foundation

/// A stored subscription
struct StoredSubscription: Identifiable, Codable {
    let id: String
    var providerName: String
    var providerPubkey: String
    var amountSats: Int64
    var currency: String
    var frequency: String  // daily, weekly, monthly, yearly
    var description: String
    var methodId: String
    var isActive: Bool
    var createdAt: Date
    var lastPaymentAt: Date?
    var nextPaymentAt: Date?
    var paymentCount: Int
    
    init(
        providerName: String,
        providerPubkey: String,
        amountSats: Int64,
        currency: String = "SAT",
        frequency: String,
        description: String,
        methodId: String = "lightning"
    ) {
        self.id = UUID().uuidString
        self.providerName = providerName
        self.providerPubkey = providerPubkey
        self.amountSats = amountSats
        self.currency = currency
        self.frequency = frequency
        self.description = description
        self.methodId = methodId
        self.isActive = true
        self.createdAt = Date()
        self.lastPaymentAt = nil
        self.nextPaymentAt = Self.calculateNextPayment(frequency: frequency, from: Date())
        self.paymentCount = 0
    }
    
    mutating func recordPayment() {
        lastPaymentAt = Date()
        paymentCount += 1
        nextPaymentAt = Self.calculateNextPayment(frequency: frequency, from: Date())
    }
    
    static func calculateNextPayment(frequency: String, from date: Date) -> Date? {
        let calendar = Calendar.current
        switch frequency.lowercased() {
        case "daily":
            return calendar.date(byAdding: .day, value: 1, to: date)
        case "weekly":
            return calendar.date(byAdding: .weekOfYear, value: 1, to: date)
        case "monthly":
            return calendar.date(byAdding: .month, value: 1, to: date)
        case "yearly":
            return calendar.date(byAdding: .year, value: 1, to: date)
        default:
            return nil
        }
    }
}

/// Manages persistent storage of subscriptions
class SubscriptionStorage {
    
    private let keychain: KeychainStorage
    private let identityName: String
    
    // In-memory cache
    private var subscriptionsCache: [StoredSubscription]?
    
    private var storageKey: String {
        "paykit.subscriptions.\(identityName)"
    }
    
    init(identityName: String, keychain: KeychainStorage = KeychainStorage(serviceIdentifier: "com.paykit.demo")) {
        self.identityName = identityName
        self.keychain = keychain
    }
    
    // MARK: - CRUD Operations
    
    func listSubscriptions() -> [StoredSubscription] {
        if let cached = subscriptionsCache {
            return cached
        }
        
        do {
            guard let data = try keychain.retrieve(key: storageKey) else {
                return []
            }
            let subscriptions = try JSONDecoder().decode([StoredSubscription].self, from: data)
            subscriptionsCache = subscriptions
            return subscriptions
        } catch {
            print("SubscriptionStorage: Failed to load subscriptions: \(error)")
            return []
        }
    }
    
    func getSubscription(id: String) -> StoredSubscription? {
        return listSubscriptions().first { $0.id == id }
    }
    
    func saveSubscription(_ subscription: StoredSubscription) throws {
        var subscriptions = listSubscriptions()
        
        if let index = subscriptions.firstIndex(where: { $0.id == subscription.id }) {
            subscriptions[index] = subscription
        } else {
            subscriptions.append(subscription)
        }
        
        try persistSubscriptions(subscriptions)
    }
    
    func deleteSubscription(id: String) throws {
        var subscriptions = listSubscriptions()
        subscriptions.removeAll { $0.id == id }
        try persistSubscriptions(subscriptions)
    }
    
    func toggleActive(id: String) throws {
        var subscriptions = listSubscriptions()
        guard let index = subscriptions.firstIndex(where: { $0.id == id }) else { return }
        subscriptions[index].isActive.toggle()
        try persistSubscriptions(subscriptions)
    }
    
    func recordPayment(subscriptionId: String) throws {
        var subscriptions = listSubscriptions()
        guard let index = subscriptions.firstIndex(where: { $0.id == subscriptionId }) else { return }
        subscriptions[index].recordPayment()
        try persistSubscriptions(subscriptions)
    }
    
    func activeSubscriptions() -> [StoredSubscription] {
        listSubscriptions().filter { $0.isActive }
    }
    
    func clearAll() throws {
        try persistSubscriptions([])
    }
    
    // MARK: - Private
    
    private func persistSubscriptions(_ subscriptions: [StoredSubscription]) throws {
        let data = try JSONEncoder().encode(subscriptions)
        try keychain.store(key: storageKey, data: data)
        subscriptionsCache = subscriptions
    }
}

