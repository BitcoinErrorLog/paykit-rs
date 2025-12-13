package com.paykit.demo.storage

import android.content.Context
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.paykit.demo.model.AutoPayRule
import com.paykit.demo.model.AutoPaySettings
import com.paykit.demo.model.PeerSpendingLimit
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Manages persistent storage of auto-pay settings using EncryptedSharedPreferences.
 */
class AutoPayStorage(context: Context) {
    
    companion object {
        private const val TAG = "AutoPayStorage"
        private const val PREFS_NAME = "paykit_autopay"
        private const val SETTINGS_KEY = "autopay_settings"
        private const val LIMITS_KEY = "autopay_limits"
        private const val RULES_KEY = "autopay_rules"
    }
    
    private val json = Json { 
        ignoreUnknownKeys = true 
        encodeDefaults = true
    }
    
    private val prefs by lazy {
        try {
            val masterKey = MasterKey.Builder(context)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build()
            
            EncryptedSharedPreferences.create(
                context,
                PREFS_NAME,
                masterKey,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
            )
        } catch (e: Exception) {
            Log.e(TAG, "Failed to create encrypted prefs, falling back", e)
            context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        }
    }
    
    private var settingsCache: AutoPaySettings? = null
    private var limitsCache: List<PeerSpendingLimit>? = null
    private var rulesCache: List<AutoPayRule>? = null
    
    // MARK: - Settings
    
    fun getSettings(): AutoPaySettings {
        settingsCache?.let { return it.resetIfNeeded() }
        
        return try {
            val jsonString = prefs.getString(SETTINGS_KEY, null) ?: return AutoPaySettings()
            val settings = json.decodeFromString<AutoPaySettings>(jsonString).resetIfNeeded()
            settingsCache = settings
            settings
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load settings: ${e.message}")
            AutoPaySettings()
        }
    }
    
    fun saveSettings(settings: AutoPaySettings) {
        try {
            val jsonString = json.encodeToString(settings)
            prefs.edit().putString(SETTINGS_KEY, jsonString).apply()
            settingsCache = settings
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save settings: ${e.message}")
        }
    }
    
    // MARK: - Peer Limits
    
    fun getPeerLimits(): List<PeerSpendingLimit> {
        limitsCache?.let { cached ->
            return cached.map { limit -> limit.resetIfNeeded() }
        }
        
        return try {
            val jsonString = prefs.getString(LIMITS_KEY, null) ?: return emptyList()
            val decoded: List<PeerSpendingLimit> = json.decodeFromString(jsonString)
            val limits: List<PeerSpendingLimit> = decoded.map { limit -> limit.resetIfNeeded() }
            limitsCache = limits
            limits
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load limits: ${e.message}")
            emptyList()
        }
    }
    
    fun savePeerLimit(limit: PeerSpendingLimit) {
        val limits = getPeerLimits().toMutableList()
        val index = limits.indexOfFirst { it.id == limit.id }
        
        if (index >= 0) {
            limits[index] = limit
        } else {
            limits.add(limit)
        }
        
        persistLimits(limits)
    }
    
    fun deletePeerLimit(id: String) {
        val limits = getPeerLimits().toMutableList()
        limits.removeAll { it.id == id }
        persistLimits(limits)
    }
    
    // MARK: - Rules
    
    fun getRules(): List<AutoPayRule> {
        rulesCache?.let { return it }
        
        return try {
            val jsonString = prefs.getString(RULES_KEY, null) ?: return emptyList()
            val rules = json.decodeFromString<List<AutoPayRule>>(jsonString)
            rulesCache = rules
            rules
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load rules: ${e.message}")
            emptyList()
        }
    }
    
    fun saveRule(rule: AutoPayRule) {
        val rules = getRules().toMutableList()
        val index = rules.indexOfFirst { it.id == rule.id }
        
        if (index >= 0) {
            rules[index] = rule
        } else {
            rules.add(rule)
        }
        
        persistRules(rules)
    }
    
    fun deleteRule(id: String) {
        val rules = getRules().toMutableList()
        rules.removeAll { it.id == id }
        persistRules(rules)
    }
    
    // MARK: - Private
    
    private fun persistLimits(limits: List<PeerSpendingLimit>) {
        try {
            val jsonString = json.encodeToString(limits)
            prefs.edit().putString(LIMITS_KEY, jsonString).apply()
            limitsCache = limits
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save limits: ${e.message}")
        }
    }
    
    private fun persistRules(rules: List<AutoPayRule>) {
        try {
            val jsonString = json.encodeToString(rules)
            prefs.edit().putString(RULES_KEY, jsonString).apply()
            rulesCache = rules
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save rules: ${e.message}")
        }
    }
}

