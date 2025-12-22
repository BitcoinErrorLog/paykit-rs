//
//  DashboardView.swift
//  PaykitDemo
//
//  Dashboard overview showing key metrics and recent activity.
//

import SwiftUI
import Combine

class DashboardViewModel: ObservableObject {
    @Published var recentReceipts: [PaymentReceipt] = []
    @Published var contactCount: Int = 0
    @Published var totalSent: UInt64 = 0
    @Published var totalReceived: UInt64 = 0
    @Published var pendingCount: Int = 0
    @Published var isLoading = true
    
    // Balance properties
    @Published var bitcoinBalanceSats: UInt64 = 0
    @Published var lightningBalanceSats: UInt64 = 0
    
    // Setup checklist properties
    @Published var hasPaymentMethods: Bool = false
    @Published var hasPublishedMethods: Bool = false
    
    // Quick Access properties
    @Published var autoPayEnabled: Bool = false
    @Published var activeSubscriptions: Int = 0
    @Published var pendingRequests: Int = 0
    
    // Directory and Health status
    @Published var publishedMethodsCount: Int = 0
    @Published var overallHealthStatus: String = "Unknown"
    
    var totalBalanceSats: UInt64 {
        bitcoinBalanceSats + lightningBalanceSats
    }
    
    var isSetupComplete: Bool {
        contactCount > 0 && hasPaymentMethods && hasPublishedMethods
    }
    
    var setupProgress: Int {
        var completed = 1 // Identity is always created at this point
        if contactCount > 0 { completed += 1 }
        if hasPaymentMethods { completed += 1 }
        if hasPublishedMethods { completed += 1 }
        return (completed * 100) / 4
    }
    
    private let keyManager = KeyManager()
    private var receiptStorage: ReceiptStorage {
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        return ReceiptStorage(identityName: identityName)
    }
    private var contactStorage: ContactStorage {
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        return ContactStorage(identityName: identityName)
    }
    private var autoPayStorage: AutoPayStorage {
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        return AutoPayStorage(identityName: identityName)
    }
    private var subscriptionStorage: SubscriptionStorage {
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        return SubscriptionStorage(identityName: identityName)
    }
    private var paymentRequestStorage: PaymentRequestStorage {
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        return PaymentRequestStorage(identityName: identityName)
    }
    
    init() {
        // Observe identity changes
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(identityDidChange),
            name: .identityDidChange,
            object: nil
        )
    }
    
    @objc private func identityDidChange() {
        loadDashboard()
    }
    
    deinit {
        NotificationCenter.default.removeObserver(self)
    }
    
    func loadDashboard() {
        isLoading = true
        
        // Load recent receipts
        recentReceipts = receiptStorage.recentReceipts(limit: 5)
        
        // Load stats
        contactCount = contactStorage.listContacts().count
        totalSent = receiptStorage.totalSent()
        totalReceived = receiptStorage.totalReceived()
        pendingCount = receiptStorage.pendingCount()
        
        // Check payment methods (for demo, assume configured if we have any receipts)
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        let methodStorage = PaymentMethodStorage(identityName: identityName)
        let methods = methodStorage.listMethods()
        hasPaymentMethods = !methods.isEmpty
        hasPublishedMethods = methods.contains { $0.isPublic }
        publishedMethodsCount = methods.filter { $0.isPublic }.count
        
        // Load Auto-Pay status
        let autoPaySettings = autoPayStorage.getSettings()
        autoPayEnabled = autoPaySettings.isEnabled
        
        // Load Subscriptions count
        activeSubscriptions = subscriptionStorage.listSubscriptions().count
        
        // Load Payment Requests count
        pendingRequests = paymentRequestStorage.pendingRequests().count
        
        isLoading = false
    }
    
    func updateBalances(from paymentService: PaymentService) {
        bitcoinBalanceSats = paymentService.bitcoinBalanceSats
        lightningBalanceSats = paymentService.lightningBalanceSats
    }
}

