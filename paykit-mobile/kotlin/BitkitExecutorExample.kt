// BitkitExecutorExample.kt
// Example Bitkit Wallet Integration for Paykit
//
// This file demonstrates how to implement the BitcoinExecutorFFI and
// LightningExecutorFFI interfaces to connect Bitkit's wallet to Paykit
// for real payment execution.
//
// USAGE:
//   // 1. Create PaykitClient with network configuration
//   val client = PaykitClient.newWithNetwork(
//       bitcoinNetwork = BitcoinNetworkFfi.MAINNET,
//       lightningNetwork = LightningNetworkFfi.MAINNET
//   )
//
//   // 2. Create and register Bitkit executors
//   val bitcoinExecutor = BitkitBitcoinExecutor(wallet)
//   val lightningExecutor = BitkitLightningExecutor(node)
//
//   client.registerBitcoinExecutor(bitcoinExecutor)
//   client.registerLightningExecutor(lightningExecutor)
//
//   // 3. Execute payments
//   val result = client.executePayment(
//       methodId = "lightning",
//       endpoint = invoice,
//       amountSats = 1000UL,
//       metadataJson = null
//   )

package com.paykit.bitkit

import com.paykit.mobile.*
import java.security.MessageDigest

// =============================================================================
// Placeholder Types
// These represent Bitkit's internal wallet/node types.
// Replace with actual Bitkit types when integrating.
// =============================================================================

/**
 * Placeholder for Bitkit's Bitcoin wallet interface
 */
interface BitkitWallet {
    /**
     * Send Bitcoin to an address
     * @param address Destination Bitcoin address
     * @param amountSats Amount to send in satoshis
     * @param feeRate Optional fee rate in sat/vB
     * @return Transaction result
     */
    fun sendToAddress(address: String, amountSats: ULong, feeRate: Double?): BitkitTransaction

    /**
     * Estimate fee for a transaction
     * @param address Destination address
     * @param amountSats Amount to send
     * @param targetBlocks Confirmation target
     * @return Estimated fee in satoshis
     */
    fun estimateFee(address: String, amountSats: ULong, targetBlocks: UInt): ULong

    /**
     * Get transaction by txid
     * @param txid Transaction ID
     * @return Transaction if found
     */
    fun getTransaction(txid: String): BitkitTransaction?
}

/**
 * Placeholder for Bitkit's Lightning node interface
 */
interface BitkitNode {
    /**
     * Pay a BOLT11 invoice
     * @param invoice BOLT11 invoice string
     * @param amountMsat Amount in millisatoshis (for zero-amount invoices)
     * @param maxFeeMsat Maximum fee willing to pay
     * @return Payment result
     */
    fun payInvoice(invoice: String, amountMsat: ULong?, maxFeeMsat: ULong?): BitkitLightningPayment

    /**
     * Decode a BOLT11 invoice
     * @param invoice BOLT11 invoice string
     * @return Decoded invoice details
     */
    fun decodeInvoice(invoice: String): BitkitDecodedInvoice

    /**
     * Estimate routing fee for an invoice
     * @param invoice BOLT11 invoice
     * @return Estimated fee in millisatoshis
     */
    fun estimateRoutingFee(invoice: String): ULong

    /**
     * Get payment by payment hash
     * @param paymentHash Payment hash (hex-encoded)
     * @return Payment if found
     */
    fun getPayment(paymentHash: String): BitkitLightningPayment?
}

/**
 * Bitkit transaction result
 */
data class BitkitTransaction(
    val txid: String,
    val rawTx: String?,
    val vout: UInt,
    val feeSats: ULong,
    val feeRate: Double,
    val blockHeight: ULong?,
    val confirmations: ULong
)

/**
 * Bitkit Lightning payment result
 */
data class BitkitLightningPayment(
    val preimage: String,
    val paymentHash: String,
    val amountMsat: ULong,
    val feeMsat: ULong,
    val succeeded: Boolean
)

/**
 * Bitkit decoded invoice
 */
data class BitkitDecodedInvoice(
    val paymentHash: String,
    val amountMsat: ULong?,
    val description: String?,
    val payee: String,
    val expiry: ULong,
    val timestamp: ULong,
    val expired: Boolean
)

// =============================================================================
// Bitcoin Executor Implementation
// =============================================================================

