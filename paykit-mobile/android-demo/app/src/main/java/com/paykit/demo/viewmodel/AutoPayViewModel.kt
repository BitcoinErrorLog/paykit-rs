package com.paykit.demo.viewmodel

import androidx.lifecycle.ViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/**
 * ViewModel for Auto-Pay settings and management.
 * 
 * Note: This is a demo stub. Full implementation will integrate with 
 * the Paykit SDK storage layer.
 */
class AutoPayViewModel : ViewModel() {

    private val _uiState = MutableStateFlow(AutoPayUiState())
    val uiState: StateFlow<AutoPayUiState> = _uiState.asStateFlow()

    fun setEnabled(enabled: Boolean) {
        _uiState.update { it.copy(isEnabled = enabled) }
    }

    fun setDailyLimit(limit: Long) {
        _uiState.update { it.copy(dailyLimit = limit) }
    }

    fun resetToDefaults() {
        _uiState.value = AutoPayUiState()
    }
}

/**
 * UI state for Auto-Pay screen.
 */
data class AutoPayUiState(
    val isEnabled: Boolean = false,
    val dailyLimit: Long = 100000,
    val usedToday: Long = 0
)