struct DashboardView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var viewModel = DashboardViewModel()
    @ObservedObject private var pubkyRingBridge = PubkyRingBridge.shared
    @State private var showingQRScanner = false
    @State private var showingPaymentView = false
    @State private var showingReceiveView = false
    @State private var showingAutoPay = false
    @State private var showingSubscriptions = false
    @State private var showingPaymentRequests = false
    @State private var showingContactDiscovery = false
    @State private var showingPubkyRingAuth = false
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 20) {
                    // Pubky Ring Connection Card
                    connectionStatusCard
                    
                    // Wallet Balance Card
                    balanceCard
                    
                    // Setup Checklist (show if incomplete)
                    if !viewModel.isSetupComplete {
                        setupChecklistSection
                    }
                    
                    // Quick Access Section
                    quickAccessSection
                    
                    // Quick Stats
                    statsSection
                    
                    // Directory Status Section
                    directoryStatusSection
                    
                    // Recent Activity
                    recentActivitySection
                    
                    // Quick Actions
                    quickActionsSection
                }
                .padding()
            }
            .navigationTitle("Dashboard")
            .onAppear {
                viewModel.loadDashboard()
                viewModel.updateBalances(from: appState.paymentService)
            }
            .refreshable {
                viewModel.loadDashboard()
                viewModel.updateBalances(from: appState.paymentService)
            }
            .sheet(isPresented: $showingQRScanner) {
                QRScannerView()
            }
            .sheet(isPresented: $showingPaymentView) {
                PaymentView()
            }
            .sheet(isPresented: $showingReceiveView) {
                ReceivePaymentView()
            }
            .sheet(isPresented: $showingAutoPay) {
                AutoPayView()
            }
            .sheet(isPresented: $showingSubscriptions) {
                SubscriptionsView()
            }
            .sheet(isPresented: $showingPaymentRequests) {
                PaymentRequestsView()
            }
            .sheet(isPresented: $showingContactDiscovery) {
                NavigationStack {
                    ContactDiscoveryView()
                }
            }
            .sheet(isPresented: $showingPubkyRingAuth) {
                PubkyRingAuthView { session in
                    // Session received, state updates automatically
                }
            }
        }
    }
    
    // MARK: - Connection Status Card
    
    private var connectionStatusCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: connectionIcon)
                    .foregroundColor(connectionColor)
                    .font(.title2)
                
                VStack(alignment: .leading, spacing: 2) {
                    Text("Pubky Ring")
                        .font(.headline)
                    
                    Text(pubkyRingBridge.connectionState.displayText)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                if pubkyRingBridge.connectionState.isConnected {
                    Button {
                        pubkyRingBridge.disconnect()
                    } label: {
                        Text("Disconnect")
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                } else {
                    Button {
                        showingPubkyRingAuth = true
                    } label: {
                        Text("Connect")
                            .font(.caption.bold())
                            .padding(.horizontal, 12)
                            .padding(.vertical, 6)
                            .background(Color.blue)
                            .foregroundColor(.white)
                            .cornerRadius(8)
                    }
                }
            }
            
            if !pubkyRingBridge.isPubkyRingInstalled && !pubkyRingBridge.connectionState.isConnected {
                Text("Pubky-ring not installed. Use QR code for cross-device auth.")
                    .font(.caption2)
                    .foregroundColor(.orange)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }
    
    private var connectionIcon: String {
        switch pubkyRingBridge.connectionState {
        case .disconnected: return "circle"
        case .connecting: return "circle.dotted"
        case .connected: return "checkmark.circle.fill"
        case .error: return "exclamationmark.circle.fill"
        }
    }
    
    private var connectionColor: Color {
        switch pubkyRingBridge.connectionState {
        case .disconnected: return .gray
        case .connecting: return .orange
        case .connected: return .green
        case .error: return .red
        }
    }
    
    // MARK: - Balance Card
    
    private var balanceCard: some View {
        VStack(spacing: 16) {
            // Total Balance
            VStack(spacing: 4) {
                Text("Total Balance")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Text(formatSats(viewModel.totalBalanceSats))
                    .font(.system(size: 32, weight: .bold, design: .rounded))
            }
            
            // Breakdown
            HStack(spacing: 24) {
                VStack(spacing: 4) {
                    HStack(spacing: 4) {
                        Image(systemName: "bitcoinsign.circle.fill")
                            .foregroundColor(.orange)
                        Text("On-chain")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Text(formatSatsShort(viewModel.bitcoinBalanceSats))
                        .font(.subheadline.bold())
                }
                
                Divider()
                    .frame(height: 30)
                
                VStack(spacing: 4) {
                    HStack(spacing: 4) {
                        Image(systemName: "bolt.fill")
                            .foregroundColor(.yellow)
                        Text("Lightning")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Text(formatSatsShort(viewModel.lightningBalanceSats))
                        .font(.subheadline.bold())
                }
            }
        }
        .frame(maxWidth: .infinity)
        .padding()
        .background(
            LinearGradient(
                colors: [Color.blue.opacity(0.1), Color.purple.opacity(0.1)],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
        )
        .cornerRadius(16)
    }
    
    // MARK: - Stats Section
    
    private var statsSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Overview")
                .font(.headline)
                .foregroundColor(.secondary)
            
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 12) {
                StatCard(
                    title: "Total Sent",
                    value: formatSats(viewModel.totalSent),
                    icon: "arrow.up.circle.fill",
                    color: .red
                )
                
                StatCard(
                    title: "Total Received",
                    value: formatSats(viewModel.totalReceived),
                    icon: "arrow.down.circle.fill",
                    color: .green
                )
                
                StatCard(
                    title: "Contacts",
                    value: "\(viewModel.contactCount)",
                    icon: "person.2.fill",
                    color: .blue
                )
                
                StatCard(
                    title: "Pending",
                    value: "\(viewModel.pendingCount)",
                    icon: "clock.fill",
                    color: .orange
                )
            }
        }
    }
    
    // MARK: - Recent Activity Section
    
    private var recentActivitySection: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Recent Activity")
                    .font(.headline)
                    .foregroundColor(.secondary)
                
                Spacer()
                
                NavigationLink(destination: ActivityListView()) {
                    Text("See All")
                        .font(.subheadline)
                        .foregroundColor(.blue)
                }
            }
            
            if viewModel.recentReceipts.isEmpty {
                emptyActivityView
            } else {
                VStack(spacing: 0) {
                    ForEach(viewModel.recentReceipts) { receipt in
                        DashboardReceiptRow(receipt: receipt)
                        
                        if receipt.id != viewModel.recentReceipts.last?.id {
                            Divider()
                        }
                    }
                }
                .background(Color(.systemBackground))
                .cornerRadius(12)
                .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
            }
        }
    }
    
    private var emptyActivityView: some View {
        VStack(spacing: 12) {
            Image(systemName: "tray")
                .font(.largeTitle)
                .foregroundColor(.secondary)
            Text("No recent activity")
                .font(.subheadline)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 40)
        .background(Color(.systemBackground))
        .cornerRadius(12)
    }
    
    // MARK: - Setup Checklist Section
    
    private var setupChecklistSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Setup Checklist")
                    .font(.headline)
                    .foregroundColor(.secondary)
                Spacer()
                Text("\(viewModel.setupProgress)%")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            VStack(spacing: 8) {
                SetupCheckItem(
                    title: "Create Identity",
                    subtitle: "Generate your Paykit identity",
                    isComplete: true,
                    icon: "person.circle.fill"
                )
                
                SetupCheckItem(
                    title: "Add Contacts",
                    subtitle: viewModel.contactCount > 0 ? "\(viewModel.contactCount) contact(s) added" : "Add payment recipients",
                    isComplete: viewModel.contactCount > 0,
                    icon: "person.2.fill"
                )
                
                SetupCheckItem(
                    title: "Configure Payment Methods",
                    subtitle: "Set up Lightning or on-chain",
                    isComplete: viewModel.hasPaymentMethods,
                    icon: "creditcard.fill"
                )
                
                SetupCheckItem(
                    title: "Publish to Directory",
                    subtitle: "Make your methods discoverable",
                    isComplete: viewModel.hasPublishedMethods,
                    icon: "globe"
                )
            }
            .padding()
            .background(Color(.systemBackground))
            .cornerRadius(12)
        }
    }
    
    // MARK: - Quick Access Section
    
    private var quickAccessSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Quick Access")
                .font(.headline)
                .foregroundColor(.secondary)
            
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 12) {
                QuickAccessCard(
                    title: "Auto-Pay",
                    icon: "arrow.clockwise.circle.fill",
                    color: .orange,
                    badge: viewModel.autoPayEnabled ? "ON" : nil,
                    action: { showingAutoPay = true }
                )
                
                QuickAccessCard(
                    title: "Subscriptions",
                    icon: "repeat.circle.fill",
                    color: .blue,
                    badge: viewModel.activeSubscriptions > 0 ? "\(viewModel.activeSubscriptions)" : nil,
                    action: { showingSubscriptions = true }
                )
                
                QuickAccessCard(
                    title: "Requests",
                    icon: "envelope.circle.fill",
                    color: .purple,
                    badge: viewModel.pendingRequests > 0 ? "\(viewModel.pendingRequests)" : nil,
                    action: { showingPaymentRequests = true }
                )
                
                QuickAccessCard(
                    title: "Discover",
                    icon: "person.2.badge.plus",
                    color: .green,
                    badge: nil,
                    action: { showingContactDiscovery = true }
                )
            }
        }
    }
    
    // MARK: - Directory Status Section
    
    private var directoryStatusSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Directory Status")
                    .font(.headline)
                    .foregroundColor(.secondary)
                Spacer()
                NavigationLink(destination: PaymentMethodsView()) {
                    Text("Manage")
                        .font(.subheadline)
                        .foregroundColor(.blue)
                }
            }
            
            HStack(spacing: 16) {
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Image(systemName: viewModel.hasPublishedMethods ? "checkmark.circle.fill" : "xmark.circle.fill")
                            .foregroundColor(viewModel.hasPublishedMethods ? .green : .orange)
                        Text(viewModel.hasPublishedMethods ? "Published" : "Not Published")
                            .font(.subheadline)
                            .fontWeight(.medium)
                    }
                    Text("\(viewModel.publishedMethodsCount) method(s) public")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
            }
            .padding()
            .background(Color(.systemBackground))
            .cornerRadius(12)
            .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
        }
    }
    
    // MARK: - Quick Actions Section
    
    private var quickActionsSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Quick Actions")
                .font(.headline)
                .foregroundColor(.secondary)
            
            HStack(spacing: 12) {
                QuickActionButton(
                    title: "Send",
                    icon: "paperplane.fill",
                    color: .blue
                ) {
                    showingPaymentView = true
                }
                
                QuickActionButton(
                    title: "Receive",
                    icon: "qrcode",
                    color: .green
                ) {
                    showingReceiveView = true
                }
                
                QuickActionButton(
                    title: "Scan",
                    icon: "qrcode.viewfinder",
                    color: .purple
                ) {
                    showingQRScanner = true
                }
            }
        }
    }
    
    // MARK: - Helpers
    
    private func formatSats(_ amount: UInt64) -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        return "\(formatter.string(from: NSNumber(value: amount)) ?? "\(amount)") sats"
    }
    
    private func formatSatsShort(_ amount: UInt64) -> String {
        if amount >= 1_000_000 {
            return String(format: "%.2fM", Double(amount) / 1_000_000)
        } else if amount >= 1_000 {
            return String(format: "%.1fk", Double(amount) / 1_000)
        } else {
            return "\(amount)"
        }
    }
}

