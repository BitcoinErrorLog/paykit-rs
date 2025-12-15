//
//  SubscriptionsView.swift
//  PaykitMobile
//
//  Subscriptions UI component for Bitkit integration.
//  This is a template that Bitkit can adapt to their design system.
//

import SwiftUI
import PaykitMobile

/// Subscriptions view model for Bitkit integration
public class BitkitSubscriptionsViewModel: ObservableObject {
    @Published public var subscriptions: [Subscription] = []
    @Published public var isLoading = false
    @Published public var errorMessage: String?
    @Published public var showError = false
    @Published public var showSuccess = false
    
    // Create subscription form
    @Published public var providerName = ""
    @Published public var providerPubkey = ""
    @Published public var amount: Int64 = 1000
    @Published public var frequencyString = "monthly"
    @Published public var methodId = "lightning"
    @Published public var description = ""
    
    // Proration calculator
    @Published public var prorationCurrentAmount: Int64 = 1000
    @Published public var prorationNewAmount: Int64 = 2000
    @Published public var daysIntoPeriod: Int32 = 15
    @Published public var prorationResult: ProrationResult?
    
    private let paykitClient: PaykitClient
    private let subscriptionStorage: SubscriptionStorageProtocol
    
    public init(
        paykitClient: PaykitClient,
        subscriptionStorage: SubscriptionStorageProtocol
    ) {
        self.paykitClient = paykitClient
        self.subscriptionStorage = subscriptionStorage
    }
    
    func loadSubscriptions() {
        isLoading = true
        subscriptions = subscriptionStorage.activeSubscriptions()
        isLoading = false
    }
    
    func createSubscription(myPublicKey: String) {
        guard !providerPubkey.isEmpty, !providerName.isEmpty else {
            errorMessage = "Please fill in provider information"
            showError = true
            return
        }
        
        isLoading = true
        
        Task {
            do {
                let frequency: PaymentFrequency
                switch frequencyString {
                case "daily": frequency = .daily
                case "weekly": frequency = .weekly
                case "monthly": frequency = .monthly
                case "yearly": frequency = .yearly
                default: frequency = .monthly
                }
                
                let terms = SubscriptionTerms(
                    amountSats: amount,
                    currency: "SAT",
                    frequency: frequency,
                    methodId: methodId,
                    description: description
                )
                
                let subscription = try paykitClient.createSubscription(
                    subscriber: myPublicKey,
                    provider: providerPubkey,
                    terms: terms
                )
                
                await MainActor.run {
                    subscriptionStorage.addSubscription(subscription)
                    subscriptions.append(subscription)
                    isLoading = false
                    showSuccess = true
                    resetForm()
                }
            } catch {
                await MainActor.run {
                    isLoading = false
                    errorMessage = error.localizedDescription
                    showError = true
                }
            }
        }
    }
    
    func calculateProration() {
        Task {
            do {
                let periodStart = Int64(Date().timeIntervalSince1970) - (Int64(daysIntoPeriod) * 86400)
                let periodEnd = periodStart + (30 * 86400) // 30 days
                let changeDate = Int64(Date().timeIntervalSince1970)
                
                let result = try paykitClient.calculateProration(
                    currentAmountSats: prorationCurrentAmount,
                    newAmountSats: prorationNewAmount,
                    periodStart: periodStart,
                    periodEnd: periodEnd,
                    changeDate: changeDate
                )
                
                await MainActor.run {
                    prorationResult = result
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    showError = true
                }
            }
        }
    }
    
    func deleteSubscriptions(at offsets: IndexSet) {
        let toDelete = offsets.map { subscriptions[$0] }
        subscriptions.remove(atOffsets: offsets)
        for subscription in toDelete {
            subscriptionStorage.deleteSubscription(id: subscription.subscriptionId)
        }
    }
    
    private func resetForm() {
        providerName = ""
        providerPubkey = ""
        amount = 1000
        frequencyString = "monthly"
        methodId = "lightning"
        description = ""
    }
}

/// Subscriptions view component
public struct BitkitSubscriptionsView: View {
    @ObservedObject var viewModel: BitkitSubscriptionsViewModel
    private let myPublicKey: String
    
    public init(viewModel: BitkitSubscriptionsViewModel, myPublicKey: String) {
        self.viewModel = viewModel
        self.myPublicKey = myPublicKey
    }
    
    public var body: some View {
        NavigationView {
            List {
                // Active Subscriptions
                Section {
                    if viewModel.subscriptions.isEmpty {
                        Text("No active subscriptions")
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(viewModel.subscriptions, id: \.subscriptionId) { sub in
                            SubscriptionRow(subscription: sub)
                        }
                        .onDelete(perform: viewModel.deleteSubscriptions)
                    }
                } header: {
                    Text("Active Subscriptions")
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
                            viewModel.createSubscription(myPublicKey: myPublicKey)
                        }
                        .buttonStyle(.borderedProminent)
                        .frame(maxWidth: .infinity)
                        .disabled(viewModel.providerPubkey.isEmpty || viewModel.providerName.isEmpty || viewModel.isLoading)
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
                            viewModel.calculateProration()
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
                            }
                        }
                    }
                    .padding(.vertical, 8)
                } header: {
                    Text("Proration Calculator")
                }
            }
            .navigationTitle("Subscriptions")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Refresh") {
                        viewModel.loadSubscriptions()
                    }
                }
            }
            .alert("Error", isPresented: $viewModel.showError) {
                Button("OK") { }
            } message: {
                Text(viewModel.errorMessage ?? "Unknown error")
            }
            .alert("Success", isPresented: $viewModel.showSuccess) {
                Button("OK") { }
            } message: {
                Text("Subscription created successfully!")
            }
            .onAppear {
                viewModel.loadSubscriptions()
            }
        }
    }
}

struct SubscriptionRow: View {
    let subscription: Subscription
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(subscription.provider)
                    .font(.headline)
                Spacer()
                if subscription.isActive {
                    Text("Active")
                        .font(.caption)
                        .foregroundColor(.green)
                } else {
                    Text("Inactive")
                        .font(.caption)
                        .foregroundColor(.gray)
                }
            }
            
            Text("\(subscription.terms.amountSats) sats / \(subscription.terms.frequency)")
                .font(.subheadline)
                .foregroundColor(.secondary)
            
            Text(subscription.terms.description)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.vertical, 4)
    }
}
