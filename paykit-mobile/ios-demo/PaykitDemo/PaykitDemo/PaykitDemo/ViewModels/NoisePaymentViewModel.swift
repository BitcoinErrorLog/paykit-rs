// NoisePaymentViewModel.swift
// ViewModel for Noise Protocol Payments
//
// This ViewModel coordinates the payment flow using:
// - NoisePaymentService for encrypted communication
// - PubkyRingIntegration for key management
// - Local storage for receipts

import Foundation
import SwiftUI
import Combine

// MARK: - Payment Flow State

public enum NoisePaymentState: Equatable {
    case idle
    case resolvingRecipient
    case derivingKeys
    case discoveringEndpoint
    case connecting
    case handshaking
    case sendingRequest
    case awaitingConfirmation
    case completed(receiptId: String)
    case failed(message: String)
    case cancelled
    
    public var isProcessing: Bool {
        switch self {
        case .idle, .completed, .failed, .cancelled:
            return false
        default:
            return true
        }
    }
    
    public var progress: Double {
        switch self {
        case .idle: return 0
        case .resolvingRecipient: return 0.1
        case .derivingKeys: return 0.15
        case .discoveringEndpoint: return 0.25
        case .connecting: return 0.4
        case .handshaking: return 0.55
        case .sendingRequest: return 0.7
        case .awaitingConfirmation: return 0.85
        case .completed: return 1.0
        case .failed, .cancelled: return 0
        }
    }
    
    public var description: String {
        switch self {
        case .idle:
            return "Ready to send payment"
        case .resolvingRecipient:
            return "Resolving recipient..."
        case .derivingKeys:
            return "Preparing encryption keys..."
        case .discoveringEndpoint:
            return "Discovering payment endpoint..."
        case .connecting:
            return "Connecting to recipient..."
        case .handshaking:
            return "Establishing secure channel..."
        case .sendingRequest:
            return "Sending payment request..."
        case .awaitingConfirmation:
            return "Awaiting confirmation..."
        case .completed(let receiptId):
            return "Payment completed! Receipt: \(receiptId)"
        case .failed(let message):
            return "Failed: \(message)"
        case .cancelled:
            return "Payment cancelled"
        }
    }
    
    public var icon: String {
        switch self {
        case .completed: return "checkmark.circle.fill"
        case .failed: return "xmark.circle.fill"
        case .cancelled: return "minus.circle.fill"
        default: return "arrow.right.circle.fill"
        }
    }
    
    public var color: Color {
        switch self {
        case .completed: return .green
        case .failed: return .red
        case .cancelled: return .orange
        default: return .blue
        }
    }
}

// MARK: - View Model

@MainActor
public final class NoisePaymentViewModel: ObservableObject {
    
    // MARK: - Form Fields
    
    @Published public var recipientInput: String = ""
    @Published public var amount: String = "1000"
    @Published public var currency: String = "SAT"
    @Published public var paymentMethod: String = "lightning"
    @Published public var memo: String = ""
    
    // MARK: - State
    
    @Published public private(set) var state: NoisePaymentState = .idle
    @Published public var showErrorAlert = false
    @Published public var showSuccessAlert = false
    
    // MARK: - Private Properties
    
    private let paymentService = NoisePaymentService.shared
    private let keyManager = KeyManager()
    private var cancellationRequested = false
    private var cancellables = Set<AnyCancellable>()
    
    // MARK: - Computed Properties
    
    public var canSend: Bool {
        !recipientInput.isEmpty && !amount.isEmpty && !state.isProcessing
    }
    
    public var progress: Double {
        state.progress
    }
    
    public var statusMessage: String {
        state.description
    }
    
    public var isProcessing: Bool {
        state.isProcessing
    }
    
    public var completedReceiptId: String? {
        if case .completed(let receiptId) = state {
            return receiptId
        }
        return nil
    }
    
    public var errorMessage: String? {
        if case .failed(let message) = state {
            return message
        }
        return nil
    }
    
    // MARK: - Initialization
    
    public init() {
        setupBindings()
    }
    
    private func setupBindings() {
        // Observe payment service state changes
        paymentService.$isConnected
            .receive(on: DispatchQueue.main)
            .sink { [weak self] connected in
                // Could update UI based on connection state
            }
            .store(in: &cancellables)
    }
    
    // MARK: - Payment Flow
    