// MARK: - Supporting Views

struct StatCard: View {
    let title: String
    let value: String
    let icon: String
    let color: Color
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Image(systemName: icon)
                    .foregroundColor(color)
                Spacer()
            }
            
            Text(value)
                .font(.title2)
                .fontWeight(.bold)
            
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }
}

struct DashboardReceiptRow: View {
    let receipt: PaymentReceipt
    
    var body: some View {
        HStack {
            // Direction indicator
            Image(systemName: receipt.direction == .sent ? "arrow.up.circle.fill" : "arrow.down.circle.fill")
                .foregroundColor(receipt.direction == .sent ? .red : .green)
                .font(.title2)
            
            VStack(alignment: .leading, spacing: 2) {
                Text(receipt.displayName)
                    .font(.body)
                    .fontWeight(.medium)
                
                Text(receipt.paymentMethod)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            VStack(alignment: .trailing, spacing: 2) {
                Text(receipt.formattedAmount)
                    .font(.body)
                    .fontWeight(.medium)
                    .foregroundColor(receipt.direction == .sent ? .red : .green)
                
                Text(receipt.status.rawValue.capitalized)
                    .font(.caption)
                    .foregroundColor(statusColor(receipt.status))
            }
        }
        .padding()
    }
    
    private func statusColor(_ status: PaymentReceiptStatus) -> Color {
        switch status {
        case .pending: return .orange
        case .completed: return .green
        case .failed: return .red
        case .refunded: return .purple
        }
    }
}

struct QuickActionButton: View {
    let title: String
    let icon: String
    let color: Color
    let action: () -> Void
    
