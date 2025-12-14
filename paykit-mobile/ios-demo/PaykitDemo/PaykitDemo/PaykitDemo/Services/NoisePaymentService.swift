// NoisePaymentService.swift
// Noise Payment Service for iOS
//
// This service coordinates Noise protocol payments, integrating:
// - Key management (PubkyRingIntegration, NoiseKeyCache)
// - Noise handshake (FfiNoiseManager from pubky-noise)
// - Message creation (PaykitMobile FFI)
// - Network transport (NWConnection)
//
// Key Architecture:
//   - Ed25519 (pkarr) keys are "cold" and managed by Pubky Ring
//   - X25519 (Noise) keys are "hot" and cached locally
//   - All payment messages are encrypted end-to-end

import Foundation
import Network
import UIKit

// MARK: - Noise Endpoint Info

/// Information about a recipient's Noise endpoint
public struct NoiseEndpointInfo: Codable {
    public let host: String
    public let port: UInt16
    public let serverPubkeyHex: String
    public let metadata: String?
    
    public var connectionAddress: String {
        "\(host):\(port)"
    }
    
    public var serverPubkeyData: Data? {
        Data(hexString: serverPubkeyHex)
    }
}

// MARK: - Payment Request

/// A payment request to send over Noise channel
public struct NoisePaymentRequest {
    public let receiptId: String
    public let payerPubkey: String
    public let payeePubkey: String
    public let methodId: String
    public let amount: String?
    public let currency: String?
    public let description: String?
    
    public init(
        payerPubkey: String,
        payeePubkey: String,
        methodId: String,
        amount: String? = nil,
        currency: String? = nil,
        description: String? = nil
    ) {
        self.receiptId = "rcpt_\(UUID().uuidString)"
        self.payerPubkey = payerPubkey
        self.payeePubkey = payeePubkey
        self.methodId = methodId
        self.amount = amount
        self.currency = currency
        self.description = description
    }
}

// MARK: - Payment Response

/// Response from a payment request
public struct NoisePaymentResponse {
    public let success: Bool
    public let receiptId: String?
    public let confirmedAt: Date?
    public let errorCode: String?
    public let errorMessage: String?
}

// MARK: - Service Errors

public enum NoisePaymentError: LocalizedError {
    case noIdentity
    case keyDerivationFailed(String)
    case endpointNotFound
    case invalidEndpoint(String)
    case connectionFailed(String)
    case handshakeFailed(String)
    case encryptionFailed(String)
    case decryptionFailed(String)
    case invalidResponse(String)
    case timeout
    case cancelled
    case serverError(code: String, message: String)
    
    public var errorDescription: String? {
        switch self {
        case .noIdentity:
            return "No identity configured. Please set up your identity first."
        case .keyDerivationFailed(let msg):
            return "Failed to derive encryption keys: \(msg)"
        case .endpointNotFound:
            return "Recipient has no Noise endpoint published."
        case .invalidEndpoint(let msg):
            return "Invalid endpoint format: \(msg)"
        case .connectionFailed(let msg):
            return "Connection failed: \(msg)"
        case .handshakeFailed(let msg):
            return "Secure handshake failed: \(msg)"
        case .encryptionFailed(let msg):
            return "Encryption failed: \(msg)"
        case .decryptionFailed(let msg):
            return "Decryption failed: \(msg)"
        case .invalidResponse(let msg):
            return "Invalid response: \(msg)"
        case .timeout:
            return "Connection timed out."
        case .cancelled:
            return "Operation was cancelled."
        case .serverError(let code, let message):
            return "Server error [\(code)]: \(message)"
        }
    }
}

// MARK: - Noise Payment Service

/// Service for managing Noise protocol payments
public final class NoisePaymentService: ObservableObject {
    
    // MARK: - Singleton
    
    public static let shared = NoisePaymentService()
    
    // MARK: - Published State
    
    @Published public private(set) var isConnected = false
    @Published public private(set) var currentSessionId: String?
    @Published public private(set) var connectedPeer: String?
    
    // MARK: - Private Properties
    
    private var noiseManager: FfiNoiseManager?
    private var connection: NWConnection?
    private var connectionQueue = DispatchQueue(label: "com.paykit.noise.connection")
    private var currentEpoch: UInt32 = 0
    
