//
//  SettingsView.swift
//  PaykitDemo
//
//  Settings and configuration view
//

import SwiftUI
import Combine

struct SettingsView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var viewModel = SettingsViewModel()
    @StateObject private var keyManager = KeyManager()
    
    var body: some View {
        NavigationView {
            List {
                // Quick Access Section
                Section {
                    NavigationLink {
                        AutoPayView()
                    } label: {
                        HStack {
                            Image(systemName: "arrow.clockwise.circle.fill")
                                .foregroundColor(.orange)
                                .frame(width: 30)
                            Text("Auto-Pay")
                        }
                    }
                    
                    NavigationLink {
                        SubscriptionsView()
                    } label: {
                        HStack {
                            Image(systemName: "repeat.circle.fill")
                                .foregroundColor(.blue)
                                .frame(width: 30)
                            Text("Subscriptions")
                        }
                    }
                    
                    NavigationLink {
                        PaymentRequestsView()
                    } label: {
                        HStack {
                            Image(systemName: "envelope.circle.fill")
                                .foregroundColor(.purple)
                                .frame(width: 30)
                            Text("Payment Requests")
                        }
                    }
                    
                    NavigationLink {
                        PaymentMethodsView()
                    } label: {
                        HStack {
                            Image(systemName: "globe")
                                .foregroundColor(.green)
                                .frame(width: 30)
                            Text("Directory Publishing")
                        }
                    }
                    
                    NavigationLink {
                        PrivateEndpointsView()
                    } label: {
                        HStack {
                            Image(systemName: "lock.shield.fill")
                                .foregroundColor(.green)
                                .frame(width: 30)
                            Text("Private Endpoints")
                        }
                    }
                    
                    NavigationLink {
                        RotationSettingsView()
                    } label: {
                        HStack {
                            Image(systemName: "arrow.triangle.2.circlepath")
                                .foregroundColor(.blue)
                                .frame(width: 30)
                            Text("Rotation Settings")
                        }
                    }
                } header: {
                    Text("Quick Access")
                }
                
                // Features Section
                Section {
                    NavigationLink {
                        AutoPayView()
                    } label: {
                        Text("Auto-Pay")
                    }
                    
                    NavigationLink {
                        SubscriptionsView()
                    } label: {
                        Text("Subscriptions")
                    }
                    
                    NavigationLink {
                        PaymentRequestsView()
                    } label: {
                        Text("Payment Requests")
                    }
                } header: {
                    Text("Features")
                }
                
                // Privacy Section
                Section {
                    NavigationLink {
                        PrivateEndpointsView()
                    } label: {
                        Text("Private Endpoints")
                    }
                    
                    NavigationLink {
                        RotationSettingsView()
                    } label: {
                        Text("Rotation Settings")
                    }
                } header: {
                    Text("Privacy")
                }
                
                // Network Section
                Section {
                    NavigationLink {
                        PaymentMethodsView()
                    } label: {
                        Text("Directory Publishing")
                    }
                } header: {
                    Text("Network")
                }
                
                // App Info Section
                Section {
                    HStack {
                        Text("Version")
                        Spacer()
                        Text(viewModel.appVersion)
                            .foregroundColor(.secondary)
                    }
                    
                    HStack {
                        Text("Paykit Library")
                        Spacer()
                        Text(viewModel.paykitVersion)
                            .foregroundColor(.secondary)
                    }
                    
                    HStack {
                        Text("Client Status")
                        Spacer()
                        HStack {
                            Circle()
                                .fill(appState.paykitClient.isAvailable ? .green : .red)
                                .frame(width: 8, height: 8)
                            Text(appState.paykitClient.isAvailable ? "Connected" : "Error")
                                .foregroundColor(.secondary)
                        }
                    }
                } header: {
                    Text("About")
                }
                
                // Network Settings
                Section {
                    Picker("Network", selection: $viewModel.selectedNetwork) {
                        Text("Mainnet").tag(NetworkType.mainnet)
                        Text("Testnet").tag(NetworkType.testnet)
                        Text("Regtest").tag(NetworkType.regtest)
                    }
                    
                    Toggle("Use Testnet for Demo", isOn: $viewModel.useTestnet)
                } header: {
                    Text("Network")
                } footer: {
                    Text("Testnet uses test Bitcoin with no real value")
                }
                
                // Identity Settings
                Section {
                    HStack {
                        Text("Current Identity")
                        Spacer()
                        Text(keyManager.currentIdentityName ?? "None")
                            .foregroundColor(.secondary)
                    }
                    
                    NavigationLink("Manage Identities") {
                        IdentityListView()
                    }
                } header: {
                    Text("Identity")
                }
                
                // Security Settings
                Section {
                    Toggle("Require Face ID", isOn: $viewModel.requireBiometric)
                    
                    Toggle("Lock on Background", isOn: $viewModel.lockOnBackground)
                    
                    NavigationLink("Manage Keys") {
                        KeyManagementView(viewModel: viewModel)
                    }
                } header: {
                    Text("Security")
                }
                
                // Notification Settings
                Section {
                    Toggle("Payment Notifications", isOn: $viewModel.paymentNotifications)
                    Toggle("Subscription Reminders", isOn: $viewModel.subscriptionReminders)
                    Toggle("Auto-Pay Alerts", isOn: $viewModel.autoPayAlerts)
                    Toggle("Limit Warnings", isOn: $viewModel.limitWarnings)
                } header: {
                    Text("Notifications")
                }
                
                // Advanced Settings
                Section {
                    NavigationLink("Developer Options") {
                        DeveloperOptionsView(viewModel: viewModel)
                    }
                    
                    Button("Clear Cache") {
                        viewModel.clearCache()
                    }
                    .foregroundColor(.orange)
                    
                    Button("Reset All Settings") {
                        viewModel.showingResetConfirmation = true
                    }
                    .foregroundColor(.red)
                } header: {
                    Text("Advanced")
                }
                
                // Help Section
                Section {
                    Link("Documentation", destination: URL(string: "https://paykit.dev/docs")!)
                    Link("GitHub Repository", destination: URL(string: "https://github.com/paykit")!)
                    Link("Report Issue", destination: URL(string: "https://github.com/paykit/issues")!)
                } header: {
                    Text("Help & Support")
                }
            }
            .navigationTitle("Settings")
            .alert("Reset Settings", isPresented: $viewModel.showingResetConfirmation) {
                Button("Cancel", role: .cancel) { }
                Button("Reset", role: .destructive) {
                    viewModel.resetAllSettings()
                }
            } message: {
                Text("This will reset all settings to their defaults. This cannot be undone.")
            }
        }
    }
}

