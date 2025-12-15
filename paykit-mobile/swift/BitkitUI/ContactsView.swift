//
//  ContactsView.swift
//  PaykitMobile
//
//  Contacts management UI component for Bitkit integration.
//  This is a template that Bitkit can adapt to their design system.
//

import SwiftUI
import PaykitMobile

/// Contacts view model for Bitkit integration
public class BitkitContactsViewModel: ObservableObject {
    @Published public var contacts: [Contact] = []
    @Published public var filteredContacts: [Contact] = []
    @Published public var isLoading = false
    @Published public var searchText = ""
    @Published public var showingAddSheet = false
    
    private let contactStorage: ContactStorageProtocol
    
    public init(contactStorage: ContactStorageProtocol) {
        self.contactStorage = contactStorage
    }
    
    func loadContacts() {
        isLoading = true
        contacts = contactStorage.listContacts()
        filteredContacts = contacts
        isLoading = false
    }
    
    func search(query: String) {
        searchText = query
        if query.isEmpty {
            filteredContacts = contacts
        } else {
            filteredContacts = contacts.filter { contact in
                contact.name.localizedCaseInsensitiveContains(query) ||
                contact.pubkey.localizedCaseInsensitiveContains(query)
            }
        }
    }
    
    func addContact(name: String, pubkey: String) {
        let contact = Contact(id: UUID().uuidString, name: name, pubkey: pubkey)
        contacts.append(contact)
        filteredContacts = contacts
        // Bitkit should persist this to their storage
    }
    
    func deleteContact(at offsets: IndexSet) {
        contacts.remove(atOffsets: offsets)
        filteredContacts = contacts
        // Bitkit should delete from their storage
    }
}

/// Contacts view component
public struct BitkitContactsView: View {
    @ObservedObject var viewModel: BitkitContactsViewModel
    @State private var newContactName = ""
    @State private var newContactPubkey = ""
    
    public init(viewModel: BitkitContactsViewModel) {
        self.viewModel = viewModel
    }
    
    public var body: some View {
        NavigationView {
            Group {
                if viewModel.isLoading {
                    ProgressView()
                } else if viewModel.filteredContacts.isEmpty {
                    emptyStateView
                } else {
                    contactsList
                }
            }
            .navigationTitle("Contacts")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { viewModel.showingAddSheet = true }) {
                        Image(systemName: "plus")
                    }
                }
            }
            .searchable(text: $viewModel.searchText, prompt: "Search contacts")
            .onChange(of: viewModel.searchText) { newValue in
                viewModel.search(query: newValue)
            }
            .sheet(isPresented: $viewModel.showingAddSheet) {
                addContactSheet
            }
            .onAppear {
                viewModel.loadContacts()
            }
        }
    }
    
    private var emptyStateView: some View {
        VStack(spacing: 24) {
            Image(systemName: "person.crop.circle.badge.plus")
                .font(.system(size: 80))
                .foregroundColor(.secondary)
            
            Text("No Contacts")
                .font(.title2)
                .fontWeight(.semibold)
            
            Text("Add contacts to easily send payments")
                .foregroundColor(.secondary)
            
            Button(action: { viewModel.showingAddSheet = true }) {
                Label("Add Contact", systemImage: "plus.circle.fill")
            }
            .buttonStyle(.borderedProminent)
        }
        .padding()
    }
    
    private var contactsList: some View {
        List {
            ForEach(viewModel.filteredContacts) { contact in
                ContactRow(contact: contact)
            }
            .onDelete(perform: viewModel.deleteContact)
        }
    }
    
    private var addContactSheet: some View {
        NavigationView {
            Form {
                Section("Contact Information") {
                    TextField("Name", text: $newContactName)
                    TextField("Public Key", text: $newContactPubkey)
                        .autocapitalization(.none)
                }
                
                Section {
                    Button("Add Contact") {
                        viewModel.addContact(name: newContactName, pubkey: newContactPubkey)
                        newContactName = ""
                        newContactPubkey = ""
                        viewModel.showingAddSheet = false
                    }
                    .disabled(newContactName.isEmpty || newContactPubkey.isEmpty)
                }
            }
            .navigationTitle("Add Contact")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        viewModel.showingAddSheet = false
                    }
                }
            }
        }
    }
}

struct ContactRow: View {
    let contact: Contact
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(contact.name)
                .font(.headline)
            Text(contact.pubkey)
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }
}