    // Server properties
    private var serverListener: NWListener?
    private var serverConnections: [UUID: ServerConnection] = [:]
    private var serverQueue = DispatchQueue(label: "com.paykit.noise.server")
    private var serverKeypair: X25519KeypairResult?
    private var serverNoiseManager: FfiNoiseManager?
    private var backgroundTask: UIBackgroundTaskIdentifier = .invalid
    
    // Interactive payment handling
    private var receiptStore: ReceiptStore?
    private var interactiveManager: PaykitInteractiveManagerFfi?
    private var receiptGenerator: ServerReceiptGenerator?
    
    private let keyManager = KeyManager()
    private let keyCache = NoiseKeyCache.shared
    private let pubkyRing = PubkyRingIntegration.shared
    
    // Configuration
    public var connectionTimeoutSecs: TimeInterval = 30.0
    
    // Callbacks for server events
    public var onPendingPaymentRequest: ((PendingPaymentRequest) -> Void)?
    public var onReceiptConfirmed: ((ReceiptRequest) -> Void)?
    
    // MARK: - Initialization
    
    private init() {
        setupInteractiveManager()
    }
    
    /// Initialize the interactive payment manager
    private func setupInteractiveManager() {
        receiptStore = createReceiptStore()
        interactiveManager = PaykitInteractiveManagerFfi(store: receiptStore!)
        
        // Set up receipt generator
        receiptGenerator = ServerReceiptGenerator()
        receiptGenerator?.onPaymentRequest { [weak self] request in
            // Notify about pending request
            let pendingRequest = PendingPaymentRequest(
                id: request.receiptId,
                payerPubkey: request.payer,
                amount: request.amount,
                currency: request.currency,
                methodId: request.methodId,
                receivedAt: Date(),
                connectionId: UUID() // Will be overwritten
            )
            self?.onPendingPaymentRequest?(pendingRequest)
        }
        
        try? interactiveManager?.setGenerator(generator: receiptGenerator!)
    }
    
    // MARK: - Key Management
    
    /// Get or derive X25519 keys for Noise protocol
    public func getOrDeriveKeys() async throws -> X25519KeypairResult {
        let deviceId = getDeviceId()
        
        // Try cache first
        if let cached = keyCache.getKey(deviceId: deviceId, epoch: currentEpoch) {
            return cached
        }
        
        // Derive via Pubky Ring (or mock)
        let keypair = try await pubkyRing.getOrDeriveKeypair(deviceId: deviceId, epoch: currentEpoch)
        return keypair
    }
    
    /// Get device ID for key derivation
    private func getDeviceId() -> String {
        return UIDevice.current.identifierForVendor?.uuidString ?? "unknown-ios-device"
    }
    
    /// Increment epoch for key rotation
    public func rotateKeys() {
        currentEpoch += 1
        keyCache.clearAllKeys(for: getDeviceId())
    }
    
    // MARK: - Connection Management
    
    /// Discover Noise endpoint for a recipient
    public func discoverEndpoint(recipientPubkey: String) async throws -> NoiseEndpointInfo {
        // Query the directory for the recipient's Noise endpoint
        // For demo, we also check local configuration
        
        // Check if we have a manual override
        if let envEndpoint = ProcessInfo.processInfo.environment["PAYKIT_PAYEE_NOISE_ENDPOINT"] {
            return try parseEndpointString(envEndpoint, recipientPubkey: recipientPubkey)
        }
        
        // Query Pubky directory for Noise endpoint
        let directoryService = DirectoryService.shared
        if let endpoint = try await directoryService.discoverNoiseEndpoint(ownerPubkey: recipientPubkey) {
            return endpoint
        }
        
        throw NoisePaymentError.endpointNotFound
    }
    
    /// Parse endpoint string in format: host:port:pubkey_hex
    private func parseEndpointString(_ str: String, recipientPubkey: String) throws -> NoiseEndpointInfo {
        let parts = str.split(separator: ":")
        guard parts.count >= 3 else {
            throw NoisePaymentError.invalidEndpoint("Expected format: host:port:pubkey_hex")
        }
        
        let host = String(parts[0])
        guard let port = UInt16(parts[1]) else {
            throw NoisePaymentError.invalidEndpoint("Invalid port")
        }
        let pubkeyHex = String(parts[2])
        
        return NoiseEndpointInfo(
            host: host,
            port: port,
            serverPubkeyHex: pubkeyHex,
            metadata: nil
        )
    }
    