struct KeyManagementView: View {
    @ObservedObject var viewModel: SettingsViewModel
    @State private var exportPassword = ""
    @State private var importPassword = ""
    @State private var importBackupText = ""
    
    var body: some View {
        List {
            // Current Identity Section
            Section {
                if viewModel.hasIdentity {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("pkarr Identity (z-base32)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text(viewModel.publicKeyZ32)
                            .font(.system(.caption2, design: .monospaced))
                            .lineLimit(2)
                    }
                    .padding(.vertical, 4)
                    
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Hex Format")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text(viewModel.publicKey.prefix(32) + "...")
                            .font(.system(.caption2, design: .monospaced))
                    }
                    .padding(.vertical, 4)
                    
                    HStack {
                        Text("Device ID")
                        Spacer()
                        Text(viewModel.getDeviceId().prefix(12) + "...")
                            .font(.system(.caption, design: .monospaced))
                            .foregroundColor(.secondary)
                    }
                    
                    Button(action: { viewModel.copyPublicKey() }) {
                        Label("Copy Public Key", systemImage: "doc.on.doc")
                    }
                } else {
                    VStack(spacing: 12) {
                        Image(systemName: "key.fill")
                            .font(.largeTitle)
                            .foregroundColor(.orange)
                        Text("No Identity")
                            .font(.headline)
                        Text("Generate a new keypair or import from backup")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .multilineTextAlignment(.center)
                        
                        Button("Create Identity") {
                            viewModel.getOrCreateIdentity()
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.orange)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical)
                }
            } header: {
                Text("Your Identity")
            }
            
            // Export Section
            if viewModel.hasIdentity {
                Section {
                    Button(action: { viewModel.showingExportPassword = true }) {
                        Label("Export Encrypted Backup", systemImage: "arrow.up.doc")
                    }
                } header: {
                    Text("Backup")
                } footer: {
                    Text("Export your keys encrypted with a password for safe storage")
                }
            }
            
            // Import Section
            Section {
                Button(action: { viewModel.showingImportPassword = true }) {
                    Label("Import from Backup", systemImage: "arrow.down.doc")
                }
            } header: {
                Text("Restore")
            }
            
            // Generate New Section
            Section {
                Button(action: { viewModel.showingGenerateConfirmation = true }) {
                    Label("Generate New Keypair", systemImage: "key.horizontal")
                }
                .foregroundColor(.orange)
            } header: {
                Text("Advanced")
            } footer: {
                Text("⚠️ Generating a new keypair will replace your current identity. Make sure you have a backup first!")
            }
            
            // Error Display
            if let error = viewModel.keyError {
                Section {
                    Text(error)
                        .foregroundColor(.red)
                        .font(.caption)
                }
            }
        }
        .navigationTitle("Key Management")
        .alert("Generate New Keys?", isPresented: $viewModel.showingGenerateConfirmation) {
            Button("Cancel", role: .cancel) { }
            Button("Generate", role: .destructive) {
                viewModel.generateNewKeypair()
            }
        } message: {
            Text("This will replace your current keys. Make sure you have a backup!")
        }
        .alert("Export Backup", isPresented: $viewModel.showingExportPassword) {
            SecureField("Password", text: $exportPassword)
            Button("Cancel", role: .cancel) { exportPassword = "" }
            Button("Export") {
                viewModel.exportKeys(password: exportPassword)
                exportPassword = ""
            }
        } message: {
            Text("Enter a password to encrypt your backup")
        }
        .alert("Import Backup", isPresented: $viewModel.showingImportPassword) {
            TextField("Backup JSON", text: $importBackupText)
            SecureField("Password", text: $importPassword)
            Button("Cancel", role: .cancel) { 
                importBackupText = ""
                importPassword = "" 
            }
            Button("Import") {
                viewModel.importKeys(backupText: importBackupText, password: importPassword)
                importBackupText = ""
                importPassword = ""
            }
        } message: {
            Text("Paste your backup JSON and enter the password")
        }
        .sheet(isPresented: $viewModel.showingExportSheet) {
            ExportBackupSheet(backup: viewModel.exportedBackup)
        }
    }
}

