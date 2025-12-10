package com.paykit.demo.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.paykit.demo.PaykitDemoApp
import com.paykit.storage.AutoPayRule
import com.paykit.storage.AutoPayStorage
import com.paykit.storage.PeerSpendingLimit
import com.paykit.storage.RecentAutoPayment
import com.paykit.storage.SpendingPeriod
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.util.UUID

/**
 * ViewModel for Auto-Pay settings and management.
 *
 * Handles all auto-pay configuration including:
 * - Enable/disable toggle
 * - Global daily limits
 * - Per-peer spending limits
 * - Auto-pay rules
 * - Recent payment history
 */
class AutoPayViewModel : ViewModel() {

    private val storage: AutoPayStorage = PaykitDemoApp.instance.autoPayStorage

    private val _uiState = MutableStateFlow(AutoPayUiState())
    val uiState: StateFlow<AutoPayUiState> = _uiState.asStateFlow()

    init {
        loadState()
        loadSampleData()
    }

    private fun loadState() {
        _uiState.update { state ->
            state.copy(
                isEnabled = storage.isEnabled(),
                dailyLimit = storage.getGlobalDailyLimit(),
                usedToday = storage.getUsedToday(),
                peerLimits = storage.getPeerLimits(),
                rules = storage.getRules(),
                recentPayments = storage.getRecentPayments()
            )
        }
    }

    private fun loadSampleData() {
        // Add sample data for demo if none exists
        if (_uiState.value.peerLimits.isEmpty()) {
            val sampleLimits = listOf(
                PeerSpendingLimit(
                    peerPubkey = "pk1abc123def456...",
                    peerName = "Alice's Store",
                    limit = 50000,
                    used = 12500,
                    period = SpendingPeriod.DAILY,
                    periodStart = System.currentTimeMillis()
                ),
                PeerSpendingLimit(
                    peerPubkey = "pk1xyz789ghi012...",
                    peerName = "Coffee Shop",
                    limit = 10000,
                    used = 3200,
                    period = SpendingPeriod.DAILY,
                    periodStart = System.currentTimeMillis()
                )
            )
            sampleLimits.forEach { storage.savePeerLimit(it) }
        }

        if (_uiState.value.rules.isEmpty()) {
            val sampleRules = listOf(
                AutoPayRule(
                    id = UUID.randomUUID().toString(),
                    name = "Small Lightning Payments",
                    description = "Auto-approve Lightning payments under 1000 sats",
                    isEnabled = true,
                    maxAmount = 1000,
                    methodFilter = "lightning",
                    peerFilter = null
                ),
                AutoPayRule(
                    id = UUID.randomUUID().toString(),
                    name = "Trusted Merchants",
                    description = "Auto-approve all payments from verified merchants",
                    isEnabled = false,
                    maxAmount = 10000,
                    methodFilter = null,
                    peerFilter = null
                )
            )
            sampleRules.forEach { storage.saveRule(it) }
        }

        if (_uiState.value.recentPayments.isEmpty()) {
            val now = System.currentTimeMillis()
            val samplePayments = listOf(
                RecentAutoPayment(
                    id = UUID.randomUUID().toString(),
                    peerPubkey = "pk1abc...",
                    peerName = "Alice's Store",
                    amount = 500,
                    description = "Monthly subscription",
                    timestamp = now - 3600000,
                    status = com.paykit.storage.PaymentStatus.COMPLETED,
                    ruleId = null
                ),
                RecentAutoPayment(
                    id = UUID.randomUUID().toString(),
                    peerPubkey = "pk1xyz...",
                    peerName = "Coffee Shop",
                    amount = 320,
                    description = "Morning coffee",
                    timestamp = now - 7200000,
                    status = com.paykit.storage.PaymentStatus.COMPLETED,
                    ruleId = null
                )
            )
            samplePayments.forEach { storage.addRecentPayment(it) }
        }

        // Reload after adding sample data
        loadState()
    }

    fun setEnabled(enabled: Boolean) {
        storage.setEnabled(enabled)
        _uiState.update { it.copy(isEnabled = enabled) }
    }

    fun setDailyLimit(limit: Long) {
        storage.setGlobalDailyLimit(limit)
        _uiState.update { it.copy(dailyLimit = limit) }
    }

    fun addPeerLimit(peerLimit: PeerSpendingLimit) {
        storage.savePeerLimit(peerLimit)
        _uiState.update { state ->
            state.copy(peerLimits = storage.getPeerLimits())
        }
    }

    fun removePeerLimit(peerPubkey: String) {
        storage.removePeerLimit(peerPubkey)
        _uiState.update { state ->
            state.copy(peerLimits = storage.getPeerLimits())
        }
    }

    fun addRule(rule: AutoPayRule) {
        storage.saveRule(rule)
        _uiState.update { state ->
            state.copy(rules = storage.getRules())
        }
    }

    fun toggleRule(ruleId: String) {
        val rule = _uiState.value.rules.find { it.id == ruleId } ?: return
        storage.saveRule(rule.copy(isEnabled = !rule.isEnabled))
        _uiState.update { state ->
            state.copy(rules = storage.getRules())
        }
    }

    fun removeRule(ruleId: String) {
        storage.removeRule(ruleId)
        _uiState.update { state ->
            state.copy(rules = storage.getRules())
        }
    }

    fun resetToDefaults() {
        storage.resetToDefaults()
        loadState()
        loadSampleData()
    }

    /**
     * Check if a payment should be auto-approved.
     */
    fun checkAutoApproval(
        peerPubkey: String,
        amount: Long,
        methodId: String
    ): AutoApprovalCheckResult {
        val result = storage.checkAutoApproval(peerPubkey, amount, methodId)
        return when (result) {
            is com.paykit.storage.AutoApprovalResult.Approved ->
                AutoApprovalCheckResult.Approved(result.ruleId, result.ruleName)
            is com.paykit.storage.AutoApprovalResult.Denied ->
                AutoApprovalCheckResult.Denied(result.reason)
            com.paykit.storage.AutoApprovalResult.NeedsApproval ->
                AutoApprovalCheckResult.NeedsApproval
        }
    }

    /**
     * Record an auto-payment (updates limits and adds to history).
     */
    fun recordPayment(
        peerPubkey: String,
        peerName: String,
        amount: Long,
        description: String,
        ruleId: String?
    ) {
        viewModelScope.launch {
            storage.recordAutoPayment(peerPubkey, peerName, amount, description, ruleId)
            loadState()
        }
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
    val rules: List<AutoPayRule> = emptyList(),
    val recentPayments: List<RecentAutoPayment> = emptyList()
)

/**
 * Result of auto-approval check.
 */
sealed class AutoApprovalCheckResult {
    data class Approved(val ruleId: String, val ruleName: String) : AutoApprovalCheckResult()
    data class Denied(val reason: String) : AutoApprovalCheckResult()
    object NeedsApproval : AutoApprovalCheckResult()
}
