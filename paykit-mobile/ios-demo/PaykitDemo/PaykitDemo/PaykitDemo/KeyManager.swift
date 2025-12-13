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
import Combine

/// Identity information for display and management
public struct IdentityInfo: Identifiable, Codable {
    public let name: String
    public let publicKeyZ32: String
    public let publicKeyHex: String
    public let nickname: String?
    public let createdAt: Date
    public var id: String { name }
}

/// Manages cryptographic keys for Paykit
///
/// This class handles:
/// - Generating Ed25519 identity keys via Rust FFI
/// - Deriving X25519 device keys for Noise protocol
/// - Secure storage in iOS Keychain
/// - Key backup/restore with password encryption
/// - Multiple identity management
public final class KeyManager: ObservableObject {
    
    // MARK: - Constants
    
    private enum Keys {
        // Legacy single identity keys (for migration)
        static let secretKey = "paykit.identity.secret"
        static let publicKey = "paykit.identity.public"
        static let publicKeyZ32 = "paykit.identity.public.z32"
        
        // Multiple identity keys (new format)
        static func secretKey(name: String) -> String { "paykit.identity.\(name).secret" }
        static func publicKey(name: String) -> String { "paykit.identity.\(name).public" }
        static func publicKeyZ32(name: String) -> String { "paykit.identity.\(name).public.z32" }
        static func nickname(name: String) -> String { "paykit.identity.\(name).nickname" }
        static func createdAt(name: String) -> String { "paykit.identity.\(name).created_at" }
        
        // Current identity and list
        static let currentIdentity = "paykit.current_identity"
        static let identityList = "paykit.identity_list"
        
        static let deviceId = "paykit.device.id"
        static let currentEpoch = "paykit.device.epoch"
    }
    
    // MARK: - Published Properties
    
    @Published public private(set) var hasIdentity: Bool = false
    @Published public private(set) var publicKeyZ32: String = ""
    @Published public private(set) var publicKeyHex: String = ""
    @Published public private(set) var currentIdentityName: String?
    
    // MARK: - Private Properties
    
    private let keychain: KeychainStorage
    private let deviceId: String
    private let userDefaults: UserDefaults
    
    // MARK: - Initialization
    
    public init(serviceIdentifier: String = Bundle.main.bundleIdentifier ?? "com.paykit.demo") {
        self.keychain = KeychainStorage(serviceIdentifier: serviceIdentifier)
        self.userDefaults = UserDefaults.standard
        
        // Get or generate device ID
        if let storedDeviceId = try? keychain.retrieveString(key: Keys.deviceId) {
            self.deviceId = storedDeviceId
        } else {
            let newDeviceId = generateDeviceId()
            try? keychain.store(key: Keys.deviceId, string: newDeviceId)
            self.deviceId = newDeviceId
        }
        
        // Migrate single identity to multiple if needed
        try? migrateSingleIdentity()
        
        // Check for existing identity
        loadIdentityState()
    }
    
    // MARK: - Identity Management
    
    /// Get the current identity or create a new one
    ///
    /// - Returns: The Ed25519 keypair
    public func getOrCreateIdentity() throws -> Ed25519Keypair {
        let currentName = getCurrentIdentityName()
        
        if let name = currentName {
            // Load existing identity
            return try getIdentity(name: name)
        } else {
            // No current identity, create default
            return try createIdentity(name: "default", nickname: nil)
        }
    }
    
    /// Get identity by name
    ///
    /// - Parameter name: Identity name
    /// - Returns: The Ed25519 keypair
    public func getIdentity(name: String) throws -> Ed25519Keypair {
        let secretKey = Keys.secretKey(name: name)
        guard let secretHex = try keychain.retrieveString(key: secretKey) else {
            throw KeyManagerError.identityNotFound(name: name)
        }
        return try ed25519KeypairFromSecret(secretKeyHex: secretHex)
    }
    
    /// Get identity info without loading secret key
    ///
    /// - Parameter name: Identity name
    /// - Returns: IdentityInfo or nil if not found
    public func getIdentityInfo(name: String) -> IdentityInfo? {
        guard let publicKeyZ32 = try? keychain.retrieveString(key: Keys.publicKeyZ32(name: name)),
              let publicKeyHex = try? keychain.retrieveString(key: Keys.publicKey(name: name)) else {
            return nil
        }
        
        let nickname = try? keychain.retrieveString(key: Keys.nickname(name: name))
        let createdAtStr = try? keychain.retrieveString(key: Keys.createdAt(name: name))
        let createdAt = createdAtStr.flatMap { Double($0) }.map { Date(timeIntervalSince1970: $0) } ?? Date()
        
        return IdentityInfo(
            name: name,
            publicKeyZ32: publicKeyZ32,
            publicKeyHex: publicKeyHex,
            nickname: nickname,
            createdAt: createdAt
        )
    }
    
