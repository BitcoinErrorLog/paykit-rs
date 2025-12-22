//
//  MockLightningExecutor.swift
//  PaykitDemo
//
//  Mock implementation of LightningExecutorProtocol for demo/testing purposes.
//  Simulates Lightning Network payments with configurable delays and success rates.
//

import Foundation
import CryptoKit

/// Mock Lightning executor for demo and testing.
/// Simulates Lightning payments with realistic delays and mock data.
public final class MockLightningExecutor: LightningExecutorProtocol {
    
    // MARK: - Configuration
    
    /// Configuration for mock behavior
    public struct Configuration {
        /// Simulated delay range in seconds
        public var delayRange: ClosedRange<Double>
        /// Success rate (0.0 to 1.0)
        public var successRate: Double
        /// Simulated channel balance in millisatoshis
        public var balanceMsat: UInt64
        /// Base routing fee in millisatoshis
        public var baseFeeMsat: UInt64
        /// Proportional routing fee (parts per million)
        public var feeRatePpm: UInt64
        
        public init(
            delayRange: ClosedRange<Double> = 0.3...1.5,
            successRate: Double = 0.98,
            balanceMsat: UInt64 = 1_000_000_000, // 1M sats
            baseFeeMsat: UInt64 = 1000,
            feeRatePpm: UInt64 = 100
        ) {
            self.delayRange = delayRange
            self.successRate = successRate
            self.balanceMsat = balanceMsat
            self.baseFeeMsat = baseFeeMsat
            self.feeRatePpm = feeRatePpm
        }
        
        public static let `default` = Configuration()
        public static let alwaysSucceed = Configuration(successRate: 1.0)
        public static let alwaysFail = Configuration(successRate: 0.0)
        public static let instant = Configuration(delayRange: 0...0)
    }
    
    // MARK: - Properties
    
    public var configuration: Configuration
    private var payments: [String: MockPayment] = [:]
    private let queue = DispatchQueue(label: "com.paykit.demo.mocklightning")
    
    // MARK: - Initialization
    
    public init(configuration: Configuration = .default) {
        self.configuration = configuration
    }
    
    // MARK: - LightningExecutorProtocol Implementation
    
    public func payInvoice(
        invoice: String,
        amountMsat: UInt64?,
        maxFeeMsat: UInt64?
    ) async throws -> LightningPaymentResult {
        // Decode the invoice (or use mock data)
        let decoded = try decodeInvoice(invoice: invoice)
        
        // Determine amount
        let paymentAmountMsat = amountMsat ?? decoded.amountMsat ?? 0
        guard paymentAmountMsat > 0 else {
            throw PaymentExecutorError.invalidInvoice("Amount required for zero-amount invoice")
        }
        
        // Calculate fee
        let feeMsat = calculateFee(amountMsat: paymentAmountMsat)
        let totalCost = paymentAmountMsat + feeMsat
        
        // Check balance
        guard totalCost <= configuration.balanceMsat else {
            throw PaymentExecutorError.insufficientFunds
        }
        
        // Check max fee
        if let maxFee = maxFeeMsat, feeMsat > maxFee {
            throw PaymentExecutorError.paymentFailed("Fee \(feeMsat) exceeds max fee \(maxFee)")
        }
        
        // Simulate network delay
        try await simulateDelay()
        
        // Check if payment should succeed
        guard Double.random(in: 0...1) <= configuration.successRate else {
            // Store failed payment
            let payment = MockPayment(
                paymentHash: decoded.paymentHash,
                preimage: nil,
                amountMsat: paymentAmountMsat,
                feeMsat: 0,
                status: .failed,
                timestamp: Date()
            )
            queue.sync { payments[decoded.paymentHash] = payment }
            
            throw PaymentExecutorError.paymentFailed("Simulated payment failure - no route found")
        }
        
        // Generate mock preimage
        let preimage = generateMockPreimage()
        
        // Store successful payment
        let payment = MockPayment(
            paymentHash: decoded.paymentHash,
            preimage: preimage,
            amountMsat: paymentAmountMsat,
            feeMsat: feeMsat,
            status: .succeeded,
            timestamp: Date()
        )
        
        queue.sync {
            payments[decoded.paymentHash] = payment
        }
        
        // Deduct from balance
        configuration.balanceMsat -= totalCost
        
        return LightningPaymentResult(
            preimage: preimage,
            paymentHash: decoded.paymentHash,
            amountMsat: paymentAmountMsat,
            feeMsat: feeMsat,
            hops: UInt32.random(in: 1...4),
            status: .succeeded
        )
    }
    
    public func decodeInvoice(invoice: String) throws -> DecodedInvoice {
        // Check for valid invoice prefix
        let lowercased = invoice.lowercased()
        guard lowercased.hasPrefix("lnbc") || 
              lowercased.hasPrefix("lntb") || 
              lowercased.hasPrefix("lnbcrt") else {
            throw PaymentExecutorError.invalidInvoice("Invalid invoice prefix")
        }
        
        // Generate deterministic mock payment hash from invoice
        let invoiceData = Data(invoice.utf8)
        let hash = SHA256.hash(data: invoiceData)
        let paymentHash = hash.compactMap { String(format: "%02x", $0) }.joined()
        
        // Extract amount from invoice (mock parsing)
        let amountMsat = extractAmount(from: invoice)
        
        return DecodedInvoice(
            paymentHash: paymentHash,
            amountMsat: amountMsat,
            description: "Mock invoice payment",
            descriptionHash: nil,
            payee: "mock_payee_\(String(paymentHash.prefix(8)))",
            expiry: 3600,
            timestamp: UInt64(Date().timeIntervalSince1970),
            isExpired: false
        )
    }
    
