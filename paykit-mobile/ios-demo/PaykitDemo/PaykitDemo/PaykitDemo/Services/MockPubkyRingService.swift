// MockPubkyRingService.swift
// Mock Pubky Ring Service for Demo/Testing
//
// This service simulates Pubky Ring for demo and testing purposes.
// In production, Paykit apps would request key derivation from the real
// Pubky Ring app via deep links/URL schemes.
//
// Key Architecture:
//   - Ed25519 (pkarr) keys are "cold" and managed by Pubky Ring
//   - X25519 (Noise) keys are "hot" and derived on-demand
//   - This mock service provides Ed25519 seed for demo compatibility
//
// For production, use PubkyRingIntegration instead.

import Foundation

/// X25519 keypair for Noise protocol encryption
public struct X25519KeypairResult: Codable {
    public let secretKeyHex: String
    public let publicKeyHex: String
    public let deviceId: String
    public let epoch: UInt32
    
    public init(secretKeyHex: String, publicKeyHex: String, deviceId: String, epoch: UInt32) {
        self.secretKeyHex = secretKeyHex
        self.publicKeyHex = publicKeyHex
        self.deviceId = deviceId
        self.epoch = epoch
    }
}

/// Error types for mock Pubky Ring service
public enum MockPubkyRingError: LocalizedError {
    case noSeedAvailable
    case derivationFailed(String)
    case invalidSeedFormat
    case keychainError(String)
    
    public var errorDescription: String? {
        switch self {
        case .noSeedAvailable:
            return "No Ed25519 seed available. Initialize the mock service first."
        case .derivationFailed(let message):
            return "Key derivation failed: \(message)"
        case .invalidSeedFormat:
            return "Invalid seed format. Seed must be 32 bytes hex-encoded."
        case .keychainError(let message):
            return "Keychain error: \(message)"
        }
    }
}

/// Mock Pubky Ring service for demo and testing
///
/// This service simulates the key derivation functionality that would
/// normally be provided by the Pubky Ring app. It stores an Ed25519
/// seed in the keychain and derives X25519 keys on-demand.
///
/// **DEMO ONLY**: In production, use PubkyRingIntegration to communicate
/// with the real Pubky Ring app.
public final class MockPubkyRingService {
    
    // MARK: - Singleton
    
    public static let shared = MockPubkyRingService()
    
    // MARK: - Constants
    
    private enum Keys {
        static let mockSeed = "mock.pubkyring.ed25519.seed"
        static let mockPublicKey = "mock.pubkyring.ed25519.public"
        static let mockPublicKeyZ32 = "mock.pubkyring.ed25519.public.z32"
    }
    
    // MARK: - Properties
    
    private let keychain: KeychainStorage
    private var cachedSeedHex: String?
    
    // MARK: - Initialization
    
    private init() {
        self.keychain = KeychainStorage(serviceIdentifier: Bundle.main.bundleIdentifier ?? "com.paykit.demo.mockring")
        loadCachedSeed()
    }
    
    // MARK: - Public Methods
    
    /// Check if the mock service has a seed available
    public var hasSeed: Bool {
        return cachedSeedHex != nil
    }
    
    /// Initialize the mock service with a new random seed
    ///
    /// This generates a new Ed25519 keypair and stores the seed securely.
    /// Call this once during demo setup.
    public func initializeWithNewSeed() throws {
        // Generate new Ed25519 keypair using paykit-mobile FFI
        let keypair = try generateEd25519Keypair()
        
        // Store seed securely
        try keychain.store(key: Keys.mockSeed, string: keypair.secretKeyHex)
        try keychain.store(key: Keys.mockPublicKey, string: keypair.publicKeyHex)
        try keychain.store(key: Keys.mockPublicKeyZ32, string: keypair.publicKeyZ32)
        
        cachedSeedHex = keypair.secretKeyHex
    }
    
    /// Initialize the mock service with an existing seed
    ///
    /// - Parameter seedHex: 32-byte Ed25519 seed as hex string (64 characters)
    public func initializeWithSeed(_ seedHex: String) throws {
        guard seedHex.count == 64 else {
            throw MockPubkyRingError.invalidSeedFormat
        }
        
        // Derive public key from seed
        let keypair = try ed25519KeypairFromSecret(secretKeyHex: seedHex)
        
        // Store seed securely
        try keychain.store(key: Keys.mockSeed, string: seedHex)
        try keychain.store(key: Keys.mockPublicKey, string: keypair.publicKeyHex)
        try keychain.store(key: Keys.mockPublicKeyZ32, string: keypair.publicKeyZ32)
        
        cachedSeedHex = seedHex
    }
    
