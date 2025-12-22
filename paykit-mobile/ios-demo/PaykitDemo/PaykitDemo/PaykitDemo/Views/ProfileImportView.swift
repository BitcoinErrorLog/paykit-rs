//
//  ProfileImportView.swift
//  PaykitDemo
//
//  Import profile from Pubky-app (pubky.app) for directory publishing
//

import SwiftUI

/// View for importing profile data from Pubky-app
struct ProfileImportView: View {
    @Environment(\.dismiss) private var dismiss
    @State private var pubkyUrl = ""
    @State private var isImporting = false
    @State private var importedProfile: ImportedProfile?
    @State private var errorMessage: String?
    @State private var showingConfirmation = false
    
    let onImport: (ImportedProfile) -> Void
    
    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                if let profile = importedProfile {
                    profilePreview(profile)
                } else {
                    importForm
                }
            }
            .navigationTitle("Import Profile")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
            }
            .alert("Error", isPresented: .constant(errorMessage != nil)) {
                Button("OK") { errorMessage = nil }
            } message: {
                Text(errorMessage ?? "")
            }
        }
    }
    
    // MARK: - Import Form
    
    private var importForm: some View {
        ScrollView {
            VStack(spacing: 24) {
                // Header
                VStack(spacing: 12) {
                    Image(systemName: "person.crop.circle.badge.plus")
                        .font(.system(size: 60))
                        .foregroundColor(.blue)
                    
                    Text("Import from Pubky-app")
                        .font(.title2.bold())
                    
                    Text("Import your profile from pubky.app to use as your Paykit directory profile.")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                        .multilineTextAlignment(.center)
                        .padding(.horizontal)
                }
                .padding(.top, 24)
                
                // URL Input
                VStack(alignment: .leading, spacing: 8) {
                    Text("Pubky Profile URL or Public Key")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    TextField("pubky://z6mk... or z6mk...", text: $pubkyUrl)
                        .textFieldStyle(.roundedBorder)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                }
                .padding(.horizontal)
                
                // QR Scanner option
                HStack {
                    Rectangle()
                        .fill(Color.secondary.opacity(0.3))
                        .frame(height: 1)
                    Text("or")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Rectangle()
                        .fill(Color.secondary.opacity(0.3))
                        .frame(height: 1)
                }
                .padding(.horizontal, 40)
                
                Button {
                    // Would open QR scanner
                } label: {
                    HStack {
                        Image(systemName: "qrcode.viewfinder")
                        Text("Scan QR Code")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color(.systemGray6))
                    .cornerRadius(12)
                }
                .padding(.horizontal)
                
                Spacer().frame(height: 20)
                
                // Import button
                Button {
                    Task { await importProfile() }
                } label: {
                    HStack {
                        if isImporting {
                            ProgressView()
                                .tint(.white)
                        }
                        Text(isImporting ? "Importing..." : "Import Profile")
                    }
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(pubkyUrl.isEmpty || isImporting ? Color.gray : Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(12)
                }
                .disabled(pubkyUrl.isEmpty || isImporting)
                .padding(.horizontal)
                
                // Info card
                VStack(alignment: .leading, spacing: 8) {
                    Label("What gets imported", systemImage: "info.circle")
                        .font(.caption.bold())
                        .foregroundColor(.secondary)
                    
                    VStack(alignment: .leading, spacing: 4) {
                        importInfoRow(icon: "person.fill", text: "Display name")
                        importInfoRow(icon: "text.alignleft", text: "Bio/description")
                        importInfoRow(icon: "photo", text: "Avatar image")
                        importInfoRow(icon: "link", text: "Links and social")
                    }
                }
                .padding()
                .background(Color(.systemGray6))
                .cornerRadius(12)
                .padding(.horizontal)
            }
            .padding(.bottom, 24)
        }
    }
    
    private func importInfoRow(icon: String, text: String) -> some View {
        HStack(spacing: 8) {
            Image(systemName: icon)
                .font(.caption)
                .foregroundColor(.blue)
                .frame(width: 20)
            Text(text)
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }
    
    // MARK: - Profile Preview
    
    private func profilePreview(_ profile: ImportedProfile) -> some View {
        ScrollView {
            VStack(spacing: 24) {
                // Avatar
                Circle()
                    .fill(Color.blue.opacity(0.2))
                    .frame(width: 100, height: 100)
                    .overlay {
                        if let avatarUrl = profile.avatarUrl {
                            AsyncImage(url: URL(string: avatarUrl)) { image in
                                image.resizable().scaledToFill()
                            } placeholder: {
                                Text(String(profile.name.prefix(1)).uppercased())
                                    .font(.largeTitle.bold())
                                    .foregroundColor(.blue)
                            }
                            .clipShape(Circle())
                        } else {
                            Text(String(profile.name.prefix(1)).uppercased())
                                .font(.largeTitle.bold())
                                .foregroundColor(.blue)
                        }
                    }
                
                // Name
                Text(profile.name)
                    .font(.title2.bold())
                
                // Pubkey
                Text(abbreviate(profile.pubkey))
                    .font(.caption.monospaced())
                    .foregroundColor(.secondary)
                
                // Bio
                if let bio = profile.bio {
                    Text(bio)
                        .font(.body)
                        .foregroundColor(.secondary)
                        .multilineTextAlignment(.center)
                        .padding(.horizontal)
                }
                
                // Links
                if !profile.links.isEmpty {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Links")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        ForEach(profile.links, id: \.self) { link in
                            HStack {
                                Image(systemName: "link")
                                    .foregroundColor(.blue)
                                Text(link)
                                    .font(.subheadline)
                                Spacer()
                            }
                            .padding(.vertical, 4)
                        }
                    }
                    .padding()
                    .background(Color(.systemGray6))
                    .cornerRadius(12)
                    .padding(.horizontal)
                }
                
                Divider()
                    .padding(.vertical)
                
                // Actions
                VStack(spacing: 12) {
                    Button {
                        Task { await publishAndUseProfile(profile) }
                    } label: {
                        HStack {
                            if isImporting {
                                ProgressView()
                                    .tint(.white)
                            }
                            Text(isImporting ? "Publishing..." : "Use This Profile")
                        }
                        .font(.headline)
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(isImporting ? Color.gray : Color.blue)
                        .foregroundColor(.white)
                        .cornerRadius(12)
                    }
                    .disabled(isImporting)
                    
                    Button {
                        importedProfile = nil
                        pubkyUrl = ""
                    } label: {
                        Text("Import Different Profile")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                    .disabled(isImporting)
                }
                .padding(.horizontal)
            }
            .padding(.vertical, 24)
        }
    }
    
    // MARK: - Logic
    
    private func importProfile() async {
        isImporting = true
        errorMessage = nil
        
        // Clean up input
        var pubkey = pubkyUrl.trimmingCharacters(in: .whitespacesAndNewlines)
        if pubkey.hasPrefix("pubky://") {
            pubkey = String(pubkey.dropFirst(8))
        }
        
        // Remove any path components (e.g., pubky://pk/path -> pk)
        if let slashIndex = pubkey.firstIndex(of: "/") {
            pubkey = String(pubkey.prefix(upTo: slashIndex))
        }
        
        do {
            // Fetch profile from directory
            let directoryProfile = try await DirectoryService.shared.fetchProfile(for: pubkey)
            
            // Convert to ImportedProfile
            let profile = ImportedProfile(
                pubkey: pubkey,
                name: directoryProfile?.name ?? "Unknown",
                bio: directoryProfile?.bio,
                avatarUrl: directoryProfile?.image,
                links: directoryProfile?.links ?? []
            )
            
            await MainActor.run {
                importedProfile = profile
                isImporting = false
            }
        } catch {
            await MainActor.run {
                errorMessage = "Failed to fetch profile: \(error.localizedDescription)"
                isImporting = false
            }
        }
    }
    
    private func abbreviate(_ key: String) -> String {
        guard key.count > 16 else { return key }
        return "\(key.prefix(8))...\(key.suffix(8))"
    }
    
    private func publishAndUseProfile(_ profile: ImportedProfile) async {
        isImporting = true
        errorMessage = nil
        
        do {
            // Convert to DirectoryProfile and publish
            let directoryProfile = DirectoryProfile(
                name: profile.name,
                bio: profile.bio,
                image: profile.avatarUrl,
                status: nil,
                links: profile.links
            )
            
            try await DirectoryService.shared.publishProfile(directoryProfile)
            
            await MainActor.run {
                isImporting = false
                onImport(profile)
                dismiss()
            }
        } catch {
            await MainActor.run {
                errorMessage = "Failed to publish profile: \(error.localizedDescription)"
                isImporting = false
            }
        }
    }
}

// MARK: - Imported Profile

struct ImportedProfile {
    let pubkey: String
    let name: String
    let bio: String?
    let avatarUrl: String?
    let links: [String]
}

#Preview {
    ProfileImportView { profile in
        print("Imported: \(profile.name)")
    }
}

