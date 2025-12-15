//
//  DashboardView.swift
//  PaykitMobile
//
//  Dashboard UI component for Bitkit integration.
//  This is a template that Bitkit can adapt to their design system.
//

import SwiftUI
import PaykitMobile

/// Dashboard view model for Bitkit integration
/// Bitkit should adapt this to use their storage and data sources
public class BitkitDashboardViewModel: ObservableObject {
    @Published public var recentReceipts: [Receipt] = []
    @Published public var contactCount: Int = 0
    @Published public var totalSent: UInt64 = 0
    @Published public var totalReceived: UInt64 = 0
    @Published public var pendingCount: Int = 0
    @Published public var isLoading = true
    
    // Quick Access properties
    @Published public var autoPayEnabled: Bool = false
    @Published public var activeSubscriptions: Int = 0
    @Published public var pendingRequests: Int = 0
    
    // Directory and Health status
    @Published public var publishedMethodsCount: Int = 0
    @Published public var overallHealthStatus: String = "Unknown"
    
    private let paykitClient: PaykitClient
    
    public init(paykitClient: PaykitClient) {
        self.paykitClient = paykitClient
    }
    
    /// Load dashboard data
    /// Bitkit should implement this to load from their storage
    public func loadDashboard(
        receiptStorage: ReceiptStorageProtocol? = nil,
        contactStorage: ContactStorageProtocol? = nil,
        autoPayStorage: AutoPayStorageProtocol? = nil,
        subscriptionStorage: SubscriptionStorageProtocol? = nil,
        paymentRequestStorage: PaymentRequestStorageProtocol? = nil
    ) {
        isLoading = true
        
        // Load recent receipts
        if let storage = receiptStorage {
            recentReceipts = storage.recentReceipts(limit: 5)
            totalSent = storage.totalSent()
            totalReceived = storage.totalReceived()
            pendingCount = storage.pendingCount()
        }
        
        // Load contact count
        if let storage = contactStorage {
            contactCount = storage.listContacts().count
        }
        
        // Load auto-pay status
        if let storage = autoPayStorage {
            autoPayEnabled = storage.getSettings().isEnabled
        }
        
        // Load subscriptions
        if let storage = subscriptionStorage {
            activeSubscriptions = storage.activeSubscriptions().count
        }
        
        // Load payment requests
        if let storage = paymentRequestStorage {
            pendingRequests = storage.pendingCount()
        }
        
        // Check payment methods health
        Task {
            do {
                let healthResults = paykitClient.checkHealth()
                let healthyCount = healthResults.filter { $0.isHealthy }.count
                let totalCount = healthResults.count
                publishedMethodsCount = totalCount
                overallHealthStatus = totalCount > 0 ? "\(healthyCount)/\(totalCount) healthy" : "No methods"
            } catch {
                overallHealthStatus = "Error"
            }
            
            await MainActor.run {
                isLoading = false
            }
        }
    }
}

/// Storage protocols for Bitkit to implement
public protocol ReceiptStorageProtocol {
    func recentReceipts(limit: Int) -> [Receipt]
    func totalSent() -> UInt64
    func totalReceived() -> UInt64
    func pendingCount() -> Int
}

public protocol ContactStorageProtocol {
    func listContacts() -> [Contact]
}

public protocol AutoPayStorageProtocol {
    func getSettings() -> AutoPaySettings
}

public protocol SubscriptionStorageProtocol {
    func activeSubscriptions() -> [Subscription]
}

public protocol PaymentRequestStorageProtocol {
    func pendingCount() -> Int
}

/// Contact model (placeholder - Bitkit should use their own)
public struct Contact: Identifiable {
    public let id: String
    public let name: String
    public let pubkey: String
    
    public init(id: String, name: String, pubkey: String) {
        self.id = id
        self.name = name
        self.pubkey = pubkey
    }
}

/// Dashboard view component
/// Bitkit should adapt this to match their design system
public struct BitkitDashboardView: View {
    @ObservedObject var viewModel: BitkitDashboardViewModel
    
    // Navigation callbacks - Bitkit should implement these
    public var onSendPayment: () -> Void = {}
    public var onReceivePayment: () -> Void = {}
    public var onScanQR: () -> Void = {}
    public var onViewReceipts: () -> Void = {}
    public var onViewContacts: () -> Void = {}
    public var onViewPaymentMethods: () -> Void = {}
    public var onViewAutoPay: () -> Void = {}
    public var onViewSubscriptions: () -> Void = {}
    public var onViewPaymentRequests: () -> Void = {}
    
    public init(viewModel: BitkitDashboardViewModel) {
        self.viewModel = viewModel
    }
    
    public var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Stats Cards
                statsSection
                
                // Quick Actions
                quickActionsSection
                
