//
//  SessionManagementView.swift
//  PaykitDemo
//
//  View and manage active Pubky sessions
//

import SwiftUI

/// View for managing active Pubky Ring sessions
struct SessionManagementView: View {
    @ObservedObject private var bridge = PubkyRingBridge.shared
    @State private var showingRevokeConfirmation = false
    @State private var sessionToRevoke: String?
    @State private var showingAuthSheet = false
    
    var body: some View {
        List {
            // Current session section
            Section {
                if let session = bridge.currentSession {
                    currentSessionRow(session)
                } else {
                    noActiveSessionRow
                }
            } header: {
                Text("Current Session")
            } footer: {
                Text("Your active Pubky Ring session is used for directory operations and identity verification.")
            }
            
            // Session details section
            if let session = bridge.currentSession {
                Section {
                    detailRow(title: "Public Key", value: abbreviate(session.pubkey))
                    detailRow(title: "Created", value: formatDate(session.createdAt))
                    
                    if let expires = session.expiresAt {
                        detailRow(title: "Expires", value: formatDate(expires))
                    } else {
                        detailRow(title: "Expires", value: "Never")
                    }
                    
                    if !session.capabilities.isEmpty {
                        capabilitiesRow(session.capabilities)
                    }
                } header: {
                    Text("Session Details")
                }
                
                // Actions section
                Section {
                    Button {
                        refreshSession()
                    } label: {
                        HStack {
                            Image(systemName: "arrow.clockwise")
                            Text("Refresh Session")
                        }
                    }
                    
                    Button(role: .destructive) {
                        showingRevokeConfirmation = true
                    } label: {
                        HStack {
                            Image(systemName: "xmark.circle")
                            Text("Disconnect")
                        }
                    }
                } header: {
                    Text("Actions")
                }
            }
            
            // Device info section
            Section {
                detailRow(title: "Device ID", value: abbreviate(bridge.deviceId))
                detailRow(title: "Pubky Ring Status", value: bridge.isPubkyRingInstalled ? "Installed" : "Not Installed")
            } header: {
                Text("Device Info")
            } footer: {
                Text("The Device ID is used for deriving Noise protocol keys and persists across sessions.")
            }
        }
        .navigationTitle("Sessions")
        .toolbar {
            if bridge.currentSession == nil {
                ToolbarItem(placement: .primaryAction) {
                    Button {
                        showingAuthSheet = true
                    } label: {
                        Image(systemName: "plus.circle")
                    }
                }
            }
        }
        .alert("Disconnect Session?", isPresented: $showingRevokeConfirmation) {
            Button("Cancel", role: .cancel) {}
            Button("Disconnect", role: .destructive) {
                bridge.disconnect()
            }
        } message: {
            Text("This will end your current session. You'll need to authenticate again to use Pubky features.")
        }
        .sheet(isPresented: $showingAuthSheet) {
            PubkyRingAuthView { session in
                // Session received, view updates automatically
            }
        }
    }
    
    // MARK: - Session Rows
    
    private func currentSessionRow(_ session: PubkySession) -> some View {
        HStack(spacing: 12) {
            Image(systemName: "checkmark.shield.fill")
                .foregroundColor(.green)
                .font(.title2)
            
            VStack(alignment: .leading, spacing: 4) {
                Text("Active Session")
                    .font(.headline)
                
                Text(abbreviate(session.pubkey))
                    .font(.caption.monospaced())
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            if session.isExpired {
                Text("Expired")
                    .font(.caption.bold())
                    .foregroundColor(.white)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.red)
                    .cornerRadius(4)
            } else {
                Text("Active")
                    .font(.caption.bold())
                    .foregroundColor(.white)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.green)
                    .cornerRadius(4)
            }
        }
        .padding(.vertical, 4)
    }
    
    private var noActiveSessionRow: some View {
        HStack(spacing: 12) {
            Image(systemName: "shield.slash")
                .foregroundColor(.gray)
                .font(.title2)
            
            VStack(alignment: .leading, spacing: 4) {
                Text("No Active Session")
                    .font(.headline)
                
                Text("Connect to Pubky Ring to enable directory features")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            Button {
                showingAuthSheet = true
            } label: {
                Text("Connect")
                    .font(.caption.bold())
                    .foregroundColor(.white)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(Color.blue)
                    .cornerRadius(8)
            }
        }
        .padding(.vertical, 4)
    }
    
    private func detailRow(title: String, value: String) -> some View {
        HStack {
            Text(title)
                .foregroundColor(.secondary)
            Spacer()
            Text(value)
                .font(.system(.body, design: .monospaced))
        }
    }
    
    private func capabilitiesRow(_ capabilities: [String]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Capabilities")
                .foregroundColor(.secondary)
            
            FlowLayout(spacing: 8) {
                ForEach(capabilities, id: \.self) { capability in
                    Text(capability)
                        .font(.caption)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Color.blue.opacity(0.1))
                        .foregroundColor(.blue)
                        .cornerRadius(4)
                }
            }
        }
    }
    
    // MARK: - Actions
    
    private func refreshSession() {
        Task {
            do {
                _ = try await bridge.requestSession()
            } catch {
                print("Failed to refresh session: \(error)")
            }
        }
    }
    
    // MARK: - Helpers
    
    private func abbreviate(_ key: String) -> String {
        guard key.count > 16 else { return key }
        return "\(key.prefix(8))...\(key.suffix(8))"
    }
    
    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
}

// MARK: - Flow Layout

struct FlowLayout: Layout {
    var spacing: CGFloat = 8
    
    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let result = arrange(proposal: proposal, subviews: subviews)
        return result.size
    }
    
    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let result = arrange(proposal: proposal, subviews: subviews)
        
        for (index, position) in result.positions.enumerated() {
            subviews[index].place(
                at: CGPoint(x: bounds.minX + position.x, y: bounds.minY + position.y),
                proposal: .unspecified
            )
        }
    }
    
    private func arrange(proposal: ProposedViewSize, subviews: Subviews) -> (size: CGSize, positions: [CGPoint]) {
        let maxWidth = proposal.width ?? .infinity
        var positions: [CGPoint] = []
        var x: CGFloat = 0
        var y: CGFloat = 0
        var rowHeight: CGFloat = 0
        var totalHeight: CGFloat = 0
        
        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            
            if x + size.width > maxWidth && x > 0 {
                x = 0
                y += rowHeight + spacing
                rowHeight = 0
            }
            
            positions.append(CGPoint(x: x, y: y))
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
            totalHeight = max(totalHeight, y + size.height)
        }
        
        return (CGSize(width: maxWidth, height: totalHeight), positions)
    }
}

#Preview {
    NavigationStack {
        SessionManagementView()
    }
}

