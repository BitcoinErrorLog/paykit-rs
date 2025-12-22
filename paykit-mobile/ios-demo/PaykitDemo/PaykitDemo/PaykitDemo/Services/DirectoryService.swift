// DirectoryService.swift
// Directory Service for Noise Endpoint Discovery
//
// This service provides methods for discovering and publishing
// Noise endpoints in the Pubky directory.
//
// Key Functions:
// - Discover noise endpoints for recipients
// - Publish our own noise endpoint
// - Query payment methods from directory

import Foundation

// MARK: - Directory Errors

public enum DirectoryError: LocalizedError {
    case notConfigured
    case networkError(String)
    case parseError(String)
    case notFound(String)
    case publishFailed(String)
    
    public var errorDescription: String? {
        switch self {
        case .notConfigured:
            return "Directory service not configured"
        case .networkError(let msg):
            return "Network error: \(msg)"
        case .parseError(let msg):
            return "Parse error: \(msg)"
        case .notFound(let resource):
            return "Not found: \(resource)"
        case .publishFailed(let msg):
            return "Publish failed: \(msg)"
        }
    }
}

// MARK: - Directory Entry Types

/// Noise endpoint published in directory
public struct DirectoryNoiseEndpoint: Codable {
    public let host: String
    public let port: UInt16
    public let pubkey: String
    public let metadata: String?
    
    public init(host: String, port: UInt16, pubkey: String, metadata: String? = nil) {
        self.host = host
        self.port = port
        self.pubkey = pubkey
        self.metadata = metadata
    }
}

/// Payment method published in directory
public struct DirectoryPaymentMethod: Codable {
    public let methodId: String
    public let endpoint: String
    
    public init(methodId: String, endpoint: String) {
        self.methodId = methodId
        self.endpoint = endpoint
    }
}

// MARK: - Directory Service

/// Service for interacting with the Pubky directory
public final class DirectoryService {
    
    // MARK: - Singleton
    
    public static let shared = DirectoryService()
    
    // MARK: - Constants
    
    private static let paykitPathPrefix = "/pub/paykit.app/v0/"
    private static let noiseEndpointPath = "/pub/paykit.app/v0/noise"
    
    // MARK: - Properties
    
    /// Mock storage for demo (in production, uses Pubky SDK)
    private var mockStorage: [String: [String: String]] = [:]
    
    /// Whether to use mock mode
    public var useMockMode = true
    
    /// PaykitClient instance for FFI operations
    private var paykitClient: PaykitClient?
    
    /// Unauthenticated transport for public reads
    private var unauthenticatedTransport: UnauthenticatedTransportFfi?
    
    /// Authenticated transport for writes (requires session)
    private var authenticatedTransport: AuthenticatedTransportFfi?
    
    /// Homeserver base URL (optional, for direct homeserver access)
    public var homeserverBaseURL: String? = nil
    
    // MARK: - Initialization
    
    public init() {
        // Initialize PaykitClient
        do {
            self.paykitClient = try PaykitClient()
        } catch {
            print("Failed to initialize PaykitClient: \(error)")
        }
    }
    
    // MARK: - Transport Setup
    
    /// Configure real Pubky transport
    /// - Parameter homeserverBaseURL: Optional base URL for homeserver
    public func configurePubkyTransport(homeserverBaseURL: String? = nil) {
        self.homeserverBaseURL = homeserverBaseURL
        
        // Create unauthenticated storage adapter
        let unauthAdapter = PubkyUnauthenticatedStorageAdapter(homeserverBaseURL: homeserverBaseURL)
        self.unauthenticatedTransport = UnauthenticatedTransportFfi.fromCallback(callback: unauthAdapter)
    }
    
    /// Configure authenticated transport with session
    /// - Parameters:
    ///   - sessionId: Session ID for authentication
    ///   - ownerPubkey: Owner's public key (z-base32 encoded)
    ///   - homeserverBaseURL: Optional base URL for homeserver
    public func configureAuthenticatedTransport(sessionId: String, ownerPubkey: String, homeserverBaseURL: String? = nil) {
        self.homeserverBaseURL = homeserverBaseURL
        
        // Create authenticated storage adapter
        let authAdapter = PubkyAuthenticatedStorageAdapter(sessionId: sessionId, homeserverBaseURL: homeserverBaseURL)
        self.authenticatedTransport = AuthenticatedTransportFfi.fromCallback(callback: authAdapter, ownerPubkey: ownerPubkey)
    }
    
    // MARK: - Noise Endpoint Discovery
    