    /// List all identities
    ///
    /// - Returns: Array of IdentityInfo
    public func listIdentities() -> [IdentityInfo] {
        guard let identityList = userDefaults.stringArray(forKey: Keys.identityList) else {
            return []
        }
        
        return identityList.compactMap { name in
            getIdentityInfo(name: name)
        }
    }
    
    /// Create a new identity
    ///
    /// - Parameters:
    ///   - name: Unique name for the identity
    ///   - nickname: Optional nickname
    /// - Returns: The new Ed25519 keypair
    @discardableResult
    public func createIdentity(name: String, nickname: String?) throws -> Ed25519Keypair {
        // Validate name
        guard !name.isEmpty else {
            throw KeyManagerError.invalidIdentityName("Name cannot be empty")
        }
        
        // Check for duplicates
        if getIdentityInfo(name: name) != nil {
            throw KeyManagerError.duplicateIdentity(name: name)
        }
        
        // Generate keypair
        let keypair = try generateEd25519Keypair()
        
        // Store in keychain with name prefix
        try keychain.store(key: Keys.secretKey(name: name), string: keypair.secretKeyHex)
        try keychain.store(key: Keys.publicKey(name: name), string: keypair.publicKeyHex)
        try keychain.store(key: Keys.publicKeyZ32(name: name), string: keypair.publicKeyZ32)
        
        if let nickname = nickname {
            try keychain.store(key: Keys.nickname(name: name), string: nickname)
        }
        
        let createdAt = String(Date().timeIntervalSince1970)
        try keychain.store(key: Keys.createdAt(name: name), string: createdAt)
        
        // Add to identity list
        var identityList = userDefaults.stringArray(forKey: Keys.identityList) ?? []
        if !identityList.contains(name) {
            identityList.append(name)
            userDefaults.set(identityList, forKey: Keys.identityList)
        }
        
        // If no current identity, set this as current
        if getCurrentIdentityName() == nil {
            try switchIdentity(name: name)
        }
        
        return keypair
    }
    
    /// Switch to a different identity
    ///
    /// - Parameter name: Identity name to switch to
    public func switchIdentity(name: String) throws {
        // Validate identity exists
        guard getIdentityInfo(name: name) != nil else {
            throw KeyManagerError.identityNotFound(name: name)
        }
        
        // Update current identity
        userDefaults.set(name, forKey: Keys.currentIdentity)
        currentIdentityName = name
        
        // Reload identity state
        loadIdentityState()
        
        // Post notification for app-wide reload
        NotificationCenter.default.post(name: .identityDidChange, object: nil)
    }
    
    /// Delete an identity
    ///
    /// - Parameter name: Identity name to delete
    public func deleteIdentity(name: String) throws {
        // Prevent deleting current identity (must switch first)
        if name == getCurrentIdentityName() {
            throw KeyManagerError.cannotDeleteCurrentIdentity
        }
        
        // Delete all keychain entries for this identity
        try? keychain.delete(key: Keys.secretKey(name: name))
        try? keychain.delete(key: Keys.publicKey(name: name))
        try? keychain.delete(key: Keys.publicKeyZ32(name: name))
        try? keychain.delete(key: Keys.nickname(name: name))
        try? keychain.delete(key: Keys.createdAt(name: name))
        
        // Remove from identity list
        var identityList = userDefaults.stringArray(forKey: Keys.identityList) ?? []
        identityList.removeAll { $0 == name }
        userDefaults.set(identityList, forKey: Keys.identityList)
        
        // If was current, set first available as current
        if let firstIdentity = identityList.first {
            try? switchIdentity(name: firstIdentity)
        } else {
            userDefaults.removeObject(forKey: Keys.currentIdentity)
            currentIdentityName = nil
            loadIdentityState()
        }
    }
    
    /// Get current identity name
    ///
    /// - Returns: Current identity name or nil
    public func getCurrentIdentityName() -> String? {
        return userDefaults.string(forKey: Keys.currentIdentity)
    }
    
