//
//  SettingsView.swift
//  PaykitMobile
//
//  Settings UI component for Bitkit integration.
//  This is a template that Bitkit can adapt to their design system.
//

import SwiftUI
import PaykitMobile

/// Settings view model for Bitkit integration
public class BitkitSettingsViewModel: ObservableObject {
    @Published public var appVersion: String = "1.0.0"
    @Published public var selectedBitcoinNetwork: BitcoinNetworkFfi = .mainnet
    @Published public var selectedLightningNetwork: LightningNetworkFfi = .mainnet
    @Published public var autoPayAlerts = true
    
    // Navigation callbacks
    public var onNavigateToAutoPay: (() -> Void)?
    public var onNavigateToSubscriptions: (() -> Void)?
    public var onNavigateToPaymentRequests: (() -> Void)?
    public var onNavigateToPaymentMethods: (() -> Void)?
    public var onNavigateToIdentityManagement: (() -> Void)?
    
    public init() {
        // Load app version
        if let version = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String {
            appVersion = version
        }
    }
}

/// Settings view component
public struct BitkitSettingsView: View {
    @ObservedObject var viewModel: BitkitSettingsViewModel
    
    public init(viewModel: BitkitSettingsViewModel) {
        self.viewModel = viewModel
    }
    
    public var body: some View {
        NavigationView {
            List {
                // Quick Access Section
                Section {
                    if let onAutoPay = viewModel.onNavigateToAutoPay {
                        Button(action: onAutoPay) {
                            HStack {
                                Image(systemName: "arrow.clockwise.circle.fill")
                                    .foregroundColor(.orange)
                                    .frame(width: 30)
                                Text("Auto-Pay")
                            }
                        }
                    }
                    
                    if let onSubscriptions = viewModel.onNavigateToSubscriptions {
                        Button(action: onSubscriptions) {
                            HStack {
                                Image(systemName: "repeat.circle.fill")
                                    .foregroundColor(.blue)
                                    .frame(width: 30)
                                Text("Subscriptions")
                            }
                        }
                    }
                    
                    if let onPaymentRequests = viewModel.onNavigateToPaymentRequests {
                        Button(action: onPaymentRequests) {
                            HStack {
                                Image(systemName: "envelope.circle.fill")
                                    .foregroundColor(.purple)
                                    .frame(width: 30)
                                Text("Payment Requests")
                            }
                        }
                    }
                    
                    if let onPaymentMethods = viewModel.onNavigateToPaymentMethods {
                        Button(action: onPaymentMethods) {
                            HStack {
                                Image(systemName: "globe")
                                    .foregroundColor(.green)
                                    .frame(width: 30)
                                Text("Payment Methods")
                            }
                        }
                    }
                } header: {
                    Text("Quick Access")
                }
                
                // Network Section
                Section {
                    Picker("Bitcoin Network", selection: $viewModel.selectedBitcoinNetwork) {
                        Text("Mainnet").tag(BitcoinNetworkFfi.mainnet)
                        Text("Testnet").tag(BitcoinNetworkFfi.testnet)
                        Text("Regtest").tag(BitcoinNetworkFfi.regtest)
                    }
                    
                    Picker("Lightning Network", selection: $viewModel.selectedLightningNetwork) {
                        Text("Mainnet").tag(LightningNetworkFfi.mainnet)
                        Text("Testnet").tag(LightningNetworkFfi.testnet)
                        Text("Regtest").tag(LightningNetworkFfi.regtest)
                    }
                } header: {
                    Text("Network")
                }
                
                // Notifications Section
                Section {
                    Toggle("Auto-Pay Alerts", isOn: $viewModel.autoPayAlerts)
                } header: {
                    Text("Notifications")
                }
                
                // App Info Section
                Section {
                    HStack {
                        Text("Version")
                        Spacer()
                        Text(viewModel.appVersion)
                            .foregroundColor(.secondary)
                    }
                } header: {
                    Text("App Information")
                }
                
                // Identity Management
                if let onIdentityManagement = viewModel.onNavigateToIdentityManagement {
                    Section {
                        Button(action: onIdentityManagement) {
                            HStack {
                                Image(systemName: "person.2.fill")
                                    .foregroundColor(.blue)
                                    .frame(width: 30)
                                Text("Manage Identities")
                            }
                        }
                    } header: {
                        Text("Identity")
                    }
                }
            }
            .navigationTitle("Settings")
        }
    }
}