    var body: some View {
        Button(action: action) {
            VStack(spacing: 8) {
                Image(systemName: icon)
                    .font(.title2)
                    .foregroundColor(color)
                
                Text(title)
                    .font(.caption)
                    .fontWeight(.medium)
                    .foregroundColor(.primary)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 16)
            .background(Color(.systemBackground))
            .cornerRadius(12)
            .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
        }
    }
}

struct SetupCheckItem: View {
    let title: String
    let subtitle: String
    let isComplete: Bool
    let icon: String
    
    var body: some View {
        HStack(spacing: 12) {
            ZStack {
                Circle()
                    .fill(isComplete ? Color.green.opacity(0.2) : Color.gray.opacity(0.2))
                    .frame(width: 40, height: 40)
                
                Image(systemName: isComplete ? "checkmark.circle.fill" : icon)
                    .foregroundColor(isComplete ? .green : .gray)
            }
            
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.subheadline)
                    .fontWeight(.medium)
                    .foregroundColor(isComplete ? .secondary : .primary)
                
                Text(subtitle)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            if isComplete {
                Image(systemName: "checkmark")
                    .foregroundColor(.green)
            }
        }
    }
}

struct QuickAccessCard: View {
    let title: String
    let icon: String
    let color: Color
    let badge: String?
    let action: () -> Void
    
    var body: some View {
        Button(action: action) {
            VStack(spacing: 8) {
                HStack {
                    Image(systemName: icon)
                        .foregroundColor(color)
                        .font(.title3)
                    Spacer()
                    if let badge = badge {
                        Text(badge)
                            .font(.caption2)
                            .fontWeight(.bold)
                            .foregroundColor(.white)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(color)
                            .cornerRadius(8)
                    }
                }
                
                Text(title)
                    .font(.caption)
                    .fontWeight(.medium)
                    .foregroundColor(.primary)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding()
            .background(Color(.systemBackground))
            .cornerRadius(12)
            .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
        }
    }
}

#Preview {
    DashboardView()
        .environmentObject(AppState())
}