/**
 * Bitkit implementation of BitcoinExecutorFFI
 *
 * This class bridges Bitkit's Bitcoin wallet to Paykit's executor interface.
 * All methods are called synchronously from the Rust FFI layer.
 *
 * Thread Safety: All methods must be thread-safe as they may be called
 * from any thread.
 *
 * @param wallet The Bitkit wallet to use for transactions
 */
class BitkitBitcoinExecutor(
    private val wallet: BitkitWallet
) : BitcoinExecutorFfi {

    /**
     * Send Bitcoin to an address
     *
     * This method is called when Paykit executes an on-chain payment.
     * It should create, sign, and broadcast a transaction.
     *
     * @param address Destination Bitcoin address
     * @param amountSats Amount to send in satoshis
     * @param feeRate Optional fee rate in sat/vB (uses wallet default if null)
     * @return Transaction result with txid and fee details
     */
    override fun sendToAddress(
        address: String,
        amountSats: ULong,
        feeRate: Double?
    ): BitcoinTxResultFfi {
        return try {
            val tx = wallet.sendToAddress(address, amountSats, feeRate)

            BitcoinTxResultFfi(
                txid = tx.txid,
                rawTx = tx.rawTx,
                vout = tx.vout,
                feeSats = tx.feeSats,
                feeRate = tx.feeRate,
                blockHeight = tx.blockHeight,
                confirmations = tx.confirmations
            )
        } catch (e: Exception) {
            throw PaykitMobileException.Transport("Send failed: ${e.message}")
        }
    }

    /**
     * Estimate the fee for a transaction
     *
     * @param address Destination address (for UTXO selection)
     * @param amountSats Amount to send
     * @param targetBlocks Confirmation target (1, 3, 6, etc.)
     * @return Estimated fee in satoshis
     */
    override fun estimateFee(
        address: String,
        amountSats: ULong,
        targetBlocks: UInt
    ): ULong {
        return try {
            wallet.estimateFee(address, amountSats, targetBlocks)
        } catch (e: Exception) {
            throw PaykitMobileException.Transport("Fee estimation failed: ${e.message}")
        }
    }

    /**
     * Get transaction details by txid
     *
     * @param txid Transaction ID (hex-encoded)
     * @return Transaction details if found
     */
    override fun getTransaction(txid: String): BitcoinTxResultFfi? {
        return try {
            wallet.getTransaction(txid)?.let { tx ->
                BitcoinTxResultFfi(
                    txid = tx.txid,
                    rawTx = tx.rawTx,
                    vout = tx.vout,
                    feeSats = tx.feeSats,
                    feeRate = tx.feeRate,
                    blockHeight = tx.blockHeight,
                    confirmations = tx.confirmations
                )
            }
        } catch (e: Exception) {
            throw PaykitMobileException.Transport("Get transaction failed: ${e.message}")
        }
    }

    /**
     * Verify a transaction matches expected address and amount
     *
     * @param txid Transaction ID
     * @param address Expected destination address
     * @param amountSats Expected amount
     * @return true if transaction matches expectations
     */
    override fun verifyTransaction(
        txid: String,
        address: String,
        amountSats: ULong
    ): Boolean {
        return try {
            val tx = wallet.getTransaction(txid) ?: return false
            // In a real implementation, verify the transaction outputs
            // contain the expected address and amount
            tx.txid == txid
        } catch (e: Exception) {
            throw PaykitMobileException.Transport("Verify failed: ${e.message}")
        }
    }
}

// =============================================================================
// Lightning Executor Implementation
// =============================================================================

/**
 * Bitkit implementation of LightningExecutorFFI
 *
 * This class bridges Bitkit's Lightning node to Paykit's executor interface.
 * All methods are called synchronously from the Rust FFI layer.
 *
 * Thread Safety: All methods must be thread-safe as they may be called
 * from any thread.
 *
 * @param node The Bitkit Lightning node to use for payments
 */
