//
//  ContactDiscoveryView.swift
//  PaykitDemo
//
//  Discover contacts from Pubky follows directory with health indicators
//

import SwiftUI

struct ContactDiscoveryView: View {
    @StateObject private var viewModel = ContactDiscoveryViewModel()
    @State private var searchQuery = ""
    @State private var filterMethod: String? = nil
    @State private var showingFilters = false
    @State private var selectedContact: DiscoveredContact? = nil
    
    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                // Search bar
                searchSection
                    .padding()
                
                // Method filters
                if showingFilters {
                    methodFiltersSection
                        .padding(.horizontal)
                        .padding(.bottom)
                }
                
                // Content
                if viewModel.isLoading {
                    Spacer()
                    ProgressView("Discovering contacts...")
                    Spacer()
                } else if filteredContacts.isEmpty {
                    emptyStateView
                } else {
                    // Health summary
                    healthSummaryCard
                        .padding(.horizontal)
                    
                    // Contacts list
                    List {
                        ForEach(filteredContacts) { contact in
                            DiscoveredContactRow(contact: contact) {
                                addContact(contact)
                            }
                        }
                    }
                    .listStyle(.plain)
                }
            }
            .navigationTitle("Discover Contacts")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    HStack {
                        Button {
                            showingFilters.toggle()
                        } label: {
                            Image(systemName: "line.3.horizontal.decrease.circle")
                                .foregroundColor(filterMethod != nil ? .blue : .gray)
                        }
                        
                        Button {
                            viewModel.refreshDiscovery()
                        } label: {
                            Image(systemName: "arrow.clockwise")
                        }
                    }
                }
            }
            .refreshable {
                viewModel.refreshDiscovery()
            }
            .onAppear {
                viewModel.loadFollows()
            }
            .sheet(item: $selectedContact) { contact in
                ContactDetailSheet(contact: contact, onAdd: {
                    addContact(contact)
                })
            }
        }
    }
    
    // MARK: - Components
    
    private var searchSection: some View {
        HStack {
            Image(systemName: "magnifyingglass")
                .foregroundColor(.gray)
            
            TextField("Search by name or pubkey", text: $searchQuery)
            
            if !searchQuery.isEmpty {
                Button {
                    searchQuery = ""
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundColor(.gray)
                }
            }
        }
        .padding(12)
        .background(Color(.systemGray6))
        .cornerRadius(10)
    }
    
    private var methodFiltersSection: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                FilterChip(title: "All", isSelected: filterMethod == nil) {
                    filterMethod = nil
                }
                FilterChip(title: "⚡ Lightning", isSelected: filterMethod == "lightning") {
                    filterMethod = "lightning"
                }
                FilterChip(title: "₿ On-chain", isSelected: filterMethod == "onchain") {
                    filterMethod = "onchain"
                }
                FilterChip(title: "📡 Noise", isSelected: filterMethod == "noise") {
                    filterMethod = "noise"
                }
            }
        }
    }
    
    private var healthSummaryCard: some View {
        VStack(spacing: 12) {
            HStack {
                Image(systemName: viewModel.directoryHealthIcon)
                    .foregroundColor(viewModel.directoryHealthColor)
                    .font(.title2)
                
                VStack(alignment: .leading, spacing: 2) {
                    Text("Directory Status")
                        .font(.headline)
                    
                    Text("\(filteredContacts.count) contacts • \(viewModel.totalHealthyEndpoints) healthy endpoints")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                Text(viewModel.lastSyncText)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            // Method breakdown
            HStack(spacing: 16) {
                MethodHealthPill(
                    icon: "bolt.fill",
                    healthy: viewModel.healthyLightningEndpoints,
                    total: viewModel.lightningEndpoints
                )
                MethodHealthPill(
                    icon: "bitcoinsign.circle.fill",
                    healthy: viewModel.healthyOnchainEndpoints,
                    total: viewModel.onchainEndpoints
                )
                MethodHealthPill(
                    icon: "antenna.radiowaves.left.and.right",
                    healthy: viewModel.healthyNoiseEndpoints,
                    total: viewModel.noiseEndpoints
                )
                Spacer()
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
    
    private var emptyStateView: some View {
        VStack(spacing: 20) {
            Spacer()
            
            Image(systemName: "person.crop.circle.badge.questionmark")
                .font(.system(size: 80))
                .foregroundColor(.gray)
            
            Text("No Contacts Found")
                .font(.title2.bold())
            
            Text(filterMethod != nil ?
                 "No contacts with \(filterMethod!) endpoints found" :
                 "No contacts with payment endpoints found in your follows directory")
                .font(.body)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
            
            Button {
                filterMethod = nil
                viewModel.refreshDiscovery()
            } label: {
                Text("Refresh Discovery")
                    .foregroundColor(.blue)
            }
            
            Spacer()
        }
    }
    
    private var filteredContacts: [DiscoveredContact] {
        var results = viewModel.discoveredContacts
        
        if !searchQuery.isEmpty {
            let query = searchQuery.lowercased()
            results = results.filter {
                ($0.name?.lowercased().contains(query) ?? false) ||
                $0.pubkey.lowercased().contains(query)
            }
        }
        
        if let method = filterMethod {
            results = results.filter { $0.supportedMethods.contains(method) }
        }
        
        return results
    }
    
    private func addContact(_ contact: DiscoveredContact) {
        do {
            try viewModel.addContact(contact)
            selectedContact = nil
        } catch {
            print("Failed to add contact: \(error)")
        }
    }
}

// MARK: - Discovered Contact Row

struct DiscoveredContactRow: View {
    let contact: DiscoveredContact
    let onAdd: () -> Void
    
    var body: some View {
        HStack(spacing: 12) {
            // Avatar
            Circle()
                .fill(Color.blue.opacity(0.2))
                .frame(width: 48, height: 48)
                .overlay {
                    Text(String((contact.name ?? contact.pubkey).prefix(1)).uppercased())
                        .foregroundColor(.blue)
                        .font(.headline)
                }
            
            VStack(alignment: .leading, spacing: 4) {
                Text(contact.name ?? contact.pubkey)
                    .font(.headline)
                
                Text(contact.abbreviatedPubkey)
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                // Method indicators
                HStack(spacing: 8) {
                    ForEach(contact.supportedMethods, id: \.self) { method in
                        MethodIndicator(method: method, isHealthy: contact.isMethodHealthy(method))
                    }
                }
            }
            
            Spacer()
            
            // Health status
            VStack(spacing: 4) {
                Image(systemName: contact.healthIcon)
                    .foregroundColor(contact.healthColor)
                    .font(.title3)
                
                Text(contact.healthStatus)
                    .font(.caption2)
                    .foregroundColor(contact.healthColor)
            }
            
            // Add button
            Button(action: onAdd) {
                Image(systemName: "plus.circle.fill")
                    .foregroundColor(.blue)
                    .font(.title2)
            }
        }
        .padding(.vertical, 8)
    }
}

struct MethodIndicator: View {
    let method: String
    let isHealthy: Bool
    
    var body: some View {
        HStack(spacing: 2) {
            Image(systemName: methodIcon)
                .font(.caption2)
            Circle()
                .fill(isHealthy ? Color.green : Color.red)
                .frame(width: 6, height: 6)
        }
        .foregroundColor(isHealthy ? .green : .red)
    }
    
    private var methodIcon: String {
        switch method {
        case "lightning": return "bolt.fill"
        case "onchain": return "bitcoinsign.circle.fill"
        case "noise": return "antenna.radiowaves.left.and.right"
        default: return "creditcard.fill"
        }
    }
}

struct MethodHealthPill: View {
    let icon: String
    let healthy: Int
    let total: Int
    
    var body: some View {
        HStack(spacing: 4) {
            Image(systemName: icon)
                .font(.caption)
            Text("\(healthy)/\(total)")
                .font(.caption)
        }
        .foregroundColor(pillColor)
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(pillColor.opacity(0.2))
        .cornerRadius(12)
    }
    
    private var pillColor: Color {
        if total == 0 { return .gray }
        if healthy == total { return .green }
        if healthy > 0 { return .orange }
        return .red
    }
}

struct FilterChip: View {
    let title: String
    let isSelected: Bool
    let action: () -> Void
    
    var body: some View {
        Button(action: action) {
            Text(title)
                .font(.caption)
                .foregroundColor(isSelected ? .white : .secondary)
                .padding(.horizontal, 12)
                .padding(.vertical, 6)
                .background(isSelected ? Color.blue : Color(.systemGray5))
                .cornerRadius(16)
        }
    }
}

// MARK: - Contact Detail Sheet

struct ContactDetailSheet: View {
    let contact: DiscoveredContact
    let onAdd: () -> Void
    @Environment(\.dismiss) private var dismiss
    
    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 24) {
                    // Header
                    VStack(spacing: 12) {
                        Circle()
                            .fill(Color.blue.opacity(0.2))
                            .frame(width: 80, height: 80)
                            .overlay {
                                Text(String((contact.name ?? contact.pubkey).prefix(1)).uppercased())
                                    .foregroundColor(.blue)
                                    .font(.largeTitle)
                            }
                        
                        Text(contact.name ?? "Unknown")
                            .font(.title2.bold())
                        
                        Button {
                            UIPasteboard.general.string = contact.pubkey
                        } label: {
                            HStack(spacing: 4) {
                                Text(contact.abbreviatedPubkey)
                                Image(systemName: "doc.on.doc")
                                    .font(.caption)
                            }
                            .foregroundColor(.blue)
                        }
                        
                        // Health status
                        HStack(spacing: 4) {
                            Circle()
                                .fill(contact.healthColor)
                                .frame(width: 8, height: 8)
                            Text(contact.healthStatus)
                                .foregroundColor(contact.healthColor)
                        }
                    }
                    
                    // Endpoints
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Payment Endpoints")
                            .font(.headline)
                            .foregroundColor(.secondary)
                        
                        VStack(spacing: 0) {
                            ForEach(contact.supportedMethods, id: \.self) { method in
                                EndpointRow(
                                    method: method,
                                    isHealthy: contact.isMethodHealthy(method)
                                )
                                
                                if method != contact.supportedMethods.last {
                                    Divider()
                                }
                            }
                        }
                        .background(Color(.systemGray6))
                        .cornerRadius(12)
                    }
                    .padding(.horizontal)
                    
                    // Actions
                    VStack(spacing: 12) {
                        Button {
                            onAdd()
                            dismiss()
                        } label: {
                            HStack {
                                Image(systemName: "person.badge.plus")
                                Text("Add to Contacts")
                            }
                            .font(.headline)
                            .foregroundColor(.white)
                            .frame(maxWidth: .infinity)
                            .padding()
                            .background(Color.blue)
                            .cornerRadius(12)
                        }
                    }
                    .padding(.horizontal)
                }
                .padding(.vertical)
            }
            .navigationTitle("Contact Details")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close") {
                        dismiss()
                    }
                }
            }
        }
    }
}

