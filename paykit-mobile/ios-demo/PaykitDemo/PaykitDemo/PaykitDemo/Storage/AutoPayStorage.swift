//
//  AutoPayStorage.swift
//  PaykitDemo
//
//  Persistent storage for auto-pay settings using Keychain.
//

import Foundation

/// Auto-pay global settings
struct AutoPaySettings: Codable {
    var isEnabled: Bool
    var globalDailyLimitSats: Int64
    var currentDailySpentSats: Int64
    var lastResetDate: Date
    var requireConfirmation: Bool
    var notifyOnPayment: Bool
    
    init() {
        self.isEnabled = false
        self.globalDailyLimitSats = 100000 // 100k sats default
        self.currentDailySpentSats = 0
        self.lastResetDate = Date()
        self.requireConfirmation = false
        self.notifyOnPayment = true
    }
    
    mutating func resetIfNeeded() {
        let calendar = Calendar.current
        if !calendar.isDateInToday(lastResetDate) {
            currentDailySpentSats = 0
            lastResetDate = Date()
        }
    }
    
    var remainingDailyLimitSats: Int64 {
        max(0, globalDailyLimitSats - currentDailySpentSats)
    }
    
    var dailyUsagePercent: Double {
        guard globalDailyLimitSats > 0 else { return 0 }
        return Double(currentDailySpentSats) / Double(globalDailyLimitSats) * 100
    }
}

/// A peer-specific spending limit
struct PeerSpendingLimit: Identifiable, Codable {
    let id: String
    var peerPubkey: String
    var peerName: String
    var limitSats: Int64
    var spentSats: Int64
    var period: String  // daily, weekly, monthly
    var lastResetDate: Date
    
    init(peerPubkey: String, peerName: String, limitSats: Int64, period: String = "daily") {
        self.id = peerPubkey
        self.peerPubkey = peerPubkey
        self.peerName = peerName
        self.limitSats = limitSats
        self.spentSats = 0
        self.period = period
        self.lastResetDate = Date()
    }
    
    mutating func resetIfNeeded() {
        let calendar = Calendar.current
        let shouldReset: Bool
        
        switch period.lowercased() {
        case "daily":
            shouldReset = !calendar.isDateInToday(lastResetDate)
        case "weekly":
            shouldReset = !calendar.isDate(lastResetDate, equalTo: Date(), toGranularity: .weekOfYear)
        case "monthly":
            shouldReset = !calendar.isDate(lastResetDate, equalTo: Date(), toGranularity: .month)
        default:
            shouldReset = false
        }
        
        if shouldReset {
            spentSats = 0
            lastResetDate = Date()
        }
    }
    
    var remainingSats: Int64 {
        max(0, limitSats - spentSats)
    }
    
    var usagePercent: Double {
        guard limitSats > 0 else { return 0 }
        return Double(spentSats) / Double(limitSats) * 100
    }
}

/// An auto-pay rule
struct AutoPayRule: Identifiable, Codable {
    let id: String
    var name: String
    var isEnabled: Bool
    var maxAmountSats: Int64?
    var allowedMethods: [String]
    var allowedPeers: [String]  // Empty = all peers
    var requireConfirmation: Bool
    var createdAt: Date
    
    init(
        name: String,
        maxAmountSats: Int64? = nil,
        allowedMethods: [String] = [],
        allowedPeers: [String] = []
    ) {
        self.id = UUID().uuidString
        self.name = name
        self.isEnabled = true
        self.maxAmountSats = maxAmountSats
        self.allowedMethods = allowedMethods
        self.allowedPeers = allowedPeers
        self.requireConfirmation = false
        self.createdAt = Date()
    }
    
    func matches(amount: Int64, method: String, peer: String) -> Bool {
        guard isEnabled else { return false }
        
        // Check amount
        if let max = maxAmountSats, amount > max {
            return false
        }
        
        // Check method
        if !allowedMethods.isEmpty && !allowedMethods.contains(method) {
            return false
        }
        
        // Check peer
        if !allowedPeers.isEmpty && !allowedPeers.contains(peer) {
            return false
        }
        
        return true
    }
}

