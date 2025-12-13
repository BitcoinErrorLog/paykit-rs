//
//  ContentView.swift
//  PaykitDemo
//
//  Main navigation view for the demo app
//

import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    
    var body: some View {
        TabView {
            // Dashboard Tab
            DashboardView()
                .tabItem {
                    Label("Dashboard", systemImage: "house.fill")
                }
            
            // Payment Methods Tab
            PaymentMethodsView()
                .tabItem {
                    Label("Methods", systemImage: "creditcard")
                }
            
            // Contacts Tab
            ContactsView()
                .tabItem {
                    Label("Contacts", systemImage: "person.2")
                }
            
            // Receipts Tab
            ReceiptsView()
                .tabItem {
                    Label("Receipts", systemImage: "doc.text.fill")
                }
            
            // Subscriptions Tab
            SubscriptionsView()
                .tabItem {
                    Label("Subscriptions", systemImage: "repeat")
                }
            
            // Auto-Pay Tab
            AutoPayView()
                .tabItem {
                    Label("Auto-Pay", systemImage: "bolt.fill")
                }
            
            // Payment Requests Tab
            PaymentRequestsView()
                .tabItem {
                    Label("Requests", systemImage: "arrow.left.arrow.right")
                }
            
            // Settings Tab
            SettingsView()
                .tabItem {
                    Label("Settings", systemImage: "gear")
                }
        }
        .alert("Error", isPresented: .constant(appState.errorMessage != nil)) {
            Button("OK") {
                appState.errorMessage = nil
            }
        } message: {
            Text(appState.errorMessage ?? "")
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(AppState())
}