struct ExportBackupSheet: View {
    let backup: String
    @Environment(\.dismiss) private var dismiss
    
    var body: some View {
        NavigationView {
            VStack(spacing: 20) {
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 60))
                    .foregroundColor(.green)
                
                Text("Backup Created!")
                    .font(.title2)
                    .fontWeight(.semibold)
                
                Text("Copy this encrypted backup and store it safely:")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)
                
                ScrollView {
                    Text(backup)
                        .font(.system(.caption2, design: .monospaced))
                        .padding()
                        .background(Color(.systemGray6))
                        .cornerRadius(8)
                }
                .frame(maxHeight: 300)
                
                Button(action: {
                    UIPasteboard.general.string = backup
                }) {
                    Label("Copy to Clipboard", systemImage: "doc.on.doc")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .tint(.orange)
                
                Spacer()
            }
            .padding()
            .navigationTitle("Export Backup")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") { dismiss() }
                }
            }
        }
    }
}

struct DeveloperOptionsView: View {
    @ObservedObject var viewModel: SettingsViewModel
    
    var body: some View {
        List {
            Section {
                Toggle("Debug Logging", isOn: $viewModel.debugLogging)
                Toggle("Show Request/Response", isOn: $viewModel.showRequestResponse)
                Toggle("Mock Payments", isOn: $viewModel.mockPayments)
            } header: {
                Text("Debug")
            }
            
            Section {
                Button("Trigger Test Payment") {
                    viewModel.triggerTestPayment()
                }
                
                Button("Simulate Auto-Pay") {
                    viewModel.simulateAutoPay()
                }
                
                Button("Test Notification") {
                    viewModel.sendTestNotification()
                }
            } header: {
                Text("Testing")
            }
            
            Section {
                HStack {
                    Text("Pending Payments")
                    Spacer()
                    Text("\(viewModel.pendingPaymentsCount)")
                        .foregroundColor(.secondary)
                }
                
                HStack {
                    Text("Cache Size")
                    Spacer()
                    Text(viewModel.cacheSize)
                        .foregroundColor(.secondary)
                }
                
                HStack {
                    Text("Last Sync")
                    Spacer()
                    Text(viewModel.lastSyncTime)
                        .foregroundColor(.secondary)
                }
            } header: {
                Text("Stats")
            }
        }
        .navigationTitle("Developer Options")
    }
}

