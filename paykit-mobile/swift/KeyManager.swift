// KeyManager.swift
// Paykit iOS Key Management
//
// This file provides secure key management using the Rust FFI functions
// and iOS Keychain for persistent storage.
//
// Key Architecture:
//   - Ed25519 identity key (pkarr): The user's main identity
//   - X25519 device keys (pubky-noise): Derived for encryption
//
// USAGE:
//   let keyManager = KeyManager()
//   let keypair = try await keyManager.getOrCreateIdentity()
//   print("Your public key: \(keypair.publicKeyZ32)")

import Foundation

/// Manages cryptographic keys for Paykit
///
/// This class handles:
/// - Generating Ed25519 identity keys via Rust FFI
/// - Deriving X25519 device keys for Noise protocol
/// - Secure storage in iOS Keychain
/// - Key backup/restore with password encryption
public final class KeyManager: ObservableObject {
    
    // MARK: - Constants
    
    private enum Keys {
        static let secretKey = "paykit.identity.secret"
        static let publicKey = "paykit.identity.public"
        static let publicKeyZ32 = "paykit.identity.public.z32"
        static let deviceId = "paykit.device.id"
        static let currentEpoch = "paykit.device.epoch"
    }
    
    // MARK: - Published Properties
    
    @Published public private(set) var hasIdentity: Bool = false
    @Published public private(set) var publicKeyZ32: String = ""
    @Published public private(set) var publicKeyHex: String = ""
    
    // MARK: - Private Properties
    
    private let keychain: KeychainStorage
    private let deviceId: String
    
    // MARK: - Initialization
    
    public init(serviceIdentifier: String = Bundle.main.bundleIdentifier ?? "com.paykit.demo") {
        self.keychain = KeychainStorage(serviceIdentifier: serviceIdentifier)
        
        // Get or generate device ID
        if let storedDeviceId = try? keychain.retrieveString(key: Keys.deviceId) {
            self.deviceId = storedDeviceId
        } else {
            let newDeviceId = generateDeviceId()
            try? keychain.store(key: Keys.deviceId, string: newDeviceId)
            self.deviceId = newDeviceId
        }
        
        // Check for existing identity
        loadIdentityState()
    }
    
    // MARK: - Identity Management
    
    /// Get the current identity or create a new one
    ///
    /// - Returns: The Ed25519 keypair
    public func getOrCreateIdentity() throws -> Ed25519Keypair {
        if let existingSecret = try keychain.retrieveString(key: Keys.secretKey) {
            // Restore from storage
            return try ed25519KeypairFromSecret(secretKeyHex: existingSecret)
        } else {
            // Generate new identity
            return try generateNewIdentity()
        }
    }
    
    /// Generate a new identity (replaces existing if any)
    ///
    /// - Returns: The new Ed25519 keypair
    @discardableResult
    public func generateNewIdentity() throws -> Ed25519Keypair {
        let keypair = try generateEd25519Keypair()
        
        // Store in keychain
        try keychain.store(key: Keys.secretKey, string: keypair.secretKeyHex)
        try keychain.store(key: Keys.publicKey, string: keypair.publicKeyHex)
        try keychain.store(key: Keys.publicKeyZ32, string: keypair.publicKeyZ32)
        
        // Update published state
        DispatchQueue.main.async {
            self.hasIdentity = true
            self.publicKeyZ32 = keypair.publicKeyZ32
            self.publicKeyHex = keypair.publicKeyHex
        }
        
        return keypair
    }
    
    /// Get the current public key in z-base32 format (pkarr format)
    ///
    /// - Returns: The public key or nil if no identity exists
    public func getCurrentPublicKeyZ32() -> String? {
        try? keychain.retrieveString(key: Keys.publicKeyZ32)
    }
    
    /// Get the current public key in hex format
    ///
    /// - Returns: The public key or nil if no identity exists
    public func getCurrentPublicKeyHex() -> String? {
        try? keychain.retrieveString(key: Keys.publicKeyHex)
    }
    
    /// Delete the current identity
    ///
    /// Warning: This cannot be undone unless you have a backup!
    public func deleteIdentity() throws {
        try keychain.delete(key: Keys.secretKey)
        try keychain.delete(key: Keys.publicKey)
        try keychain.delete(key: Keys.publicKeyZ32)
        
        DispatchQueue.main.async {
            self.hasIdentity = false
            self.publicKeyZ32 = ""
            self.publicKeyHex = ""
        }
    }
    
    // MARK: - X25519 Device Keys
    
    /// Get or derive X25519 device key for Noise protocol
    ///
    /// - Parameter epoch: Key rotation epoch (default 0)
    /// - Returns: The X25519 keypair for this device and epoch
    public func getDeviceX25519Key(epoch: UInt32 = 0) throws -> X25519Keypair {
        guard let secretHex = try keychain.retrieveString(key: Keys.secretKey) else {
            throw KeyManagerError.noIdentity
        }
        
        return try deriveX25519Keypair(
            ed25519SecretHex: secretHex,
            deviceId: deviceId,
            epoch: epoch
        )
    }
    