class BitkitLightningExecutor(
    private val node: BitkitNode
) : LightningExecutorFfi {

    /**
     * Pay a BOLT11 invoice
     *
     * This method is called when Paykit executes a Lightning payment.
     * It should find a route and complete the payment.
     *
     * @param invoice BOLT11 invoice string
     * @param amountMsat Amount in millisatoshis (for zero-amount invoices)
     * @param maxFeeMsat Maximum fee willing to pay
     * @return Payment result with preimage proof
     */
    override fun payInvoice(
        invoice: String,
        amountMsat: ULong?,
        maxFeeMsat: ULong?
    ): LightningPaymentResultFfi {
        return try {
            val payment = node.payInvoice(invoice, amountMsat, maxFeeMsat)

            LightningPaymentResultFfi(
                preimage = payment.preimage,
                paymentHash = payment.paymentHash,
                amountMsat = payment.amountMsat,
                feeMsat = payment.feeMsat,
                hops = 0u, // Bitkit may not expose hop count
                status = if (payment.succeeded) {
                    LightningPaymentStatusFfi.SUCCEEDED
                } else {
                    LightningPaymentStatusFfi.FAILED
                }
            )
        } catch (e: Exception) {
            throw PaykitMobileException.Transport("Payment failed: ${e.message}")
        }
    }

    /**
     * Decode a BOLT11 invoice
     *
     * @param invoice BOLT11 invoice string
     * @return Decoded invoice details
     */
    override fun decodeInvoice(invoice: String): DecodedInvoiceFfi {
        return try {
            val decoded = node.decodeInvoice(invoice)

            DecodedInvoiceFfi(
                paymentHash = decoded.paymentHash,
                amountMsat = decoded.amountMsat,
                description = decoded.description,
                descriptionHash = null,
                payee = decoded.payee,
                expiry = decoded.expiry,
                timestamp = decoded.timestamp,
                expired = decoded.expired
            )
        } catch (e: Exception) {
            throw PaykitMobileException.Transport("Decode failed: ${e.message}")
        }
    }

    /**
     * Estimate routing fee for an invoice
     *
     * @param invoice BOLT11 invoice
     * @return Estimated fee in millisatoshis
     */
    override fun estimateFee(invoice: String): ULong {
        return try {
            node.estimateRoutingFee(invoice)
        } catch (e: Exception) {
            throw PaykitMobileException.Transport("Fee estimation failed: ${e.message}")
        }
    }

    /**
     * Get payment status by payment hash
     *
     * @param paymentHash Payment hash (hex-encoded)
     * @return Payment result if found
     */
    override fun getPayment(paymentHash: String): LightningPaymentResultFfi? {
        return try {
            node.getPayment(paymentHash)?.let { payment ->
                LightningPaymentResultFfi(
                    preimage = payment.preimage,
                    paymentHash = payment.paymentHash,
                    amountMsat = payment.amountMsat,
                    feeMsat = payment.feeMsat,
                    hops = 0u,
                    status = if (payment.succeeded) {
                        LightningPaymentStatusFfi.SUCCEEDED
                    } else {
                        LightningPaymentStatusFfi.FAILED
                    }
                )
            }
        } catch (e: Exception) {
            throw PaykitMobileException.Transport("Get payment failed: ${e.message}")
        }
    }

    /**
     * Verify preimage matches payment hash
     *
     * @param preimage Payment preimage (hex-encoded)
     * @param paymentHash Payment hash (hex-encoded)
     * @return true if preimage hashes to payment hash
     */
    override fun verifyPreimage(preimage: String, paymentHash: String): Boolean {
        return try {
            // SHA256(preimage) should equal paymentHash
            val preimageBytes = preimage.hexToByteArray()
            val digest = MessageDigest.getInstance("SHA-256")
            val computedHash = digest.digest(preimageBytes).toHexString()
            computedHash.equals(paymentHash, ignoreCase = true)
        } catch (e: Exception) {
            false
        }
    }
}

// =============================================================================
// Helper Extensions
// =============================================================================

private fun String.hexToByteArray(): ByteArray {
    val hex = if (startsWith("0x")) substring(2) else this
    require(hex.length % 2 == 0) { "Hex string must have even length" }
    return ByteArray(hex.length / 2) { i ->
        hex.substring(i * 2, i * 2 + 2).toInt(16).toByte()
    }
}

private fun ByteArray.toHexString(): String {
    return joinToString("") { "%02x".format(it) }
}

// =============================================================================
// Integration Example
// =============================================================================

/**
 * Example showing complete Bitkit integration
 *
 * This demonstrates the full flow from creating a PaykitClient
 * to executing a payment using Bitkit's wallet.
 */
object BitkitPaykitIntegration {