    /// Connect to a Noise endpoint
    public func connect(to endpoint: NoiseEndpointInfo) async throws {
        // Ensure we have keys
        let keypair = try await getOrDeriveKeys()
        
        // Create connection
        let nwEndpoint = NWEndpoint.hostPort(
            host: NWEndpoint.Host(endpoint.host),
            port: NWEndpoint.Port(rawValue: endpoint.port)!
        )
        
        connection = NWConnection(to: nwEndpoint, using: .tcp)
        
        // Wait for connection
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            var completed = false
            
            connection?.stateUpdateHandler = { [weak self] state in
                guard !completed else { return }
                
                switch state {
                case .ready:
                    completed = true
                    continuation.resume()
                case .failed(let error):
                    completed = true
                    self?.connection = nil
                    continuation.resume(throwing: NoisePaymentError.connectionFailed(error.localizedDescription))
                case .cancelled:
                    completed = true
                    self?.connection = nil
                    continuation.resume(throwing: NoisePaymentError.cancelled)
                default:
                    break
                }
            }
            
            connection?.start(queue: connectionQueue)
            
            // Timeout
            DispatchQueue.main.asyncAfter(deadline: .now() + connectionTimeoutSecs) {
                guard !completed else { return }
                completed = true
                self.connection?.cancel()
                self.connection = nil
                continuation.resume(throwing: NoisePaymentError.timeout)
            }
        }
        
        // Perform Noise handshake
        try await performHandshake(
            serverPubkey: endpoint.serverPubkeyData!,
            localKeypair: keypair
        )
        