struct EndpointRow: View {
    let method: String
    let isHealthy: Bool
    
    var body: some View {
        HStack {
            Image(systemName: methodIcon)
                .foregroundColor(isHealthy ? .green : .red)
                .frame(width: 28)
            
            Text(methodName)
            
            Spacer()
            
            HStack(spacing: 4) {
                Circle()
                    .fill(isHealthy ? Color.green : Color.red)
                    .frame(width: 8, height: 8)
                Text(isHealthy ? "Healthy" : "Unreachable")
                    .font(.caption)
                    .foregroundColor(isHealthy ? .green : .red)
            }
        }
        .padding()
    }
    
    private var methodIcon: String {
        switch method {
        case "lightning": return "bolt.fill"
        case "onchain": return "bitcoinsign.circle.fill"
        case "noise": return "antenna.radiowaves.left.and.right"
        default: return "creditcard.fill"
        }
    }
    
    private var methodName: String {
        switch method {
        case "lightning": return "Lightning Network"
        case "onchain": return "On-Chain Bitcoin"
        case "noise": return "Noise Protocol"
        default: return method.capitalized
        }
    }
}

// MARK: - View Model

@MainActor
class ContactDiscoveryViewModel: ObservableObject {
    @Published var discoveredContacts: [DiscoveredContact] = []
    @Published var isLoading = false
    @Published var lastSyncDate: Date?
    
