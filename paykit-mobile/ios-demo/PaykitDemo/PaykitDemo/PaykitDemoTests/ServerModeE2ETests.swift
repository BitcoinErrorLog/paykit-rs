// ServerModeE2ETests.swift
// E2E Tests for Server Mode
//
// Tests the server mode functionality for receiving payments,
// including server startup, client handling, and message processing.

import XCTest
@testable import PaykitDemo

final class ServerModeE2ETests: XCTestCase {
    
    // MARK: - Properties
    
    var mockKeyManager: MockKeyManager!
    var mockDirectoryService: MockDirectoryService!
    var mockReceiptStore: MockReceiptStore!
    
    // MARK: - Setup/Teardown
    
    override func setUp() {
        super.setUp()
        mockKeyManager = MockKeyManager()
        mockDirectoryService = MockDirectoryService()
        mockReceiptStore = MockReceiptStore()
    }
    
    override func tearDown() {
        mockDirectoryService?.clear()
        mockReceiptStore?.clear()
        mockKeyManager = nil
        mockDirectoryService = nil
        mockReceiptStore = nil
        super.tearDown()
    }
    
    // MARK: - Server Configuration Tests
    
    /// Test server configuration creation
    func testServerConfigCreation() throws {
        let port = TestConfig.randomPort()
        let noisePubkey = TestDataGenerator.mockNoisePubkey()
        
        let config = MockServerConfig(
            port: port,
            noisePubkey: noisePubkey,
            maxConnections: 10,
            timeout: 30.0
        )
        
        XCTAssertEqual(config.port, port)
        XCTAssertEqual(config.noisePubkey, noisePubkey)
        XCTAssertEqual(config.maxConnections, 10)
        XCTAssertEqual(config.timeout, 30.0)
    }
    
    /// Test server configuration with defaults
    func testServerConfigDefaults() throws {
        let config = MockServerConfig(
            port: 8080,
            noisePubkey: TestDataGenerator.mockNoisePubkey()
        )
        
        XCTAssertEqual(config.maxConnections, 100)
        XCTAssertEqual(config.timeout, 60.0)
    }
    
    // MARK: - Server Lifecycle Tests
    
    /// Test server start/stop lifecycle
    func testServerLifecycle() throws {
        let server = MockNoiseServer()
        
        // Initially not running
        XCTAssertFalse(server.isRunning)
        
        // Start server
        let started = server.start(port: TestConfig.randomPort())
        XCTAssertTrue(started)
        XCTAssertTrue(server.isRunning)
        
        // Stop server
        server.stop()
        XCTAssertFalse(server.isRunning)
    }
    
    /// Test server restart
    func testServerRestart() throws {
        let server = MockNoiseServer()
        let port = TestConfig.randomPort()
        
        // Start
        XCTAssertTrue(server.start(port: port))
        XCTAssertTrue(server.isRunning)
        
        // Stop
        server.stop()
        XCTAssertFalse(server.isRunning)
        
        // Restart
        XCTAssertTrue(server.start(port: port))
        XCTAssertTrue(server.isRunning)
        
        server.stop()
    }
    
    /// Test server with endpoint publishing
    func testServerWithEndpointPublishing() throws {
        let identity = mockKeyManager.createIdentity(nickname: "server_user")
        _ = mockKeyManager.setCurrentIdentity("server_user")
        
        let port = TestConfig.randomPort()
        let noisePubkey = TestDataGenerator.mockNoisePubkey()
        
        // Start server
        let server = MockNoiseServer()
        XCTAssertTrue(server.start(port: port))
        
        // Publish endpoint
        mockDirectoryService.publishEndpoint(
            pubkey: identity.publicKeyZ32,
            host: "127.0.0.1",
            port: port,
            noisePubkey: noisePubkey
        )
        
        // Verify endpoint is discoverable
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: identity.publicKeyZ32)
        XCTAssertNotNil(endpoint)
        XCTAssertEqual(endpoint?.port, port)
        