    /// Initiate a payment
    public func sendPayment() async {
        guard canSend else { return }
        
        cancellationRequested = false
        
        do {
            // Step 1: Resolve recipient
            state = .resolvingRecipient
            let payeePubkey = try resolveRecipient(recipientInput)
            
            guard !cancellationRequested else { throw NoisePaymentError.cancelled }
            
            // Step 2: Derive/get encryption keys
            state = .derivingKeys
            _ = try await paymentService.getOrDeriveKeys()
            
            guard !cancellationRequested else { throw NoisePaymentError.cancelled }
            
            // Step 3: Discover endpoint
            state = .discoveringEndpoint
            let endpoint = try await discoverEndpoint(for: payeePubkey)
            
            guard !cancellationRequested else { throw NoisePaymentError.cancelled }
            
            // Step 4: Connect
            state = .connecting
            try await paymentService.connect(to: endpoint)
            
            guard !cancellationRequested else { throw NoisePaymentError.cancelled }
            
            // Step 5: Handshake (done in connect)
            state = .handshaking
            // Already completed in connect()
            
            guard !cancellationRequested else { throw NoisePaymentError.cancelled }
            
            // Step 6: Send request
            state = .sendingRequest
            let request = createPaymentRequest(payeePubkey: payeePubkey)
            
            state = .awaitingConfirmation
            let response = try await paymentService.sendPaymentRequest(request)
            
            // Step 7: Handle response
            if response.success, let receiptId = response.receiptId {
                // Save receipt
                saveReceipt(request: request, confirmedAt: response.confirmedAt ?? Date())
                
                state = .completed(receiptId: receiptId)
                showSuccessAlert = true
            } else {
                let errorMsg = response.errorMessage ?? "Payment rejected"
                state = .failed(message: errorMsg)
                showErrorAlert = true
            }
            
        } catch NoisePaymentError.cancelled {
            state = .cancelled
        } catch {
            state = .failed(message: error.localizedDescription)
            showErrorAlert = true
        }
        
        // Cleanup
        paymentService.disconnect()
    }
    
    /// Cancel the current payment
    public func cancel() {
        cancellationRequested = true
        paymentService.disconnect()
        state = .cancelled
    }
    
    /// Reset form to initial state
    public func reset() {
        recipientInput = ""
        amount = "1000"
        currency = "SAT"
        paymentMethod = "lightning"
        memo = ""
        state = .idle
    }
    
    // MARK: - Helper Methods
    
    /// Resolve recipient input to public key
    private func resolveRecipient(_ input: String) throws -> String {
        // Handle pubky:// URI
        if input.hasPrefix("pubky://") {
            return String(input.dropFirst(8))
        }
        
        // Try to find in contacts
        let identityName = keyManager.currentIdentityName ?? "default"
        let storage = ContactStorage(identityName: identityName)
        let contacts = storage.listContacts()
        
        if let contact = contacts.first(where: { 
            $0.name.lowercased() == input.lowercased() ||
            $0.publicKey.lowercased() == input.lowercased()
        }) {
            return contact.publicKey
        }
        
        // Assume it's a raw public key
        return input
    }
    
    /// Discover endpoint for recipient
    private func discoverEndpoint(for pubkey: String) async throws -> NoiseEndpointInfo {
        // Try the payment service first
        do {
            return try await paymentService.discoverEndpoint(recipientPubkey: pubkey)
        } catch NoisePaymentError.endpointNotFound {
            // Check for manual override in test mode
            if let testEndpoint = getTestEndpoint() {
                return testEndpoint
            }
            throw NoisePaymentError.endpointNotFound
        }
    }
    
    /// Get test endpoint from environment (for demo)
    private func getTestEndpoint() -> NoiseEndpointInfo? {
        guard let host = ProcessInfo.processInfo.environment["PAYKIT_TEST_HOST"],
              let portStr = ProcessInfo.processInfo.environment["PAYKIT_TEST_PORT"],
              let port = UInt16(portStr),
              let pubkey = ProcessInfo.processInfo.environment["PAYKIT_TEST_PUBKEY"] else {
            return nil
        }
        
        return NoiseEndpointInfo(
            host: host,
            port: port,
            serverPubkeyHex: pubkey,
            metadata: nil
        )
    }
    
    /// Create payment request
    private func createPaymentRequest(payeePubkey: String) -> NoisePaymentRequest {
        let payerPubkey = keyManager.publicKeyZ32 ?? ""
        
        return NoisePaymentRequest(
            payerPubkey: payerPubkey,
            payeePubkey: payeePubkey,
            methodId: paymentMethod,
            amount: amount,
            currency: currency,
            description: memo.isEmpty ? nil : memo
        )
    }
    
