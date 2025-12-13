//
//  PaymentRequestsView.swift
//  PaykitDemo
//
//  View for creating and managing payment requests
//

import SwiftUI
import Combine

struct PaymentRequestsView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var viewModel = PaymentRequestsViewModel()
    
    var body: some View {
        NavigationView {
            List {
                // Pending Requests Section
                Section {
                    if viewModel.pendingRequests.isEmpty {
                        Text("No pending requests")
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(viewModel.pendingRequests) { request in
                            PaymentRequestRow(request: request) { action in
                                viewModel.handleRequestAction(request: request, action: action)
                            }
                        }
                        .onDelete { indexSet in
                            for index in indexSet {
                                viewModel.deleteRequest(id: viewModel.pendingRequests[index].id)
                            }
                        }
                    }
                } header: {
                    Text("Pending Requests")
                }
                
                // Create Request Section
                Section {
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Create Payment Request")
                            .font(.headline)
                        
                        TextField("Recipient Public Key", text: $viewModel.recipientPubkey)
                            .textFieldStyle(.roundedBorder)
                            .autocapitalization(.none)
                        
                        HStack {
                            Text("Amount:")
                            Spacer()
                            TextField("sats", value: $viewModel.requestAmount, format: .number)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 100)
                                .multilineTextAlignment(.trailing)
                        }
                        
                        Picker("Method", selection: $viewModel.requestMethod) {
                            Text("Lightning").tag("lightning")
                            Text("On-Chain").tag("onchain")
                        }
                        .pickerStyle(.segmented)
                        
                        TextField("Description", text: $viewModel.requestDescription)
                            .textFieldStyle(.roundedBorder)
                        
                        Toggle("Set Expiry", isOn: $viewModel.hasExpiry)
                        
                        if viewModel.hasExpiry {
                            Stepper("Expires in \(viewModel.expiryHours) hours", 
                                   value: $viewModel.expiryHours, 
                                   in: 1...168)
                        }
                        
                        Button("Create Request") {
                            // Get public key from KeyManager if available
                            let myPubkey = "pk1demo..." // TODO: Get from KeyManager
                            viewModel.createRequest(client: appState.paykitClient, myPublicKey: myPubkey)
                        }
                        .buttonStyle(.borderedProminent)
                        .frame(maxWidth: .infinity)
                        .disabled(viewModel.recipientPubkey.isEmpty)
                    }
                    .padding(.vertical, 8)
                } header: {
                    Text("New Request")
                }
                
                // Request History Section
                Section {
                    if viewModel.requestHistory.isEmpty {
                        Text("No request history")
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(viewModel.requestHistory) { request in
                            RequestHistoryRow(request: request)
                        }
                        .onDelete { indexSet in
                            for index in indexSet {
                                viewModel.deleteRequest(id: viewModel.requestHistory[index].id)
                            }
                        }
                    }
                } header: {
                    Text("History")
                }
            }
            .navigationTitle("Payment Requests")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Refresh") {
                        viewModel.refreshRequests()
                    }
                }
            }
            .alert("Error", isPresented: $viewModel.showError) {
                Button("OK") { }
            } message: {
                Text(viewModel.errorMessage ?? "Unknown error")
            }
            .onAppear {
                viewModel.refreshRequests()
            }
        }
    }
}

struct PaymentRequestRow: View {
    let request: StoredPaymentRequest
    let onAction: (RequestAction) -> Void
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading) {
                    Text(request.counterpartyName)
                        .font(.headline)
                    Text(request.description)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                VStack(alignment: .trailing) {
                    Text("\(request.amountSats) sats")
                        .font(.subheadline)
                        .fontWeight(.medium)
                    HStack(spacing: 4) {
                        Text(request.methodId)
                        Image(systemName: request.direction == .incoming ? "arrow.down" : "arrow.up")
                    }
                    .font(.caption)
                    .foregroundColor(.secondary)
                }
            }
            
            if let expiresAt = request.expiresAt {
                HStack {
                    Image(systemName: "clock")
                        .font(.caption)
                    Text("Expires \(expiresAt, style: .relative)")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }
            
            HStack {
                Button("Accept") {
                    onAction(.accept)
                }
                .buttonStyle(.borderedProminent)
                .tint(.green)
                
                Button("Decline") {
                    onAction(.decline)
                }
                .buttonStyle(.bordered)
                .tint(.red)
                
                Spacer()
            }
        }
        .padding(.vertical, 4)
    }
}

