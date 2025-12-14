//
//  PaymentView.swift
//  PaykitDemo
//
//  Interactive payment view using Noise protocol for secure payment negotiation.
//

import SwiftUI
import Network
import Combine

// MARK: - Payment Flow Status

/// Status of the payment flow (different from PaymentReceiptStatus)
enum PaymentFlowStatus: Equatable {
    case idle
    case resolvingRecipient
    case discoveringEndpoint
    case connecting
    case handshaking
    case sendingRequest
    case awaitingConfirmation
    case completed
    case failed(String)
    case cancelled
    
    /// Progress as a percentage (0.0 - 1.0)
    var progress: Double {
        switch self {
        case .idle: return 0.0
        case .resolvingRecipient: return 0.15
        case .discoveringEndpoint: return 0.30
        case .connecting: return 0.45
        case .handshaking: return 0.60
        case .sendingRequest: return 0.75
        case .awaitingConfirmation: return 0.90
        case .completed: return 1.0
        case .failed, .cancelled: return 0.0
        }
    }
    
    /// Human-readable description
    var description: String {
        switch self {
        case .idle: return "Ready"
        case .resolvingRecipient: return "Resolving recipient..."
        case .discoveringEndpoint: return "Discovering endpoint..."
        case .connecting: return "Connecting..."
        case .handshaking: return "Handshaking..."
        case .sendingRequest: return "Sending request..."
        case .awaitingConfirmation: return "Awaiting confirmation..."
        case .completed: return "Payment completed!"
        case .failed(let msg): return "Failed: \(msg)"
        case .cancelled: return "Cancelled"
        }
    }
}

// MARK: - Payment View