                // Recent Activity
                recentActivitySection
                
                // Quick Access Cards
                quickAccessSection
            }
            .padding()
        }
        .refreshable {
            // Bitkit should implement refresh logic
            viewModel.loadDashboard()
        }
    }
    
    // MARK: - Stats Section
    
    private var statsSection: some View {
        LazyVGrid(columns: [
            GridItem(.flexible()),
            GridItem(.flexible())
        ], spacing: 16) {
            StatCard(
                title: "Total Sent",
                value: formatSats(viewModel.totalSent),
                icon: "arrow.up.circle.fill",
                color: .blue
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
                color: .purple
            )
            
            StatCard(
                title: "Pending",
                value: "\(viewModel.pendingCount)",
                icon: "clock.fill",
                color: .orange
            )
        }
    }
    
    // MARK: - Quick Actions
    
    private var quickActionsSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Quick Actions")
                .font(.headline)
            
            HStack(spacing: 12) {
                QuickActionButton(
                    title: "Send",
                    icon: "arrow.up.circle.fill",
                    color: .blue,
                    action: onSendPayment
                )
                
                QuickActionButton(
                    title: "Receive",
                    icon: "arrow.down.circle.fill",
                    color: .green,
                    action: onReceivePayment
                )
                
                QuickActionButton(
                    title: "Scan",
                    icon: "qrcode.viewfinder",
                    color: .purple,
                    action: onScanQR
                )
            }
        }
    }
    
    // MARK: - Recent Activity
    
    private var recentActivitySection: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Recent Activity")
                    .font(.headline)
                Spacer()
                Button("View All", action: onViewReceipts)
                    .font(.subheadline)
            }
            
            if viewModel.isLoading {
                ProgressView()
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding()
            } else if viewModel.recentReceipts.isEmpty {
                Text("No recent activity")
                    .foregroundColor(.secondary)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding()
            } else {
                ForEach(viewModel.recentReceipts.prefix(5)) { receipt in
                    ReceiptRow(receipt: receipt)
                }
            }
        }
    }
    
    // MARK: - Quick Access
    
    private var quickAccessSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Quick Access")
                .font(.headline)
            
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 12) {
                if viewModel.autoPayEnabled {
                    QuickAccessCard(
                        title: "Auto-Pay",
                        subtitle: "ON",
                        icon: "repeat.circle.fill",
                        color: .green,
                        action: onViewAutoPay
                    )
                }
                
                if viewModel.activeSubscriptions > 0 {
                    QuickAccessCard(
                        title: "Subscriptions",
                        subtitle: "\(viewModel.activeSubscriptions) active",
                        icon: "calendar",
                        color: .blue,
                        action: onViewSubscriptions
                    )
                }
                
                if viewModel.pendingRequests > 0 {
                    QuickAccessCard(
                        title: "Requests",
                        subtitle: "\(viewModel.pendingRequests) pending",
                        icon: "bell.badge",
                        color: .orange,
                        action: onViewPaymentRequests
                    )
                }
                
                QuickAccessCard(
                    title: "Payment Methods",
                    subtitle: viewModel.overallHealthStatus,
                    icon: "creditcard.fill",
                    color: .purple,
                    action: onViewPaymentMethods
                )
            }
        }
    }
    
    // MARK: - Helpers
    
    private func formatSats(_ sats: UInt64) -> String {
        if sats >= 1_000_000 {
            return String(format: "%.2fM", Double(sats) / 1_000_000.0)
        } else if sats >= 1_000 {
            return String(format: "%.1fK", Double(sats) / 1_000.0)
        } else {
            return "\(sats)"
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
        .background(Color(.systemGray6))
        .cornerRadius(12)
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
                    .font(.title)
                    .foregroundColor(color)
                Text(title)
                    .font(.caption)
            }
            .frame(maxWidth: .infinity)
            .padding()
            .background(Color(.systemGray6))
            .cornerRadius(12)
        }
    }
}

struct ReceiptRow: View {
    let receipt: Receipt
    
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text(receipt.payer)
                    .font(.subheadline)
                    .fontWeight(.medium)
                if let amount = receipt.amount {
                    Text(amount)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            Spacer()
            if let currency = receipt.currency {
                Text(currency)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 8)
        .padding(.horizontal, 12)
        .background(Color(.systemGray6))
        .cornerRadius(8)
    }
}

struct QuickAccessCard: View {
    let title: String
    let subtitle: String
    let icon: String
    let color: Color
    let action: () -> Void
    
    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Image(systemName: icon)
                        .foregroundColor(color)
                    Spacer()
                }
                Text(title)
                    .font(.headline)
                Text(subtitle)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding()
            .background(Color(.systemGray6))
            .cornerRadius(12)
        }
    }
}
