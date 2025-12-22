//
//  PubkyRingAuthView.swift
//  PaykitDemo
//
//  View for authenticating with Pubky-ring.
//  Supports same-device, cross-device (QR), and manual authentication.
//

import SwiftUI

struct PubkyRingAuthView: View {
    @Environment(\.dismiss) private var dismiss
    @ObservedObject private var bridge = PubkyRingBridge.shared
    
    @State private var selectedTab = 0
    @State private var crossDeviceRequest: CrossDeviceRequest?
    @State private var isPolling = false
    @State private var errorMessage: String?
    @State private var manualPubkey = ""
    @State private var manualSessionSecret = ""
    @State private var timeRemaining: TimeInterval = 0
    
    let onSessionReceived: (PubkySession) -> Void
    
    private let timer = Timer.publish(every: 1, on: .main, in: .common).autoconnect()
    
    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                // Tab selection
                if bridge.isPubkyRingInstalled {
                    Picker("Authentication Method", selection: $selectedTab) {
                        Text("Same Device").tag(0)
                        Text("QR Code").tag(1)
                        Text("Manual").tag(2)
                    }
                    .pickerStyle(.segmented)
                    .padding()
                } else {
                    Picker("Authentication Method", selection: $selectedTab) {
                        Text("QR Code").tag(1)
                        Text("Manual").tag(2)
                    }
                    .pickerStyle(.segmented)
                    .padding()
                    .onAppear { selectedTab = 1 }
                }
                
                // Content based on selected tab
                TabView(selection: $selectedTab) {
                    if bridge.isPubkyRingInstalled {
                        sameDeviceTab.tag(0)
                    }
                    crossDeviceTab.tag(1)
                    manualEntryTab.tag(2)
                }
                .tabViewStyle(.page(indexDisplayMode: .never))
            }
            .navigationTitle("Connect Pubky")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
            }
            .alert("Error", isPresented: .constant(errorMessage != nil)) {
                Button("OK") { errorMessage = nil }
            } message: {
                Text(errorMessage ?? "")
            }
        }
    }
    
    // MARK: - Same Device Tab
    
    private var sameDeviceTab: some View {
        VStack(spacing: 24) {
            Spacer()
            
            Image(systemName: "link.circle.fill")
                .font(.system(size: 80))
                .foregroundColor(.blue)
            
            Text("Connect with Pubky-ring")
                .font(.title2.bold())
            
            Text("Pubky-ring is installed on this device. Tap the button below to connect.")
                .font(.body)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
            
            Button {
                Task { await authenticateWithPubkyRing() }
            } label: {
                HStack {
                    Image(systemName: "arrow.right.circle.fill")
                    Text("Open Pubky-ring")
                }
                .font(.headline)
                .frame(maxWidth: .infinity)
                .padding()
                .background(Color.blue)
                .foregroundColor(.white)
                .cornerRadius(12)
            }
            .padding(.horizontal)
            
            Spacer()
        }
        .padding()
    }
    
    // MARK: - Cross Device Tab
    
    private var crossDeviceTab: some View {
        VStack(spacing: 16) {
            if let request = crossDeviceRequest, !request.isExpired {
                crossDeviceActiveView(request: request)
            } else {
                crossDeviceSetupView
            }
        }
        .padding()
        .onReceive(timer) { _ in
            if let request = crossDeviceRequest {
                timeRemaining = request.timeRemaining
                if request.isExpired {
                    crossDeviceRequest = nil
                    isPolling = false
                }
            }
        }
    }
    
    private var crossDeviceSetupView: some View {
        VStack(spacing: 24) {
            Spacer()
            
            Image(systemName: "qrcode")
                .font(.system(size: 80))
                .foregroundColor(.purple)
            
            Text("Scan from Another Device")
                .font(.title2.bold())
            
            Text("Generate a QR code to scan with a device that has Pubky-ring installed.")
                .font(.body)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
            
            Button {
                generateCrossDeviceRequest()
            } label: {
                HStack {
                    Image(systemName: "qrcode.viewfinder")
                    Text("Generate QR Code")
                }
                .font(.headline)
                .frame(maxWidth: .infinity)
                .padding()
                .background(Color.purple)
                .foregroundColor(.white)
                .cornerRadius(12)
            }
            .padding(.horizontal)
            
            Spacer()
        }
    }
    
    private func crossDeviceActiveView(request: CrossDeviceRequest) -> some View {
        ScrollView {
            VStack(spacing: 16) {
                Text("Scan this QR code")
                    .font(.headline)
                
                if let qrImage = request.qrCodeImage {
                    Image(uiImage: qrImage)
                        .interpolation(.none)
                        .resizable()
                        .scaledToFit()
                        .frame(width: 200, height: 200)
                        .padding()
                        .background(Color.white)
                        .cornerRadius(12)
                }
                
                HStack {
                    Image(systemName: "clock")
                    Text("Expires in \(Int(timeRemaining))s")
                }
                .font(.caption)
                .foregroundColor(timeRemaining < 60 ? .red : .secondary)
                
                if isPolling {
                    HStack(spacing: 8) {
                        ProgressView()
                            .progressViewStyle(CircularProgressViewStyle())
                        Text("Waiting for authentication...")
                            .foregroundColor(.secondary)
                    }
                }
                
                Divider()
                    .padding(.vertical)
                
                Text("Or share this link:")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                HStack {
                    Text(request.url.absoluteString)
                        .font(.caption2)
                        .lineLimit(1)
                        .foregroundColor(.blue)
                    
                    Button {
                        UIPasteboard.general.string = request.url.absoluteString
                    } label: {
                        Image(systemName: "doc.on.doc")
                    }
                }
                .padding()
                .background(Color(.systemGray6))
                .cornerRadius(8)
                
                ShareLink(item: request.url) {
                    HStack {
                        Image(systemName: "square.and.arrow.up")
                        Text("Share Link")
                    }
                }
                
                Button("Generate New Code") {
                    generateCrossDeviceRequest()
                }
                .font(.caption)
                .foregroundColor(.secondary)
            }
        }
    }
    
    // MARK: - Manual Entry Tab
    
    private var manualEntryTab: some View {
        ScrollView {
            VStack(spacing: 24) {
                Image(systemName: "keyboard")
                    .font(.system(size: 60))
                    .foregroundColor(.orange)
                
                Text("Manual Entry")
                    .font(.title2.bold())
                
                Text("Enter your Pubky credentials manually if other methods aren't available.")
                    .font(.body)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal)
                
                VStack(alignment: .leading, spacing: 8) {
                    Text("Public Key (z-base32)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    TextField("e.g., z6mk...", text: $manualPubkey)
                        .textFieldStyle(.roundedBorder)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                }
                .padding(.horizontal)
                
                VStack(alignment: .leading, spacing: 8) {
                    Text("Session Secret")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    SecureField("Secret from Pubky-ring", text: $manualSessionSecret)
                        .textFieldStyle(.roundedBorder)
                }
                .padding(.horizontal)
                
                Button {
                    importManualSession()
                } label: {
                    HStack {
                        Image(systemName: "checkmark.circle.fill")
                        Text("Import Session")
                    }
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(manualPubkey.isEmpty || manualSessionSecret.isEmpty ? Color.gray : Color.orange)
                    .foregroundColor(.white)
                    .cornerRadius(12)
                }
                .disabled(manualPubkey.isEmpty || manualSessionSecret.isEmpty)
                .padding(.horizontal)
            }
            .padding(.vertical)
        }
    }
    
    // MARK: - Actions
    
    private func authenticateWithPubkyRing() async {
        do {
            let session = try await bridge.requestSession()
            await MainActor.run {
                onSessionReceived(session)
                dismiss()
            }
        } catch {
            await MainActor.run {
                errorMessage = (error as? PubkyRingError)?.userMessage ?? error.localizedDescription
            }
        }
    }
    
    private func generateCrossDeviceRequest() {
        crossDeviceRequest = bridge.generateCrossDeviceRequest()
        timeRemaining = crossDeviceRequest?.timeRemaining ?? 0
        
        isPolling = true
        Task {
            await pollForSession()
        }
    }
    
    private func pollForSession() async {
        guard let request = crossDeviceRequest else { return }
        
        do {
            let session = try await bridge.pollForCrossDeviceSession(requestId: request.requestId)
            await MainActor.run {
                isPolling = false
                onSessionReceived(session)
                dismiss()
            }
        } catch {
            await MainActor.run {
                isPolling = false
                if !(error is CancellationError) {
                    errorMessage = (error as? PubkyRingError)?.userMessage ?? "Authentication timed out. Please try again."
                }
            }
        }
    }
    
    private func importManualSession() {
        let session = bridge.importSession(
            pubkey: manualPubkey.trimmingCharacters(in: .whitespacesAndNewlines),
            sessionSecret: manualSessionSecret.trimmingCharacters(in: .whitespacesAndNewlines)
        )
        onSessionReceived(session)
        dismiss()
    }
}

