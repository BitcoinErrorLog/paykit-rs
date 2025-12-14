// DirectoryE2ETests.swift
// E2E Tests for Directory Operations
//
// Tests endpoint discovery, publishing, and removal via the Pubky directory.

import XCTest
@testable import PaykitDemo

final class DirectoryE2ETests: XCTestCase {
    
    // MARK: - Properties
    
    var mockDirectoryService: MockDirectoryService!
    var mockKeyManager: MockKeyManager!
    
    // MARK: - Setup/Teardown
    
    override func setUp() {
        super.setUp()
        mockDirectoryService = MockDirectoryService()
        mockKeyManager = MockKeyManager()
    }
    
    override func tearDown() {
        mockDirectoryService?.clear()
        mockDirectoryService = nil
        mockKeyManager = nil
        super.tearDown()
    }
    
    // MARK: - Endpoint Publishing Tests
    
    /// Test publishing a Noise endpoint
    func testPublishEndpoint() throws {
        let identity = mockKeyManager.createIdentity(nickname: "publisher")
        let port = TestConfig.randomPort()
        let noisePubkey = TestDataGenerator.mockNoisePubkey()
        
        mockDirectoryService.publishEndpoint(
            pubkey: identity.publicKeyZ32,
            host: "192.168.1.100",
            port: port,
            noisePubkey: noisePubkey,
            metadata: "My payment server"
        )
        
        // Verify endpoint was published
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: identity.publicKeyZ32)
        XCTAssertNotNil(endpoint)
        XCTAssertEqual(endpoint?.host, "192.168.1.100")
        XCTAssertEqual(endpoint?.port, port)
        XCTAssertEqual(endpoint?.serverNoisePubkey, noisePubkey)
        XCTAssertEqual(endpoint?.metadata, "My payment server")
    }
    
    /// Test publishing endpoint without metadata
    func testPublishEndpointWithoutMetadata() throws {
        let identity = mockKeyManager.createIdentity(nickname: "publisher")
        
        mockDirectoryService.publishEndpoint(
            pubkey: identity.publicKeyZ32,
            host: "10.0.0.1",
            port: 8080,
            noisePubkey: TestDataGenerator.mockNoisePubkey()
        )
        
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: identity.publicKeyZ32)
        XCTAssertNotNil(endpoint)
        XCTAssertNil(endpoint?.metadata)
    }
    
    /// Test updating an existing endpoint
    func testUpdateEndpoint() throws {
        let identity = mockKeyManager.createIdentity(nickname: "publisher")
        
        // Publish initial endpoint
        mockDirectoryService.publishEndpoint(
            pubkey: identity.publicKeyZ32,
            host: "192.168.1.1",
            port: 8080,
            noisePubkey: "initial_key"
        )
        
        // Update with new endpoint
        let newPort = TestConfig.randomPort()
        mockDirectoryService.publishEndpoint(
            pubkey: identity.publicKeyZ32,
            host: "192.168.1.2",
            port: newPort,
            noisePubkey: "updated_key",
            metadata: "Updated server"
        )
        
        // Verify update
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: identity.publicKeyZ32)
        XCTAssertEqual(endpoint?.host, "192.168.1.2")
        XCTAssertEqual(endpoint?.port, newPort)
        XCTAssertEqual(endpoint?.serverNoisePubkey, "updated_key")
    }
    
    // MARK: - Endpoint Discovery Tests
    
    /// Test discovering an existing endpoint
    func testDiscoverExistingEndpoint() throws {
        let pubkey = "existing_user_pubkey"
        
        mockDirectoryService.publishEndpoint(
            pubkey: pubkey,
            host: "example.com",
            port: 9999,
            noisePubkey: TestDataGenerator.mockNoisePubkey()
        )
        
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: pubkey)
        XCTAssertNotNil(endpoint)
        PaymentAssertions.assertEndpointValid(
            endpoint!,
            expectedHost: "example.com",
            expectedPort: 9999
        )
    }
    
    /// Test discovering non-existent endpoint
    func testDiscoverNonExistentEndpoint() throws {
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: "unknown_user")
        XCTAssertNil(endpoint)
    }
    
    /// Test discovering multiple endpoints
    func testDiscoverMultipleEndpoints() throws {
        // Publish multiple endpoints
        let users = ["user1", "user2", "user3"]
        for (index, user) in users.enumerated() {
            mockDirectoryService.publishEndpoint(
                pubkey: user,
                host: "server\(index).example.com",
                port: UInt16(8000 + index),
                noisePubkey: TestDataGenerator.mockNoisePubkey()
            )
        }
        
        // Verify each can be discovered
        for (index, user) in users.enumerated() {
            let endpoint = mockDirectoryService.discoverEndpoint(pubkey: user)
            XCTAssertNotNil(endpoint, "Should find endpoint for \(user)")
            XCTAssertEqual(endpoint?.host, "server\(index).example.com")
            XCTAssertEqual(endpoint?.port, UInt16(8000 + index))
        }
    }
    
    // MARK: - Endpoint Removal Tests
    
    /// Test removing an endpoint
    func testRemoveEndpoint() throws {
        let pubkey = "removable_user"
        
        // Publish endpoint
        mockDirectoryService.publishEndpoint(
            pubkey: pubkey,
            host: "temp.example.com",
            port: 7777,
            noisePubkey: TestDataGenerator.mockNoisePubkey()
        )
        
        // Verify it exists
        XCTAssertNotNil(mockDirectoryService.discoverEndpoint(pubkey: pubkey))
        
        // Remove it
        mockDirectoryService.removeEndpoint(pubkey: pubkey)
        
        // Verify it's gone
        XCTAssertNil(mockDirectoryService.discoverEndpoint(pubkey: pubkey))
    }
    
    /// Test removing non-existent endpoint (should not error)
    func testRemoveNonExistentEndpoint() throws {
        // Should not throw
        mockDirectoryService.removeEndpoint(pubkey: "never_existed")
        
        // Verify still nil
        XCTAssertNil(mockDirectoryService.discoverEndpoint(pubkey: "never_existed"))
    }
    
    // MARK: - Endpoint Validation Tests
    
    /// Test endpoint with localhost
    func testEndpointWithLocalhost() throws {
        mockDirectoryService.publishEndpoint(
            pubkey: "local_user",
            host: "127.0.0.1",
            port: 12345,
            noisePubkey: TestDataGenerator.mockNoisePubkey()
        )
        
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: "local_user")
        XCTAssertEqual(endpoint?.host, "127.0.0.1")
    }
    
    /// Test endpoint with IPv6
    func testEndpointWithIPv6() throws {
        mockDirectoryService.publishEndpoint(
            pubkey: "ipv6_user",
            host: "::1",
            port: 8888,
            noisePubkey: TestDataGenerator.mockNoisePubkey()
        )
        
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: "ipv6_user")
        XCTAssertEqual(endpoint?.host, "::1")
    }
    
    /// Test endpoint with domain name
    func testEndpointWithDomain() throws {
        mockDirectoryService.publishEndpoint(
            pubkey: "domain_user",
            host: "payments.example.org",
            port: 443,
            noisePubkey: TestDataGenerator.mockNoisePubkey()
        )
        
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: "domain_user")
        XCTAssertEqual(endpoint?.host, "payments.example.org")
        XCTAssertEqual(endpoint?.port, 443)
    }
    
    // MARK: - Metadata Tests
    
    /// Test endpoint with long metadata
    func testEndpointWithLongMetadata() throws {
        let longMetadata = String(repeating: "A", count: 1000)
        
        mockDirectoryService.publishEndpoint(
            pubkey: "metadata_user",
            host: "meta.example.com",
            port: 5555,
            noisePubkey: TestDataGenerator.mockNoisePubkey(),
            metadata: longMetadata
        )
        
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: "metadata_user")
        XCTAssertEqual(endpoint?.metadata?.count, 1000)
    }
    
    /// Test endpoint with special characters in metadata
    func testEndpointWithSpecialMetadata() throws {
        let specialMetadata = "Server: 🚀 Payment Hub™ | v2.0 <test>"
        
        mockDirectoryService.publishEndpoint(
            pubkey: "special_user",
            host: "special.example.com",
            port: 6666,
            noisePubkey: TestDataGenerator.mockNoisePubkey(),
            metadata: specialMetadata
        )
        
        let endpoint = mockDirectoryService.discoverEndpoint(pubkey: "special_user")
        XCTAssertEqual(endpoint?.metadata, specialMetadata)
    }
    
    // MARK: - Stress Tests
    
    /// Test publishing many endpoints
    func testPublishManyEndpoints() throws {
        for i in 0..<100 {
            mockDirectoryService.publishEndpoint(
                pubkey: "user_\(i)",
                host: "server\(i).example.com",
                port: UInt16(10000 + i),
                noisePubkey: TestDataGenerator.mockNoisePubkey()
            )
        }
        
        // Verify all can be discovered
        for i in 0..<100 {
            let endpoint = mockDirectoryService.discoverEndpoint(pubkey: "user_\(i)")
            XCTAssertNotNil(endpoint, "Should find endpoint for user_\(i)")
        }
    }
    
    /// Test clearing all endpoints
    func testClearAllEndpoints() throws {
        // Publish several endpoints
        for i in 0..<10 {
            mockDirectoryService.publishEndpoint(
                pubkey: "user_\(i)",
                host: "server.example.com",
                port: UInt16(8000 + i),
                noisePubkey: TestDataGenerator.mockNoisePubkey()
            )
        }
        
        // Clear all
        mockDirectoryService.clear()
        
        // Verify all are gone
        for i in 0..<10 {
            XCTAssertNil(mockDirectoryService.discoverEndpoint(pubkey: "user_\(i)"))
        }
    }
}
