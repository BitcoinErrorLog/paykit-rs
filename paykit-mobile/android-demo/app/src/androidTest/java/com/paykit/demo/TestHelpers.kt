// TestHelpers.kt
// Test Helpers for PaykitDemo E2E Tests
//
// Provides common utilities, mock services, and test fixtures for E2E testing.

package com.paykit.demo

import java.util.UUID
import java.util.Date

// MARK: - Test Configuration

/**
 * Configuration for E2E tests
 */
object TestConfig {
    /** Test timeout for async operations (ms) */
    const val DEFAULT_TIMEOUT_MS = 30000L
    
    /** Short timeout for quick operations (ms) */
    const val SHORT_TIMEOUT_MS = 5000L
    
    /** Generate a random available port */
    fun randomPort(): Int = (10000..60000).random()
    
    /** Test user identifiers */
    fun testUserAlice() = "alice_test_${UUID.randomUUID().toString().take(8)}"
    fun testUserBob() = "bob_test_${UUID.randomUUID().toString().take(8)}"
}

// MARK: - Mock Key Manager

/**
 * Mock key manager for testing
 */
class MockKeyManager {
    private val identities = mutableMapOf<String, MockIdentity>()
    private var currentIdentity: String? = null
    
    data class MockIdentity(
        val nickname: String,
        val publicKeyZ32: String = "z32_${UUID.randomUUID().toString().replace("-", "").take(52)}",
        val secretKey: ByteArray = ByteArray(32) { (0..255).random().toByte() }
    ) {
        override fun equals(other: Any?): Boolean {
            if (this === other) return true
            if (javaClass != other?.javaClass) return false
            other as MockIdentity
            return nickname == other.nickname && publicKeyZ32 == other.publicKeyZ32
        }
        
        override fun hashCode(): Int {
            var result = nickname.hashCode()
            result = 31 * result + publicKeyZ32.hashCode()
            return result
        }
    }
    
    fun createIdentity(nickname: String): MockIdentity {
        val identity = MockIdentity(nickname = nickname)
        identities[nickname] = identity
        return identity
    }
    
    fun setCurrentIdentity(nickname: String): Boolean {
        if (!identities.containsKey(nickname)) return false
        currentIdentity = nickname
        return true
    }
    
    fun getCurrentIdentity(): MockIdentity? {
        return currentIdentity?.let { identities[it] }
    }
    
    fun getPublicKey(): String? {
        return getCurrentIdentity()?.publicKeyZ32
    }
}

// MARK: - Mock Receipt Store

/**
 * Mock receipt store for testing
 */
class MockReceiptStore {
    private val receipts = mutableMapOf<String, MockReceipt>()
    
    data class MockReceipt(
        val id: String,
        val payerPubkey: String,
        val payeePubkey: String,
        val amountSats: Long,
        val createdAt: Date = Date(),
        val status: String
    )
    
    fun storeReceipt(receipt: MockReceipt) {
        receipts[receipt.id] = receipt
    }
    
    fun getReceipt(id: String): MockReceipt? {
        return receipts[id]
    }
    
    fun getAllReceipts(): List<MockReceipt> {
        return receipts.values.toList()
    }
    
    fun clear() {
        receipts.clear()
    }
}

// MARK: - Mock Directory Service

/**
 * Mock directory service for testing endpoint discovery
 */
class MockDirectoryService {
    private val endpoints = mutableMapOf<String, MockEndpoint>()
    
    data class MockEndpoint(
        val recipientPubkey: String,
        val host: String,
        val port: Int,
        val serverNoisePubkey: String,
        val metadata: String? = null
    )
    
    fun publishEndpoint(
        pubkey: String,
        host: String,
        port: Int,
        noisePubkey: String,
        metadata: String? = null
    ) {
        endpoints[pubkey] = MockEndpoint(
            recipientPubkey = pubkey,
            host = host,
            port = port,
            serverNoisePubkey = noisePubkey,
            metadata = metadata
        )
    }
    
    fun discoverEndpoint(pubkey: String): MockEndpoint? {
        return endpoints[pubkey]
    }
    
    fun removeEndpoint(pubkey: String) {
        endpoints.remove(pubkey)
    }
    
    fun clear() {
        endpoints.clear()
    }
}

// MARK: - Test Data Generators

/**
 * Generates test data for E2E tests
 */
object TestDataGenerator {
    
    /**
     * Generate a valid test payment request
     */
    fun createPaymentRequest(
        from: String = "test_payer",
        to: String = "test_payee",
        amount: Long = 1000
    ): Pair<String, Map<String, Any>> {
        val receiptId = "rcpt_test_${UUID.randomUUID()}"
        val request = mapOf(
            "receipt_id" to receiptId,
            "payer_pubkey" to from,
            "payee_pubkey" to to,
            "method_id" to "lightning",
            "amount_sats" to amount,
            "created_at" to java.text.SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'").format(Date())
        )
        return receiptId to request
    }
    