// MARK: - Connection Status Card

/// Card showing Pubky-ring connection status with action buttons
struct PubkyRingConnectionCard: View {
    @ObservedObject private var bridge = PubkyRingBridge.shared
    @State private var showingAuthSheet = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: statusIcon)
                    .foregroundColor(statusColor)
                    .font(.title2)
                
                VStack(alignment: .leading, spacing: 2) {
                    Text("Pubky Ring")
                        .font(.headline)
                    
                    Text(bridge.connectionState.displayText)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                if bridge.connectionState.isConnected {
                    Button {
                        bridge.disconnect()
                    } label: {
                        Text("Disconnect")
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                } else {
                    Button {
                        showingAuthSheet = true
                    } label: {
                        Text("Connect")
                            .font(.caption.bold())
                            .padding(.horizontal, 12)
                            .padding(.vertical, 6)
                            .background(Color.blue)
                            .foregroundColor(.white)
                            .cornerRadius(8)
                    }
                }
            }
            
            if !bridge.isPubkyRingInstalled && !bridge.connectionState.isConnected {
                Text("Pubky-ring not installed. Use QR code for cross-device auth.")
                    .font(.caption2)
                    .foregroundColor(.orange)
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
        .sheet(isPresented: $showingAuthSheet) {
            PubkyRingAuthView { session in
                // Session received, card will update automatically
            }
        }
    }
    
    private var statusIcon: String {
        switch bridge.connectionState {
        case .disconnected:
            return "circle"
        case .connecting:
            return "circle.dotted"
        case .connected:
            return "checkmark.circle.fill"
        case .error:
            return "exclamationmark.circle.fill"
        }
    }
    
    private var statusColor: Color {
        switch bridge.connectionState {
        case .disconnected:
            return .gray
        case .connecting:
            return .orange
        case .connected:
            return .green
        case .error:
            return .red
        }
    }
}

// MARK: - Preview

#Preview {
    PubkyRingAuthView { session in
        print("Received session: \(session.pubkey)")
    }
}

#Preview("Connection Card") {
    PubkyRingConnectionCard()
        .padding()
}

