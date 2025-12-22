//
//  ReceiptDetailView.swift
//  PaykitDemo
//
//  Detailed receipt view with payment hash lookup and verification
//

import SwiftUI

/// View for displaying detailed payment receipt information
/// Accepts either PaymentReceipt directly or ReceiptDisplayModel for extended details
struct ReceiptDetailView: View {
    let displayModel: ReceiptDisplayModel
    @Environment(\.dismiss) private var dismiss
    @State private var copied: CopiedField?
    @State private var showingVerification = false
    @State private var isVerifying = false
    @State private var verificationResult: VerificationResult?
    
    enum CopiedField {
        case paymentHash
        case preimage
        case txid
    }
    
    /// Initialize with PaymentReceipt
    init(receipt: PaymentReceipt, paymentHash: String? = nil, preimage: String? = nil) {
        self.displayModel = ReceiptDisplayModel(
            receipt: receipt,
            paymentHash: paymentHash,
            preimage: preimage
        )
    }
    
    /// Initialize with ReceiptDisplayModel
    init(displayModel: ReceiptDisplayModel) {
        self.displayModel = displayModel
    }
    
    var body: some View {
        List {
            // Amount Section
            Section {
                HStack {
                    Spacer()
                    VStack(spacing: 8) {
                        Text(formatSats(displayModel.amountSats))
                            .font(.system(size: 36, weight: .bold, design: .rounded))
                        
                        HStack(spacing: 8) {
                            Image(systemName: displayModel.isIncoming ? "arrow.down.circle.fill" : "arrow.up.circle.fill")
                                .foregroundColor(displayModel.isIncoming ? .green : .blue)
                            
                            Text(displayModel.isIncoming ? "Received" : "Sent")
                                .font(.headline)
                                .foregroundColor(displayModel.isIncoming ? .green : .blue)
                        }
                        
                        Text(formatDate(displayModel.timestamp))
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Spacer()
                }
                .padding(.vertical, 16)
            }
            
            // Status Section
            Section {
                HStack {
                    Text("Status")
                    Spacer()
                    statusBadge(displayModel.status)
                }
                
                HStack {
                    Text("Method")
                    Spacer()
                    HStack(spacing: 4) {
                        Image(systemName: displayModel.methodIcon)
                            .foregroundColor(.blue)
                        Text(displayModel.methodDisplayName)
                            .foregroundColor(.secondary)
                    }
                }
                
                if let fee = displayModel.feeSats {
                    HStack {
                        Text("Fee")
                        Spacer()
                        Text("\(fee) sats")
                            .foregroundColor(.secondary)
                    }
                }
                
                if let confirmations = displayModel.confirmations {
                    HStack {
                        Text("Confirmations")
                        Spacer()
                        Text("\(confirmations)")
                            .foregroundColor(.secondary)
                    }
                }
            } header: {
                Text("Details")
            }
            
            // Counterparty Section
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text(displayModel.isIncoming ? "From" : "To")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Text(displayModel.counterpartyPubkey)
                        .font(.system(.caption, design: .monospaced))
                    
                    Button {
                        copyToClipboard(displayModel.counterpartyPubkey, field: nil)
                    } label: {
                        Label("Copy", systemImage: "doc.on.doc")
                    }
                    .buttonStyle(.bordered)
                }
            } header: {
                Text("Counterparty")
            }
            
            // Payment Hash Section (Lightning)
            if let paymentHash = displayModel.paymentHash {
                Section {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Payment Hash")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        Text(paymentHash)
                            .font(.system(.caption2, design: .monospaced))
                            .lineLimit(2)
                        
                        Button {
                            copyToClipboard(paymentHash, field: .paymentHash)
                        } label: {
                            Label(copied == .paymentHash ? "Copied!" : "Copy", 
                                  systemImage: copied == .paymentHash ? "checkmark" : "doc.on.doc")
                        }
                        .buttonStyle(.bordered)
                    }
                    
                    if let preimage = displayModel.preimage {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Preimage (Proof of Payment)")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            
                            Text(preimage)
                                .font(.system(.caption2, design: .monospaced))
                                .lineLimit(2)
                            
                            Button {
                                copyToClipboard(preimage, field: .preimage)
                            } label: {
                                Label(copied == .preimage ? "Copied!" : "Copy",
                                      systemImage: copied == .preimage ? "checkmark" : "doc.on.doc")
                            }
                            .buttonStyle(.bordered)
                        }
                    }
                } header: {
                    Text("Lightning Details")
                }
            }
            
            // Transaction ID Section (On-chain)
            if let txid = displayModel.txid {
                Section {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Transaction ID")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        Text(txid)
                            .font(.system(.caption2, design: .monospaced))
                            .lineLimit(2)
                        
                        HStack {
                            Button {
                                copyToClipboard(txid, field: .txid)
                            } label: {
                                Label(copied == .txid ? "Copied!" : "Copy",
                                      systemImage: copied == .txid ? "checkmark" : "doc.on.doc")
                            }
                            .buttonStyle(.bordered)
                            
                            Button {
                                openInExplorer(txid: txid)
                            } label: {
                                Label("View in Explorer", systemImage: "safari")
                            }
                            .buttonStyle(.bordered)
                        }
                    }
                } header: {
                    Text("On-Chain Details")
                }
            }
            
            // Memo/Description Section
            if let memo = displayModel.memo {
                Section {
                    Text(memo)
                        .foregroundColor(.secondary)
                } header: {
                    Text("Memo")
                }
            }
            
            // Verification Section
            Section {
                Button {
                    Task { await verifyPayment() }
                } label: {
                    HStack {
                        if isVerifying {
                            ProgressView()
                                .padding(.trailing, 4)
                        }
                        Label(
                            isVerifying ? "Verifying..." : "Verify Payment",
                            systemImage: "checkmark.shield"
                        )
                    }
                }
                .disabled(isVerifying)
                
                if let result = verificationResult {
                    HStack {
                        Image(systemName: result.isValid ? "checkmark.circle.fill" : "xmark.circle.fill")
                            .foregroundColor(result.isValid ? .green : .red)
                        Text(result.message)
                            .font(.caption)
                    }
                }
            } header: {
                Text("Verification")
            }
        }
        .navigationTitle("Receipt")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                ShareLink(item: generateShareText()) {
                    Image(systemName: "square.and.arrow.up")
                }
            }
        }
    }
    
    // MARK: - Helper Views
    
    private func statusBadge(_ status: PaymentReceiptStatus) -> some View {
        Text(displayModel.statusDisplayName)
            .font(.caption.bold())
            .foregroundColor(.white)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(displayModel.statusColor)
            .cornerRadius(4)
    }
    
    // MARK: - Actions
    
    private func copyToClipboard(_ text: String, field: CopiedField?) {
        UIPasteboard.general.string = text
        if let field = field {
            copied = field
            DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
                if copied == field {
                    copied = nil
                }
            }
        }
    }
    
    private func openInExplorer(txid: String) {
        // Would open mempool.space or similar
        if let url = URL(string: "https://mempool.space/tx/\(txid)") {
            UIApplication.shared.open(url)
        }
    }
    
    private func verifyPayment() async {
        isVerifying = true
        
        // Simulate verification
        try? await Task.sleep(nanoseconds: 1_500_000_000)
        
        await MainActor.run {
            if displayModel.status == .completed {
                verificationResult = VerificationResult(
                    isValid: true,
                    message: "Payment verified successfully"
                )
            } else {
                verificationResult = VerificationResult(
                    isValid: false,
                    message: "Payment not yet confirmed"
                )
            }
            isVerifying = false
        }
    }
    
    private func generateShareText() -> String {
        var text = "Payment Receipt\n"
        text += "Amount: \(formatSats(displayModel.amountSats))\n"
        text += "Status: \(displayModel.statusDisplayName)\n"
        text += "Date: \(formatDate(displayModel.timestamp))\n"
        if let hash = displayModel.paymentHash {
            text += "Payment Hash: \(hash)\n"
        }
        if let txid = displayModel.txid {
            text += "Transaction ID: \(txid)\n"
        }
        return text
    }
    
    // MARK: - Formatters
    
    private func formatSats(_ sats: UInt64) -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        return "\(formatter.string(from: NSNumber(value: sats)) ?? "\(sats)") sats"
    }
    
    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .long
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
}

