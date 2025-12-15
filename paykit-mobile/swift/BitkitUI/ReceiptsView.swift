//
//  ReceiptsView.swift
//  PaykitMobile
//
//  Receipts history UI component for Bitkit integration.
//  This is a template that Bitkit can adapt to their design system.
//

import SwiftUI
import PaykitMobile

/// Receipts view model for Bitkit integration
public class BitkitReceiptsViewModel: ObservableObject {
    @Published public var receipts: [Receipt] = []
    @Published public var filteredReceipts: [Receipt] = []
    @Published public var isLoading = false
    @Published public var searchText = ""
    @Published public var filterDirection: PaymentDirectionFilter = .all
    @Published public var totalSent: UInt64 = 0
    @Published public var totalReceived: UInt64 = 0
    
    private let receiptStorage: ReceiptStorageProtocol
    
    public enum PaymentDirectionFilter: String, CaseIterable {
        case all = "All"
        case sent = "Sent"
        case received = "Received"
    }
    
    public init(receiptStorage: ReceiptStorageProtocol) {
        self.receiptStorage = receiptStorage
    }
    
    func loadReceipts() {
        isLoading = true
        receipts = receiptStorage.recentReceipts(limit: 100)
        totalSent = receiptStorage.totalSent()
        totalReceived = receiptStorage.totalReceived()
        applyFilters()
        isLoading = false
    }
    
    func applyFilters() {
        var filtered = receipts
        
        // Apply direction filter
        switch filterDirection {
        case .all:
            break
        case .sent:
            // Filter for sent payments (Bitkit should implement direction detection)
            break
        case .received:
            // Filter for received payments
            break
        }
        
        // Apply search filter
        if !searchText.isEmpty {
            filtered = filtered.filter { receipt in
                receipt.payer.localizedCaseInsensitiveContains(searchText) ||
                receipt.payee.localizedCaseInsensitiveContains(searchText) ||
                (receipt.amount?.localizedCaseInsensitiveContains(searchText) ?? false)
            }
        }
        
        filteredReceipts = filtered
    }
}

/// Receipts view component
public struct BitkitReceiptsView: View {
    @ObservedObject var viewModel: BitkitReceiptsViewModel
    
    public init(viewModel: BitkitReceiptsViewModel) {
        self.viewModel = viewModel
    }
    
    public var body: some View {
        NavigationView {
            VStack {
                // Stats
                statsSection
                
                // Filter
                filterSection
                
                // Receipts List
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
            .searchable(text: $viewModel.searchText, prompt: "Search receipts")
            .onChange(of: viewModel.searchText) { _ in
                viewModel.applyFilters()
            }
            .onChange(of: viewModel.filterDirection) { _ in
                viewModel.applyFilters()
            }
            .onAppear {
                viewModel.loadReceipts()
            }
        }
    }
    
    private var statsSection: some View {
        HStack(spacing: 16) {
            StatBox(title: "Total Sent", value: formatSats(viewModel.totalSent), color: .blue)
            StatBox(title: "Total Received", value: formatSats(viewModel.totalReceived), color: .green)
        }
        .padding()
    }
    
    private var filterSection: some View {
        Picker("Filter", selection: $viewModel.filterDirection) {
            ForEach(BitkitReceiptsViewModel.PaymentDirectionFilter.allCases, id: \.self) { filter in
                Text(filter.rawValue).tag(filter)
            }
        }
        .pickerStyle(.segmented)
        .padding(.horizontal)
    }
    
    private var receiptsList: some View {
        List(viewModel.filteredReceipts) { receipt in
            ReceiptRow(receipt: receipt)
        }
    }
    
    private var emptyStateView: some View {
        VStack(spacing: 24) {
            Image(systemName: "doc.text")
                .font(.system(size: 80))
                .foregroundColor(.secondary)
            
            Text("No Receipts")
                .font(.title2)
                .fontWeight(.semibold)
            
            Text("Your payment history will appear here")
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
    
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

struct StatBox: View {
    let title: String
    let value: String
    let color: Color
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
            Text(value)
                .font(.title3)
                .fontWeight(.bold)
                .foregroundColor(color)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
}

struct ReceiptRow: View {
    let receipt: Receipt
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(receipt.payer)
                    .font(.headline)
                Spacer()
                if let amount = receipt.amount {
                    Text(amount)
                        .font(.subheadline)
                        .fontWeight(.semibold)
                }
            }
            
            if let currency = receipt.currency {
                Text(currency)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Text("Method: \(receipt.methodId)")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.vertical, 4)
    }
}
