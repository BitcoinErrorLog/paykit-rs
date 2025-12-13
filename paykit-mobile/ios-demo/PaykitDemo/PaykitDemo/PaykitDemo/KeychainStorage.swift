// KeychainStorage.swift
// Paykit iOS Keychain Adapter
//
// This file provides an implementation of SecureStorage using iOS Keychain Services.
//
// USAGE:
//   1. Add this file to your Xcode project
//   2. Create a KeychainStorage instance with your app's bundle ID
//   3. Pass it to PaykitClient for secure storage operations
//
// Example:
//   let storage = KeychainStorage(serviceIdentifier: "com.example.myapp")
//   let client = PaykitClient(storage: storage)

import Foundation
import Security

/// Keychain-based secure storage for iOS.
///
/// This class wraps iOS Keychain Services to provide secure storage
/// for sensitive data like private keys and authentication tokens.
public final class KeychainStorage {
    
    // MARK: - Properties
    
    private let serviceIdentifier: String
    private let accessGroup: String?
    
    // MARK: - Error Types
    
    public enum KeychainError: LocalizedError {
        case unhandledError(status: OSStatus)
        case itemNotFound
        case duplicateItem
        case invalidData
        case accessDenied
        
        public var errorDescription: String? {
            switch self {
            case .unhandledError(let status):
                return "Keychain error: \(status)"
            case .itemNotFound:
                return "Item not found in keychain"
            case .duplicateItem:
                return "Item already exists in keychain"
            case .invalidData:
                return "Invalid data format"
            case .accessDenied:
                return "Access denied to keychain"
            }
        }
    }
    
    // MARK: - Initialization
    
    /// Create a new KeychainStorage instance.
    ///
    /// - Parameters:
    ///   - serviceIdentifier: Unique identifier for your app (usually bundle ID)
    ///   - accessGroup: Optional keychain access group for sharing between apps
    public init(serviceIdentifier: String, accessGroup: String? = nil) {
        self.serviceIdentifier = serviceIdentifier
        self.accessGroup = accessGroup
    }
    
    // MARK: - Public Methods
    
    /// Store data securely in the keychain.
    ///
    /// - Parameters:
    ///   - key: Unique identifier for the data
    ///   - data: The data to store
    /// - Throws: KeychainError if storage fails
    public func store(key: String, data: Data) throws {
        // Delete existing item first (if any)
        try? delete(key: key)
        
        var query = baseQuery(for: key)
        query[kSecValueData as String] = data
        query[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        
        let status = SecItemAdd(query as CFDictionary, nil)
        
        guard status == errSecSuccess else {
            throw KeychainError.unhandledError(status: status)
        }
    }
    
    /// Store a string securely in the keychain.
    ///
    /// - Parameters:
    ///   - key: Unique identifier for the string
    ///   - value: The string to store
    /// - Throws: KeychainError if storage fails
    public func store(key: String, string: String) throws {
        guard let data = string.data(using: .utf8) else {
            throw KeychainError.invalidData
        }
        try store(key: key, data: data)
    }
    
    /// Retrieve data from the keychain.
    ///
    /// - Parameter key: The key to retrieve
    /// - Returns: The stored data, or nil if not found
    /// - Throws: KeychainError on access errors
    public func retrieve(key: String) throws -> Data? {
        var query = baseQuery(for: key)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        
        switch status {
        case errSecSuccess:
            return result as? Data
        case errSecItemNotFound:
            return nil
        default:
            throw KeychainError.unhandledError(status: status)
        }
    }
    
    /// Retrieve a string from the keychain.
    ///
    /// - Parameter key: The key to retrieve
    /// - Returns: The stored string, or nil if not found
    /// - Throws: KeychainError on access errors
    public func retrieveString(key: String) throws -> String? {
        guard let data = try retrieve(key: key) else {
            return nil
        }
        return String(data: data, encoding: .utf8)
    }
    
    /// Delete data from the keychain.
    ///
    /// - Parameter key: The key to delete
    /// - Throws: KeychainError on deletion errors (except not found)
    public func delete(key: String) throws {
        let query = baseQuery(for: key)
        let status = SecItemDelete(query as CFDictionary)
        
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.unhandledError(status: status)
        }
    }
    
    /// List all keys stored in the keychain for this service.
    ///
    /// - Returns: Array of stored keys
    /// - Throws: KeychainError on access errors
    public func listKeys() throws -> [String] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceIdentifier,
            kSecReturnAttributes as String: true,
            kSecMatchLimit as String: kSecMatchLimitAll
        ]
        
        if let group = accessGroup {
            query[kSecAttrAccessGroup as String] = group
        }
        
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        
        switch status {
        case errSecSuccess:
            guard let items = result as? [[String: Any]] else {
                return []
            }
            return items.compactMap { $0[kSecAttrAccount as String] as? String }
        case errSecItemNotFound:
            return []
        default:
            throw KeychainError.unhandledError(status: status)
        }
    }
    
    /// Check if a key exists in the keychain.
    ///
    /// - Parameter key: The key to check
    /// - Returns: true if the key exists
    public func contains(key: String) -> Bool {
        do {
            return try retrieve(key: key) != nil
        } catch {
            return false
        }
    }
    
    /// Clear all data for this service.
    ///
    /// - Throws: KeychainError on deletion errors
    public func clear() throws {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceIdentifier
        ]
        
        if let group = accessGroup {
            query[kSecAttrAccessGroup as String] = group
        }
        
        let status = SecItemDelete(query as CFDictionary)
        
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.unhandledError(status: status)
        }
    }
    
    // MARK: - Private Helpers
    
    private func baseQuery(for key: String) -> [String: Any] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceIdentifier,
            kSecAttrAccount as String: key
        ]
        
        if let group = accessGroup {
            query[kSecAttrAccessGroup as String] = group
        }
        
        return query
    }
}

// MARK: - Paykit Integration

/// Extension to bridge KeychainStorage to Paykit's SecureStorage protocol.
///
/// Usage:
/// ```swift
/// let storage = KeychainStorage(serviceIdentifier: "com.example.myapp")
/// let paykitStorage = storage.asPaykitStorage()
/// ```
extension KeychainStorage {
    
    /// Convert to Paykit-compatible storage interface.
    ///
    /// This wraps the KeychainStorage to implement the protocol expected by Paykit.
    public func asPaykitStorage() -> PaykitSecureStorageAdapter {
        return PaykitSecureStorageAdapter(keychain: self)
    }
}

/// Adapter that bridges KeychainStorage to Paykit's expected interface.
public final class PaykitSecureStorageAdapter {
    
    private let keychain: KeychainStorage
    
    init(keychain: KeychainStorage) {
        self.keychain = keychain
    }
    
    public func store(key: String, value: Data) -> Result<Void, Error> {
        do {
            try keychain.store(key: key, data: value)
            return .success(())
        } catch {
            return .failure(error)
        }
    }
    
    public func retrieve(key: String) -> Result<Data?, Error> {
        do {
            let data = try keychain.retrieve(key: key)
            return .success(data)
        } catch {
            return .failure(error)
        }
    }
    
    public func delete(key: String) -> Result<Void, Error> {
        do {
            try keychain.delete(key: key)
            return .success(())
        } catch {
            return .failure(error)
        }
    }
    
    public func listKeys() -> Result<[String], Error> {
        do {
            let keys = try keychain.listKeys()
            return .success(keys)
        } catch {
            return .failure(error)
        }
    }
}
