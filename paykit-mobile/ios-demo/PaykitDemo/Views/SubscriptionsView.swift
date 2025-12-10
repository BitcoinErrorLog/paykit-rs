//
//  SubscriptionsView.swift
//  PaykitDemo
//
//  View for managing subscriptions
//

import SwiftUI

struct SubscriptionsView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var viewModel = SubscriptionsViewModel()
    
    var body: some View {
        NavigationView {
            List {
                // Active Subscriptions
                Section {
                    if viewModel.subscriptions.isEmpty {
                        Text("No active subscriptions")
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(viewModel.subscriptions) { sub in
                            SubscriptionRow(subscription: sub)
                        }
                    }
                } header: {
                    Text("Active Subscriptions")
                }
                
                // Create New Subscription
                Section {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Create Subscription")
                            .font(.headline)
                        
                        TextField("Provider Public Key", text: $viewModel.providerPubkey)
                            .textFieldStyle(.roundedBorder)
                            .autocapitalization(.none)
                        
                        HStack {
                            Text("Amount:")
                            Spacer()
                            TextField("sats", value: $viewModel.amount, format: .number)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 100)
                                .multilineTextAlignment(.trailing)
                        }
                        
                        Picker("Frequency", selection: $viewModel.frequency) {
                            Text("Daily").tag(PaymentFrequency.daily)
                            Text("Weekly").tag(PaymentFrequency.weekly)
                            Text("Monthly").tag(PaymentFrequency.monthly(dayOfMonth: 1))
                        }
                        
                        Picker("Method", selection: $viewModel.methodId) {
                            Text("Lightning").tag("lightning")
                            Text("On-Chain").tag("onchain")
                        }
                        
                        TextField("Description", text: $viewModel.description)
                            .textFieldStyle(.roundedBorder)
                        
                        Button("Create Subscription") {
                            viewModel.createSubscription(client: appState.paykitClient)
                        }
                        .buttonStyle(.borderedProminent)
                        .frame(maxWidth: .infinity)
                        .disabled(viewModel.providerPubkey.isEmpty)
                    }
                    .padding(.vertical, 8)
                } header: {
                    Text("New Subscription")
                }
                
                // Proration Calculator
                Section {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Proration Calculator")
                            .font(.headline)
                        
                        HStack {
                            Text("Current Amount:")
                            Spacer()
                            TextField("sats", value: $viewModel.prorationCurrentAmount, format: .number)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 100)
                                .multilineTextAlignment(.trailing)
                        }
                        
                        HStack {
                            Text("New Amount:")
                            Spacer()
                            TextField("sats", value: $viewModel.prorationNewAmount, format: .number)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 100)
                                .multilineTextAlignment(.trailing)
                        }
                        
                        HStack {
                            Text("Days Into Period:")
                            Spacer()
                            TextField("days", value: $viewModel.daysIntoPeriod, format: .number)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 60)
                                .multilineTextAlignment(.trailing)
                            Text("/ 30")
                        }
                        
                        Button("Calculate Proration") {
                            viewModel.calculateProration(client: appState.paykitClient)
                        }
                        .buttonStyle(.borderedProminent)
                        
                        if let result = viewModel.prorationResult {
                            VStack(alignment: .leading, spacing: 4) {
                                Divider()
                                HStack {
                                    Text("Credit:")
                                    Spacer()
                                    Text("\(result.creditSats) sats")
                                }
                                HStack {
                                    Text("Charge:")
                                    Spacer()
                                    Text("\(result.chargeSats) sats")
                                }
                                HStack {
                                    Text("Net:")
                                    Spacer()
                                    Text("\(result.netSats) sats")
                                        .fontWeight(.bold)
                                        .foregroundColor(result.isRefund ? .green : .orange)
                                }
                                Text(result.isRefund ? "Refund due" : "Additional charge")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            .padding(.top, 8)
                        }
                    }
                    .padding(.vertical, 8)
                } header: {
                    Text("Proration")
                } footer: {
                    Text("Calculate charges when upgrading or downgrading mid-period")
                }
            }
            .navigationTitle("Subscriptions")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { viewModel.showingAddSubscription = true }) {
                        Image(systemName: "plus")
                    }
                }
            }
        }
    }
}

