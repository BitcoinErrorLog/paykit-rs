package com.paykit.demo.storage

/**
 * Simple receipt storage model for Noise payments
 */
data class StoredReceipt(
    val id: String,
    val payer: String,
    val payee: String,
    val amount: Long,
    val currency: String,
    val method: String,
    val timestamp: Long,
    val status: String,
    val notes: String? = null
)

