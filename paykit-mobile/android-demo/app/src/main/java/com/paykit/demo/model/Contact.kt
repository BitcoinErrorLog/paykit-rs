package com.paykit.demo.model

import kotlinx.serialization.Serializable

/**
 * A payment contact (recipient)
 */
@Serializable
data class Contact(
    /** Unique identifier (same as public key) */
    val id: String,
    /** Public key in z-base32 format */
    val publicKeyZ32: String,
    /** Display name */
    var name: String,
    /** Optional notes */
    var notes: String? = null,
    /** When the contact was added (Unix timestamp) */
    val createdAt: Long = System.currentTimeMillis(),
    /** Last payment to this contact (Unix timestamp, if any) */
    var lastPaymentAt: Long? = null,
    /** Total number of payments to this contact */
    var paymentCount: Int = 0
) {
    companion object {
        /**
         * Create a new contact from public key and name
         */
        fun create(publicKeyZ32: String, name: String, notes: String? = null): Contact {
            return Contact(
                id = publicKeyZ32,
                publicKeyZ32 = publicKeyZ32,
                name = name,
                notes = notes
            )
        }
    }
    
    /**
     * Record a payment to this contact
     */
    fun recordPayment(): Contact {
        return copy(
            lastPaymentAt = System.currentTimeMillis(),
            paymentCount = paymentCount + 1
        )
    }
    
    /**
     * Abbreviated public key for display (first and last 8 chars)
     */
    val abbreviatedKey: String
        get() {
            if (publicKeyZ32.length <= 16) return publicKeyZ32
            return "${publicKeyZ32.take(8)}...${publicKeyZ32.takeLast(8)}"
        }
}

