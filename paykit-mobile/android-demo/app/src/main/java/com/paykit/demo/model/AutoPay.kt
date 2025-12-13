package com.paykit.demo.model

import kotlinx.serialization.Serializable
import java.util.*

/**
 * Auto-pay global settings
 */
@Serializable
data class AutoPaySettings(
    var isEnabled: Boolean = false,
    var globalDailyLimitSats: Long = 100000L, // 100k sats default
    var currentDailySpentSats: Long = 0L,
    var lastResetDate: Long = System.currentTimeMillis(),
    var requireConfirmation: Boolean = false,
    var notifyOnPayment: Boolean = true
) {
    fun resetIfNeeded(): AutoPaySettings {
        val calendar = Calendar.getInstance()
        val today = calendar.get(Calendar.DAY_OF_YEAR)
        calendar.timeInMillis = lastResetDate
        val resetDay = calendar.get(Calendar.DAY_OF_YEAR)
        
        return if (today != resetDay) {
            copy(
                currentDailySpentSats = 0L,
                lastResetDate = System.currentTimeMillis()
            )
        } else {
            this
        }
    }
    
    val remainingDailyLimitSats: Long
        get() = maxOf(0L, globalDailyLimitSats - currentDailySpentSats)
    
    val dailyUsagePercent: Double
        get() = if (globalDailyLimitSats > 0) {
            currentDailySpentSats.toDouble() / globalDailyLimitSats.toDouble() * 100.0
        } else 0.0
}

/**
 * A peer-specific spending limit
 */
@Serializable
data class PeerSpendingLimit(
    val id: String,
    var peerPubkey: String,
    var peerName: String,
    var limitSats: Long,
    var spentSats: Long = 0L,
    var period: String = "daily",  // daily, weekly, monthly
    var lastResetDate: Long = System.currentTimeMillis()
) {
    companion object {
        fun create(peerPubkey: String, peerName: String, limitSats: Long, period: String = "daily"): PeerSpendingLimit {
            return PeerSpendingLimit(
                id = peerPubkey,
                peerPubkey = peerPubkey,
                peerName = peerName,
                limitSats = limitSats,
                period = period
            )
        }
    }
    
    fun resetIfNeeded(): PeerSpendingLimit {
        val calendar = Calendar.getInstance()
        val calendarReset = Calendar.getInstance().apply { timeInMillis = lastResetDate }
        
        val shouldReset = when (period.lowercase()) {
            "daily" -> calendar.get(Calendar.DAY_OF_YEAR) != calendarReset.get(Calendar.DAY_OF_YEAR)
            "weekly" -> calendar.get(Calendar.WEEK_OF_YEAR) != calendarReset.get(Calendar.WEEK_OF_YEAR)
            "monthly" -> calendar.get(Calendar.MONTH) != calendarReset.get(Calendar.MONTH)
            else -> false
        }
        
        return if (shouldReset) {
            copy(spentSats = 0L, lastResetDate = System.currentTimeMillis())
        } else {
            this
        }
    }
    
    val remainingSats: Long
        get() = maxOf(0L, limitSats - spentSats)
    
    val usagePercent: Double
        get() = if (limitSats > 0) {
            spentSats.toDouble() / limitSats.toDouble() * 100.0
        } else 0.0
}

/**
 * An auto-pay rule
 */
@Serializable
data class AutoPayRule(
    val id: String = UUID.randomUUID().toString(),
    var name: String,
    var isEnabled: Boolean = true,
    var maxAmountSats: Long? = null,
    var allowedMethods: List<String> = emptyList(),
    var allowedPeers: List<String> = emptyList(),  // Empty = all peers
    var requireConfirmation: Boolean = false,
    val createdAt: Long = System.currentTimeMillis()
) {
    fun matches(amount: Long, method: String, peer: String): Boolean {
        if (!isEnabled) return false
        
        // Check amount
        maxAmountSats?.let { max ->
            if (amount > max) return false
        }
        
        // Check method
        if (allowedMethods.isNotEmpty() && !allowedMethods.contains(method)) {
            return false
        }
        
        // Check peer
        if (allowedPeers.isNotEmpty() && !allowedPeers.contains(peer)) {
            return false
        }
        
        return true
    }
}

