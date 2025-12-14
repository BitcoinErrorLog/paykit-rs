// ReceivePaymentView.swift
// Receive Payment View (Server Mode)
//
// This view allows users to receive payments via Noise protocol.
// It displays connection info for payers and shows incoming payment requests.

import SwiftUI

// MARK: - Receive Payment View

struct ReceivePaymentView: View {
    @StateObject private var viewModel = NoiseReceiveViewModel()
    @State private var showingQRSheet = false
    @State private var copiedToClipboard = false
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 24) {
                    // Server Status Card
                    serverStatusCard
                    
                    // Connection Info (when listening)
                    if viewModel.isListening {
                        connectionInfoCard
                    }
                    
                    // Pending Requests
                    if !viewModel.pendingRequests.isEmpty {
                        pendingRequestsSection
                    }
                    
                    // Recent Receipts
                    recentReceiptsSection
                    
                    Spacer()
                }
                .padding()
            }
            .navigationTitle("Receive Payments")
            .sheet(isPresented: $showingQRSheet) {
                QRCodeSheet(connectionInfo: viewModel.getConnectionInfo() ?? "")
            }
            .onAppear {
                viewModel.loadRecentReceipts()
            }
        }
    }
    
    // MARK: - Server Status Card
    
    private var serverStatusCard: some View {
        VStack(spacing: 16) {
            HStack {
                Circle()
                    .fill(viewModel.isListening ? Color.green : Color.gray)
                    .frame(width: 12, height: 12)
                
                Text(viewModel.isListening ? "Listening for Payments" : "Not Listening")
                    .font(.headline)
                
                Spacer()
            }
            
            if viewModel.isListening {
                HStack {
                    Image(systemName: "antenna.radiowaves.left.and.right")
                        .foregroundColor(.green)
                    Text("Port: \(viewModel.listeningPort ?? 0)")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                    Spacer()
                }
            }
            
            // Control Button
            Button(action: {
                Task {
                    if viewModel.isListening {
                        viewModel.stopListening()
                    } else {
                        await viewModel.startListening()
                    }
                }
            }) {
                HStack {
                    Image(systemName: viewModel.isListening ? "stop.fill" : "play.fill")
                    Text(viewModel.isListening ? "Stop Listening" : "Start Listening")
                }
                .frame(maxWidth: .infinity)
                .padding()
                .background(viewModel.isListening ? Color.red : Color.blue)
                .foregroundColor(.white)
                .cornerRadius(12)
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(16)
    }
    
    // MARK: - Connection Info Card
    
    private var connectionInfoCard: some View {
        VStack(spacing: 16) {
            HStack {
                Text("Connection Info")
                    .font(.headline)
                Spacer()
                Button(action: { showingQRSheet = true }) {
                    Image(systemName: "qrcode")
                        .font(.title2)
                }
            }
            
            Text("Share this with payers to receive payments:")
                .font(.caption)
                .foregroundColor(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
            
            // Noise Public Key
            if let pubkey = viewModel.noisePubkeyHex {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Noise Public Key")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    HStack {
                        Text(pubkey.prefix(16) + "..." + pubkey.suffix(8))
                            .font(.system(.caption, design: .monospaced))
                        
                        Spacer()
                        
                        Button(action: {
                            copyToClipboard(pubkey)
                        }) {
                            Image(systemName: copiedToClipboard ? "checkmark" : "doc.on.doc")
                                .foregroundColor(copiedToClipboard ? .green : .blue)
                        }
                    }
                }
            }
            
            // Full Connection String
            if let connInfo = viewModel.getConnectionInfo() {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Connection String")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    HStack {
                        Text(connInfo.prefix(30) + "...")
                            .font(.system(.caption, design: .monospaced))
                            .lineLimit(1)
                        
                        Spacer()
                        
                        Button(action: {
                            copyToClipboard(connInfo)
                        }) {
                            Image(systemName: "doc.on.doc")
                                .foregroundColor(.blue)
                        }
                    }
                }
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(16)
    }
    
    // MARK: - Pending Requests Section
    
    private var pendingRequestsSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Pending Requests")
                    .font(.headline)
                
                Spacer()
                
                Text("\(viewModel.pendingRequests.count)")
                    .font(.caption)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.orange)
                    .foregroundColor(.white)
                    .cornerRadius(8)
            }
            
            ForEach(viewModel.pendingRequests) { request in
                PendingRequestCard(
                    request: request,
                    onAccept: {
                        Task { await viewModel.acceptRequest(request) }
                    },
                    onDecline: {
                        Task { await viewModel.declineRequest(request) }
                    }
                )
            }
        }
    }
    
    // MARK: - Recent Receipts Section
    
    private var recentReceiptsSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Recent Receipts")
                .font(.headline)
            
            if viewModel.recentReceipts.isEmpty {
                Text("No receipts yet")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding()
            } else {
                ForEach(viewModel.recentReceipts, id: \.id) { receipt in
                    ReceiptRow(receipt: receipt)
                }
            }
        }
    }
    
    // MARK: - Helpers
    
    private func copyToClipboard(_ text: String) {
        UIPasteboard.general.string = text
        copiedToClipboard = true
        
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
            copiedToClipboard = false
        }
    }
}

