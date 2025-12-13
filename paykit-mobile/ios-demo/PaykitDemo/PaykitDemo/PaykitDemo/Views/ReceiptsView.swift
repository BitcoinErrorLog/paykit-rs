//
//  ReceiptsView.swift
//  PaykitDemo
//
//  View for browsing and managing payment receipts.
//

import SwiftUI

class ReceiptsViewModel: ObservableObject {
    @Published var receipts: [Receipt] = []
    @Published var searchText = ""
    @Published var filterDirection: PaymentDirection?
    @Published var filterStatus: PaymentStatus?
    @Published var isLoading = true
    @Published var errorMessage: String?
    @Published var showError = false
    
    private let storage = ReceiptStorage()
    
    var filteredReceipts: [Receipt] {
        var result = receipts
        
        // Apply direction filter
        if let direction = filterDirection {
            result = result.filter { $0.direction == direction }
        }
        
        // Apply status filter
        if let status = filterStatus {
            result = result.filter { $0.status == status }
        }
        
        // Apply search
        if !searchText.isEmpty {
            let query = searchText.lowercased()
            result = result.filter { receipt in
                receipt.displayName.lowercased().contains(query) ||
                receipt.counterpartyKey.lowercased().contains(query) ||
                (receipt.memo?.lowercased().contains(query) ?? false)
            }
        }
        
        return result
    }
    
    func loadReceipts() {
        isLoading = true
        receipts = storage.listReceipts()
        isLoading = false
    }
    
    /// Create a receipt using the PaykitClient FFI and store it
    /// - Parameters:
    ///   - client: The PaykitClientWrapper
    ///   - direction: Payment direction (sent or received)
    ///   - counterpartyKey: The counterparty's public key
    ///   - counterpartyName: Optional display name
    ///   - amountSats: Amount in satoshis
    ///   - methodId: Payment method (e.g., "lightning", "onchain")
    ///   - memo: Optional memo/note
    /// - Returns: The created receipt, or nil if failed
    @discardableResult
    func createReceipt(
        client: PaykitClientWrapper,
        direction: PaymentDirection,
        counterpartyKey: String,
        counterpartyName: String? = nil,
        amountSats: UInt64,
        methodId: String,
        memo: String? = nil
    ) -> Receipt? {
        let payer = direction == .sent ? "self" : counterpartyKey
        let payee = direction == .sent ? counterpartyKey : "self"
        
        guard let ffiReceipt = client.createReceipt(
            payer: payer,
            payee: payee,
            methodId: methodId,
            amount: String(amountSats),
            currency: "SAT"
        ) else {
            showErrorMessage("Failed to create receipt via FFI")
            return nil
        }
        
        // Convert FFI receipt to local storage format
        var localReceipt = Receipt.fromFFI(ffiReceipt, direction: direction, counterpartyName: counterpartyName)
        localReceipt.memo = memo
        
        do {
            try storage.addReceipt(localReceipt)
            loadReceipts()
            return localReceipt
        } catch {
            showErrorMessage("Failed to save receipt: \(error.localizedDescription)")
            return nil
        }
    }
    
    /// Mark a receipt as completed
    func completeReceipt(id: String, txId: String? = nil) {
        guard var receipt = storage.getReceipt(id: id) else { return }
        receipt.complete(txId: txId)
        do {
            try storage.updateReceipt(receipt)
            loadReceipts()
        } catch {
            showErrorMessage("Failed to update receipt: \(error.localizedDescription)")
        }
    }
    
    /// Mark a receipt as failed
    func failReceipt(id: String) {
        guard var receipt = storage.getReceipt(id: id) else { return }
        receipt.fail()
        do {
            try storage.updateReceipt(receipt)
            loadReceipts()
        } catch {
            showErrorMessage("Failed to update receipt: \(error.localizedDescription)")
        }
    }
    
    func deleteReceipt(_ receipt: Receipt) {
        do {
            try storage.deleteReceipt(id: receipt.id)
            loadReceipts()
        } catch {
            showErrorMessage("Failed to delete receipt: \(error.localizedDescription)")
        }
    }
    
    func clearFilters() {
        filterDirection = nil
        filterStatus = nil
        searchText = ""
    }
    
    private func showErrorMessage(_ message: String) {
        errorMessage = message
        showError = true
    }
}

struct ReceiptsView: View {
    @StateObject private var viewModel = ReceiptsViewModel()
    @State private var showFilterSheet = false
    