    /// Generate a new identity (replaces existing if any) - Legacy method for backward compatibility
    ///
    /// - Returns: The new Ed25519 keypair
    @discardableResult
    public func generateNewIdentity() throws -> Ed25519Keypair {
        // Use current identity name or "default"
        let name = getCurrentIdentityName() ?? "default"
        
        // Delete existing if any
        if getIdentityInfo(name: name) != nil {
            try? deleteIdentity(name: name)
        }
        
        return try createIdentity(name: name, nickname: nil)
    }
    
    /// Get the current public key in z-base32 format (pkarr format)
    ///
    /// - Returns: The public key or nil if no identity exists
    public func getCurrentPublicKeyZ32() -> String? {
        guard let name = getCurrentIdentityName() else {
            return nil
        }
        return try? keychain.retrieveString(key: Keys.publicKeyZ32(name: name))
    }
    
    /// Get the current public key in hex format
    ///
    /// - Returns: The public key or nil if no identity exists
    public func getCurrentPublicKeyHex() -> String? {
        guard let name = getCurrentIdentityName() else {
            return nil
        }
        return try? keychain.retrieveString(key: Keys.publicKey(name: name))
    }
    
    /// Delete the current identity - Legacy method for backward compatibility
    ///
    /// Warning: This cannot be undone unless you have a backup!
    public func deleteIdentity() throws {
        guard let name = getCurrentIdentityName() else {
            throw KeyManagerError.noIdentity
        }
        try deleteIdentity(name: name)
    }
    
    /// Migrate existing single identity to named system
    private func migrateSingleIdentity() throws {
        // Check for old single identity keys
        guard try keychain.retrieveString(key: Keys.secretKey) != nil else {
            return // No migration needed
        }
        
        // Load old identity
        guard let oldSecret = try? keychain.retrieveString(key: Keys.secretKey),
              let oldPublic = try? keychain.retrieveString(key: Keys.publicKey),
              let oldPublicZ32 = try? keychain.retrieveString(key: Keys.publicKeyZ32) else {
            return // Migration failed, but don't throw
        }
        
        // Create "default" identity with old keys
        try? keychain.store(key: Keys.secretKey(name: "default"), string: oldSecret)
        try? keychain.store(key: Keys.publicKey(name: "default"), string: oldPublic)
        try? keychain.store(key: Keys.publicKeyZ32(name: "default"), string: oldPublicZ32)
        
        let createdAt = String(Date().timeIntervalSince1970)
        try? keychain.store(key: Keys.createdAt(name: "default"), string: createdAt)
        
        // Delete old keys
        try? keychain.delete(key: Keys.secretKey)
        try? keychain.delete(key: Keys.publicKey)
        try? keychain.delete(key: Keys.publicKeyZ32)
        
        // Set "default" as current
        userDefaults.set("default", forKey: Keys.currentIdentity)
        var identityList = userDefaults.stringArray(forKey: Keys.identityList) ?? []
        if !identityList.contains("default") {
            identityList.append("default")
            userDefaults.set(identityList, forKey: Keys.identityList)
        }
    }
    
    // MARK: - X25519 Device Keys
    
