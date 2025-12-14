// BitkitExecutorExample.swift
// Example Bitkit Wallet Integration for Paykit
//
// This file demonstrates how to implement the BitcoinExecutorFFI and
// LightningExecutorFFI interfaces to connect Bitkit's wallet to Paykit
// for real payment execution.
//
// USAGE:
//   // 1. Create PaykitClient with network configuration
//   let client = try PaykitClient.newWithNetwork(
//       bitcoinNetwork: .mainnet,
//       lightningNetwork: .mainnet
//   )
//
//   // 2. Create and register Bitkit executors
//   let bitcoinExecutor = BitkitBitcoinExecutor(wallet: bitkitWallet)
//   let lightningExecutor = BitkitLightningExecutor(node: bitkitNode)
//
//   try client.registerBitcoinExecutor(executor: bitcoinExecutor)
//   try client.registerLightningExecutor(executor: lightningExecutor)
//
//   // 3. Execute payments
//   let result = try client.executePayment(
//       methodId: "lightning",
//       endpoint: invoice,
//       amountSats: 1000,
//       metadataJson: nil
//   )

import Foundation

// MARK: - Placeholder Types
// These represent Bitkit's internal wallet/node types.
// Replace with actual Bitkit types when integrating.

/// Placeholder for Bitkit's Bitcoin wallet interface
public protocol BitkitWalletProtocol {
    func sendToAddress(address: String, amountSats: UInt64, feeRate: Double?) throws -> BitkitTransaction
    func estimateFee(address: String, amountSats: UInt64, targetBlocks: UInt32) throws -> UInt64
    func getTransaction(txid: String) throws -> BitkitTransaction?
}

/// Placeholder for Bitkit's Lightning node interface
public protocol BitkitNodeProtocol {
    func payInvoice(invoice: String, amountMsat: UInt64?, maxFeeMsat: UInt64?) throws -> BitkitLightningPayment
    func decodeInvoice(invoice: String) throws -> BitkitDecodedInvoice
    func estimateRoutingFee(invoice: String) throws -> UInt64
    func getPayment(paymentHash: String) throws -> BitkitLightningPayment?
}

/// Bitkit transaction result
public struct BitkitTransaction {
    public let txid: String
    public let rawTx: String?
    public let vout: UInt32
    public let feeSats: UInt64
    public let feeRate: Double
    public let blockHeight: UInt64?
    public let confirmations: UInt64
}

/// Bitkit Lightning payment result
public struct BitkitLightningPayment {
    public let preimage: String
    public let paymentHash: String
    public let amountMsat: UInt64
    public let feeMsat: UInt64
    public let succeeded: Bool
}

/// Bitkit decoded invoice
public struct BitkitDecodedInvoice {
    public let paymentHash: String
    public let amountMsat: UInt64?
    public let description: String?
    public let payee: String
    public let expiry: UInt64
    public let timestamp: UInt64
    public let expired: Bool
}

// MARK: - Bitcoin Executor Implementation

/// Bitkit implementation of BitcoinExecutorFFI
///
/// This class bridges Bitkit's Bitcoin wallet to Paykit's executor interface.
/// All methods are called synchronously from the Rust FFI layer.
///
/// Thread Safety: All methods must be thread-safe as they may be called
/// from any thread.
public final class BitkitBitcoinExecutor: BitcoinExecutorFFI {
    
    private let wallet: BitkitWalletProtocol
    
    /// Initialize with a Bitkit wallet instance
    ///
    /// - Parameter wallet: The Bitkit wallet to use for transactions
    public init(wallet: BitkitWalletProtocol) {
        self.wallet = wallet
    }
    
    // MARK: - BitcoinExecutorFFI Implementation
    
    /// Send Bitcoin to an address
    ///
    /// This method is called when Paykit executes an on-chain payment.
    /// It should create, sign, and broadcast a transaction.
    ///
    /// - Parameters:
    ///   - address: Destination Bitcoin address
    ///   - amountSats: Amount to send in satoshis
    ///   - feeRate: Optional fee rate in sat/vB (uses wallet default if nil)
    /// - Returns: Transaction result with txid and fee details
    public func sendToAddress(
        address: String,
        amountSats: UInt64,
        feeRate: Double?
    ) throws -> BitcoinTxResultFfi {
        do {
            let tx = try wallet.sendToAddress(
                address: address,
                amountSats: amountSats,
                feeRate: feeRate
            )
            
            return BitcoinTxResultFfi(
                txid: tx.txid,
                rawTx: tx.rawTx,
                vout: tx.vout,
                feeSats: tx.feeSats,
                feeRate: tx.feeRate,
                blockHeight: tx.blockHeight,
                confirmations: tx.confirmations
            )
        } catch {
            throw PaykitMobileError.Transport(message: "Send failed: \(error.localizedDescription)")
        }
    }
    