    var body: some View {
        NavigationView {
            VStack(spacing: 0) {
                // Search Bar
                searchBar
                
                // Active Filters
                if viewModel.filterDirection != nil || viewModel.filterStatus != nil {
                    activeFiltersBar
                }
                
                // Receipt List
                if viewModel.isLoading {
                    ProgressView()
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if viewModel.filteredReceipts.isEmpty {
                    emptyStateView
                } else {
                    receiptsList
                }
            }
            .navigationTitle("Receipts")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { showFilterSheet = true }) {
                        Image(systemName: "line.3.horizontal.decrease.circle")
                    }
                }
            }
            .sheet(isPresented: $showFilterSheet) {
                FilterSheet(viewModel: viewModel)
            }
            .onAppear {
                viewModel.loadReceipts()
            }
            .refreshable {
                viewModel.loadReceipts()
            }
        }
    }
    
    // MARK: - Search Bar
    
    private var searchBar: some View {
        HStack {
            Image(systemName: "magnifyingglass")
                .foregroundColor(.secondary)
            
            TextField("Search receipts...", text: $viewModel.searchText)
                .textFieldStyle(PlainTextFieldStyle())
            
            if !viewModel.searchText.isEmpty {
                Button(action: { viewModel.searchText = "" }) {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(10)
        .background(Color(.systemGray6))
        .cornerRadius(10)
        .padding()
    }
    
    // MARK: - Active Filters Bar
    
    private var activeFiltersBar: some View {
        HStack {
            if let direction = viewModel.filterDirection {
                FilterChip(text: direction.rawValue.capitalized) {
                    viewModel.filterDirection = nil
                }
            }
            
            if let status = viewModel.filterStatus {
                FilterChip(text: status.rawValue.capitalized) {
                    viewModel.filterStatus = nil
                }
            }
            
            Spacer()
            
            Button("Clear All") {
                viewModel.clearFilters()
            }
            .font(.caption)
            .foregroundColor(.blue)
        }
        .padding(.horizontal)
        .padding(.bottom, 8)
    }
    
    // MARK: - Receipts List
    
    private var receiptsList: some View {
        List {
            ForEach(viewModel.filteredReceipts) { receipt in
                NavigationLink(destination: ReceiptDetailView(receipt: receipt)) {
                    ReceiptRowView(receipt: receipt)
                }
            }
            .onDelete { indexSet in
                for index in indexSet {
                    let receipt = viewModel.filteredReceipts[index]
                    viewModel.deleteReceipt(receipt)
                }
            }
        }
        .listStyle(PlainListStyle())
    }
    
    // MARK: - Empty State
    
    private var emptyStateView: some View {
        VStack(spacing: 16) {
            Image(systemName: "doc.text.magnifyingglass")
                .font(.system(size: 60))
                .foregroundColor(.secondary)
            
            Text("No receipts found")
                .font(.title3)
                .fontWeight(.medium)
            
            if viewModel.filterDirection != nil || viewModel.filterStatus != nil || !viewModel.searchText.isEmpty {
                Text("Try adjusting your filters or search")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                
                Button("Clear Filters") {
                    viewModel.clearFilters()
                }
                .buttonStyle(.bordered)
            } else {
                Text("Payment receipts will appear here")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

// MARK: - Receipt Row View

struct ReceiptRowView: View {
    let receipt: Receipt
    
    var body: some View {
        HStack {
            // Direction indicator
            Circle()
                .fill(receipt.direction == .sent ? Color.red.opacity(0.2) : Color.green.opacity(0.2))
                .frame(width: 44, height: 44)
                .overlay(
                    Image(systemName: receipt.direction == .sent ? "arrow.up" : "arrow.down")
                        .foregroundColor(receipt.direction == .sent ? .red : .green)
                )
            
            VStack(alignment: .leading, spacing: 4) {
                Text(receipt.displayName)
                    .font(.body)
                    .fontWeight(.medium)
                
                HStack(spacing: 8) {
                    Text(receipt.paymentMethod)
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Text("•")
                        .foregroundColor(.secondary)
                    
                    Text(formatDate(receipt.createdAt))
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            Spacer()
            
            VStack(alignment: .trailing, spacing: 4) {
                Text(receipt.formattedAmount)
                    .font(.body)
                    .fontWeight(.medium)
                    .foregroundColor(receipt.direction == .sent ? .red : .green)
                
                StatusBadge(status: receipt.status)
            }
        }
        .padding(.vertical, 4)
    }
    
    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .short
        formatter.timeStyle = .none
        return formatter.string(from: date)
    }
}

// MARK: - Status Badge

struct StatusBadge: View {
    let status: PaymentStatus
    
    var body: some View {
        Text(status.rawValue.capitalized)
            .font(.caption2)
            .fontWeight(.medium)
            .foregroundColor(statusColor)
            .padding(.horizontal, 8)
            .padding(.vertical, 2)
            .background(statusColor.opacity(0.15))
            .cornerRadius(4)
    }
    
    private var statusColor: Color {
        switch status {
        case .pending: return .orange
        case .completed: return .green
        case .failed: return .red
        case .refunded: return .purple
        }
    }
}

// MARK: - Filter Chip

struct FilterChip: View {
    let text: String
    let onRemove: () -> Void
    
    var body: some View {
        HStack(spacing: 4) {
            Text(text)
                .font(.caption)
                .fontWeight(.medium)
            
            Button(action: onRemove) {
                Image(systemName: "xmark")
                    .font(.caption2)
            }
        }
        .foregroundColor(.blue)
        .padding(.horizontal, 10)
        .padding(.vertical, 4)
        .background(Color.blue.opacity(0.15))
        .cornerRadius(16)
    }
}

// MARK: - Filter Sheet

struct FilterSheet: View {
    @ObservedObject var viewModel: ReceiptsViewModel
    @Environment(\.dismiss) private var dismiss
    
    var body: some View {
        NavigationView {
            Form {
                Section("Direction") {
                    Picker("Direction", selection: $viewModel.filterDirection) {
                        Text("All").tag(PaymentDirection?.none)
                        Text("Sent").tag(PaymentDirection?.some(.sent))
                        Text("Received").tag(PaymentDirection?.some(.received))
                    }
                    .pickerStyle(SegmentedPickerStyle())
                }
                
                Section("Status") {
                    Picker("Status", selection: $viewModel.filterStatus) {
                        Text("All").tag(PaymentStatus?.none)
                        Text("Pending").tag(PaymentStatus?.some(.pending))
                        Text("Completed").tag(PaymentStatus?.some(.completed))
                        Text("Failed").tag(PaymentStatus?.some(.failed))
                        Text("Refunded").tag(PaymentStatus?.some(.refunded))
                    }
                    .pickerStyle(MenuPickerStyle())
                }
                
                Section {
                    Button("Clear All Filters") {
                        viewModel.clearFilters()
                    }
                    .foregroundColor(.red)
                }
            }
            .navigationTitle("Filters")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
        }
    }
}

// MARK: - Receipt Detail View

struct ReceiptDetailView: View {
    let receipt: Receipt
    
    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // Amount Header
                VStack(spacing: 8) {
                    Image(systemName: receipt.direction == .sent ? "arrow.up.circle.fill" : "arrow.down.circle.fill")
                        .font(.system(size: 60))
                        .foregroundColor(receipt.direction == .sent ? .red : .green)
                    
                    Text(receipt.formattedAmount)
                        .font(.largeTitle)
                        .fontWeight(.bold)
                        .foregroundColor(receipt.direction == .sent ? .red : .green)
                    
                    StatusBadge(status: receipt.status)
                }
                .padding(.top, 20)
                
                // Details Card
                VStack(alignment: .leading, spacing: 16) {
                    DetailRow(label: "Counterparty", value: receipt.displayName)
                    DetailRow(label: "Public Key", value: receipt.abbreviatedCounterparty)
                    DetailRow(label: "Payment Method", value: receipt.paymentMethod)
                    DetailRow(label: "Created", value: formatDate(receipt.createdAt))
                    
                    if let completedAt = receipt.completedAt {
                        DetailRow(label: "Completed", value: formatDate(completedAt))
                    }
                    
                    if let memo = receipt.memo, !memo.isEmpty {
                        DetailRow(label: "Memo", value: memo)
                    }
                    
                    if let txId = receipt.txId, !txId.isEmpty {
                        DetailRow(label: "Transaction ID", value: txId)
                    }
                }
                .padding()
                .background(Color(.systemBackground))
                .cornerRadius(12)
                .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
                .padding(.horizontal)
                
                Spacer()
            }
        }
        .background(Color(.systemGroupedBackground))
        .navigationTitle("Receipt Details")
        .navigationBarTitleDisplayMode(.inline)
    }
    
    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
}

struct DetailRow: View {
    let label: String
    let value: String
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            
            Text(value)
                .font(.body)
        }
    }
}

#Preview {
    ReceiptsView()
}

