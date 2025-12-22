//
//  ContentView.swift
//  PaykitDemo
//
//  Main navigation view for the demo app with deep link handling
//

import SwiftUI

// MARK: - Deep Link Routes

enum DeepLinkRoute: Equatable {
    case dashboard
    case send(pubkey: String?, amount: UInt64?)
    case receive
    case contacts
    case contactDetail(pubkey: String)
    case discovery
    case settings
    case profile
    case profileImport
    case pubkyRingAuth
    case receipt(id: String)
    case receiptLookup
    case smartCheckout(pubkey: String, amount: UInt64?)
    case sessionManagement
    case subscriptions
    case autoPay
    case paymentMethods
}

// MARK: - Navigation State

@MainActor
class NavigationState: ObservableObject {
    @Published var selectedTab: Tab = .dashboard
    @Published var pendingDeepLink: DeepLinkRoute?
    @Published var showingPubkyRingAuth = false
    @Published var showingSendPayment = false
    @Published var sendPaymentPubkey: String?
    @Published var sendPaymentAmount: UInt64?
    
    enum Tab: Int, CaseIterable {
        case dashboard = 0
        case send
        case receive
        case contacts
        case settings
    }
    
    func handleDeepLink(_ route: DeepLinkRoute) {
        switch route {
        case .dashboard:
            selectedTab = .dashboard
            
        case .send(let pubkey, let amount):
            sendPaymentPubkey = pubkey
            sendPaymentAmount = amount
            selectedTab = .send
            showingSendPayment = true
            
        case .receive:
            selectedTab = .receive
            
        case .contacts:
            selectedTab = .contacts
            
        case .contactDetail:
            selectedTab = .contacts
            pendingDeepLink = route
            
        case .discovery:
            selectedTab = .contacts
            pendingDeepLink = route
            
        case .settings:
            selectedTab = .settings
            
        case .profile, .profileImport, .sessionManagement:
            selectedTab = .settings
            pendingDeepLink = route
            
        case .pubkyRingAuth:
            showingPubkyRingAuth = true
            
        case .receipt:
            selectedTab = .dashboard
            pendingDeepLink = route
            
        case .receiptLookup:
            selectedTab = .dashboard
            pendingDeepLink = route
            
        case .smartCheckout(let pubkey, let amount):
            sendPaymentPubkey = pubkey
            sendPaymentAmount = amount
            selectedTab = .contacts
            pendingDeepLink = route
            
        case .subscriptions, .autoPay, .paymentMethods:
            selectedTab = .settings
            pendingDeepLink = route
        }
    }
    
    /// Parse URL into DeepLinkRoute
    static func parseURL(_ url: URL) -> DeepLinkRoute? {
        guard let scheme = url.scheme,
              scheme == "paykit" || scheme == "paykitdemo" else {
            return nil
        }
        
        let host = url.host ?? ""
        let queryItems = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems ?? []
        
        func queryValue(_ name: String) -> String? {
            queryItems.first { $0.name == name }?.value
        }
        
        switch host {
        case "dashboard":
            return .dashboard
            
        case "send":
            let pubkey = queryValue("pubkey")
            let amount = queryValue("amount").flatMap { UInt64($0) }
            return .send(pubkey: pubkey, amount: amount)
            
        case "receive":
            return .receive
            
        case "contacts":
            if let pubkey = queryValue("pubkey") {
                return .contactDetail(pubkey: pubkey)
            }
            return .contacts
            
        case "discovery", "discover":
            return .discovery
            
        case "settings":
            return .settings
            
        case "profile":
            return .profile
            
        case "profile-import", "import-profile":
            return .profileImport
            
        case "pubkyring-auth", "auth":
            return .pubkyRingAuth
            
        case "receipt":
            if let id = queryValue("id") {
                return .receipt(id: id)
            }
            return nil
            
        case "receipt-lookup", "find-receipt":
            return .receiptLookup
            
        case "smart-checkout", "checkout":
            guard let pubkey = queryValue("pubkey") else { return nil }
            let amount = queryValue("amount").flatMap { UInt64($0) }
            return .smartCheckout(pubkey: pubkey, amount: amount)
            
        case "sessions", "session-management":
            return .sessionManagement
            
        case "subscriptions":
            return .subscriptions
            
        case "auto-pay", "autopay":
            return .autoPay
            
        case "payment-methods", "methods":
            return .paymentMethods
            
        default:
            return nil
        }
    }
}

