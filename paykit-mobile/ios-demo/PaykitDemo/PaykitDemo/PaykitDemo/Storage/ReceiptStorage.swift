//
//  ReceiptStorage.swift
//  PaykitDemo
//
//  Persistent storage for receipts using Keychain.
//

import Foundation

/// Manages persistent storage of payment receipts
class ReceiptStorage {
    
    private let keychain: KeychainStorage
    private let identityName: String
    private let maxReceiptsToKeep = 500  // Limit stored receipts
    
    // In-memory cache
    private var receiptsCache: [Receipt]?
    
    private var receiptsKey: String {
        "paykit.receipts.\(identityName)"
    }
    
    init(identityName: String, keychain: KeychainStorage = KeychainStorage(serviceIdentifier: "com.paykit.demo")) {
        self.identityName = identityName
        self.keychain = keychain
    }
    
    // MARK: - CRUD Operations
    
    /// Get all receipts (newest first)
    func listReceipts() -> [Receipt] {
        if let cached = receiptsCache {
            return cached
        }
        
        do {
            guard let data = try keychain.retrieve(key: receiptsKey) else {
                return []
            }
            var receipts = try JSONDecoder().decode([Receipt].self, from: data)
            // Sort by date, newest first
            receipts.sort { $0.createdAt > $1.createdAt }
            receiptsCache = receipts
            return receipts
        } catch {
            print("ReceiptStorage: Failed to load receipts: \(error)")
            return []
        }
    }
    
    /// Get receipts filtered by status
    func listReceipts(status: PaymentStatus) -> [Receipt] {
        return listReceipts().filter { $0.status == status }
    }
    
    /// Get receipts filtered by direction
    func listReceipts(direction: PaymentDirection) -> [Receipt] {
        return listReceipts().filter { $0.direction == direction }
    }
    
    /// Get recent receipts (limited count)
    func recentReceipts(limit: Int = 10) -> [Receipt] {
        return Array(listReceipts().prefix(limit))
    }
    
    /// Get a specific receipt
    func getReceipt(id: String) -> Receipt? {
        return listReceipts().first { $0.id == id }
    }
    
    /// Add a new receipt
    func addReceipt(_ receipt: Receipt) throws {
        var receipts = listReceipts()
        
        // Add new receipt at the beginning (newest first)
        receipts.insert(receipt, at: 0)
        
        // Trim to max size
        if receipts.count > maxReceiptsToKeep {
            receipts = Array(receipts.prefix(maxReceiptsToKeep))
        }
        
        try persistReceipts(receipts)
    }
    
    /// Update an existing receipt
    func updateReceipt(_ receipt: Receipt) throws {
        var receipts = listReceipts()
        
        guard let index = receipts.firstIndex(where: { $0.id == receipt.id }) else {
            throw ReceiptStorageError.notFound(id: receipt.id)
        }
        
        receipts[index] = receipt
        try persistReceipts(receipts)
    }
    
    /// Delete a receipt
    func deleteReceipt(id: String) throws {
        var receipts = listReceipts()
        receipts.removeAll { $0.id == id }
        try persistReceipts(receipts)
    }
    
    /// Search receipts by counterparty or memo
    func searchReceipts(query: String) -> [Receipt] {
        let query = query.lowercased()
        return listReceipts().filter { receipt in
            receipt.displayName.lowercased().contains(query) ||
            receipt.counterpartyKey.lowercased().contains(query) ||
            (receipt.memo?.lowercased().contains(query) ?? false)
        }
    }
    
    /// Get receipts for a specific counterparty
    func receiptsForCounterparty(publicKey: String) -> [Receipt] {
        return listReceipts().filter { $0.counterpartyKey == publicKey }
    }
    
    /// Clear all receipts
    func clearAll() throws {
        try persistReceipts([])
    }
    
    // MARK: - Statistics
    
    /// Total sent amount
    func totalSent() -> UInt64 {
        return listReceipts(direction: .sent)
            .filter { $0.status == .completed }
            .reduce(0) { $0 + $1.amountSats }
    }
    
    /// Total received amount
    func totalReceived() -> UInt64 {
        return listReceipts(direction: .received)
            .filter { $0.status == .completed }
            .reduce(0) { $0 + $1.amountSats }
    }
    
    /// Count of completed transactions
    func completedCount() -> Int {
        return listReceipts(status: .completed).count
    }
    
    /// Count of pending transactions
    func pendingCount() -> Int {
        return listReceipts(status: .pending).count
    }
    
    // MARK: - Private
    
    private func persistReceipts(_ receipts: [Receipt]) throws {
        let data = try JSONEncoder().encode(receipts)
        try keychain.store(key: receiptsKey, data: data)
        receiptsCache = receipts
    }
}

// MARK: - Errors

enum ReceiptStorageError: LocalizedError {
    case encodingFailed
    case decodingFailed
    case notFound(id: String)
    
    var errorDescription: String? {
        switch self {
        case .encodingFailed:
            return "Failed to encode receipts"
        case .decodingFailed:
            return "Failed to decode receipts"
        case .notFound(let id):
            return "Receipt not found: \(id)"
        }
    }
}

