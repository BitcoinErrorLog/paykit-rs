//
//  RotationSettingsStorage.swift
//  PaykitDemo
//
//  Storage for endpoint rotation configuration and history.
//

import Foundation

/// Rotation policy types
enum RotationPolicy: String, Codable, CaseIterable, Identifiable {
    case onUse = "on-use"
    case afterUses = "after-uses"
    case manual = "manual"
    
    var id: String { rawValue }
    
    var displayName: String {
        switch self {
        case .onUse: return "Rotate on every use"
        case .afterUses: return "Rotate after N uses"
        case .manual: return "Manual only"
        }
    }
    
    var description: String {
        switch self {
        case .onUse: return "Best privacy - new endpoint after each payment"
        case .afterUses: return "Rotate after a specified number of uses"
        case .manual: return "Only rotate when manually triggered"
        }
    }
}

/// Rotation settings for a specific method
struct MethodRotationSettings: Codable, Equatable {
    var policy: RotationPolicy
    var threshold: Int // For afterUses policy
    var useCount: Int
    var lastRotated: Date?
    var rotationCount: Int
    
    init(policy: RotationPolicy = .onUse, threshold: Int = 5) {
        self.policy = policy
        self.threshold = threshold
        self.useCount = 0
        self.lastRotated = nil
        self.rotationCount = 0
    }
}

/// Global rotation settings
struct RotationSettings: Codable {
    var autoRotateEnabled: Bool
    var defaultPolicy: RotationPolicy
    var defaultThreshold: Int
    var methodSettings: [String: MethodRotationSettings]
    
    init() {
        self.autoRotateEnabled = true
        self.defaultPolicy = .onUse
        self.defaultThreshold = 5
        self.methodSettings = [:]
    }
}

/// Rotation event for history tracking
struct RotationEvent: Codable, Identifiable {
    let id: UUID
    let methodId: String
    let timestamp: Date
    let reason: String
    
    init(methodId: String, reason: String) {
        self.id = UUID()
        self.methodId = methodId
        self.timestamp = Date()
        self.reason = reason
    }
}

/// Manages rotation settings and history persistence
class RotationSettingsStorage {
    
    private let identityName: String
    private let userDefaults: UserDefaults
    
    private var settingsKey: String {
        "paykit.rotation_settings.\(identityName)"
    }
    
    private var historyKey: String {
        "paykit.rotation_history.\(identityName)"
    }
    
    init(identityName: String, userDefaults: UserDefaults = .standard) {
        self.identityName = identityName
        self.userDefaults = userDefaults
    }
    
    // MARK: - Settings
    
    func loadSettings() -> RotationSettings {
        guard let data = userDefaults.data(forKey: settingsKey),
              let settings = try? JSONDecoder().decode(RotationSettings.self, from: data) else {
            return RotationSettings()
        }
        return settings
    }
    
    func saveSettings(_ settings: RotationSettings) {
        if let data = try? JSONEncoder().encode(settings) {
            userDefaults.set(data, forKey: settingsKey)
        }
    }
    
    func getMethodSettings(_ methodId: String) -> MethodRotationSettings {
        let settings = loadSettings()
        return settings.methodSettings[methodId] ?? MethodRotationSettings(
            policy: settings.defaultPolicy,
            threshold: settings.defaultThreshold
        )
    }
    
    func updateMethodSettings(_ methodId: String, _ methodSettings: MethodRotationSettings) {
        var settings = loadSettings()
        settings.methodSettings[methodId] = methodSettings
        saveSettings(settings)
    }
    
    // MARK: - Use Tracking
    
    /// Record a payment use for a method
    /// Returns true if rotation should occur
    func recordUse(methodId: String) -> Bool {
        var settings = loadSettings()
        var methodSettings = settings.methodSettings[methodId] ?? MethodRotationSettings(
            policy: settings.defaultPolicy,
            threshold: settings.defaultThreshold
        )
        
        guard settings.autoRotateEnabled else {
            return false
        }
        
        methodSettings.useCount += 1
        settings.methodSettings[methodId] = methodSettings
        saveSettings(settings)
        
        switch methodSettings.policy {
        case .onUse:
            return true
        case .afterUses:
            return methodSettings.useCount >= methodSettings.threshold
        case .manual:
            return false
        }
    }
    
    /// Record that a rotation occurred
    func recordRotation(methodId: String, reason: String) {
        var settings = loadSettings()
        var methodSettings = settings.methodSettings[methodId] ?? MethodRotationSettings(
            policy: settings.defaultPolicy,
            threshold: settings.defaultThreshold
        )
        
        methodSettings.useCount = 0
        methodSettings.lastRotated = Date()
        methodSettings.rotationCount += 1
        
        settings.methodSettings[methodId] = methodSettings
        saveSettings(settings)
        
        // Add to history
        addHistoryEvent(RotationEvent(methodId: methodId, reason: reason))
    }
    
    // MARK: - History
    
    func loadHistory() -> [RotationEvent] {
        guard let data = userDefaults.data(forKey: historyKey),
              let events = try? JSONDecoder().decode([RotationEvent].self, from: data) else {
            return []
        }
        return events.sorted { $0.timestamp > $1.timestamp }
    }
    
    private func addHistoryEvent(_ event: RotationEvent) {
        var history = loadHistory()
        history.insert(event, at: 0)
        
        // Keep only last 100 events
        if history.count > 100 {
            history = Array(history.prefix(100))
        }
        
        if let data = try? JSONEncoder().encode(history) {
            userDefaults.set(data, forKey: historyKey)
        }
    }
    
    func clearHistory() {
        userDefaults.removeObject(forKey: historyKey)
    }
    
    // MARK: - Statistics
    
    func totalRotations() -> Int {
        let settings = loadSettings()
        return settings.methodSettings.values.reduce(0) { $0 + $1.rotationCount }
    }
    
    func methodsWithRotations() -> [String] {
        let settings = loadSettings()
        return settings.methodSettings.filter { $0.value.rotationCount > 0 }.map { $0.key }
    }
}

