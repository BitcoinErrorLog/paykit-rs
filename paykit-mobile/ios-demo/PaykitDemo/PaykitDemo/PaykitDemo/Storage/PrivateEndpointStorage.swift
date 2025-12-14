//
//  PrivateEndpointStorage.swift
//  PaykitDemo
//
//  Persistent storage for private endpoints using Keychain.
//

import Foundation
// PaykitMobile types are in the same module, no import needed

/// Manages persistent storage of private payment endpoints
class PrivateEndpointStorage {
    
    private let keychain: KeychainStorage
    private let identityName: String
    
    // In-memory cache
    private var endpointsCache: [String: [PrivateEndpointOffer]]?
    
    private var endpointsKey: String {
        "paykit.private_endpoints.\(identityName)"
    }
    
    init(identityName: String, keychain: KeychainStorage = KeychainStorage(serviceIdentifier: "com.paykit.demo")) {
        self.identityName = identityName
        self.keychain = keychain
    }
    
    // MARK: - CRUD Operations
    
    /// Get all private endpoints for a peer
    func listForPeer(_ peerPubkey: String) -> [PrivateEndpointOffer] {
        let all = loadAllEndpoints()
        return all[peerPubkey] ?? []
    }
    
    /// Get a specific endpoint for a peer and method
    func get(peerPubkey: String, methodId: String) -> PrivateEndpointOffer? {
        let endpoints = listForPeer(peerPubkey)
        return endpoints.first { $0.methodId == methodId }
    }
    
    /// Save a private endpoint
    func save(_ endpoint: PrivateEndpointOffer, forPeer peerPubkey: String) throws {
        var all = loadAllEndpoints()
        
        // Get or create list for this peer
        var peerEndpoints = all[peerPubkey] ?? []
        
        // Remove existing endpoint for this method if it exists
        peerEndpoints.removeAll { $0.methodId == endpoint.methodId }
        
        // Add the new endpoint
        peerEndpoints.append(endpoint)
        
        // Update the dictionary
        all[peerPubkey] = peerEndpoints
        
        try persistAllEndpoints(all)
    }
    
    /// Remove a specific endpoint
    func remove(peerPubkey: String, methodId: String) throws {
        var all = loadAllEndpoints()
        
        guard var peerEndpoints = all[peerPubkey] else {
            return // Nothing to remove
        }
        
        peerEndpoints.removeAll { $0.methodId == methodId }
        
        if peerEndpoints.isEmpty {
            all.removeValue(forKey: peerPubkey)
        } else {
            all[peerPubkey] = peerEndpoints
        }
        
        try persistAllEndpoints(all)
    }
    
    /// Remove all endpoints for a peer
    func removeAllForPeer(_ peerPubkey: String) throws {
        var all = loadAllEndpoints()
        all.removeValue(forKey: peerPubkey)
        try persistAllEndpoints(all)
    }
    
    /// List all peers that have private endpoints
    func listPeers() -> [String] {
        let all = loadAllEndpoints()
        return Array(all.keys)
    }
    
    /// Clean up expired endpoints
    /// Note: Expiration checking would need to be added to PrivateEndpointOffer
    /// For now, this is a placeholder
    func cleanupExpired() -> Int {
        // TODO: Implement expiration checking when PrivateEndpointOffer includes expiresAt
        return 0
    }
    
    /// Get count of all stored endpoints
    func count() -> Int {
        let all = loadAllEndpoints()
        return all.values.reduce(0) { $0 + $1.count }
    }
    
    /// Clear all endpoints
    func clearAll() throws {
        try persistAllEndpoints([:])
    }
    
    // MARK: - Private Helpers
    
    private func loadAllEndpoints() -> [String: [PrivateEndpointOffer]] {
        if let cached = endpointsCache {
            return cached
        }
        
        do {
            guard let data = try keychain.retrieve(key: endpointsKey) else {
                return [:]
            }
            
            // Decode from JSON
            let decoder = JSONDecoder()
            let stored = try decoder.decode(StoredEndpoints.self, from: data)
            
            // Convert stored format to PrivateEndpointOffer
            var result: [String: [PrivateEndpointOffer]] = [:]
            for (peer, endpoints) in stored.endpoints {
                result[peer] = endpoints.map { stored in
                    PrivateEndpointOffer(
                        methodId: stored.methodId,
                        endpoint: stored.endpoint
                    )
                }
            }
            
            endpointsCache = result
            return result
        } catch {
            print("PrivateEndpointStorage: Failed to load endpoints: \(error)")
            return [:]
        }
    }
    
    private func persistAllEndpoints(_ endpoints: [String: [PrivateEndpointOffer]]) throws {
        // Convert PrivateEndpointOffer to storable format
        var stored: [String: [StoredEndpoint]] = [:]
        for (peer, offers) in endpoints {
            stored[peer] = offers.map { offer in
                StoredEndpoint(
                    methodId: offer.methodId,
                    endpoint: offer.endpoint
                )
            }
        }
        
        let storedEndpoints = StoredEndpoints(endpoints: stored)
        let encoder = JSONEncoder()
        let data = try encoder.encode(storedEndpoints)
        
        try keychain.store(key: endpointsKey, data: data)
        endpointsCache = endpoints
    }
}

// MARK: - Storage Models

private struct StoredEndpoints: Codable {
    var endpoints: [String: [StoredEndpoint]]
}

private struct StoredEndpoint: Codable {
    let methodId: String
    let endpoint: String
}

// MARK: - Errors

enum PrivateEndpointStorageError: LocalizedError {
    case encodingFailed
    case decodingFailed
    case notFound(peer: String, methodId: String)
    
    var errorDescription: String? {
        switch self {
        case .encodingFailed:
            return "Failed to encode private endpoints"
        case .decodingFailed:
            return "Failed to decode private endpoints"
        case .notFound(let peer, let methodId):
            return "Private endpoint not found for peer \(peer), method \(methodId)"
        }
    }
}

