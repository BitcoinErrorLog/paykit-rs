// NoiseKeyCache.swift
// X25519 Key Cache for Noise Protocol
//
// This file provides caching for derived X25519 keys used in Noise protocol
// communications. Keys are cached both in-memory and persistently in the
// Keychain to reduce round trips to Pubky Ring.
//
// Key Architecture:
//   - Ed25519 (pkarr) keys are "cold" and managed by Pubky Ring
//   - X25519 (Noise) keys are "hot" and cached locally in Paykit
//   - This cache stores the hot X25519 keys for quick access

import Foundation

/// Cache for X25519 Noise protocol keys
///
/// This class provides:
/// - In-memory caching for fast access
/// - Persistent storage in Keychain for app restarts
/// - Key rotation support via epoch parameter
/// - Secure clearing of keys when needed
public final class NoiseKeyCache {
    
    // MARK: - Singleton
    
    public static let shared = NoiseKeyCache()
    
    // MARK: - Constants
    
    private enum Keys {
        static func cacheKey(deviceId: String, epoch: UInt32) -> String {
            return "noise.key.cache.\(deviceId).\(epoch)"
        }
        static let cacheIndex = "noise.key.cache.index"
    }
    
    // MARK: - Properties
    
    private let keychain: KeychainStorage
    private var memoryCache: [String: X25519KeypairResult] = [:]
    private let cacheQueue = DispatchQueue(label: "com.paykit.noise.cache", attributes: .concurrent)
    
    /// Maximum number of keys to keep in cache (per device)
    public var maxCachedEpochs: Int = 5
    
    // MARK: - Initialization
    
    private init() {
        self.keychain = KeychainStorage(serviceIdentifier: Bundle.main.bundleIdentifier ?? "com.paykit.demo")
        loadCacheIndex()
    }
    
    // MARK: - Public Methods
    
    /// Get a cached key if available
    ///
    /// - Parameters:
    ///   - deviceId: Device identifier used for derivation
    ///   - epoch: Key rotation epoch
    /// - Returns: Cached keypair or nil if not found
    public func getKey(deviceId: String, epoch: UInt32) -> X25519KeypairResult? {
        let key = Keys.cacheKey(deviceId: deviceId, epoch: epoch)
        
        // Check memory cache first (fast path)
        var result: X25519KeypairResult?
        cacheQueue.sync {
            result = memoryCache[key]
        }
        
        if let cached = result {
            return cached
        }
        
        // Check persistent cache
        if let keypair = loadFromKeychain(key: key) {
            // Populate memory cache
            cacheQueue.async(flags: .barrier) {
                self.memoryCache[key] = keypair
            }
            return keypair
        }
        
        return nil
    }
    
    /// Store a key in the cache
    ///
    /// - Parameters:
    ///   - keypair: The keypair to cache
    ///   - deviceId: Device identifier used for derivation
    ///   - epoch: Key rotation epoch
    public func setKey(_ keypair: X25519KeypairResult, deviceId: String, epoch: UInt32) {
        let key = Keys.cacheKey(deviceId: deviceId, epoch: epoch)
        
        // Store in memory cache
        cacheQueue.async(flags: .barrier) {
            self.memoryCache[key] = keypair
        }
        
        // Store in keychain
        saveToKeychain(keypair: keypair, key: key)
        
        // Update cache index
        updateCacheIndex(add: key)
        
        // Cleanup old epochs if needed
        cleanupOldEpochs(deviceId: deviceId, currentEpoch: epoch)
    }
    
    /// Get or derive a key
    ///
    /// This method checks the cache first, then derives a new key if needed.
    /// Uses PubkyRingIntegration for derivation.
    ///
    /// - Parameters:
    ///   - deviceId: Device identifier
    ///   - epoch: Key rotation epoch
    /// - Returns: Keypair (from cache or freshly derived)
    public func getOrDerive(deviceId: String, epoch: UInt32) async throws -> X25519KeypairResult {
        // Check cache
        if let cached = getKey(deviceId: deviceId, epoch: epoch) {
            return cached
        }
        
        // Derive new key
        let keypair = try await PubkyRingIntegration.shared.deriveX25519Keypair(
            deviceId: deviceId,
            epoch: epoch
        )
        
        // Cache it
        setKey(keypair, deviceId: deviceId, epoch: epoch)
        
        return keypair
    }
    
    /// Get the latest cached epoch for a device
    ///
    /// - Parameter deviceId: Device identifier
    /// - Returns: Latest epoch number or nil if no keys cached
    public func getLatestEpoch(deviceId: String) -> UInt32? {
        var latestEpoch: UInt32?
        
        cacheQueue.sync {
            let prefix = "noise.key.cache.\(deviceId)."
            for key in memoryCache.keys {
                if key.hasPrefix(prefix),
                   let epochStr = key.components(separatedBy: ".").last,
                   let epoch = UInt32(epochStr) {
                    if latestEpoch == nil || epoch > latestEpoch! {
                        latestEpoch = epoch
                    }
                }
            }
        }
        
        return latestEpoch
    }
    
