//
//  AutoPayModels.swift
//  PaykitDemo
//
//  Data models for auto-pay functionality
//

import Foundation

// MARK: - Spending Period

enum SpendingPeriod: String, Codable, CaseIterable {
    case hourly = "Hourly"
    case daily = "Daily"
    case weekly = "Weekly"
    case monthly = "Monthly"
    
    var seconds: Int64 {
        switch self {
        case .hourly: return 3600
        case .daily: return 86400
        case .weekly: return 604800
        case .monthly: return 2592000
        }
    }
}

// MARK: - Peer Spending Limit

struct PeerSpendingLimit: Identifiable, Codable {
    let id: String
    let peerPubkey: String
    let peerName: String
    var limit: Int64
    var used: Int64
    let period: SpendingPeriod
    let periodStart: Date
    
    var remaining: Int64 {
        max(0, limit - used)
    }
    
    var percentUsed: Double {
        guard limit > 0 else { return 0 }
        return Double(used) / Double(limit)
    }
    
    var isExhausted: Bool {
        used >= limit
    }
    
    mutating func reset() {
        used = 0
    }
    
    func shouldReset(now: Date = Date()) -> Bool {
        let elapsed = now.timeIntervalSince(periodStart)
        return elapsed >= Double(period.seconds)
    }
}

// MARK: - Auto-Pay Rule

struct AutoPayRule: Identifiable, Codable {
    let id: String
    var name: String
    var description: String
    var isEnabled: Bool
    var maxAmount: Int64?
    var methodFilter: String?
    var peerFilter: String?
    
    /// Check if this rule matches a payment
    func matches(
        peerPubkey: String,
        amount: Int64,
        methodId: String
    ) -> Bool {
        // Check amount
        if let max = maxAmount, amount > max {
            return false
        }
        
        // Check method
        if let method = methodFilter, method != methodId {
            return false
        }
        
        // Check peer
        if let peer = peerFilter, peer != peerPubkey {
            return false
        }
        
        return isEnabled
    }
}

// MARK: - Recent Auto-Payment

struct RecentAutoPayment: Identifiable, Codable {
    let id: String
    let peerPubkey: String
    let peerName: String
    let amount: Int64
    let description: String
    let timestamp: Date
    let status: PaymentExecutionStatus
    let ruleId: String?
    
    var formattedAmount: String {
        "\(amount) sats"
    }
    
    var formattedTime: String {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .abbreviated
        return formatter.localizedString(for: timestamp, relativeTo: Date())
    }
}

// MARK: - Payment Execution Status

enum PaymentExecutionStatus: String, Codable {
    case pending
    case processing
    case completed
    case failed
    
    var color: String {
        switch self {
        case .pending: return "yellow"
        case .processing: return "blue"
        case .completed: return "green"
        case .failed: return "red"
        }
    }
}

// MARK: - Auto-Pay Settings

struct AutoPaySettings: Codable {
    var isEnabled: Bool
    var globalDailyLimit: Int64
    var requireBiometricAbove: Int64?
    var notifyOnAutoPay: Bool
    var notifyOnLimitReached: Bool
    
    static var defaults: AutoPaySettings {
        AutoPaySettings(
            isEnabled: false,
            globalDailyLimit: 100000,
            requireBiometricAbove: 10000,
            notifyOnAutoPay: true,
            notifyOnLimitReached: true
        )
    }
}

// MARK: - Spending Summary

struct SpendingSummary: Codable {
    let period: SpendingPeriod
    let periodStart: Date
    let periodEnd: Date
    let totalSpent: Int64
    let totalLimit: Int64
    let paymentCount: Int
    let topPeers: [PeerSpending]
    
    var percentUsed: Double {
        guard totalLimit > 0 else { return 0 }
        return Double(totalSpent) / Double(totalLimit)
    }
    
    var remaining: Int64 {
        max(0, totalLimit - totalSpent)
    }
}

struct PeerSpending: Codable, Identifiable {
    var id: String { peerPubkey }
    let peerPubkey: String
    let peerName: String
    let amount: Int64
    let count: Int
}
