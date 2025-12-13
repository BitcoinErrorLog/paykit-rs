//
//  ContactStorage.swift
//  PaykitDemo
//
//  Persistent storage for contacts using Keychain.
//

import Foundation

/// Manages persistent storage of contacts
class ContactStorage {
    
    private let keychain: KeychainStorage
    private let identityName: String
    
    // In-memory cache
    private var contactsCache: [Contact]?
    
    private var contactsKey: String {
        "paykit.contacts.\(identityName)"
    }
    
    init(identityName: String, keychain: KeychainStorage = KeychainStorage(serviceIdentifier: "com.paykit.demo")) {
        self.identityName = identityName
        self.keychain = keychain
    }
    
    // MARK: - CRUD Operations
    
    /// Get all contacts
    func listContacts() -> [Contact] {
        if let cached = contactsCache {
            return cached
        }
        
        do {
            guard let data = try keychain.retrieve(key: contactsKey) else {
                return []
            }
            let contacts = try JSONDecoder().decode([Contact].self, from: data)
            contactsCache = contacts
            return contacts
        } catch {
            print("ContactStorage: Failed to load contacts: \(error)")
            return []
        }
    }
    
    /// Get a specific contact
    func getContact(id: String) -> Contact? {
        return listContacts().first { $0.id == id }
    }
    
    /// Save a new contact or update existing
    func saveContact(_ contact: Contact) throws {
        var contacts = listContacts()
        
        if let index = contacts.firstIndex(where: { $0.id == contact.id }) {
            // Update existing
            contacts[index] = contact
        } else {
            // Add new
            contacts.append(contact)
        }
        
        try persistContacts(contacts)
    }
    
    /// Delete a contact
    func deleteContact(id: String) throws {
        var contacts = listContacts()
        contacts.removeAll { $0.id == id }
        try persistContacts(contacts)
    }
    
    /// Search contacts by name
    func searchContacts(query: String) -> [Contact] {
        let query = query.lowercased()
        return listContacts().filter { contact in
            contact.name.lowercased().contains(query) ||
            contact.publicKeyZ32.lowercased().contains(query)
        }
    }
    
    /// Record a payment to a contact
    func recordPayment(contactId: String) throws {
        var contacts = listContacts()
        guard let index = contacts.firstIndex(where: { $0.id == contactId }) else {
            return
        }
        
        contacts[index].recordPayment()
        try persistContacts(contacts)
    }
    
    /// Clear all contacts
    func clearAll() throws {
        try persistContacts([])
    }
    
    /// Import contacts (merge with existing)
    func importContacts(_ newContacts: [Contact]) throws {
        var contacts = listContacts()
        
        for newContact in newContacts {
            if !contacts.contains(where: { $0.id == newContact.id }) {
                contacts.append(newContact)
            }
        }
        
        try persistContacts(contacts)
    }
    
    /// Export contacts as JSON string
    func exportContacts() throws -> String {
        let contacts = listContacts()
        let data = try JSONEncoder().encode(contacts)
        return String(data: data, encoding: .utf8) ?? "[]"
    }
    
    // MARK: - Private
    
    private func persistContacts(_ contacts: [Contact]) throws {
        let data = try JSONEncoder().encode(contacts)
        try keychain.store(key: contactsKey, data: data)
        contactsCache = contacts
    }
}

// MARK: - Errors

enum ContactStorageError: LocalizedError {
    case encodingFailed
    case decodingFailed
    case notFound(id: String)
    
    var errorDescription: String? {
        switch self {
        case .encodingFailed:
            return "Failed to encode contacts"
        case .decodingFailed:
            return "Failed to decode contacts"
        case .notFound(let id):
            return "Contact not found: \(id)"
        }
    }
}

