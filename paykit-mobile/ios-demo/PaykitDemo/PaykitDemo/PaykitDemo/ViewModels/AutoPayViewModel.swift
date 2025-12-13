//
//  AutoPayViewModel.swift
//  PaykitDemo
//
//  ViewModel for auto-pay settings and spending limits
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
    
    private let storage = UserDefaults.standard
    private let storageKeyPrefix = "paykit.autopay."
    
    // MARK: - Initialization
    
    init() {
        loadSettings()
        loadSampleData()
    }
    
    // MARK: - Peer Limits
    
    func addPeerLimit(_ limit: PeerSpendingLimit) {
        peerLimits.append(limit)
        savePeerLimits()
    }
    
    func updatePeerLimit(_ limit: PeerSpendingLimit) {
        if let index = peerLimits.firstIndex(where: { $0.id == limit.id }) {
            peerLimits[index] = limit
            savePeerLimits()
        }
    }
    
    func removePeerLimits(at offsets: IndexSet) {
        peerLimits.remove(atOffsets: offsets)
        savePeerLimits()
    }
    
    // MARK: - Auto-Pay Rules
    
    func addRule(_ rule: AutoPayRule) {
        autoPayRules.append(rule)
        saveRules()
    }
    
    func updateRule(_ rule: AutoPayRule) {
        if let index = autoPayRules.firstIndex(where: { $0.id == rule.id }) {
            autoPayRules[index] = rule
            saveRules()
        }
    }
    
    func removeRules(at offsets: IndexSet) {
        autoPayRules.remove(atOffsets: offsets)
        saveRules()
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
        
        saveRecentPayments()
    }
    
    // MARK: - Reset
    
    func resetToDefaults() {
        isEnabled = false
        dailyLimit = 100000
        usedToday = 0
        peerLimits = []
        autoPayRules = []
        recentPayments = []
        saveSettings()
    }
    
    func resetDailyUsage() {
        usedToday = 0
        for i in peerLimits.indices {
            if peerLimits[i].period == .daily {
                peerLimits[i].used = 0
            }
        }
        savePeerLimits()
    }
    
    // MARK: - Persistence
    
    private func saveSettings() {
        storage.set(isEnabled, forKey: storageKeyPrefix + "enabled")
        storage.set(dailyLimit, forKey: storageKeyPrefix + "dailyLimit")
        storage.set(usedToday, forKey: storageKeyPrefix + "usedToday")
    }
    
    private func loadSettings() {
        isEnabled = storage.bool(forKey: storageKeyPrefix + "enabled")
        dailyLimit = storage.object(forKey: storageKeyPrefix + "dailyLimit") as? Int64 ?? 100000
        usedToday = storage.object(forKey: storageKeyPrefix + "usedToday") as? Int64 ?? 0
    }
    
    private func savePeerLimits() {
        if let data = try? JSONEncoder().encode(peerLimits) {
            storage.set(data, forKey: storageKeyPrefix + "peerLimits")
        }
    }
    
    private func saveRules() {
        if let data = try? JSONEncoder().encode(autoPayRules) {
            storage.set(data, forKey: storageKeyPrefix + "rules")
        }
    }
    
    private func saveRecentPayments() {
        if let data = try? JSONEncoder().encode(recentPayments) {
            storage.set(data, forKey: storageKeyPrefix + "recentPayments")
        }
    }
    
    private func loadSampleData() {
        // Add sample data for demo purposes
        if peerLimits.isEmpty {
            peerLimits = [
                PeerSpendingLimit(
                    id: "1",
                    peerPubkey: "pk1abc123def456...",
                    peerName: "Alice's Store",
                    limit: 50000,
                    used: 12500,
                    period: .daily,
                    periodStart: Date()
                ),
                PeerSpendingLimit(
                    id: "2",
                    peerPubkey: "pk1xyz789ghi012...",
                    peerName: "Coffee Shop",
                    limit: 10000,
                    used: 3200,
                    period: .daily,
                    periodStart: Date()
                ),
            ]
        }
        
        if autoPayRules.isEmpty {
            autoPayRules = [
                AutoPayRule(
                    id: "1",
                    name: "Small Lightning Payments",
                    description: "Auto-approve Lightning payments under 1000 sats",
                    isEnabled: true,
                    maxAmount: 1000,
                    methodFilter: "lightning",
                    peerFilter: nil
                ),
                AutoPayRule(
                    id: "2",
                    name: "Trusted Merchants",
                    description: "Auto-approve all payments from verified merchants",
                    isEnabled: false,
                    maxAmount: 10000,
                    methodFilter: nil,
                    peerFilter: nil
                ),
            ]
        }
        
        if recentPayments.isEmpty {
            let now = Date()
            recentPayments = [
                RecentAutoPayment(
                    id: "1",
                    peerPubkey: "pk1abc...",
                    peerName: "Alice's Store",
                    amount: 500,
                    description: "Monthly subscription",
                    timestamp: now.addingTimeInterval(-3600),
                    status: .completed,
                    ruleId: "1"
                ),
                RecentAutoPayment(
                    id: "2",
                    peerPubkey: "pk1xyz...",
                    peerName: "Coffee Shop",
                    amount: 320,
                    description: "Morning coffee",
                    timestamp: now.addingTimeInterval(-7200),
                    status: .completed,
                    ruleId: "1"
                ),
            ]
        }
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
