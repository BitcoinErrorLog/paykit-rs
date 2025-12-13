package com.paykit.demo.storage

import android.content.Context
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.util.Date

/**
 * Manages persistent storage of payment requests using EncryptedSharedPreferences.
 */
class PaymentRequestStorage(context: Context, private val identityName: String) {
    
    companion object {
        private const val TAG = "PaymentRequestStorage"
        private const val MAX_REQUESTS_TO_KEEP = 200
    }
    
    private val PREFS_NAME = "paykit_payment_requests_$identityName"
    private val REQUESTS_KEY = "requests_list"
    
    private val json = Json { 
        ignoreUnknownKeys = true 
        encodeDefaults = true
    }
    
    private val prefs by lazy {
        try {
            val masterKey = MasterKey.Builder(context)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build()
            
            EncryptedSharedPreferences.create(
                context,
                PREFS_NAME,
                masterKey,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
            )
        } catch (e: Exception) {
            Log.e(TAG, "Failed to create encrypted prefs, falling back to regular prefs", e)
            context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        }
    }
    
    // In-memory cache
    private var requestsCache: List<StoredPaymentRequest>? = null
    
    // MARK: - CRUD Operations
    
    /**
     * Get all requests (newest first)
     */
    fun listRequests(): List<StoredPaymentRequest> {
        requestsCache?.let { return it }
        
        return try {
            val jsonString = prefs.getString(REQUESTS_KEY, null) ?: return emptyList()
            val requests = json.decodeFromString<List<StoredPaymentRequest>>(jsonString)
                .sortedByDescending { it.createdAt }
            requestsCache = requests
            requests
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load requests: ${e.message}")
            emptyList()
        }
    }
    
    /**
     * Get pending requests only
     */
    fun pendingRequests(): List<StoredPaymentRequest> {
        return listRequests().filter { it.status == PaymentRequestStatus.PENDING }
    }
    
    /**
     * Get requests filtered by status
     */
    fun listRequests(status: PaymentRequestStatus): List<StoredPaymentRequest> {
        return listRequests().filter { it.status == status }
    }
    
    /**
     * Get requests filtered by direction
     */
    fun listRequests(direction: RequestDirection): List<StoredPaymentRequest> {
        return listRequests().filter { it.direction == direction }
    }
    
    /**
     * Get recent requests (limited count)
     */
    fun recentRequests(limit: Int = 10): List<StoredPaymentRequest> {
        return listRequests().take(limit)
    }
    
    /**
     * Get a specific request
     */
    fun getRequest(id: String): StoredPaymentRequest? {
        return listRequests().find { it.id == id }
    }
    
    /**
     * Add a new request
     */
    fun addRequest(request: StoredPaymentRequest) {
        val requests = listRequests().toMutableList()
        
        // Add new request at the beginning (newest first)
        requests.add(0, request)
        
        // Trim to max size
        val trimmed = if (requests.size > MAX_REQUESTS_TO_KEEP) {
            requests.take(MAX_REQUESTS_TO_KEEP)
        } else {
            requests
        }
        
        persistRequests(trimmed)
    }
    
    /**
     * Update an existing request
     */
    fun updateRequest(request: StoredPaymentRequest) {
        val requests = listRequests().toMutableList()
        val index = requests.indexOfFirst { it.id == request.id }
        
        if (index >= 0) {
            requests[index] = request
            persistRequests(requests)
        }
    }
    
    /**
     * Update request status
     */
    fun updateStatus(id: String, status: PaymentRequestStatus) {
        val request = getRequest(id) ?: return
        updateRequest(request.copy(status = status))
    }
    
    /**
     * Delete a request
     */
    fun deleteRequest(id: String) {
        val requests = listRequests().toMutableList()
        requests.removeAll { it.id == id }
        persistRequests(requests)
    }
    
