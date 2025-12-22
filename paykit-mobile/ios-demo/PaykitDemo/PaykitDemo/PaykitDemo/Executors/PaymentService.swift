//
//  PaymentService.swift
//  PaykitDemo
//
//  High-level payment service that coordinates payment execution.
//  Uses executor protocols to abstract Bitcoin/Lightning operations.
//

import Foundation

/// Detected payment type from a scanned or entered string.
public enum DetectedPaymentType: Equatable {
    case bitcoinAddress(address: String)
    case bolt11Invoice(invoice: String)
    case pubkyUri(pubkey: String)
    case unknown
}

/// Result of a payment execution.
public enum PaymentResult {
    case bitcoin(BitcoinTxResult)
    case lightning(LightningPaymentResult)
    
    public var isSuccess: Bool {
        switch self {
        case .bitcoin:
            return true
        case .lightning(let result):
            return result.status == .succeeded
        }
    }
}

/// High-level payment service for executing payments.
/// Integrates with executor protocols for Bitcoin and Lightning.
public final class PaymentService: ObservableObject {
    
    // MARK: - Shared Instance
    
    public static let shared = PaymentService()
    
    // MARK: - Published State
    
    @Published public var isExecuting = false
    @Published public var lastError: Error?
    @Published public private(set) var recentPayments: [PaymentRecord] = []
    
    // MARK: - Properties
    
    public var bitcoinExecutor: BitcoinExecutorProtocol
    public var lightningExecutor: LightningExecutorProtocol
    
    private let receiptStorage: ReceiptStorage
    private let queue = DispatchQueue(label: "com.paykit.demo.paymentservice")
    
    // MARK: - Initialization
    
    public init(
        bitcoinExecutor: BitcoinExecutorProtocol = MockBitcoinExecutor(),
        lightningExecutor: LightningExecutorProtocol = MockLightningExecutor(),
        receiptStorage: ReceiptStorage = ReceiptStorage()
    ) {
        self.bitcoinExecutor = bitcoinExecutor
        self.lightningExecutor = lightningExecutor
        self.receiptStorage = receiptStorage
    }
    
    // MARK: - Payment Detection
    
    /// Detect the type of payment from a scanned or entered string.
    public func detectPaymentType(_ input: String) -> DetectedPaymentType {
        let trimmed = input.trimmingCharacters(in: .whitespacesAndNewlines)
        
        // Check for BOLT11 invoice
        let lowercased = trimmed.lowercased()
        if lowercased.hasPrefix("lnbc") || 
           lowercased.hasPrefix("lntb") || 
           lowercased.hasPrefix("lnbcrt") {
            return .bolt11Invoice(invoice: trimmed)
        }
        
        // Check for Lightning URI
        if lowercased.hasPrefix("lightning:") {
            let invoice = String(trimmed.dropFirst(10))
            return .bolt11Invoice(invoice: invoice)
        }
        
        // Check for Pubky URI
        if lowercased.hasPrefix("pubky://") {
            let pubkey = String(trimmed.dropFirst(8))
            return .pubkyUri(pubkey: pubkey)
        }
        
        // Check for Bitcoin address
        if isBitcoinAddress(trimmed) {
            return .bitcoinAddress(address: trimmed)
        }
        
        // Check for Bitcoin URI
        if lowercased.hasPrefix("bitcoin:") {
            let address = extractAddressFromBitcoinUri(trimmed)
            if let addr = address {
                return .bitcoinAddress(address: addr)
            }
        }
        
        return .unknown
    }
    
    // MARK: - Payment Execution
    
    /// Send a Bitcoin payment to an address.
    public func sendBitcoin(
        to address: String,
        amountSats: UInt64,
        feeRate: Double? = nil,
        memo: String? = nil
    ) async throws -> BitcoinTxResult {
        await setExecuting(true)
        defer { Task { await setExecuting(false) } }
        
        do {
            let result = try await bitcoinExecutor.sendToAddress(
                address: address,
                amountSats: amountSats,
                feeRate: feeRate
            )
            
            // Store receipt
            let record = PaymentRecord(
                id: result.txid,
                type: .bitcoin,
                direction: .sent,
                amountSats: amountSats,
                feeSats: result.feeSats,
                counterparty: address,
                memo: memo,
                timestamp: Date(),
                status: .completed,
                paymentHash: nil,
                preimage: nil
            )
            await addPaymentRecord(record)
            
            return result
        } catch {
            await setError(error)
            throw error
        }
    }
    
    /// Pay a Lightning invoice.
    public func payLightningInvoice(
        _ invoice: String,
        amountMsat: UInt64? = nil,
        maxFeeMsat: UInt64? = nil,
        memo: String? = nil
    ) async throws -> LightningPaymentResult {
        await setExecuting(true)
        defer { Task { await setExecuting(false) } }
        
        do {
            let result = try await lightningExecutor.payInvoice(
                invoice: invoice,
                amountMsat: amountMsat,
                maxFeeMsat: maxFeeMsat
            )
            
            // Store receipt
            let record = PaymentRecord(
                id: result.paymentHash,
                type: .lightning,
                direction: .sent,
                amountSats: result.amountMsat / 1000,
                feeSats: result.feeMsat / 1000,
                counterparty: nil,
                memo: memo,
                timestamp: Date(),
                status: result.status == .succeeded ? .completed : .failed,
                paymentHash: result.paymentHash,
                preimage: result.preimage
            )
            await addPaymentRecord(record)
            
            return result
        } catch {
            await setError(error)
            throw error
        }
    }
    
