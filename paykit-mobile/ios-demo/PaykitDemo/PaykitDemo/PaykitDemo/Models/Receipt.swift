//
//  Receipt.swift
//  PaykitDemo
//
//  Receipt model for payment history tracking.
//

import Foundation

/// Payment status
enum PaymentStatus: String, Codable {
    case pending = "pending"
    case completed = "completed"
    case failed = "failed"
    case refunded = "refunded"
}

/// Payment direction
enum PaymentDirection: String, Codable {
    case sent = "sent"
    case received = "received"
}

/// A payment receipt
struct Receipt: Identifiable, Codable, Equatable {
    /// Unique identifier
    let id: String
    /// Direction of payment
    let direction: PaymentDirection
    /// Counterparty public key (z-base32)
    let counterpartyKey: String
    /// Counterparty display name (if known)
    var counterpartyName: String?
    /// Amount in satoshis
    let amountSats: UInt64
    /// Payment status
    var status: PaymentStatus
    /// Payment method used
    let paymentMethod: String
    /// When the payment was initiated
    let createdAt: Date
    /// When the payment was completed (if applicable)
    var completedAt: Date?
    /// Optional memo/note
    var memo: String?
    /// Transaction ID (if applicable)
    var txId: String?
    
    init(
        direction: PaymentDirection,
        counterpartyKey: String,
        counterpartyName: String? = nil,
        amountSats: UInt64,
        paymentMethod: String,
        memo: String? = nil
    ) {
        self.id = UUID().uuidString
        self.direction = direction
        self.counterpartyKey = counterpartyKey
        self.counterpartyName = counterpartyName
        self.amountSats = amountSats
        self.status = .pending
        self.paymentMethod = paymentMethod
        self.createdAt = Date()
        self.completedAt = nil
        self.memo = memo
        self.txId = nil
    }
    
    /// Mark as completed
    mutating func complete(txId: String? = nil) {
        self.status = .completed
        self.completedAt = Date()
        self.txId = txId
    }
    
    /// Mark as failed
    mutating func fail() {
        self.status = .failed
    }
}

extension Receipt {
    /// Abbreviated counterparty key for display
    var abbreviatedCounterparty: String {
        guard counterpartyKey.count > 16 else { return counterpartyKey }
        let prefix = counterpartyKey.prefix(8)
        let suffix = counterpartyKey.suffix(8)
        return "\(prefix)...\(suffix)"
    }
    
    /// Display name (name if known, otherwise abbreviated key)
    var displayName: String {
        counterpartyName ?? abbreviatedCounterparty
    }
    
    /// Formatted amount
    var formattedAmount: String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        return "\(direction == .sent ? "-" : "+")\(formatter.string(from: NSNumber(value: amountSats)) ?? "\(amountSats)") sats"
    }
    
    /// Status color name (for SwiftUI)
    var statusColorName: String {
        switch status {
        case .pending: return "orange"
        case .completed: return "green"
        case .failed: return "red"
        case .refunded: return "purple"
        }
    }
}