    /// Save receipt to local storage
    private func saveReceipt(request: NoisePaymentRequest, confirmedAt: Date) {
        let identityName = keyManager.currentIdentityName ?? "default"
        let storage = ReceiptStorage(identityName: identityName)
        
        let receipt = StoredReceipt(
            id: request.receiptId,
            payer: request.payerPubkey,
            payee: request.payeePubkey,
            amount: Int64(request.amount ?? "0") ?? 0,
            currency: request.currency ?? "SAT",
            method: request.methodId,
            timestamp: confirmedAt,
            status: .completed,
            notes: request.description
        )
        
        storage.saveReceipt(receipt)
    }
}

// MARK: - Receiving Payments View Model

@MainActor
public final class NoiseReceiveViewModel: ObservableObject {
    
    // MARK: - State
    
    @Published public private(set) var isListening = false
    @Published public private(set) var listeningPort: UInt16?
    @Published public private(set) var noisePubkeyHex: String?
    @Published public private(set) var pendingRequests: [PendingPaymentRequest] = []
    @Published public private(set) var recentReceipts: [StoredReceipt] = []
    
    // MARK: - Models
    
    public struct PendingPaymentRequest: Identifiable {
        public let id: String
        public let payerPubkey: String
        public let amount: String?
        public let currency: String?
        public let methodId: String
        public let receivedAt: Date
    }
    
    // MARK: - Private Properties
    
    private let paymentService = NoisePaymentService.shared
    private let keyManager = KeyManager()
    
    // MARK: - Server Control
    
    /// Start listening for payments
    public func startListening(port: UInt16 = 0) async {
        do {
            let status = try await paymentService.startServer(port: port)
            
            isListening = status.isRunning
            listeningPort = status.port
            noisePubkeyHex = status.noisePubkeyHex
            
        } catch {
            print("Failed to start server: \(error)")
        }
    }
    
    /// Stop listening
    public func stopListening() {
        paymentService.stopServer()
        isListening = false
        listeningPort = nil
    }
    
    /// Get connection info for sharing
    public func getConnectionInfo() -> String? {
        guard let port = listeningPort, let pubkey = noisePubkeyHex else {
            return nil
        }
        
        // Get local IP (simplified - in production would need better network detection)
        let host = getLocalIPAddress() ?? "localhost"
        
        return "\(host):\(port):\(pubkey)"
    }
    
    /// Accept a pending payment request
    public func acceptRequest(_ request: PendingPaymentRequest) async {
        // Would send confirmation back to payer
        // For now, just remove from pending
        pendingRequests.removeAll { $0.id == request.id }
    }
    
    /// Decline a pending payment request
    public func declineRequest(_ request: PendingPaymentRequest) async {
        // Would send rejection back to payer
        pendingRequests.removeAll { $0.id == request.id }
    }
    
    /// Load recent receipts
    public func loadRecentReceipts() {
        let identityName = keyManager.currentIdentityName ?? "default"
        let storage = ReceiptStorage(identityName: identityName)
        recentReceipts = storage.listReceipts()
            .sorted { $0.timestamp > $1.timestamp }
            .prefix(10)
            .map { $0 }
    }
    
    // MARK: - Helpers
    
    private func getLocalIPAddress() -> String? {
        var address: String?
        var ifaddr: UnsafeMutablePointer<ifaddrs>?
        
        guard getifaddrs(&ifaddr) == 0, let firstAddr = ifaddr else {
            return nil
        }
        
        defer { freeifaddrs(ifaddr) }
        
        for ptr in sequence(first: firstAddr, next: { $0.pointee.ifa_next }) {
            let interface = ptr.pointee
            let addrFamily = interface.ifa_addr.pointee.sa_family
            
            if addrFamily == UInt8(AF_INET) {
                let name = String(cString: interface.ifa_name)
                if name == "en0" {
                    var hostname = [CChar](repeating: 0, count: Int(NI_MAXHOST))
                    getnameinfo(
                        interface.ifa_addr,
                        socklen_t(interface.ifa_addr.pointee.sa_len),
                        &hostname,
                        socklen_t(hostname.count),
                        nil,
                        0,
                        NI_NUMERICHOST
                    )
                    address = String(cString: hostname)
                }
            }
        }
        
        return address
    }
}

