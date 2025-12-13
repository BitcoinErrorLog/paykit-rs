//
//  SubscriptionsView.swift
//  PaykitDemo
//
//  View for managing subscriptions with persistent storage
//

import SwiftUI
import Combine

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
                            SubscriptionRow(subscription: sub, viewModel: viewModel)
                        }
                        .onDelete(perform: viewModel.deleteSubscriptions)
                    }
                } header: {
                    HStack {
                        Text("Active Subscriptions")
                        Spacer()
                        if !viewModel.subscriptions.isEmpty {
                            Text("\(viewModel.subscriptions.count)")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                }
                
                // Create New Subscription
                Section {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Create Subscription")
                            .font(.headline)
                        
                        TextField("Provider Name", text: $viewModel.providerName)
                            .textFieldStyle(.roundedBorder)
                        
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
                        
                        Picker("Frequency", selection: $viewModel.frequencyString) {
                            Text("Daily").tag("daily")
                            Text("Weekly").tag("weekly")
                            Text("Monthly").tag("monthly")
                            Text("Yearly").tag("yearly")
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
                        .disabled(viewModel.providerPubkey.isEmpty || viewModel.providerName.isEmpty)
                        
                        if let error = viewModel.errorMessage {
                            Text(error)
                                .font(.caption)
                                .foregroundColor(.red)
                        }
                        
                        if viewModel.showSuccess {
                            Text("Subscription created!")
                                .font(.caption)
                                .foregroundColor(.green)
                        }
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
            .refreshable {
                viewModel.loadSubscriptions()
            }
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { viewModel.loadSubscriptions() }) {
                        Image(systemName: "arrow.clockwise")
                    }
                }
            }
            .onAppear {
                viewModel.loadSubscriptions()
            }
        }
    }
}

struct SubscriptionRow: View {
    let subscription: StoredSubscription
    @ObservedObject var viewModel: SubscriptionsViewModel
    
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
                    Text(subscription.frequency.capitalized)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            HStack {
                if let nextPayment = subscription.nextPaymentAt {
                    Text("Next payment:")
                    Text(nextPayment, style: .date)
                        .foregroundColor(.secondary)
                } else {
                    Text("No scheduled payment")
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                Button(action: { viewModel.toggleSubscription(id: subscription.id) }) {
                    Text(subscription.isActive ? "Active" : "Paused")
                        .font(.caption)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 2)
                        .background(subscription.isActive ? Color.green.opacity(0.2) : Color.gray.opacity(0.2))
                        .foregroundColor(subscription.isActive ? .green : .gray)
                        .cornerRadius(4)
                }
            }
            
            if subscription.paymentCount > 0 {
                Text("\(subscription.paymentCount) payment(s) made")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - View Model

class SubscriptionsViewModel: ObservableObject {
    @Published var subscriptions: [StoredSubscription] = []
    @Published var showingAddSubscription = false
    @Published var errorMessage: String?
    @Published var showSuccess = false
    
    // New subscription form
    @Published var providerName = ""
    @Published var providerPubkey = ""
    @Published var amount: Int64 = 1000
    @Published var frequencyString = "monthly"
    @Published var methodId = "lightning"
    @Published var description = ""
    
    // Proration calculator
    @Published var prorationCurrentAmount: Int64 = 1000
    @Published var prorationNewAmount: Int64 = 2000
    @Published var daysIntoPeriod: Int = 15
    @Published var prorationResult: ProrationResult?
    
    private let storage = SubscriptionStorage()
    
    init() {
        loadSubscriptions()
    }
    
    func loadSubscriptions() {
        subscriptions = storage.listSubscriptions()
    }
    
    func createSubscription(client: PaykitClientWrapper) {
        errorMessage = nil
        showSuccess = false
        
        // Convert frequency string to PaymentFrequency for FFI call
        let frequency: PaymentFrequency
        switch frequencyString {
        case "daily": frequency = .daily
        case "weekly": frequency = .weekly
        case "yearly": frequency = .yearly(month: 1, day: 1)
        default: frequency = .monthly(dayOfMonth: 1)
        }
        
        let terms = SubscriptionTerms(
            amountSats: amount,
            currency: "SAT",
            frequency: frequency,
            methodId: methodId,
            description: description
        )
        
        // Create via FFI (validates with protocol)
        if let _ = client.createSubscription(
            subscriber: "pk1subscriber...", // Would use actual key
            provider: providerPubkey,
            terms: terms
        ) {
            // Create stored subscription with persistence
            let storedSub = StoredSubscription(
                providerName: providerName,
                providerPubkey: providerPubkey,
                amountSats: amount,
                currency: "SAT",
                frequency: frequencyString,
                description: description,
                methodId: methodId
            )
            
            do {
                try storage.saveSubscription(storedSub)
                loadSubscriptions()
                
                // Reset form
                providerName = ""
                providerPubkey = ""
                description = ""
                showSuccess = true
                
                // Hide success message after 2 seconds
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
                    self.showSuccess = false
                }
            } catch {
                errorMessage = "Failed to save: \(error.localizedDescription)"
            }
        } else {
            errorMessage = "Failed to create subscription via FFI"
        }
    }
    
    func toggleSubscription(id: String) {
        do {
            try storage.toggleActive(id: id)
            loadSubscriptions()
        } catch {
            errorMessage = "Failed to toggle: \(error.localizedDescription)"
        }
    }
    
    func deleteSubscriptions(at offsets: IndexSet) {
        for index in offsets {
            let sub = subscriptions[index]
            do {
                try storage.deleteSubscription(id: sub.id)
            } catch {
                errorMessage = "Failed to delete: \(error.localizedDescription)"
            }
        }
        loadSubscriptions()
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
}

#Preview {
    SubscriptionsView()
        .environmentObject(AppState())
}