    /// Get the Ed25519 public key (for identity display)
    ///
    /// - Returns: Public key in z-base32 format (pkarr format)
    public func getEd25519PublicKeyZ32() throws -> String {
        guard let publicKeyZ32 = try? keychain.retrieveString(key: Keys.mockPublicKeyZ32) else {
            throw MockPubkyRingError.noSeedAvailable
        }
        return publicKeyZ32
    }
    
    /// Get the Ed25519 public key in hex format
    ///
    /// - Returns: Public key as hex string
    public func getEd25519PublicKeyHex() throws -> String {
        guard let publicKeyHex = try? keychain.retrieveString(key: Keys.mockPublicKey) else {
            throw MockPubkyRingError.noSeedAvailable
        }
        return publicKeyHex
    }
    
    /// Derive X25519 keypair for Noise protocol
    ///
    /// This uses the pubky-noise KDF to derive device-specific encryption keys.
    ///
    /// - Parameters:
    ///   - deviceId: Unique identifier for this device
    ///   - epoch: Key rotation epoch (increment to rotate keys)
    /// - Returns: Derived X25519 keypair
    public func deriveX25519Keypair(deviceId: String, epoch: UInt32) throws -> X25519KeypairResult {
        guard let seedHex = cachedSeedHex else {
            throw MockPubkyRingError.noSeedAvailable
        }
        
        // Use paykit-mobile FFI for key derivation
        // This uses the same HKDF as pubky-noise
        do {
            let keypair = try deriveX25519Keypair(ed25519SecretHex: seedHex, deviceId: deviceId, epoch: epoch)
            return X25519KeypairResult(
                secretKeyHex: keypair.secretKeyHex,
                publicKeyHex: keypair.publicKeyHex,
                deviceId: deviceId,
                epoch: epoch
            )
        } catch {
            throw MockPubkyRingError.derivationFailed(error.localizedDescription)
        }
    }
    
    /// Get the raw Ed25519 seed (32 bytes)
    ///
    /// **WARNING**: This exposes the cold key, which defeats the purpose of
    /// the cold/hot key architecture. Only use this for demo purposes with
    /// FfiNoiseManager which requires the seed.
    ///
    /// - Returns: Ed25519 seed as raw bytes
    public func getEd25519SeedBytes() throws -> Data {
        guard let seedHex = cachedSeedHex else {
            throw MockPubkyRingError.noSeedAvailable
        }
        
        guard let data = Data(hexString: seedHex) else {
            throw MockPubkyRingError.invalidSeedFormat
        }
        
        return data
    }
    
    /// Sign a message with Ed25519 (for DHT records, not payments)
    ///
    /// - Parameter message: Message to sign
    /// - Returns: 64-byte signature as hex string
    public func signMessage(_ message: Data) throws -> String {
        guard let seedHex = cachedSeedHex else {
            throw MockPubkyRingError.noSeedAvailable
        }
        
        return try signMessage(secretKeyHex: seedHex, message: Array(message))
    }
    
    /// Clear the mock seed (for testing)
    public func clearSeed() {
        try? keychain.delete(key: Keys.mockSeed)
        try? keychain.delete(key: Keys.mockPublicKey)
        try? keychain.delete(key: Keys.mockPublicKeyZ32)
        cachedSeedHex = nil
    }
    
    // MARK: - Private Methods
    
    private func loadCachedSeed() {
        cachedSeedHex = try? keychain.retrieveString(key: Keys.mockSeed)
    }
}

// MARK: - Data Extension for Hex

extension Data {
    init?(hexString: String) {
        let len = hexString.count / 2
        var data = Data(capacity: len)
        var index = hexString.startIndex
        
        for _ in 0..<len {
            let nextIndex = hexString.index(index, offsetBy: 2)
            guard let byte = UInt8(hexString[index..<nextIndex], radix: 16) else {
                return nil
            }
            data.append(byte)
            index = nextIndex
        }
        
        self = data
    }
    
    var hexString: String {
        return map { String(format: "%02x", $0) }.joined()
    }
}

