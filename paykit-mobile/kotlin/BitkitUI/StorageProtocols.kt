package com.paykit.mobile.bitkit

import com.paykit.mobile.paykit_mobile.Receipt
import com.paykit.mobile.paykit_mobile.Subscription
import com.paykit.mobile.paykit_mobile.PaymentRequest

// MARK: - Receipt Storage

interface ReceiptStorageProtocol {
    fun recentReceipts(limit: Int): List<Receipt>
    fun totalSent(): Long
    fun totalReceived(): Long
    fun pendingCount(): Int
    fun saveReceipt(receipt: Receipt)
    fun deleteReceipt(id: String)
    fun receipt(id: String): Receipt?
}

// MARK: - Contact Storage

interface ContactStorageProtocol {
    fun listContacts(): List<Contact>
    fun addContact(contact: Contact)
    fun updateContact(contact: Contact)
    fun deleteContact(id: String)
    fun contact(id: String): Contact?
    fun contact(pubkey: String): Contact?
}

// MARK: - AutoPay Storage

interface AutoPayStorageProtocol {
    fun getSettings(): AutoPaySettings
    fun saveSettings(settings: AutoPaySettings)
    fun getPeerLimits(): List<StoredPeerLimit>
    fun savePeerLimit(limit: StoredPeerLimit)
    fun deletePeerLimit(id: String)
    fun getRules(): List<StoredAutoPayRule>
    fun saveRule(rule: StoredAutoPayRule)
    fun deleteRule(id: String)
}

// MARK: - Subscription Storage

interface SubscriptionStorageProtocol {
    fun activeSubscriptions(): List<Subscription>
    fun addSubscription(subscription: Subscription)
    fun updateSubscription(subscription: Subscription)
    fun deleteSubscription(id: String)
    fun subscription(id: String): Subscription?
}

// MARK: - Payment Request Storage

interface PaymentRequestStorageProtocol {
    fun pendingRequests(): List<PaymentRequest>
    fun requestHistory(): List<PaymentRequest>
    fun pendingCount(): Int
    fun addRequest(request: PaymentRequest)
    fun updateRequest(request: PaymentRequest)
    fun deleteRequest(id: String)
    fun request(id: String): PaymentRequest?
}

// MARK: - Models

data class Contact(
    val id: String = java.util.UUID.randomUUID().toString(),
    val name: String,
    val pubkey: String,
    val createdAt: Long = System.currentTimeMillis(),
    val updatedAt: Long = System.currentTimeMillis()
)

data class AutoPaySettings(
    val isEnabled: Boolean = false,
    val globalDailyLimitSats: Long = 100000L
) {
    companion object {
        val defaults = AutoPaySettings()
    }
}

data class StoredPeerLimit(
    val id: String,
    val peerPubkey: String,
    val peerName: String,
    val limitSats: Long,
    val spentSats: Long = 0L,
    val period: String = "daily",
    val lastResetDate: Long = System.currentTimeMillis()
)

data class StoredAutoPayRule(
    val id: String = java.util.UUID.randomUUID().toString(),
    val name: String,
    val isEnabled: Boolean = true,
    val maxAmountSats: Long? = null,
    val allowedMethods: List<String> = emptyList(),
    val allowedPeers: List<String> = emptyList(),
    val requireConfirmation: Boolean = false,
    val createdAt: Long = System.currentTimeMillis()
)
