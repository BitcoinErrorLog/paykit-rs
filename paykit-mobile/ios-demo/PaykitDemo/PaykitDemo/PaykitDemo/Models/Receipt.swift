//
//  Receipt.swift
//  PaykitDemo
//
//  Receipt model for payment history tracking.
//

import Foundation

/// Payment status
public enum PaymentReceiptStatus: String, Codable {
    case pending = "pending"
    case completed = "completed"
    case failed = "failed"
    case refunded = "refunded"
}

/// Payment direction
public enum PaymentDirection: String, Codable {
    case sent = "sent"
    case received = "received"
}

/// A payment receipt (local model, different from PaykitMobile.Receipt)
public struct PaymentReceipt: Identifiable, Codable, Equatable {
    /// Unique identifier
    public let id: String
    /// Direction of payment
    public let direction: PaymentDirection
    /// Counterparty public key (z-base32)
    public let counterpartyKey: String
    /// Counterparty display name (if known)
    public var counterpartyName: String?
    /// Amount in satoshis
    public let amountSats: UInt64
    /// Payment status
    public var status: PaymentReceiptStatus
    /// Payment method used
    public let paymentMethod: String
    /// When the payment was initiated
    public let createdAt: Date
    /// When the payment was completed (if applicable)
    public var completedAt: Date?
    /// Optional memo/note
    public var memo: String?
    /// Transaction ID (if applicable)
    public var txId: String?
    /// Payment proof (optional, JSON string)
    public var proof: String?
    /// Whether proof has been verified
    public var proofVerified: Bool = false
    /// Timestamp when proof was verified
    public var proofVerifiedAt: Date?
    
    public init(
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
        self.proof = nil
        self.proofVerified = false
        self.proofVerifiedAt = nil
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

extension PaymentReceipt {
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
    
    /// Create a local PaymentReceipt from an FFI Receipt
    /// - Parameters:
    ///   - ffiReceipt: The FFI Receipt from PaykitClient.createReceipt()
    ///   - direction: Whether this is a sent or received payment
    ///   - counterpartyName: Optional display name for the counterparty
    /// - Returns: A local PaymentReceipt for storage
    static func fromFFI(
        _ ffiReceipt: Receipt,
        direction: PaymentDirection,
        counterpartyName: String? = nil
    ) -> PaymentReceipt {
        let counterpartyKey = direction == .sent ? ffiReceipt.payee : ffiReceipt.payer
        let amountSats = UInt64(ffiReceipt.amount ?? "0") ?? 0
        
        var receipt = PaymentReceipt(
            direction: direction,
            counterpartyKey: counterpartyKey,
            counterpartyName: counterpartyName,
            amountSats: amountSats,
            paymentMethod: ffiReceipt.methodId
        )
        
        // Override the auto-generated ID with the FFI receipt ID
        // We need to use a different initializer approach
        return PaymentReceipt(
            id: ffiReceipt.receiptId,
            direction: direction,
            counterpartyKey: counterpartyKey,
            counterpartyName: counterpartyName,
            amountSats: amountSats,
            status: .pending,
            paymentMethod: ffiReceipt.methodId,
            createdAt: Date(timeIntervalSince1970: Double(ffiReceipt.createdAt)),
            completedAt: nil,
            memo: nil,
            txId: nil,
            proof: nil,
            proofVerified: false,
            proofVerifiedAt: nil
        )
    }
    
    /// Mark proof as verified
    mutating func markProofVerified() {
        self.proofVerified = true
        self.proofVerifiedAt = Date()
    }
}

// MARK: - Full Initializer for FFI Integration

extension PaymentReceipt {
    /// Full initializer for creating receipts with all fields
    init(
        id: String,
        direction: PaymentDirection,
        counterpartyKey: String,
        counterpartyName: String?,
        amountSats: UInt64,
        status: PaymentReceiptStatus,
        paymentMethod: String,
        createdAt: Date,
        completedAt: Date?,
        memo: String?,
        txId: String?,
        proof: String?,
        proofVerified: Bool,
        proofVerifiedAt: Date?
    ) {
        self.id = id
        self.direction = direction
        self.counterpartyKey = counterpartyKey
        self.counterpartyName = counterpartyName
        self.amountSats = amountSats
        self.status = status
        self.paymentMethod = paymentMethod
        self.createdAt = createdAt
        self.completedAt = completedAt
        self.memo = memo
        self.txId = txId
        self.proof = proof
        self.proofVerified = proofVerified
        self.proofVerifiedAt = proofVerifiedAt
    }
}

