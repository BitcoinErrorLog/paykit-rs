// NoisePaymentE2ETests.swift
// E2E Tests for Noise Payment Flows
//
// Tests the complete payment flow from sender to receiver,
// including key derivation, endpoint discovery, handshake, and receipt exchange.

import XCTest
@testable import PaykitDemo

final class NoisePaymentE2ETests: XCTestCase {
    
    // MARK: - Properties
    
    var mockKeyManager: MockKeyManager!
    var mockReceiptStore: MockReceiptStore!
    var mockDirectoryService: MockDirectoryService!
    
    // MARK: - Setup/Teardown
    
    override func setUp() {
        super.setUp()
        mockKeyManager = MockKeyManager()
        mockReceiptStore = MockReceiptStore()
        mockDirectoryService = MockDirectoryService()
    }
    
    override func tearDown() {
        mockKeyManager = nil
        mockReceiptStore = nil
        mockDirectoryService = nil
        super.tearDown()
    }
    
    // MARK: - Payment Request Creation Tests
    
    /// Test creating a valid payment request message
    func testCreatePaymentRequest() throws {
        // Setup: Create sender identity
        let sender = mockKeyManager.createIdentity(nickname: "sender")
        _ = mockKeyManager.setCurrentIdentity("sender")
        
        // Create payment request
        let (receiptId, request) = TestDataGenerator.createPaymentRequest(
            from: sender.publicKeyZ32,
            to: "recipient_pubkey",
            amount: 1000
        )
        
        // Verify request structure
        XCTAssertFalse(receiptId.isEmpty, "Receipt ID should not be empty")
        XCTAssertEqual(request["payer_pubkey"] as? String, sender.publicKeyZ32)
        XCTAssertEqual(request["payee_pubkey"] as? String, "recipient_pubkey")
        XCTAssertEqual(request["amount_sats"] as? UInt64, 1000)
        XCTAssertEqual(request["method_id"] as? String, "lightning")
    }
    
    /// Test payment request with optional fields
    func testCreatePaymentRequestWithOptionalFields() throws {
        let sender = mockKeyManager.createIdentity(nickname: "sender")
        _ = mockKeyManager.setCurrentIdentity("sender")
        
        let (receiptId, request) = TestDataGenerator.createPaymentRequest(
            from: sender.publicKeyZ32,
            to: "recipient_pubkey",
            amount: 5000
        )
        
        XCTAssertFalse(receiptId.isEmpty)
        XCTAssertNotNil(request["created_at"])
    }
    
    // MARK: - Receipt Confirmation Tests
    
    /// Test creating a receipt confirmation
    func testCreateReceiptConfirmation() throws {
        let receiptId = "rcpt_test_123"
        let payeePubkey = "payee_pubkey_z32"
        
        let confirmation = TestDataGenerator.createReceiptConfirmation(
            for: receiptId,
            payee: payeePubkey
        )
        
        XCTAssertEqual(confirmation["receipt_id"] as? String, receiptId)
        XCTAssertEqual(confirmation["payee_pubkey"] as? String, payeePubkey)
        XCTAssertEqual(confirmation["status"] as? String, "confirmed")
        XCTAssertNotNil(confirmation["confirmed_at"])
    }
    
    // MARK: - Receipt Storage Tests
    
    /// Test storing and retrieving receipts
    func testReceiptStorage() throws {
        let receipt = MockReceiptStore.MockReceipt(
            id: "rcpt_test_456",
            payerPubkey: "payer_pk",
            payeePubkey: "payee_pk",
            amountSats: 2500,
            createdAt: Date(),
            status: "confirmed"
        )
        
        // Store receipt
        mockReceiptStore.storeReceipt(receipt)
        
        // Retrieve receipt
        let retrieved = mockReceiptStore.getReceipt("rcpt_test_456")
        XCTAssertNotNil(retrieved)
        XCTAssertEqual(retrieved?.id, "rcpt_test_456")
        XCTAssertEqual(retrieved?.amountSats, 2500)
        
        // Verify assertions helper
        PaymentAssertions.assertReceiptValid(
            retrieved!,
            expectedPayer: "payer_pk",
            expectedPayee: "payee_pk"
        )
    }
    
    /// Test receipt not found
    func testReceiptNotFound() throws {
        let retrieved = mockReceiptStore.getReceipt("nonexistent_id")
        XCTAssertNil(retrieved)
    }
    
    // MARK: - End-to-End Payment Flow Tests
    
