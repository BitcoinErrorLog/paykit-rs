// PubkyStorageAdapter.swift
// Adapter for Pubky Storage Operations
//
// This adapter implements the Pubky storage callback interfaces
// to enable real Pubky directory operations in Paykit mobile apps.
//
// It uses HTTP requests to communicate with Pubky homeservers.

import Foundation

// MARK: - Pubky Storage Adapter

/// Adapter for unauthenticated (read-only) Pubky storage operations.
///
/// This adapter makes HTTP requests to Pubky homeservers to read
/// public data from other users' storage.
public final class PubkyUnauthenticatedStorageAdapter: PubkyUnauthenticatedStorageCallback {
    
    // MARK: - Properties
    
    /// Base URL for Pubky homeserver (e.g., "https://homeserver.example.com")
    /// If nil, uses default Pubky mainnet homeservers
    private let homeserverBaseURL: String?
    
    /// URLSession for HTTP requests
    private let urlSession: URLSession
    
    // MARK: - Initialization
    
    /// Create a new unauthenticated storage adapter.
    ///
    /// - Parameter homeserverBaseURL: Optional base URL for homeserver.
    ///   If nil, uses default Pubky mainnet homeservers.
    public init(homeserverBaseURL: String? = nil) {
        self.homeserverBaseURL = homeserverBaseURL
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 30.0
        config.timeoutIntervalForResource = 60.0
        self.urlSession = URLSession(configuration: config)
    }
    
    // MARK: - PubkyUnauthenticatedStorageCallback
    
    public func get(ownerPubkey: String, path: String) -> StorageGetResult {
        // Construct URL: https://_pubky.{ownerPubkey}{path}
        // For now, if homeserverBaseURL is provided, use it directly
        // Otherwise, construct _pubky subdomain URL
        let urlString: String
        if let baseURL = homeserverBaseURL {
            // Direct homeserver URL
            urlString = "\(baseURL)/pubky\(ownerPubkey)\(path)"
        } else {
            // Use _pubky subdomain (requires Pkarr resolution)
            // Note: Pkarr resolution for _pubky subdomains is a future enhancement.
            // For production use, configure homeserverBaseURL directly.
            // DHT-based resolution would require pkarr library integration.
            urlString = "https://_pubky.\(ownerPubkey)\(path)"
        }
        
        guard let url = URL(string: urlString) else {
            return StorageGetResult.err(message: "Invalid URL: \(urlString)")
        }
        
        // Perform synchronous HTTP request (blocking)
        // Note: In production, this should be async, but UniFFI callbacks are sync
        let semaphore = DispatchSemaphore(value: 0)
        var result: StorageGetResult?
        
        let task = urlSession.dataTask(with: url) { data, response, error in
            defer { semaphore.signal() }
            
            if let error = error {
                result = StorageGetResult.err(message: "Network error: \(error.localizedDescription)")
                return
            }
            
            guard let httpResponse = response as? HTTPURLResponse else {
                result = StorageGetResult.err(message: "Invalid response type")
                return
            }
            
            if httpResponse.statusCode == 404 {
                result = StorageGetResult.ok(content: nil)
                return
            }
            
            guard (200...299).contains(httpResponse.statusCode) else {
                result = StorageGetResult.err(message: "HTTP \(httpResponse.statusCode)")
                return
            }
            
            guard let data = data else {
                result = StorageGetResult.ok(content: nil)
                return
            }
            
            guard let content = String(data: data, encoding: .utf8) else {
                result = StorageGetResult.err(message: "Failed to decode response as UTF-8")
                return
            }
            
            result = StorageGetResult.ok(content: content)
        }
        
        task.resume()
        semaphore.wait()
        
        return result ?? StorageGetResult.err(message: "Request failed")
    }
    