    /// Clear a specific key from cache
    ///
    /// - Parameters:
    ///   - deviceId: Device identifier
    ///   - epoch: Key rotation epoch
    public func clearKey(deviceId: String, epoch: UInt32) {
        let key = Keys.cacheKey(deviceId: deviceId, epoch: epoch)
        
        // Remove from memory
        cacheQueue.async(flags: .barrier) {
            self.memoryCache.removeValue(forKey: key)
        }
        
        // Remove from keychain
        try? keychain.delete(key: key)
        
        // Update index
        updateCacheIndex(remove: key)
    }
    
    /// Clear all keys for a device
    ///
    /// - Parameter deviceId: Device identifier
    public func clearAllKeys(for deviceId: String) {
        let prefix = "noise.key.cache.\(deviceId)."
        
        cacheQueue.async(flags: .barrier) {
            let keysToRemove = self.memoryCache.keys.filter { $0.hasPrefix(prefix) }
            for key in keysToRemove {
                self.memoryCache.removeValue(forKey: key)
                try? self.keychain.delete(key: key)
                self.updateCacheIndex(remove: key)
            }
        }
    }
    
    /// Clear all cached keys
    public func clearAllKeys() {
        cacheQueue.async(flags: .barrier) {
            for key in self.memoryCache.keys {
                try? self.keychain.delete(key: key)
            }
            self.memoryCache.removeAll()
        }
        
        try? keychain.delete(key: Keys.cacheIndex)
    }
    
    /// Get cache statistics
    ///
    /// - Returns: Dictionary with cache statistics
    public func getCacheStats() -> [String: Any] {
        var stats: [String: Any] = [:]
        
        cacheQueue.sync {
            stats["memoryCount"] = memoryCache.count
            stats["keys"] = Array(memoryCache.keys)
        }
        
        return stats
    }
    
    // MARK: - Private Methods
    
    private func loadCacheIndex() {
        guard let indexData = try? keychain.retrieve(key: Keys.cacheIndex),
              let index = try? JSONDecoder().decode([String].self, from: indexData) else {
            return
        }
        
        // Load all indexed keys into memory cache
        for key in index {
            if let keypair = loadFromKeychain(key: key) {
                memoryCache[key] = keypair
            }
        }
    }
    
    private func loadFromKeychain(key: String) -> X25519KeypairResult? {
        guard let data = try? keychain.retrieve(key: key) else {
            return nil
        }
        
        return try? JSONDecoder().decode(X25519KeypairResult.self, from: data)
    }
    
    private func saveToKeychain(keypair: X25519KeypairResult, key: String) {
        guard let data = try? JSONEncoder().encode(keypair) else {
            return
        }
        
        try? keychain.store(key: key, data: data)
    }
    
    private func updateCacheIndex(add key: String) {
        var index = getCacheIndex()
        if !index.contains(key) {
            index.append(key)
            saveCacheIndex(index)
        }
    }
    
    private func updateCacheIndex(remove key: String) {
        var index = getCacheIndex()
        index.removeAll { $0 == key }
        saveCacheIndex(index)
    }
    
    private func getCacheIndex() -> [String] {
        guard let data = try? keychain.retrieve(key: Keys.cacheIndex),
              let index = try? JSONDecoder().decode([String].self, from: data) else {
            return []
        }
        return index
    }
    
    private func saveCacheIndex(_ index: [String]) {
        guard let data = try? JSONEncoder().encode(index) else {
            return
        }
        try? keychain.store(key: Keys.cacheIndex, data: data)
    }
    
    private func cleanupOldEpochs(deviceId: String, currentEpoch: UInt32) {
        let prefix = "noise.key.cache.\(deviceId)."
        
        cacheQueue.async(flags: .barrier) {
            // Get all epochs for this device
            var epochs: [(key: String, epoch: UInt32)] = []
            
            for key in self.memoryCache.keys {
                if key.hasPrefix(prefix),
                   let epochStr = key.components(separatedBy: ".").last,
                   let epoch = UInt32(epochStr) {
                    epochs.append((key, epoch))
                }
            }
            
            // Sort by epoch descending
            epochs.sort { $0.epoch > $1.epoch }
            
            // Remove old epochs beyond limit
            while epochs.count > self.maxCachedEpochs {
                let oldest = epochs.removeLast()
                self.memoryCache.removeValue(forKey: oldest.key)
                try? self.keychain.delete(key: oldest.key)
                self.updateCacheIndex(remove: oldest.key)
            }
        }
    }
}

