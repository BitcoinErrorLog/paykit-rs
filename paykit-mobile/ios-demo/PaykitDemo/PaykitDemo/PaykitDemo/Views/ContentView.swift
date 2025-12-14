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
            
            // Send Payment Tab (Noise)
            PaymentView()
                .tabItem {
                    Label("Send", systemImage: "paperplane.fill")
                }
            
            // Receive Payment Tab (Noise)
            ReceivePaymentView()
                .tabItem {
                    Label("Receive", systemImage: "arrow.down.circle.fill")
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
