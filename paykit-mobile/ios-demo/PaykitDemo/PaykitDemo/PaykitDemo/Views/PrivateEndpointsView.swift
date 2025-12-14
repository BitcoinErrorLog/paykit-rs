//
//  PrivateEndpointsView.swift
//  PaykitDemo
//
//  View for managing private payment endpoints exchanged with peers
//

import SwiftUI
import PaykitMobile

struct PrivateEndpointsView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var viewModel = PrivateEndpointsViewModel()
    @State private var showingCleanupAlert = false
    
    var body: some View {
        NavigationView {
            Group {
                if viewModel.isLoading {
                    ProgressView("Loading endpoints...")
                } else if viewModel.peers.isEmpty {
                    EmptyEndpointsView()
                } else {
                    EndpointsList(viewModel: viewModel)
                }
            }
            .navigationTitle("Private Endpoints")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Menu {
                        Button(action: { viewModel.refresh(identityName: appState.currentIdentityName) }) {
                            Label("Refresh", systemImage: "arrow.clockwise")
                        }
                        
                        Button(action: { showingCleanupAlert = true }) {
                            Label("Cleanup Expired", systemImage: "trash")
                        }
                        
                        Divider()
                        
                        Button(role: .destructive, action: { viewModel.clearAll(identityName: appState.currentIdentityName) }) {
                            Label("Clear All", systemImage: "trash.fill")
                        }
                    } label: {
                        Image(systemName: "ellipsis.circle")
                    }
                }
            }
            .alert("Cleanup Expired", isPresented: $showingCleanupAlert) {
                Button("Cancel", role: .cancel) { }
                Button("Cleanup") {
                    viewModel.cleanupExpired(identityName: appState.currentIdentityName)
                }
            } message: {
                Text("Remove all expired private endpoints?")
            }
        }
        .onAppear {
            viewModel.refresh(identityName: appState.currentIdentityName)
        }
    }
}

// MARK: - Empty State View

private struct EmptyEndpointsView: View {
    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "lock.shield")
                .font(.system(size: 60))
                .foregroundColor(.secondary)
            
            Text("No Private Endpoints")
                .font(.title2)
                .fontWeight(.semibold)
            
            Text("Private endpoints are exchanged during secure payment sessions for enhanced privacy.")
                .font(.body)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 40)
            
            VStack(alignment: .leading, spacing: 8) {
                Label("Per-peer dedicated addresses", systemImage: "person.2")
                Label("Automatic expiration", systemImage: "clock")
                Label("Encrypted storage", systemImage: "lock.fill")
            }
            .font(.footnote)
            .foregroundColor(.secondary)
            .padding(.top, 20)
        }
        .padding()
    }
}

// MARK: - Endpoints List View

private struct EndpointsList: View {
    @ObservedObject var viewModel: PrivateEndpointsViewModel
    
    var body: some View {
        List {
            // Statistics Section
            Section {
                HStack {
                    StatCard(title: "Total", value: "\(viewModel.totalCount)", icon: "link")
                    StatCard(title: "Peers", value: "\(viewModel.peers.count)", icon: "person.2")
                    StatCard(title: "Expired", value: "\(viewModel.expiredCount)", icon: "clock.badge.exclamationmark", isWarning: viewModel.expiredCount > 0)
                }
                .listRowInsets(EdgeInsets())
                .listRowBackground(Color.clear)
            }
            
            // Peers Section
            ForEach(viewModel.peers, id: \.self) { peer in
                Section {
                    let endpoints = viewModel.endpointsForPeer(peer)
                    ForEach(endpoints, id: \.methodId) { endpoint in
                        EndpointRow(endpoint: endpoint, peer: peer)
                            .swipeActions(edge: .trailing, allowsFullSwipe: true) {
                                Button(role: .destructive) {
                                    viewModel.removeEndpoint(peer: peer, methodId: endpoint.methodId)
                                } label: {
                                    Label("Delete", systemImage: "trash")
                                }
                            }
                    }
                } header: {
                    PeerHeader(peer: peer)
                }
            }
        }
        .listStyle(.insetGrouped)
    }
}

// MARK: - Stat Card

private struct StatCard: View {
    let title: String
    let value: String
    let icon: String
    var isWarning: Bool = false
    
    var body: some View {
        VStack(spacing: 4) {
            Image(systemName: icon)
                .font(.title2)
                .foregroundColor(isWarning ? .orange : .accentColor)
            
            Text(value)
                .font(.title3)
                .fontWeight(.bold)
            
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 12)
        .background(Color(.secondarySystemBackground))
        .cornerRadius(10)
    }
}

