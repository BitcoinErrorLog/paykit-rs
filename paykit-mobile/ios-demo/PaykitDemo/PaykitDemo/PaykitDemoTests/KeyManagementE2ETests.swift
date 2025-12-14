// KeyManagementE2ETests.swift
// E2E Tests for Key Management
//
// Tests key derivation, caching, and Pubky Ring integration.

import XCTest
@testable import PaykitDemo

final class KeyManagementE2ETests: XCTestCase {
    
    // MARK: - Properties
    
    var mockKeyManager: MockKeyManager!
    
    // MARK: - Setup/Teardown
    
    override func setUp() {
        super.setUp()
        mockKeyManager = MockKeyManager()
    }
    
    override func tearDown() {
        mockKeyManager = nil
        super.tearDown()
    }
    
    // MARK: - Identity Creation Tests
    
    /// Test creating a new identity
    func testCreateIdentity() throws {
        let identity = mockKeyManager.createIdentity(nickname: "test_user")
        
        XCTAssertEqual(identity.nickname, "test_user")
        XCTAssertFalse(identity.publicKeyZ32.isEmpty, "Public key should not be empty")
        XCTAssertTrue(identity.publicKeyZ32.hasPrefix("z32_"), "Public key should have z32 prefix")
        XCTAssertEqual(identity.secretKey.count, 32, "Secret key should be 32 bytes")
    }
    
    /// Test creating multiple identities
    func testCreateMultipleIdentities() throws {
        let identity1 = mockKeyManager.createIdentity(nickname: "alice")
        let identity2 = mockKeyManager.createIdentity(nickname: "bob")
        let identity3 = mockKeyManager.createIdentity(nickname: "charlie")
        
        // All should have unique public keys
        XCTAssertNotEqual(identity1.publicKeyZ32, identity2.publicKeyZ32)
        XCTAssertNotEqual(identity2.publicKeyZ32, identity3.publicKeyZ32)
        XCTAssertNotEqual(identity1.publicKeyZ32, identity3.publicKeyZ32)
        
        // All should have correct nicknames
        XCTAssertEqual(identity1.nickname, "alice")
        XCTAssertEqual(identity2.nickname, "bob")
        XCTAssertEqual(identity3.nickname, "charlie")
    }
    
    // MARK: - Identity Switching Tests
    
    /// Test switching between identities
    func testSwitchIdentity() throws {
        _ = mockKeyManager.createIdentity(nickname: "alice")
        _ = mockKeyManager.createIdentity(nickname: "bob")
        
        // Set alice as current
        let setAlice = mockKeyManager.setCurrentIdentity("alice")
        XCTAssertTrue(setAlice)
        XCTAssertEqual(mockKeyManager.getCurrentIdentity()?.nickname, "alice")
        
        // Switch to bob
        let setBob = mockKeyManager.setCurrentIdentity("bob")
        XCTAssertTrue(setBob)
        XCTAssertEqual(mockKeyManager.getCurrentIdentity()?.nickname, "bob")
    }
    
    /// Test setting non-existent identity
    func testSetNonExistentIdentity() throws {
        _ = mockKeyManager.createIdentity(nickname: "alice")
        
        let result = mockKeyManager.setCurrentIdentity("nonexistent")
        XCTAssertFalse(result, "Should fail to set non-existent identity")
    }
    
    /// Test no current identity initially
    func testNoCurrentIdentityInitially() throws {
        let current = mockKeyManager.getCurrentIdentity()
        XCTAssertNil(current, "Should have no current identity initially")
    }
    
    // MARK: - Public Key Tests
    
    /// Test getting public key
    func testGetPublicKey() throws {
        let identity = mockKeyManager.createIdentity(nickname: "test")
        _ = mockKeyManager.setCurrentIdentity("test")
        
        let publicKey = mockKeyManager.getPublicKey()
        XCTAssertNotNil(publicKey)
        XCTAssertEqual(publicKey, identity.publicKeyZ32)
    }
    