    /// Smart payment - automatically detects type and executes.
    public func pay(
        input: String,
        amountSats: UInt64? = nil,
        memo: String? = nil
    ) async throws -> PaymentResult {
        let paymentType = detectPaymentType(input)
        
        switch paymentType {
        case .bitcoinAddress(let address):
            guard let amount = amountSats else {
                throw PaymentExecutorError.paymentFailed("Amount required for Bitcoin payment")
            }
            let result = try await sendBitcoin(to: address, amountSats: amount, memo: memo)
            return .bitcoin(result)
            
        case .bolt11Invoice(let invoice):
            let amountMsat = amountSats.map { $0 * 1000 }
            let result = try await payLightningInvoice(invoice, amountMsat: amountMsat, memo: memo)
            return .lightning(result)
            
        case .pubkyUri(let pubkey):
            // For Pubky URIs, we need to discover payment methods first
            // This would integrate with NoisePaymentService
            throw PaymentExecutorError.paymentFailed("Pubky payments require contact discovery. Use Noise Payment flow.")
            
        case .unknown:
            throw PaymentExecutorError.paymentFailed("Unable to detect payment type")
        }
    }
    
    // MARK: - Fee Estimation
    
    /// Estimate fee for a Bitcoin transaction.
    public func estimateBitcoinFee(
        to address: String,
        amountSats: UInt64,
        targetBlocks: UInt32 = 6
    ) async throws -> UInt64 {
        return try await bitcoinExecutor.estimateFee(
            address: address,
            amountSats: amountSats,
            targetBlocks: targetBlocks
        )
    }
    
    /// Estimate fee for a Lightning payment.
    public func estimateLightningFee(invoice: String) async throws -> UInt64 {
        return try await lightningExecutor.estimateFee(invoice: invoice)
    }
    
    // MARK: - Payment Lookup
    
    /// Get Bitcoin transaction by txid.
    public func getBitcoinTransaction(txid: String) async throws -> BitcoinTxResult? {
        return try await bitcoinExecutor.getTransaction(txid: txid)
    }
    
    /// Get Lightning payment by payment hash.
    public func getLightningPayment(paymentHash: String) async throws -> LightningPaymentResult? {
        return try await lightningExecutor.getPayment(paymentHash: paymentHash)
    }
    
    // MARK: - Balance Info
    
    /// Get current Bitcoin balance (mock).
    public var bitcoinBalanceSats: UInt64 {
        (bitcoinExecutor as? MockBitcoinExecutor)?.currentBalance ?? 0
    }
    
    /// Get current Lightning balance (mock).
    public var lightningBalanceSats: UInt64 {
        (lightningExecutor as? MockLightningExecutor)?.currentBalanceSats ?? 0
    }
    
    // MARK: - Private Helpers
    
    @MainActor
    private func setExecuting(_ value: Bool) {
        isExecuting = value
    }
    
    @MainActor
    private func setError(_ error: Error) {
        lastError = error
    }
    
    @MainActor
    private func addPaymentRecord(_ record: PaymentRecord) {
        recentPayments.insert(record, at: 0)
        if recentPayments.count > 100 {
            recentPayments = Array(recentPayments.prefix(100))
        }
    }
    
    private func isBitcoinAddress(_ input: String) -> Bool {
        let validPrefixes = ["1", "3", "bc1", "tb1", "bcrt1"]
        let hasValidPrefix = validPrefixes.contains { input.hasPrefix($0) }
        let hasValidLength = input.count >= 26 && input.count <= 62
        return hasValidPrefix && hasValidLength
    }
    
    private func extractAddressFromBitcoinUri(_ uri: String) -> String? {
        let stripped = uri.replacingOccurrences(of: "bitcoin:", with: "")
        if let queryIndex = stripped.firstIndex(of: "?") {
            return String(stripped[..<queryIndex])
        }
        return stripped
    }
}

// MARK: - Payment Record

/// Record of a completed or pending payment.
public struct PaymentRecord: Identifiable, Equatable {
    public let id: String
    public let type: PaymentType
    public let direction: PaymentDirection
    public let amountSats: UInt64
    public let feeSats: UInt64
    public let counterparty: String?
    public let memo: String?
    public let timestamp: Date
    public let status: PaymentStatus
    public let paymentHash: String?
    public let preimage: String?
    
    public enum PaymentType: String, Equatable {
        case bitcoin
        case lightning
    }
    
    public enum PaymentDirection: String, Equatable {
        case sent
        case received
    }
    
    public enum PaymentStatus: String, Equatable {
        case pending
        case completed
        case failed
    }
}