    @Published var lightningEndpoints = 0
    @Published var healthyLightningEndpoints = 0
    @Published var onchainEndpoints = 0
    @Published var healthyOnchainEndpoints = 0
    @Published var noiseEndpoints = 0
    @Published var healthyNoiseEndpoints = 0
    
    var totalHealthyEndpoints: Int {
        healthyLightningEndpoints + healthyOnchainEndpoints + healthyNoiseEndpoints
    }
    
    var directoryHealthColor: Color {
        if discoveredContacts.isEmpty { return .gray }
        let total = lightningEndpoints + onchainEndpoints + noiseEndpoints
        if total == 0 { return .gray }
        let healthyPercent = Double(totalHealthyEndpoints) / Double(total)
        if healthyPercent >= 0.8 { return .green }
        if healthyPercent >= 0.5 { return .orange }
        return .red
    }
    
    var directoryHealthIcon: String {
        if discoveredContacts.isEmpty { return "antenna.radiowaves.left.and.right.slash" }
        let total = max(1, lightningEndpoints + onchainEndpoints + noiseEndpoints)
        let healthyPercent = Double(totalHealthyEndpoints) / Double(total)
        if healthyPercent >= 0.8 { return "checkmark.shield.fill" }
        if healthyPercent >= 0.5 { return "exclamationmark.shield.fill" }
        return "xmark.shield.fill"
    }
    
