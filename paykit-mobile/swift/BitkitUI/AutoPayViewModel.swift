//
//  AutoPayViewModel.swift
//  PaykitMobile
//
//  Auto-pay ViewModel for Bitkit integration.
//  This is adapted from the demo app to work with Bitkit's storage.
//

import Foundation
import SwiftUI
import Combine
import PaykitMobile

/// Auto-pay ViewModel for Bitkit integration
/// Bitkit should adapt this to use their storage mechanism
public class BitkitAutoPayViewModel: ObservableObject, AutopayEvaluator {
    // MARK: - Published Properties
    
    @Published public var isEnabled: Bool = false {
        didSet { saveSettings() }
    }
    
    @Published public var dailyLimit: Int64 = 100000 {
        didSet { saveSettings() }
    }
    
    @Published public var usedToday: Int64 = 0
    
    @Published public var peerLimits: [PeerSpendingLimit] = []
    @Published public var autoPayRules: [AutoPayRule] = []
    @Published public var recentPayments: [RecentAutoPayment] = []
    
    @Published public var showingAddPeer = false
    @Published public var showingAddRule = false
    
    // MARK: - Private Properties
    
    private let autoPayStorage: AutoPayStorageProtocol
    private let identityName: String
    
    // MARK: - Initialization
    
    public init(
        identityName: String,
        autoPayStorage: AutoPayStorageProtocol
    ) {
        self.identityName = identityName
        self.autoPayStorage = autoPayStorage
        loadFromStorage()
    }
    
    // MARK: - Peer Limits
    
    public func addPeerLimit(_ limit: PeerSpendingLimit) {
        peerLimits.append(limit)
        savePeerLimitToStorage(limit)
    }
    
    public func updatePeerLimit(_ limit: PeerSpendingLimit) {
        if let index = peerLimits.firstIndex(where: { $0.id == limit.id }) {
            peerLimits[index] = limit
            savePeerLimitToStorage(limit)
        }
    }
    
    public func removePeerLimits(at offsets: IndexSet) {
        let toRemove = offsets.map { peerLimits[$0] }
        peerLimits.remove(atOffsets: offsets)
        for limit in toRemove {
            autoPayStorage.deletePeerLimit(id: limit.id)
        }
    }
    
    // MARK: - Auto-Pay Rules
    
    public func addRule(_ rule: AutoPayRule) {
        autoPayRules.append(rule)
        saveRuleToStorage(rule)
    }
    
    public func updateRule(_ rule: AutoPayRule) {
        if let index = autoPayRules.firstIndex(where: { $0.id == rule.id }) {
            autoPayRules[index] = rule
            saveRuleToStorage(rule)
        }
    }
    
    public func removeRules(at offsets: IndexSet) {
        let toRemove = offsets.map { autoPayRules[$0] }
        autoPayRules.remove(atOffsets: offsets)
        for rule in toRemove {
            autoPayStorage.deleteRule(id: rule.id)
        }
    }
    
    // MARK: - Auto-Pay Logic
    
    /// Check if a payment should be auto-approved
    public func shouldAutoApprove(
        peerPubkey: String,
        amount: Int64,
        methodId: String
    ) -> AutoApprovalResult {
        guard isEnabled else {
            return .denied(reason: "Auto-pay is disabled")
        }
        
        // Check global daily limit
        if usedToday + amount > dailyLimit {
            return .denied(reason: "Would exceed daily limit")
        }
        
        // Check peer-specific limit
        if let peerLimit = peerLimits.first(where: { $0.peerPubkey == peerPubkey }) {
            if peerLimit.used + amount > peerLimit.limit {
                return .denied(reason: "Would exceed peer limit")
            }
        }
        
        // Check auto-pay rules
        for rule in autoPayRules where rule.isEnabled {
            if rule.matches(peerPubkey: peerPubkey, amount: amount, methodId: methodId) {
                return .approved(ruleId: rule.id, ruleName: rule.name)
            }
        }
        
        return .needsApproval
    }
    
    /// Record an auto-approved payment
    public func recordPayment(
        peerPubkey: String,
        peerName: String,
        amount: Int64,
        description: String,
        ruleId: String?
    ) {
        // Update global usage
        usedToday += amount
        
        // Update peer usage
        if let index = peerLimits.firstIndex(where: { $0.peerPubkey == peerPubkey }) {
            peerLimits[index].used += amount
            savePeerLimitToStorage(peerLimits[index])
        }
        
        // Add to recent payments
        let payment = RecentAutoPayment(
            id: UUID().uuidString,
            peerPubkey: peerPubkey,
            peerName: peerName,
            amount: amount,
            description: description,
            timestamp: Date(),
            status: .completed,
            ruleId: ruleId
        )
        recentPayments.insert(payment, at: 0)
        
        // Keep only last 50 payments
        if recentPayments.count > 50 {
            recentPayments = Array(recentPayments.prefix(50))
        }
    }
    
    // MARK: - AutopayEvaluator Protocol
    
    public func evaluate(peerPubkey: String, amount: Int64, methodId: String) -> AutopayEvaluationResult {
        let result = shouldAutoApprove(peerPubkey: peerPubkey, amount: amount, methodId: methodId)
        
        switch result {
        case .approved(let ruleId, let ruleName):
            return .approved(ruleId: ruleId, ruleName: ruleName)
        case .denied(let reason):
            return .denied(reason: reason)
        case .needsApproval:
            return .needsApproval
        }
    }
    
    // MARK: - Reset
    
