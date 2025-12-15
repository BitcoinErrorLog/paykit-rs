//
//  AutoPayView.swift
//  PaykitMobile
//
//  Auto-Pay settings UI component for Bitkit integration.
//  This is a template that Bitkit can adapt to their design system.
//

import SwiftUI
import PaykitMobile

/// Auto-Pay view component
public struct BitkitAutoPayView: View {
    @ObservedObject var viewModel: BitkitAutoPayViewModel
    
    public init(viewModel: BitkitAutoPayViewModel) {
        self.viewModel = viewModel
    }
    
    public var body: some View {
        NavigationView {
            List {
                // Auto-Pay Status Section
                Section {
                    HStack {
                        VStack(alignment: .leading) {
                            Text("Auto-Pay")
                                .font(.headline)
                            Text(viewModel.isEnabled ? "Enabled" : "Disabled")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                        Spacer()
                        Toggle("", isOn: $viewModel.isEnabled)
                            .labelsHidden()
                    }
                } header: {
                    Text("Status")
                }
                
                if viewModel.isEnabled {
                    // Global Spending Limit
                    Section {
                        VStack(alignment: .leading, spacing: 8) {
                            HStack {
                                Text("Daily Limit")
                                Spacer()
                                Text("\(viewModel.dailyLimit) sats")
                                    .foregroundColor(.secondary)
                            }
                            
                            Slider(
                                value: Binding(
                                    get: { Double(viewModel.dailyLimit) },
                                    set: { viewModel.dailyLimit = Int64($0) }
                                ),
                                in: 1000...1000000,
                                step: 1000
                            )
                            
                            HStack {
                                Text("Used Today")
                                Spacer()
                                Text("\(viewModel.usedToday) sats")
                                    .foregroundColor(viewModel.usedToday > viewModel.dailyLimit ? .red : .green)
                            }
                            
                            ProgressView(value: Double(viewModel.usedToday), total: Double(viewModel.dailyLimit))
                                .tint(viewModel.usedToday > viewModel.dailyLimit * 80 / 100 ? .orange : .green)
                        }
                        .padding(.vertical, 4)
                    } header: {
                        Text("Global Spending Limit")
                    }
                    
                    // Per-Peer Limits Section
                    Section {
                        ForEach(viewModel.peerLimits) { peerLimit in
                            PeerLimitRow(peerLimit: peerLimit) { updated in
                                viewModel.updatePeerLimit(updated)
                            }
                        }
                        .onDelete { indexSet in
                            viewModel.removePeerLimits(at: indexSet)
                        }
                        
                        Button(action: { viewModel.showingAddPeer = true }) {
                            Label("Add Peer Limit", systemImage: "plus")
                        }
                    } header: {
                        Text("Per-Peer Limits")
                    } footer: {
                        Text("Set individual spending limits for specific peers")
                    }
                    
                    // Auto-Pay Rules Section
                    Section {
                        ForEach(viewModel.autoPayRules) { rule in
                            AutoPayRuleRow(rule: rule)
                        }
                        .onDelete { indexSet in
                            viewModel.removeRules(at: indexSet)
                        }
                        
                        Button(action: { viewModel.showingAddRule = true }) {
                            Label("Add Rule", systemImage: "plus")
                        }
                    } header: {
                        Text("Auto-Pay Rules")
                    } footer: {
                        Text("Automatically approve payments matching these rules")
                    }
                    
                    // Recent Auto-Payments Section
                    Section {
                        if viewModel.recentPayments.isEmpty {
                            Text("No recent auto-payments")
                                .foregroundColor(.secondary)
                        } else {
                            ForEach(viewModel.recentPayments) { payment in
                                RecentPaymentRow(payment: payment)
                            }
                        }
                    } header: {
                        Text("Recent Auto-Payments")
                    }
                }
            }
            .navigationTitle("Auto-Pay")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Reset") {
                        viewModel.resetToDefaults()
                    }
                }
            }
            .sheet(isPresented: $viewModel.showingAddPeer) {
                AddPeerLimitSheet(viewModel: viewModel)
            }
            .sheet(isPresented: $viewModel.showingAddRule) {
                AddAutoPayRuleSheet(viewModel: viewModel)
            }
        }
    }
}

// MARK: - Supporting Views

