//
//  MockBitcoinExecutor.swift
//  PaykitDemo
//
//  Mock implementation of BitcoinExecutorProtocol for demo/testing purposes.
//  Simulates on-chain Bitcoin payments with configurable delays and success rates.
//

import Foundation
import CryptoKit

/// Mock Bitcoin executor for demo and testing.
/// Simulates on-chain transactions with realistic delays and mock data.
public final class MockBitcoinExecutor: BitcoinExecutorProtocol {
    
    // MARK: - Configuration
    
    /// Configuration for mock behavior
    public struct Configuration {
        /// Simulated delay range in seconds
        public var delayRange: ClosedRange<Double>
        /// Success rate (0.0 to 1.0)
        public var successRate: Double
        /// Simulated balance in satoshis
        public var balanceSats: UInt64
        /// Base fee rate in sat/vB
        public var baseFeeRate: Double
        
        public init(
            delayRange: ClosedRange<Double> = 0.5...2.0,
            successRate: Double = 0.95,
            balanceSats: UInt64 = 1_000_000,
            baseFeeRate: Double = 2.0
        ) {
            self.delayRange = delayRange
            self.successRate = successRate
            self.balanceSats = balanceSats
            self.baseFeeRate = baseFeeRate
        }
        
        public static let `default` = Configuration()
        public static let alwaysSucceed = Configuration(successRate: 1.0)
        public static let alwaysFail = Configuration(successRate: 0.0)
        public static let instant = Configuration(delayRange: 0...0)
    }
    
    // MARK: - Properties
    
    public var configuration: Configuration
    private var transactions: [String: MockTransaction] = [:]
    private let queue = DispatchQueue(label: "com.paykit.demo.mockbitcoin")
    
    // MARK: - Initialization
    
    public init(configuration: Configuration = .default) {
        self.configuration = configuration
    }
    
    // MARK: - BitcoinExecutorProtocol Implementation
    
    public func sendToAddress(
        address: String,
        amountSats: UInt64,
        feeRate: Double?
    ) async throws -> BitcoinTxResult {
        // Validate address (basic check)
        guard isValidBitcoinAddress(address) else {
            throw PaymentExecutorError.invalidAddress(address)
        }
        
        // Check balance
        let effectiveFeeRate = feeRate ?? configuration.baseFeeRate
        let estimatedFee = UInt64(250.0 * effectiveFeeRate) // ~250 vbytes typical
        let totalCost = amountSats + estimatedFee
        
        guard totalCost <= configuration.balanceSats else {
            throw PaymentExecutorError.insufficientFunds
        }
        
        // Simulate network delay
        try await simulateDelay()
        
        // Check if payment should succeed
        guard Double.random(in: 0...1) <= configuration.successRate else {
            throw PaymentExecutorError.paymentFailed("Simulated transaction failure")
        }
        
        // Generate mock transaction
        let txid = generateMockTxid()
        let tx = MockTransaction(
            txid: txid,
            address: address,
            amountSats: amountSats,
            feeSats: estimatedFee,
            feeRate: effectiveFeeRate,
            timestamp: Date()
        )
        
        // Store transaction
        queue.sync {
            transactions[txid] = tx
        }
        
        // Deduct from balance
        configuration.balanceSats -= totalCost
        
        return BitcoinTxResult(
            txid: txid,
            rawTx: nil,
            vout: 0,
            feeSats: estimatedFee,
            feeRate: effectiveFeeRate,
            blockHeight: nil,
            confirmations: 0
        )
    }
    
    public func estimateFee(
        address: String,
        amountSats: UInt64,
        targetBlocks: UInt32
    ) async throws -> UInt64 {
        // Simulate brief delay for fee estimation
        try await Task.sleep(nanoseconds: 100_000_000) // 0.1 seconds
        
        // Calculate fee based on target blocks
        let feeMultiplier: Double
        switch targetBlocks {
        case 1:
            feeMultiplier = 3.0 // High priority
        case 2...6:
            feeMultiplier = 2.0 // Medium priority
        default:
            feeMultiplier = 1.0 // Low priority
        }
        
        let feeRate = configuration.baseFeeRate * feeMultiplier
        let txSize: UInt64 = 140 // Typical P2WPKH size
        
        return UInt64(Double(txSize) * feeRate)
    }
    
    public func getTransaction(txid: String) async throws -> BitcoinTxResult? {
        return queue.sync {
            guard let tx = transactions[txid] else {
                return nil
            }
            
            // Simulate confirmations based on time elapsed
            let elapsed = Date().timeIntervalSince(tx.timestamp)
            let confirmations = min(UInt32(elapsed / 600.0), 6) // ~10 min per block
            
            return BitcoinTxResult(
                txid: tx.txid,
                rawTx: nil,
                vout: 0,
                feeSats: tx.feeSats,
                feeRate: tx.feeRate,
                blockHeight: confirmations > 0 ? 100000 : nil,
                confirmations: confirmations
            )
        }
    }
    
    public func verifyTransaction(
        txid: String,
        address: String,
        amountSats: UInt64
    ) async throws -> Bool {
        return queue.sync {
            guard let tx = transactions[txid] else {
                return false
            }
            return tx.address == address && tx.amountSats == amountSats
        }
    }
    
    // MARK: - Helper Methods
    
    private func simulateDelay() async throws {
        let delay = Double.random(in: configuration.delayRange)
        if delay > 0 {
            try await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000))
        }
    }
    
    private func generateMockTxid() -> String {
        // Generate a realistic-looking txid (64 hex chars)
        let randomBytes = (0..<32).map { _ in UInt8.random(in: 0...255) }
        return randomBytes.map { String(format: "%02x", $0) }.joined()
    }
    
    private func isValidBitcoinAddress(_ address: String) -> Bool {
        // Basic validation - check prefix and length
        let validPrefixes = ["1", "3", "bc1", "tb1", "bcrt1"]
        let hasValidPrefix = validPrefixes.contains { address.hasPrefix($0) }
        let hasValidLength = address.count >= 26 && address.count <= 62
        return hasValidPrefix && hasValidLength
    }
    
    // MARK: - Test Helpers
    
    /// Get current mock balance
    public var currentBalance: UInt64 {
        configuration.balanceSats
    }
    
    /// Set mock balance for testing
    public func setBalance(_ sats: UInt64) {
        configuration.balanceSats = sats
    }
    
    /// Clear all stored transactions
    public func clearTransactions() {
        queue.sync {
            transactions.removeAll()
        }
    }
    
    /// Get all stored transactions
    public func getAllTransactions() -> [String: MockTransaction] {
        queue.sync { transactions }
    }
}

// MARK: - Mock Transaction

/// Represents a mock transaction stored by the executor
public struct MockTransaction {
    public let txid: String
    public let address: String
    public let amountSats: UInt64
    public let feeSats: UInt64
    public let feeRate: Double
    public let timestamp: Date
}