    var lastSyncText: String {
        guard let date = lastSyncDate else { return "Never" }
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .abbreviated
        return formatter.localizedString(for: date, relativeTo: Date())
    }
    
    private let directoryService = DirectoryService()
    private let keyManager = KeyManager()
    
    func loadFollows() {
        isLoading = true
        
        Task {
            // Simulate loading from directory
            try? await Task.sleep(nanoseconds: 1_000_000_000)
            
            // Generate mock discovered contacts for demo
            let mockContacts = generateMockContacts()
            
            await MainActor.run {
                self.discoveredContacts = mockContacts
                self.lastSyncDate = Date()
                self.updateHealthStats()
                self.isLoading = false
            }
        }
    }
    
    func refreshDiscovery() {
        loadFollows()
    }
    
    private func updateHealthStats() {
        lightningEndpoints = 0
        healthyLightningEndpoints = 0
        onchainEndpoints = 0
        healthyOnchainEndpoints = 0
        noiseEndpoints = 0
        healthyNoiseEndpoints = 0
        
        for contact in discoveredContacts {
            for method in contact.supportedMethods {
                switch method {
                case "lightning":
                    lightningEndpoints += 1
                    if contact.isMethodHealthy(method) { healthyLightningEndpoints += 1 }
                case "onchain":
                    onchainEndpoints += 1
                    if contact.isMethodHealthy(method) { healthyOnchainEndpoints += 1 }
                case "noise":
                    noiseEndpoints += 1
                    if contact.isMethodHealthy(method) { healthyNoiseEndpoints += 1 }
                default:
                    break
                }
            }
        }
    }
    
    func addContact(_ discoveredContact: DiscoveredContact) throws {
        let identityName = keyManager.currentIdentityName ?? "default"
        let storage = ContactStorage(identityName: identityName)
        
        let contact = Contact(
            publicKeyZ32: discoveredContact.pubkey,
            name: discoveredContact.name ?? discoveredContact.pubkey,
            notes: "Discovered from Pubky follows"
        )
        try storage.saveContact(contact)
        
        // Remove from discovered list
        discoveredContacts.removeAll { $0.id == discoveredContact.id }
    }
    
    private func generateMockContacts() -> [DiscoveredContact] {
        [
            DiscoveredContact(
                pubkey: "z6mktest1234567890abcdefghijklmnopqrstuvwxyz",
                name: "Alice",
                supportedMethods: ["lightning", "noise"],
                endpointHealth: ["lightning": true, "noise": true]
            ),
            DiscoveredContact(
                pubkey: "z6mkexample98765432109876543210fedcba",
                name: "Bob's Wallet",
                supportedMethods: ["lightning", "onchain"],
                endpointHealth: ["lightning": true, "onchain": false]
            ),
            DiscoveredContact(
                pubkey: "z6mkdemo555444333222111000zzzyyyxxx",
                name: nil,
                supportedMethods: ["onchain", "noise"],
                endpointHealth: ["onchain": true, "noise": true]
            ),
            DiscoveredContact(
                pubkey: "z6mknoiseonly999888777666555444333",
                name: "Charlie",
                supportedMethods: ["noise"],
                endpointHealth: ["noise": true]
            )
        ]
    }
}

// Note: DiscoveredContact model is now in Models/DiscoveredContact.swift

// MARK: - Preview

#Preview {
    ContactDiscoveryView()
}