/// Manages persistent storage of auto-pay settings
class AutoPayStorage {
    
    private let keychain: KeychainStorage
    private let settingsKey = "paykit.autopay.settings"
    private let limitsKey = "paykit.autopay.limits"
    private let rulesKey = "paykit.autopay.rules"
    
    // In-memory cache
    private var settingsCache: AutoPaySettings?
    private var limitsCache: [PeerSpendingLimit]?
    private var rulesCache: [AutoPayRule]?
    
    init(keychain: KeychainStorage = KeychainStorage(serviceIdentifier: "com.paykit.demo")) {
        self.keychain = keychain
    }
    
    // MARK: - Settings
    
    func getSettings() -> AutoPaySettings {
        if var cached = settingsCache {
            cached.resetIfNeeded()
            return cached
        }
        
        do {
            guard let data = try keychain.retrieve(key: settingsKey) else {
                return AutoPaySettings()
            }
            var settings = try JSONDecoder().decode(AutoPaySettings.self, from: data)
            settings.resetIfNeeded()
            settingsCache = settings
            return settings
        } catch {
            print("AutoPayStorage: Failed to load settings: \(error)")
            return AutoPaySettings()
        }
    }
    
    func saveSettings(_ settings: AutoPaySettings) throws {
        let data = try JSONEncoder().encode(settings)
        try keychain.store(key: settingsKey, data: data)
        settingsCache = settings
    }
    
    // MARK: - Peer Limits
    
    func getPeerLimits() -> [PeerSpendingLimit] {
        if var cached = limitsCache {
            for i in cached.indices {
                cached[i].resetIfNeeded()
            }
            return cached
        }
        
        do {
            guard let data = try keychain.retrieve(key: limitsKey) else {
                return []
            }
            var limits = try JSONDecoder().decode([PeerSpendingLimit].self, from: data)
            for i in limits.indices {
                limits[i].resetIfNeeded()
            }
            limitsCache = limits
            return limits
        } catch {
            print("AutoPayStorage: Failed to load limits: \(error)")
            return []
        }
    }
    
    func savePeerLimit(_ limit: PeerSpendingLimit) throws {
        var limits = getPeerLimits()
        if let index = limits.firstIndex(where: { $0.id == limit.id }) {
            limits[index] = limit
        } else {
            limits.append(limit)
        }
        try persistLimits(limits)
    }
    
    func deletePeerLimit(id: String) throws {
        var limits = getPeerLimits()
        limits.removeAll { $0.id == id }
        try persistLimits(limits)
    }
    
    // MARK: - Rules
    
    func getRules() -> [AutoPayRule] {
        if let cached = rulesCache {
            return cached
        }
        
        do {
            guard let data = try keychain.retrieve(key: rulesKey) else {
                return []
            }
            let rules = try JSONDecoder().decode([AutoPayRule].self, from: data)
            rulesCache = rules
            return rules
        } catch {
            print("AutoPayStorage: Failed to load rules: \(error)")
            return []
        }
    }
    
    func saveRule(_ rule: AutoPayRule) throws {
        var rules = getRules()
        if let index = rules.firstIndex(where: { $0.id == rule.id }) {
            rules[index] = rule
        } else {
            rules.append(rule)
        }
        try persistRules(rules)
    }
    
    func deleteRule(id: String) throws {
        var rules = getRules()
        rules.removeAll { $0.id == id }
        try persistRules(rules)
    }
    
    // MARK: - Private
    
    private func persistLimits(_ limits: [PeerSpendingLimit]) throws {
        let data = try JSONEncoder().encode(limits)
        try keychain.store(key: limitsKey, data: data)
        limitsCache = limits
    }
    
    private func persistRules(_ rules: [AutoPayRule]) throws {
        let data = try JSONEncoder().encode(rules)
        try keychain.store(key: rulesKey, data: data)
        rulesCache = rules
    }
}

