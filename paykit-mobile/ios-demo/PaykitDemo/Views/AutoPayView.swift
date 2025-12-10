//
//  AutoPayView.swift
//  PaykitDemo
//
//  Auto-pay settings and spending limits management
//

import SwiftUI

struct AutoPayView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var viewModel = AutoPayViewModel()
    
    var body: some View {
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
                AddPeerLimitView(viewModel: viewModel)
            }
            .sheet(isPresented: $viewModel.showingAddRule) {
                AddAutoPayRuleView(viewModel: viewModel)
            }
        }
    }
}

// MARK: - Supporting Views

struct PeerLimitRow: View {
    let peerLimit: PeerSpendingLimit
    let onUpdate: (PeerSpendingLimit) -> Void
    
    @State private var isEditing = false
    @State private var newLimit: Int64
    
    init(peerLimit: PeerSpendingLimit, onUpdate: @escaping (PeerSpendingLimit) -> Void) {
        self.peerLimit = peerLimit
        self.onUpdate = onUpdate
        self._newLimit = State(initialValue: peerLimit.limit)
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                VStack(alignment: .leading) {
                    Text(peerLimit.peerName)
                        .font(.headline)
                    Text(peerLimit.peerPubkey.prefix(16) + "...")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                VStack(alignment: .trailing) {
                    Text("\(peerLimit.limit) sats")
                        .font(.subheadline)
                    Text("\(peerLimit.period.rawValue)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            HStack {
                Text("Used")
                ProgressView(value: Double(peerLimit.used), total: Double(peerLimit.limit))
                    .frame(maxWidth: .infinity)
                Text("\(peerLimit.used)/\(peerLimit.limit)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 4)
    }
}

struct AutoPayRuleRow: View {
    let rule: AutoPayRule
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Image(systemName: rule.isEnabled ? "checkmark.circle.fill" : "circle")
                    .foregroundColor(rule.isEnabled ? .green : .gray)
                
                VStack(alignment: .leading) {
                    Text(rule.name)
                        .font(.headline)
                    Text(rule.description)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                if let maxAmount = rule.maxAmount {
                    Text("≤ \(maxAmount) sats")
                        .font(.caption)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Color.blue.opacity(0.1))
                        .cornerRadius(8)
                }
            }
        }
    }
}

struct RecentPaymentRow: View {
    let payment: RecentAutoPayment
    
    var body: some View {
        HStack {
            VStack(alignment: .leading) {
                Text(payment.peerName)
                    .font(.headline)
                Text(payment.description)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            VStack(alignment: .trailing) {
                Text("\(payment.amount) sats")
                    .font(.subheadline)
                    .foregroundColor(payment.status == .completed ? .green : .orange)
                Text(payment.timestamp, style: .relative)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }
}

struct AddPeerLimitView: View {
    @ObservedObject var viewModel: AutoPayViewModel
    @Environment(\.dismiss) var dismiss
    
    @State private var peerPubkey = ""
    @State private var peerName = ""
    @State private var limit: Int64 = 10000
    @State private var period: SpendingPeriod = .daily
    
    var body: some View {
        NavigationView {
            Form {
                Section("Peer Information") {
                    TextField("Peer Public Key", text: $peerPubkey)
                        .textContentType(.none)
                        .autocapitalization(.none)
                    
                    TextField("Display Name", text: $peerName)
                }
                
                Section("Spending Limit") {
                    Stepper("Limit: \(limit) sats", value: $limit, in: 100...1000000, step: 1000)
                    
                    Picker("Period", selection: $period) {
                        ForEach(SpendingPeriod.allCases, id: \.self) { p in
                            Text(p.rawValue).tag(p)
                        }
                    }
                }
            }
            .navigationTitle("Add Peer Limit")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Add") {
                        let newLimit = PeerSpendingLimit(
                            id: UUID().uuidString,
                            peerPubkey: peerPubkey,
                            peerName: peerName.isEmpty ? "Unknown" : peerName,
                            limit: limit,
                            used: 0,
                            period: period,
                            periodStart: Date()
                        )
                        viewModel.addPeerLimit(newLimit)
                        dismiss()
                    }
                    .disabled(peerPubkey.isEmpty)
                }
            }
        }
    }
}

struct AddAutoPayRuleView: View {
    @ObservedObject var viewModel: AutoPayViewModel
    @Environment(\.dismiss) var dismiss
    
    @State private var name = ""
    @State private var description = ""
    @State private var maxAmount: Int64 = 1000
    @State private var hasMaxAmount = true
    @State private var methodFilter: String? = nil
    @State private var peerFilter: String? = nil
    
    var body: some View {
        NavigationView {
            Form {
                Section("Rule Details") {
                    TextField("Rule Name", text: $name)
                    TextField("Description", text: $description)
                }
                
                Section("Conditions") {
                    Toggle("Maximum Amount", isOn: $hasMaxAmount)
                    
                    if hasMaxAmount {
                        Stepper("Max: \(maxAmount) sats", value: $maxAmount, in: 100...100000, step: 100)
                    }
                    
                    Picker("Payment Method", selection: $methodFilter) {
                        Text("Any").tag(nil as String?)
                        Text("Lightning").tag("lightning" as String?)
                        Text("On-Chain").tag("onchain" as String?)
                    }
                }
            }
            .navigationTitle("Add Rule")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Add") {
                        let newRule = AutoPayRule(
                            id: UUID().uuidString,
                            name: name,
                            description: description,
                            isEnabled: true,
                            maxAmount: hasMaxAmount ? maxAmount : nil,
                            methodFilter: methodFilter,
                            peerFilter: nil
                        )
                        viewModel.addRule(newRule)
                        dismiss()
                    }
                    .disabled(name.isEmpty)
                }
            }
        }
    }
}

#Preview {
    AutoPayView()
        .environmentObject(AppState())
}