    public func resetToDefaults() {
        isEnabled = false
        dailyLimit = 100000
        usedToday = 0
        peerLimits = []
        autoPayRules = []
        recentPayments = []
        
        var settings = autoPayStorage.getSettings()
        settings.isEnabled = false
        settings.globalDailyLimit = 100000
        autoPayStorage.saveSettings(settings)
        
        // Clear all limits and rules
        for limit in autoPayStorage.getPeerLimits() {
            autoPayStorage.deletePeerLimit(id: limit.id)
        }
        for rule in autoPayStorage.getRules() {
            autoPayStorage.deleteRule(id: rule.id)
        }
    }
    
    public func resetDailyUsage() {
        usedToday = 0
        for i in peerLimits.indices {
            if peerLimits[i].period == .daily {
                peerLimits[i].used = 0
                savePeerLimitToStorage(peerLimits[i])
            }
        }
    }
    
    // MARK: - Persistence
    
    private func saveSettings() {
        var settings = autoPayStorage.getSettings()
        settings.isEnabled = isEnabled
        settings.globalDailyLimit = dailyLimit
        autoPayStorage.saveSettings(settings)
    }
    
    private func loadFromStorage() {
        let settings = autoPayStorage.getSettings()
        isEnabled = settings.isEnabled
        dailyLimit = settings.globalDailyLimit
        usedToday = 0  // Bitkit should track this separately or load from storage
        
        // Load peer limits
        peerLimits = autoPayStorage.getPeerLimits().map { storedLimit in
            PeerSpendingLimit(
                id: storedLimit.id,
                peerPubkey: storedLimit.peerPubkey,
                peerName: storedLimit.peerName,
                limit: storedLimit.limitSats,
                used: storedLimit.spentSats,
                period: SpendingPeriod(rawValue: storedLimit.period.capitalized) ?? .daily,
                periodStart: storedLimit.lastResetDate
            )
        }
        
        // Load auto-pay rules
        autoPayRules = autoPayStorage.getRules().map { storedRule in
            AutoPayRule(
                id: storedRule.id,
                name: storedRule.name,
                description: "Max: \(storedRule.maxAmountSats ?? 0) sats",
                isEnabled: storedRule.isEnabled,
                maxAmount: storedRule.maxAmountSats,
                methodFilter: storedRule.allowedMethods.first,
                peerFilter: storedRule.allowedPeers.first
            )
        }
    }
    
    private func savePeerLimitToStorage(_ limit: PeerSpendingLimit) {
        let storedLimit = StoredPeerLimit(
            id: limit.id,
            peerPubkey: limit.peerPubkey,
            peerName: limit.peerName,
            limitSats: limit.limit,
            spentSats: limit.used,
            period: limit.period.rawValue.lowercased(),
            lastResetDate: limit.periodStart
        )
        autoPayStorage.savePeerLimit(storedLimit)
    }
    
    private func saveRuleToStorage(_ rule: AutoPayRule) {
        let storedRule = StoredAutoPayRule(
            id: rule.id,
            name: rule.name,
            isEnabled: rule.isEnabled,
            maxAmountSats: rule.maxAmount,
            allowedMethods: rule.methodFilter.map { [$0] } ?? [],
            allowedPeers: rule.peerFilter.map { [$0] } ?? [],
            requireConfirmation: false,
            createdAt: Date()
        )
        autoPayStorage.saveRule(storedRule)
    }
}

// MARK: - Auto-Pay Storage Protocol

/// Protocol for auto-pay storage that Bitkit must implement
public protocol AutoPayStorageProtocol {
    func getSettings() -> AutoPaySettings
    func saveSettings(_ settings: AutoPaySettings)
    func getPeerLimits() -> [StoredPeerLimit]
    func savePeerLimit(_ limit: StoredPeerLimit)
    func deletePeerLimit(id: String)
    func getRules() -> [StoredAutoPayRule]
    func saveRule(_ rule: StoredAutoPayRule)
    func deleteRule(id: String)
}

// MARK: - Storage Models

public struct StoredPeerLimit: Identifiable, Codable {
    public let id: String
    public var peerPubkey: String
    public var peerName: String
    public var limitSats: Int64
    public var spentSats: Int64
    public var period: String  // daily, weekly, monthly
    public var lastResetDate: Date
    
    public init(
        id: String,
        peerPubkey: String,
        peerName: String,
        limitSats: Int64,
        spentSats: Int64 = 0,
        period: String = "daily",
        lastResetDate: Date = Date()
    ) {
        self.id = id
        self.peerPubkey = peerPubkey
        self.peerName = peerName
        self.limitSats = limitSats
        self.spentSats = spentSats
        self.period = period
        self.lastResetDate = lastResetDate
    }
    
    public mutating func resetIfNeeded() {
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
            self.lastResetDate = Date()
        }
    }
}

public struct StoredAutoPayRule: Identifiable, Codable {
    public let id: String
    public var name: String
    public var isEnabled: Bool
    public var maxAmountSats: Int64?
    public var allowedMethods: [String]
    public var allowedPeers: [String]
    public var requireConfirmation: Bool
    public var createdAt: Date
    
    public init(
        id: String = UUID().uuidString,
        name: String,
        isEnabled: Bool = true,
        maxAmountSats: Int64? = nil,
        allowedMethods: [String] = [],
        allowedPeers: [String] = [],
        requireConfirmation: Bool = false,
        createdAt: Date = Date()
    ) {
        self.id = id
        self.name = name
        self.isEnabled = isEnabled
        self.maxAmountSats = maxAmountSats
        self.allowedMethods = allowedMethods
        self.allowedPeers = allowedPeers
        self.requireConfirmation = requireConfirmation
        self.createdAt = createdAt
    }
}

// MARK: - Auto-Approval Result

public enum AutoApprovalResult {
    case approved(ruleId: String, ruleName: String)
    case denied(reason: String)
    case needsApproval
    
    public var isApproved: Bool {
        if case .approved = self { return true }
        return false
    }
}
