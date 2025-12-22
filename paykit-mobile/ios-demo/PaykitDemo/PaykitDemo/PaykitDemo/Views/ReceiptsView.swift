//
//  ReceiptsView.swift
//  PaykitDemo
//
//  View for browsing and managing payment receipts.
//

import SwiftUI
import Combine

class ReceiptsViewModel: ObservableObject {
    @Published var receipts: [PaymentReceipt] = []
    @Published var searchText = ""
    @Published var filterDirection: PaymentDirection?
    @Published var filterStatus: PaymentReceiptStatus?
    @Published var isLoading = true
    @Published var errorMessage: String?
    @Published var showError = false
    @Published var exportData: String?
    @Published var showExportSheet = false
    
    private let keyManager = KeyManager()
    private var storage: ReceiptStorage {
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        return ReceiptStorage(identityName: identityName)
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
        loadReceipts()
    }
    
    deinit {
        NotificationCenter.default.removeObserver(self)
    }
    
    var filteredReceipts: [PaymentReceipt] {
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
    func createPaymentReceipt(
        client: PaykitClientWrapper,
        direction: PaymentDirection,
        counterpartyKey: String,
        counterpartyName: String? = nil,
        amountSats: UInt64,
        methodId: String,
        memo: String? = nil
    ) -> PaymentReceipt? {
        let payer = direction == .sent ? "self" : counterpartyKey
        let payee = direction == .sent ? counterpartyKey : "self"
        
        guard let ffiReceipt = client.createPaymentReceipt(
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
        var localReceipt = PaymentReceipt.fromFFI(ffiReceipt, direction: direction, counterpartyName: counterpartyName)
        localReceipt.memo = memo
        
        do {
            try storage.addPaymentReceipt(localReceipt)
            loadReceipts()
            return localReceipt
        } catch {
            showErrorMessage("Failed to save receipt: \(error.localizedDescription)")
            return nil
        }
    }
    
    /// Mark a receipt as completed
    func completePaymentReceipt(id: String, txId: String? = nil) {
        guard var receipt = storage.getPaymentReceipt(id: id) else { return }
        receipt.complete(txId: txId)
        do {
            try storage.updatePaymentReceipt(receipt)
            loadReceipts()
        } catch {
            showErrorMessage("Failed to update receipt: \(error.localizedDescription)")
        }
    }
    
    /// Mark a receipt as failed
    func failPaymentReceipt(id: String) {
        guard var receipt = storage.getPaymentReceipt(id: id) else { return }
        receipt.fail()
        do {
            try storage.updatePaymentReceipt(receipt)
            loadReceipts()
        } catch {
            showErrorMessage("Failed to update receipt: \(error.localizedDescription)")
        }
    }
    
    func deletePaymentReceipt(_ receipt: PaymentReceipt) {
        do {
            try storage.deletePaymentReceipt(id: receipt.id)
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
    
    // MARK: - Export Functions
    
    /// Export receipts to JSON format
    func exportToJSON() -> String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        encoder.dateEncodingStrategy = .iso8601
        
        let receiptDicts = filteredReceipts.map { receipt -> [String: Any] in
            [
                "id": receipt.id,
                "direction": receipt.direction.rawValue,
                "counterparty": receipt.counterpartyKey,
                "displayName": receipt.displayName,
                "amount": receipt.amountSats,
                "currency": "SAT",
                "paymentMethod": receipt.paymentMethod,
                "status": receipt.status.rawValue,
                "createdAt": ISO8601DateFormatter().string(from: receipt.createdAt),
                "completedAt": receipt.completedAt.map { ISO8601DateFormatter().string(from: $0) } as Any,
                "memo": receipt.memo as Any,
                "txId": receipt.txId as Any
            ]
        }
        
        if let jsonData = try? JSONSerialization.data(withJSONObject: receiptDicts, options: .prettyPrinted),
           let jsonString = String(data: jsonData, encoding: .utf8) {
            return jsonString
        }
        return "[]"
    }
    
    /// Export receipts to CSV format
    func exportToCSV() -> String {
        var csv = "ID,Direction,Counterparty,Display Name,Amount,Currency,Payment Method,Status,Created At,Completed At,Memo,Transaction ID\n"
        
        let dateFormatter = DateFormatter()
        dateFormatter.dateFormat = "yyyy-MM-dd HH:mm:ss"
        
        for receipt in filteredReceipts {
            let row = [
                receipt.id,
                receipt.direction.rawValue,
                receipt.counterpartyKey,
                receipt.displayName.replacingOccurrences(of: ",", with: ";"),
                String(receipt.amountSats),
                "SAT",
                receipt.paymentMethod,
                receipt.status.rawValue,
                dateFormatter.string(from: receipt.createdAt),
                receipt.completedAt.map { dateFormatter.string(from: $0) } ?? "",
                (receipt.memo ?? "").replacingOccurrences(of: ",", with: ";"),
                receipt.txId ?? ""
            ]
            csv += row.joined(separator: ",") + "\n"
        }
        
        return csv
    }
    
    /// Prepare export data and show share sheet
    func prepareExport(format: ExportFormat) {
        switch format {
        case .json:
            exportData = exportToJSON()
        case .csv:
            exportData = exportToCSV()
        }
        showExportSheet = true
    }
    
    private func showErrorMessage(_ message: String) {
        errorMessage = message
        showError = true
    }
}

enum ExportFormat {
    case json
    case csv
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
                ToolbarItem(placement: .navigationBarLeading) {
                    Menu {
                        Button(action: { viewModel.prepareExport(format: .json) }) {
                            Label("Export as JSON", systemImage: "doc.text")
                        }
                        Button(action: { viewModel.prepareExport(format: .csv) }) {
                            Label("Export as CSV", systemImage: "tablecells")
                        }
                    } label: {
                        Image(systemName: "square.and.arrow.up")
                    }
                    .disabled(viewModel.receipts.isEmpty)
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    HStack(spacing: 12) {
                        NavigationLink(destination: ReceiptLookupView()) {
                            Image(systemName: "magnifyingglass.circle")
                        }
                        
                        Button(action: { showFilterSheet = true }) {
                            Image(systemName: "line.3.horizontal.decrease.circle")
                        }
                    }
                }
            }
            .sheet(isPresented: $showFilterSheet) {
                FilterSheet(viewModel: viewModel)
            }
            .sheet(isPresented: $viewModel.showExportSheet) {
                if let exportData = viewModel.exportData {
                    ShareSheet(items: [exportData])
                }
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
                    viewModel.deletePaymentReceipt(receipt)
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
    let receipt: PaymentReceipt
    
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
    let status: PaymentReceiptStatus
    
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
                        Text("All").tag(PaymentReceiptStatus?.none)
                        Text("Pending").tag(PaymentReceiptStatus?.some(.pending))
                        Text("Completed").tag(PaymentReceiptStatus?.some(.completed))
                        Text("Failed").tag(PaymentReceiptStatus?.some(.failed))
                        Text("Refunded").tag(PaymentReceiptStatus?.some(.refunded))
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

// Note: ReceiptDetailView is now in ReceiptDetailView.swift with enhanced features
// including payment hash display, verification, and receipt lookup

// MARK: - Share Sheet

struct ShareSheet: UIViewControllerRepresentable {
    let items: [Any]
    
    func makeUIViewController(context: Context) -> UIActivityViewController {
        UIActivityViewController(activityItems: items, applicationActivities: nil)
    }
    
    func updateUIViewController(_ uiViewController: UIActivityViewController, context: Context) {}
}

#Preview {
    ReceiptsView()
}

