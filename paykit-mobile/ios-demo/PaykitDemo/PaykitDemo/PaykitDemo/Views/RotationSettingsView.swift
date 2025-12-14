//
//  RotationSettingsView.swift
//  PaykitDemo
//
//  View for configuring endpoint rotation policies
//

import SwiftUI

struct RotationSettingsView: View {
    @EnvironmentObject var appState: AppState
    @StateObject private var viewModel = RotationSettingsViewModel()
    @Environment(\.dismiss) private var dismiss
    
    var body: some View {
        NavigationView {
            Form {
                // Global Settings
                Section {
                    Toggle("Auto-Rotate After Payments", isOn: $viewModel.autoRotateEnabled)
                        .onChange(of: viewModel.autoRotateEnabled) { _ in
                            viewModel.saveSettings()
                        }
                    
                    Picker("Default Policy", selection: $viewModel.defaultPolicy) {
                        ForEach(RotationPolicy.allCases) { policy in
                            Text(policy.displayName).tag(policy)
                        }
                    }
                    .onChange(of: viewModel.defaultPolicy) { _ in
                        viewModel.saveSettings()
                    }
                    
                    if viewModel.defaultPolicy == .afterUses {
                        Stepper("Threshold: \(viewModel.defaultThreshold) uses",
                                value: $viewModel.defaultThreshold, in: 1...100)
                            .onChange(of: viewModel.defaultThreshold) { _ in
                                viewModel.saveSettings()
                            }
                    }
                } header: {
                    Text("Global Settings")
                } footer: {
                    Text("Auto-rotation replaces your payment endpoints after use to prevent address reuse and enhance privacy.")
                }
                
                // Per-Method Settings
                Section {
                    ForEach(["lightning", "onchain"], id: \.self) { method in
                        MethodPolicyRow(
                            methodId: method,
                            settings: viewModel.getMethodSettings(method),
                            onUpdate: { newSettings in
                                viewModel.updateMethodSettings(method, newSettings)
                            }
                        )
                    }
                } header: {
                    Text("Per-Method Policies")
                } footer: {
                    Text("Override the default policy for specific payment methods.")
                }
                
                // Rotation Status
                Section {
                    HStack {
                        Text("Total Rotations")
                        Spacer()
                        Text("\(viewModel.totalRotations)")
                            .foregroundColor(.secondary)
                    }
                    
                    ForEach(viewModel.methodStats, id: \.0) { stat in
                        HStack {
                            MethodIcon(methodId: stat.0)
                            Text(stat.0.capitalized)
                            Spacer()
                            VStack(alignment: .trailing) {
                                Text("\(stat.1.rotationCount) rotations")
                                    .font(.caption)
                                if let lastRotated = stat.1.lastRotated {
                                    Text(lastRotated, style: .relative)
                                        .font(.caption2)
                                        .foregroundColor(.secondary)
                                }
                            }
                        }
                    }
                } header: {
                    Text("Rotation Status")
                }
                
                // History
                Section {
                    if viewModel.history.isEmpty {
                        Text("No rotation history yet")
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(viewModel.history.prefix(10)) { event in
                            HStack {
                                MethodIcon(methodId: event.methodId)
                                VStack(alignment: .leading) {
                                    Text(event.methodId.capitalized)
                                        .font(.body)
                                    Text(event.reason)
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                }
                                Spacer()
                                Text(event.timestamp, style: .relative)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                        
                        if viewModel.history.count > 10 {
                            Text("+ \(viewModel.history.count - 10) more events")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                    
                    if !viewModel.history.isEmpty {
                        Button("Clear History", role: .destructive) {
                            viewModel.clearHistory()
                        }
                    }
                } header: {
                    Text("Rotation History")
                }
            }
            .navigationTitle("Rotation Settings")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
        }
        .onAppear {
            viewModel.load(identityName: appState.currentIdentityName)
        }
    }
}

// MARK: - Method Policy Row

private struct MethodPolicyRow: View {
    let methodId: String
    let settings: MethodRotationSettings
    let onUpdate: (MethodRotationSettings) -> Void
    
    @State private var showingPicker = false
    
    var body: some View {
        Button {
            showingPicker = true
        } label: {
            HStack {
                MethodIcon(methodId: methodId)
                
                VStack(alignment: .leading) {
                    Text(methodId.capitalized)
                        .foregroundColor(.primary)
                    Text(settings.policy.displayName)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                Text("Uses: \(settings.useCount)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Image(systemName: "chevron.right")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .sheet(isPresented: $showingPicker) {
            MethodPolicyPicker(
                methodId: methodId,
                settings: settings,
                onSave: onUpdate
            )
        }
    }
}

// MARK: - Method Policy Picker

private struct MethodPolicyPicker: View {
    let methodId: String
    @State var settings: MethodRotationSettings
    let onSave: (MethodRotationSettings) -> Void
    
    @Environment(\.dismiss) private var dismiss
    
    var body: some View {
        NavigationView {
            Form {
                Section {
                    Picker("Policy", selection: $settings.policy) {
                        ForEach(RotationPolicy.allCases) { policy in
                            VStack(alignment: .leading) {
                                Text(policy.displayName)
                                Text(policy.description)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            .tag(policy)
                        }
                    }
                    .pickerStyle(.inline)
                    .labelsHidden()
                }
                
                if settings.policy == .afterUses {
                    Section {
                        Stepper("Rotate after \(settings.threshold) uses",
                                value: $settings.threshold, in: 1...100)
                    } header: {
                        Text("Threshold")
                    }
                }
                
                Section {
                    HStack {
                        Text("Current Use Count")
                        Spacer()
                        Text("\(settings.useCount)")
                            .foregroundColor(.secondary)
                    }
                    
                    HStack {
                        Text("Total Rotations")
                        Spacer()
                        Text("\(settings.rotationCount)")
                            .foregroundColor(.secondary)
                    }
                    
                    if let lastRotated = settings.lastRotated {
                        HStack {
                            Text("Last Rotated")
                            Spacer()
                            Text(lastRotated, style: .relative)
                                .foregroundColor(.secondary)
                        }
                    }
                } header: {
                    Text("Statistics")
                }
            }
            .navigationTitle("\(methodId.capitalized) Policy")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Save") {
                        onSave(settings)
                        dismiss()
                    }
                }
            }
        }
    }
}

// MARK: - Method Icon

private struct MethodIcon: View {
    let methodId: String
    
    var body: some View {
        Image(systemName: iconName)
            .foregroundColor(iconColor)
            .frame(width: 24)
    }
    
    private var iconName: String {
        switch methodId.lowercased() {
        case "lightning": return "bolt.fill"
        case "onchain": return "bitcoinsign.circle.fill"
        default: return "creditcard.fill"
        }
    }
    
    private var iconColor: Color {
        switch methodId.lowercased() {
        case "lightning": return .orange
        case "onchain": return .yellow
        default: return .blue
        }
    }
}

// MARK: - View Model

@MainActor
class RotationSettingsViewModel: ObservableObject {
    @Published var autoRotateEnabled = true
    @Published var defaultPolicy: RotationPolicy = .onUse
    @Published var defaultThreshold = 5
    @Published var methodSettings: [String: MethodRotationSettings] = [:]
    @Published var history: [RotationEvent] = []
    @Published var totalRotations = 0
    
    private var storage: RotationSettingsStorage?
    
    var methodStats: [(String, MethodRotationSettings)] {
        methodSettings.sorted { $0.key < $1.key }
    }
    
    func load(identityName: String?) {
        guard let identity = identityName else { return }
        storage = RotationSettingsStorage(identityName: identity)
        
        let settings = storage?.loadSettings() ?? RotationSettings()
        autoRotateEnabled = settings.autoRotateEnabled
        defaultPolicy = settings.defaultPolicy
        defaultThreshold = settings.defaultThreshold
        methodSettings = settings.methodSettings
        
        history = storage?.loadHistory() ?? []
        totalRotations = storage?.totalRotations() ?? 0
    }
    
    func saveSettings() {
        var settings = RotationSettings()
        settings.autoRotateEnabled = autoRotateEnabled
        settings.defaultPolicy = defaultPolicy
        settings.defaultThreshold = defaultThreshold
        settings.methodSettings = methodSettings
        storage?.saveSettings(settings)
    }
    
    func getMethodSettings(_ methodId: String) -> MethodRotationSettings {
        methodSettings[methodId] ?? MethodRotationSettings(
            policy: defaultPolicy,
            threshold: defaultThreshold
        )
    }
    
    func updateMethodSettings(_ methodId: String, _ settings: MethodRotationSettings) {
        methodSettings[methodId] = settings
        storage?.updateMethodSettings(methodId, settings)
    }
    
    func clearHistory() {
        storage?.clearHistory()
        history = []
    }
}

// MARK: - Preview

#Preview {
    RotationSettingsView()
        .environmentObject(AppState())
}

