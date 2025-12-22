//
//  ActivityListView.swift
//  PaykitDemo
//
//  Unified activity list showing all payment types in a timeline
//

import SwiftUI

// MARK: - Activity Item

/// Unified activity item representing any payment type
struct ActivityItem: Identifiable {
    let id: String
    let type: ActivityType
    let direction: ActivityDirection
    let amountSats: UInt64
    let counterparty: String
    let counterpartyName: String?
    let status: ActivityStatus
    let timestamp: Date
    let memo: String?
    let methodIcon: String
    let methodName: String
    
    enum ActivityType: String {
        case noise = "noise"
        case lightning = "lightning"
        case onchain = "onchain"
        case subscription = "subscription"
        case autoPay = "autopay"
    }
    
    enum ActivityDirection: String {
        case sent
        case received
    }
    
    enum ActivityStatus: String {
        case pending
        case completed
        case failed
    }
    
    var isIncoming: Bool { direction == .received }
    
    var formattedAmount: String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        let sign = isIncoming ? "+" : "-"
        return "\(sign)\(formatter.string(from: NSNumber(value: amountSats)) ?? "\(amountSats)") sats"
    }
    
    var displayName: String {
        counterpartyName ?? abbreviatedKey
    }
    
    var abbreviatedKey: String {
        guard counterparty.count > 16 else { return counterparty }
        return "\(counterparty.prefix(8))...\(counterparty.suffix(8))"
    }
    
    var statusColor: Color {
        switch status {
        case .pending: return .orange
        case .completed: return .green
        case .failed: return .red
        }
    }
    
    var typeIcon: String {
        switch type {
        case .noise: return "antenna.radiowaves.left.and.right"
        case .lightning: return "bolt.fill"
        case .onchain: return "bitcoinsign.circle.fill"
        case .subscription: return "repeat.circle.fill"
        case .autoPay: return "arrow.clockwise.circle.fill"
        }
    }
}

// MARK: - Activity List View Model

@MainActor
class ActivityListViewModel: ObservableObject {
    @Published var activities: [ActivityItem] = []
    @Published var isLoading = false
    @Published var filter: ActivityFilter = .all
    
    enum ActivityFilter: String, CaseIterable {
        case all = "All"
        case sent = "Sent"
        case received = "Received"
        case noise = "Noise"
        case lightning = "Lightning"
        case onchain = "On-Chain"
    }
    
    private let keyManager = KeyManager()
    
    var filteredActivities: [ActivityItem] {
        switch filter {
        case .all:
            return activities
        case .sent:
            return activities.filter { $0.direction == .sent }
        case .received:
            return activities.filter { $0.direction == .received }
        case .noise:
            return activities.filter { $0.type == .noise }
        case .lightning:
            return activities.filter { $0.type == .lightning }
        case .onchain:
            return activities.filter { $0.type == .onchain }
        }
    }
    
    func loadActivities() {
        isLoading = true
        
        // Load receipts from storage
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        let receiptStorage = ReceiptStorage(identityName: identityName)
        let receipts = receiptStorage.listReceipts()
        
        // Convert receipts to activities
        activities = receipts.map { receipt in
            ActivityItem(
                id: receipt.id,
                type: mapMethodToType(receipt.paymentMethod),
                direction: receipt.direction == .sent ? .sent : .received,
                amountSats: receipt.amountSats,
                counterparty: receipt.counterpartyKey,
                counterpartyName: receipt.counterpartyName,
                status: mapStatus(receipt.status),
                timestamp: receipt.createdAt,
                memo: receipt.memo,
                methodIcon: iconForMethod(receipt.paymentMethod),
                methodName: receipt.paymentMethod
            )
        }.sorted { $0.timestamp > $1.timestamp }
        
        isLoading = false
    }
    
    private func mapMethodToType(_ method: String) -> ActivityItem.ActivityType {
        switch method.lowercased() {
        case "noise": return .noise
        case "lightning": return .lightning
        case "onchain", "bitcoin": return .onchain
        case "subscription": return .subscription
        case "autopay", "auto-pay": return .autoPay
        default: return .noise
        }
    }
    
    private func mapStatus(_ status: PaymentReceiptStatus) -> ActivityItem.ActivityStatus {
        switch status {
        case .pending: return .pending
        case .completed: return .completed
        case .failed, .refunded: return .failed
        }
    }
    
    private func iconForMethod(_ method: String) -> String {
        switch method.lowercased() {
        case "noise": return "antenna.radiowaves.left.and.right"
        case "lightning": return "bolt.fill"
        case "onchain", "bitcoin": return "bitcoinsign.circle.fill"
        case "subscription": return "repeat.circle.fill"
        case "autopay", "auto-pay": return "arrow.clockwise.circle.fill"
        default: return "creditcard"
        }
    }
}

// MARK: - Activity List View

struct ActivityListView: View {
    @StateObject private var viewModel = ActivityListViewModel()
    @State private var showingFilters = false
    
    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                // Filter bar
                filterBar
                
