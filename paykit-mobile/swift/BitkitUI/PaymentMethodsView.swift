//
//  PaymentMethodsView.swift
//  PaykitMobile
//
//  Payment Methods UI component for Bitkit integration.
//  This is a template that Bitkit can adapt to their design system.
//

import SwiftUI
import PaykitMobile

/// Payment Methods view model for Bitkit integration
public class BitkitPaymentMethodsViewModel: ObservableObject {
    @Published public var methods: [PaykitMobile.PaymentMethodInfo] = []
    @Published public var healthResults: [PaykitMobile.HealthCheckResult] = []
    @Published public var isLoading = false
    
    private let paykitClient: PaykitClient
    
    public init(paykitClient: PaykitClient) {
        self.paykitClient = paykitClient
    }
    
    func loadMethods() {
        isLoading = true
        
        Task {
            do {
                let methodsList = try paykitClient.listMethods()
                let health = paykitClient.checkHealth()
                
                await MainActor.run {
                    self.methods = methodsList
                    self.healthResults = health
                    self.isLoading = false
                }
            } catch {
                await MainActor.run {
                    self.isLoading = false
                }
            }
        }
    }
    
    func validateEndpoint(methodId: String, endpoint: String) -> Bool {
        do {
            return try paykitClient.validateEndpoint(methodId: methodId, endpoint: endpoint)
        } catch {
            return false
        }
    }
}

/// Payment Methods view component
public struct BitkitPaymentMethodsView: View {
    @ObservedObject var viewModel: BitkitPaymentMethodsViewModel
    
    public init(viewModel: BitkitPaymentMethodsViewModel) {
        self.viewModel = viewModel
    }
    
    public var body: some View {
        NavigationView {
            Group {
                if viewModel.isLoading {
                    ProgressView()
                } else if viewModel.methods.isEmpty {
                    emptyStateView
                } else {
                    methodsList
                }
            }
            .navigationTitle("Payment Methods")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: viewModel.loadMethods) {
                        Image(systemName: "arrow.clockwise")
                    }
                }
            }
            .onAppear {
                viewModel.loadMethods()
            }
        }
    }
    
    private var emptyStateView: some View {
        VStack(spacing: 24) {
            Image(systemName: "creditcard")
                .font(.system(size: 80))
                .foregroundColor(.secondary)
            
            Text("No Payment Methods")
                .font(.title2)
                .fontWeight(.semibold)
            
            Text("Payment methods will appear here once configured")
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding()
    }
    
    private var methodsList: some View {
        List {
            ForEach(viewModel.methods, id: \.methodId) { method in
                PaymentMethodRow(
                    method: method,
                    health: viewModel.healthResults.first { $0.methodId == method.methodId }
                )
            }
        }
    }
}

struct PaymentMethodRow: View {
    let method: PaykitMobile.PaymentMethodInfo
    let health: PaykitMobile.HealthCheckResult?
    
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text(method.methodId)
                    .font(.headline)
                
                if let health = health {
                    HStack {
                        Circle()
                            .fill(health.isHealthy ? Color.green : Color.red)
                            .frame(width: 8, height: 8)
                        Text(health.isHealthy ? "Healthy" : "Unhealthy")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
            
            Spacer()
            
            if let health = health, !health.isHealthy {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundColor(.orange)
            }
        }
        .padding(.vertical, 4)
    }
}
