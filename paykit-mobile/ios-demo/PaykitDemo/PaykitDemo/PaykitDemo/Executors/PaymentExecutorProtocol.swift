//
//  PaymentExecutorProtocol.swift
//  PaykitDemo
//
//  Defines the payment executor protocol for Bitcoin and Lightning payments.
//  This abstraction allows swapping between mock executors (demo) and real
//  wallet integrations (production).
//

import Foundation

// MARK: - Bitcoin Executor Protocol

/// Protocol for executing on-chain Bitcoin payments.
/// Implement this with your wallet's on-chain capabilities.
public protocol BitcoinExecutorProtocol {
    /// Send Bitcoin to an address.
    /// - Parameters:
    ///   - address: The destination Bitcoin address
    ///   - amountSats: Amount to send in satoshis
    ///   - feeRate: Fee rate in sat/vB (optional, uses default if nil)
    /// - Returns: Transaction result with txid and fee info
    func sendToAddress(
        address: String,
        amountSats: UInt64,
        feeRate: Double?
    ) async throws -> BitcoinTxResult
    
    /// Estimate the fee for a transaction.
    /// - Parameters:
    ///   - address: The destination address
    ///   - amountSats: Amount to send
    ///   - targetBlocks: Target confirmation time in blocks
    /// - Returns: Estimated fee in satoshis
    func estimateFee(
        address: String,
        amountSats: UInt64,
        targetBlocks: UInt32
    ) async throws -> UInt64
    
    /// Get transaction details by txid.
    func getTransaction(txid: String) async throws -> BitcoinTxResult?
    
    /// Verify a transaction matches expected parameters.
    func verifyTransaction(
        txid: String,
        address: String,
        amountSats: UInt64
    ) async throws -> Bool
}

// MARK: - Lightning Executor Protocol

/// Protocol for executing Lightning Network payments.
/// Implement this with your wallet's Lightning capabilities.
public protocol LightningExecutorProtocol {
    /// Pay a BOLT11 invoice.
    /// - Parameters:
    ///   - invoice: The BOLT11 invoice string
    ///   - amountMsat: Amount in millisatoshis (for zero-amount invoices)
    ///   - maxFeeMsat: Maximum fee to pay (optional)
    /// - Returns: Payment result with preimage and fee info
    func payInvoice(
        invoice: String,
        amountMsat: UInt64?,
        maxFeeMsat: UInt64?
    ) async throws -> LightningPaymentResult
    
    /// Decode a BOLT11 invoice without paying.
    func decodeInvoice(invoice: String) throws -> DecodedInvoice
    
    /// Estimate routing fee for an invoice.
    func estimateFee(invoice: String) async throws -> UInt64
    
    /// Get payment status by payment hash.
    func getPayment(paymentHash: String) async throws -> LightningPaymentResult?
    
    /// Verify a preimage matches a payment hash.
    func verifyPreimage(preimage: String, paymentHash: String) -> Bool
}

// MARK: - Result Types

/// Result of a Bitcoin transaction.
public struct BitcoinTxResult: Equatable {
    public let txid: String
    public let rawTx: String?
    public let vout: UInt32
    public let feeSats: UInt64
    public let feeRate: Double
    public let blockHeight: UInt32?
    public let confirmations: UInt32
    
    public init(
        txid: String,
        rawTx: String? = nil,
        vout: UInt32 = 0,
        feeSats: UInt64,
        feeRate: Double,
        blockHeight: UInt32? = nil,
        confirmations: UInt32 = 0
    ) {
        self.txid = txid
        self.rawTx = rawTx
        self.vout = vout
        self.feeSats = feeSats
        self.feeRate = feeRate
        self.blockHeight = blockHeight
        self.confirmations = confirmations
    }
}

/// Result of a Lightning payment.
public struct LightningPaymentResult: Equatable {
    public let preimage: String
    public let paymentHash: String
    public let amountMsat: UInt64
    public let feeMsat: UInt64
    public let hops: UInt32
    public let status: LightningPaymentStatus
    
    public init(
        preimage: String,
        paymentHash: String,
        amountMsat: UInt64,
        feeMsat: UInt64,
        hops: UInt32 = 0,
        status: LightningPaymentStatus
    ) {
        self.preimage = preimage
        self.paymentHash = paymentHash
        self.amountMsat = amountMsat
        self.feeMsat = feeMsat
        self.hops = hops
        self.status = status
    }
}

/// Status of a Lightning payment.
public enum LightningPaymentStatus: String, Equatable {
    case pending
    case succeeded
    case failed
}

/// Decoded BOLT11 invoice information.
public struct DecodedInvoice: Equatable {
    public let paymentHash: String
    public let amountMsat: UInt64?
    public let description: String?
    public let descriptionHash: String?
    public let payee: String
    public let expiry: UInt64
    public let timestamp: UInt64
    public let isExpired: Bool
    
    public init(
        paymentHash: String,
        amountMsat: UInt64? = nil,
        description: String? = nil,
        descriptionHash: String? = nil,
        payee: String = "",
        expiry: UInt64 = 3600,
        timestamp: UInt64 = 0,
        isExpired: Bool = false
    ) {
        self.paymentHash = paymentHash
        self.amountMsat = amountMsat
        self.description = description
        self.descriptionHash = descriptionHash
        self.payee = payee
        self.expiry = expiry
        self.timestamp = timestamp
        self.isExpired = isExpired
    }
}

// MARK: - Executor Errors

/// Errors that can occur during payment execution.
public enum PaymentExecutorError: LocalizedError {
    case notInitialized
    case invalidAddress(String)
    case invalidInvoice(String)
    case insufficientFunds
    case paymentFailed(String)
    case timeout
    case networkError(String)
    
    public var errorDescription: String? {
        switch self {
        case .notInitialized:
            return "Payment executor not initialized"
        case .invalidAddress(let address):
            return "Invalid Bitcoin address: \(address)"
        case .invalidInvoice(let reason):
            return "Invalid invoice: \(reason)"
        case .insufficientFunds:
            return "Insufficient funds for this payment"
        case .paymentFailed(let reason):
            return "Payment failed: \(reason)"
        case .timeout:
            return "Payment timed out"
        case .networkError(let reason):
            return "Network error: \(reason)"
        }
    }
}