    /// Discover noise endpoint for a recipient
    public func discoverNoiseEndpoint(recipientPubkey: String) async throws -> NoiseEndpointInfo? {
        if useMockMode {
            return try await discoverNoiseEndpointMock(recipientPubkey: recipientPubkey)
        }
        
        // Production: Use Pubky SDK via FFI
        return try await discoverNoiseEndpointPubky(recipientPubkey: recipientPubkey)
    }
    
    /// Mock implementation for demo
    private func discoverNoiseEndpointMock(recipientPubkey: String) async throws -> NoiseEndpointInfo? {
        // Check local mock storage
        guard let userStorage = mockStorage[recipientPubkey],
              let endpointJson = userStorage[Self.noiseEndpointPath] else {
            return nil
        }
        
        // Parse JSON
        guard let data = endpointJson.data(using: .utf8),
              let entry = try? JSONDecoder().decode(DirectoryNoiseEndpoint.self, from: data) else {
            return nil
        }
        
        return NoiseEndpointInfo(
            recipientPubkey: recipientPubkey,
            host: entry.host,
            port: entry.port,
            serverNoisePubkey: entry.pubkey,
            metadata: entry.metadata
        )
    }
    
    /// Pubky SDK implementation
    private func discoverNoiseEndpointPubky(recipientPubkey: String) async throws -> NoiseEndpointInfo? {
        guard let client = paykitClient else {
            throw DirectoryError.notConfigured
        }
        
        // Use configured transport or create a new one
        let transport: UnauthenticatedTransportFfi
        if let existing = unauthenticatedTransport {
            transport = existing
        } else {
            // Create transport with adapter
            let adapter = PubkyUnauthenticatedStorageAdapter(homeserverBaseURL: homeserverBaseURL)
            transport = UnauthenticatedTransportFfi.fromCallback(callback: adapter)
            unauthenticatedTransport = transport
        }
        
        do {
            // Use PaykitClient method (no await needed, it's sync)
            return try client.discoverNoiseEndpoint(transport: transport, recipientPubkey: recipientPubkey)
        } catch {
            throw DirectoryError.networkError(error.localizedDescription)
        }
    }
    
    // MARK: - Noise Endpoint Publishing
    
    /// Publish our noise endpoint to the directory
    public func publishNoiseEndpoint(
        host: String,
        port: UInt16,
        noisePubkey: String,
        metadata: String? = nil
    ) async throws {
        let entry = DirectoryNoiseEndpoint(
            host: host,
            port: port,
            pubkey: noisePubkey,
            metadata: metadata
        )
        
        if useMockMode {
            try await publishNoiseEndpointMock(entry: entry)
        } else {
            try await publishNoiseEndpointPubky(entry: entry)
        }
    }
    
    /// Mock implementation
    private func publishNoiseEndpointMock(entry: DirectoryNoiseEndpoint) async throws {
        let keyManager = KeyManager()
        let ownerPubkey = keyManager.publicKeyZ32
        guard !ownerPubkey.isEmpty else {
            throw DirectoryError.notConfigured
        }
        
        let jsonData = try JSONEncoder().encode(entry)
        let jsonString = String(data: jsonData, encoding: .utf8)!
        
        if mockStorage[ownerPubkey] == nil {
            mockStorage[ownerPubkey] = [:]
        }
        mockStorage[ownerPubkey]![Self.noiseEndpointPath] = jsonString
    }
    
    /// Pubky SDK implementation
    private func publishNoiseEndpointPubky(entry: DirectoryNoiseEndpoint) async throws {
        guard let client = paykitClient else {
            throw DirectoryError.notConfigured
        }
        
        guard let transport = authenticatedTransport else {
            throw DirectoryError.notConfigured
        }
        
        do {
            // Use PaykitClient method (no await needed, it's sync)
            try client.publishNoiseEndpoint(
                transport: transport,
                host: entry.host,
                port: entry.port,
                noisePubkey: entry.pubkey,
                metadata: entry.metadata
            )
        } catch {
            throw DirectoryError.publishFailed(error.localizedDescription)
        }
    }
    
    /// Remove noise endpoint from directory
    public func removeNoiseEndpoint() async throws {
        if useMockMode {
            let keyManager = KeyManager()
            let ownerPubkey = keyManager.publicKeyZ32
            guard !ownerPubkey.isEmpty else {
                throw DirectoryError.notConfigured
            }
            mockStorage[ownerPubkey]?.removeValue(forKey: Self.noiseEndpointPath)
        } else {
            guard let client = paykitClient,
                  let transport = authenticatedTransport else {
                throw DirectoryError.notConfigured
            }
            
            do {
                // Use PaykitClient method (no await needed, it's sync)
                try client.removeNoiseEndpoint(transport: transport)
            } catch {
                throw DirectoryError.publishFailed(error.localizedDescription)
            }
        }
    }
    