    /// Get the current device ID
    public func getDeviceId() -> String {
        deviceId
    }
    
    /// Get the current key epoch
    public func getCurrentEpoch() -> UInt32 {
        guard let epochStr = try? keychain.retrieveString(key: Keys.currentEpoch),
              let epoch = UInt32(epochStr) else {
            return 0
        }
        return epoch
    }
    
    /// Increment the key epoch (for key rotation)
    public func incrementEpoch() throws -> UInt32 {
        let newEpoch = getCurrentEpoch() + 1
        try keychain.store(key: Keys.currentEpoch, string: String(newEpoch))
        return newEpoch
    }
    
    // MARK: - Signing
    
    /// Sign data with the identity key
    ///
    /// - Parameter data: Data to sign
    /// - Returns: Hex-encoded signature
    public func sign(data: Data) throws -> String {
        guard let secretHex = try keychain.retrieveString(key: Keys.secretKey) else {
            throw KeyManagerError.noIdentity
        }
        
        return try signMessage(secretKeyHex: secretHex, message: data)
    }
    
    /// Verify a signature
    ///
    /// - Parameters:
    ///   - publicKeyHex: Signer's public key in hex
    ///   - data: Original data
    ///   - signatureHex: Signature to verify
    /// - Returns: true if valid
    public func verify(publicKeyHex: String, data: Data, signatureHex: String) throws -> Bool {
        return try verifySignature(
            publicKeyHex: publicKeyHex,
            message: data,
            signatureHex: signatureHex
        )
    }
    
    // MARK: - Backup & Restore
    
    /// Export identity to encrypted backup
    ///
    /// - Parameter password: Password to encrypt the backup
    /// - Returns: The encrypted backup
    public func exportBackup(password: String) throws -> KeyBackup {
        guard let secretHex = try keychain.retrieveString(key: Keys.secretKey) else {
            throw KeyManagerError.noIdentity
        }
        
        return try exportKeypairToBackup(secretKeyHex: secretHex, password: password)
    }
    
    /// Import identity from encrypted backup
    ///
    /// - Parameters:
    ///   - backup: The encrypted backup
    ///   - password: Password to decrypt
    /// - Returns: The restored keypair
    @discardableResult
    public func importBackup(_ backup: KeyBackup, password: String) throws -> Ed25519Keypair {
        let keypair = try importKeypairFromBackup(backup: backup, password: password)
        
        // Store in keychain
        try keychain.store(key: Keys.secretKey, string: keypair.secretKeyHex)
        try keychain.store(key: Keys.publicKey, string: keypair.publicKeyHex)
        try keychain.store(key: Keys.publicKeyZ32, string: keypair.publicKeyZ32)
        
        // Update published state
        DispatchQueue.main.async {
            self.hasIdentity = true
            self.publicKeyZ32 = keypair.publicKeyZ32
            self.publicKeyHex = keypair.publicKeyHex
        }
        
        return keypair
    }
    
    /// Convert backup to shareable string (JSON)
    public func backupToString(_ backup: KeyBackup) throws -> String {
        let dict: [String: Any] = [
            "version": backup.version,
            "encrypted_data": backup.encryptedDataHex,
            "salt": backup.saltHex,
            "nonce": backup.nonceHex,
            "public_key": backup.publicKeyZ32
        ]
        let data = try JSONSerialization.data(withJSONObject: dict, options: [.prettyPrinted, .sortedKeys])
        return String(data: data, encoding: .utf8) ?? ""
    }
    
    /// Parse backup from string (JSON)
    public func backupFromString(_ string: String) throws -> KeyBackup {
        guard let data = string.data(using: .utf8),
              let dict = try JSONSerialization.jsonObject(with: data) as? [String: Any],
              let version = dict["version"] as? UInt32,
              let encryptedData = dict["encrypted_data"] as? String,
              let salt = dict["salt"] as? String,
              let nonce = dict["nonce"] as? String,
              let publicKey = dict["public_key"] as? String else {
            throw KeyManagerError.invalidBackup
        }
        
        return KeyBackup(
            version: version,
            encryptedDataHex: encryptedData,
            saltHex: salt,
            nonceHex: nonce,
            publicKeyZ32: publicKey
        )
    }
    
    // MARK: - Private Helpers
    
    private func loadIdentityState() {
        if let pubZ32 = try? keychain.retrieveString(key: Keys.publicKeyZ32),
           let pubHex = try? keychain.retrieveString(key: Keys.publicKey) {
            self.hasIdentity = true
            self.publicKeyZ32 = pubZ32
            self.publicKeyHex = pubHex
        } else {
            self.hasIdentity = false
            self.publicKeyZ32 = ""
            self.publicKeyHex = ""
        }
    }
}

// MARK: - Errors

public enum KeyManagerError: LocalizedError {
    case noIdentity
    case invalidBackup
    case backupDecryptionFailed
    
    public var errorDescription: String? {
        switch self {
        case .noIdentity:
            return "No identity key exists. Generate or import one first."
        case .invalidBackup:
            return "Invalid backup format."
        case .backupDecryptionFailed:
            return "Failed to decrypt backup. Check your password."
        }
    }
}