struct PaymentView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var viewModel = PaymentViewModel()
    var initialRecipient: String? = nil
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 20) {
                    // Payment Form
                    paymentForm
                    
                    // Status Card
                    if viewModel.paymentStatus != .idle {
                        statusCard
                    }
                    
                    // Action Buttons
                    actionButtons
                    
                    Spacer()
                }
                .padding()
            }
            .navigationTitle("Send Payment")
            .onAppear {
                if let recipient = initialRecipient {
                    viewModel.recipientUri = recipient
                }
            }
            .alert("Payment Error", isPresented: $viewModel.showError) {
                Button("OK") { viewModel.showError = false }
            } message: {
                Text(viewModel.errorMessage)
            }
            .alert("Payment Successful!", isPresented: $viewModel.showSuccess) {
                Button("OK") { 
                    viewModel.showSuccess = false
                    viewModel.resetForm()
                }
            } message: {
                Text("Receipt ID: \(viewModel.confirmedReceiptId ?? "Unknown")")
            }
        }
    }
    
    private var paymentForm: some View {
        VStack(spacing: 16) {
            // Recipient
            VStack(alignment: .leading, spacing: 8) {
                Text("Recipient")
                    .font(.headline)
                
                TextField("pubky://... or contact name", text: $viewModel.recipientUri)
                    .textFieldStyle(.roundedBorder)
                    .autocapitalization(.none)
                    .disabled(viewModel.isProcessing)
            }
            
            // Amount
            VStack(alignment: .leading, spacing: 8) {
                Text("Amount")
                    .font(.headline)
                
                HStack {
                    TextField("Amount", text: $viewModel.amount)
                        .textFieldStyle(.roundedBorder)
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
            
            // Selection Strategy
            VStack(alignment: .leading, spacing: 8) {
                Text("Selection Strategy")
                    .font(.headline)
                
                Picker("Strategy", selection: $viewModel.selectionStrategy) {
                    Text("Balanced").tag(SelectionStrategy.balanced)
                    Text("Cost Optimized").tag(SelectionStrategy.costOptimized)
                    Text("Speed Optimized").tag(SelectionStrategy.speedOptimized)
                    Text("Privacy Optimized").tag(SelectionStrategy.privacyOptimized)
                }
                .pickerStyle(.segmented)
                .disabled(viewModel.isProcessing)
                .onChange(of: viewModel.selectionStrategy) { _ in
                    viewModel.applyStrategy()
                }
            }
            
            // Payment Method
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Payment Method")
                        .font(.headline)
                    Spacer()
                    if let methodHealth = viewModel.methodHealth {
                        HStack(spacing: 4) {
                            Image(systemName: methodHealth.icon)
                                .foregroundColor(methodHealth.color)
                            Text(methodHealth.status)
                                .font(.caption)
                                .foregroundColor(methodHealth.color)
                        }
                    }
                }
                
                Picker("Method", selection: $viewModel.paymentMethod) {
                    Text("Lightning").tag("lightning")
                    Text("On-Chain").tag("onchain")
                }
                .pickerStyle(.segmented)
                .disabled(viewModel.isProcessing)
            }
            
            // Description
            VStack(alignment: .leading, spacing: 8) {
                Text("Description (optional)")
                    .font(.headline)
                
                TextField("Payment for...", text: $viewModel.description)
                    .textFieldStyle(.roundedBorder)
                    .disabled(viewModel.isProcessing)
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
    
    private var statusCard: some View {
        VStack(spacing: 12) {
            HStack {
                Image(systemName: viewModel.statusIcon)
                    .foregroundColor(viewModel.statusColor)
                Text(viewModel.statusTitle)
                    .font(.headline)
                Spacer()
            }
            
            if viewModel.isProcessing {
                ProgressView(value: viewModel.progress)
                    .progressViewStyle(.linear)
            }
            
            Text(viewModel.statusMessage)
                .font(.caption)
                .foregroundColor(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding()
        .background(viewModel.statusColor.opacity(0.1))
        .cornerRadius(12)
    }
    
    private var actionButtons: some View {
        VStack(spacing: 12) {
            Button(action: {
                Task {
                    await viewModel.initiatePayment()
                }
            }) {
                HStack {
                    if viewModel.isProcessing {
                        ProgressView()
                            .progressViewStyle(CircularProgressViewStyle(tint: .white))
                    } else {
                        Image(systemName: "paperplane.fill")
                    }
                    Text(viewModel.isProcessing ? "Processing..." : "Send Payment")
                }
                .frame(maxWidth: .infinity)
                .padding()
                .background(viewModel.canSend ? Color.blue : Color.gray)
                .foregroundColor(.white)
                .cornerRadius(12)
            }
            .disabled(!viewModel.canSend)
            
            if viewModel.isProcessing {
                Button("Cancel") {
                    viewModel.cancelPayment()
                }
                .foregroundColor(.red)
            }
        }
    }
}

// MARK: - Payment Status

enum PaymentViewStatus {
    case idle
    case resolvingRecipient
    case discoveringEndpoint
    case connecting
    case handshaking
    case sendingRequest
    case awaitingConfirmation
    case completed
    case failed
    case cancelled
    
    var description: String {
        switch self {
        case .idle: return "Ready to send"
        case .resolvingRecipient: return "Resolving recipient..."
        case .discoveringEndpoint: return "Discovering payment endpoint..."
        case .connecting: return "Connecting to recipient..."
        case .handshaking: return "Performing secure handshake..."
        case .sendingRequest: return "Sending payment request..."
        case .awaitingConfirmation: return "Awaiting confirmation..."
        case .completed: return "Payment completed!"
        case .failed: return "Payment failed"
        case .cancelled: return "Payment cancelled"
        }
    }
    
    var progress: Double {
        switch self {
        case .idle: return 0
        case .resolvingRecipient: return 0.1
        case .discoveringEndpoint: return 0.2
        case .connecting: return 0.4
        case .handshaking: return 0.5
        case .sendingRequest: return 0.6
        case .awaitingConfirmation: return 0.8
        case .completed: return 1.0
        case .failed, .cancelled: return 0
        }
    }
}

// MARK: - Payment View Model

@MainActor
class PaymentViewModel: ObservableObject {
    // Form Fields
    @Published var recipientUri: String = ""
    @Published var amount: String = "1000"
    @Published var currency: String = "SAT"
    @Published var selectionStrategy: SelectionStrategy = .balanced
    @Published var paymentMethod: String = "lightning"
    @Published var description: String = ""
    
    // State
    @Published var paymentStatus: PaymentFlowStatus = .idle
    @Published var isProcessing: Bool = false
    @Published var showError: Bool = false
    @Published var showSuccess: Bool = false
    @Published var errorMessage: String = ""
    @Published var confirmedReceiptId: String?
    @Published var methodHealth: (icon: String, color: Color, status: String)?
    
    // Noise Manager
    private var noiseManager: FfiNoiseManager?
    private var currentSessionId: String?
    private var connection: NWConnection?
    private var cancellationRequested = false
    
    var canSend: Bool {
        !recipientUri.isEmpty && !amount.isEmpty && !isProcessing
    }
    
    var progress: Double {
        paymentStatus.progress
    }
    
    var statusTitle: String {
        paymentStatus.description
    }
    
    var statusMessage: String {
        switch paymentStatus {
        case .connecting:
            return "Establishing encrypted connection..."
        case .handshaking:
            return "Verifying identity with Noise protocol..."
        case .sendingRequest:
            return "Sending payment request over secure channel..."
        case .awaitingConfirmation:
            return "Waiting for recipient to confirm payment..."
        case .completed:
            return "Receipt received and stored"
        default:
            return ""
        }
    }
    
    var statusIcon: String {
        switch paymentStatus {
        case .completed: return "checkmark.circle.fill"
        case .failed, .cancelled: return "xmark.circle.fill"
        default: return "arrow.right.circle.fill"
        }
    }
    
    var statusColor: Color {
        switch paymentStatus {
        case .completed: return .green
        case .failed: return .red
        case .cancelled: return .orange
        default: return .blue
        }
    }
    
    // MARK: - Payment Flow
    
    func initiatePayment() async {
        guard canSend else { return }
        
        isProcessing = true
        cancellationRequested = false
        
        do {
            // Step 1: Resolve recipient
            paymentStatus = .resolvingRecipient
            let payeePubkey = try resolveRecipient(recipientUri)
            
            if cancellationRequested { throw PaymentError.cancelled }
            
            // Step 2: Discover endpoint
            paymentStatus = .discoveringEndpoint
            let noiseEndpoint = try await discoverNoiseEndpoint(payeePubkey: payeePubkey)
            
            if cancellationRequested { throw PaymentError.cancelled }
            
            // Step 3: Parse endpoint and connect
            paymentStatus = .connecting
            let (host, port, serverPk) = try parseNoiseEndpoint(noiseEndpoint)
            try await connectToRecipient(host: host, port: port)
            
            if cancellationRequested { throw PaymentError.cancelled }
            
            // Step 4: Perform Noise handshake
            paymentStatus = .handshaking
            try await performHandshake(serverPk: serverPk)
            
            if cancellationRequested { throw PaymentError.cancelled }
            
            // Step 5: Send payment request
            paymentStatus = .sendingRequest
            try await sendPaymentRequest(payeePubkey: payeePubkey)
            
            if cancellationRequested { throw PaymentError.cancelled }
            
            // Step 6: Await confirmation
            paymentStatus = .awaitingConfirmation
            let receipt = try await receiveConfirmation()
            
            // Step 7: Store receipt and complete
            confirmedReceiptId = receipt.id
            paymentStatus = .completed
            showSuccess = true
            
        } catch PaymentError.cancelled {
            paymentStatus = .cancelled
        } catch {
            paymentStatus = .failed(error.localizedDescription)
            errorMessage = error.localizedDescription
            showError = true
        }
        
        isProcessing = false
        cleanup()
    }
    
    func cancelPayment() {
        cancellationRequested = true
        cleanup()
        paymentStatus = .cancelled
        isProcessing = false
    }
    
    func resetForm() {
        recipientUri = ""
        amount = "1000"
        currency = "SAT"
        selectionStrategy = .balanced
        paymentMethod = "lightning"
        description = ""
        paymentStatus = .idle
        confirmedReceiptId = nil
        methodHealth = nil
    }
    
    // MARK: - Helper Methods
    
    private func resolveRecipient(_ uri: String) throws -> String {
        // Extract public key from pubky:// URI or contact name
        if uri.hasPrefix("pubky://") {
            return String(uri.dropFirst(8))
        }
        
        // Try to find in contacts
        let keyManager = KeyManager()
        let storage = ContactStorage(identityName: keyManager.currentIdentityName ?? "default")
        let contacts = storage.listContacts()
        
        if let contact = contacts.first(where: { $0.name.lowercased() == uri.lowercased() }) {
            return contact.publicKeyZ32
        }
        
        // Assume it's a raw public key
        return uri
    }
    
    private func discoverNoiseEndpoint(payeePubkey: String) async throws -> String {
        // Query the Pubky directory for payment methods
        let directoryService = DirectoryService.shared
        
        do {
            let methods = try await directoryService.discoverPaymentMethods(recipientPubkey: payeePubkey)
            
            // Look for Noise endpoint
            if let noiseEndpoint = try await directoryService.discoverNoiseEndpoint(recipientPubkey: payeePubkey) {
                return "noise://\(noiseEndpoint.host):\(noiseEndpoint.port)@\(noiseEndpoint.serverNoisePubkey)"
            }
            
            // If no Noise endpoint, check for other methods
            if methods.isEmpty {
                throw PaymentError.noEndpoint
            }
            
            // Return first available method (could be enhanced to select best method)
            throw PaymentError.noEndpoint // Still need Noise endpoint for this flow
        } catch {
            throw PaymentError.noEndpoint
        }
    }
    
    private func parseNoiseEndpoint(_ endpoint: String) throws -> (String, UInt16, Data) {
        // Format: noise://host:port@pubkey_hex
        guard endpoint.hasPrefix("noise://") else {
            throw PaymentError.invalidEndpoint
        }
        
        let withoutPrefix = String(endpoint.dropFirst(8))
        let parts = withoutPrefix.split(separator: "@")
        guard parts.count == 2 else {
            throw PaymentError.invalidEndpoint
        }
        
        let hostPort = String(parts[0])
        let pkHex = String(parts[1])
        
        let colonIndex = hostPort.lastIndex(of: ":")!
        let host = String(hostPort[..<colonIndex])
        let portStr = String(hostPort[hostPort.index(after: colonIndex)...])
        
        guard let port = UInt16(portStr) else {
            throw PaymentError.invalidEndpoint
        }
        
        guard pkHex.count == 64,
              let pkData = Data(hexString: pkHex) else {
            throw PaymentError.invalidEndpoint
        }
        
        return (host, port, pkData)
    }
    
    private func connectToRecipient(host: String, port: UInt16) async throws {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            let endpoint = NWEndpoint.hostPort(host: NWEndpoint.Host(host), port: NWEndpoint.Port(rawValue: port)!)
            connection = NWConnection(to: endpoint, using: .tcp)
            
            connection?.stateUpdateHandler = { [weak self] state in
                switch state {
                case .ready:
                    continuation.resume()
                case .failed(let error):
                    continuation.resume(throwing: error)
                case .cancelled:
                    continuation.resume(throwing: PaymentError.connectionCancelled)
                default:
                    break
                }
            }
            
            connection?.start(queue: .main)
        }
    }
    
    private func performHandshake(serverPk: Data) async throws {
        // Initialize Noise manager
        let keyManager = KeyManager()
        guard let secretKey = keyManager.getSecretKeyData() else {
            throw PaymentError.noIdentity
        }
        
        let deviceId = (UIDevice.current.identifierForVendor?.uuidString ?? "unknown").data(using: .utf8)!
        let config = FfiMobileConfig(
            autoReconnect: false,
            maxReconnectAttempts: 0,
            reconnectDelayMs: 0,
            batterySaver: false,
            chunkSize: 32768
        )
        
        noiseManager = try FfiNoiseManager.newClient(
            config: config,
            clientSeed: secretKey,
            clientKid: "paykit-ios",
            deviceId: deviceId
        )
        
        // Step 1: Initiate connection
        let result = try noiseManager!.initiateConnection(serverPk: serverPk, hint: nil)
        
        // Step 2: Send first message
        try await sendData(result.firstMessage)
        
        // Step 3: Receive server response
        let response = try await receiveData()
        
        // Step 4: Complete handshake
        currentSessionId = try noiseManager!.completeConnection(
            sessionId: result.sessionId,
            serverResponse: response
        )
    }
    
    private func sendPaymentRequest(payeePubkey: String) async throws {
        guard let sessionId = currentSessionId, let manager = noiseManager else {
            throw PaymentError.notConnected
        }
        
        // Create payment request using PaykitMessageBuilder
        let message = try createReceiptRequestMessage(
            receiptId: UUID().uuidString,
            payerPubkey: KeyManager().publicKeyZ32 ?? "",
            payeePubkey: payeePubkey,
            methodId: paymentMethod,
            amount: amount,
            currency: currency
        )
        let requestJson = message.payloadJson
        
        // Encrypt and send
        let plaintext = requestJson.data(using: .utf8)!
        let ciphertext = try manager.encrypt(sessionId: sessionId, plaintext: plaintext)
        
        // Send with length prefix
        var outboundData = Data()
        var length = UInt32(ciphertext.count).bigEndian
        outboundData.append(Data(bytes: &length, count: 4))
        outboundData.append(ciphertext)
        
        try await sendData(outboundData)
    }
    
    private func receiveConfirmation() async throws -> PaymentReceipt {
        guard let sessionId = currentSessionId, let manager = noiseManager else {
            throw PaymentError.notConnected
        }
        
        // Receive length prefix
        let lengthData = try await receiveData(length: 4)
        let length = UInt32(bigEndian: lengthData.withUnsafeBytes { $0.load(as: UInt32.self) })
        
        // Receive ciphertext
        let ciphertext = try await receiveData(length: Int(length))
        
        // Decrypt
        let plaintext = try manager.decrypt(sessionId: sessionId, ciphertext: ciphertext)
        let json = String(data: plaintext, encoding: .utf8)!
        
        // Parse response
        let builder = createMessageBuilder()
        let parsed = try builder.parseMessage(messageJson: json)
        
        guard case .confirmReceipt(let receiptRequest) = parsed else {
            throw PaymentError.invalidResponse
        }
        
        // Store receipt
        let keyManager = KeyManager()
        let storage = ReceiptStorage(identityName: keyManager.currentIdentityName ?? "default")
        var receipt = PaymentReceipt(
            direction: .sent,
            counterpartyKey: recipientUri,
            counterpartyName: nil,
            amountSats: UInt64(amount) ?? 0,
            paymentMethod: paymentMethod,
            memo: description.isEmpty ? nil : description
        )
        receipt.status = .completed
        receipt.completedAt = Date()
        try? storage.addPaymentReceipt(receipt)
        
        return receipt
    }
    
    private func sendData(_ data: Data) async throws {
        guard let connection = connection else {
            throw PaymentError.notConnected
        }
        
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            connection.send(content: data, completion: .contentProcessed { error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            })
        }
    }
    
    private func receiveData(length: Int? = nil) async throws -> Data {
        guard let connection = connection else {
            throw PaymentError.notConnected
        }
        
        return try await withCheckedThrowingContinuation { continuation in
            connection.receive(minimumIncompleteLength: length ?? 1, maximumLength: length ?? 65536) { data, _, _, error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else if let data = data {
                    continuation.resume(returning: data)
                } else {
                    continuation.resume(throwing: PaymentError.noData)
                }
            }
        }
    }
    
    private func cleanup() {
        connection?.cancel()
        connection = nil
        if let sessionId = currentSessionId {
            noiseManager?.removeSession(sessionId: sessionId)
        }
        noiseManager = nil
        currentSessionId = nil
    }
}

// MARK: - Payment Errors

enum PaymentError: LocalizedError {
    case noEndpoint
    case invalidEndpoint
    case connectionFailed
    case connectionCancelled
    case handshakeFailed
    case noIdentity
    case notConnected
    case invalidResponse
    case noData
    case cancelled
    
    var errorDescription: String? {
        switch self {
        case .noEndpoint:
            return "Recipient has no noise:// endpoint published. They must be running a payment receiver."
        case .invalidEndpoint:
            return "Invalid noise endpoint format"
        case .connectionFailed:
            return "Failed to connect to recipient"
        case .connectionCancelled:
            return "Connection was cancelled"
        case .handshakeFailed:
            return "Secure handshake failed"
        case .noIdentity:
            return "No identity configured. Please set up your identity first."
        case .notConnected:
            return "Not connected to recipient"
        case .invalidResponse:
            return "Received invalid response from recipient"
        case .noData:
            return "No data received from recipient"
        case .cancelled:
            return "Payment was cancelled"
        }
    }
}

// MARK: - Extensions

extension Data {
    init?(hexString: String) {
        let len = hexString.count / 2
        var data = Data(capacity: len)
        var index = hexString.startIndex
        
        for _ in 0..<len {
            let nextIndex = hexString.index(index, offsetBy: 2)
            guard let byte = UInt8(hexString[index..<nextIndex], radix: 16) else {
                return nil
            }
            data.append(byte)
            index = nextIndex
        }
        
        self = data
    }
}

// MARK: - Preview

#Preview {
    PaymentView()
        .environmentObject(AppState())
}