// MARK: - Content View

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var navigationState = NavigationState()
    
    var body: some View {
        TabView(selection: $navigationState.selectedTab) {
            // Dashboard Tab
            NavigationStack {
                DashboardView()
            }
            .tabItem {
                Label("Dashboard", systemImage: "house.fill")
            }
            .tag(NavigationState.Tab.dashboard)
            
            // Send Payment Tab (Noise)
            NavigationStack {
                PaymentView()
            }
            .tabItem {
                Label("Send", systemImage: "paperplane.fill")
            }
            .tag(NavigationState.Tab.send)
            
            // Receive Payment Tab (Noise)
            NavigationStack {
                ReceivePaymentView()
            }
            .tabItem {
                Label("Receive", systemImage: "arrow.down.circle.fill")
            }
            .tag(NavigationState.Tab.receive)
            
            // Contacts Tab
            NavigationStack {
                ContactsView()
            }
            .tabItem {
                Label("Contacts", systemImage: "person.2")
            }
            .tag(NavigationState.Tab.contacts)
            
            // Settings Tab with nested navigation
            NavigationStack {
                SettingsContainerView()
            }
            .tabItem {
                Label("Settings", systemImage: "gear")
            }
            .tag(NavigationState.Tab.settings)
        }
        .environmentObject(navigationState)
        .alert("Error", isPresented: .constant(appState.errorMessage != nil)) {
            Button("OK") {
                appState.errorMessage = nil
            }
        } message: {
            Text(appState.errorMessage ?? "")
        }
        .sheet(isPresented: $navigationState.showingPubkyRingAuth) {
            PubkyRingAuthView { session in
                // Session received
            }
        }
        .onOpenURL { url in
            handleDeepLink(url)
        }
    }
    
    // MARK: - Deep Link Handling
    
    private func handleDeepLink(_ url: URL) {
        // Handle Pubky Ring callback
        if PubkyRingBridge.shared.handleCallback(url: url) {
            return
        }
        
        // Parse URL into route using NavigationState's parser
        if let route = NavigationState.parseURL(url) {
            navigationState.handleDeepLink(route)
        }
    }
}

// MARK: - Settings Container View

struct SettingsContainerView: View {
    var body: some View {
        List {
            // Profile Section
            Section {
                NavigationLink {
                    ProfileSettingsView()
                } label: {
                    SettingsRow(icon: "person.circle.fill", title: "Profile", color: .blue)
                }
                
                NavigationLink {
                    IdentityListView()
                } label: {
                    SettingsRow(icon: "key.fill", title: "Identities", color: .orange)
                }
            } header: {
                Text("Account")
            }
            
            // Payment Section
            Section {
                NavigationLink {
                    PaymentMethodsView()
                } label: {
                    SettingsRow(icon: "creditcard.fill", title: "Payment Methods", color: .green)
                }
                
                NavigationLink {
                    ReceiptsView()
                } label: {
                    SettingsRow(icon: "doc.text.fill", title: "Receipts", color: .purple)
                }
                
                NavigationLink {
                    AutoPayView()
                } label: {
                    SettingsRow(icon: "arrow.clockwise.circle.fill", title: "Auto-Pay", color: .orange)
                }
                
                NavigationLink {
                    SubscriptionsView()
                } label: {
                    SettingsRow(icon: "repeat.circle.fill", title: "Subscriptions", color: .blue)
                }
            } header: {
                Text("Payments")
            }
            
            // Privacy & Security Section
            Section {
                NavigationLink {
                    RotationSettingsView()
                } label: {
                    SettingsRow(icon: "arrow.triangle.2.circlepath", title: "Key Rotation", color: .red)
                }
                
                NavigationLink {
                    PrivateEndpointsView()
                } label: {
                    SettingsRow(icon: "lock.shield.fill", title: "Private Endpoints", color: .gray)
                }
            } header: {
                Text("Privacy & Security")
            }
            
            // Pubky Ring Section
            Section {
                PubkyRingSettingsRow()
            } header: {
                Text("Pubky Ring")
            }
            
            // About Section
            Section {
                NavigationLink {
                    AboutView()
                } label: {
                    SettingsRow(icon: "info.circle.fill", title: "About", color: .secondary)
                }
            } header: {
                Text("App")
            }
        }
        .navigationTitle("Settings")
    }
}

struct SettingsRow: View {
    let icon: String
    let title: String
    let color: Color
    
    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .foregroundColor(color)
                .frame(width: 24)
            
            Text(title)
        }
    }
}