    // MARK: - Payment Method Discovery
    
    /// Discover all payment methods for a recipient
    public func discoverPaymentMethods(recipientPubkey: String) async throws -> [DirectoryPaymentMethod] {
        if useMockMode {
            return try await discoverPaymentMethodsMock(recipientPubkey: recipientPubkey)
        }
        
        return try await discoverPaymentMethodsPubky(recipientPubkey: recipientPubkey)
    }
    
    /// Mock implementation
    private func discoverPaymentMethodsMock(recipientPubkey: String) async throws -> [DirectoryPaymentMethod] {
        guard let userStorage = mockStorage[recipientPubkey] else {
            return []
        }
        
        var methods: [DirectoryPaymentMethod] = []
        
        for (path, content) in userStorage {
            if path.hasPrefix(Self.paykitPathPrefix) && path != Self.noiseEndpointPath {
                let methodId = String(path.dropFirst(Self.paykitPathPrefix.count))
                methods.append(DirectoryPaymentMethod(methodId: methodId, endpoint: content))
            }
        }
        
        return methods
    }
    
    /// Pubky SDK implementation
    private func discoverPaymentMethodsPubky(recipientPubkey: String) async throws -> [DirectoryPaymentMethod] {
        guard let client = paykitClient else {
            throw DirectoryError.notConfigured
        }
        
        // Use configured transport or create a new one
        let transport: UnauthenticatedTransportFfi
        if let existing = unauthenticatedTransport {
            transport = existing
        } else {
            let adapter = PubkyUnauthenticatedStorageAdapter(homeserverBaseURL: homeserverBaseURL)
            transport = UnauthenticatedTransportFfi.fromCallback(callback: adapter)
            unauthenticatedTransport = transport
        }
        
        do {
            let supportedPayments = try await client.fetchSupportedPayments(transport: transport, ownerPubkey: recipientPubkey)
            return supportedPayments.map { method in
                DirectoryPaymentMethod(methodId: method.methodId, endpoint: method.endpoint)
            }
        } catch {
            throw DirectoryError.networkError(error.localizedDescription)
        }
    }
    
    // MARK: - Payment Method Publishing
    
    /// Publish a payment method to the directory
    public func publishPaymentMethod(methodId: String, endpoint: String) async throws {
        if useMockMode {
            let keyManager = KeyManager()
            let ownerPubkey = keyManager.publicKeyZ32
            guard !ownerPubkey.isEmpty else {
                throw DirectoryError.notConfigured
            }
            
            let path = "\(Self.paykitPathPrefix)\(methodId)"
            
            if mockStorage[ownerPubkey] == nil {
                mockStorage[ownerPubkey] = [:]
            }
            mockStorage[ownerPubkey]![path] = endpoint
        } else {
            guard let client = paykitClient,
                  let transport = authenticatedTransport else {
                throw DirectoryError.notConfigured
            }
            
            do {
                try await client.publishPaymentEndpoint(
                    transport: transport,
                    methodId: methodId,
                    endpointData: endpoint
                )
            } catch {
                throw DirectoryError.publishFailed(error.localizedDescription)
            }
        }
    }
    
    /// Remove a payment method from the directory
    public func removePaymentMethod(methodId: String) async throws {
        if useMockMode {
            let keyManager = KeyManager()
            let ownerPubkey = keyManager.publicKeyZ32
            guard !ownerPubkey.isEmpty else {
                throw DirectoryError.notConfigured
            }
            
            let path = "\(Self.paykitPathPrefix)\(methodId)"
            mockStorage[ownerPubkey]?.removeValue(forKey: path)
        } else {
            guard let client = paykitClient,
                  let transport = authenticatedTransport else {
                throw DirectoryError.notConfigured
            }
            
            do {
                try await client.removePaymentEndpointFromDirectory(transport: transport, methodId: methodId)
            } catch {
                throw DirectoryError.publishFailed(error.localizedDescription)
            }
        }
    }
    
    // MARK: - Profile Operations
    
    private static let profilePath = "/pub/pubky.app/profile.json"
    
    /// Fetch profile from Pubky directory
    public func fetchProfile(for pubkey: String) async throws -> DirectoryProfile? {
        if useMockMode {
            return try await fetchProfileMock(for: pubkey)
        }
        return try await fetchProfilePubky(for: pubkey)
    }
    