                // Activity list
                if viewModel.isLoading {
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if viewModel.filteredActivities.isEmpty {
                    emptyState
                } else {
                    activityList
                }
            }
            .navigationTitle("Activity")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button {
                        showingFilters = true
                    } label: {
                        Image(systemName: "line.3.horizontal.decrease.circle")
                    }
                }
            }
            .sheet(isPresented: $showingFilters) {
                filterSheet
            }
            .onAppear {
                viewModel.loadActivities()
            }
            .refreshable {
                viewModel.loadActivities()
            }
        }
    }
    
    private var filterBar: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(ActivityListViewModel.ActivityFilter.allCases, id: \.self) { filter in
                    Button {
                        withAnimation {
                            viewModel.filter = filter
                        }
                    } label: {
                        Text(filter.rawValue)
                            .font(.subheadline)
                            .padding(.horizontal, 12)
                            .padding(.vertical, 6)
                            .background(viewModel.filter == filter ? Color.blue : Color(.systemGray5))
                            .foregroundColor(viewModel.filter == filter ? .white : .primary)
                            .cornerRadius(16)
                    }
                }
            }
            .padding(.horizontal)
            .padding(.vertical, 8)
        }
        .background(Color(.systemBackground))
    }
    
    private var activityList: some View {
        List {
            ForEach(viewModel.filteredActivities) { activity in
                NavigationLink {
                    ActivityDetailView(activity: activity)
                } label: {
                    ActivityRowView(activity: activity)
                }
            }
        }
        .listStyle(.plain)
    }
    
    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "clock.arrow.circlepath")
                .font(.system(size: 60))
                .foregroundColor(.secondary)
            
            Text("No Activity Yet")
                .font(.title2.bold())
            
            Text("Your payment history will appear here")
                .font(.subheadline)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
    
    private var filterSheet: some View {
        NavigationStack {
            List {
                ForEach(ActivityListViewModel.ActivityFilter.allCases, id: \.self) { filter in
                    Button {
                        viewModel.filter = filter
                        showingFilters = false
                    } label: {
                        HStack {
                            Text(filter.rawValue)
                            Spacer()
                            if viewModel.filter == filter {
                                Image(systemName: "checkmark")
                                    .foregroundColor(.blue)
                            }
                        }
                    }
                    .foregroundColor(.primary)
                }
            }
            .navigationTitle("Filter Activity")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") {
                        showingFilters = false
                    }
                }
            }
        }
        .presentationDetents([.medium])
    }
}

// MARK: - Activity Row View

struct ActivityRowView: View {
    let activity: ActivityItem
    
    var body: some View {
        HStack(spacing: 12) {
            // Icon
            ZStack {
                Circle()
                    .fill(activity.isIncoming ? Color.green.opacity(0.15) : Color.blue.opacity(0.15))
                    .frame(width: 44, height: 44)
                
                Image(systemName: activity.typeIcon)
                    .font(.system(size: 18))
                    .foregroundColor(activity.isIncoming ? .green : .blue)
            }
            
            // Details
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(activity.displayName)
                        .font(.headline)
                        .lineLimit(1)
                    
                    if activity.status == .pending {
                        Text("Pending")
                            .font(.caption2)
                            .foregroundColor(.white)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color.orange)
                            .cornerRadius(4)
                    }
                }
                
                HStack(spacing: 4) {
                    Image(systemName: activity.methodIcon)
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Text(activity.methodName)
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Text("•")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Text(formatRelativeDate(activity.timestamp))
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            Spacer()
            
            // Amount
            Text(activity.formattedAmount)
                .font(.subheadline.bold())
                .foregroundColor(activity.isIncoming ? .green : .primary)
        }
        .padding(.vertical, 4)
    }
    
    private func formatRelativeDate(_ date: Date) -> String {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .abbreviated
        return formatter.localizedString(for: date, relativeTo: Date())
    }
}

// MARK: - Activity Detail View

struct ActivityDetailView: View {
    let activity: ActivityItem
    
    var body: some View {
        List {
            // Amount section
            Section {
                HStack {
                    Spacer()
                    VStack(spacing: 8) {
                        Image(systemName: activity.isIncoming ? "arrow.down.circle.fill" : "arrow.up.circle.fill")
                            .font(.system(size: 50))
                            .foregroundColor(activity.isIncoming ? .green : .blue)
                        
                        Text(activity.formattedAmount)
                            .font(.title.bold())
                            .foregroundColor(activity.isIncoming ? .green : .primary)
                        
                        Text(activity.isIncoming ? "Received" : "Sent")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                    Spacer()
                }
                .padding(.vertical)
            }
            
            // Details section
            Section("Details") {
                detailRow("Status", value: activity.status.rawValue.capitalized, color: activity.statusColor)
                detailRow("Method", value: activity.methodName)
                detailRow("Date", value: formatDate(activity.timestamp))
            }
            
            // Counterparty section
            Section(activity.isIncoming ? "From" : "To") {
                if let name = activity.counterpartyName {
                    detailRow("Name", value: name)
                }
                VStack(alignment: .leading, spacing: 4) {
                    Text("Public Key")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(activity.counterparty)
                        .font(.caption.monospaced())
                }
                
                Button {
                    UIPasteboard.general.string = activity.counterparty
                } label: {
                    Label("Copy Public Key", systemImage: "doc.on.doc")
                }
            }
            
            // Memo section
            if let memo = activity.memo, !memo.isEmpty {
                Section("Memo") {
                    Text(memo)
                }
            }
        }
        .navigationTitle("Activity Details")
        .navigationBarTitleDisplayMode(.inline)
    }
    
    private func detailRow(_ label: String, value: String, color: Color? = nil) -> some View {
        HStack {
            Text(label)
            Spacer()
            if let color = color {
                Text(value)
                    .foregroundColor(color)
            } else {
                Text(value)
                    .foregroundColor(.secondary)
            }
        }
    }
    
    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
}

#Preview {
    ActivityListView()
}