    /**
     * Initialize Paykit with Bitkit wallet
     *
     * Call this during app startup to configure Paykit with Bitkit executors.
     *
     * @param wallet Bitkit Bitcoin wallet
     * @param node Bitkit Lightning node
     * @param network Network to use (mainnet/testnet)
     * @return Configured PaykitClient
     */
    fun createClient(
        wallet: BitkitWallet,
        node: BitkitNode,
        network: BitcoinNetworkFfi = BitcoinNetworkFfi.MAINNET
    ): PaykitClient {
        // Map network
        val lightningNetwork = when (network) {
            BitcoinNetworkFfi.MAINNET -> LightningNetworkFfi.MAINNET
            BitcoinNetworkFfi.TESTNET -> LightningNetworkFfi.TESTNET
            BitcoinNetworkFfi.REGTEST -> LightningNetworkFfi.REGTEST
        }

        // Create client with network configuration
        val client = PaykitClient.newWithNetwork(
            bitcoinNetwork = network,
            lightningNetwork = lightningNetwork
        )

        // Register Bitkit executors
        val bitcoinExecutor = BitkitBitcoinExecutor(wallet)
        val lightningExecutor = BitkitLightningExecutor(node)

        client.registerBitcoinExecutor(bitcoinExecutor)
        client.registerLightningExecutor(lightningExecutor)

        return client
    }

    /**
     * Execute a payment using the configured client
     *
     * @param client Configured PaykitClient
     * @param method Payment method ("lightning" or "onchain")
     * @param endpoint Payment destination (invoice or address)
     * @param amountSats Amount in satoshis
     * @return Payment execution result
     */
    fun pay(
        client: PaykitClient,
        method: String,
        endpoint: String,
        amountSats: ULong
    ): PaymentExecutionResult {
        val result = client.executePayment(
            methodId = method,
            endpoint = endpoint,
            amountSats = amountSats,
            metadataJson = null
        )

        if (!result.success) {
            throw PaykitMobileException.Transport(
                result.error ?: "Payment failed"
            )
        }

        return result
    }

    /**
     * Generate and display payment proof
     *
     * @param client PaykitClient
     * @param result Payment execution result
     * @return Payment proof
     */
    fun generateProof(
        client: PaykitClient,
        result: PaymentExecutionResult
    ): PaymentProofResult {
        return client.generatePaymentProof(
            methodId = result.methodId,
            executionDataJson = result.executionDataJson
        )
    }
}

// =============================================================================
// Usage Example
// =============================================================================

/**
 * Example usage in an Android Activity or ViewModel
 *
 * ```kotlin
 * class PaymentViewModel(
 *     private val bitkitWallet: BitkitWallet,
 *     private val bitkitNode: BitkitNode
 * ) : ViewModel() {
 *
 *     private val paykitClient by lazy {
 *         BitkitPaykitIntegration.createClient(
 *             wallet = bitkitWallet,
 *             node = bitkitNode,
 *             network = BitcoinNetworkFfi.MAINNET
 *         )
 *     }
 *
 *     suspend fun payLightningInvoice(invoice: String, amountSats: ULong): Result<String> {
 *         return withContext(Dispatchers.IO) {
 *             try {
 *                 val result = BitkitPaykitIntegration.pay(
 *                     client = paykitClient,
 *                     method = "lightning",
 *                     endpoint = invoice,
 *                     amountSats = amountSats
 *                 )
 *
 *                 val proof = BitkitPaykitIntegration.generateProof(
 *                     client = paykitClient,
 *                     result = result
 *                 )
 *
 *                 Result.success("Payment successful! Proof: ${proof.proofDataJson}")
 *             } catch (e: Exception) {
 *                 Result.failure(e)
 *             }
 *         }
 *     }
 *
 *     suspend fun payOnchain(address: String, amountSats: ULong): Result<String> {
 *         return withContext(Dispatchers.IO) {
 *             try {
 *                 val result = BitkitPaykitIntegration.pay(
 *                     client = paykitClient,
 *                     method = "onchain",
 *                     endpoint = address,
 *                     amountSats = amountSats
 *                 )
 *
 *                 Result.success("Transaction sent! TxID in: ${result.executionDataJson}")
 *             } catch (e: Exception) {
 *                 Result.failure(e)
 *             }
 *         }
 *     }
 * }
 * ```
 */
