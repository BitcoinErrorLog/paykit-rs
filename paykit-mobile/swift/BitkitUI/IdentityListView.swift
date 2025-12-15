//
//  IdentityListView.swift
//  PaykitMobile
//
//  Identity Management UI component for Bitkit integration.
//  This is a template that Bitkit can adapt to their design system.
//

import SwiftUI
import PaykitMobile

/// Identity information model
public struct IdentityInfo: Identifiable {
    public let id: String
    public let name: String
    public let publicKey: String
    public let createdAt: Date
    
    public init(id: String, name: String, publicKey: String, createdAt: Date) {
        self.id = id
        self.name = name
        self.publicKey = publicKey
        self.createdAt = createdAt
    }
}

/// Identity Management view model for Bitkit integration
public class BitkitIdentityViewModel: ObservableObject {
    @Published public var identities: [IdentityInfo] = []
    @Published public var currentIdentityName: String?
    @Published public var showingCreateSheet = false
    @Published public var identityToDelete: IdentityInfo?
    @Published public var showingDeleteConfirmation = false
    @Published public var errorMessage: String?
    @Published public var showingError = false
    
    private let identityManager: IdentityManagerProtocol
    
    public init(identityManager: IdentityManagerProtocol) {
        self.identityManager = identityManager
    }
    
    func loadIdentities() {
        identities = identityManager.listIdentities()
        currentIdentityName = identityManager.getCurrentIdentityName()
    }
    
    func switchToIdentity(_ name: String) throws {
        try identityManager.switchIdentity(name: name)
        currentIdentityName = name
        loadIdentities()
    }
    
    func createIdentity(name: String) throws {
        try identityManager.createIdentity(name: name)
        loadIdentities()
    }
    
    func deleteIdentity(_ name: String) throws {
        try identityManager.deleteIdentity(name: name)
        loadIdentities()
    }
}

/// Protocol for identity management that Bitkit must implement
public protocol IdentityManagerProtocol {
    func listIdentities() -> [IdentityInfo]
    func getCurrentIdentityName() -> String?
    func switchIdentity(name: String) throws
    func createIdentity(name: String) throws
    func deleteIdentity(name: String) throws
}

/// Identity Management view component
public struct BitkitIdentityListView: View {
    @ObservedObject var viewModel: BitkitIdentityViewModel
    @Environment(\.dismiss) private var dismiss
    
    public init(viewModel: BitkitIdentityViewModel) {
        self.viewModel = viewModel
    }
    
    public var body: some View {
        NavigationView {
            List {
                ForEach(viewModel.identities) { identity in
                    IdentityRowView(
                        identity: identity,
                        isCurrent: viewModel.currentIdentityName == identity.name,
                        onSwitch: {
                            do {
                                try viewModel.switchToIdentity(identity.name)
                            } catch {
                                viewModel.errorMessage = error.localizedDescription
                                viewModel.showingError = true
                            }
                        },
                        onDelete: {
                            viewModel.identityToDelete = identity
                            viewModel.showingDeleteConfirmation = true
                        }
                    )
                }
            }
            .navigationTitle("Identities")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Create") {
                        viewModel.showingCreateSheet = true
                    }
                }
            }
            .sheet(isPresented: $viewModel.showingCreateSheet) {
                CreateIdentitySheet(viewModel: viewModel)
            }
            .confirmationDialog(
                "Delete Identity",
                isPresented: $viewModel.showingDeleteConfirmation,
                presenting: viewModel.identityToDelete
            ) { identity in
                Button("Delete", role: .destructive) {
                    do {
                        try viewModel.deleteIdentity(identity.name)
                    } catch {
                        viewModel.errorMessage = error.localizedDescription
                        viewModel.showingError = true
                    }
                }
            } message: { identity in
                Text("Are you sure? This will delete all data for '\(identity.name)'. This cannot be undone.")
            }
            .alert("Error", isPresented: $viewModel.showingError) {
                Button("OK") { }
            } message: {
                Text(viewModel.errorMessage ?? "An unknown error occurred")
            }
            .onAppear {
                viewModel.loadIdentities()
            }
        }
    }
}

struct IdentityRowView: View {
    let identity: IdentityInfo
    let isCurrent: Bool
    let onSwitch: () -> Void
    let onDelete: () -> Void
    
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(identity.name)
                        .font(.headline)
                    if isCurrent {
                        Text("(Current)")
                            .font(.caption)
                            .foregroundColor(.green)
                    }
                }
                Text(identity.publicKey)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            if !isCurrent {
                Button("Switch") {
                    onSwitch()
                }
                .buttonStyle(.bordered)
            }
            
            Button(role: .destructive, action: onDelete) {
                Image(systemName: "trash")
            }
        }
    }
}

struct CreateIdentitySheet: View {
    @ObservedObject var viewModel: BitkitIdentityViewModel
    @Environment(\.dismiss) private var dismiss
    @State private var identityName = ""
    
    var body: some View {
        NavigationView {
            Form {
                Section("Identity Name") {
                    TextField("Name", text: $identityName)
                }
                
                Section {
                    Button("Create") {
                        do {
                            try viewModel.createIdentity(name: identityName)
                            dismiss()
                        } catch {
                            viewModel.errorMessage = error.localizedDescription
                            viewModel.showingError = true
                        }
                    }
                    .disabled(identityName.isEmpty)
                }
            }
            .navigationTitle("Create Identity")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                }
            }
        }
    }
}
