//
//  SmartCheckoutView.swift
//  PaykitDemo
//
//  Smart checkout flow that discovers and selects the best payment method
//  for a recipient. Matches Bitkit's smart checkout patterns.
//

import SwiftUI

/// Smart checkout discovers available payment methods for a recipient
/// and helps the user select the best option based on their preferences.
struct SmartCheckoutView: View {
    let recipientPubkey: String
    let recipientName: String?
    let amount: UInt64
    let onComplete: (SmartCheckoutResult) -> Void
    let onCancel: () -> Void
    
    @EnvironmentObject var appState: AppState
    @State private var state: SmartCheckoutState = .discovering
    @State private var discoveredMethods: [DiscoveredMethod] = []
    @State private var selectedMethod: DiscoveredMethod?
    @State private var strategy: CheckoutStrategy = .balanced
    @State private var errorMessage: String?
    
    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                // Recipient header
                recipientHeader
                    .padding()
                
                Divider()
                
                // Content based on state
                switch state {
                case .discovering:
                    discoveryView
                case .selectingMethod:
                    methodSelectionView
                case .confirming:
                    confirmationView
                case .error(let message):
                    errorView(message: message)
                }
            }
            .navigationTitle("Smart Checkout")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { onCancel() }
                }
            }
        }
        .task {
            await discoverMethods()
        }
    }
    
    // MARK: - Recipient Header
    
    private var recipientHeader: some View {
        HStack(spacing: 16) {
            Circle()
                .fill(Color.blue.opacity(0.2))
                .frame(width: 56, height: 56)
                .overlay {
                    Text(String((recipientName ?? recipientPubkey).prefix(1)).uppercased())
                        .font(.title2.bold())
                        .foregroundColor(.blue)
                }
            
            VStack(alignment: .leading, spacing: 4) {
                Text(recipientName ?? "Unknown")
                    .font(.headline)
                
                Text(abbreviate(recipientPubkey))
                    .font(.caption.monospaced())
                    .foregroundColor(.secondary)
                
                Text("\(formatSats(amount))")
                    .font(.subheadline.bold())
                    .foregroundColor(.blue)
            }
            
            Spacer()
        }
    }
    
    // MARK: - Discovery View
    
    private var discoveryView: some View {
        VStack(spacing: 24) {
            Spacer()
            
            ProgressView()
                .scaleEffect(1.5)
            
            Text("Discovering payment methods...")
                .font(.headline)
            
            Text("Checking what payment options are available for this recipient")
                .font(.subheadline)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
            
            Spacer()
        }
    }
    
    // MARK: - Method Selection View
    
    private var methodSelectionView: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Strategy picker
                strategyPicker
                
                // Recommended method (if any)
                if let recommended = getRecommendedMethod() {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Recommended")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        
                        MethodCard(
                            method: recommended,
                            isSelected: selectedMethod?.id == recommended.id,
                            isRecommended: true
                        ) {
                            selectedMethod = recommended
                        }
                    }
                }
                
                // All available methods
                VStack(alignment: .leading, spacing: 8) {
                    Text("All Payment Methods")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    ForEach(discoveredMethods) { method in
                        MethodCard(
                            method: method,
                            isSelected: selectedMethod?.id == method.id,
                            isRecommended: false
                        ) {
                            selectedMethod = method
                        }
                    }
                }
                
                // Continue button
                Button {
                    if selectedMethod != nil {
                        state = .confirming
                    }
                } label: {
                    Text("Continue")
                        .font(.headline)
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(selectedMethod != nil ? Color.blue : Color.gray)
                        .foregroundColor(.white)
                        .cornerRadius(12)
                }
                .disabled(selectedMethod == nil)
            }
            .padding()
        }
    }
    
    private var strategyPicker: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Optimization")
                .font(.caption)
                .foregroundColor(.secondary)
            
            Picker("Strategy", selection: $strategy) {
                Text("Balanced").tag(CheckoutStrategy.balanced)
                Text("Lowest Fee").tag(CheckoutStrategy.lowestFee)
                Text("Fastest").tag(CheckoutStrategy.fastest)
                Text("Most Private").tag(CheckoutStrategy.mostPrivate)
            }
            .pickerStyle(.segmented)
            .onChange(of: strategy) { _ in
                // Re-select recommended based on strategy
                selectedMethod = getRecommendedMethod()
            }
        }
    }
    
    // MARK: - Confirmation View
    
    private var confirmationView: some View {
        VStack(spacing: 24) {
            Spacer()
            
            if let method = selectedMethod {
                Image(systemName: method.icon)
                    .font(.system(size: 60))
                    .foregroundColor(.blue)
                
                Text("Confirm Payment")
                    .font(.title2.bold())
                
                VStack(spacing: 8) {
                    HStack {
                        Text("Amount")
                        Spacer()
                        Text(formatSats(amount))
                            .bold()
                    }
                    
                    HStack {
                        Text("Method")
                        Spacer()
                        Text(method.displayName)
                    }
                    
                    HStack {
                        Text("Estimated Fee")
                        Spacer()
                        Text(formatSats(method.estimatedFee))
                            .foregroundColor(.secondary)
                    }
                    
                    Divider()
                    
                    HStack {
                        Text("Total")
                            .bold()
                        Spacer()
                        Text(formatSats(amount + method.estimatedFee))
                            .bold()
                    }
                }
                .padding()
                .background(Color(.systemGray6))
                .cornerRadius(12)
                .padding(.horizontal)
                
                Button {
                    completeCheckout(with: method)
                } label: {
                    Text("Pay Now")
                        .font(.headline)
                        .frame(maxWidth: .infinity)
                        .padding()
                        .background(Color.blue)
                        .foregroundColor(.white)
                        .cornerRadius(12)
                }
                .padding(.horizontal)
            }
            
            Spacer()
        }
    }
    
    // MARK: - Error View
    
    private func errorView(message: String) -> some View {
        VStack(spacing: 24) {
            Spacer()
            
            Image(systemName: "exclamationmark.circle.fill")
                .font(.system(size: 60))
                .foregroundColor(.red)
            
            Text("Discovery Failed")
                .font(.title2.bold())
            
            Text(message)
                .font(.subheadline)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
            
            Button("Try Again") {
                Task { await discoverMethods() }
            }
            .buttonStyle(.borderedProminent)
            
            Spacer()
        }
    }
    
    // MARK: - Logic
    
    private func discoverMethods() async {
        state = .discovering
        
        // Simulate network delay
        try? await Task.sleep(nanoseconds: 1_500_000_000)
        
        // Generate mock discovered methods
        let methods = generateMockMethods()
        
        if methods.isEmpty {
            state = .error("No payment methods available for this recipient")
        } else {
            discoveredMethods = methods
            selectedMethod = getRecommendedMethod()
            state = .selectingMethod
        }
    }
    
    private func generateMockMethods() -> [DiscoveredMethod] {
        var methods: [DiscoveredMethod] = []
        
        // Lightning
        methods.append(DiscoveredMethod(
            id: "lightning",
            methodId: "lightning",
            displayName: "Lightning Network",
            icon: "bolt.fill",
            isHealthy: true,
            estimatedFee: max(1, amount / 1000),
            estimatedTime: "< 1 second",
            privacyScore: 0.8,
            speedScore: 1.0,
            costScore: 0.9
        ))
        
        // On-chain (if amount > 10k sats)
        if amount > 10_000 {
            methods.append(DiscoveredMethod(
                id: "onchain",
                methodId: "onchain",
                displayName: "On-Chain Bitcoin",
                icon: "bitcoinsign.circle.fill",
                isHealthy: true,
                estimatedFee: 500,
                estimatedTime: "~10 minutes",
                privacyScore: 0.5,
                speedScore: 0.3,
                costScore: 0.7
            ))
        }
        
        // Noise (if available)
        methods.append(DiscoveredMethod(
            id: "noise",
            methodId: "noise",
            displayName: "Noise Protocol",
            icon: "antenna.radiowaves.left.and.right",
            isHealthy: Bool.random(),
            estimatedFee: 0,
            estimatedTime: "< 1 second",
            privacyScore: 1.0,
            speedScore: 0.9,
            costScore: 1.0
        ))
        
        return methods.filter { $0.isHealthy }
    }
    
    private func getRecommendedMethod() -> DiscoveredMethod? {
        guard !discoveredMethods.isEmpty else { return nil }
        
        switch strategy {
        case .balanced:
            return discoveredMethods.max { $0.balancedScore < $1.balancedScore }
        case .lowestFee:
            return discoveredMethods.min { $0.estimatedFee < $1.estimatedFee }
        case .fastest:
            return discoveredMethods.max { $0.speedScore < $1.speedScore }
        case .mostPrivate:
            return discoveredMethods.max { $0.privacyScore < $1.privacyScore }
        }
    }
    
    private func completeCheckout(with method: DiscoveredMethod) {
        let result = SmartCheckoutResult(
            recipientPubkey: recipientPubkey,
            selectedMethod: method.methodId,
            amount: amount,
            estimatedFee: method.estimatedFee
        )
        onComplete(result)
    }
    
    // MARK: - Helpers
    
    private func abbreviate(_ key: String) -> String {
        guard key.count > 16 else { return key }
        return "\(key.prefix(8))...\(key.suffix(8))"
    }
    
    private func formatSats(_ sats: UInt64) -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        return "\(formatter.string(from: NSNumber(value: sats)) ?? "\(sats)") sats"
    }
}