// MARK: - Peer Header

private struct PeerHeader: View {
    let peer: String
    
    var body: some View {
        HStack {
            Image(systemName: "person.circle.fill")
                .foregroundColor(.accentColor)
            Text(truncatedPeer)
                .font(.footnote)
                .fontWeight(.medium)
        }
    }
    
    private var truncatedPeer: String {
        if peer.count > 16 {
            return "\(peer.prefix(8))...\(peer.suffix(8))"
        }
        return peer
    }
}

// MARK: - Endpoint Row

private struct EndpointRow: View {
    let endpoint: PrivateEndpointOffer
    let peer: String
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                MethodBadge(methodId: endpoint.methodId)
                Spacer()
                StatusBadge(isExpired: false) // TODO: Check expiration when available
            }
            
            Text(truncatedEndpoint)
                .font(.caption)
                .foregroundColor(.secondary)
                .lineLimit(1)
        }
        .padding(.vertical, 4)
    }
    
    private var truncatedEndpoint: String {
        if endpoint.endpoint.count > 40 {
            return "\(endpoint.endpoint.prefix(20))...\(endpoint.endpoint.suffix(15))"
        }
        return endpoint.endpoint
    }
}

// MARK: - Method Badge

private struct MethodBadge: View {
    let methodId: String
    
    var body: some View {
        HStack(spacing: 4) {
            Image(systemName: iconForMethod)
                .font(.caption)
            Text(methodId)
                .font(.caption)
                .fontWeight(.medium)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(colorForMethod.opacity(0.2))
        .foregroundColor(colorForMethod)
        .cornerRadius(6)
    }
    
    private var iconForMethod: String {
        switch methodId.lowercased() {
        case "lightning":
            return "bolt.fill"
        case "onchain":
            return "bitcoinsign.circle.fill"
        default:
            return "creditcard.fill"
        }
    }
    
    private var colorForMethod: Color {
        switch methodId.lowercased() {
        case "lightning":
            return .orange
        case "onchain":
            return .yellow
        default:
            return .blue
        }
    }
}

// MARK: - Status Badge

private struct StatusBadge: View {
    let isExpired: Bool
    
    var body: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(isExpired ? Color.red : Color.green)
                .frame(width: 8, height: 8)
            Text(isExpired ? "Expired" : "Active")
                .font(.caption2)
                .foregroundColor(isExpired ? .red : .green)
        }
    }
}

// MARK: - View Model

@MainActor
class PrivateEndpointsViewModel: ObservableObject {
    @Published var peers: [String] = []
    @Published var endpoints: [String: [PrivateEndpointOffer]] = [:]
    @Published var isLoading = false
    @Published var errorMessage: String?
    
    private var storage: PrivateEndpointStorage?
    
    var totalCount: Int {
        endpoints.values.reduce(0) { $0 + $1.count }
    }
    
    var expiredCount: Int {
        // TODO: Implement when PrivateEndpointOffer has expiration
        0
    }
    
    func endpointsForPeer(_ peer: String) -> [PrivateEndpointOffer] {
        endpoints[peer] ?? []
    }
    
    func refresh(identityName: String?) {
        guard let identity = identityName else { return }
        
        isLoading = true
        storage = PrivateEndpointStorage(identityName: identity)
        
        peers = storage?.listPeers() ?? []
        endpoints = [:]
        for peer in peers {
            endpoints[peer] = storage?.listForPeer(peer) ?? []
        }
        
        isLoading = false
    }
    
    func removeEndpoint(peer: String, methodId: String) {
        do {
            try storage?.remove(peerPubkey: peer, methodId: methodId)
            
            // Update local state
            endpoints[peer]?.removeAll { $0.methodId == methodId }
            if endpoints[peer]?.isEmpty == true {
                endpoints.removeValue(forKey: peer)
                peers.removeAll { $0 == peer }
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }
    
    func cleanupExpired(identityName: String?) {
        guard let identity = identityName else { return }
        storage = PrivateEndpointStorage(identityName: identity)
        let removed = storage?.cleanupExpired() ?? 0
        if removed > 0 {
            refresh(identityName: identity)
        }
    }
    
    func clearAll(identityName: String?) {
        guard let identity = identityName else { return }
        do {
            storage = PrivateEndpointStorage(identityName: identity)
            try storage?.clearAll()
            peers = []
            endpoints = [:]
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}

// MARK: - Preview

#Preview {
    PrivateEndpointsView()
        .environmentObject(AppState())
}