    /// Test public key format
    func testPublicKeyFormat() throws {
        let identity = mockKeyManager.createIdentity(nickname: "test")
        
        // Should be z32 format (base32 with specific prefix)
        XCTAssertTrue(identity.publicKeyZ32.hasPrefix("z32_"))
        XCTAssertGreaterThan(identity.publicKeyZ32.count, 10)
    }
    
    // MARK: - Secret Key Tests
    
    /// Test secret key generation
    func testSecretKeyGeneration() throws {
        let identity = mockKeyManager.createIdentity(nickname: "test")
        
        XCTAssertEqual(identity.secretKey.count, 32, "Ed25519 secret key should be 32 bytes")
        
        // Check it's not all zeros
        let isAllZeros = identity.secretKey.allSatisfy { $0 == 0 }
        XCTAssertFalse(isAllZeros, "Secret key should not be all zeros")
    }
    
    /// Test secret keys are unique
    func testSecretKeysAreUnique() throws {
        let identity1 = mockKeyManager.createIdentity(nickname: "alice")
        let identity2 = mockKeyManager.createIdentity(nickname: "bob")
        
        XCTAssertNotEqual(identity1.secretKey, identity2.secretKey)
    }
    
    // MARK: - Key Derivation Tests
    
    /// Test Noise key derivation (mock)
    func testNoiseKeyDerivation() throws {
        // In real implementation, this would use HKDF-SHA512
        let identity = mockKeyManager.createIdentity(nickname: "test")
        
        // Simulate deriving Noise keypair from Ed25519 key
        let noisePrivateKey = deriveNoiseKey(from: identity.secretKey)
        
        XCTAssertEqual(noisePrivateKey.count, 32, "Noise private key should be 32 bytes")
        XCTAssertNotEqual(noisePrivateKey, identity.secretKey, "Noise key should differ from identity key")
    }
    
    /// Helper: Derive Noise key from identity key (mock implementation)
    private func deriveNoiseKey(from secretKey: Data) -> Data {
        // In real implementation: HKDF-SHA512(secretKey, salt, info)
        // Here we just XOR with a constant for testing
        var derived = Data(count: 32)
        for i in 0..<32 {
            derived[i] = secretKey[i] ^ 0x42
        }
        return derived
    }
    
    // MARK: - Key Caching Tests
    
    /// Test key caching behavior (mock)
    func testKeyCaching() throws {
        // Create identity
        let identity = mockKeyManager.createIdentity(nickname: "cached_user")
        _ = mockKeyManager.setCurrentIdentity("cached_user")
        
        // First access
        let key1 = mockKeyManager.getPublicKey()
        
        // Second access should return same key (cached)
        let key2 = mockKeyManager.getPublicKey()
        
        XCTAssertEqual(key1, key2, "Cached key should be consistent")
        XCTAssertEqual(key1, identity.publicKeyZ32)
    }
    
    // MARK: - Stress Tests
    
    /// Test creating many identities
    func testCreateManyIdentities() throws {
        var identities: [MockKeyManager.MockIdentity] = []
        
        for i in 0..<100 {
            let identity = mockKeyManager.createIdentity(nickname: "user_\(i)")
            identities.append(identity)
        }
        
        // All should have unique public keys
        let publicKeys = Set(identities.map { $0.publicKeyZ32 })
        XCTAssertEqual(publicKeys.count, 100, "All public keys should be unique")
    }
    
    /// Test rapid identity switching
    func testRapidIdentitySwitching() throws {
        // Create identities
        for i in 0..<10 {
            _ = mockKeyManager.createIdentity(nickname: "user_\(i)")
        }
        
        // Rapidly switch between them
        for _ in 0..<100 {
            let randomUser = "user_\(Int.random(in: 0..<10))"
            let success = mockKeyManager.setCurrentIdentity(randomUser)
            XCTAssertTrue(success)
            
            let current = mockKeyManager.getCurrentIdentity()
            XCTAssertNotNil(current)
            XCTAssertEqual(current?.nickname, randomUser)
        }
    }
}
