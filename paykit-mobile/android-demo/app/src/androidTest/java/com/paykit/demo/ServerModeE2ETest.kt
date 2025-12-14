// ServerModeE2ETest.kt
// E2E Tests for Server Mode
//
// Tests the server mode functionality for receiving payments,
// including server startup, client handling, and message processing.

package com.paykit.demo

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ServerModeE2ETest {
    
    // MARK: - Properties
    
    private lateinit var mockKeyManager: MockKeyManager
    private lateinit var mockDirectoryService: MockDirectoryService
    private lateinit var mockReceiptStore: MockReceiptStore
    
    // MARK: - Setup/Teardown
    
    @Before
    fun setUp() {
        mockKeyManager = MockKeyManager()
        mockDirectoryService = MockDirectoryService()
        mockReceiptStore = MockReceiptStore()
    }
    
    @After
    fun tearDown() {
        mockDirectoryService.clear()
        mockReceiptStore.clear()
    }
    
    // MARK: - Server Configuration Tests
    
    /** Test server configuration creation */
    @Test
    fun testServerConfigCreation() {
        val port = TestConfig.randomPort()
        val noisePubkey = TestDataGenerator.mockNoisePubkey()
        
        val config = MockServerConfig(
            port = port,
            noisePubkey = noisePubkey,
            maxConnections = 10,
            timeout = 30000L
        )
        
        assertEquals(port, config.port)
        assertEquals(noisePubkey, config.noisePubkey)
        assertEquals(10, config.maxConnections)
        assertEquals(30000L, config.timeout)
    }
    
    /** Test server configuration with defaults */
    @Test
    fun testServerConfigDefaults() {
        val config = MockServerConfig(
            port = 8080,
            noisePubkey = TestDataGenerator.mockNoisePubkey()
        )
        
        assertEquals(100, config.maxConnections)
        assertEquals(60000L, config.timeout)
    }
    
    // MARK: - Server Lifecycle Tests
    
    /** Test server start/stop lifecycle */
    @Test
    fun testServerLifecycle() {
        val server = MockNoiseServer()
        
        // Initially not running
        assertFalse(server.isRunning)
        
        // Start server
        val started = server.start(port = TestConfig.randomPort())
        assertTrue(started)
        assertTrue(server.isRunning)
        
        // Stop server
        server.stop()
        assertFalse(server.isRunning)
    }
    
    /** Test server restart */
    @Test
    fun testServerRestart() {
        val server = MockNoiseServer()
        val port = TestConfig.randomPort()
        
        // Start
        assertTrue(server.start(port = port))
        assertTrue(server.isRunning)
        
        // Stop
        server.stop()
        assertFalse(server.isRunning)
        
        // Restart
        assertTrue(server.start(port = port))
        assertTrue(server.isRunning)
        
        server.stop()
    }
    
    /** Test server with endpoint publishing */
    @Test
    fun testServerWithEndpointPublishing() {
        val identity = mockKeyManager.createIdentity(nickname = "server_user")
        mockKeyManager.setCurrentIdentity("server_user")
        
        val port = TestConfig.randomPort()
        val noisePubkey = TestDataGenerator.mockNoisePubkey()
        
        // Start server
        val server = MockNoiseServer()
        assertTrue(server.start(port = port))
        
        // Publish endpoint
        mockDirectoryService.publishEndpoint(
            pubkey = identity.publicKeyZ32,
            host = "127.0.0.1",
            port = port,
            noisePubkey = noisePubkey
        )
        
        // Verify endpoint is discoverable
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = identity.publicKeyZ32)
        assertNotNull(endpoint)
        assertEquals(port, endpoint?.port)
        
        server.stop()
    }
    
    // MARK: - Client Connection Tests
    
    /** Test handling single client connection */
    @Test
    fun testSingleClientConnection() {
        val server = MockNoiseServer()
        assertTrue(server.start(port = TestConfig.randomPort()))
        
        // Simulate client connection
        val clientId = server.acceptConnection()
        assertNotNull(clientId)
        assertEquals(1, server.activeConnections)
        
        // Disconnect client
        server.disconnectClient(clientId!!)
        assertEquals(0, server.activeConnections)
        
        server.stop()
    }
    
    /** Test handling multiple client connections */
    @Test
    fun testMultipleClientConnections() {
        val server = MockNoiseServer()
        assertTrue(server.start(port = TestConfig.randomPort()))
        
        // Connect multiple clients
        val clientIds = mutableListOf<String>()
        for (i in 0 until 5) {
            val clientId = server.acceptConnection()
            if (clientId != null) {
                clientIds.add(clientId)
            }
        }
        
        assertEquals(5, clientIds.size)
        assertEquals(5, server.activeConnections)
        
        // Disconnect all
        for (clientId in clientIds) {
            server.disconnectClient(clientId)
        }
        
        assertEquals(0, server.activeConnections)
        server.stop()
    }
    
    // MARK: - Message Processing Tests
    
    /** Test processing payment request message */
    @Test
    fun testProcessPaymentRequest() {
        val server = MockNoiseServer()
        assertTrue(server.start(port = TestConfig.randomPort()))
        
        val clientId = server.acceptConnection()!!
        
        // Create payment request
        val (receiptId, request) = TestDataGenerator.createPaymentRequest(
            from = "payer_pk",
            to = "payee_pk",
            amount = 1000
        )
        
        // Process request
        val response = server.processMessage(
            clientId = clientId,
            messageType = "payment_request",
            payload = request
        )
        
        assertNotNull(response)
        assertEquals("confirmed", response?.get("status"))
        assertEquals(receiptId, response?.get("receipt_id"))
        
        server.stop()
    }
    
    /** Test processing invalid message */
    @Test
    fun testProcessInvalidMessage() {
        val server = MockNoiseServer()
        assertTrue(server.start(port = TestConfig.randomPort()))
        
        val clientId = server.acceptConnection()!!
        
        // Send invalid message
        val response = server.processMessage(
            clientId = clientId,
            messageType = "unknown_type",
            payload = emptyMap()
        )
        
        assertNotNull(response)
        assertEquals("error", response?.get("status"))
        
        server.stop()
    }
    
    // MARK: - Receipt Generation Tests
    
    /** Test generating receipt for payment */
    @Test
    fun testReceiptGeneration() {
        val serverIdentity = mockKeyManager.createIdentity(nickname = "server")
        mockKeyManager.setCurrentIdentity("server")
        
        val server = MockNoiseServer()
        server.receiptCallback = { request ->
            val receipt = MockReceiptStore.MockReceipt(
                id = request["receipt_id"] as String,
                payerPubkey = request["payer_pubkey"] as String,
                payeePubkey = serverIdentity.publicKeyZ32,
                amountSats = request["amount_sats"] as Long,
                status = "confirmed"
            )
            mockReceiptStore.storeReceipt(receipt)
            receipt.id
        }
        
        assertTrue(server.start(port = TestConfig.randomPort()))
        val clientId = server.acceptConnection()!!
        
        // Process payment request
        val (receiptId, request) = TestDataGenerator.createPaymentRequest(
            from = "payer",
            to = serverIdentity.publicKeyZ32,
            amount = 500
        )
        
        server.processMessage(
            clientId = clientId,
            messageType = "payment_request",
            payload = request
        )
        
        // Verify receipt was stored
        val storedReceipt = mockReceiptStore.getReceipt(receiptId)
        assertNotNull(storedReceipt)
        assertEquals(500L, storedReceipt?.amountSats)
        
        server.stop()
    }
    
    // MARK: - Error Handling Tests
    
    /** Test server handles client disconnect gracefully */
    @Test
    fun testClientDisconnectHandling() {
        val server = MockNoiseServer()
        assertTrue(server.start(port = TestConfig.randomPort()))
        
        val clientId = server.acceptConnection()!!
        assertEquals(1, server.activeConnections)
        
        // Simulate unexpected disconnect
        server.simulateClientDisconnect(clientId)
        assertEquals(0, server.activeConnections)
        
        // Server should still be running
        assertTrue(server.isRunning)
        
        server.stop()
    }
    
    /** Test server rejects when at max connections */
    @Test
    fun testMaxConnectionsReached() {
        val server = MockNoiseServer()
        server.maxConnections = 2
        assertTrue(server.start(port = TestConfig.randomPort()))
        
        // Connect to max
        server.acceptConnection()
        server.acceptConnection()
        assertEquals(2, server.activeConnections)
        
        // Next connection should be rejected
        val rejectedId = server.acceptConnection()
        assertNull(rejectedId)
        assertEquals(2, server.activeConnections)
        
        server.stop()
    }
}
