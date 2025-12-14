// PubkyRingIntegration.swift
// Pubky Ring Integration Protocol
//
// This file defines the integration protocol for communicating with the
// real Pubky Ring app for key derivation. In production, Paykit apps
// request X25519 keys from Pubky Ring rather than storing Ed25519 seeds.
//
// Key Architecture:
//   - Ed25519 (pkarr) keys are "cold" and managed by Pubky Ring
//   - X25519 (Noise) keys are "hot" and derived on-demand by Pubky Ring
//   - Paykit apps request pre-derived keys, never the seed
//
// URL Scheme: pubkyring://derive-keypair?deviceId=...&epoch=...
// Response: JSON with secret_key_hex and public_key_hex

import Foundation
import UIKit

/// Response from Pubky Ring key derivation
public struct PubkyRingKeypairResponse: Codable {
    public let secretKeyHex: String
    public let publicKeyHex: String
    
    enum CodingKeys: String, CodingKey {
        case secretKeyHex = "secret_key_hex"
        case publicKeyHex = "public_key_hex"
    }
}

/// Error response from Pubky Ring
public struct PubkyRingErrorResponse: Codable {
    public let error: String
    public let message: String
}

/// Error types for Pubky Ring integration
public enum PubkyRingError: LocalizedError {
    case appNotInstalled
    case requestFailed(String)
    case invalidResponse
    case derivationFailed(String)
    case serviceUnavailable
    case timeout
    case userCancelled
    
    public var errorDescription: String? {
        switch self {
        case .appNotInstalled:
            return "Pubky Ring app is not installed. Please install Pubky Ring to use this feature."
        case .requestFailed(let message):
            return "Request to Pubky Ring failed: \(message)"
        case .invalidResponse:
            return "Invalid response from Pubky Ring."
        case .derivationFailed(let message):
            return "Key derivation failed: \(message)"
        case .serviceUnavailable:
            return "Pubky Ring service is unavailable."
        case .timeout:
            return "Request to Pubky Ring timed out."
        case .userCancelled:
            return "User cancelled the request."
        }
    }
    
    public var errorCode: String {
        switch self {
        case .appNotInstalled: return "app_not_installed"
        case .requestFailed: return "request_failed"
        case .invalidResponse: return "invalid_response"
        case .derivationFailed: return "derivation_failed"
        case .serviceUnavailable: return "service_unavailable"
        case .timeout: return "timeout"
        case .userCancelled: return "user_cancelled"
        }
    }
}

/// Integration protocol for Pubky Ring app
///
/// This class handles communication with the Pubky Ring app for key derivation.
/// It uses URL schemes to request X25519 keys derived from the user's Ed25519
/// identity stored in Pubky Ring.
///
/// **Fallback**: If Pubky Ring is not installed, falls back to MockPubkyRingService.
public final class PubkyRingIntegration {
    
    // MARK: - Singleton
    
    public static let shared = PubkyRingIntegration()
    
    // MARK: - Constants
    
    private enum Constants {
        static let pubkyRingScheme = "pubkyring"
        static let deriveKeypairPath = "derive-keypair"
        static let callbackScheme = "paykit" // Your app's URL scheme
        static let requestTimeout: TimeInterval = 30.0
    }
    
    // MARK: - Properties
    
    private var pendingCompletions: [String: (Result<X25519KeypairResult, PubkyRingError>) -> Void] = [:]
    private let completionQueue = DispatchQueue(label: "com.paykit.pubkyring.completion")
    
    /// Whether to use mock service as fallback when Pubky Ring is unavailable
    public var useMockFallback: Bool = true
    
    // MARK: - Initialization
    
    private init() {}
    
    // MARK: - Public Methods
    
    /// Check if Pubky Ring app is installed
    public var isPubkyRingInstalled: Bool {
        guard let url = URL(string: "\(Constants.pubkyRingScheme)://") else {
            return false
        }
        return UIApplication.shared.canOpenURL(url)
    }
    
    /// Derive X25519 keypair from Pubky Ring
    ///
    /// This method attempts to request key derivation from Pubky Ring.
    /// If Pubky Ring is not installed and `useMockFallback` is true,
    /// it falls back to MockPubkyRingService.
    ///
    /// - Parameters:
    ///   - deviceId: Unique identifier for this device
    ///   - epoch: Key rotation epoch (increment to rotate keys)
    /// - Returns: Derived X25519 keypair
    /// - Throws: PubkyRingError if derivation fails
    public func deriveX25519Keypair(deviceId: String, epoch: UInt32) async throws -> X25519KeypairResult {
        // Try Pubky Ring first if installed
        if isPubkyRingInstalled {
            return try await requestFromPubkyRing(deviceId: deviceId, epoch: epoch)
        }
        
        // Fall back to mock service
        if useMockFallback {
            return try useMockService(deviceId: deviceId, epoch: epoch)
        }
        
        throw PubkyRingError.appNotInstalled
    }
    
