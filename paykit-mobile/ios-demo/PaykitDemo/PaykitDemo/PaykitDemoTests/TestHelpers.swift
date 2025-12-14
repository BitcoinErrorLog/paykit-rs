// TestHelpers.swift
// Test Helpers for PaykitDemo E2E Tests
//
// Provides common utilities, mock services, and test fixtures for E2E testing.

import XCTest
import Foundation

// MARK: - Test Configuration

/// Configuration for E2E tests
struct TestConfig {
    /// Test timeout for async operations
    static let defaultTimeout: TimeInterval = 30.0
    
    /// Short timeout for quick operations
    static let shortTimeout: TimeInterval = 5.0
    
    /// Port range for test servers
    static let testPortRange = 10000...60000
    
    /// Test user identifiers
    static let testUserAlice = "alice_test_\(UUID().uuidString.prefix(8))"
    static let testUserBob = "bob_test_\(UUID().uuidString.prefix(8))"
    
    /// Generate a random available port
    static func randomPort() -> UInt16 {
        UInt16.random(in: 10000...60000)
    }
}

// MARK: - Mock Key Manager

/// Mock key manager for testing
class MockKeyManager {
    private var identities: [String: MockIdentity] = [:]
    private var currentIdentity: String?
    
    struct MockIdentity {
        let nickname: String
        let publicKeyZ32: String
        let secretKey: Data
        
        init(nickname: String) {
            self.nickname = nickname
            self.publicKeyZ32 = "z32_\(UUID().uuidString.replacingOccurrences(of: "-", with: "").prefix(52))"
            self.secretKey = Data((0..<32).map { _ in UInt8.random(in: 0...255) })
        }
    }
    
    func createIdentity(nickname: String) -> MockIdentity {
        let identity = MockIdentity(nickname: nickname)
        identities[nickname] = identity
        return identity
    }
    
    func setCurrentIdentity(_ nickname: String) -> Bool {
        guard identities[nickname] != nil else { return false }
        currentIdentity = nickname
        return true
    }
    
    func getCurrentIdentity() -> MockIdentity? {
        guard let current = currentIdentity else { return nil }
        return identities[current]
    }
    
    func getPublicKey() -> String? {
        return getCurrentIdentity()?.publicKeyZ32
    }
}

// MARK: - Mock Receipt Store

/// Mock receipt store for testing
class MockReceiptStore {
    private var receipts: [String: MockReceipt] = [:]
    
    struct MockReceipt {
        let id: String
        let payerPubkey: String
        let payeePubkey: String
        let amountSats: UInt64
        let createdAt: Date
        let status: String
    }
    
    func storeReceipt(_ receipt: MockReceipt) {
        receipts[receipt.id] = receipt
    }
    
    func getReceipt(_ id: String) -> MockReceipt? {
        return receipts[id]
    }
    
    func getAllReceipts() -> [MockReceipt] {
        return Array(receipts.values)
    }
    
    func clear() {
        receipts.removeAll()
    }
}

// MARK: - Mock Directory Service

/// Mock directory service for testing endpoint discovery
class MockDirectoryService {
    private var endpoints: [String: MockEndpoint] = [:]
    
    struct MockEndpoint {
        let recipientPubkey: String
        let host: String
        let port: UInt16
        let serverNoisePubkey: String
        let metadata: String?
    }
    
    func publishEndpoint(
        pubkey: String,
        host: String,
        port: UInt16,
        noisePubkey: String,
        metadata: String? = nil
    ) {
        endpoints[pubkey] = MockEndpoint(
            recipientPubkey: pubkey,
            host: host,
            port: port,
            serverNoisePubkey: noisePubkey,
            metadata: metadata
        )
    }
    
    func discoverEndpoint(pubkey: String) -> MockEndpoint? {
        return endpoints[pubkey]
    }
    
    func removeEndpoint(pubkey: String) {
        endpoints.removeValue(forKey: pubkey)
    }
    
    func clear() {
        endpoints.removeAll()
    }
}

// MARK: - Test Data Generators

/// Generates test data for E2E tests
struct TestDataGenerator {
    /// Generate a valid test payment request
    static func createPaymentRequest(
        from payer: String = "test_payer",
        to payee: String = "test_payee",
        amount: UInt64 = 1000
    ) -> (receiptId: String, request: [String: Any]) {
        let receiptId = "rcpt_test_\(UUID().uuidString)"
        let request: [String: Any] = [
            "receipt_id": receiptId,
            "payer_pubkey": payer,
            "payee_pubkey": payee,
            "method_id": "lightning",
            "amount_sats": amount,
            "created_at": ISO8601DateFormatter().string(from: Date())
        ]
        return (receiptId, request)
    }
    
    /// Generate a valid test receipt confirmation
    static func createReceiptConfirmation(
        for receiptId: String,
        payee: String = "test_payee"
    ) -> [String: Any] {
        return [
            "receipt_id": receiptId,
            "confirmed_at": ISO8601DateFormatter().string(from: Date()),
            "payee_pubkey": payee,
            "status": "confirmed"
        ]
    }
    
