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
    
    var body: some View {
        NavigationView {
            List {
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
    
    var body: some View {
        List {
            Section {
                HStack {
                    Text("Public Key")
                    Spacer()
                    Text(viewModel.publicKey.prefix(16) + "...")
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.secondary)
                }
                
                Button("Copy Public Key") {
                    UIPasteboard.general.string = viewModel.publicKey
                }
                
                Button("Export Keys") {
                    viewModel.showingExportSheet = true
                }
            } header: {
                Text("Your Keys")
            }
            
            Section {
                Button("Generate New Keypair") {
                    viewModel.showingGenerateConfirmation = true
                }
                .foregroundColor(.orange)
                
                Button("Import from Backup") {
                    viewModel.showingImportSheet = true
                }
            } header: {
                Text("Key Management")
            } footer: {
                Text("Warning: Generating a new keypair will change your identity")
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
    @Published var paykitVersion = "0.0.1"
    
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
    
    @Published var publicKey = "pk1abc123def456ghi789jkl012mno345"
    @Published var pendingPaymentsCount = 0
    @Published var cacheSize = "1.2 MB"
    @Published var lastSyncTime = "Just now"
    
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
        // In a real app, this would generate a new Ed25519 keypair
        publicKey = "pk1new\(UUID().uuidString.prefix(20))"
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