// MARK: - Supporting Types

enum SmartCheckoutState {
    case discovering
    case selectingMethod
    case confirming
    case error(String)
}

/// Strategy for selecting payment methods in smart checkout
/// Named differently to avoid conflict with PaykitMobile.SelectionStrategy
enum CheckoutStrategy: String, CaseIterable {
    case balanced
    case lowestFee
    case fastest
    case mostPrivate
}

struct DiscoveredMethod: Identifiable {
    let id: String
    let methodId: String
    let displayName: String
    let icon: String
    let isHealthy: Bool
    let estimatedFee: UInt64
    let estimatedTime: String
    let privacyScore: Double
    let speedScore: Double
    let costScore: Double
    
    var balancedScore: Double {
        (privacyScore + speedScore + costScore) / 3.0
    }
}

struct SmartCheckoutResult {
    let recipientPubkey: String
    let selectedMethod: String
    let amount: UInt64
    let estimatedFee: UInt64
}

// MARK: - Method Card

struct MethodCard: View {
    let method: DiscoveredMethod
    let isSelected: Bool
    let isRecommended: Bool
    let onSelect: () -> Void
    
    var body: some View {
        Button(action: onSelect) {
            HStack(spacing: 16) {
                Image(systemName: method.icon)
                    .font(.title2)
                    .foregroundColor(isSelected ? .white : .blue)
                    .frame(width: 44, height: 44)
                    .background(isSelected ? Color.blue : Color.blue.opacity(0.1))
                    .cornerRadius(12)
                
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text(method.displayName)
                            .font(.headline)
                        
                        if isRecommended {
                            Text("Best")
                                .font(.caption2.bold())
                                .foregroundColor(.white)
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(Color.green)
                                .cornerRadius(4)
                        }
                    }
                    
                    Text(method.estimatedTime)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                VStack(alignment: .trailing, spacing: 4) {
                    Text("~\(method.estimatedFee) sats")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                    
                    Text("fee")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                
                if isSelected {
                    Image(systemName: "checkmark.circle.fill")
                        .foregroundColor(.blue)
                }
            }
            .padding()
            .background(isSelected ? Color.blue.opacity(0.1) : Color(.systemGray6))
            .cornerRadius(12)
            .overlay(
                RoundedRectangle(cornerRadius: 12)
                    .stroke(isSelected ? Color.blue : Color.clear, lineWidth: 2)
            )
        }
        .buttonStyle(.plain)
    }
}

#Preview {
    SmartCheckoutView(
        recipientPubkey: "z6mktest1234567890abcdefghijklmnop",
        recipientName: "Alice",
        amount: 10000
    ) { result in
        print("Completed: \(result)")
    } onCancel: {
        print("Cancelled")
    }
    .environmentObject(AppState())
}