    /// Generate random bytes for test keys
    static func randomBytes(_ count: Int) -> Data {
        Data((0..<count).map { _ in UInt8.random(in: 0...255) })
    }
    
    /// Generate a mock Noise public key (hex string)
    static func mockNoisePubkey() -> String {
        randomBytes(32).map { String(format: "%02x", $0) }.joined()
    }
}

// MARK: - Async Test Helpers

/// Helpers for async testing
extension XCTestCase {
    /// Wait for an async operation with timeout
    func waitForAsync<T>(
        timeout: TimeInterval = TestConfig.defaultTimeout,
        operation: @escaping () async throws -> T
    ) throws -> T {
        let expectation = self.expectation(description: "Async operation")
        var result: Result<T, Error>?
        
        Task {
            do {
                let value = try await operation()
                result = .success(value)
            } catch {
                result = .failure(error)
            }
            expectation.fulfill()
        }
        
        wait(for: [expectation], timeout: timeout)
        
        switch result {
        case .success(let value):
            return value
        case .failure(let error):
            throw error
        case .none:
            throw TestError.timeout
        }
    }
    
    /// Assert that an async operation completes without error
    func assertNoThrowAsync(
        timeout: TimeInterval = TestConfig.defaultTimeout,
        _ operation: @escaping () async throws -> Void,
        file: StaticString = #file,
        line: UInt = #line
    ) {
        let expectation = self.expectation(description: "Async operation")
        var caughtError: Error?
        
        Task {
            do {
                try await operation()
            } catch {
                caughtError = error
            }
            expectation.fulfill()
        }
        
        wait(for: [expectation], timeout: timeout)
        
        if let error = caughtError {
            XCTFail("Unexpected error: \(error)", file: file, line: line)
        }
    }
}

// MARK: - Test Errors

enum TestError: Error, LocalizedError {
    case timeout
    case setupFailed(String)
    case assertionFailed(String)
    case mockError(String)
    
    var errorDescription: String? {
        switch self {
        case .timeout:
            return "Test operation timed out"
        case .setupFailed(let msg):
            return "Test setup failed: \(msg)"
        case .assertionFailed(let msg):
            return "Assertion failed: \(msg)"
        case .mockError(let msg):
            return "Mock error: \(msg)"
        }
    }
}

// MARK: - Network Test Helpers

/// Helpers for network-based testing
struct NetworkTestHelpers {
    /// Check if a port is available
    static func isPortAvailable(_ port: UInt16) -> Bool {
        // Simple check - try to create a socket
        // In real tests, this would use proper socket APIs
        return true
    }
    
    /// Find an available port
    static func findAvailablePort() -> UInt16 {
        // Return a random port from the test range
        // In production, this would actually check availability
        return TestConfig.randomPort()
    }
    
    /// Create a local loopback address for testing
    static func loopbackAddress(port: UInt16) -> String {
        "127.0.0.1:\(port)"
    }
}

// MARK: - Assertion Helpers

/// Custom assertions for payment tests
struct PaymentAssertions {
    /// Assert that a receipt was created correctly
    static func assertReceiptValid(
        _ receipt: MockReceiptStore.MockReceipt,
        expectedPayer: String,
        expectedPayee: String,
        file: StaticString = #file,
        line: UInt = #line
    ) {
        XCTAssertFalse(receipt.id.isEmpty, "Receipt ID should not be empty", file: file, line: line)
        XCTAssertEqual(receipt.payerPubkey, expectedPayer, "Payer should match", file: file, line: line)
        XCTAssertEqual(receipt.payeePubkey, expectedPayee, "Payee should match", file: file, line: line)
        XCTAssertGreaterThan(receipt.amountSats, 0, "Amount should be positive", file: file, line: line)
    }
    
    /// Assert that an endpoint is valid
    static func assertEndpointValid(
        _ endpoint: MockDirectoryService.MockEndpoint,
        expectedHost: String? = nil,
        expectedPort: UInt16? = nil,
        file: StaticString = #file,
        line: UInt = #line
    ) {
        XCTAssertFalse(endpoint.recipientPubkey.isEmpty, "Pubkey should not be empty", file: file, line: line)
        XCTAssertFalse(endpoint.host.isEmpty, "Host should not be empty", file: file, line: line)
        XCTAssertGreaterThan(endpoint.port, 0, "Port should be positive", file: file, line: line)
        XCTAssertFalse(endpoint.serverNoisePubkey.isEmpty, "Noise pubkey should not be empty", file: file, line: line)
        
        if let expectedHost = expectedHost {
            XCTAssertEqual(endpoint.host, expectedHost, file: file, line: line)
        }
        if let expectedPort = expectedPort {
            XCTAssertEqual(endpoint.port, expectedPort, file: file, line: line)
        }
    }
}