    /// Get or derive X25519 keypair with caching
    ///
    /// This method first checks the NoiseKeyCache, then requests from
    /// Pubky Ring if not cached.
    ///
    /// - Parameters:
    ///   - deviceId: Unique identifier for this device
    ///   - epoch: Key rotation epoch
    /// - Returns: X25519 keypair (from cache or freshly derived)
    public func getOrDeriveKeypair(deviceId: String, epoch: UInt32) async throws -> X25519KeypairResult {
        // Check cache first
        if let cached = NoiseKeyCache.shared.getKey(deviceId: deviceId, epoch: epoch) {
            return cached
        }
        
        // Derive and cache
        let keypair = try await deriveX25519Keypair(deviceId: deviceId, epoch: epoch)
        NoiseKeyCache.shared.setKey(keypair, deviceId: deviceId, epoch: epoch)
        
        return keypair
    }
    
    /// Handle callback URL from Pubky Ring
    ///
    /// Call this from your app's URL handler when receiving a callback
    /// from Pubky Ring.
    ///
    /// - Parameter url: The callback URL
    /// - Returns: true if the URL was handled
    @discardableResult
    public func handleCallback(url: URL) -> Bool {
        guard url.scheme == Constants.callbackScheme,
              url.host == "pubkyring-callback" else {
            return false
        }
        
        guard let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
              let queryItems = components.queryItems else {
            return false
        }
        
        // Extract request ID
        guard let requestId = queryItems.first(where: { $0.name == "request_id" })?.value else {
            return false
        }
        
        // Get completion handler
        var completion: ((Result<X25519KeypairResult, PubkyRingError>) -> Void)?
        completionQueue.sync {
            completion = pendingCompletions.removeValue(forKey: requestId)
        }
        
        guard let handler = completion else {
            return false
        }
        
        // Check for error
        if let errorCode = queryItems.first(where: { $0.name == "error" })?.value {
            let message = queryItems.first(where: { $0.name == "message" })?.value ?? "Unknown error"
            handler(.failure(mapErrorCode(errorCode, message: message)))
            return true
        }
        
        // Parse success response
        guard let secretKeyHex = queryItems.first(where: { $0.name == "secret_key_hex" })?.value,
              let publicKeyHex = queryItems.first(where: { $0.name == "public_key_hex" })?.value,
              let deviceId = queryItems.first(where: { $0.name == "device_id" })?.value,
              let epochStr = queryItems.first(where: { $0.name == "epoch" })?.value,
              let epoch = UInt32(epochStr) else {
            handler(.failure(.invalidResponse))
            return true
        }
        
        let result = X25519KeypairResult(
            secretKeyHex: secretKeyHex,
            publicKeyHex: publicKeyHex,
            deviceId: deviceId,
            epoch: epoch
        )
        
        handler(.success(result))
        return true
    }
    
    // MARK: - Private Methods
    
    private func requestFromPubkyRing(deviceId: String, epoch: UInt32) async throws -> X25519KeypairResult {
        // Generate request ID
        let requestId = UUID().uuidString
        
        // Build URL
        var components = URLComponents()
        components.scheme = Constants.pubkyRingScheme
        components.host = Constants.deriveKeypairPath
        components.queryItems = [
            URLQueryItem(name: "device_id", value: deviceId),
            URLQueryItem(name: "epoch", value: String(epoch)),
            URLQueryItem(name: "callback", value: "\(Constants.callbackScheme)://pubkyring-callback"),
            URLQueryItem(name: "request_id", value: requestId)
        ]
        
        guard let url = components.url else {
            throw PubkyRingError.requestFailed("Failed to build request URL")
        }
        
        // Create continuation for async/await
        return try await withCheckedThrowingContinuation { continuation in
            // Store completion handler
            completionQueue.sync {
                pendingCompletions[requestId] = { result in
                    switch result {
                    case .success(let keypair):
                        continuation.resume(returning: keypair)
                    case .failure(let error):
                        continuation.resume(throwing: error)
                    }
                }
            }
            
            // Set timeout
            DispatchQueue.main.asyncAfter(deadline: .now() + Constants.requestTimeout) { [weak self] in
                self?.completionQueue.sync {
                    if let handler = self?.pendingCompletions.removeValue(forKey: requestId) {
                        handler(.failure(.timeout))
                    }
                }
            }
            
            // Open Pubky Ring
            DispatchQueue.main.async {
                UIApplication.shared.open(url, options: [:]) { success in
                    if !success {
                        self.completionQueue.sync {
                            if let handler = self.pendingCompletions.removeValue(forKey: requestId) {
                                handler(.failure(.serviceUnavailable))
                            }
                        }
                    }
                }
            }
        }
    }
    
    private func useMockService(deviceId: String, epoch: UInt32) throws -> X25519KeypairResult {
        let mock = MockPubkyRingService.shared
        
        // Initialize mock if needed
        if !mock.hasSeed {
            try mock.initializeWithNewSeed()
        }
        
        return try mock.deriveKeypair(deviceId: deviceId, epoch: epoch)
    }
    
    private func mapErrorCode(_ code: String, message: String) -> PubkyRingError {
        switch code {
        case "key_not_found":
            return .derivationFailed("No identity configured in Pubky Ring")
        case "derivation_failed":
            return .derivationFailed(message)
        case "invalid_parameters":
            return .requestFailed("Invalid parameters: \(message)")
        case "service_unavailable":
            return .serviceUnavailable
        case "user_cancelled":
            return .userCancelled
        default:
            return .requestFailed(message)
        }
    }
}