    /// Get or derive X25519 device key for Noise protocol
    ///
    /// - Parameter epoch: Key rotation epoch (default 0)
    /// - Returns: The X25519 keypair for this device and epoch
    public func getDeviceX25519Key(epoch: UInt32 = 0) throws -> X25519Keypair {
        guard let name = getCurrentIdentityName() else {
            throw KeyManagerError.noIdentity
        }
        
        let secretKey = Keys.secretKey(name: name)
        guard let secretHex = try keychain.retrieveString(key: secretKey) else {
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
        guard let name = getCurrentIdentityName() else {
            throw KeyManagerError.noIdentity
        }
        
        let secretKey = Keys.secretKey(name: name)
        guard let secretHex = try keychain.retrieveString(key: secretKey) else {
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
    
    /// Get the secret key as raw bytes (for Noise protocol)
    ///
    /// - Returns: 32-byte secret key data, or nil if no identity
    public func getSecretKeyData() -> Data? {
        guard let name = getCurrentIdentityName() else { return nil }
        
        let secretKey = Keys.secretKey(name: name)
        guard let secretHex = try? keychain.retrieveString(key: secretKey) else { return nil }
        
        // Convert hex to bytes
        var data = Data(capacity: secretHex.count / 2)
        var index = secretHex.startIndex
        for _ in 0..<secretHex.count / 2 {
            let nextIndex = secretHex.index(index, offsetBy: 2)
            if let byte = UInt8(secretHex[index..<nextIndex], radix: 16) {
                data.append(byte)
            }
            index = nextIndex
        }
        
        return data.count == 32 ? data : nil
    }
    
    // MARK: - Backup & Restore
    
    /// Export identity to encrypted backup
    ///
    /// - Parameter password: Password to encrypt the backup
    /// - Returns: The encrypted backup
    public func exportBackup(password: String) throws -> KeyBackup {
        guard let name = getCurrentIdentityName() else {
            throw KeyManagerError.noIdentity
        }
        
        let secretKey = Keys.secretKey(name: name)
        guard let secretHex = try keychain.retrieveString(key: secretKey) else {
            throw KeyManagerError.noIdentity
        }
        
        return try exportKeypairToBackup(secretKeyHex: secretHex, password: password)
    }
    
    /// Import identity from encrypted backup
    ///
    /// - Parameters:
    ///   - backup: The encrypted backup
    ///   - password: Password to decrypt
    ///   - name: Name for the imported identity (defaults to backup public key prefix)
    /// - Returns: The restored keypair
    @discardableResult
    public func importBackup(_ backup: KeyBackup, password: String, name: String? = nil) throws -> Ed25519Keypair {
        let keypair = try importKeypairFromBackup(backup: backup, password: password)
        
        // Use provided name or generate from public key
        let identityName = name ?? String(backup.publicKeyZ32.prefix(8))
        
        // Store in keychain with name prefix
        try keychain.store(key: Keys.secretKey(name: identityName), string: keypair.secretKeyHex)
        try keychain.store(key: Keys.publicKey(name: identityName), string: keypair.publicKeyHex)
        try keychain.store(key: Keys.publicKeyZ32(name: identityName), string: keypair.publicKeyZ32)
        
        let createdAt = String(Date().timeIntervalSince1970)
        try keychain.store(key: Keys.createdAt(name: identityName), string: createdAt)
        
        // Add to identity list
        var identityList = userDefaults.stringArray(forKey: Keys.identityList) ?? []
        if !identityList.contains(identityName) {
            identityList.append(identityName)
            userDefaults.set(identityList, forKey: Keys.identityList)
        }
        
        // If no current identity, set this as current
        if getCurrentIdentityName() == nil {
            try switchIdentity(name: identityName)
        }
        
        // Update published state
        DispatchQueue.main.async {
            self.loadIdentityState()
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
        guard let name = getCurrentIdentityName() else {
            self.hasIdentity = false
            self.publicKeyZ32 = ""
            self.publicKeyHex = ""
            self.currentIdentityName = nil
            return
        }
        
        if let pubZ32 = try? keychain.retrieveString(key: Keys.publicKeyZ32(name: name)),
           let pubHex = try? keychain.retrieveString(key: Keys.publicKey(name: name)) {
            self.hasIdentity = true
            self.publicKeyZ32 = pubZ32
            self.publicKeyHex = pubHex
            self.currentIdentityName = name
        } else {
            self.hasIdentity = false
            self.publicKeyZ32 = ""
            self.publicKeyHex = ""
            self.currentIdentityName = nil
        }
    }
}

// MARK: - Notification Names

extension Notification.Name {
    public static let identityDidChange = Notification.Name("identityDidChange")
}

// MARK: - Errors

public enum KeyManagerError: LocalizedError {
    case noIdentity
    case invalidBackup
    case backupDecryptionFailed
    case identityNotFound(name: String)
    case duplicateIdentity(name: String)
    case invalidIdentityName(String)
    case cannotDeleteCurrentIdentity
    
    public var errorDescription: String? {
        switch self {
        case .noIdentity:
            return "No identity key exists. Generate or import one first."
        case .invalidBackup:
            return "Invalid backup format."
        case .backupDecryptionFailed:
            return "Failed to decrypt backup. Check your password."
        case .identityNotFound(let name):
            return "Identity '\(name)' not found."
        case .duplicateIdentity(let name):
            return "Identity '\(name)' already exists."
        case .invalidIdentityName(let message):
            return "Invalid identity name: \(message)"
        case .cannotDeleteCurrentIdentity:
            return "Cannot delete current identity. Switch to another identity first."
        }
    }
}

