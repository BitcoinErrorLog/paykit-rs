//
//  PaymentRequestStorage.swift
//  PaykitDemo
//
//  Persistent storage for payment requests using Keychain.
//

import Foundation

/// Manages persistent storage of payment requests
class PaymentRequestStorage {
    
    private let keychain: KeychainStorage
    private let requestsKey = "paykit.payment_requests.list"
    private let maxRequestsToKeep = 200  // Limit stored requests
    
    // In-memory cache
    private var requestsCache: [StoredPaymentRequest]?
    
    init(keychain: KeychainStorage = KeychainStorage(serviceIdentifier: "com.paykit.demo")) {
        self.keychain = keychain
    }
    
    // MARK: - CRUD Operations
    
    /// Get all requests (newest first)
    func listRequests() -> [StoredPaymentRequest] {
        if let cached = requestsCache {
            return cached
        }
        
        do {
            guard let data = try keychain.retrieve(key: requestsKey) else {
                return []
            }
            var requests = try JSONDecoder().decode([StoredPaymentRequest].self, from: data)
            // Sort by date, newest first
            requests.sort { $0.createdAt > $1.createdAt }
            requestsCache = requests
            return requests
        } catch {
            print("PaymentRequestStorage: Failed to load requests: \(error)")
            return []
        }
    }
    
    /// Get pending requests only
    func pendingRequests() -> [StoredPaymentRequest] {
        return listRequests().filter { $0.status == .pending }
    }
    
    /// Get requests filtered by status
    func listRequests(status: PaymentRequestStatus) -> [StoredPaymentRequest] {
        return listRequests().filter { $0.status == status }
    }
    
    /// Get requests filtered by direction
    func listRequests(direction: RequestDirection) -> [StoredPaymentRequest] {
        return listRequests().filter { $0.direction == direction }
    }
    
    /// Get recent requests (limited count)
    func recentRequests(limit: Int = 10) -> [StoredPaymentRequest] {
        return Array(listRequests().prefix(limit))
    }
    
    /// Get a specific request
    func getRequest(id: String) -> StoredPaymentRequest? {
        return listRequests().first { $0.id == id }
    }
    
    /// Add a new request
    func addRequest(_ request: StoredPaymentRequest) throws {
        var requests = listRequests()
        
        // Add new request at the beginning (newest first)
        requests.insert(request, at: 0)
        
        // Trim to max size
        if requests.count > maxRequestsToKeep {
            requests = Array(requests.prefix(maxRequestsToKeep))
        }
        
        try persistRequests(requests)
    }
    
    /// Update an existing request
    func updateRequest(_ request: StoredPaymentRequest) throws {
        var requests = listRequests()
        
        guard let index = requests.firstIndex(where: { $0.id == request.id }) else {
            throw PaymentRequestStorageError.notFound(id: request.id)
        }
        
        requests[index] = request
        try persistRequests(requests)
    }
    
    /// Update request status
    func updateStatus(id: String, status: PaymentRequestStatus) throws {
        guard var request = getRequest(id: id) else {
            throw PaymentRequestStorageError.notFound(id: id)
        }
        request.status = status
        try updateRequest(request)
    }
    
    /// Delete a request
    func deleteRequest(id: String) throws {
        var requests = listRequests()
        requests.removeAll { $0.id == id }
        try persistRequests(requests)
    }
    
    /// Check and mark expired requests
    func checkExpirations() throws {
        let now = Date()
        var requests = listRequests()
        var hasChanges = false
        
        for i in 0..<requests.count {
            if requests[i].status == .pending,
               let expiresAt = requests[i].expiresAt,
               expiresAt < now {
                requests[i].status = .expired
                hasChanges = true
            }
        }
        
        if hasChanges {
            try persistRequests(requests)
        }
    }
    
    /// Clear all requests
    func clearAll() throws {
        try persistRequests([])
    }
    
    // MARK: - Statistics
    
    /// Count of pending requests
    func pendingCount() -> Int {
        return listRequests(status: .pending).count
    }
    
    /// Count of incoming pending requests
    func incomingPendingCount() -> Int {
        return listRequests(direction: .incoming).filter { $0.status == .pending }.count
    }
    
    /// Count of outgoing pending requests
    func outgoingPendingCount() -> Int {
        return listRequests(direction: .outgoing).filter { $0.status == .pending }.count
    }
    
    // MARK: - Private
    
    private func persistRequests(_ requests: [StoredPaymentRequest]) throws {
        let data = try JSONEncoder().encode(requests)
        try keychain.store(key: requestsKey, data: data)
        requestsCache = requests
    }
}

// MARK: - Data Models

/// A payment request stored in persistent storage
struct StoredPaymentRequest: Identifiable, Codable {
    let id: String
    let fromPubkey: String
    let toPubkey: String
    let amountSats: Int64
    let currency: String
    let methodId: String
    let description: String
    let createdAt: Date
    let expiresAt: Date?
    var status: PaymentRequestStatus
    let direction: RequestDirection
    
    /// Display name for the counterparty
    var counterpartyName: String {
        // In a real app, this would look up the contact name
        let key = direction == .incoming ? fromPubkey : toPubkey
        if key.count > 12 {
            return String(key.prefix(6)) + "..." + String(key.suffix(4))
        }
        return key
    }
    
    /// Create from FFI PaymentRequest
    static func fromFFI(_ ffiRequest: PaymentRequest, direction: RequestDirection) -> StoredPaymentRequest {
        StoredPaymentRequest(
            id: ffiRequest.requestId,
            fromPubkey: ffiRequest.fromPubkey,
            toPubkey: ffiRequest.toPubkey,
            amountSats: ffiRequest.amountSats,
            currency: ffiRequest.currency,
            methodId: ffiRequest.methodId,
            description: ffiRequest.description,
            createdAt: Date(timeIntervalSince1970: Double(ffiRequest.createdAt)),
            expiresAt: ffiRequest.expiresAt.map { Date(timeIntervalSince1970: Double($0)) },
            status: .pending,
            direction: direction
        )
    }
}

/// Status of a payment request
enum PaymentRequestStatus: String, Codable {
    case pending = "Pending"
    case accepted = "Accepted"
    case declined = "Declined"
    case expired = "Expired"
    case paid = "Paid"
}

/// Direction of the request (incoming = someone is requesting from you)
enum RequestDirection: String, Codable {
    case incoming  // Someone is requesting payment from you
    case outgoing  // You are requesting payment from someone
}

// MARK: - Errors

enum PaymentRequestStorageError: LocalizedError {
    case encodingFailed
    case decodingFailed
    case notFound(id: String)
    
    var errorDescription: String? {
        switch self {
        case .encodingFailed:
            return "Failed to encode payment requests"
        case .decodingFailed:
            return "Failed to decode payment requests"
        case .notFound(let id):
            return "Payment request not found: \(id)"
        }
    }
}