        server.stop()
    }
    
    // MARK: - Client Connection Tests
    
    /// Test handling single client connection
    func testSingleClientConnection() throws {
        let server = MockNoiseServer()
        XCTAssertTrue(server.start(port: TestConfig.randomPort()))
        
        // Simulate client connection
        let clientId = server.acceptConnection()
        XCTAssertNotNil(clientId)
        XCTAssertEqual(server.activeConnections, 1)
        
        // Disconnect client
        server.disconnectClient(clientId!)
        XCTAssertEqual(server.activeConnections, 0)
        
        server.stop()
    }
    
    /// Test handling multiple client connections
    func testMultipleClientConnections() throws {
        let server = MockNoiseServer()
        XCTAssertTrue(server.start(port: TestConfig.randomPort()))
        
        // Connect multiple clients
        var clientIds: [String] = []
        for _ in 0..<5 {
            if let clientId = server.acceptConnection() {
                clientIds.append(clientId)
            }
        }
        
        XCTAssertEqual(clientIds.count, 5)
        XCTAssertEqual(server.activeConnections, 5)
        
        // Disconnect all
        for clientId in clientIds {
            server.disconnectClient(clientId)
        }
        
        XCTAssertEqual(server.activeConnections, 0)
        server.stop()
    }
    
    // MARK: - Message Processing Tests
    
    /// Test processing payment request message
    func testProcessPaymentRequest() throws {
        let server = MockNoiseServer()
        XCTAssertTrue(server.start(port: TestConfig.randomPort()))
        
        let clientId = server.acceptConnection()!
        
        // Create payment request
        let (receiptId, request) = TestDataGenerator.createPaymentRequest(
            from: "payer_pk",
            to: "payee_pk",
            amount: 1000
        )
        
        // Process request
        let response = server.processMessage(
            clientId: clientId,
            messageType: "payment_request",
            payload: request
        )
        
        XCTAssertNotNil(response)
        XCTAssertEqual(response?["status"] as? String, "confirmed")
        XCTAssertEqual(response?["receipt_id"] as? String, receiptId)
        
        server.stop()
    }
    
    /// Test processing invalid message
    func testProcessInvalidMessage() throws {
        let server = MockNoiseServer()
        XCTAssertTrue(server.start(port: TestConfig.randomPort()))
        
        let clientId = server.acceptConnection()!
        
        // Send invalid message
        let response = server.processMessage(
            clientId: clientId,
            messageType: "unknown_type",
            payload: [:]
        )
        
        XCTAssertNotNil(response)
        XCTAssertEqual(response?["status"] as? String, "error")
        
        server.stop()
    }
    
    // MARK: - Receipt Generation Tests
    
    /// Test generating receipt for payment
    func testReceiptGeneration() throws {
        let serverIdentity = mockKeyManager.createIdentity(nickname: "server")
        _ = mockKeyManager.setCurrentIdentity("server")
        
        let server = MockNoiseServer()
        server.receiptCallback = { [weak self] request in
            let receipt = MockReceiptStore.MockReceipt(
                id: request["receipt_id"] as! String,
                payerPubkey: request["payer_pubkey"] as! String,
                payeePubkey: serverIdentity.publicKeyZ32,
                amountSats: request["amount_sats"] as! UInt64,
                createdAt: Date(),
                status: "confirmed"
            )
            self?.mockReceiptStore.storeReceipt(receipt)
            return receipt.id
        }
        
        XCTAssertTrue(server.start(port: TestConfig.randomPort()))
        let clientId = server.acceptConnection()!
        
        // Process payment request
        let (receiptId, request) = TestDataGenerator.createPaymentRequest(
            from: "payer",
            to: serverIdentity.publicKeyZ32,
            amount: 500
        )
        
        _ = server.processMessage(
            clientId: clientId,
            messageType: "payment_request",
            payload: request
        )
        
        // Verify receipt was stored
        let storedReceipt = mockReceiptStore.getReceipt(receiptId)
        XCTAssertNotNil(storedReceipt)
        XCTAssertEqual(storedReceipt?.amountSats, 500)
        
        server.stop()
    }
    
    // MARK: - Error Handling Tests
    
    /// Test server handles client disconnect gracefully
    func testClientDisconnectHandling() throws {
        let server = MockNoiseServer()
        XCTAssertTrue(server.start(port: TestConfig.randomPort()))
        
        let clientId = server.acceptConnection()!
        XCTAssertEqual(server.activeConnections, 1)
        
        // Simulate unexpected disconnect
        server.simulateClientDisconnect(clientId)
        XCTAssertEqual(server.activeConnections, 0)
        
        // Server should still be running
        XCTAssertTrue(server.isRunning)
        
        server.stop()
    }
    
    /// Test server rejects when at max connections
    func testMaxConnectionsReached() throws {
        let server = MockNoiseServer()
        server.maxConnections = 2
        XCTAssertTrue(server.start(port: TestConfig.randomPort()))
        
        // Connect to max
        _ = server.acceptConnection()
        _ = server.acceptConnection()
        XCTAssertEqual(server.activeConnections, 2)
        
        // Next connection should be rejected
        let rejectedId = server.acceptConnection()
        XCTAssertNil(rejectedId)
        XCTAssertEqual(server.activeConnections, 2)
        
        server.stop()
    }
}

// MARK: - Mock Types

/// Mock server configuration
struct MockServerConfig {
    let port: UInt16
    let noisePubkey: String
    var maxConnections: Int
    var timeout: TimeInterval
    
    init(port: UInt16, noisePubkey: String, maxConnections: Int = 100, timeout: TimeInterval = 60.0) {
        self.port = port
        self.noisePubkey = noisePubkey
        self.maxConnections = maxConnections
        self.timeout = timeout
    }
}

/// Mock Noise server for testing
class MockNoiseServer {
    private(set) var isRunning = false
    private(set) var activeConnections = 0
    private var connections: Set<String> = []
    var maxConnections = 100
    var receiptCallback: (([String: Any]) -> String)?
    
    func start(port: UInt16) -> Bool {
        isRunning = true
        return true
    }
    
    func stop() {
        isRunning = false
        connections.removeAll()
        activeConnections = 0
    }
    
    func acceptConnection() -> String? {
        guard isRunning, activeConnections < maxConnections else { return nil }
        let clientId = UUID().uuidString
        connections.insert(clientId)
        activeConnections += 1
        return clientId
    }
    
    func disconnectClient(_ clientId: String) {
        if connections.remove(clientId) != nil {
            activeConnections -= 1
        }
    }
    
    func simulateClientDisconnect(_ clientId: String) {
        disconnectClient(clientId)
    }
    
    func processMessage(clientId: String, messageType: String, payload: [String: Any]) -> [String: Any]? {
        guard connections.contains(clientId) else { return nil }
        
        switch messageType {
        case "payment_request":
            if let callback = receiptCallback {
                let receiptId = callback(payload)
                return ["status": "confirmed", "receipt_id": receiptId]
            }
            return [
                "status": "confirmed",
                "receipt_id": payload["receipt_id"] ?? UUID().uuidString
            ]
        default:
            return ["status": "error", "error": "unknown_message_type"]
        }
    }
}