    /// Estimate the fee for a transaction
    ///
    /// - Parameters:
    ///   - address: Destination address (for UTXO selection)
    ///   - amountSats: Amount to send
    ///   - targetBlocks: Confirmation target (1, 3, 6, etc.)
    /// - Returns: Estimated fee in satoshis
    public func estimateFee(
        address: String,
        amountSats: UInt64,
        targetBlocks: UInt32
    ) throws -> UInt64 {
        do {
            return try wallet.estimateFee(
                address: address,
                amountSats: amountSats,
                targetBlocks: targetBlocks
            )
        } catch {
            throw PaykitMobileError.Transport(message: "Fee estimation failed: \(error.localizedDescription)")
        }
    }
    
    /// Get transaction details by txid
    ///
    /// - Parameter txid: Transaction ID (hex-encoded)
    /// - Returns: Transaction details if found
    public func getTransaction(txid: String) throws -> BitcoinTxResultFfi? {
        do {
            guard let tx = try wallet.getTransaction(txid: txid) else {
                return nil
            }
            
            return BitcoinTxResultFfi(
                txid: tx.txid,
                rawTx: tx.rawTx,
                vout: tx.vout,
                feeSats: tx.feeSats,
                feeRate: tx.feeRate,
                blockHeight: tx.blockHeight,
                confirmations: tx.confirmations
            )
        } catch {
            throw PaykitMobileError.Transport(message: "Get transaction failed: \(error.localizedDescription)")
        }
    }
    
    /// Verify a transaction matches expected address and amount
    ///
    /// - Parameters:
    ///   - txid: Transaction ID
    ///   - address: Expected destination address
    ///   - amountSats: Expected amount
    /// - Returns: true if transaction matches expectations
    public func verifyTransaction(
        txid: String,
        address: String,
        amountSats: UInt64
    ) throws -> Bool {
        do {
            guard let tx = try wallet.getTransaction(txid: txid) else {
                return false
            }
            // In a real implementation, verify the transaction outputs
            // contain the expected address and amount
            return tx.txid == txid
        } catch {
            throw PaykitMobileError.Transport(message: "Verify failed: \(error.localizedDescription)")
        }
    }
}

// MARK: - Lightning Executor Implementation

/// Bitkit implementation of LightningExecutorFFI
///
/// This class bridges Bitkit's Lightning node to Paykit's executor interface.
/// All methods are called synchronously from the Rust FFI layer.
///
/// Thread Safety: All methods must be thread-safe as they may be called
/// from any thread.
public final class BitkitLightningExecutor: LightningExecutorFFI {
    
    private let node: BitkitNodeProtocol
    
    /// Initialize with a Bitkit Lightning node instance
    ///
    /// - Parameter node: The Bitkit Lightning node to use for payments
    public init(node: BitkitNodeProtocol) {
        self.node = node
    }
    
    // MARK: - LightningExecutorFFI Implementation
    
    /// Pay a BOLT11 invoice
    ///
    /// This method is called when Paykit executes a Lightning payment.
    /// It should find a route and complete the payment.
    ///
    /// - Parameters:
    ///   - invoice: BOLT11 invoice string
    ///   - amountMsat: Amount in millisatoshis (for zero-amount invoices)
    ///   - maxFeeMsat: Maximum fee willing to pay
    /// - Returns: Payment result with preimage proof
    public func payInvoice(
        invoice: String,
        amountMsat: UInt64?,
        maxFeeMsat: UInt64?
    ) throws -> LightningPaymentResultFfi {
        do {
            let payment = try node.payInvoice(
                invoice: invoice,
                amountMsat: amountMsat,
                maxFeeMsat: maxFeeMsat
            )
            
            return LightningPaymentResultFfi(
                preimage: payment.preimage,
                paymentHash: payment.paymentHash,
                amountMsat: payment.amountMsat,
                feeMsat: payment.feeMsat,
                hops: 0, // Bitkit may not expose hop count
                status: payment.succeeded ? .succeeded : .failed
            )
        } catch {
            throw PaykitMobileError.Transport(message: "Payment failed: \(error.localizedDescription)")
        }
    }
    
    /// Decode a BOLT11 invoice
    ///
    /// - Parameter invoice: BOLT11 invoice string
    /// - Returns: Decoded invoice details
    public func decodeInvoice(invoice: String) throws -> DecodedInvoiceFfi {
        do {
            let decoded = try node.decodeInvoice(invoice: invoice)
            
            return DecodedInvoiceFfi(
                paymentHash: decoded.paymentHash,
                amountMsat: decoded.amountMsat,
                description: decoded.description,
                descriptionHash: nil,
                payee: decoded.payee,
                expiry: decoded.expiry,
                timestamp: decoded.timestamp,
                expired: decoded.expired
            )
        } catch {
            throw PaykitMobileError.Transport(message: "Decode failed: \(error.localizedDescription)")
        }
    }
    
