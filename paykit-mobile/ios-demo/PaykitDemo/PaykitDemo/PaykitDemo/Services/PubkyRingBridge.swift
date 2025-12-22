//
//  PubkyRingBridge.swift
//  PaykitDemo
//
//  Bridge for communicating with Pubky-ring app via URL schemes.
//  Supports same-device, cross-device (QR), and manual authentication.
//

import Foundation
import UIKit
import CoreImage

// MARK: - PubkyRingBridge

/// Bridge service for communicating with Pubky-ring app.
/// Handles session retrieval, noise key derivation, and cross-device auth.
public final class PubkyRingBridge: ObservableObject {
    
    // MARK: - Singleton
    
    public static let shared = PubkyRingBridge()
    
    // MARK: - Published State
    
    @Published public var connectionState: ConnectionState = .disconnected
    @Published public var currentSession: PubkySession?
    
    // MARK: - Constants
    
    private let pubkyRingScheme = "pubkyring"
    private let callbackScheme = "paykitdemo"
    
    public static var crossDeviceWebUrl: String {
        ProcessInfo.processInfo.environment["PUBKY_CROSS_DEVICE_URL"] ?? "https://pubky.app/auth"
    }
    
    public static var sessionRelayUrl: String {
        ProcessInfo.processInfo.environment["PUBKY_RELAY_URL"] ?? "https://relay.pubky.app/sessions"
    }
    
    // MARK: - State
    
    private var pendingSessionContinuation: CheckedContinuation<PubkySession, Error>?
    private var pendingCrossDeviceRequestId: String?
    private var sessionCache: [String: PubkySession] = [:]
    private var _deviceId: String?
    
    private let storage = KeychainStorage(serviceIdentifier: "com.paykit.demo.bridge")
    
    // MARK: - Initialization
    
    private init() {
        _deviceId = loadOrGenerateDeviceId()
    }
    
    // MARK: - Device ID
    
    public var deviceId: String {
        _deviceId ?? loadOrGenerateDeviceId()
    }
    
    private func loadOrGenerateDeviceId() -> String {
        let key = "device_id"
        if let data = try? storage.retrieve(key: key),
           let id = String(data: data, encoding: .utf8), !id.isEmpty {
            return id
        }
        
        let newId = UUID().uuidString.lowercased()
        if let data = newId.data(using: .utf8) {
            try? storage.store(key: key, data: data)
        }
        return newId
    }
    
    // MARK: - Public API
    
    /// Check if Pubky-ring is installed
    public var isPubkyRingInstalled: Bool {
        guard let url = URL(string: "\(pubkyRingScheme)://") else { return false }
        return UIApplication.shared.canOpenURL(url)
    }
    
    /// Get authentication status
    public var authenticationStatus: AuthenticationStatus {
        isPubkyRingInstalled ? .pubkyRingAvailable : .crossDeviceOnly
    }
    
    /// Request session from Pubky-ring (same device)
    public func requestSession() async throws -> PubkySession {
        guard isPubkyRingInstalled else {
            throw PubkyRingError.appNotInstalled
        }
        
        await MainActor.run {
            connectionState = .connecting
        }
        
        let callbackUrl = "\(callbackScheme)://session"
        let requestUrl = "\(pubkyRingScheme)://session?callback=\(callbackUrl.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? callbackUrl)"
        
        guard let url = URL(string: requestUrl) else {
            throw PubkyRingError.invalidUrl
        }
        
        return try await withCheckedThrowingContinuation { continuation in
            self.pendingSessionContinuation = continuation
            
            DispatchQueue.main.async {
                UIApplication.shared.open(url) { success in
                    if !success {
                        self.pendingSessionContinuation?.resume(throwing: PubkyRingError.failedToOpenApp)
                        self.pendingSessionContinuation = nil
                        Task { @MainActor in
                            self.connectionState = .error("Failed to open Pubky-ring")
                        }
                    }
                }
            }
        }
    }
    
    /// Generate cross-device authentication request
    public func generateCrossDeviceRequest() -> CrossDeviceRequest {
        let requestId = UUID().uuidString.lowercased()
        pendingCrossDeviceRequestId = requestId
        
        var components = URLComponents(string: PubkyRingBridge.crossDeviceWebUrl)!
        components.queryItems = [
            URLQueryItem(name: "request_id", value: requestId),
            URLQueryItem(name: "callback_scheme", value: callbackScheme),
            URLQueryItem(name: "app_name", value: "Paykit Demo"),
            URLQueryItem(name: "relay_url", value: PubkyRingBridge.sessionRelayUrl)
        ]
        
        let url = components.url!
        let qrImage = generateQRCode(from: url.absoluteString)
        
        return CrossDeviceRequest(
            requestId: requestId,
            url: url,
            qrCodeImage: qrImage,
            expiresAt: Date().addingTimeInterval(300)
        )
    }
    
