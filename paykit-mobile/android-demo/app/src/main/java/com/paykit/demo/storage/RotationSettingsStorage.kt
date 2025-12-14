package com.paykit.demo.storage

import android.content.Context
import android.content.SharedPreferences
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.decodeFromString

/**
 * Rotation policy types
 */
enum class RotationPolicy(val displayName: String, val description: String) {
    ON_USE("Rotate on every use", "Best privacy - new endpoint after each payment"),
    AFTER_USES("Rotate after N uses", "Rotate after a specified number of uses"),
    MANUAL("Manual only", "Only rotate when manually triggered")
}

/**
 * Rotation settings for a specific method
 */
@Serializable
data class MethodRotationSettings(
    val policy: String = "ON_USE", // RotationPolicy name
    val threshold: Int = 5,
    var useCount: Int = 0,
    var lastRotated: Long? = null,
    var rotationCount: Int = 0
)

/**
 * Global rotation settings
 */
@Serializable
data class RotationSettings(
    var autoRotateEnabled: Boolean = true,
    var defaultPolicy: String = "ON_USE", // RotationPolicy name
    var defaultThreshold: Int = 5,
    var methodSettings: Map<String, MethodRotationSettings> = emptyMap()
)

/**
 * Rotation event for history tracking
 */
@Serializable
data class RotationEvent(
    val id: String,
    val methodId: String,
    val timestamp: Long,
    val reason: String
)

/**
 * Manages rotation settings and history persistence
 */
class RotationSettingsStorage(context: Context, private val identityName: String) {
    
    private val PREFS_NAME = "paykit_rotation_settings_$identityName"
    private val SETTINGS_KEY = "rotation_settings"
    private val HISTORY_KEY = "rotation_history"
    
    private val json = Json { 
        ignoreUnknownKeys = true 
        encodeDefaults = true
    }
    
    private val prefs: SharedPreferences = 
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    
    // MARK: - Settings
    
    fun loadSettings(): RotationSettings {
        return try {
            val jsonString = prefs.getString(SETTINGS_KEY, null) ?: return RotationSettings()
            json.decodeFromString(jsonString)
        } catch (e: Exception) {
            RotationSettings()
        }
    }
    
    fun saveSettings(settings: RotationSettings) {
        try {
            val jsonString = json.encodeToString(settings)
            prefs.edit().putString(SETTINGS_KEY, jsonString).apply()
        } catch (e: Exception) {
            // Log error
        }
    }
    
    fun getMethodSettings(methodId: String): MethodRotationSettings {
        val settings = loadSettings()
        return settings.methodSettings[methodId] ?: MethodRotationSettings(
            policy = settings.defaultPolicy,
            threshold = settings.defaultThreshold
        )
    }
    
    fun updateMethodSettings(methodId: String, methodSettings: MethodRotationSettings) {
        val settings = loadSettings()
        val updated = settings.methodSettings.toMutableMap()
        updated[methodId] = methodSettings
        saveSettings(settings.copy(methodSettings = updated))
    }
    
    // MARK: - Use Tracking
    
    /**
     * Record a payment use for a method
     * Returns true if rotation should occur
     */
    fun recordUse(methodId: String): Boolean {
        val settings = loadSettings()
        if (!settings.autoRotateEnabled) {
            return false
        }
        
        val methodSettings = settings.methodSettings[methodId] ?: MethodRotationSettings(
            policy = settings.defaultPolicy,
            threshold = settings.defaultThreshold
        )
        
        val updatedMethod = methodSettings.copy(useCount = methodSettings.useCount + 1)
        updateMethodSettings(methodId, updatedMethod)
        
        return when (RotationPolicy.valueOf(methodSettings.policy)) {
            RotationPolicy.ON_USE -> true
            RotationPolicy.AFTER_USES -> updatedMethod.useCount >= methodSettings.threshold
            RotationPolicy.MANUAL -> false
        }
    }
    
    /**
     * Record that a rotation occurred
     */
    fun recordRotation(methodId: String, reason: String) {
        val settings = loadSettings()
        val methodSettings = settings.methodSettings[methodId] ?: MethodRotationSettings(
            policy = settings.defaultPolicy,
            threshold = settings.defaultThreshold
        )
        
        val updatedMethod = methodSettings.copy(
            useCount = 0,
            lastRotated = System.currentTimeMillis(),
            rotationCount = methodSettings.rotationCount + 1
        )
        
        updateMethodSettings(methodId, updatedMethod)
        
        // Add to history
        addHistoryEvent(RotationEvent(
            id = java.util.UUID.randomUUID().toString(),
            methodId = methodId,
            timestamp = System.currentTimeMillis(),
            reason = reason
        ))
    }
    
    // MARK: - History
    
    fun loadHistory(): List<RotationEvent> {
        return try {
            val jsonString = prefs.getString(HISTORY_KEY, null) ?: return emptyList()
            json.decodeFromString<List<RotationEvent>>(jsonString)
                .sortedByDescending { it.timestamp }
        } catch (e: Exception) {
            emptyList()
        }
    }
    
    private fun addHistoryEvent(event: RotationEvent) {
        try {
            val history = loadHistory().toMutableList()
            history.add(0, event)
            
            // Keep only last 100 events
            val trimmed = history.take(100)
            
            val jsonString = json.encodeToString(trimmed)
            prefs.edit().putString(HISTORY_KEY, jsonString).apply()
        } catch (e: Exception) {
            // Log error
        }
    }
    
    fun clearHistory() {
        prefs.edit().remove(HISTORY_KEY).apply()
    }
    
    // MARK: - Statistics
    
    fun totalRotations(): Int {
        val settings = loadSettings()
        return settings.methodSettings.values.sumOf { it.rotationCount }
    }
    
    fun methodsWithRotations(): List<String> {
        val settings = loadSettings()
        return settings.methodSettings.filter { it.value.rotationCount > 0 }.keys.toList()
    }
}

