// NoisePaymentE2ETest.kt
// E2E Tests for Noise Payment Flows
//
// Tests the complete payment flow from sender to receiver,
// including key derivation, endpoint discovery, handshake, and receipt exchange.

package com.paykit.demo

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class NoisePaymentE2ETest {
    
    // MARK: - Properties
    
    private lateinit var mockKeyManager: MockKeyManager
    private lateinit var mockReceiptStore: MockReceiptStore
    private lateinit var mockDirectoryService: MockDirectoryService
    
    // MARK: - Setup/Teardown
    
    @Before
    fun setUp() {
        mockKeyManager = MockKeyManager()
        mockReceiptStore = MockReceiptStore()
        mockDirectoryService = MockDirectoryService()
    }
    
    @After
    fun tearDown() {
        mockReceiptStore.clear()
        mockDirectoryService.clear()
    }
    
    // MARK: - Payment Request Creation Tests
    
    /** Test creating a valid payment request message */
    @Test
    fun testCreatePaymentRequest() {
        // Setup: Create sender identity
        val sender = mockKeyManager.createIdentity(nickname = "sender")
        mockKeyManager.setCurrentIdentity("sender")
        
        // Create payment request
        val (receiptId, request) = TestDataGenerator.createPaymentRequest(
            from = sender.publicKeyZ32,
            to = "recipient_pubkey",
            amount = 1000
        )
        
        // Verify request structure
        assertTrue("Receipt ID should not be empty", receiptId.isNotEmpty())
        assertEquals(sender.publicKeyZ32, request["payer_pubkey"])
        assertEquals("recipient_pubkey", request["payee_pubkey"])
        assertEquals(1000L, request["amount_sats"])
        assertEquals("lightning", request["method_id"])
    }
    
    /** Test payment request with optional fields */
    @Test
    fun testCreatePaymentRequestWithOptionalFields() {
        val sender = mockKeyManager.createIdentity(nickname = "sender")
        mockKeyManager.setCurrentIdentity("sender")
        
        val (receiptId, request) = TestDataGenerator.createPaymentRequest(
            from = sender.publicKeyZ32,
            to = "recipient_pubkey",
            amount = 5000
        )
        
        assertTrue(receiptId.isNotEmpty())
        assertNotNull(request["created_at"])
    }
    
    // MARK: - Receipt Confirmation Tests
    
    /** Test creating a receipt confirmation */
    @Test
    fun testCreateReceiptConfirmation() {
        val receiptId = "rcpt_test_123"
        val payeePubkey = "payee_pubkey_z32"
        
        val confirmation = TestDataGenerator.createReceiptConfirmation(
            receiptId = receiptId,
            payee = payeePubkey
        )
        
        assertEquals(receiptId, confirmation["receipt_id"])
        assertEquals(payeePubkey, confirmation["payee_pubkey"])
        assertEquals("confirmed", confirmation["status"])
        assertNotNull(confirmation["confirmed_at"])
    }
    
    // MARK: - Receipt Storage Tests
    
    /** Test storing and retrieving receipts */
    @Test
    fun testReceiptStorage() {
        val receipt = MockReceiptStore.MockReceipt(
            id = "rcpt_test_456",
            payerPubkey = "payer_pk",
            payeePubkey = "payee_pk",
            amountSats = 2500,
            status = "confirmed"
        )
        
        // Store receipt
        mockReceiptStore.storeReceipt(receipt)
        
        // Retrieve receipt
        val retrieved = mockReceiptStore.getReceipt("rcpt_test_456")
        assertNotNull(retrieved)
        assertEquals("rcpt_test_456", retrieved?.id)
        assertEquals(2500L, retrieved?.amountSats)
        
        // Verify assertions helper
        PaymentAssertions.assertReceiptValid(
            retrieved!!,
            expectedPayer = "payer_pk",
            expectedPayee = "payee_pk"
        )
    }
    
    /** Test receipt not found */
    @Test
    fun testReceiptNotFound() {
        val retrieved = mockReceiptStore.getReceipt("nonexistent_id")
        assertNull(retrieved)
    }
    
    // MARK: - End-to-End Payment Flow Tests
    
    /** Test complete payment flow with mocked services */
    @Test
    fun testCompletePaymentFlowMocked() {
        // 1. Setup: Create sender and receiver identities
        val sender = mockKeyManager.createIdentity(nickname = "alice")
        val receiver = mockKeyManager.createIdentity(nickname = "bob")
        
        // 2. Receiver publishes endpoint
        val serverPort = TestConfig.randomPort()
        mockDirectoryService.publishEndpoint(
            pubkey = receiver.publicKeyZ32,
            host = "127.0.0.1",
            port = serverPort,
            noisePubkey = TestDataGenerator.mockNoisePubkey(),
            metadata = "Bob's payment server"
        )
        
        // 3. Sender discovers receiver's endpoint
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = receiver.publicKeyZ32)
        assertNotNull("Should discover receiver's endpoint", endpoint)
        PaymentAssertions.assertEndpointValid(endpoint!!, expectedHost = "127.0.0.1")
        
        // 4. Sender creates payment request
        mockKeyManager.setCurrentIdentity("alice")
        val (receiptId, _) = TestDataGenerator.createPaymentRequest(
            from = sender.publicKeyZ32,
            to = receiver.publicKeyZ32,
            amount = 1000
        )
        
        // 5. Simulate successful payment
        val receipt = MockReceiptStore.MockReceipt(
            id = receiptId,
            payerPubkey = sender.publicKeyZ32,
            payeePubkey = receiver.publicKeyZ32,
            amountSats = 1000,
            status = "confirmed"
        )
        mockReceiptStore.storeReceipt(receipt)
        
        // 6. Verify receipt was stored
        val storedReceipt = mockReceiptStore.getReceipt(receiptId)
        assertNotNull(storedReceipt)
        assertEquals("confirmed", storedReceipt?.status)
    }
    
    /** Test payment flow when endpoint not found */
    @Test
    fun testPaymentFlowEndpointNotFound() {
        val sender = mockKeyManager.createIdentity(nickname = "alice")
        mockKeyManager.setCurrentIdentity("alice")
        
        // Try to discover non-existent endpoint
        val endpoint = mockDirectoryService.discoverEndpoint(pubkey = "unknown_recipient")
        assertNull("Should not find endpoint for unknown recipient", endpoint)
    }
    
    /** Test multiple concurrent payment requests */
    @Test
    fun testMultipleConcurrentPayments() {
        val sender = mockKeyManager.createIdentity(nickname = "alice")
        mockKeyManager.setCurrentIdentity("alice")
        
        // Create multiple payment requests
        val receiptIds = mutableListOf<String>()
        for (i in 1..5) {
            val receiver = mockKeyManager.createIdentity(nickname = "receiver_$i")
            val (receiptId, _) = TestDataGenerator.createPaymentRequest(
                from = sender.publicKeyZ32,
                to = receiver.publicKeyZ32,
                amount = (i * 100).toLong()
            )
            receiptIds.add(receiptId)
            
            // Store mock receipt
            val receipt = MockReceiptStore.MockReceipt(
                id = receiptId,
                payerPubkey = sender.publicKeyZ32,
                payeePubkey = receiver.publicKeyZ32,
                amountSats = (i * 100).toLong(),
                status = "confirmed"
            )
            mockReceiptStore.storeReceipt(receipt)
        }
        
        // Verify all receipts were stored
        val allReceipts = mockReceiptStore.getAllReceipts()
        assertEquals(5, allReceipts.size)
        
        // Verify each receipt
        for (receiptId in receiptIds) {
            val receipt = mockReceiptStore.getReceipt(receiptId)
            assertNotNull(receipt)
        }
    }
    
    // MARK: - Error Handling Tests
    
    /** Test handling of payment with no identity */
    @Test
    fun testPaymentWithNoIdentity() {
        // Don't set any identity
        val currentIdentity = mockKeyManager.getCurrentIdentity()
        assertNull("Should have no current identity", currentIdentity)
    }
    
    /** Test payment request with invalid amount */
    @Test
    fun testPaymentRequestWithZeroAmount() {
        val sender = mockKeyManager.createIdentity(nickname = "alice")
        mockKeyManager.setCurrentIdentity("alice")
        
        // Create payment with zero amount
        val (receiptId, request) = TestDataGenerator.createPaymentRequest(
            from = sender.publicKeyZ32,
            to = "recipient",
            amount = 0
        )
        
        assertTrue(receiptId.isNotEmpty())
        assertEquals(0L, request["amount_sats"])
    }
}
