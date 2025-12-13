//
//  ContactsView.swift
//  PaykitDemo
//
//  Contact management view for storing payment recipients.
//

import SwiftUI
import Combine

struct ContactsView: View {
    @StateObject private var viewModel = ContactsViewModel()
    @State private var showingAddSheet = false
    @State private var searchText = ""
    
    var body: some View {
        NavigationView {
            Group {
                if viewModel.contacts.isEmpty && !viewModel.isLoading {
                    emptyStateView
                } else {
                    contactsList
                }
            }
            .navigationTitle("Contacts")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { showingAddSheet = true }) {
                        Image(systemName: "plus")
                    }
                }
            }
            .searchable(text: $searchText, prompt: "Search contacts")
            .onChange(of: searchText) { newValue in
                viewModel.search(query: newValue)
            }
            .sheet(isPresented: $showingAddSheet) {
                AddContactSheet(viewModel: viewModel)
            }
            .onAppear {
                viewModel.loadContacts()
            }
        }
    }
    
    private var emptyStateView: some View {
        VStack(spacing: 20) {
            Image(systemName: "person.crop.circle.badge.plus")
                .font(.system(size: 80))
                .foregroundColor(.secondary)
            
            Text("No Contacts Yet")
                .font(.title2)
                .fontWeight(.semibold)
            
            Text("Add contacts to easily send payments\nto your favorite recipients.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
            
            Button(action: { showingAddSheet = true }) {
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
                    .swipeActions(edge: .trailing, allowsFullSwipe: true) {
                        Button(role: .destructive) {
                            viewModel.deleteContact(id: contact.id)
                        } label: {
                            Label("Delete", systemImage: "trash")
                        }
                    }
            }
        }
    }
}

struct ContactRow: View {
    let contact: Contact
    @State private var showingDetail = false
    @State private var copied = false
    
    var body: some View {
        HStack(spacing: 12) {
            Circle()
                .fill(Color.blue.opacity(0.2))
                .frame(width: 44, height: 44)
                .overlay(
                    Text(String(contact.name.prefix(1)).uppercased())
                        .font(.headline)
                        .foregroundColor(.blue)
                )
            
            VStack(alignment: .leading, spacing: 4) {
                Text(contact.name)
                    .font(.headline)
                
                Text(contact.abbreviatedKey)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .fontDesign(.monospaced)
            }
            
            Spacer()
            
            VStack(alignment: .trailing, spacing: 4) {
                if contact.paymentCount > 0 {
                    Text("\(contact.paymentCount) payments")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                
                Button(action: copyPublicKey) {
                    Image(systemName: copied ? "checkmark.circle.fill" : "doc.on.doc")
                        .foregroundColor(copied ? .green : .secondary)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.vertical, 4)
        .onTapGesture {
            showingDetail = true
        }
        .sheet(isPresented: $showingDetail) {
            ContactDetailSheet(contact: contact)
        }
    }
    
    private func copyPublicKey() {
        UIPasteboard.general.string = contact.publicKeyZ32
        copied = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
            copied = false
        }
    }
}

struct AddContactSheet: View {
    @ObservedObject var viewModel: ContactsViewModel
    @Environment(\.dismiss) private var dismiss
    
    @State private var name = ""
    @State private var publicKey = ""
    @State private var notes = ""
    @State private var errorMessage: String?
    
    var body: some View {
        NavigationView {
            Form {
                Section {
                    TextField("Name", text: $name)
                    TextField("Public Key (z-base32)", text: $publicKey)
                        .autocapitalization(.none)
                        .textContentType(.none)
                        .fontDesign(.monospaced)
                } header: {
                    Text("Contact Info")
                }
                
                Section {
                    TextField("Notes (optional)", text: $notes, axis: .vertical)
                        .lineLimit(3...6)
                } header: {
                    Text("Notes")
                }
                
                if let error = errorMessage {
                    Section {
                        Text(error)
                            .foregroundColor(.red)
                    }
                }
            }
            .navigationTitle("Add Contact")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") {
                        saveContact()
                    }
                    .disabled(name.isEmpty || publicKey.isEmpty)
                }
            }
        }
    }
    
    private func saveContact() {
        // Basic validation
        guard !name.isEmpty else {
            errorMessage = "Name is required"
            return
        }
        
        guard !publicKey.isEmpty else {
            errorMessage = "Public key is required"
            return
        }
        
        // TODO: Validate z-base32 format
        
        let contact = Contact(
            publicKeyZ32: publicKey,
            name: name,
            notes: notes.isEmpty ? nil : notes
        )
        
        do {
            try viewModel.addContact(contact)
            dismiss()
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}

struct ContactDetailSheet: View {
    let contact: Contact
    @Environment(\.dismiss) private var dismiss
    @State private var copied = false
    
    var body: some View {
        NavigationView {
            List {
                Section {
                    HStack {
                        Text("Name")
                        Spacer()
                        Text(contact.name)
                            .foregroundColor(.secondary)
                    }
                    
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Public Key")
                        Text(contact.publicKeyZ32)
                            .font(.caption)
                            .fontDesign(.monospaced)
                            .foregroundColor(.secondary)
                    }
                    
                    Button(action: {
                        UIPasteboard.general.string = contact.publicKeyZ32
                        copied = true
                        DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
                            copied = false
                        }
                    }) {
                        HStack {
                            Text("Copy Public Key")
                            Spacer()
                            Image(systemName: copied ? "checkmark" : "doc.on.doc")
                        }
                    }
                }
                
                if let notes = contact.notes {
                    Section("Notes") {
                        Text(notes)
                    }
                }
                
                Section("Statistics") {
                    HStack {
                        Text("Payments")
                        Spacer()
                        Text("\(contact.paymentCount)")
                            .foregroundColor(.secondary)
                    }
                    
                    HStack {
                        Text("Added")
                        Spacer()
                        Text(contact.createdAt, style: .date)
                            .foregroundColor(.secondary)
                    }
                    
                    if let lastPayment = contact.lastPaymentAt {
                        HStack {
                            Text("Last Payment")
                            Spacer()
                            Text(lastPayment, style: .relative)
                                .foregroundColor(.secondary)
                        }
                    }
                }
            }
            .navigationTitle("Contact Details")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
        }
    }
}

// MARK: - View Model

class ContactsViewModel: ObservableObject {
    @Published var contacts: [Contact] = []
    @Published var filteredContacts: [Contact] = []
    @Published var isLoading = false
    @Published var error: String?
    
    private let storage = ContactStorage()
    private var searchQuery = ""
    
    func loadContacts() {
        isLoading = true
        contacts = storage.listContacts()
        filteredContacts = contacts
        isLoading = false
    }
    
    func addContact(_ contact: Contact) throws {
        try storage.saveContact(contact)
        loadContacts()
    }
    
    func deleteContact(id: String) {
        do {
            try storage.deleteContact(id: id)
            loadContacts()
        } catch {
            self.error = error.localizedDescription
        }
    }
    
    func search(query: String) {
        searchQuery = query
        if query.isEmpty {
            filteredContacts = contacts
        } else {
            filteredContacts = storage.searchContacts(query: query)
        }
    }
    
    func recordPayment(contactId: String) {
        do {
            try storage.recordPayment(contactId: contactId)
            loadContacts()
        } catch {
            self.error = error.localizedDescription
        }
    }
}

#Preview {
    ContactsView()
}

