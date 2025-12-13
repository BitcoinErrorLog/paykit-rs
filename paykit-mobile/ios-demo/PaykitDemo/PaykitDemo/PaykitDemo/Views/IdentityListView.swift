//
//  IdentityListView.swift
//  PaykitDemo
//
//  Identity management view for creating, switching, and deleting identities
//

import SwiftUI

struct IdentityListView: View {
    @StateObject private var keyManager = KeyManager()
    @State private var identities: [IdentityInfo] = []
    @State private var showingCreateSheet = false
    @State private var identityToDelete: IdentityInfo?
    @State private var showingDeleteConfirmation = false
    @State private var errorMessage: String?
    @State private var showingError = false
    @Environment(\.dismiss) private var dismiss
    
    var body: some View {
        List {
            ForEach(identities) { identity in
                IdentityRowView(
                    identity: identity,
                    isCurrent: keyManager.currentIdentityName == identity.name,
                    onSwitch: { switchToIdentity(identity.name) },
                    onDelete: { identityToDelete = identity }
                )
            }
        }
        .navigationTitle("Identities")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button("Create") {
                    showingCreateSheet = true
                }
            }
        }
        .sheet(isPresented: $showingCreateSheet) {
            CreateIdentityView(onCreated: { loadIdentities() })
        }
        .confirmationDialog(
            "Delete Identity",
            isPresented: $showingDeleteConfirmation,
            presenting: identityToDelete
        ) { identity in
            Button("Delete", role: .destructive) {
                deleteIdentity(identity.name)
            }
        } message: { identity in
            Text("Are you sure? This will delete all data for '\(identity.name)'. This cannot be undone.")
        }
        .alert("Error", isPresented: $showingError) {
            Button("OK") { }
        } message: {
            Text(errorMessage ?? "An unknown error occurred")
        }
        .onAppear {
            loadIdentities()
        }
        .onReceive(NotificationCenter.default.publisher(for: .identityDidChange)) { _ in
            loadIdentities()
        }
    }
    
    private func loadIdentities() {
        identities = keyManager.listIdentities()
    }
    
    private func switchToIdentity(_ name: String) {
        do {
            try keyManager.switchIdentity(name: name)
            loadIdentities()
        } catch {
            errorMessage = error.localizedDescription
            showingError = true
        }
    }
    
    private func deleteIdentity(_ name: String) {
        do {
            try keyManager.deleteIdentity(name: name)
            loadIdentities()
        } catch {
            errorMessage = error.localizedDescription
            showingError = true
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
                    Text(identity.nickname ?? identity.name)
                        .font(.headline)
                    if isCurrent {
                        Text("(Current)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                
                Text(identity.publicKeyZ32)
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                
                if let nickname = identity.nickname, nickname != identity.name {
                    Text(identity.name)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            Spacer()
            
            if !isCurrent {
                Button("Switch") {
                    onSwitch()
                }
                .buttonStyle(.bordered)
            }
            
            Menu {
                if !isCurrent {
                    Button("Switch to This Identity") {
                        onSwitch()
                    }
                }
                Button("Delete", role: .destructive) {
                    onDelete()
                }
            } label: {
                Image(systemName: "ellipsis")
            }
        }
        .padding(.vertical, 4)
    }
}

struct CreateIdentityView: View {
    @State private var name: String = ""
    @State private var nickname: String = ""
    @State private var errorMessage: String?
    @State private var showingError = false
    @Environment(\.dismiss) private var dismiss
    let onCreated: () -> Void
    
    var body: some View {
        NavigationView {
            Form {
                Section {
                    TextField("Identity Name", text: $name)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                    
                    TextField("Nickname (Optional)", text: $nickname)
                } header: {
                    Text("Identity Details")
                } footer: {
                    Text("The identity name must be unique and cannot be changed later.")
                }
            }
            .navigationTitle("Create Identity")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Create") {
                        createIdentity()
                    }
                    .disabled(name.isEmpty)
                }
            }
            .alert("Error", isPresented: $showingError) {
                Button("OK") { }
            } message: {
                Text(errorMessage ?? "An unknown error occurred")
            }
        }
    }
    
    private func createIdentity() {
        do {
            let keyManager = KeyManager()
            _ = try keyManager.createIdentity(
                name: name.trimmingCharacters(in: .whitespacesAndNewlines),
                nickname: nickname.isEmpty ? nil : nickname.trimmingCharacters(in: .whitespacesAndNewlines)
            )
            onCreated()
            dismiss()
        } catch {
            errorMessage = error.localizedDescription
            showingError = true
        }
    }
}