    /// Estimate routing fee for an invoice
    ///
    /// - Parameter invoice: BOLT11 invoice
    /// - Returns: Estimated fee in millisatoshis
    public func estimateFee(invoice: String) throws -> UInt64 {
        do {
            return try node.estimateRoutingFee(invoice: invoice)
        } catch {
            throw PaykitMobileError.Transport(message: "Fee estimation failed: \(error.localizedDescription)")
        }
    }
    
    /// Get payment status by payment hash
    ///
    /// - Parameter paymentHash: Payment hash (hex-encoded)
    /// - Returns: Payment result if found
    public func getPayment(paymentHash: String) throws -> LightningPaymentResultFfi? {
        do {
            guard let payment = try node.getPayment(paymentHash: paymentHash) else {
                return nil
            }
            
            return LightningPaymentResultFfi(
                preimage: payment.preimage,
                paymentHash: payment.paymentHash,
                amountMsat: payment.amountMsat,
                feeMsat: payment.feeMsat,
                hops: 0,
                status: payment.succeeded ? .succeeded : .failed
            )
        } catch {
            throw PaykitMobileError.Transport(message: "Get payment failed: \(error.localizedDescription)")
        }
    }
    
    /// Verify preimage matches payment hash
    ///
    /// - Parameters:
    ///   - preimage: Payment preimage (hex-encoded)
    ///   - paymentHash: Payment hash (hex-encoded)
    /// - Returns: true if preimage hashes to payment hash
    public func verifyPreimage(preimage: String, paymentHash: String) -> Bool {
        // SHA256(preimage) should equal paymentHash
        // In a real implementation, compute the hash and compare
        guard let preimageData = Data(hexString: preimage) else {
            return false
        }
        
        // Compute SHA256 of preimage
        let computedHash = sha256(preimageData).hexString
        return computedHash.lowercased() == paymentHash.lowercased()
    }
}

// MARK: - Helper Extensions

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
    
    var hexString: String {
        map { String(format: "%02x", $0) }.joined()
    }
}

private func sha256(_ data: Data) -> Data {
    // In a real implementation, use CryptoKit or CommonCrypto
    // This is a placeholder that would need to be replaced
    var hash = [UInt8](repeating: 0, count: 32)
    // CC_SHA256(data.bytes, CC_LONG(data.count), &hash)
    return Data(hash)
}

// MARK: - Integration Example

/// Example showing complete Bitkit integration
///
/// This demonstrates the full flow from creating a PaykitClient
/// to executing a payment using Bitkit's wallet.
public enum BitkitPaykitIntegration {
    
    /// Initialize Paykit with Bitkit wallet
    ///
    /// Call this during app startup to configure Paykit with Bitkit executors.
    ///
    /// - Parameters:
    ///   - wallet: Bitkit Bitcoin wallet
    ///   - node: Bitkit Lightning node
    ///   - network: Network to use (mainnet/testnet)
    /// - Returns: Configured PaykitClient
    public static func createClient(
        wallet: BitkitWalletProtocol,
        node: BitkitNodeProtocol,
        network: BitcoinNetworkFfi = .mainnet
    ) throws -> PaykitClient {
        // Create client with network configuration
        let lightningNetwork: LightningNetworkFfi = switch network {
        case .mainnet: .mainnet
        case .testnet: .testnet
        case .regtest: .regtest
        }
        
        let client = try PaykitClient.newWithNetwork(
            bitcoinNetwork: network,
            lightningNetwork: lightningNetwork
        )
        
        // Register Bitkit executors
        let bitcoinExecutor = BitkitBitcoinExecutor(wallet: wallet)
        let lightningExecutor = BitkitLightningExecutor(node: node)
        
        try client.registerBitcoinExecutor(executor: bitcoinExecutor)
        try client.registerLightningExecutor(executor: lightningExecutor)
        
        return client
    }
    
    /// Execute a payment using the configured client
    ///
    /// - Parameters:
    ///   - client: Configured PaykitClient
    ///   - method: Payment method ("lightning" or "onchain")
    ///   - endpoint: Payment destination (invoice or address)
    ///   - amountSats: Amount in satoshis
    /// - Returns: Payment execution result
    public static func pay(
        client: PaykitClient,
        method: String,
        endpoint: String,
        amountSats: UInt64
    ) throws -> PaymentExecutionResult {
        let result = try client.executePayment(
            methodId: method,
            endpoint: endpoint,
            amountSats: amountSats,
            metadataJson: nil
        )
        
        if !result.success {
            throw PaykitMobileError.Transport(
                message: result.error ?? "Payment failed"
            )
        }
        
        return result
    }
    
    /// Generate and display payment proof
    ///
    /// - Parameters:
    ///   - client: PaykitClient
    ///   - result: Payment execution result
    /// - Returns: Payment proof
    public static func generateProof(
        client: PaykitClient,
        result: PaymentExecutionResult
    ) throws -> PaymentProofResult {
        return try client.generatePaymentProof(
            methodId: result.methodId,
            executionDataJson: result.executionDataJson
        )
    }
}