    /**
     * Check and mark expired requests
     */
    fun checkExpirations() {
        val now = System.currentTimeMillis()
        val requests = listRequests().toMutableList()
        var hasChanges = false
        
        for (i in requests.indices) {
            val request = requests[i]
            if (request.status == PaymentRequestStatus.PENDING && 
                request.expiresAt != null && 
                request.expiresAt < now) {
                requests[i] = request.copy(status = PaymentRequestStatus.EXPIRED)
                hasChanges = true
            }
        }
        
        if (hasChanges) {
            persistRequests(requests)
        }
    }
    
    /**
     * Clear all requests
     */
    fun clearAll() {
        persistRequests(emptyList())
    }
    
    // MARK: - Statistics
    
    /**
     * Count of pending requests
     */
    fun pendingCount(): Int {
        return listRequests(PaymentRequestStatus.PENDING).size
    }
    
    /**
     * Count of incoming pending requests
     */
    fun incomingPendingCount(): Int {
        return listRequests(RequestDirection.INCOMING).count { it.status == PaymentRequestStatus.PENDING }
    }
    
    /**
     * Count of outgoing pending requests
     */
    fun outgoingPendingCount(): Int {
        return listRequests(RequestDirection.OUTGOING).count { it.status == PaymentRequestStatus.PENDING }
    }
    
    // MARK: - Private
    
    private fun persistRequests(requests: List<StoredPaymentRequest>) {
        try {
            val jsonString = json.encodeToString(requests)
            prefs.edit().putString(REQUESTS_KEY, jsonString).apply()
            requestsCache = requests
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save requests: ${e.message}")
        }
    }
}

// MARK: - Data Models

/**
 * A payment request stored in persistent storage
 */
@Serializable
data class StoredPaymentRequest(
    val id: String,
    val fromPubkey: String,
    val toPubkey: String,
    val amountSats: Long,
    val currency: String,
    val methodId: String,
    val description: String,
    val createdAt: Long,
    val expiresAt: Long?,
    val status: PaymentRequestStatus,
    val direction: RequestDirection
) {
    /**
     * Display name for the counterparty
     */
    val counterpartyName: String
        get() {
            // In a real app, this would look up the contact name
            val key = if (direction == RequestDirection.INCOMING) fromPubkey else toPubkey
            return if (key.length > 12) {
                "${key.take(6)}...${key.takeLast(4)}"
            } else {
                key
            }
        }
    
    /**
     * Get createdAt as Date
     */
    val createdAtDate: Date
        get() = Date(createdAt)
    
    /**
     * Get expiresAt as Date
     */
    val expiresAtDate: Date?
        get() = expiresAt?.let { Date(it) }
    
    companion object {
        /**
         * Create from FFI PaymentRequest
         */
        fun fromFFI(
            ffiRequest: com.paykit.mobile.PaymentRequest, 
            direction: RequestDirection
        ): StoredPaymentRequest {
            return StoredPaymentRequest(
                id = ffiRequest.requestId,
                fromPubkey = ffiRequest.fromPubkey,
                toPubkey = ffiRequest.toPubkey,
                amountSats = ffiRequest.amountSats,
                currency = ffiRequest.currency,
                methodId = ffiRequest.methodId,
                description = ffiRequest.description,
                createdAt = ffiRequest.createdAt * 1000, // Convert to milliseconds
                expiresAt = ffiRequest.expiresAt?.let { it * 1000 },
                status = PaymentRequestStatus.PENDING,
                direction = direction
            )
        }
    }
}

/**
 * Status of a payment request
 */
@Serializable
enum class PaymentRequestStatus {
    PENDING,
    ACCEPTED,
    DECLINED,
    EXPIRED,
    PAID;
    
    val displayName: String
        get() = when (this) {
            PENDING -> "Pending"
            ACCEPTED -> "Accepted"
            DECLINED -> "Declined"
            EXPIRED -> "Expired"
            PAID -> "Paid"
        }
}

/**
 * Direction of the request (incoming = someone is requesting from you)
 */
@Serializable
enum class RequestDirection {
    INCOMING,  // Someone is requesting payment from you
    OUTGOING   // You are requesting payment from someone
}

