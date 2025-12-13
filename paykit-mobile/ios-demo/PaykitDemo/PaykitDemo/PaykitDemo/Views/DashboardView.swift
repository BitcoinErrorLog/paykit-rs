//
//  DashboardView.swift
//  PaykitDemo
//
//  Dashboard overview showing key metrics and recent activity.
//

import SwiftUI

class DashboardViewModel: ObservableObject {
    @Published var recentReceipts: [Receipt] = []
    @Published var contactCount: Int = 0
    @Published var totalSent: UInt64 = 0
    @Published var totalReceived: UInt64 = 0
    @Published var pendingCount: Int = 0
    @Published var isLoading = true
    
    // Setup checklist properties
    @Published var hasPaymentMethods: Bool = false
    @Published var hasPublishedMethods: Bool = false
    
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
        
        isLoading = false
    }
}

struct DashboardView: View {
    @StateObject private var viewModel = DashboardViewModel()
    @State private var showingQRScanner = false
    @State private var showingPaymentView = false
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 20) {
                    // Setup Checklist (show if incomplete)
                    if !viewModel.isSetupComplete {
                        setupChecklistSection
                    }
                    
                    // Quick Stats
                    statsSection
                    
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
            }
            .refreshable {
                viewModel.loadDashboard()
            }
            .sheet(isPresented: $showingQRScanner) {
                QRScannerView()
            }
            .sheet(isPresented: $showingPaymentView) {
                PaymentView()
            }
        }
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
                
                NavigationLink(destination: ReceiptsView()) {
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
                        ReceiptRow(receipt: receipt)
                        
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
                    // TODO: Navigate to receive flow
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

struct ReceiptRow: View {
    let receipt: Receipt
    
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
    
    private func statusColor(_ status: PaymentStatus) -> Color {
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

#Preview {
    DashboardView()
}