    public func list(ownerPubkey: String, prefix: String) -> StorageListResult {
        // Construct URL for listing: https://_pubky.{ownerPubkey}{prefix}?shallow=true
        let urlString: String
        if let baseURL = homeserverBaseURL {
            urlString = "\(baseURL)/pubky\(ownerPubkey)\(prefix)?shallow=true"
        } else {
            urlString = "https://_pubky.\(ownerPubkey)\(prefix)?shallow=true"
        }
        
        guard let url = URL(string: urlString) else {
            return StorageListResult.err(message: "Invalid URL: \(urlString)")
        }
        
        let semaphore = DispatchSemaphore(value: 0)
        var result: StorageListResult?
        
        let task = urlSession.dataTask(with: url) { data, response, error in
            defer { semaphore.signal() }
            
            if let error = error {
                result = StorageListResult.err(message: "Network error: \(error.localizedDescription)")
                return
            }
            
            guard let httpResponse = response as? HTTPURLResponse else {
                result = StorageListResult.err(message: "Invalid response type")
                return
            }
            
            if httpResponse.statusCode == 404 {
                result = StorageListResult.ok(entries: [])
                return
            }
            
            guard (200...299).contains(httpResponse.statusCode) else {
                result = StorageListResult.err(message: "HTTP \(httpResponse.statusCode)")
                return
            }
            
            guard let data = data else {
                result = StorageListResult.ok(entries: [])
                return
            }
            
            // Parse JSON array of resources
            do {
                let resources = try JSONDecoder().decode([PubkyResource].self, from: data)
                let entries = resources.map { $0.path }
                result = StorageListResult.ok(entries: entries)
            } catch {
                // If JSON parsing fails, try to parse as simple array of strings
                if let jsonArray = try? JSONSerialization.jsonObject(with: data) as? [String] {
                    result = StorageListResult.ok(entries: jsonArray)
                } else {
                    result = StorageListResult.err(message: "Failed to parse response: \(error.localizedDescription)")
                }
            }
        }
        
        task.resume()
        semaphore.wait()
        
        return result ?? StorageListResult.err(message: "Request failed")
    }
}

// MARK: - Pubky Resource

/// Represents a Pubky resource entry from list operations
private struct PubkyResource: Codable {
    let path: String
}

// MARK: - Authenticated Storage Adapter

/// Adapter for authenticated Pubky storage operations.
///
/// This adapter makes HTTP requests to Pubky homeservers with
/// session authentication to read/write the owner's storage.
public final class PubkyAuthenticatedStorageAdapter: PubkyAuthenticatedStorageCallback {
    
    // MARK: - Properties
    
    /// Base URL for Pubky homeserver
    private let homeserverBaseURL: String?
    
    /// Session ID for authenticated requests
    private let sessionId: String
    
    /// URLSession for HTTP requests
    private let urlSession: URLSession
    
    // MARK: - Initialization
    
    /// Create a new authenticated storage adapter.
    ///
    /// - Parameters:
    ///   - sessionId: Session ID for authentication
    ///   - homeserverBaseURL: Optional base URL for homeserver
    public init(sessionId: String, homeserverBaseURL: String? = nil) {
        self.sessionId = sessionId
        self.homeserverBaseURL = homeserverBaseURL
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 30.0
        config.timeoutIntervalForResource = 60.0
        config.httpCookieStorage = HTTPCookieStorage.shared
        self.urlSession = URLSession(configuration: config)
    }
    
    // MARK: - PubkyAuthenticatedStorageCallback
    
    public func put(path: String, content: String) -> StorageOperationResult {
        // Construct URL: {homeserverBaseURL}{path}
        let urlString: String
        if let baseURL = homeserverBaseURL {
            urlString = "\(baseURL)\(path)"
        } else {
            // Use default homeserver
            urlString = "https://homeserver.pubky.app\(path)"
        }
        
        guard let url = URL(string: urlString) else {
            return StorageOperationResult.err(message: "Invalid URL: \(urlString)")
        }
        
        var request = URLRequest(url: url)
        request.httpMethod = "PUT"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        // Add session cookie
        request.setValue("session=\(sessionId)", forHTTPHeaderField: "Cookie")
        
        let httpBody = content.data(using: .utf8)
        request.httpBody = httpBody
        
        let semaphore = DispatchSemaphore(value: 0)
        var result: StorageOperationResult?
        
        let task = urlSession.dataTask(with: request) { data, response, error in
            defer { semaphore.signal() }
            
            if let error = error {
                result = StorageOperationResult.err(message: "Network error: \(error.localizedDescription)")
                return
            }
            
            guard let httpResponse = response as? HTTPURLResponse else {
                result = StorageOperationResult.err(message: "Invalid response type")
                return
            }
            
            guard (200...299).contains(httpResponse.statusCode) else {
                result = StorageOperationResult.err(message: "HTTP \(httpResponse.statusCode)")
                return
            }
            
            result = StorageOperationResult.ok()
        }
        
        task.resume()
        semaphore.wait()
        
        return result ?? StorageOperationResult.err(message: "Request failed")
    }
    
