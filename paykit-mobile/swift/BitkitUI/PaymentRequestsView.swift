//
//  PaymentRequestsView.swift
//  PaykitMobile
//
//  Payment Requests UI component for Bitkit integration.
//  This is a template that Bitkit can adapt to their design system.
//

import SwiftUI
import PaykitMobile

/// Payment Requests view model for Bitkit integration
public class BitkitPaymentRequestsViewModel: ObservableObject {
    @Published public var pendingRequests: [PaymentRequest] = []
    @Published public var requestHistory: [PaymentRequest] = []
    @Published public var isLoading = false
    @Published public var errorMessage: String?
    @Published public var showError = false
    
    // Create request form
    @Published public var recipientPubkey = ""
    @Published public var requestAmount: Int64 = 1000
    @Published public var requestMethod = "lightning"
    @Published public var requestDescription = ""
    @Published public var hasExpiry = false
    @Published public var expiryHours: Int = 24
    
    private let paykitClient: PaykitClient
    private let paymentRequestStorage: PaymentRequestStorageProtocol
    
    public init(
        paykitClient: PaykitClient,
        paymentRequestStorage: PaymentRequestStorageProtocol
    ) {
        self.paykitClient = paykitClient
        self.paymentRequestStorage = paymentRequestStorage
    }
    
    func refreshRequests() {
        isLoading = true
        // Bitkit should load from storage
        pendingRequests = paymentRequestStorage.pendingRequests()
        requestHistory = paymentRequestStorage.requestHistory()
        isLoading = false
    }
    
    func createRequest(myPublicKey: String) {
        guard !recipientPubkey.isEmpty else {
            errorMessage = "Please enter recipient public key"
            showError = true
            return
        }
        
        isLoading = true
        
        Task {
            do {
                let expiresInSecs: UInt64? = hasExpiry ? UInt64(expiryHours * 3600) : nil
                
                let request = try paykitClient.createPaymentRequest(
                    fromPubkey: myPublicKey,
                    toPubkey: recipientPubkey,
                    amountSats: requestAmount,
                    currency: "SAT",
                    methodId: requestMethod,
                    description: requestDescription,
                    expiresInSecs: expiresInSecs
                )
                
                await MainActor.run {
                    pendingRequests.append(request)
                    isLoading = false
                    resetForm()
                }
            } catch {
                await MainActor.run {
                    isLoading = false
                    errorMessage = error.localizedDescription
                    showError = true
                }
            }
        }
    }
    
    func handleRequestAction(request: PaymentRequest, action: RequestAction) {
        switch action {
        case .accept:
            // Bitkit should implement payment request acceptance
            break
        case .decline:
            pendingRequests.removeAll { $0.requestId == request.requestId }
            // Bitkit should update storage
        }
    }
    
    func deleteRequest(id: String) {
        pendingRequests.removeAll { $0.requestId == id }
        requestHistory.removeAll { $0.requestId == id }
        // Bitkit should delete from storage
    }
    
    private func resetForm() {
        recipientPubkey = ""
        requestAmount = 1000
        requestMethod = "lightning"
        requestDescription = ""
        hasExpiry = false
        expiryHours = 24
    }
}

enum RequestAction {
    case accept
    case decline
}

/// Payment Requests view component
public struct BitkitPaymentRequestsView: View {
    @ObservedObject var viewModel: BitkitPaymentRequestsViewModel
    @State private var myPublicKey: String = "" // Bitkit should provide this
    
    public init(viewModel: BitkitPaymentRequestsViewModel) {
        self.viewModel = viewModel
    }
    
    public var body: some View {
        NavigationView {
            List {
                // Pending Requests Section
                Section {
                    if viewModel.pendingRequests.isEmpty {
                        Text("No pending requests")
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(viewModel.pendingRequests, id: \.requestId) { request in
                            PaymentRequestRow(request: request) { action in
                                viewModel.handleRequestAction(request: request, action: action)
                            }
                        }
                        .onDelete { indexSet in
                            for index in indexSet {
                                viewModel.deleteRequest(id: viewModel.pendingRequests[index].requestId)
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
                            viewModel.createRequest(myPublicKey: myPublicKey)
                        }
                        .buttonStyle(.borderedProminent)
                        .frame(maxWidth: .infinity)
                        .disabled(viewModel.recipientPubkey.isEmpty || viewModel.isLoading)
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
                        ForEach(viewModel.requestHistory, id: \.requestId) { request in
                            RequestHistoryRow(request: request)
                        }
                        .onDelete { indexSet in
                            for index in indexSet {
                                viewModel.deleteRequest(id: viewModel.requestHistory[index].requestId)
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
    let request: PaymentRequest
    let onAction: (RequestAction) -> Void
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading) {
                    Text(request.fromPubkey.prefix(16) + "...")
                        .font(.headline)
                    if !request.description.isEmpty {
                        Text(request.description)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
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
            
            HStack {
                Button("Accept") {
                    onAction(.accept)
                }
                .buttonStyle(.borderedProminent)
                
                Button("Decline") {
                    onAction(.decline)
                }
                .buttonStyle(.bordered)
            }
        }
        .padding(.vertical, 4)
    }
}

struct RequestHistoryRow: View {
    let request: PaymentRequest
    
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text(request.fromPubkey.prefix(16) + "...")
                    .font(.subheadline)
                if !request.description.isEmpty {
                    Text(request.description)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            Spacer()
            Text("\(request.amountSats) sats")
                .font(.subheadline)
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Payment Request Storage Protocol Extension

extension PaymentRequestStorageProtocol {
    func pendingRequests() -> [PaymentRequest] {
        // Bitkit should implement this
        return []
    }
    
    func requestHistory() -> [PaymentRequest] {
        // Bitkit should implement this
        return []
    }
}
