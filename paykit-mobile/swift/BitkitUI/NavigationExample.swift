//
//  NavigationExample.swift
//  PaykitMobile
//
//  Navigation structure example for Bitkit integration.
//  This shows how to set up the main navigation with all screens.
//

import SwiftUI
import PaykitMobile

/// Main navigation view for Bitkit
/// Bitkit should adapt this to their navigation structure
public struct BitkitMainNavigationView: View {
    @StateObject private var paykitClient = PaykitClient.new()
    
    // View Models
    @StateObject private var dashboardViewModel: BitkitDashboardViewModel
    @StateObject private var paymentViewModel: BitkitPaymentViewModel
    @StateObject private var receiveViewModel: BitkitReceivePaymentViewModel
    @StateObject private var contactsViewModel: BitkitContactsViewModel
    @StateObject private var receiptsViewModel: BitkitReceiptsViewModel
    @StateObject private var paymentMethodsViewModel: BitkitPaymentMethodsViewModel
    
    @State private var selectedTab = 0
    
    public init() {
        let client = PaykitClient.new()
        
        // Initialize view models
        // Bitkit should provide their storage implementations
        _dashboardViewModel = StateObject(wrappedValue: BitkitDashboardViewModel(paykitClient: client))
        _paymentViewModel = StateObject(wrappedValue: BitkitPaymentViewModel(paykitClient: client))
        _receiveViewModel = StateObject(wrappedValue: BitkitReceivePaymentViewModel(paykitClient: client))
        
        // Placeholder storage - Bitkit should replace with actual implementations
        let contactStorage = MockContactStorage()
        let receiptStorage = MockReceiptStorage()
        
        _contactsViewModel = StateObject(wrappedValue: BitkitContactsViewModel(contactStorage: contactStorage))
        _receiptsViewModel = StateObject(wrappedValue: BitkitReceiptsViewModel(receiptStorage: receiptStorage))
        _paymentMethodsViewModel = StateObject(wrappedValue: BitkitPaymentMethodsViewModel(paykitClient: client))
    }
    
    public var body: some View {
        TabView(selection: $selectedTab) {
            // Dashboard Tab
            BitkitDashboardView(viewModel: dashboardViewModel)
                .tabItem {
                    Label("Dashboard", systemImage: "house.fill")
                }
                .tag(0)
            
            // Send Tab
            BitkitPaymentView(viewModel: paymentViewModel)
                .tabItem {
                    Label("Send", systemImage: "arrow.up.circle.fill")
                }
                .tag(1)
            
            // Receive Tab
            BitkitReceivePaymentView(viewModel: receiveViewModel)
                .tabItem {
                    Label("Receive", systemImage: "arrow.down.circle.fill")
                }
                .tag(2)
            
            // Contacts Tab
            BitkitContactsView(viewModel: contactsViewModel)
                .tabItem {
                    Label("Contacts", systemImage: "person.2.fill")
                }
                .tag(3)
            
            // Receipts Tab
            BitkitReceiptsView(viewModel: receiptsViewModel)
                .tabItem {
                    Label("Receipts", systemImage: "doc.text.fill")
                }
                .tag(4)
        }
        .onAppear {
            // Load initial data
            dashboardViewModel.loadDashboard()
        }
    }
}

// MARK: - Mock Storage (for example only - Bitkit should implement real storage)

class MockContactStorage: ContactStorageProtocol {
    func listContacts() -> [Contact] {
        return []
    }
}

class MockReceiptStorage: ReceiptStorageProtocol {
    func recentReceipts(limit: Int) -> [Receipt] {
        return []
    }
    
    func totalSent() -> UInt64 {
        return 0
    }
    
    func totalReceived() -> UInt64 {
        return 0
    }
    
    func pendingCount() -> Int {
        return 0
    }
}
