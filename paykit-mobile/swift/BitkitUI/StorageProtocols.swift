//
//  StorageProtocols.swift
//  PaykitMobile
//
//  Complete storage protocol definitions for Bitkit integration.
//  Bitkit must implement these protocols with their storage mechanism.
//

import Foundation
import PaykitMobile

// MARK: - Receipt Storage

public protocol ReceiptStorageProtocol {
    func recentReceipts(limit: Int) -> [Receipt]
    func totalSent() -> UInt64
    func totalReceived() -> UInt64
    func pendingCount() -> Int
    func saveReceipt(_ receipt: Receipt)
    func deleteReceipt(id: String)
    func receipt(id: String) -> Receipt?
}

// MARK: - Contact Storage

public protocol ContactStorageProtocol {
    func listContacts() -> [Contact]
    func addContact(_ contact: Contact)
    func updateContact(_ contact: Contact)
    func deleteContact(id: String)
    func contact(id: String) -> Contact?
    func contact(pubkey: String) -> Contact?
}

// MARK: - AutoPay Storage

public protocol AutoPayStorageProtocol {
    func getSettings() -> AutoPaySettings
    func saveSettings(_ settings: AutoPaySettings)
    func getPeerLimits() -> [StoredPeerLimit]
    func savePeerLimit(_ limit: StoredPeerLimit)
    func deletePeerLimit(id: String)
    func getRules() -> [StoredAutoPayRule]
    func saveRule(_ rule: StoredAutoPayRule)
    func deleteRule(id: String)
}

// MARK: - Subscription Storage

public protocol SubscriptionStorageProtocol {
    func activeSubscriptions() -> [Subscription]
    func addSubscription(_ subscription: Subscription)
    func updateSubscription(_ subscription: Subscription)
    func deleteSubscription(id: String)
    func subscription(id: String) -> Subscription?
}

// MARK: - Payment Request Storage

public protocol PaymentRequestStorageProtocol {
    func pendingRequests() -> [PaymentRequest]
    func requestHistory() -> [PaymentRequest]
    func pendingCount() -> Int
    func addRequest(_ request: PaymentRequest)
    func updateRequest(_ request: PaymentRequest)
    func deleteRequest(id: String)
    func request(id: String) -> PaymentRequest?
}

// MARK: - Models

public struct Contact: Identifiable, Codable {
    public let id: String
    public var name: String
    public var pubkey: String
    public var createdAt: Date
    public var updatedAt: Date
    
    public init(id: String = UUID().uuidString, name: String, pubkey: String, createdAt: Date = Date(), updatedAt: Date = Date()) {
        self.id = id
        self.name = name
        self.pubkey = pubkey
        self.createdAt = createdAt
        self.updatedAt = updatedAt
    }
}

// MARK: - AutoPay Models

public struct AutoPaySettings: Codable {
    public var isEnabled: Bool
    public var globalDailyLimitSats: Int64
    
    public static let defaults = AutoPaySettings(isEnabled: false, globalDailyLimitSats: 100000)
    
    public init(isEnabled: Bool = false, globalDailyLimitSats: Int64 = 100000) {
        self.isEnabled = isEnabled
        self.globalDailyLimitSats = globalDailyLimitSats
    }
}

public struct PeerSpendingLimit: Identifiable, Codable {
    public let id: String
    public var peerPubkey: String
    public var peerName: String
    public var limit: Int64
    public var used: Int64
    public var period: SpendingPeriod
    public var periodStart: Date
    
    public init(id: String, peerPubkey: String, peerName: String, limit: Int64, used: Int64 = 0, period: SpendingPeriod = .daily, periodStart: Date = Date()) {
        self.id = id
        self.peerPubkey = peerPubkey
        self.peerName = peerName
        self.limit = limit
        self.used = used
        self.period = period
        self.periodStart = periodStart
    }
}

public enum SpendingPeriod: String, Codable, CaseIterable {
    case daily = "Daily"
    case weekly = "Weekly"
    case monthly = "Monthly"
}

public struct AutoPayRule: Identifiable, Codable {
    public let id: String
    public var name: String
    public var description: String
    public var isEnabled: Bool
    public var maxAmount: Int64?
    public var methodFilter: String?
    public var peerFilter: String?
    
    public init(id: String = UUID().uuidString, name: String, description: String, isEnabled: Bool = true, maxAmount: Int64? = nil, methodFilter: String? = nil, peerFilter: String? = nil) {
        self.id = id
        self.name = name
        self.description = description
        self.isEnabled = isEnabled
        self.maxAmount = maxAmount
        self.methodFilter = methodFilter
        self.peerFilter = peerFilter
    }
    
    public func matches(peerPubkey: String, amount: Int64, methodId: String) -> Bool {
        guard isEnabled else { return false }
        
        if let max = maxAmount, amount > max {
            return false
        }
        
        if let method = methodFilter, method != methodId {
            return false
        }
        
        if let peer = peerFilter, peer != peerPubkey {
            return false
        }
        
        return true
    }
}

public struct RecentAutoPayment: Identifiable, Codable {
    public let id: String
    public var peerPubkey: String
    public var peerName: String
    public var amount: Int64
    public var description: String
    public var timestamp: Date
    public var status: PaymentStatus
    public var ruleId: String?
    
    public var formattedTime: String {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .abbreviated
        return formatter.localizedString(for: timestamp, relativeTo: Date())
    }
    
    public init(id: String = UUID().uuidString, peerPubkey: String, peerName: String, amount: Int64, description: String, timestamp: Date = Date(), status: PaymentStatus = .completed, ruleId: String? = nil) {
        self.id = id
        self.peerPubkey = peerPubkey
        self.peerName = peerName
        self.amount = amount
        self.description = description
        self.timestamp = timestamp
        self.status = status
        self.ruleId = ruleId
    }
}

public enum PaymentStatus: String, Codable {
    case pending
    case processing
    case completed
    case failed
}
