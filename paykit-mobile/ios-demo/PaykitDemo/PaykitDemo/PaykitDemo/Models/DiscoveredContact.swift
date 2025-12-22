//
//  DiscoveredContact.swift
//  PaykitDemo
//
//  Shared model for discovered contacts with health tracking
//

import Foundation
import SwiftUI

/// A contact discovered from Pubky follows directory with payment endpoint health status
public struct DiscoveredContact: Identifiable, Equatable {
    public let id: String
    public let pubkey: String
    public let name: String?
    public let supportedMethods: [String]
    public var endpointHealth: [String: Bool]
    public var lastHealthCheckDates: [String: Date]
    
    public init(
        pubkey: String,
        name: String? = nil,
        supportedMethods: [String] = [],
        endpointHealth: [String: Bool] = [:],
        lastHealthCheckDates: [String: Date] = [:]
    ) {
        self.id = pubkey
        self.pubkey = pubkey
        self.name = name
        self.supportedMethods = supportedMethods
        self.endpointHealth = endpointHealth
        self.lastHealthCheckDates = lastHealthCheckDates
    }
    
    // Legacy initializer for compatibility with existing code
    public init(
        publicKeyZ32: String,
        hasPaymentMethods: Bool,
        supportedMethods: [String]
    ) {
        self.id = publicKeyZ32
        self.pubkey = publicKeyZ32
        self.name = nil
        self.supportedMethods = supportedMethods
        
        // Initialize health as true for all methods if hasPaymentMethods
        var health: [String: Bool] = [:]
        if hasPaymentMethods {
            for method in supportedMethods {
                health[method] = true
            }
        }
        self.endpointHealth = health
        self.lastHealthCheckDates = [:]
    }
    
    // Computed properties
    public var publicKeyZ32: String { pubkey }
    
    public var hasPaymentMethods: Bool {
        !supportedMethods.isEmpty
    }
    
    public var abbreviatedPubkey: String {
        guard pubkey.count > 16 else { return pubkey }
        return "\(pubkey.prefix(8))...\(pubkey.suffix(8))"
    }
    
    // Alias for compatibility
    public var abbreviatedKey: String { abbreviatedPubkey }
    
    public func isMethodHealthy(_ method: String) -> Bool {
        endpointHealth[method] ?? true
    }
    
    public func lastHealthCheck(_ method: String) -> Date? {
        lastHealthCheckDates[method]
    }
    
    public var healthyCount: Int {
        supportedMethods.filter { isMethodHealthy($0) }.count
    }
    
    public var healthStatus: String {
        if supportedMethods.isEmpty { return "No endpoints" }
        if healthyCount == supportedMethods.count { return "All healthy" }
        if healthyCount > 0 { return "\(healthyCount)/\(supportedMethods.count)" }
        return "Unreachable"
    }
    
    public var healthColor: Color {
        if supportedMethods.isEmpty { return .gray }
        if healthyCount == supportedMethods.count { return .green }
        if healthyCount > 0 { return .orange }
        return .red
    }
    
    public var healthIcon: String {
        if supportedMethods.isEmpty { return "questionmark.circle" }
        if healthyCount == supportedMethods.count { return "checkmark.circle.fill" }
        if healthyCount > 0 { return "exclamationmark.circle.fill" }
        return "xmark.circle.fill"
    }
    
    public static func == (lhs: DiscoveredContact, rhs: DiscoveredContact) -> Bool {
        lhs.id == rhs.id
    }
}