    /// Poll for cross-device session
    public func pollForCrossDeviceSession(requestId: String, timeout: TimeInterval = 300) async throws -> PubkySession {
        let startTime = Date()
        let pollInterval: TimeInterval = 2.0
        
        await MainActor.run {
            connectionState = .connecting
        }
        
        while Date().timeIntervalSince(startTime) < timeout {
            if let session = try? await pollRelayForSession(requestId: requestId) {
                await MainActor.run {
                    self.currentSession = session
                    self.connectionState = .connected(pubkey: session.pubkey)
                }
                sessionCache[session.pubkey] = session
                pendingCrossDeviceRequestId = nil
                return session
            }
            
            try await Task.sleep(nanoseconds: UInt64(pollInterval * 1_000_000_000))
        }
        
        pendingCrossDeviceRequestId = nil
        await MainActor.run {
            connectionState = .error("Timeout")
        }
        throw PubkyRingError.timeout
    }
    
    /// Import session manually
    public func importSession(pubkey: String, sessionSecret: String, capabilities: [String] = []) -> PubkySession {
        let session = PubkySession(
            pubkey: pubkey,
            sessionSecret: sessionSecret,
            capabilities: capabilities,
            createdAt: Date()
        )
        sessionCache[pubkey] = session
        
        DispatchQueue.main.async {
            self.currentSession = session
            self.connectionState = .connected(pubkey: pubkey)
        }
        
        return session
    }
    
    /// Disconnect current session
    public func disconnect() {
        currentSession = nil
        connectionState = .disconnected
    }
    
    /// Handle callback URL
    @discardableResult
    public func handleCallback(url: URL) -> Bool {
        guard url.scheme == callbackScheme else { return false }
        
        let path = url.host ?? url.path
        
        switch path {
        case "session":
            return handleSessionCallback(url: url)
        case "cross-session":
            return handleCrossDeviceCallback(url: url)
        default:
            return false
        }
    }
    
    // MARK: - Private Methods
    
    private func handleSessionCallback(url: URL) -> Bool {
        guard let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
              let queryItems = components.queryItems else {
            pendingSessionContinuation?.resume(throwing: PubkyRingError.invalidCallback)
            pendingSessionContinuation = nil
            return true
        }
        
        var params: [String: String] = [:]
        for item in queryItems {
            if let value = item.value {
                params[item.name] = value
            }
        }
        
        guard let pubkey = params["pubky"],
              let sessionSecret = params["session_secret"] else {
            pendingSessionContinuation?.resume(throwing: PubkyRingError.missingParameters)
            pendingSessionContinuation = nil
            return true
        }
        
        let capabilities = params["capabilities"]?.components(separatedBy: ",") ?? []
        
        let session = PubkySession(
            pubkey: pubkey,
            sessionSecret: sessionSecret,
            capabilities: capabilities,
            createdAt: Date()
        )
        
        sessionCache[pubkey] = session
        
        DispatchQueue.main.async {
            self.currentSession = session
            self.connectionState = .connected(pubkey: pubkey)
        }
        
        pendingSessionContinuation?.resume(returning: session)
        pendingSessionContinuation = nil
        
        return true
    }
    
    private func handleCrossDeviceCallback(url: URL) -> Bool {
        guard let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
              let queryItems = components.queryItems else {
            return false
        }
        
        var params: [String: String] = [:]
        for item in queryItems {
            if let value = item.value {
                params[item.name] = value
            }
        }
        
        if let requestId = params["request_id"], requestId != pendingCrossDeviceRequestId {
            return false
        }
        
        guard let pubkey = params["pubky"],
              let sessionSecret = params["session_secret"] else {
            return false
        }
        
        let capabilities = params["capabilities"]?.components(separatedBy: ",") ?? []
        
        let session = PubkySession(
            pubkey: pubkey,
            sessionSecret: sessionSecret,
            capabilities: capabilities,
            createdAt: Date()
        )
        
        sessionCache[pubkey] = session
        pendingCrossDeviceRequestId = nil
        
        DispatchQueue.main.async {
            self.currentSession = session
            self.connectionState = .connected(pubkey: pubkey)
        }
        
        return true
    }
    