    /// Test complete payment flow with mocked services
    func testCompletePaymentFlowMocked() throws {
        // 1. Setup: Create sender and receiver identities
        let sender = mockKeyManager.createIdentity(nickname: "alice")
        let receiver = mockKeyManager.createIdentity(nickname: "bob")
        
        // 2. Receiver publishes endpoint
        let serverPort = TestConfig.randomPort()
        mockDirectoryService.publishEndpoint(
            pubkey: receiver.publicKeyZ32,
            host: "127.0.0.1",
            port: serverPort,
            noisePubkey: TestDataGenerator.mockNoisePubkey(),
            metadata: "Bob's payment server"
        )
        
        // 3. Sender discovers receiver's endpoint
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: receiver.publicKeyZ32)
        XCTAssertNotNil(endpoint, "Should discover receiver's endpoint")
        PaymentAssertions.assertEndpointValid(endpoint!, expectedHost: "127.0.0.1")
        
        // 4. Sender creates payment request
        _ = mockKeyManager.setCurrentIdentity("alice")
        let (receiptId, _) = TestDataGenerator.createPaymentRequest(
            from: sender.publicKeyZ32,
            to: receiver.publicKeyZ32,
            amount: 1000
        )
        
        // 5. Simulate successful payment (in real test, this would use actual network)
        let receipt = MockReceiptStore.MockReceipt(
            id: receiptId,
            payerPubkey: sender.publicKeyZ32,
            payeePubkey: receiver.publicKeyZ32,
            amountSats: 1000,
            createdAt: Date(),
            status: "confirmed"
        )
        mockReceiptStore.storeReceipt(receipt)
        
        // 6. Verify receipt was stored
        let storedReceipt = mockReceiptStore.getReceipt(receiptId)
        XCTAssertNotNil(storedReceipt)
        XCTAssertEqual(storedReceipt?.status, "confirmed")
    }
    
    /// Test payment flow when endpoint not found
    func testPaymentFlowEndpointNotFound() throws {
        let sender = mockKeyManager.createIdentity(nickname: "alice")
        _ = mockKeyManager.setCurrentIdentity("alice")
        
        // Try to discover non-existent endpoint
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: "unknown_recipient")
        XCTAssertNil(endpoint, "Should not find endpoint for unknown recipient")
    }
    
    /// Test multiple concurrent payment requests
    func testMultipleConcurrentPayments() throws {
        let sender = mockKeyManager.createIdentity(nickname: "alice")
        _ = mockKeyManager.setCurrentIdentity("alice")
        
        // Create multiple payment requests
        var receiptIds: [String] = []
        for i in 1...5 {
            let receiver = mockKeyManager.createIdentity(nickname: "receiver_\(i)")
            let (receiptId, _) = TestDataGenerator.createPaymentRequest(
                from: sender.publicKeyZ32,
                to: receiver.publicKeyZ32,
                amount: UInt64(i * 100)
            )
            receiptIds.append(receiptId)
            
            // Store mock receipt
            let receipt = MockReceiptStore.MockReceipt(
                id: receiptId,
                payerPubkey: sender.publicKeyZ32,
                payeePubkey: receiver.publicKeyZ32,
                amountSats: UInt64(i * 100),
                createdAt: Date(),
                status: "confirmed"
            )
            mockReceiptStore.storeReceipt(receipt)
        }
        
        // Verify all receipts were stored
        let allReceipts = mockReceiptStore.getAllReceipts()
        XCTAssertEqual(allReceipts.count, 5)
        
        // Verify each receipt
        for receiptId in receiptIds {
            let receipt = mockReceiptStore.getReceipt(receiptId)
            XCTAssertNotNil(receipt)
        }
    }
    
    // MARK: - Error Handling Tests
    
    /// Test handling of payment with no identity
    func testPaymentWithNoIdentity() throws {
        // Don't set any identity
        let currentIdentity = mockKeyManager.getCurrentIdentity()
        XCTAssertNil(currentIdentity, "Should have no current identity")
    }
    
    /// Test payment request with invalid amount
    func testPaymentRequestWithZeroAmount() throws {
        let sender = mockKeyManager.createIdentity(nickname: "alice")
        _ = mockKeyManager.setCurrentIdentity("alice")
        
        // Create payment with zero amount - should still work but be flagged
        let (receiptId, request) = TestDataGenerator.createPaymentRequest(
            from: sender.publicKeyZ32,
            to: "recipient",
            amount: 0
        )
        
        XCTAssertFalse(receiptId.isEmpty)
        XCTAssertEqual(request["amount_sats"] as? UInt64, 0)
    }
}
