package com.paykit.demo.model

import kotlinx.serialization.Serializable
import java.util.UUID

/**
 * A stored subscription
 */
@Serializable
data class StoredSubscription(
    val id: String = UUID.randomUUID().toString(),
    var providerName: String,
    var providerPubkey: String,
    var amountSats: Long,
    var currency: String = "SAT",
    var frequency: String,  // daily, weekly, monthly, yearly
    var description: String,
    var methodId: String = "lightning",
    var isActive: Boolean = true,
    val createdAt: Long = System.currentTimeMillis(),
    var lastPaymentAt: Long? = null,
    var nextPaymentAt: Long? = null,
    var paymentCount: Int = 0
) {
    companion object {
        fun calculateNextPayment(frequency: String, fromMs: Long = System.currentTimeMillis()): Long? {
            val oneDay = 24 * 60 * 60 * 1000L
            return when (frequency.lowercase()) {
                "daily" -> fromMs + oneDay
                "weekly" -> fromMs + (7 * oneDay)
                "monthly" -> fromMs + (30 * oneDay)
                "yearly" -> fromMs + (365 * oneDay)
                else -> null
            }
        }
    }
    
    fun recordPayment(): StoredSubscription {
        return copy(
            lastPaymentAt = System.currentTimeMillis(),
            paymentCount = paymentCount + 1,
            nextPaymentAt = calculateNextPayment(frequency)
        )
    }
}