// MARK: - Pending Request Card

struct PendingRequestCard: View {
    let request: NoiseReceiveViewModel.PendingPaymentRequest
    let onAccept: () -> Void
    let onDecline: () -> Void
    
    var body: some View {
        VStack(spacing: 12) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("From: \(request.payerPubkey.prefix(12))...")
                        .font(.subheadline)
                    
                    if let amount = request.amount {
                        Text("\(amount) \(request.currency ?? "SAT")")
                            .font(.headline)
                    }
                    
                    Text(request.methodId.capitalized)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                Text(request.receivedAt, style: .relative)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            HStack(spacing: 12) {
                Button(action: onDecline) {
                    Text("Decline")
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                        .background(Color(.systemGray5))
                        .foregroundColor(.red)
                        .cornerRadius(8)
                }
                
                Button(action: onAccept) {
                    Text("Accept")
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                        .background(Color.green)
                        .foregroundColor(.white)
                        .cornerRadius(8)
                }
            }
        }
        .padding()
        .background(Color.orange.opacity(0.1))
        .cornerRadius(12)
    }
}

// MARK: - Receipt Row

struct ReceiptRow: View {
    let receipt: StoredReceipt
    
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text(receipt.id.prefix(16) + "...")
                    .font(.system(.caption, design: .monospaced))
                
                Text("\(receipt.amount) \(receipt.currency)")
                    .font(.subheadline)
                    .fontWeight(.medium)
            }
            
            Spacer()
            
            VStack(alignment: .trailing, spacing: 4) {
                Text(receipt.timestamp, style: .date)
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                StatusBadge(status: receipt.status)
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
}

// MARK: - Status Badge

struct StatusBadge: View {
    let status: ReceiptStatus
    
    var body: some View {
        Text(status.rawValue.capitalized)
            .font(.caption2)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(backgroundColor)
            .foregroundColor(.white)
            .cornerRadius(4)
    }
    
    private var backgroundColor: Color {
        switch status {
        case .pending: return .orange
        case .completed: return .green
        case .failed: return .red
        case .cancelled: return .gray
        }
    }
}

// MARK: - QR Code Sheet

struct QRCodeSheet: View {
    let connectionInfo: String
    @Environment(\.dismiss) private var dismiss
    
    var body: some View {
        NavigationView {
            VStack(spacing: 24) {
                Text("Scan to Send Payment")
                    .font(.headline)
                
                // QR Code Placeholder
                // In production, generate actual QR code
                ZStack {
                    RoundedRectangle(cornerRadius: 12)
                        .fill(Color(.systemGray6))
                        .frame(width: 250, height: 250)
                    
                    VStack(spacing: 8) {
                        Image(systemName: "qrcode")
                            .font(.system(size: 80))
                            .foregroundColor(.gray)
                        
                        Text("QR Code")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                
                // Connection String
                VStack(alignment: .leading, spacing: 8) {
                    Text("Connection Info")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    Text(connectionInfo)
                        .font(.system(.caption, design: .monospaced))
                        .padding()
                        .background(Color(.systemGray6))
                        .cornerRadius(8)
                }
                .padding(.horizontal)
                
                Button(action: {
                    UIPasteboard.general.string = connectionInfo
                }) {
                    HStack {
                        Image(systemName: "doc.on.doc")
                        Text("Copy Connection Info")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(12)
                }
                .padding(.horizontal)
                
                Spacer()
            }
            .padding()
            .navigationTitle("Share Connection")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") { dismiss() }
                }
            }
        }
    }
}

// MARK: - Preview

#Preview {
    ReceivePaymentView()
}

