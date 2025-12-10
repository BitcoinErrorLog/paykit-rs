//
//  PaymentRequestsView.swift
//  PaykitDemo
//
//  View for creating and managing payment requests
//

import SwiftUI

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
                            viewModel.createRequest(client: appState.paykitClient)
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
                    ForEach(viewModel.requestHistory) { request in
                        RequestHistoryRow(request: request)
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
        }
    }
}

struct PaymentRequestRow: View {
    let request: PaymentRequestInfo
    let onAction: (RequestAction) -> Void
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading) {
                    Text(request.fromName)
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
                    Text(request.methodId)
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
    let request: PaymentRequestInfo
    
    var body: some View {
        HStack {
            VStack(alignment: .leading) {
                Text(request.fromName)
                    .font(.subheadline)
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
    
    private func statusColor(_ status: RequestStatusType) -> Color {
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
    @Published var pendingRequests: [PaymentRequestInfo] = []
    @Published var requestHistory: [PaymentRequestInfo] = []
    
    // Create request form
    @Published var recipientPubkey = ""
    @Published var requestAmount: Int64 = 1000
    @Published var requestMethod = "lightning"
    @Published var requestDescription = ""
    @Published var hasExpiry = true
    @Published var expiryHours = 24
    
    init() {
        loadSampleData()
    }
    
    func createRequest(client: PaykitClientWrapper) {
        let expirySeconds = hasExpiry ? UInt64(expiryHours * 3600) : nil
        
        if let request = client.createPaymentRequest(
            fromPubkey: "pk1me...",
            toPubkey: recipientPubkey,
            amountSats: requestAmount,
            currency: "SAT",
            methodId: requestMethod,
            description: requestDescription,
            expiresInSecs: expirySeconds
        ) {
            let newRequest = PaymentRequestInfo(
                id: request.requestId,
                fromPubkey: request.toPubkey,
                fromName: "You",
                amountSats: request.amountSats,
                methodId: request.methodId,
                description: request.description,
                createdAt: Date(),
                expiresAt: request.expiresAt.map { Date(timeIntervalSince1970: Double($0)) },
                status: .pending
            )
            pendingRequests.insert(newRequest, at: 0)
            
            // Reset form
            recipientPubkey = ""
            requestDescription = ""
        }
    }
    
    func handleRequestAction(request: PaymentRequestInfo, action: RequestAction) {
        guard let index = pendingRequests.firstIndex(where: { $0.id == request.id }) else {
            return
        }
        
        var updatedRequest = request
        switch action {
        case .accept:
            updatedRequest.status = .accepted
        case .decline:
            updatedRequest.status = .declined
        }
        
        pendingRequests.remove(at: index)
        requestHistory.insert(updatedRequest, at: 0)
    }
    
    func refreshRequests() {
        // In a real app, this would fetch from the network
    }
    
    private func loadSampleData() {
        pendingRequests = [
            PaymentRequestInfo(
                id: "1",
                fromPubkey: "pk1alice...",
                fromName: "Alice",
                amountSats: 5000,
                methodId: "lightning",
                description: "Split dinner bill",
                createdAt: Date().addingTimeInterval(-3600),
                expiresAt: Date().addingTimeInterval(86400),
                status: .pending
            ),
            PaymentRequestInfo(
                id: "2",
                fromPubkey: "pk1bob...",
                fromName: "Bob",
                amountSats: 2500,
                methodId: "lightning",
                description: "Concert tickets",
                createdAt: Date().addingTimeInterval(-7200),
                expiresAt: nil,
                status: .pending
            ),
        ]
        
        requestHistory = [
            PaymentRequestInfo(
                id: "3",
                fromPubkey: "pk1charlie...",
                fromName: "Charlie",
                amountSats: 1000,
                methodId: "lightning",
                description: "Coffee",
                createdAt: Date().addingTimeInterval(-86400),
                expiresAt: nil,
                status: .paid
            ),
        ]
    }
}

struct PaymentRequestInfo: Identifiable {
    let id: String
    let fromPubkey: String
    let fromName: String
    let amountSats: Int64
    let methodId: String
    let description: String
    let createdAt: Date
    let expiresAt: Date?
    var status: RequestStatusType
}

enum RequestStatusType: String {
    case pending = "Pending"
    case accepted = "Accepted"
    case declined = "Declined"
    case expired = "Expired"
    case paid = "Paid"
}

#Preview {
    PaymentRequestsView()
        .environmentObject(AppState())
}