struct PubkyRingSettingsRow: View {
    @ObservedObject private var bridge = PubkyRingBridge.shared
    @State private var showingAuth = false
    
    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: connectionIcon)
                .foregroundColor(connectionColor)
                .frame(width: 24)
            
            VStack(alignment: .leading, spacing: 2) {
                Text("Connection Status")
                Text(bridge.connectionState.displayText)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            if bridge.connectionState.isConnected {
                Button("Disconnect") {
                    bridge.disconnect()
                }
                .font(.caption)
                .foregroundColor(.red)
            } else {
                Button("Connect") {
                    showingAuth = true
                }
                .font(.caption.bold())
            }
        }
        .sheet(isPresented: $showingAuth) {
            PubkyRingAuthView { session in
                // Session received
            }
        }
    }
    
    private var connectionIcon: String {
        switch bridge.connectionState {
        case .disconnected: return "circle"
        case .connecting: return "circle.dotted"
        case .connected: return "checkmark.circle.fill"
        case .error: return "exclamationmark.circle.fill"
        }
    }
    
    private var connectionColor: Color {
        switch bridge.connectionState {
        case .disconnected: return .gray
        case .connecting: return .orange
        case .connected: return .green
        case .error: return .red
        }
    }
}

// MARK: - Profile Settings View

struct ProfileSettingsView: View {
    @State private var displayName = ""
    @State private var bio = ""
    
    private let keyManager = KeyManager()
    
    var body: some View {
        Form {
            Section {
                HStack {
                    Spacer()
                    VStack(spacing: 12) {
                        Circle()
                            .fill(Color.blue.opacity(0.2))
                            .frame(width: 80, height: 80)
                            .overlay {
                                Text(String(displayName.prefix(1)).uppercased())
                                    .font(.largeTitle)
                                    .foregroundColor(.blue)
                            }
                        
                        if let pubkey = keyManager.publicKeyZ32 {
                            Text(abbreviate(pubkey))
                                .font(.caption.monospaced())
                                .foregroundColor(.secondary)
                        }
                    }
                    Spacer()
                }
                .listRowBackground(Color.clear)
            }
            
            Section {
                TextField("Display Name", text: $displayName)
                TextField("Bio", text: $bio, axis: .vertical)
                    .lineLimit(3...6)
            } header: {
                Text("Profile Info")
            }
            
            Section {
                Button {
                    // Save profile
                } label: {
                    Text("Save Profile")
                        .frame(maxWidth: .infinity)
                }
                
                Button {
                    // Publish to directory
                } label: {
                    Text("Publish to Directory")
                        .frame(maxWidth: .infinity)
                }
            }
        }
        .navigationTitle("Profile")
        .onAppear {
            displayName = keyManager.currentIdentityName ?? "Default"
        }
    }
    
    private func abbreviate(_ key: String) -> String {
        guard key.count > 16 else { return key }
        return "\(key.prefix(8))...\(key.suffix(8))"
    }
}

// MARK: - About View

struct AboutView: View {
    var body: some View {
        List {
            Section {
                HStack {
                    Text("Version")
                    Spacer()
                    Text("1.0.0")
                        .foregroundColor(.secondary)
                }
                
                HStack {
                    Text("Build")
                    Spacer()
                    Text("Demo")
                        .foregroundColor(.secondary)
                }
            }
            
            Section {
                Link("Paykit Documentation", destination: URL(string: "https://github.com/pubky/paykit")!)
                Link("Pubky Ring", destination: URL(string: "https://github.com/pubky/pubky-ring")!)
                Link("Noise Protocol", destination: URL(string: "https://noiseprotocol.org")!)
            } header: {
                Text("Links")
            }
            
            Section {
                Text("This is a demo application showcasing Paykit integration patterns for mobile apps. It demonstrates:")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                VStack(alignment: .leading, spacing: 8) {
                    FeatureRow(text: "Payment executor protocols")
                    FeatureRow(text: "Pubky Ring authentication")
                    FeatureRow(text: "Contact discovery with health indicators")
                    FeatureRow(text: "Noise protocol payments")
                    FeatureRow(text: "Subscriptions and auto-pay")
                    FeatureRow(text: "Deep link handling")
                }
            } header: {
                Text("Features")
            }
        }
        .navigationTitle("About")
    }
}

struct FeatureRow: View {
    let text: String
    
    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "checkmark.circle.fill")
                .foregroundColor(.green)
                .font(.caption)
            Text(text)
                .font(.caption)
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(AppState())
}