// MARK: - Supporting Types

/// Extended receipt view model for detailed display (wraps PaymentReceipt)
struct ReceiptDisplayModel: Identifiable {
    let id: String
    let receipt: PaymentReceipt
    
    // Additional detail fields not in base model
    let paymentHash: String?
    let preimage: String?
    let feeSats: UInt64?
    let confirmations: Int?
    
    init(receipt: PaymentReceipt, paymentHash: String? = nil, preimage: String? = nil, feeSats: UInt64? = nil, confirmations: Int? = nil) {
        self.id = receipt.id
        self.receipt = receipt
        self.paymentHash = paymentHash
        self.preimage = preimage
        self.feeSats = feeSats
        self.confirmations = confirmations
    }
    
    // Convenience accessors
    var amountSats: UInt64 { receipt.amountSats }
    var direction: PaymentDirection { receipt.direction }
    var status: PaymentReceiptStatus { receipt.status }
    var paymentMethod: String { receipt.paymentMethod }
    var timestamp: Date { receipt.createdAt }
    var counterpartyPubkey: String { receipt.counterpartyKey }
    var txid: String? { receipt.txId }
    var memo: String? { receipt.memo }
    
    var isIncoming: Bool { direction == .received }
    
    var statusDisplayName: String {
        switch status {
        case .pending: return "Pending"
        case .completed: return "Completed"
        case .failed: return "Failed"
        case .refunded: return "Refunded"
        }
    }
    
