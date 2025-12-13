package com.paykit.demo.model

import kotlinx.serialization.Serializable
import java.util.UUID

/**
 * Payment status
 */
@Serializable
enum class PaymentStatus {
    PENDING,
    COMPLETED,
    FAILED,
    REFUNDED
}

/**
 * Payment direction
 */
@Serializable
enum class PaymentDirection {
    SENT,
    RECEIVED
}

/**
 * A payment receipt
 */
@Serializable
data class Receipt(
    /** Unique identifier */
    val id: String,
    /** Direction of payment */
    val direction: PaymentDirection,
    /** Counterparty public key (z-base32) */
    val counterpartyKey: String,
    /** Counterparty display name (if known) */
    var counterpartyName: String? = null,
    /** Amount in satoshis */
    val amountSats: Long,
    /** Payment status */
    var status: PaymentStatus = PaymentStatus.PENDING,
    /** Payment method used */
    val paymentMethod: String,
    /** When the payment was initiated (Unix timestamp) */
    val createdAt: Long = System.currentTimeMillis(),
    /** When the payment was completed (Unix timestamp, if applicable) */
    var completedAt: Long? = null,
    /** Optional memo/note */
    var memo: String? = null,
    /** Transaction ID (if applicable) */
    var txId: String? = null,
    /** Payment proof (optional, JSON string) */
    var proof: String? = null,
    /** Whether proof has been verified */
    var proofVerified: Boolean = false,
    /** Timestamp when proof was verified (Unix timestamp in milliseconds) */
    var proofVerifiedAt: Long? = null
) {
    companion object {
        /**
         * Create a new receipt
         */
        fun create(
            direction: PaymentDirection,
            counterpartyKey: String,
            counterpartyName: String? = null,
            amountSats: Long,
            paymentMethod: String,
            memo: String? = null
        ): Receipt {
            return Receipt(
                id = UUID.randomUUID().toString(),
                direction = direction,
                counterpartyKey = counterpartyKey,
                counterpartyName = counterpartyName,
                amountSats = amountSats,
                paymentMethod = paymentMethod,
                memo = memo
            )
        }
        
        /**
         * Create a local Receipt from an FFI Receipt
         * @param ffiReceipt The FFI Receipt from PaykitClient.createReceipt()
         * @param direction Whether this is a sent or received payment
         * @param counterpartyName Optional display name for the counterparty
         * @return A local Receipt for storage
         */
        fun fromFFI(
            ffiReceipt: com.paykit.mobile.Receipt,
            direction: PaymentDirection,
            counterpartyName: String? = null
        ): Receipt {
            val counterpartyKey = if (direction == PaymentDirection.SENT) {
                ffiReceipt.payee
            } else {
                ffiReceipt.payer
            }
            val amountSats = ffiReceipt.amount?.toLongOrNull() ?: 0L
            
            return Receipt(
                id = ffiReceipt.receiptId,
                direction = direction,
                counterpartyKey = counterpartyKey,
                counterpartyName = counterpartyName,
                amountSats = amountSats,
                paymentMethod = ffiReceipt.methodId,
                createdAt = ffiReceipt.createdAt * 1000 // Convert to milliseconds
            )
        }
    }
    
    /**
     * Mark proof as verified
     */
    fun markProofVerified(): Receipt {
        return this.copy(
            proofVerified = true,
            proofVerifiedAt = System.currentTimeMillis()
        )
    }
    
    /**
     * Mark as completed
     */
    fun complete(txId: String? = null): Receipt {
        return copy(
            status = PaymentStatus.COMPLETED,
            completedAt = System.currentTimeMillis(),
            txId = txId
        )
    }
    
    /**
     * Mark as failed
     */
    fun fail(): Receipt {
        return copy(status = PaymentStatus.FAILED)
    }
    
    /**
     * Abbreviated counterparty key for display
     */
    val abbreviatedCounterparty: String
        get() {
            if (counterpartyKey.length <= 16) return counterpartyKey
            return "${counterpartyKey.take(8)}...${counterpartyKey.takeLast(8)}"
        }
    
    /**
     * Display name (name if known, otherwise abbreviated key)
     */
    val displayName: String
        get() = counterpartyName ?: abbreviatedCounterparty
    
    /**
     * Formatted amount with direction sign
     */
    val formattedAmount: String
        get() {
            val sign = if (direction == PaymentDirection.SENT) "-" else "+"
            return "$sign${String.format("%,d", amountSats)} sats"
        }
}

