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
    @EnvironmentObject var appState: AppState
    @State private var showingAddSheet = false
    @State private var showingDiscoveryResults = false
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
                    HStack(spacing: 12) {
                        Button(action: {
                            Task {
                                await viewModel.discoverContacts(directoryService: appState.paykitClient.createDirectoryService())
                            }
                        }) {
                            Label("Discover", systemImage: "person.badge.plus")
                        }
                        
                        Menu {
                            Button(action: { showingAddSheet = true }) {
                                Label("Add Contact", systemImage: "person.crop.circle.badge.plus")
                            }
                        } label: {
                            Image(systemName: "ellipsis.circle")
                        }
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
            .sheet(isPresented: $viewModel.showingDiscoveryResults) {
                DiscoveryResultsView(
                    contacts: viewModel.discoveredContacts,
                    onImport: { contacts in
                        viewModel.importDiscovered(contacts)
                    },
                    onDismiss: {
                        viewModel.showingDiscoveryResults = false
                    }
                )
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
            
            Text("No Contacts Yet")
                .font(.title2)
                .fontWeight(.semibold)
            
            Text("Add contacts to easily send payments\nto your favorite recipients.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
            
            VStack(spacing: 12) {
                Button(action: {
                    Task {
                        await viewModel.discoverContacts(directoryService: appState.paykitClient.createDirectoryService())
                    }
                }) {
                    Label("Discover from Pubky", systemImage: "person.badge.plus")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
                
                Button(action: { showingAddSheet = true }) {
                    Label("Add Contact Manually", systemImage: "plus.circle.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)
            }
            .padding(.horizontal)
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
    @State private var showingSmartCheckout = false
    @State private var paymentAmount: String = ""
    @State private var showingAmountInput = false
    
    var body: some View {
        NavigationView {
            List {
                // Quick Pay Section
                Section {
                    Button {
                        showingAmountInput = true
                    } label: {
                        HStack {
                            Image(systemName: "paperplane.fill")
                                .foregroundColor(.white)
                                .frame(width: 32, height: 32)
                                .background(Color.blue)
                                .cornerRadius(8)
                            
                            VStack(alignment: .leading, spacing: 2) {
                                Text("Send Payment")
                                    .font(.headline)
                                    .foregroundColor(.primary)
                                Text("Smart checkout with method discovery")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            
                            Spacer()
                            
                            Image(systemName: "chevron.right")
                                .foregroundColor(.secondary)
                        }
                    }
                } header: {
                    Text("Actions")
                }
                
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
                } header: {
                    Text("Contact Info")
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
            .alert("Enter Amount", isPresented: $showingAmountInput) {
                TextField("Amount in sats", text: $paymentAmount)
                    .keyboardType(.numberPad)
                Button("Cancel", role: .cancel) {
                    paymentAmount = ""
                }
                Button("Continue") {
                    if let amount = UInt64(paymentAmount), amount > 0 {
                        showingSmartCheckout = true
                    }
                }
            } message: {
                Text("How many sats to send to \(contact.name)?")
            }
            .sheet(isPresented: $showingSmartCheckout) {
                SmartCheckoutView(
                    recipientPubkey: contact.publicKeyZ32,
                    recipientName: contact.name,
                    amount: UInt64(paymentAmount) ?? 1000
                ) { result in
                    print("Payment completed: \(result.selectedMethod) for \(result.amount) sats")
                    showingSmartCheckout = false
                    paymentAmount = ""
                    dismiss()
                } onCancel: {
                    showingSmartCheckout = false
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
    @Published var discoveredContacts: [DiscoveredContact] = []
    @Published var showingDiscoveryResults = false
    
    private let keyManager = KeyManager()
    private var storage: ContactStorage {
        let identityName = keyManager.getCurrentIdentityName() ?? "default"
        return ContactStorage(identityName: identityName)
    }
    private var searchQuery = ""
    
    init() {
        // Observe identity changes
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(identityDidChange),
            name: .identityDidChange,
            object: nil
        )
    }
    
    @objc private func identityDidChange() {
        loadContacts()
    }
    
    deinit {
        NotificationCenter.default.removeObserver(self)
    }
    
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
    
    func discoverContacts(directoryService: DirectoryService) async {
        isLoading = true
        error = nil
        
        // Get current identity's public key
        guard let currentIdentityName = keyManager.getCurrentIdentityName(),
              let _ = keyManager.getIdentityInfo(name: currentIdentityName),
              !keyManager.publicKeyZ32.isEmpty else {
            await MainActor.run {
                self.error = "No active identity found"
                self.isLoading = false
            }
            return
        }
        let publicKey = keyManager.publicKeyZ32
        
        do {
            // For demo, use locally stored contacts as there's no remote fetch yet
            // In production, this would fetch from Pubky follows
            let contactPubkeys = storage.listContacts().map { $0.publicKeyZ32 }
            
            // Fetch supported payments for each contact to get more info
            var discovered: [DiscoveredContact] = []
            for pubkey in contactPubkeys {
                // Check if contact already exists locally
                let existingContact = contacts.first { $0.publicKeyZ32 == pubkey }
                if existingContact != nil {
                    continue // Skip contacts we already have
                }
                
                // Try to discover supported payment methods
                let supportedPayments = try? await directoryService.discoverPaymentMethods(recipientPubkey: pubkey)
                let hasPaymentMethods = supportedPayments?.isEmpty == false
                
                // Create a discovered contact
                let discoveredContact = DiscoveredContact(
                    publicKeyZ32: pubkey,
                    hasPaymentMethods: hasPaymentMethods,
                    supportedMethods: supportedPayments?.map { $0.methodId } ?? []
                )
                discovered.append(discoveredContact)
            }
            
            await MainActor.run {
                self.discoveredContacts = discovered
                self.showingDiscoveryResults = true
                self.isLoading = false
            }
        } catch {
            await MainActor.run {
                self.error = "Failed to discover contacts: \(error.localizedDescription)"
                self.isLoading = false
            }
        }
    }
    
    func importDiscovered(_ contactsToImport: [DiscoveredContact]) {
        for discovered in contactsToImport {
            // Generate a default name from the public key
            let name = "Contact \(discovered.publicKeyZ32.prefix(8))"
            let contact = Contact(
                publicKeyZ32: discovered.publicKeyZ32,
                name: name,
                notes: discovered.hasPaymentMethods ? "Discovered from follows" : nil
            )
            do {
                try addContact(contact)
            } catch {
                self.error = "Failed to import contact: \(error.localizedDescription)"
            }
        }
        showingDiscoveryResults = false
    }
}

// Note: DiscoveredContact model is now in Models/DiscoveredContact.swift

/// View for displaying discovered contacts and allowing import
struct DiscoveryResultsView: View {
    let contacts: [DiscoveredContact]
    let onImport: ([DiscoveredContact]) -> Void
    let onDismiss: () -> Void
    
    @State private var selectedContacts: Set<String> = []
    @State private var isImporting = false
    
    var body: some View {
        NavigationView {
            Group {
                if contacts.isEmpty {
                    emptyStateView
                } else {
                    discoveryList
                }
            }
            .navigationTitle("Discovered Contacts")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        onDismiss()
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Import") {
                        importSelected()
                    }
                    .disabled(selectedContacts.isEmpty || isImporting)
                }
            }
        }
    }
    
    private var emptyStateView: some View {
        VStack(spacing: 20) {
            Image(systemName: "person.badge.plus")
                .font(.system(size: 80))
                .foregroundColor(.secondary)
            
            Text("No New Contacts")
                .font(.title2)
                .fontWeight(.semibold)
            
            Text("No new contacts found in your follows list.\nAll contacts may already be imported.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
        }
        .padding()
    }
    
    private var discoveryList: some View {
        List {
            Section {
                Text("Select contacts to import")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            ForEach(contacts) { contact in
                DiscoveredContactRow(
                    contact: contact,
                    isSelected: selectedContacts.contains(contact.id),
                    onToggle: {
                        if selectedContacts.contains(contact.id) {
                            selectedContacts.remove(contact.id)
                        } else {
                            selectedContacts.insert(contact.id)
                        }
                    }
                )
            }
        }
    }
    
    private func importSelected() {
        isImporting = true
        let toImport = contacts.filter { selectedContacts.contains($0.id) }
        onImport(toImport)
        isImporting = false
    }
}

struct DiscoveredContactRow: View {
    let contact: DiscoveredContact
    let isSelected: Bool
    let onToggle: () -> Void
    
    var body: some View {
        HStack(spacing: 12) {
            Button(action: onToggle) {
                Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                    .foregroundColor(isSelected ? .blue : .secondary)
            }
            .buttonStyle(.plain)
            
            Circle()
                .fill(Color.green.opacity(0.2))
                .frame(width: 44, height: 44)
                .overlay(
                    Image(systemName: "person.badge.plus")
                        .foregroundColor(.green)
                )
            
            VStack(alignment: .leading, spacing: 4) {
                Text(contact.abbreviatedKey)
                    .font(.headline)
                    .fontDesign(.monospaced)
                
                if contact.hasPaymentMethods {
                    HStack(spacing: 4) {
                        Image(systemName: "checkmark.circle.fill")
                            .font(.caption2)
                            .foregroundColor(.green)
                        Text("Has payment methods")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    
                    if !contact.supportedMethods.isEmpty {
                        Text(contact.supportedMethods.joined(separator: ", "))
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                } else {
                    Text("No payment methods")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }
            
            Spacer()
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
        .onTapGesture {
            onToggle()
        }
    }
}

#Preview {
    ContactsView()
}

