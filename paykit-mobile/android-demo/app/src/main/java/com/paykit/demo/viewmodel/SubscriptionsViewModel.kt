package com.paykit.demo.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import com.paykit.demo.model.StoredSubscription
import com.paykit.demo.storage.SubscriptionStorage
import com.paykit.mobile.KeyManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/**
 * ViewModel for Subscriptions management.
 * 
 * Uses EncryptedSharedPreferences-backed SubscriptionStorage for secure persistence.
 */
class SubscriptionsViewModel(application: Application) : AndroidViewModel(application) {

    private val keyManager = KeyManager(application)
    private val storage: SubscriptionStorage
        get() {
            val identityName = keyManager.currentIdentityName.value ?: "default"
            return SubscriptionStorage(application, identityName)
        }
    
    private val _uiState = MutableStateFlow(SubscriptionsUiState())
    val uiState: StateFlow<SubscriptionsUiState> = _uiState.asStateFlow()

    init {
        loadFromStorage()
    }

    fun loadSubscriptions() {
        loadFromStorage()
    }

    fun createSubscription(
        providerName: String,
        providerPubkey: String,
        amountSats: Long,
        frequency: String,
        description: String,
        methodId: String = "lightning"
    ) {
        val nextPayment = StoredSubscription.calculateNextPayment(frequency)
        
        val subscription = StoredSubscription(
            providerName = providerName,
            providerPubkey = providerPubkey,
            amountSats = amountSats,
            frequency = frequency,
            description = description,
            methodId = methodId,
            nextPaymentAt = nextPayment
        )
        
        storage.saveSubscription(subscription)
        loadFromStorage()
        
        _uiState.update { it.copy(showSuccess = true, errorMessage = null) }
    }

    fun toggleSubscription(id: String) {
        storage.toggleActive(id)
        loadFromStorage()
    }

    fun deleteSubscription(id: String) {
        storage.deleteSubscription(id)
        loadFromStorage()
    }

    fun recordPayment(subscriptionId: String) {
        storage.recordPayment(subscriptionId)
        loadFromStorage()
    }

    fun dismissSuccess() {
        _uiState.update { it.copy(showSuccess = false) }
    }

    fun clearError() {
        _uiState.update { it.copy(errorMessage = null) }
    }

    /**
     * Calculate proration for plan changes.
     */
    fun calculateProration(
        currentAmountSats: Long,
        newAmountSats: Long,
        daysIntoPeriod: Int,
        periodDays: Int = 30
    ): ProrationResult {
        val daysRemaining = periodDays - daysIntoPeriod
        val creditPerDay = currentAmountSats.toDouble() / periodDays
        val chargePerDay = newAmountSats.toDouble() / periodDays
        
        val credit = (creditPerDay * daysRemaining).toLong()
        val charge = (chargePerDay * daysRemaining).toLong()
        val net = charge - credit
        
        return ProrationResult(
            creditSats = credit,
            chargeSats = charge,
            netSats = kotlin.math.abs(net),
            isRefund = net < 0
        )
    }

    private fun loadFromStorage() {
        val subscriptions = storage.listSubscriptions()
        _uiState.update { it.copy(subscriptions = subscriptions) }
    }
}

/**
 * UI state for Subscriptions screen.
 */
data class SubscriptionsUiState(
    val subscriptions: List<StoredSubscription> = emptyList(),
    val showSuccess: Boolean = false,
    val errorMessage: String? = null
)

/**
 * Result of proration calculation.
 */
data class ProrationResult(
    val creditSats: Long,
    val chargeSats: Long,
    val netSats: Long,
    val isRefund: Boolean
)