    /// Mock implementation for demo
    private func fetchProfileMock(for pubkey: String) async throws -> DirectoryProfile? {
        // Simulate network delay
        try? await Task.sleep(nanoseconds: 800_000_000)
        
        // Check mock storage
        if let userStorage = mockStorage[pubkey],
           let profileJson = userStorage[Self.profilePath],
           let data = profileJson.data(using: .utf8),
           let profile = try? JSONDecoder().decode(DirectoryProfile.self, from: data) {
            return profile
        }
        
        // Return mock profile for demo purposes
        return DirectoryProfile(
            name: "Demo User (\(String(pubkey.prefix(8))))",
            bio: "Imported from Pubky directory",
            image: nil,
            status: nil,
            links: []
        )
    }
    
    /// Pubky SDK implementation
    private func fetchProfilePubky(for pubkey: String) async throws -> DirectoryProfile? {
        guard let adapter = PubkyStorageAdapter.shared.unauthAdapter else {
            // Fall back to mock if not configured
            return try await fetchProfileMock(for: pubkey)
        }
        
        let result = adapter.get(ownerPubkey: pubkey, path: Self.profilePath)
        
        switch result {
        case .success(let data):
            guard let jsonData = data.data(using: .utf8) else { return nil }
            return try JSONDecoder().decode(DirectoryProfile.self, from: jsonData)
        case .notFound:
            return nil
        case .error(let msg):
            throw DirectoryError.networkError(msg)
        }
    }
    
    /// Publish profile to Pubky directory
    public func publishProfile(_ profile: DirectoryProfile) async throws {
        if useMockMode {
            try await publishProfileMock(profile)
            return
        }
        try await publishProfilePubky(profile)
    }
    
    /// Mock implementation for demo
    private func publishProfileMock(_ profile: DirectoryProfile) async throws {
        // Simulate network delay
        try? await Task.sleep(nanoseconds: 500_000_000)
        
        guard let session = PubkyRingBridge.shared.currentSession else {
            throw DirectoryError.notConfigured
        }
        
        let encoder = JSONEncoder()
        encoder.outputFormatting = .prettyPrinted
        let data = try encoder.encode(profile)
        guard let json = String(data: data, encoding: .utf8) else {
            throw DirectoryError.publishFailed("Failed to encode profile")
        }
        
        // Store in mock storage
        if mockStorage[session.pubkey] == nil {
            mockStorage[session.pubkey] = [:]
        }
        mockStorage[session.pubkey]?[Self.profilePath] = json
    }
    
    /// Pubky SDK implementation
    private func publishProfilePubky(_ profile: DirectoryProfile) async throws {
        guard let transport = authenticatedTransport else {
            throw DirectoryError.notConfigured
        }
        
        let encoder = JSONEncoder()
        let data = try encoder.encode(profile)
        
        guard let adapter = PubkyStorageAdapter.shared.authAdapter else {
            throw DirectoryError.notConfigured
        }
        
        let result = adapter.put(path: Self.profilePath, content: data)
        
        switch result {
        case .success:
            return
        case .error(let msg):
            throw DirectoryError.publishFailed(msg)
        }
    }
    
    // MARK: - Demo Helpers
    
    /// Set up demo data for testing
    public func setupDemoData() {
        // Add some demo endpoints
        let demoRecipient = "demo_recipient_pk"
        
        mockStorage[demoRecipient] = [
            Self.noiseEndpointPath: """
                {"host":"127.0.0.1","port":8888,"pubkey":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}
                """,
            "\(Self.paykitPathPrefix)lightning": "lnbc1...",
            "\(Self.paykitPathPrefix)onchain": "bc1q...",
            Self.profilePath: """
                {"name":"Demo Recipient","bio":"Test profile for development"}
                """
        ]
    }
    
    /// Clear all mock data
    public func clearMockData() {
        mockStorage.removeAll()
    }
}

// MARK: - Directory Profile Model

/// Profile data from Pubky directory
public struct DirectoryProfile: Codable, Equatable {
    public let name: String?
    public let bio: String?
    public let image: String?
    public let status: String?
    public let links: [String]
    
    public init(name: String?, bio: String?, image: String?, status: String?, links: [String]) {
        self.name = name
        self.bio = bio
        self.image = image
        self.status = status
        self.links = links
    }
    
    enum CodingKeys: String, CodingKey {
        case name, bio, image, status, links
    }
    
    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        name = try container.decodeIfPresent(String.self, forKey: .name)
        bio = try container.decodeIfPresent(String.self, forKey: .bio)
        image = try container.decodeIfPresent(String.self, forKey: .image)
        status = try container.decodeIfPresent(String.self, forKey: .status)
        links = try container.decodeIfPresent([String].self, forKey: .links) ?? []
    }
}