struct RequestHistoryRow: View {
    let request: StoredPaymentRequest
    
    var body: some View {
        HStack {
            VStack(alignment: .leading) {
                HStack(spacing: 4) {
                    Image(systemName: request.direction == .incoming ? "arrow.down.circle" : "arrow.up.circle")
                        .foregroundColor(request.direction == .incoming ? .green : .blue)
                    Text(request.counterpartyName)
                        .font(.subheadline)
                }
                Text(request.description)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            VStack(alignment: .trailing) {
                Text("\(request.amountSats) sats")
                    .font(.caption)
                Text(request.status.rawValue)
                    .font(.caption)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(statusColor(request.status).opacity(0.2))
                    .foregroundColor(statusColor(request.status))
                    .cornerRadius(4)
            }
        }
    }
    
    private func statusColor(_ status: PaymentRequestStatus) -> Color {
        switch status {
        case .pending: return .yellow
        case .accepted: return .green
        case .declined: return .red
        case .expired: return .gray
        case .paid: return .blue
        }
    }
}

enum RequestAction {
    case accept
    case decline
}

// MARK: - View Model

class PaymentRequestsViewModel: ObservableObject {
    @Published var pendingRequests: [StoredPaymentRequest] = []
    @Published var requestHistory: [StoredPaymentRequest] = []
    @Published var errorMessage: String?
    @Published var showError = false
    
    // Create request form
    @Published var recipientPubkey = ""
    @Published var requestAmount: Int64 = 1000
    @Published var requestMethod = "lightning"
    @Published var requestDescription = ""
    @Published var hasExpiry = true
    @Published var expiryHours = 24
    
    private let storage = PaymentRequestStorage()
    
    init() {
        loadRequests()
    }
    
    func loadRequests() {
        // Check for expired requests first
        try? storage.checkExpirations()
        
        // Load from persistent storage
        let allRequests = storage.listRequests()
        pendingRequests = allRequests.filter { $0.status == .pending }
        requestHistory = allRequests.filter { $0.status != .pending }
    }
    
    func createRequest(client: PaykitClientWrapper, myPublicKey: String) {
        let expirySeconds = hasExpiry ? UInt64(expiryHours * 3600) : nil
        
        guard let ffiRequest = client.createPaymentRequest(
            fromPubkey: myPublicKey,
            toPubkey: recipientPubkey,
            amountSats: requestAmount,
            currency: "SAT",
            methodId: requestMethod,
            description: requestDescription,
            expiresInSecs: expirySeconds
        ) else {
            showErrorMessage("Failed to create payment request")
            return
        }
        
        // Convert FFI request to storable format
        let newRequest = StoredPaymentRequest.fromFFI(ffiRequest, direction: .outgoing)
        
        do {
            try storage.addRequest(newRequest)
            loadRequests()  // Refresh the lists
            
            // Reset form
            recipientPubkey = ""
            requestDescription = ""
        } catch {
            showErrorMessage("Failed to save payment request: \(error.localizedDescription)")
        }
    }
    
    func handleRequestAction(request: StoredPaymentRequest, action: RequestAction) {
        let newStatus: PaymentRequestStatus
        switch action {
        case .accept:
            newStatus = .accepted
        case .decline:
            newStatus = .declined
        }
        
        do {
            try storage.updateStatus(id: request.id, status: newStatus)
            loadRequests()  // Refresh the lists
        } catch {
            showErrorMessage("Failed to update request: \(error.localizedDescription)")
        }
    }
    
    func deleteRequest(id: String) {
        do {
            try storage.deleteRequest(id: id)
            loadRequests()
        } catch {
            showErrorMessage("Failed to delete request: \(error.localizedDescription)")
        }
    }
    
    func refreshRequests() {
        loadRequests()
    }
    
    func clearAllRequests() {
        do {
            try storage.clearAll()
            loadRequests()
        } catch {
            showErrorMessage("Failed to clear requests: \(error.localizedDescription)")
        }
    }
    
    private func showErrorMessage(_ message: String) {
        errorMessage = message
        showError = true
    }
}

#Preview {
    PaymentRequestsView()
        .environmentObject(AppState())
}