struct PeerLimitRow: View {
    let peerLimit: PeerSpendingLimit
    let onUpdate: (PeerSpendingLimit) -> Void
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(peerLimit.peerName)
                .font(.headline)
            Text("Limit: \(peerLimit.limit) sats")
                .font(.caption)
                .foregroundColor(.secondary)
            Text("Used: \(peerLimit.used) sats")
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }
}

struct AutoPayRuleRow: View {
    let rule: AutoPayRule
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(rule.name)
                    .font(.headline)
                Spacer()
                if rule.isEnabled {
                    Image(systemName: "checkmark.circle.fill")
                        .foregroundColor(.green)
                } else {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundColor(.gray)
                }
            }
            Text(rule.description)
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }
}

struct RecentPaymentRow: View {
    let payment: RecentAutoPayment
    
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text(payment.peerName)
                    .font(.subheadline)
                    .fontWeight(.medium)
                Text(payment.description)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            Spacer()
            VStack(alignment: .trailing, spacing: 4) {
                Text("\(payment.amount) sats")
                    .font(.subheadline)
                Text(payment.formattedTime)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }
}

struct AddPeerLimitSheet: View {
    @ObservedObject var viewModel: BitkitAutoPayViewModel
    @Environment(\.dismiss) private var dismiss
    @State private var peerPubkey = ""
    @State private var peerName = ""
    @State private var limit: Int64 = 10000
    @State private var period: SpendingPeriod = .daily
    
    var body: some View {
        NavigationView {
            Form {
                Section("Peer Information") {
                    TextField("Peer Name", text: $peerName)
                    TextField("Public Key", text: $peerPubkey)
                        .autocapitalization(.none)
                }
                
                Section("Limit") {
                    HStack {
                        Text("Amount")
                        Spacer()
                        TextField("sats", value: $limit, format: .number)
                            .keyboardType(.numberPad)
                            .multilineTextAlignment(.trailing)
                    }
                    
                    Picker("Period", selection: $period) {
                        ForEach(SpendingPeriod.allCases, id: \.self) { period in
                            Text(period.rawValue).tag(period)
                        }
                    }
                }
                
                Section {
                    Button("Add Limit") {
                        let limit = PeerSpendingLimit(
                            id: peerPubkey,
                            peerPubkey: peerPubkey,
                            peerName: peerName,
                            limit: limit,
                            used: 0,
                            period: period,
                            periodStart: Date()
                        )
                        viewModel.addPeerLimit(limit)
                        dismiss()
                    }
                    .disabled(peerPubkey.isEmpty || peerName.isEmpty)
                }
            }
            .navigationTitle("Add Peer Limit")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                }
            }
        }
    }
}

struct AddAutoPayRuleSheet: View {
    @ObservedObject var viewModel: BitkitAutoPayViewModel
    @Environment(\.dismiss) private var dismiss
    @State private var ruleName = ""
    @State private var maxAmount: Int64? = nil
    @State private var methodFilter: String? = nil
    @State private var peerFilter: String? = nil
    
    var body: some View {
        NavigationView {
            Form {
                Section("Rule Information") {
                    TextField("Rule Name", text: $ruleName)
                }
                
                Section("Filters") {
                    HStack {
                        Text("Max Amount")
                        Spacer()
                        TextField("sats (optional)", value: $maxAmount, format: .number)
                            .keyboardType(.numberPad)
                            .multilineTextAlignment(.trailing)
                    }
                    
                    TextField("Method Filter (optional)", text: Binding(
                        get: { methodFilter ?? "" },
                        set: { methodFilter = $0.isEmpty ? nil : $0 }
                    ))
                    
                    TextField("Peer Filter (optional)", text: Binding(
                        get: { peerFilter ?? "" },
                        set: { peerFilter = $0.isEmpty ? nil : $0 }
                    ))
                }
                
                Section {
                    Button("Add Rule") {
                        let rule = AutoPayRule(
                            id: UUID().uuidString,
                            name: ruleName,
                            description: "Auto-approve payments",
                            isEnabled: true,
                            maxAmount: maxAmount,
                            methodFilter: methodFilter,
                            peerFilter: peerFilter
                        )
                        viewModel.addRule(rule)
                        dismiss()
                    }
                    .disabled(ruleName.isEmpty)
                }
            }
            .navigationTitle("Add Auto-Pay Rule")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                }
            }
        }
    }
}