    var statusColor: Color {
        switch status {
        case .pending: return .orange
        case .completed: return .green
        case .failed: return .red
        case .refunded: return .purple
        }
    }
    
    var methodDisplayName: String {
        switch paymentMethod.lowercased() {
        case "lightning": return "Lightning"
        case "onchain", "bitcoin": return "On-Chain"
        case "noise": return "Noise"
        default: return paymentMethod
        }
    }
    
    var methodIcon: String {
        switch paymentMethod.lowercased() {
        case "lightning": return "bolt.fill"
        case "onchain", "bitcoin": return "bitcoinsign.circle.fill"
        case "noise": return "antenna.radiowaves.left.and.right"
        default: return "creditcard"
        }
    }
}

struct VerificationResult {
    let isValid: Bool
    let message: String
}

// MARK: - Receipt Lookup View

struct ReceiptLookupView: View {
    @State private var searchQuery = ""
    @State private var isSearching = false
    @State private var foundDisplayModel: ReceiptDisplayModel?
    @State private var errorMessage: String?
    @EnvironmentObject var appState: AppState
    
    var body: some View {
        VStack(spacing: 24) {
            // Header
            VStack(spacing: 12) {
                Image(systemName: "magnifyingglass.circle.fill")
                    .font(.system(size: 60))
                    .foregroundColor(.blue)
                
                Text("Receipt Lookup")
                    .font(.title2.bold())
                
                Text("Search by payment hash or transaction ID")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)
            }
            .padding(.top, 40)
            
            // Search Field
            VStack(alignment: .leading, spacing: 8) {
                Text("Payment Hash or Transaction ID")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                HStack {
                    TextField("Enter hash or txid...", text: $searchQuery)
                        .textFieldStyle(.roundedBorder)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                    
                    Button {
                        // Would open QR scanner
                    } label: {
                        Image(systemName: "qrcode.viewfinder")
                    }
                    .buttonStyle(.bordered)
                }
            }
            .padding(.horizontal)
            
            // Search Button
            Button {
                Task { await searchReceipt() }
            } label: {
                HStack {
                    if isSearching {
                        ProgressView()
                            .tint(.white)
                    }
                    Text(isSearching ? "Searching..." : "Search")
                }
                .font(.headline)
                .frame(maxWidth: .infinity)
                .padding()
                .background(searchQuery.isEmpty || isSearching ? Color.gray : Color.blue)
                .foregroundColor(.white)
                .cornerRadius(12)
            }
            .disabled(searchQuery.isEmpty || isSearching)
            .padding(.horizontal)
            
            // Error Message
            if let error = errorMessage {
                Text(error)
                    .font(.caption)
                    .foregroundColor(.red)
                    .padding()
                    .background(Color.red.opacity(0.1))
                    .cornerRadius(8)
                    .padding(.horizontal)
            }
            
            Spacer()
        }
        .navigationTitle("Find Receipt")
        .sheet(item: $foundDisplayModel) { displayModel in
            NavigationStack {
                ReceiptDetailView(displayModel: displayModel)
            }
        }
    }
    
    private func searchReceipt() async {
        isSearching = true
        errorMessage = nil
        
        // Simulate search
        try? await Task.sleep(nanoseconds: 1_000_000_000)
        
        await MainActor.run {
            // For demo, generate a mock receipt
            if searchQuery.count >= 16 {
                let receipt = PaymentReceipt(
                    direction: .sent,
                    counterpartyKey: "z6mktest123456789",
                    counterpartyName: nil,
                    amountSats: 10000,
                    paymentMethod: "lightning",
                    memo: "Demo payment"
                )
                foundDisplayModel = ReceiptDisplayModel(
                    receipt: receipt,
                    paymentHash: searchQuery,
                    preimage: UUID().uuidString.replacingOccurrences(of: "-", with: ""),
                    feeSats: 10,
                    confirmations: nil
                )
            } else {
                errorMessage = "No receipt found for this query. Check the payment hash or transaction ID and try again."
            }
            isSearching = false
        }
    }
}

#Preview("Receipt Detail") {
    let receipt = PaymentReceipt(
        direction: .sent,
        counterpartyKey: "z6mktest1234567890abcdef",
        counterpartyName: nil,
        amountSats: 50000,
        paymentMethod: "lightning",
        memo: "Payment for services"
    )
    let displayModel = ReceiptDisplayModel(
        receipt: receipt,
        paymentHash: "abc123def456789012345678901234567890abcdef123456789012345678901234",
        preimage: "preimage123456789012345678901234567890abcdef",
        feeSats: 5,
        confirmations: nil
    )
    return NavigationStack {
        ReceiptDetailView(displayModel: displayModel)
    }
}

#Preview("Receipt Lookup") {
    NavigationStack {
        ReceiptLookupView()
    }
    .environmentObject(AppState())
}