        await MainActor.run {
            self.isConnected = true
        }
    }
    
    /// Perform Noise IK handshake
    private func performHandshake(serverPubkey: Data, localKeypair: X25519KeypairResult) async throws {
        // For demo, we use the mock seed from MockPubkyRingService
        // In production, this would use the derived X25519 keys directly
        
        guard let seedData = try? MockPubkyRingService.shared.getEd25519SeedBytes() else {
            throw NoisePaymentError.noIdentity
        }
        
        let deviceIdData = getDeviceId().data(using: .utf8)!
        
        let config = FfiMobileConfig(
            autoReconnect: false,
            maxReconnectAttempts: 0,
            reconnectDelayMs: 0,
            batterySaver: false,
            chunkSize: 32768
        )
        
        do {
            noiseManager = try FfiNoiseManager.newClient(
                config: config,
                clientSeed: seedData,
                clientKid: "paykit-ios",
                deviceId: deviceIdData
            )
        } catch {
            throw NoisePaymentError.handshakeFailed("Failed to create Noise manager: \(error.localizedDescription)")
        }
        
        // Step 1: Initiate connection
        let initResult: FfiConnectionResult
        do {
            initResult = try noiseManager!.initiateConnection(serverPk: serverPubkey, hint: nil)
        } catch {
            throw NoisePaymentError.handshakeFailed("Failed to initiate: \(error.localizedDescription)")
        }
        
        // Step 2: Send first message
        try await sendRawData(initResult.firstMessage)
        
        // Step 3: Receive server response
        let response = try await receiveRawData()
        
        // Step 4: Complete handshake
        do {
            let sessionId = try noiseManager!.completeConnection(
                sessionId: initResult.sessionId,
                serverResponse: response
            )
            
            await MainActor.run {
                self.currentSessionId = sessionId
            }
        } catch {
            throw NoisePaymentError.handshakeFailed("Failed to complete: \(error.localizedDescription)")
        }
    }
    
    /// Disconnect from current peer
    public func disconnect() {
        if let sessionId = currentSessionId {
            noiseManager?.removeSession(sessionId: sessionId)
        }
        
        connection?.cancel()
        connection = nil
        noiseManager = nil
        
        Task { @MainActor in
            self.isConnected = false
            self.currentSessionId = nil
            self.connectedPeer = nil
        }
    }
    
    // MARK: - Payment Operations
    
    /// Send a payment request
    public func sendPaymentRequest(_ request: NoisePaymentRequest) async throws -> NoisePaymentResponse {
        guard let sessionId = currentSessionId, let manager = noiseManager else {
            throw NoisePaymentError.connectionFailed("Not connected")
        }
        
        // Create message JSON
        let messageJson: [String: Any] = [
            "type": "request_receipt",
            "receipt_id": request.receiptId,
            "payer": request.payerPubkey,
            "payee": request.payeePubkey,
            "method_id": request.methodId,
            "amount": request.amount as Any,
            "currency": request.currency as Any,
            "description": request.description as Any,
            "created_at": Int(Date().timeIntervalSince1970)
        ]
        
        let jsonData = try JSONSerialization.data(withJSONObject: messageJson)
        
        // Encrypt
        let ciphertext: Data
        do {
            ciphertext = try manager.encrypt(sessionId: sessionId, plaintext: jsonData)
        } catch {
            throw NoisePaymentError.encryptionFailed(error.localizedDescription)
        }
        
        // Send with length prefix
        try await sendLengthPrefixedData(ciphertext)
        
        // Receive response
        let responseCiphertext = try await receiveLengthPrefixedData()
        
        // Decrypt
        let responsePlaintext: Data
        do {
            responsePlaintext = try manager.decrypt(sessionId: sessionId, ciphertext: responseCiphertext)
        } catch {
            throw NoisePaymentError.decryptionFailed(error.localizedDescription)
        }
        
        // Parse response
        return try parsePaymentResponse(responsePlaintext, expectedReceiptId: request.receiptId)
    }
    
    /// Parse payment response JSON
    private func parsePaymentResponse(_ data: Data, expectedReceiptId: String) throws -> NoisePaymentResponse {
        guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let msgType = json["type"] as? String else {
            throw NoisePaymentError.invalidResponse("Invalid JSON structure")
        }
        
        switch msgType {
        case "confirm_receipt":
            let receiptId = json["receipt_id"] as? String
            let confirmedAt = (json["confirmed_at"] as? Int).map { Date(timeIntervalSince1970: Double($0)) }
            
            return NoisePaymentResponse(
                success: true,
                receiptId: receiptId,
                confirmedAt: confirmedAt,
                errorCode: nil,
                errorMessage: nil
            )
            
        case "error":
            let code = json["code"] as? String ?? "unknown"
            let message = json["message"] as? String ?? "Unknown error"
            
            return NoisePaymentResponse(
                success: false,
                receiptId: nil,
                confirmedAt: nil,
                errorCode: code,
                errorMessage: message
            )
            
        default:
            throw NoisePaymentError.invalidResponse("Unexpected message type: \(msgType)")
        }
    }
    
    // MARK: - Network I/O
    
    /// Send raw data
    private func sendRawData(_ data: Data) async throws {
        guard let conn = connection else {
            throw NoisePaymentError.connectionFailed("No connection")
        }
        
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            conn.send(content: data, completion: .contentProcessed { error in
                if let error = error {
                    continuation.resume(throwing: NoisePaymentError.connectionFailed(error.localizedDescription))
                } else {
                    continuation.resume()
                }
            })
        }
    }
    
    /// Receive raw data
    private func receiveRawData(length: Int? = nil) async throws -> Data {
        guard let conn = connection else {
            throw NoisePaymentError.connectionFailed("No connection")
        }
        
        return try await withCheckedThrowingContinuation { continuation in
            conn.receive(
                minimumIncompleteLength: length ?? 1,
                maximumLength: length ?? 65536
            ) { data, _, _, error in
                if let error = error {
                    continuation.resume(throwing: NoisePaymentError.connectionFailed(error.localizedDescription))
                } else if let data = data, !data.isEmpty {
                    continuation.resume(returning: data)
                } else {
                    continuation.resume(throwing: NoisePaymentError.connectionFailed("No data received"))
                }
            }
        }
    }
    
    /// Send data with 4-byte length prefix
    private func sendLengthPrefixedData(_ data: Data) async throws {
        var message = Data()
        var length = UInt32(data.count).bigEndian
        message.append(Data(bytes: &length, count: 4))
        message.append(data)
        try await sendRawData(message)
    }
    
    /// Receive length-prefixed data
    private func receiveLengthPrefixedData() async throws -> Data {
        // Read length
        let lengthData = try await receiveRawData(length: 4)
        let length = lengthData.withUnsafeBytes { $0.load(as: UInt32.self) }.bigEndian
        
        // Read content
        return try await receiveRawData(length: Int(length))
    }
}

