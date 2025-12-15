//
//  PaymentView.swift
//  PaykitMobile
//
//  Send Payment UI component for Bitkit integration.
//  This is a template that Bitkit can adapt to their design system.
//

import SwiftUI
import PaykitMobile

/// Payment view model for Bitkit integration
public class BitkitPaymentViewModel: ObservableObject {
    @Published public var recipientUri: String = ""
    @Published public var amount: String = ""
    @Published public var currency: String = "SAT"
    @Published public var methodId: String = ""
    @Published public var availableMethods: [PaykitMobile.PaymentMethodInfo] = []
    @Published public var isProcessing = false
    @Published public var errorMessage: String?
    @Published public var showError = false
    @Published public var showSuccess = false
    @Published public var confirmedReceiptId: String?
    
    private let paykitClient: PaykitClient
    
    public init(paykitClient: PaykitClient) {
        self.paykitClient = paykitClient
        loadPaymentMethods()
    }
    
    func loadPaymentMethods() {
        Task {
            do {
                let methods = try paykitClient.listMethods()
                await MainActor.run {
                    self.availableMethods = methods
                    if let firstMethod = methods.first {
                        self.methodId = firstMethod.methodId
                    }
                }
            } catch {
                await MainActor.run {
                    self.errorMessage = "Failed to load payment methods: \(error.localizedDescription)"
                    self.showError = true
                }
            }
        }
    }
    
    func sendPayment() {
        guard !recipientUri.isEmpty, !amount.isEmpty, let amountSats = UInt64(amount) else {
            errorMessage = "Please fill in all fields"
            showError = true
            return
        }
        
        isProcessing = true
        
        Task {
            do {
                // Bitkit should implement recipient resolution and endpoint discovery
                // For now, this is a placeholder
                let endpoint = try await resolveEndpoint(for: recipientUri)
                
                let result = try paykitClient.executePayment(
                    methodId: methodId,
                    endpoint: endpoint,
                    amountSats: amountSats,
                    metadataJson: nil
                )
                
                await MainActor.run {
                    isProcessing = false
                    if result.success {
                        confirmedReceiptId = result.executionId
                        showSuccess = true
                        resetForm()
                    } else {
                        errorMessage = result.error ?? "Payment failed"
                        showError = true
                    }
                }
            } catch {
                await MainActor.run {
                    isProcessing = false
                    errorMessage = error.localizedDescription
                    showError = true
                }
            }
        }
    }
    
    func resetForm() {
        recipientUri = ""
        amount = ""
        currency = "SAT"
    }
    
    // Placeholder - Bitkit should implement endpoint resolution
    private func resolveEndpoint(for recipient: String) async throws -> String {
        // This should resolve the recipient's payment endpoint
        // from their directory or contact information
        throw NSError(
            domain: "BitkitPaymentViewModel",
            code: 1,
            userInfo: [NSLocalizedDescriptionKey: "Endpoint resolution not implemented. Bitkit should implement this."]
        )
    }
}

/// Send Payment view component
public struct BitkitPaymentView: View {
    @ObservedObject var viewModel: BitkitPaymentViewModel
    
    public init(viewModel: BitkitPaymentViewModel) {
        self.viewModel = viewModel
    }
    
    public var body: some View {
        Form {
            Section("Recipient") {
                TextField("pubky://... or contact name", text: $viewModel.recipientUri)
                    .disabled(viewModel.isProcessing)
            }
            
            Section("Amount") {
                HStack {
                    TextField("Amount", text: $viewModel.amount)
                        .keyboardType(.numberPad)
                        .disabled(viewModel.isProcessing)
                    
                    Picker("Currency", selection: $viewModel.currency) {
                        Text("SAT").tag("SAT")
                        Text("BTC").tag("BTC")
                        Text("USD").tag("USD")
                    }
                    .pickerStyle(.menu)
                    .disabled(viewModel.isProcessing)
                }
            }
            
            Section("Payment Method") {
                Picker("Method", selection: $viewModel.methodId) {
                    ForEach(viewModel.availableMethods, id: \.methodId) { method in
                        Text(method.methodId).tag(method.methodId)
                    }
                }
                .disabled(viewModel.isProcessing)
            }
            
            Section {
                Button(action: viewModel.sendPayment) {
                    HStack {
                        if viewModel.isProcessing {
                            ProgressView()
                        }
                        Text(viewModel.isProcessing ? "Processing..." : "Send Payment")
                    }
                    .frame(maxWidth: .infinity)
                }
                .disabled(viewModel.isProcessing || viewModel.recipientUri.isEmpty || viewModel.amount.isEmpty)
            }
        }
        .navigationTitle("Send Payment")
        .alert("Error", isPresented: $viewModel.showError) {
            Button("OK") { }
        } message: {
            Text(viewModel.errorMessage ?? "Unknown error")
        }
        .alert("Success", isPresented: $viewModel.showSuccess) {
            Button("OK") { }
        } message: {
            if let receiptId = viewModel.confirmedReceiptId {
                Text("Payment sent! Receipt ID: \(receiptId)")
            } else {
                Text("Payment sent successfully!")
            }
        }
    }
}
