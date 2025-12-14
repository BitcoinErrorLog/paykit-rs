//
//  PaymentMethodStorage.swift
//  PaykitDemo
//
//  Storage for configured payment methods.
//

import Foundation

/// Represents a configured payment method
public struct ConfiguredPaymentMethod: Codable, Identifiable {
    public var id: String { methodId }
    public let methodId: String
    public let endpoint: String
    public var isPublic: Bool
    public let addedAt: Date
    
    public init(methodId: String, endpoint: String, isPublic: Bool = false) {
        self.methodId = methodId
        self.endpoint = endpoint
        self.isPublic = isPublic
        self.addedAt = Date()
    }
}

/// Storage for payment methods
public class PaymentMethodStorage {
    private let keychain: KeychainStorage
    private let identityName: String
    
    private var cacheKey: String { "paykit.methods.\(identityName)" }
    
    public init(identityName: String) {
        self.identityName = identityName
        self.keychain = KeychainStorage(serviceIdentifier: "com.paykit.demo.methods.\(identityName)")
    }
    
    /// List all configured payment methods
    public func listMethods() -> [ConfiguredPaymentMethod] {
        guard let data = try? keychain.retrieve(key: cacheKey) else {
            return []
        }
        return (try? JSONDecoder().decode([ConfiguredPaymentMethod].self, from: data)) ?? []
    }
    
    /// Add a new payment method
    public func addMethod(_ method: ConfiguredPaymentMethod) throws {
        var methods = listMethods()
        // Remove existing if present
        methods.removeAll { $0.methodId == method.methodId }
        methods.append(method)
        
        let data = try JSONEncoder().encode(methods)
        try keychain.store(key: cacheKey, data: data)
    }
    
    /// Remove a payment method
    public func removeMethod(methodId: String) throws {
        var methods = listMethods()
        methods.removeAll { $0.methodId == methodId }
        
        let data = try JSONEncoder().encode(methods)
        try keychain.store(key: cacheKey, data: data)
    }
    
    /// Update a payment method's public status
    public func setPublic(methodId: String, isPublic: Bool) throws {
        var methods = listMethods()
        if let index = methods.firstIndex(where: { $0.methodId == methodId }) {
            methods[index].isPublic = isPublic
            
            let data = try JSONEncoder().encode(methods)
            try keychain.store(key: cacheKey, data: data)
        }
    }
    
    /// Get a specific method
    public func getMethod(methodId: String) -> ConfiguredPaymentMethod? {
        return listMethods().first { $0.methodId == methodId }
    }
    
    /// Clear all methods
    public func clear() throws {
        try keychain.delete(key: cacheKey)
    }
}