    public func estimateFee(invoice: String) async throws -> UInt64 {
        let decoded = try decodeInvoice(invoice: invoice)
        let amountMsat = decoded.amountMsat ?? 100_000_000 // Default 100k sats
        return calculateFee(amountMsat: amountMsat)
    }
    
    public func getPayment(paymentHash: String) async throws -> LightningPaymentResult? {
        return queue.sync {
            guard let payment = payments[paymentHash] else {
                return nil
            }
            
            return LightningPaymentResult(
                preimage: payment.preimage ?? "",
                paymentHash: paymentHash,
                amountMsat: payment.amountMsat,
                feeMsat: payment.feeMsat,
                hops: 2,
                status: payment.status
            )
        }
    }
    
    public func verifyPreimage(preimage: String, paymentHash: String) -> Bool {
        guard let preimageData = Data(hexString: preimage) else {
            return false
        }
        
        let hash = SHA256.hash(data: preimageData)
        let computedHash = hash.compactMap { String(format: "%02x", $0) }.joined()
        
        return computedHash.lowercased() == paymentHash.lowercased()
    }
    
    // MARK: - Helper Methods
    
    private func simulateDelay() async throws {
        let delay = Double.random(in: configuration.delayRange)
        if delay > 0 {
            try await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000))
        }
    }
    
    private func calculateFee(amountMsat: UInt64) -> UInt64 {
        let proportionalFee = (amountMsat * configuration.feeRatePpm) / 1_000_000
        return configuration.baseFeeMsat + proportionalFee
    }
    
    private func generateMockPreimage() -> String {
        // Generate 32 random bytes as preimage
        let randomBytes = (0..<32).map { _ in UInt8.random(in: 0...255) }
        return randomBytes.map { String(format: "%02x", $0) }.joined()
    }
    
    private func extractAmount(from invoice: String) -> UInt64? {
        // Mock amount extraction based on invoice characters
        // Real implementation would parse BOLT11
        let lowercased = invoice.lowercased()
        
        // Remove prefix
        var remaining = lowercased
        if remaining.hasPrefix("lnbcrt") {
            remaining = String(remaining.dropFirst(6))
        } else if remaining.hasPrefix("lnbc") {
            remaining = String(remaining.dropFirst(4))
        } else if remaining.hasPrefix("lntb") {
            remaining = String(remaining.dropFirst(4))
        }
        
        // Try to extract amount
        var amountStr = ""
        var multiplier: UInt64 = 1
        
        for char in remaining {
            if char.isNumber {
                amountStr.append(char)
            } else if char == "m" {
                multiplier = 100_000_000_000 // milli-BTC to msat
                break
            } else if char == "u" {
                multiplier = 100_000_000 // micro-BTC to msat
                break
            } else if char == "n" {
                multiplier = 100_000 // nano-BTC to msat
                break
            } else if char == "p" {
                multiplier = 100 // pico-BTC to msat
                break
            } else {
                break
            }
        }
        
        guard let amount = UInt64(amountStr), amount > 0 else {
            return nil
        }
        
        return amount * multiplier
    }
    
    // MARK: - Test Helpers
    
    /// Get current mock balance in msat
    public var currentBalanceMsat: UInt64 {
        configuration.balanceMsat
    }
    
    /// Get current mock balance in sats
    public var currentBalanceSats: UInt64 {
        configuration.balanceMsat / 1000
    }
    
    /// Set mock balance for testing (in msat)
    public func setBalance(_ msat: UInt64) {
        configuration.balanceMsat = msat
    }
    
    /// Clear all stored payments
    public func clearPayments() {
        queue.sync {
            payments.removeAll()
        }
    }
    
    /// Get all stored payments
    public func getAllPayments() -> [String: MockPayment] {
        queue.sync { payments }
    }
}

// MARK: - Mock Payment

/// Represents a mock payment stored by the executor
public struct MockPayment {
    public let paymentHash: String
    public let preimage: String?
    public let amountMsat: UInt64
    public let feeMsat: UInt64
    public let status: LightningPaymentStatus
    public let timestamp: Date
}

// MARK: - Data Extension

private extension Data {
    init?(hexString: String) {
        let hex = hexString.dropFirst(hexString.hasPrefix("0x") ? 2 : 0)
        guard hex.count % 2 == 0 else { return nil }
        
        var data = Data(capacity: hex.count / 2)
        var index = hex.startIndex
        while index < hex.endIndex {
            let nextIndex = hex.index(index, offsetBy: 2)
            guard let byte = UInt8(hex[index..<nextIndex], radix: 16) else { return nil }
            data.append(byte)
            index = nextIndex
        }
        self = data
    }
}