// MARK: - View Model

class SettingsViewModel: ObservableObject {
    @Published var appVersion = "1.0.0"
    @Published var paykitVersion: String
    
    @Published var selectedNetwork: NetworkType = .mainnet
    @Published var useTestnet = false
    
    @Published var requireBiometric = false
    @Published var lockOnBackground = true
    
    @Published var paymentNotifications = true
    @Published var subscriptionReminders = true
    @Published var autoPayAlerts = true
    @Published var limitWarnings = true
    
    @Published var debugLogging = false
    @Published var showRequestResponse = false
    @Published var mockPayments = true
    
    @Published var showingResetConfirmation = false
    @Published var showingExportSheet = false
    @Published var showingImportSheet = false
    @Published var showingGenerateConfirmation = false
    @Published var showingExportPassword = false
    @Published var showingImportPassword = false
    
    @Published var publicKey = ""
    @Published var publicKeyZ32 = ""
    @Published var hasIdentity = false
    @Published var pendingPaymentsCount = 0
    @Published var cacheSize = "1.2 MB"
    @Published var lastSyncTime = "Just now"
    
    @Published var exportedBackup: String = ""
    @Published var importBackupText: String = ""
    @Published var backupPassword: String = ""
    @Published var keyError: String? = nil
    
    private let keyManager: KeyManager
    
    init() {
        self.keyManager = KeyManager()
        self.paykitVersion = getVersion()
        
        // Load initial state from KeyManager
        self.hasIdentity = keyManager.hasIdentity
        self.publicKeyZ32 = keyManager.publicKeyZ32
        self.publicKey = keyManager.publicKeyHex
    }
    
    func clearCache() {
        cacheSize = "0 MB"
    }
    
    func resetAllSettings() {
        selectedNetwork = .mainnet
        useTestnet = false
        requireBiometric = false
        lockOnBackground = true
        paymentNotifications = true
        subscriptionReminders = true
        autoPayAlerts = true
        limitWarnings = true
        debugLogging = false
        showRequestResponse = false
        mockPayments = true
    }
    
    func generateNewKeypair() {
        do {
            let keypair = try keyManager.generateNewIdentity()
            publicKey = keypair.publicKeyHex
            publicKeyZ32 = keypair.publicKeyZ32
            hasIdentity = true
            keyError = nil
        } catch {
            keyError = "Failed to generate keypair: \(error.localizedDescription)"
        }
    }
    
    func getOrCreateIdentity() {
        do {
            let keypair = try keyManager.getOrCreateIdentity()
            publicKey = keypair.publicKeyHex
            publicKeyZ32 = keypair.publicKeyZ32
            hasIdentity = true
            keyError = nil
        } catch {
            keyError = "Failed to get/create identity: \(error.localizedDescription)"
        }
    }
    
    func exportKeys(password: String) {
        do {
            let backup = try keyManager.exportBackup(password: password)
            exportedBackup = try keyManager.backupToString(backup)
            showingExportSheet = true
            keyError = nil
        } catch {
            keyError = "Failed to export: \(error.localizedDescription)"
        }
    }
    
    func importKeys(backupText: String, password: String) {
        do {
            let backup = try keyManager.backupFromString(backupText)
            let keypair = try keyManager.importBackup(backup, password: password)
            publicKey = keypair.publicKeyHex
            publicKeyZ32 = keypair.publicKeyZ32
            hasIdentity = true
            keyError = nil
        } catch {
            keyError = "Failed to import: \(error.localizedDescription)"
        }
    }
    
    func copyPublicKey() {
        let keyToCopy = publicKeyZ32.isEmpty ? publicKey : publicKeyZ32
        UIPasteboard.general.string = keyToCopy
    }
    
    func getDeviceId() -> String {
        keyManager.getDeviceId()
    }
    
    func signData(_ data: Data) -> String? {
        try? keyManager.sign(data: data)
    }
    
    func triggerTestPayment() {
        // Simulate a test payment
    }
    
    func simulateAutoPay() {
        // Simulate an auto-pay trigger
    }
    
    func sendTestNotification() {
        // Send a local notification
    }
}

enum NetworkType {
    case mainnet
    case testnet
    case regtest
}

#Preview {
    SettingsView()
        .environmentObject(AppState())
}