    private func pollRelayForSession(requestId: String) async throws -> PubkySession? {
        let urlString = "\(PubkyRingBridge.sessionRelayUrl)/\(requestId)"
        guard let url = URL(string: urlString) else { return nil }
        
        let (data, response) = try await URLSession.shared.data(from: url)
        
        guard let httpResponse = response as? HTTPURLResponse else { return nil }
        
        if httpResponse.statusCode == 404 {
            return nil
        }
        
        if httpResponse.statusCode == 200 {
            let decoder = JSONDecoder()
            decoder.dateDecodingStrategy = .iso8601
            return try decoder.decode(PubkySession.self, from: data)
        }
        
        return nil
    }
    
    private func generateQRCode(from string: String) -> UIImage? {
        guard let data = string.data(using: .utf8),
              let filter = CIFilter(name: "CIQRCodeGenerator") else { return nil }
        
        filter.setValue(data, forKey: "inputMessage")
        filter.setValue("H", forKey: "inputCorrectionLevel")
        
        guard let outputImage = filter.outputImage else { return nil }
        
        let scale = CGAffineTransform(scaleX: 10, y: 10)
        let scaledImage = outputImage.transformed(by: scale)
        
        let context = CIContext()
        guard let cgImage = context.createCGImage(scaledImage, from: scaledImage.extent) else { return nil }
        
        return UIImage(cgImage: cgImage)
    }
}

// MARK: - Data Models

/// Connection state for Pubky Ring
public enum ConnectionState: Equatable {
    case disconnected
    case connecting
    case connected(pubkey: String)
    case error(String)
    
    public var isConnected: Bool {
        if case .connected = self { return true }
        return false
    }
    
    public var displayText: String {
        switch self {
        case .disconnected:
            return "Not Connected"
        case .connecting:
            return "Connecting..."
        case .connected(let pubkey):
            let short = pubkey.count > 16 ? "\(pubkey.prefix(8))..." : pubkey
            return "Connected: \(short)"
        case .error(let msg):
            return "Error: \(msg)"
        }
    }
}

/// Session from Pubky-ring
public struct PubkySession: Codable, Equatable {
    public let pubkey: String
    public let sessionSecret: String
    public let capabilities: [String]
    public let createdAt: Date
    public let expiresAt: Date?
    
    public init(pubkey: String, sessionSecret: String, capabilities: [String], createdAt: Date, expiresAt: Date? = nil) {
        self.pubkey = pubkey
        self.sessionSecret = sessionSecret
        self.capabilities = capabilities
        self.createdAt = createdAt
        self.expiresAt = expiresAt
    }
    
    public var isExpired: Bool {
        guard let expiresAt = expiresAt else { return false }
        return Date() > expiresAt
    }
}

/// Cross-device request data
public struct CrossDeviceRequest {
    public let requestId: String
    public let url: URL
    public let qrCodeImage: UIImage?
    public let expiresAt: Date
    
    public var isExpired: Bool {
        Date() > expiresAt
    }
    
    public var timeRemaining: TimeInterval {
        max(0, expiresAt.timeIntervalSinceNow)
    }
}

/// Authentication status
public enum AuthenticationStatus {
    case pubkyRingAvailable
    case crossDeviceOnly
    
    public var description: String {
        switch self {
        case .pubkyRingAvailable:
            return "Pubky-ring is available on this device"
        case .crossDeviceOnly:
            return "Use QR code to authenticate from another device"
        }
    }
}

// MARK: - Errors

public enum PubkyRingError: LocalizedError {
    case appNotInstalled
    case invalidUrl
    case failedToOpenApp
    case invalidCallback
    case missingParameters
    case timeout
    case cancelled
    
    public var errorDescription: String? {
        switch self {
        case .appNotInstalled:
            return "Pubky-ring app is not installed"
        case .invalidUrl:
            return "Invalid URL for Pubky-ring request"
        case .failedToOpenApp:
            return "Failed to open Pubky-ring app"
        case .invalidCallback:
            return "Invalid callback from Pubky-ring"
        case .missingParameters:
            return "Missing parameters in callback"
        case .timeout:
            return "Request timed out"
        case .cancelled:
            return "Request was cancelled"
        }
    }
    
    public var userMessage: String {
        switch self {
        case .appNotInstalled:
            return "Pubky-ring is not installed. Use QR code to authenticate from another device."
        case .timeout:
            return "Authentication timed out. Please try again."
        default:
            return "Something went wrong. Please try again."
        }
    }
}

