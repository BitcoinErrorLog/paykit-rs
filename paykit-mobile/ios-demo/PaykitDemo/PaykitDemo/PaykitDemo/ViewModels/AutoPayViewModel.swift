//
//  AutoPayViewModel.swift
//  PaykitDemo
//
//  ViewModel for auto-pay settings and spending limits
//  Uses Keychain-backed AutoPayStorage for secure persistence
//

import Foundation
import SwiftUI
import Combine

class AutoPayViewModel: ObservableObject {
    // MARK: - Published Properties
    
    @Published var isEnabled: Bool = false {
        didSet { saveSettings() }
    }
    
    @Published var dailyLimit: Int64 = 100000 {
        didSet { saveSettings() }
    }
    
    @Published var usedToday: Int64 = 0
    
    @Published var peerLimits: [PeerSpendingLimit] = []
    @Published var autoPayRules: [AutoPayRule] = []
    @Published var recentPayments: [RecentAutoPayment] = []
    
    @Published var showingAddPeer = false
    @Published var showingAddRule = false
    
    // MARK: - Private Properties
    
    private let autoPayStorage = AutoPayStorage()
    
    // MARK: - Initialization
    
    init() {
        loadFromStorage()
    }
    
    // MARK: - Peer Limits
    
    func addPeerLimit(_ limit: PeerSpendingLimit) {
        peerLimits.append(limit)
        savePeerLimitToStorage(limit)
    }
    
    func updatePeerLimit(_ limit: PeerSpendingLimit) {
        if let index = peerLimits.firstIndex(where: { $0.id == limit.id }) {
            peerLimits[index] = limit
            savePeerLimitToStorage(limit)
        }
    }
    
    func removePeerLimits(at offsets: IndexSet) {
        let toRemove = offsets.map { peerLimits[$0] }
        peerLimits.remove(atOffsets: offsets)
        for limit in toRemove {
            try? autoPayStorage.deletePeerLimit(id: limit.id)
        }
    }
    
    // MARK: - Auto-Pay Rules
    
    func addRule(_ rule: AutoPayRule) {
        autoPayRules.append(rule)
        saveRuleToStorage(rule)
    }
    
    func updateRule(_ rule: AutoPayRule) {
        if let index = autoPayRules.firstIndex(where: { $0.id == rule.id }) {
            autoPayRules[index] = rule
            saveRuleToStorage(rule)
        }
    }
    
    func removeRules(at offsets: IndexSet) {
        let toRemove = offsets.map { autoPayRules[$0] }
        autoPayRules.remove(atOffsets: offsets)
        for rule in toRemove {
            try? autoPayStorage.deleteRule(id: rule.id)
        }
    }
    
    // MARK: - Auto-Pay Logic
    
    /// Check if a payment should be auto-approved
    func shouldAutoApprove(
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
            // Check amount filter
            if let maxAmount = rule.maxAmount, amount > maxAmount {
                continue
            }
            
            // Check method filter
            if let methodFilter = rule.methodFilter, methodFilter != methodId {
                continue
            }
            
            // Check peer filter
            if let peerFilter = rule.peerFilter, peerFilter != peerPubkey {
                continue
            }
            
            // Rule matched
            return .approved(ruleId: rule.id, ruleName: rule.name)
        }
        
        return .needsApproval
    }
    
    /// Record an auto-approved payment
    func recordPayment(
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
        
        // Note: Recent payments are kept in-memory for demo purposes
        // Could be persisted to Keychain if needed
    }
    
    // MARK: - Reset
    
    func resetToDefaults() {
        isEnabled = false
        dailyLimit = 100000
        usedToday = 0
        peerLimits = []
        autoPayRules = []
        recentPayments = []
        
        // Clear storage
        var settings = autoPayStorage.getSettings()
        settings.isEnabled = false
        settings.globalDailyLimitSats = 100000
        settings.currentDailySpentSats = 0
        try? autoPayStorage.saveSettings(settings)
        
        // Clear all limits and rules
        for limit in autoPayStorage.getPeerLimits() {
            try? autoPayStorage.deletePeerLimit(id: limit.id)
        }
        for rule in autoPayStorage.getRules() {
            try? autoPayStorage.deleteRule(id: rule.id)
        }
    }
    
    func resetDailyUsage() {
        usedToday = 0
        for i in peerLimits.indices {
            if peerLimits[i].period == .daily {
                peerLimits[i].used = 0
                savePeerLimitToStorage(peerLimits[i])
            }
        }
        
        // Also update global settings
        var settings = autoPayStorage.getSettings()
        settings.currentDailySpentSats = 0
        try? autoPayStorage.saveSettings(settings)
    }
    
    // MARK: - Persistence
    
    private func saveSettings() {
        var settings = autoPayStorage.getSettings()
        settings.isEnabled = isEnabled
        settings.globalDailyLimitSats = dailyLimit
        settings.currentDailySpentSats = usedToday
        try? autoPayStorage.saveSettings(settings)
    }
    
    private func loadFromStorage() {
        // Load global settings
        let settings = autoPayStorage.getSettings()
        isEnabled = settings.isEnabled
        dailyLimit = settings.globalDailyLimitSats
        usedToday = settings.currentDailySpentSats
        
        // Load peer limits - convert from storage type to view model type
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
        
        // Load auto-pay rules - convert from storage type to view model type
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
        
        // Recent payments stay in-memory for demo (could add to storage later)
        recentPayments = []
    }
    
    private func savePeerLimitToStorage(_ limit: PeerSpendingLimit) {
        // Convert view model type to storage type
        var storedLimit = StoredPeerLimit(
            peerPubkey: limit.peerPubkey,
            peerName: limit.peerName,
            limitSats: limit.limit,
            period: limit.period.rawValue.lowercased()
        )
        storedLimit.spentSats = limit.used
        try? autoPayStorage.savePeerLimit(storedLimit)
    }
    
    private func saveRuleToStorage(_ rule: AutoPayRule) {
        // Convert view model type to storage type
        var storedRule = StoredAutoPayRule(
            name: rule.name,
            maxAmountSats: rule.maxAmount,
            allowedMethods: rule.methodFilter.map { [$0] } ?? [],
            allowedPeers: rule.peerFilter.map { [$0] } ?? []
        )
        storedRule.isEnabled = rule.isEnabled
        try? autoPayStorage.saveRule(storedRule)
    }
}

// MARK: - Auto-Approval Result

enum AutoApprovalResult {
    case approved(ruleId: String, ruleName: String)
    case denied(reason: String)
    case needsApproval
    
    var isApproved: Bool {
        if case .approved = self { return true }
        return false
    }
}