struct SubscriptionRow: View {
    let subscription: SubscriptionInfo
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading) {
                    Text(subscription.providerName)
                        .font(.headline)
                    Text(subscription.description)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                VStack(alignment: .trailing) {
                    Text("\(subscription.amountSats) sats")
                        .font(.subheadline)
                        .fontWeight(.medium)
                    Text(subscription.frequencyText)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            HStack {
                Text("Next payment:")
                Text(subscription.nextPaymentDate, style: .date)
                    .foregroundColor(.secondary)
                Spacer()
                if subscription.isActive {
                    Text("Active")
                        .font(.caption)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 2)
                        .background(Color.green.opacity(0.2))
                        .foregroundColor(.green)
                        .cornerRadius(4)
                }
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - View Model

class SubscriptionsViewModel: ObservableObject {
    @Published var subscriptions: [SubscriptionInfo] = []
    @Published var showingAddSubscription = false
    
    // New subscription form
    @Published var providerPubkey = ""
    @Published var amount: Int64 = 1000
    @Published var frequency: PaymentFrequency = .monthly(dayOfMonth: 1)
    @Published var methodId = "lightning"
    @Published var description = ""
    
    // Proration calculator
    @Published var prorationCurrentAmount: Int64 = 1000
    @Published var prorationNewAmount: Int64 = 2000
    @Published var daysIntoPeriod: Int = 15
    @Published var prorationResult: ProrationResult?
    
    init() {
        loadSampleSubscriptions()
    }
    
    func createSubscription(client: PaykitClientWrapper) {
        let terms = SubscriptionTerms(
            amountSats: amount,
            currency: "SAT",
            frequency: frequency,
            methodId: methodId,
            description: description
        )
        
        // In a real app, we'd use the actual subscriber key
        if let _ = client.createSubscription(
            subscriber: "pk1subscriber...",
            provider: providerPubkey,
            terms: terms
        ) {
            // Add to list
            let newSub = SubscriptionInfo(
                id: UUID().uuidString,
                providerId: providerPubkey,
                providerName: "New Provider",
                amountSats: amount,
                frequency: frequency,
                description: description,
                nextPaymentDate: Date().addingTimeInterval(86400 * 30),
                isActive: true
            )
            subscriptions.append(newSub)
            
            // Reset form
            providerPubkey = ""
            description = ""
        }
    }
    
    func calculateProration(client: PaykitClientWrapper) {
        let now = Date()
        let periodStart = now.addingTimeInterval(-Double(daysIntoPeriod) * 86400)
        let periodEnd = periodStart.addingTimeInterval(30 * 86400)
        
        prorationResult = client.calculateProration(
            currentAmountSats: prorationCurrentAmount,
            newAmountSats: prorationNewAmount,
            periodStart: Int64(periodStart.timeIntervalSince1970),
            periodEnd: Int64(periodEnd.timeIntervalSince1970),
            changeDate: Int64(now.timeIntervalSince1970)
        )
    }
    
    private func loadSampleSubscriptions() {
        subscriptions = [
            SubscriptionInfo(
                id: "1",
                providerId: "pk1provider1...",
                providerName: "Premium News",
                amountSats: 5000,
                frequency: .monthly(dayOfMonth: 1),
                description: "Monthly news subscription",
                nextPaymentDate: Date().addingTimeInterval(86400 * 15),
                isActive: true
            ),
            SubscriptionInfo(
                id: "2",
                providerId: "pk1provider2...",
                providerName: "Coffee Club",
                amountSats: 10000,
                frequency: .weekly,
                description: "Weekly coffee delivery",
                nextPaymentDate: Date().addingTimeInterval(86400 * 3),
                isActive: true
            ),
        ]
    }
}

struct SubscriptionInfo: Identifiable {
    let id: String
    let providerId: String
    let providerName: String
    let amountSats: Int64
    let frequency: PaymentFrequency
    let description: String
    let nextPaymentDate: Date
    let isActive: Bool
    
    var frequencyText: String {
        switch frequency {
        case .daily: return "Daily"
        case .weekly: return "Weekly"
        case .monthly: return "Monthly"
        case .yearly: return "Yearly"
        case .custom(let secs): return "Every \(secs / 86400) days"
        }
    }
}

#Preview {
    SubscriptionsView()
        .environmentObject(AppState())
}
