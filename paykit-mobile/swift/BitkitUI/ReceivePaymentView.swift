//
//  ReceivePaymentView.swift
//  PaykitMobile
//
//  Receive Payment UI component for Bitkit integration.
//  This is a template that Bitkit can adapt to their design system.
//

import SwiftUI
import PaykitMobile

/// Receive Payment view model for Bitkit integration
public class BitkitReceivePaymentViewModel: ObservableObject {
    @Published public var isListening = false
    @Published public var connectionInfo: String = ""
    @Published public var qrCodeData: String = ""
    @Published public var incomingRequests: [PaymentRequest] = []
    @Published public var errorMessage: String?
    
    private let paykitClient: PaykitClient
    
    public init(paykitClient: PaykitClient) {
        self.paykitClient = paykitClient
    }
    
    func startListening() {
        // Bitkit should implement Noise server mode listening
        // This is a placeholder
        isListening = true
        // Generate connection info and QR code
        connectionInfo = "pubky://..." // Bitkit should generate this
        qrCodeData = connectionInfo
    }
    
    func stopListening() {
        isListening = false
        connectionInfo = ""
        qrCodeData = ""
    }
    
    func acceptRequest(_ request: PaymentRequest) {
        // Bitkit should implement payment request acceptance
        // This would execute the payment and generate a receipt
    }
    
    func declineRequest(_ request: PaymentRequest) {
        // Bitkit should implement payment request decline
        incomingRequests.removeAll { $0.requestId == request.requestId }
    }
}

/// Receive Payment view component
public struct BitkitReceivePaymentView: View {
    @ObservedObject var viewModel: BitkitReceivePaymentViewModel
    
    public init(viewModel: BitkitReceivePaymentViewModel) {
        self.viewModel = viewModel
    }
    
    public var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 24) {
                    // Connection Status
                    connectionStatusSection
                    
                    // QR Code
                    if viewModel.isListening {
                        qrCodeSection
                    }
                    
                    // Connection Info
                    if viewModel.isListening {
                        connectionInfoSection
                    }
                    
                    // Incoming Requests
                    if !viewModel.incomingRequests.isEmpty {
                        incomingRequestsSection
                    }
                    
                    // Control Buttons
                    controlButtonsSection
                }
                .padding()
            }
            .navigationTitle("Receive Payment")
        }
    }
    
    private var connectionStatusSection: some View {
        VStack(spacing: 8) {
            HStack {
                Circle()
                    .fill(viewModel.isListening ? Color.green : Color.gray)
                    .frame(width: 12, height: 12)
                Text(viewModel.isListening ? "Listening for payments" : "Not listening")
                    .font(.headline)
            }
        }
        .frame(maxWidth: .infinity)
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
    
    private var qrCodeSection: some View {
        VStack(spacing: 16) {
            Text("Scan to Pay")
                .font(.headline)
            
            // Bitkit should integrate a QR code generator
            // For now, show placeholder
            RoundedRectangle(cornerRadius: 12)
                .fill(Color.white)
                .frame(width: 200, height: 200)
                .overlay(
                    Text("QR Code")
                        .foregroundColor(.gray)
                )
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
    
    private var connectionInfoSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Connection Info")
                .font(.headline)
            
            Text(viewModel.connectionInfo)
                .font(.caption)
                .foregroundColor(.secondary)
                .textSelection(.enabled)
            
            Button(action: {
                // Copy to clipboard
                UIPasteboard.general.string = viewModel.connectionInfo
            }) {
                Label("Copy", systemImage: "doc.on.doc")
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
    
    private var incomingRequestsSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Incoming Requests")
                .font(.headline)
            
            ForEach(viewModel.incomingRequests, id: \.requestId) { request in
                IncomingRequestRow(
                    request: request,
                    onAccept: { viewModel.acceptRequest(request) },
                    onDecline: { viewModel.declineRequest(request) }
                )
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
    
    private var controlButtonsSection: some View {
        VStack(spacing: 12) {
            if viewModel.isListening {
                Button(action: viewModel.stopListening) {
                    Label("Stop Listening", systemImage: "stop.circle.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
            } else {
                Button(action: viewModel.startListening) {
                    Label("Start Listening", systemImage: "play.circle.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
            }
        }
    }
}

struct IncomingRequestRow: View {
    let request: PaymentRequest
    let onAccept: () -> Void
    let onDecline: () -> Void
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("From: \(request.fromPubkey.prefix(16))...")
                    .font(.subheadline)
                Spacer()
                Text("\(request.amountSats) sats")
                    .font(.headline)
            }
            
            if !request.description.isEmpty {
                Text(request.description)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            HStack {
                Button("Accept", action: onAccept)
                    .buttonStyle(.borderedProminent)
                Button("Decline", action: onDecline)
                    .buttonStyle(.bordered)
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
}
