package com.paykit.demo.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import com.paykit.demo.model.AutoPayRule
import com.paykit.demo.model.PeerSpendingLimit
import com.paykit.demo.storage.AutoPayStorage
import com.paykit.mobile.KeyManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/**
 * ViewModel for Auto-Pay settings and management.
 * 
 * Uses EncryptedSharedPreferences-backed AutoPayStorage for secure persistence.
 */
class AutoPayViewModel(application: Application) : AndroidViewModel(application) {

    private val keyManager = KeyManager(application)
    private val storage: AutoPayStorage
        get() {
            val identityName = keyManager.currentIdentityName.value ?: "default"
            return AutoPayStorage(application, identityName)
        }
    
    private val _uiState = MutableStateFlow(AutoPayUiState())
    val uiState: StateFlow<AutoPayUiState> = _uiState.asStateFlow()

    init {
        loadFromStorage()
    }

    fun setEnabled(enabled: Boolean) {
        _uiState.update { it.copy(isEnabled = enabled) }
        saveSettingsToStorage()
    }

    fun setDailyLimit(limit: Long) {
        _uiState.update { it.copy(dailyLimit = limit) }
        saveSettingsToStorage()
    }

    fun addPeerLimit(limit: PeerSpendingLimit) {
        val currentLimits = _uiState.value.peerLimits.toMutableList()
        currentLimits.add(limit)
        _uiState.update { it.copy(peerLimits = currentLimits) }
        storage.savePeerLimit(limit)
    }

    fun updatePeerLimit(limit: PeerSpendingLimit) {
        val currentLimits = _uiState.value.peerLimits.toMutableList()
        val index = currentLimits.indexOfFirst { it.id == limit.id }
        if (index >= 0) {
            currentLimits[index] = limit
            _uiState.update { it.copy(peerLimits = currentLimits) }
            storage.savePeerLimit(limit)
        }
    }

    fun removePeerLimit(id: String) {
        val currentLimits = _uiState.value.peerLimits.toMutableList()
        currentLimits.removeAll { it.id == id }
        _uiState.update { it.copy(peerLimits = currentLimits) }
        storage.deletePeerLimit(id)
    }

    fun addRule(rule: AutoPayRule) {
        val currentRules = _uiState.value.autoPayRules.toMutableList()
        currentRules.add(rule)
        _uiState.update { it.copy(autoPayRules = currentRules) }
        storage.saveRule(rule)
    }

    fun updateRule(rule: AutoPayRule) {
        val currentRules = _uiState.value.autoPayRules.toMutableList()
        val index = currentRules.indexOfFirst { it.id == rule.id }
        if (index >= 0) {
            currentRules[index] = rule
            _uiState.update { it.copy(autoPayRules = currentRules) }
            storage.saveRule(rule)
        }
    }

    fun removeRule(id: String) {
        val currentRules = _uiState.value.autoPayRules.toMutableList()
        currentRules.removeAll { it.id == id }
        _uiState.update { it.copy(autoPayRules = currentRules) }
        storage.deleteRule(id)
    }

    fun resetToDefaults() {
        // Clear storage
        val settings = storage.getSettings()
        storage.saveSettings(settings.copy(
            isEnabled = false,
            globalDailyLimitSats = 100000,
            currentDailySpentSats = 0
        ))
        
        // Clear all limits and rules
        storage.getPeerLimits().forEach { storage.deletePeerLimit(it.id) }
        storage.getRules().forEach { storage.deleteRule(it.id) }
        
        _uiState.value = AutoPayUiState()
    }

    fun resetDailyUsage() {
        _uiState.update { it.copy(usedToday = 0) }
        
        val settings = storage.getSettings()
        storage.saveSettings(settings.copy(currentDailySpentSats = 0))
        
        // Reset daily peer limits
        val updatedLimits = _uiState.value.peerLimits.map { limit ->
            if (limit.period.lowercase() == "daily") {
                val updated = limit.copy(spentSats = 0)
                storage.savePeerLimit(updated)
                updated
            } else {
                limit
            }
        }
        _uiState.update { it.copy(peerLimits = updatedLimits) }
    }

    private fun loadFromStorage() {
        val settings = storage.getSettings()
        val peerLimits = storage.getPeerLimits()
        val rules = storage.getRules()
        
        _uiState.value = AutoPayUiState(
            isEnabled = settings.isEnabled,
            dailyLimit = settings.globalDailyLimitSats,
            usedToday = settings.currentDailySpentSats,
            peerLimits = peerLimits,
            autoPayRules = rules
        )
    }

    private fun saveSettingsToStorage() {
        val currentState = _uiState.value
        var settings = storage.getSettings()
        settings = settings.copy(
            isEnabled = currentState.isEnabled,
            globalDailyLimitSats = currentState.dailyLimit,
            currentDailySpentSats = currentState.usedToday
        )
        storage.saveSettings(settings)
    }
}

/**
 * UI state for Auto-Pay screen.
 */
data class AutoPayUiState(
    val isEnabled: Boolean = false,
    val dailyLimit: Long = 100000,
    val usedToday: Long = 0,
    val peerLimits: List<PeerSpendingLimit> = emptyList(),
    val autoPayRules: List<AutoPayRule> = emptyList()
)