// MARK: - Server Mode Support

extension NoisePaymentService {
    
    /// Server status
    public struct ServerStatus {
        public let isRunning: Bool
        public let port: UInt16?
        public let noisePubkeyHex: String?
        public let activeConnections: Int
    }
    
    /// Start listening for incoming payment requests
    public func startServer(port: UInt16 = 0) async throws -> ServerStatus {
        // Stop existing server if running
        if serverListener != nil {
            stopServer()
        }
        
        // Get our keys for publishing
        let keypair = try await getOrDeriveKeys()
        serverKeypair = keypair
        
        // Create server configuration
        let serverConfig = try PaykitClient().createNoiseServerConfig(
            port: port,
            serverKeypair: X25519Keypair(
                secretKeyHex: keypair.secretKeyHex,
                publicKeyHex: keypair.publicKeyHex
            )
        )
        
        // Create Noise manager for server
        serverNoiseManager = try PaykitClient().createNoiseManagerServer(config: serverConfig)
        
        // Create NWListener
        let parameters = NWParameters.tcp
        parameters.allowLocalEndpointReuse = true
        
        if port > 0 {
            let endpoint = NWEndpoint.hostPort(host: .any, port: NWEndpoint.Port(rawValue: port)!)
            serverListener = try NWListener(using: parameters, on: endpoint)
        } else {
            serverListener = try NWListener(using: parameters, on: .any)
        }
        
        guard let listener = serverListener else {
            throw NoisePaymentError.serverError(code: "INIT_FAILED", message: "Failed to create listener")
        }
        
        // Set up new connection handler
        listener.newConnectionHandler = { [weak self] connection in
            self?.handleNewConnection(connection)
        }
        
        // Start listening
        let actualPort = try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<UInt16, Error>) in
            listener.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    if let port = listener.port {
                        continuation.resume(returning: port.rawValue)
                    } else {
                        continuation.resume(throwing: NoisePaymentError.serverError(code: "NO_PORT", message: "Failed to get listener port"))
                    }
                case .failed(let error):
                    continuation.resume(throwing: NoisePaymentError.serverError(code: "LISTENER_FAILED", message: error.localizedDescription))
                case .cancelled:
                    continuation.resume(throwing: NoisePaymentError.cancelled)
                default:
                    break
                }
            }
            
            listener.start(queue: self.serverQueue)
        }
        
        // Register for background tasks
        registerBackgroundTask()
        
        return ServerStatus(
            isRunning: true,
            port: actualPort,
            noisePubkeyHex: keypair.publicKeyHex,
            activeConnections: serverConnections.count
        )
    }
    
    /// Handle new incoming connection
    private func handleNewConnection(_ connection: NWConnection) {
        let connectionId = UUID()
        
        // Create server connection handler
        let serverConnection = ServerConnection(
            id: connectionId,
            connection: connection,
            noiseManager: serverNoiseManager
        )
        
        serverConnections[connectionId] = serverConnection
        
        // Set up connection state handler
        connection.stateUpdateHandler = { [weak self] state in
            switch state {
            case .ready:
                self?.handleReadyConnection(serverConnection)
            case .failed(let error), .cancelled:
                self?.serverConnections.removeValue(forKey: connectionId)
            default:
                break
            }
        }
        
        // Start connection
        connection.start(queue: serverQueue)
    }
    
    /// Handle ready connection - perform Noise handshake
    private func handleReadyConnection(_ serverConnection: ServerConnection) {
        Task {
            do {
                // Perform server-side Noise handshake
                // This is handled by FfiNoiseManager in server mode
                // The handshake happens automatically when data is received
                
                // Set up receive handler
                serverConnection.connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { [weak self] data, context, isComplete, error in
                    if let error = error {
                        print("Server connection receive error: \(error)")
                        self?.serverConnections.removeValue(forKey: serverConnection.id)
                        return
                    }
                    
                    if let data = data, !data.isEmpty {
                        self?.handleServerMessage(serverConnection, data: data)
                    }
                    
                    if !isComplete {
                        // Continue receiving
                        serverConnection.connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { _, _, _, _ in }
                    }
                }
            } catch {
                print("Server connection setup error: \(error)")
                serverConnections.removeValue(forKey: serverConnection.id)
            }
        }
    }
    
    /// Handle message from client
    private func handleServerMessage(_ serverConnection: ServerConnection, data: Data) {
        guard let manager = interactiveManager else {
            print("Server: No interactive manager available")
            return
        }
        
        // Check if handshake is complete
        if !serverConnection.isHandshakeComplete {
            // First message should be the Noise handshake
            handleServerHandshake(serverConnection, data: data)
            return
        }
        
        guard let sessionId = serverConnection.sessionId,
              let noiseManager = serverConnection.noiseManager else {
            print("Server: No session ID or noise manager for connection \(serverConnection.id)")
            return
        }
        
        do {
            // Decrypt message using Noise manager
            let plaintext = try noiseManager.decrypt(sessionId: sessionId, ciphertext: data)
            
            guard let messageJson = String(data: plaintext, encoding: .utf8) else {
                print("Server: Failed to decode message as UTF-8")
                return
            }
            
            // Get our public key (payee)
            let myPubkey = keyManager.publicKeyZ32
            let peerPubkey = serverConnection.clientPubkeyHex ?? "unknown"
            
            // Handle message using interactive manager
            let responseJson = try manager.handleMessage(
                messageJson: messageJson,
                peerPubkey: peerPubkey,
                myPubkey: myPubkey
            )
            
            // If there's a response, encrypt and send it
            if let response = responseJson {
                sendEncryptedResponse(serverConnection, responseJson: response)
            }
            
        } catch {
            print("Server: Error processing message: \(error.localizedDescription)")
            // Send error response
            sendErrorResponse(serverConnection, code: "PROCESSING_ERROR", message: error.localizedDescription)
        }
    }
    
    /// Handle Noise handshake for server connection
    private func handleServerHandshake(_ serverConnection: ServerConnection, data: Data) {
        guard let noiseManager = serverConnection.noiseManager else {
            print("Server: No noise manager for handshake")
            return
        }
        
        do {
            // Accept connection (server-side handshake)
            let acceptResult = try noiseManager.acceptConnection(firstMsg: data)
            
            // Store session ID and mark handshake complete
            serverConnection.sessionId = acceptResult.sessionId
            serverConnection.isHandshakeComplete = true
            serverConnection.clientPubkeyHex = acceptResult.sessionId // Session ID may include client identity
            
            // Send handshake response to client
            serverConnection.connection.send(content: acceptResult.responseMessage, completion: .contentProcessed { error in
                if let error = error {
                    print("Server: Failed to send handshake response: \(error.localizedDescription)")
                } else {
                    print("Server: Handshake complete for connection \(serverConnection.id)")
                }
            })
            
            // Continue receiving messages
            receiveNextMessage(serverConnection)
            
        } catch {
            print("Server: Handshake failed: \(error.localizedDescription)")
            serverConnections.removeValue(forKey: serverConnection.id)
            serverConnection.connection.cancel()
        }
    }
    
    /// Send encrypted response to client
    private func sendEncryptedResponse(_ serverConnection: ServerConnection, responseJson: String) {
        guard let sessionId = serverConnection.sessionId,
              let noiseManager = serverConnection.noiseManager,
              let responseData = responseJson.data(using: .utf8) else {
            return
        }
        
        do {
            let ciphertext = try noiseManager.encrypt(sessionId: sessionId, plaintext: responseData)
            
            // Send with length prefix
            var message = Data()
            var length = UInt32(ciphertext.count).bigEndian
            message.append(Data(bytes: &length, count: 4))
            message.append(ciphertext)
            
            serverConnection.connection.send(content: message, completion: .contentProcessed { error in
                if let error = error {
                    print("Server: Failed to send response: \(error.localizedDescription)")
                }
            })
        } catch {
            print("Server: Failed to encrypt response: \(error.localizedDescription)")
        }
    }
    
    /// Send error response to client
    private func sendErrorResponse(_ serverConnection: ServerConnection, code: String, message: String) {
        let errorJson = """
        {"type": "Error", "code": "\(code)", "message": "\(message)"}
        """
        sendEncryptedResponse(serverConnection, responseJson: errorJson)
    }
    
    /// Continue receiving messages on a connection
    private func receiveNextMessage(_ serverConnection: ServerConnection) {
        serverConnection.connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { [weak self] data, context, isComplete, error in
            if let error = error {
                print("Server: Receive error: \(error.localizedDescription)")
                self?.serverConnections.removeValue(forKey: serverConnection.id)
                return
            }
            
            if let data = data, !data.isEmpty {
                self?.handleServerMessage(serverConnection, data: data)
            }
            
            if !isComplete {
                self?.receiveNextMessage(serverConnection)
            }
        }
    }
    
    /// Register background task for server
    private func registerBackgroundTask() {
        backgroundTask = UIApplication.shared.beginBackgroundTask { [weak self] in
            self?.endBackgroundTask()
        }
    }
    
    /// End background task
    private func endBackgroundTask() {
        if backgroundTask != .invalid {
            UIApplication.shared.endBackgroundTask(backgroundTask)
            backgroundTask = .invalid
        }
    }
    
    /// Stop the server
    public func stopServer() {
        // Cancel all connections
        for (_, serverConnection) in serverConnections {
            serverConnection.connection.cancel()
        }
        serverConnections.removeAll()
        
        // Stop listener
        serverListener?.cancel()
        serverListener = nil
        
        // End background task
        endBackgroundTask()
    }
    
    /// Get current server status
    public func getServerStatus() -> ServerStatus {
        let port = serverListener?.port?.rawValue
        return ServerStatus(
            isRunning: serverListener != nil,
            port: port,
            noisePubkeyHex: serverKeypair?.publicKeyHex,
            activeConnections: serverConnections.count
        )
    }
}

