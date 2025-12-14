//
//  PaymentMethodsView.swift
//  PaykitDemo
//
//  View for displaying and selecting payment methods
//

import SwiftUI
import Combine

struct PaymentMethodsView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var viewModel = PaymentMethodsViewModel()
    
    var body: some View {
        NavigationView {
            List {
                // Privacy Features Section (moved to top)
                Section {
                    NavigationLink {
                        PrivateEndpointsView()
                    } label: {
                        HStack {
                            Image(systemName: "lock.shield.fill")
                                .foregroundColor(.green)
                                .frame(width: 30)
                            VStack(alignment: .leading) {
                                Text("Private Endpoints")
                                Text("Manage per-peer private addresses")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                    
                    NavigationLink {
                        RotationSettingsView()
                    } label: {
                        HStack {
                            Image(systemName: "arrow.triangle.2.circlepath")
                                .foregroundColor(.blue)
                                .frame(width: 30)
                            VStack(alignment: .leading) {
                                Text("Rotation Settings")
                                Text("Configure endpoint rotation policies")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                } header: {
                    Text("Privacy Features")
                }
                
                // Directory Publishing Section
                Section {
                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            Text("Publish to Directory")
                                .font(.headline)
                            Spacer()
                            Toggle("", isOn: $viewModel.isPublishingEnabled)
                        }
                        
                        if viewModel.isPublishingEnabled {
                            HStack {
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundColor(.green)
                                Text("\(viewModel.publishedMethodsCount) method(s) published")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            
                            Button("Publish All Methods") {
                                viewModel.publishAllMethods(client: appState.paykitClient)
                            }
                            .buttonStyle(.bordered)
                            .frame(maxWidth: .infinity)
                        } else {
                            Text("Methods are not publicly discoverable")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                    .padding(.vertical, 8)
                } header: {
                    Text("Directory Publishing")
                }
                
                // Directory Publishing Section
                Section {
                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            Text("Publish to Directory")
                                .font(.headline)
                            Spacer()
                            Toggle("", isOn: $viewModel.isPublishingEnabled)
                        }
                        
                        if viewModel.isPublishingEnabled {
                            HStack {
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundColor(.green)
                                Text("\(viewModel.publishedMethodsCount) method(s) published")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            
                            Button("Publish All Methods") {
                                viewModel.publishAllMethods()
                            }
                            .buttonStyle(.bordered)
                            .frame(maxWidth: .infinity)
                        } else {
                            Text("Methods are not publicly discoverable")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                    .padding(.vertical, 8)
                } header: {
                    Text("Directory Publishing")
                }
                
                // Available Methods Section
                Section {
                    ForEach(viewModel.methods, id: \.id) { method in
                        PaymentMethodRow(method: method)
                    }
                } header: {
                    Text("Available Methods")
                } footer: {
                    Text("Tap a method to see details and validate endpoints")
                }
                
                // Health Status Section
                Section {
                    ForEach(viewModel.healthResults, id: \.methodId) { result in
                        HealthStatusRow(result: result)
                    }
                    
                    Button("Refresh Health") {
                        viewModel.checkHealth(client: appState.paykitClient)
                    }
                } header: {
                    Text("Health Status")
                } footer: {
                    Text("Health status auto-refreshes when screen appears")
                }
                
                // Method Selection Section
                Section {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Test Method Selection")
                            .font(.headline)
                        
                        HStack {
                            Text("Amount:")
                            Spacer()
                            TextField("sats", value: $viewModel.testAmount, format: .number)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 100)
                                .multilineTextAlignment(.trailing)
                        }
                        
                        Picker("Strategy", selection: $viewModel.selectionStrategy) {
                            Text("Balanced").tag(SelectionStrategy.balanced)
                            Text("Cost").tag(SelectionStrategy.costOptimized)
                            Text("Speed").tag(SelectionStrategy.speedOptimized)
                            Text("Privacy").tag(SelectionStrategy.privacyOptimized)
                        }
                        .pickerStyle(.segmented)
                        
                        Button("Select Best Method") {
                            viewModel.selectMethod(client: appState.paykitClient)
                        }
                        .buttonStyle(.borderedProminent)
                        .frame(maxWidth: .infinity)
                        
                        if let result = viewModel.selectionResult {
                            VStack(alignment: .leading, spacing: 4) {
                                HStack {
                                    Text("Selected:")
                                    Text(result.primaryMethod)
                                        .fontWeight(.bold)
                                        .foregroundColor(.green)
                                }
                                Text(result.reason)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            .padding(.top, 8)
                        }
                    }
                    .padding(.vertical, 8)
                } header: {
                    Text("Selection Test")
                }
                
                // Endpoint Validation Section
                Section {
                    VStack(alignment: .leading, spacing: 12) {
                        Picker("Method", selection: $viewModel.validationMethod) {
                            ForEach(viewModel.methods, id: \.id) { method in
                                Text(method.name).tag(method.id)
                            }
                        }
                        
                        TextField("Endpoint (address/invoice)", text: $viewModel.validationEndpoint)
                            .textFieldStyle(.roundedBorder)
                            .autocapitalization(.none)
                        
                        HStack {
                            Button("Validate") {
                                viewModel.validateEndpoint(client: appState.paykitClient)
                            }
                            .buttonStyle(.borderedProminent)
                            
                            if let isValid = viewModel.validationResult {
                                Image(systemName: isValid ? "checkmark.circle.fill" : "xmark.circle.fill")
                                    .foregroundColor(isValid ? .green : .red)
                                    .font(.title2)
                            }
                        }
                    }
                    .padding(.vertical, 8)
                } header: {
                    Text("Endpoint Validation")
                }
                
                // Privacy Features Section
                Section {
                    NavigationLink {
                        PrivateEndpointsView()
                    } label: {
                        HStack {
                            Image(systemName: "lock.shield.fill")
                                .foregroundColor(.green)
                                .frame(width: 30)
                            VStack(alignment: .leading) {
                                Text("Private Endpoints")
                                Text("Manage per-peer private addresses")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                    
                    NavigationLink {
                        RotationSettingsView()
                    } label: {
                        HStack {
                            Image(systemName: "arrow.triangle.2.circlepath")
                                .foregroundColor(.blue)
                                .frame(width: 30)
                            VStack(alignment: .leading) {
                                Text("Rotation Settings")
                                Text("Configure endpoint rotation policies")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                } header: {
                    Text("Privacy Features")
                } footer: {
                    Text("Enhance privacy by using dedicated endpoints per peer and automatically rotating them after use.")
                }
            }
            .navigationTitle("Payment Methods")
            .onAppear {
                viewModel.loadMethods(client: appState.paykitClient)
                viewModel.checkHealth(client: appState.paykitClient)
                viewModel.loadPublishingStatus(client: appState.paykitClient)
            }
        }
    }
}

struct PaymentMethodRow: View {
    let method: PaymentMethodInfo
    
    var body: some View {
        HStack {
            Image(systemName: method.icon)
                .font(.title2)
                .foregroundColor(.blue)
                .frame(width: 40)
            
            VStack(alignment: .leading) {
                Text(method.name)
                    .font(.headline)
                Text(method.description)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            if method.isUsable {
                Image(systemName: "checkmark.circle.fill")
                    .foregroundColor(.green)
            }
        }
    }
}

struct HealthStatusRow: View {
    let result: HealthCheckResult
    
    var body: some View {
        HStack {
            Text(result.methodId)
                .font(.headline)
            
            Spacer()
            
            HStack(spacing: 8) {
                if let latency = result.latencyMs {
                    Text("\(latency)ms")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Circle()
                    .fill(statusColor(result.status))
                    .frame(width: 12, height: 12)
                
                Text(statusText(result.status))
                    .font(.caption)
            }
        }
    }
    
    private func statusColor(_ status: HealthStatus) -> Color {
        switch status {
        case .healthy: return .green
        case .degraded: return .orange
        case .unavailable: return .red
        case .unknown: return .gray
        }
    }
    
    private func statusText(_ status: HealthStatus) -> String {
        switch status {
        case .healthy: return "Healthy"
        case .degraded: return "Degraded"
        case .unavailable: return "Unavailable"
        case .unknown: return "Unknown"
        }
    }
}

// MARK: - View Model

class PaymentMethodsViewModel: ObservableObject {
    @Published var methods: [PaymentMethodInfo] = []
    @Published var healthResults: [HealthCheckResult] = []
    @Published var testAmount: Int64 = 10000
    @Published var selectionStrategy: SelectionStrategy = .balanced
    @Published var selectionResult: SelectionResult?
    @Published var validationMethod: String = "onchain"
    @Published var validationEndpoint: String = ""
    @Published var validationResult: Bool?
    @Published var isPublishingEnabled: Bool = false
    @Published var publishedMethodsCount: Int = 0
    
    func loadMethods(client: PaykitClientWrapper) {
        let methodIds = client.listMethods()
        methods = methodIds.map { id in
            PaymentMethodInfo(
                id: id,
                name: id == "lightning" ? "Lightning Network" : "Bitcoin On-Chain",
                description: id == "lightning" 
                    ? "Fast, low-fee payments" 
                    : "Standard Bitcoin transactions",
                icon: id == "lightning" ? "bolt.fill" : "bitcoinsign.circle",
                isUsable: client.isMethodUsable(methodId: id)
            )
        }
    }
    
    func checkHealth(client: PaykitClientWrapper) {
        healthResults = client.checkHealth()
    }
    
    func loadPublishingStatus(client: PaykitClientWrapper) {
        // Check which methods are published
        let keyManager = KeyManager()
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        let methodStorage = PaymentMethodStorage(identityName: identityName)
        let allMethods = methodStorage.listMethods()
        publishedMethodsCount = allMethods.filter { $0.isPublic }.count
        isPublishingEnabled = publishedMethodsCount > 0
    }
    
    func publishAllMethods(client: PaykitClientWrapper) {
        let keyManager = KeyManager()
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        let methodStorage = PaymentMethodStorage(identityName: identityName)
        let allMethods = methodStorage.listMethods()
        
        for method in allMethods {
            if !method.isPublic {
                try? methodStorage.setPublic(methodId: method.methodId, isPublic: true)
            }
        }
        
        loadPublishingStatus(client: client)
    }
    
    func selectMethod(client: PaykitClientWrapper) {
        let paymentMethods = methods.map { method in
            PaymentMethod(
                methodId: method.id,
                endpoint: method.id == "lightning" ? "lnbc..." : "bc1q..."
            )
        }
        
        let prefs = SelectionPreferences(
            strategy: selectionStrategy,
            excludedMethods: [],
            maxFeeSats: nil,
            maxConfirmationTimeSecs: nil
        )
        
        selectionResult = client.selectMethod(
            methods: paymentMethods,
            amountSats: UInt64(testAmount),
            preferences: prefs
        )
    }
    
    func validateEndpoint(client: PaykitClientWrapper) {
        validationResult = client.validateEndpoint(
            methodId: validationMethod,
            endpoint: validationEndpoint
        )
    }
}

struct PaymentMethodInfo: Identifiable {
    let id: String
    let name: String
    let description: String
    let icon: String
    let isUsable: Bool
}

#Preview {
    PaymentMethodsView()
        .environmentObject(AppState())
}