    public func get(path: String) -> StorageGetResult {
        let urlString: String
        if let baseURL = homeserverBaseURL {
            urlString = "\(baseURL)\(path)"
        } else {
            urlString = "https://homeserver.pubky.app\(path)"
        }
        
        guard let url = URL(string: urlString) else {
            return StorageGetResult.err(message: "Invalid URL: \(urlString)")
        }
        
        var request = URLRequest(url: url)
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.setValue("session=\(sessionId)", forHTTPHeaderField: "Cookie")
        
        let semaphore = DispatchSemaphore(value: 0)
        var result: StorageGetResult?
        
        let task = urlSession.dataTask(with: request) { data, response, error in
            defer { semaphore.signal() }
            
            if let error = error {
                result = StorageGetResult.err(message: "Network error: \(error.localizedDescription)")
                return
            }
            
            guard let httpResponse = response as? HTTPURLResponse else {
                result = StorageGetResult.err(message: "Invalid response type")
                return
            }
            
            if httpResponse.statusCode == 404 {
                result = StorageGetResult.ok(content: nil)
                return
            }
            
            guard (200...299).contains(httpResponse.statusCode) else {
                result = StorageGetResult.err(message: "HTTP \(httpResponse.statusCode)")
                return
            }
            
            guard let data = data else {
                result = StorageGetResult.ok(content: nil)
                return
            }
            
            guard let content = String(data: data, encoding: .utf8) else {
                result = StorageGetResult.err(message: "Failed to decode response as UTF-8")
                return
            }
            
            result = StorageGetResult.ok(content: content)
        }
        
        task.resume()
        semaphore.wait()
        
        return result ?? StorageGetResult.err(message: "Request failed")
    }
    
    public func delete(path: String) -> StorageOperationResult {
        let urlString: String
        if let baseURL = homeserverBaseURL {
            urlString = "\(baseURL)\(path)"
        } else {
            urlString = "https://homeserver.pubky.app\(path)"
        }
        
        guard let url = URL(string: urlString) else {
            return StorageOperationResult.err(message: "Invalid URL: \(urlString)")
        }
        
        var request = URLRequest(url: url)
        request.httpMethod = "DELETE"
        request.setValue("session=\(sessionId)", forHTTPHeaderField: "Cookie")
        
        let semaphore = DispatchSemaphore(value: 0)
        var result: StorageOperationResult?
        
        let task = urlSession.dataTask(with: request) { data, response, error in
            defer { semaphore.signal() }
            
            if let error = error {
                result = StorageOperationResult.err(message: "Network error: \(error.localizedDescription)")
                return
            }
            
            guard let httpResponse = response as? HTTPURLResponse else {
                result = StorageOperationResult.err(message: "Invalid response type")
                return
            }
            
            // 204 No Content or 200 OK are both valid for DELETE
            guard (200...299).contains(httpResponse.statusCode) || httpResponse.statusCode == 404 else {
                result = StorageOperationResult.err(message: "HTTP \(httpResponse.statusCode)")
                return
            }
            
            result = StorageOperationResult.ok()
        }
        
        task.resume()
        semaphore.wait()
        
        return result ?? StorageOperationResult.err(message: "Request failed")
    }
    
    public func list(prefix: String) -> StorageListResult {
        let urlString: String
        if let baseURL = homeserverBaseURL {
            urlString = "\(baseURL)\(prefix)?shallow=true"
        } else {
            urlString = "https://homeserver.pubky.app\(prefix)?shallow=true"
        }
        
        guard let url = URL(string: urlString) else {
            return StorageListResult.err(message: "Invalid URL: \(urlString)")
        }
        
        var request = URLRequest(url: url)
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.setValue("session=\(sessionId)", forHTTPHeaderField: "Cookie")
        
        let semaphore = DispatchSemaphore(value: 0)
        var result: StorageListResult?
        
        let task = urlSession.dataTask(with: request) { data, response, error in
            defer { semaphore.signal() }
            
            if let error = error {
                result = StorageListResult.err(message: "Network error: \(error.localizedDescription)")
                return
            }
            
            guard let httpResponse = response as? HTTPURLResponse else {
                result = StorageListResult.err(message: "Invalid response type")
                return
            }
            
            if httpResponse.statusCode == 404 {
                result = StorageListResult.ok(entries: [])
                return
            }
            
            guard (200...299).contains(httpResponse.statusCode) else {
                result = StorageListResult.err(message: "HTTP \(httpResponse.statusCode)")
                return
            }
            
            guard let data = data else {
                result = StorageListResult.ok(entries: [])
                return
            }
            
            do {
                let resources = try JSONDecoder().decode([PubkyResource].self, from: data)
                let entries = resources.map { $0.path }
                result = StorageListResult.ok(entries: entries)
            } catch {
                if let jsonArray = try? JSONSerialization.jsonObject(with: data) as? [String] {
                    result = StorageListResult.ok(entries: jsonArray)
                } else {
                    result = StorageListResult.err(message: "Failed to parse response: \(error.localizedDescription)")
                }
            }
        }
        
        task.resume()
        semaphore.wait()
        
        return result ?? StorageListResult.err(message: "Request failed")
    }
}