    /**
     * Generate a valid test receipt confirmation
     */
    fun createReceiptConfirmation(
        receiptId: String,
        payee: String = "test_payee"
    ): Map<String, Any> {
        return mapOf(
            "receipt_id" to receiptId,
            "confirmed_at" to java.text.SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'").format(Date()),
            "payee_pubkey" to payee,
            "status" to "confirmed"
        )
    }
    
    /**
     * Generate random bytes for test keys
     */
    fun randomBytes(count: Int): ByteArray {
        return ByteArray(count) { (0..255).random().toByte() }
    }
    
    /**
     * Generate a mock Noise public key (hex string)
     */
    fun mockNoisePubkey(): String {
        return randomBytes(32).joinToString("") { "%02x".format(it) }
    }
}

// MARK: - Test Errors

/**
 * Test-specific exceptions
 */
sealed class TestError(message: String) : Exception(message) {
    object Timeout : TestError("Test operation timed out")
    class SetupFailed(msg: String) : TestError("Test setup failed: $msg")
    class AssertionFailed(msg: String) : TestError("Assertion failed: $msg")
    class MockError(msg: String) : TestError("Mock error: $msg")
}

// MARK: - Network Test Helpers

/**
 * Helpers for network-based testing
 */
object NetworkTestHelpers {
    
    /**
     * Check if a port is available
     */
    fun isPortAvailable(port: Int): Boolean {
        return try {
            java.net.ServerSocket(port).use { true }
        } catch (e: Exception) {
            false
        }
    }
    
    /**
     * Find an available port
     */
    fun findAvailablePort(): Int {
        return TestConfig.randomPort()
    }
    
    /**
     * Create a local loopback address for testing
     */
    fun loopbackAddress(port: Int): String {
        return "127.0.0.1:$port"
    }
}

// MARK: - Assertion Helpers

/**
 * Custom assertions for payment tests
 */
object PaymentAssertions {
    
    /**
     * Assert that a receipt was created correctly
     */
    fun assertReceiptValid(
        receipt: MockReceiptStore.MockReceipt,
        expectedPayer: String,
        expectedPayee: String
    ) {
        assert(receipt.id.isNotEmpty()) { "Receipt ID should not be empty" }
        assert(receipt.payerPubkey == expectedPayer) { "Payer should match" }
        assert(receipt.payeePubkey == expectedPayee) { "Payee should match" }
        assert(receipt.amountSats > 0) { "Amount should be positive" }
    }
    
    /**
     * Assert that an endpoint is valid
     */
    fun assertEndpointValid(
        endpoint: MockDirectoryService.MockEndpoint,
        expectedHost: String? = null,
        expectedPort: Int? = null
    ) {
        assert(endpoint.recipientPubkey.isNotEmpty()) { "Pubkey should not be empty" }
        assert(endpoint.host.isNotEmpty()) { "Host should not be empty" }
        assert(endpoint.port > 0) { "Port should be positive" }
        assert(endpoint.serverNoisePubkey.isNotEmpty()) { "Noise pubkey should not be empty" }
        
        expectedHost?.let { assert(endpoint.host == it) }
        expectedPort?.let { assert(endpoint.port == it) }
    }
}

// MARK: - Mock Server

/**
 * Mock server configuration
 */
data class MockServerConfig(
    val port: Int,
    val noisePubkey: String,
    val maxConnections: Int = 100,
    val timeout: Long = 60000L
)

/**
 * Mock Noise server for testing
 */
class MockNoiseServer {
    var isRunning = false
        private set
    var activeConnections = 0
        private set
    private val connections = mutableSetOf<String>()
    var maxConnections = 100
    var receiptCallback: ((Map<String, Any>) -> String)? = null
    
    fun start(port: Int): Boolean {
        isRunning = true
        return true
    }
    
    fun stop() {
        isRunning = false
        connections.clear()
        activeConnections = 0
    }
    
    fun acceptConnection(): String? {
        if (!isRunning || activeConnections >= maxConnections) return null
        val clientId = UUID.randomUUID().toString()
        connections.add(clientId)
        activeConnections++
        return clientId
    }
    
    fun disconnectClient(clientId: String) {
        if (connections.remove(clientId)) {
            activeConnections--
        }
    }
    
    fun simulateClientDisconnect(clientId: String) {
        disconnectClient(clientId)
    }
    
    fun processMessage(clientId: String, messageType: String, payload: Map<String, Any>): Map<String, Any>? {
        if (!connections.contains(clientId)) return null
        
        return when (messageType) {
            "payment_request" -> {
                val callback = receiptCallback
                if (callback != null) {
                    val receiptId = callback(payload)
                    mapOf("status" to "confirmed", "receipt_id" to receiptId)
                } else {
                    mapOf(
                        "status" to "confirmed",
                        "receipt_id" to (payload["receipt_id"] ?: UUID.randomUUID().toString())
                    )
                }
            }
            else -> mapOf("status" to "error", "error" to "unknown_message_type")
        }
    }
}