// MARK: - Server Connection

/// Represents an active server connection with Noise session state
private class ServerConnection {
    let id: UUID
    let connection: NWConnection
    let noiseManager: FfiNoiseManager?
    var sessionId: String?
    var isHandshakeComplete: Bool = false
    var clientPubkeyHex: String?
    
    init(id: UUID, connection: NWConnection, noiseManager: FfiNoiseManager?) {
        self.id = id
        self.connection = connection
        self.noiseManager = noiseManager
    }
}

// MARK: - Server Receipt Generator

/// Receipt generator callback implementation for server mode
public class ServerReceiptGenerator: ReceiptGeneratorCallback {
    
    private var pendingRequestCallback: ((ReceiptRequest) -> Void)?
    private var invoiceGenerator: ((ReceiptRequest) -> String)?
    
    public init() {}
    
    /// Set callback for when a payment request is received
    public func onPaymentRequest(_ callback: @escaping (ReceiptRequest) -> Void) {
        pendingRequestCallback = callback
    }
    
    /// Set callback for generating invoices (for real payments)
    public func setInvoiceGenerator(_ generator: @escaping (ReceiptRequest) -> String) {
        invoiceGenerator = generator
    }
    
    public func generateReceipt(request: ReceiptRequest) -> ReceiptGenerationResult {
        // Notify callback about pending request
        pendingRequestCallback?(request)
        
        // Generate invoice if generator is set, otherwise use mock
        let invoice = invoiceGenerator?(request) ?? "mock_invoice_\(request.receiptId)"
        
        // Create confirmed receipt with invoice in metadata
        var metadataDict = [String: String]()
        if let metadataJson = request.metadataJson.data(using: .utf8),
           let parsed = try? JSONSerialization.jsonObject(with: metadataJson) as? [String: String] {
            metadataDict = parsed
        }
        metadataDict["invoice"] = invoice
        metadataDict["confirmed_at"] = ISO8601DateFormatter().string(from: Date())
        
        let updatedMetadataJson = (try? JSONSerialization.data(withJSONObject: metadataDict))
            .flatMap { String(data: $0, encoding: .utf8) } ?? "{}"
        
        let confirmedReceipt = ReceiptRequest(
            receiptId: request.receiptId,
            payer: request.payer,
            payee: request.payee,
            methodId: request.methodId,
            amount: request.amount,
            currency: request.currency,
            metadataJson: updatedMetadataJson
        )
        
        return ReceiptGenerationResult.ok(receipt: confirmedReceipt)
    }
}

// MARK: - Pending Payment Request (for ViewModel)

/// Represents a pending payment request received by the server
public struct PendingPaymentRequest: Identifiable {
    public let id: String
    public let payerPubkey: String
    public let amount: String?
    public let currency: String?
    public let methodId: String
    public let receivedAt: Date
    public let connectionId: UUID
}

