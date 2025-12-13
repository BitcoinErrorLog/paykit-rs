package com.paykit.demo

import android.util.Log
import com.paykit.mobile.*

/**
 * Wrapper around the FFI PaykitClient that provides null-safe access
 * and convenience methods for the demo app.
 */
class PaykitClientWrapper private constructor(
    private val client: PaykitClient?
) {
    companion object {
        private const val TAG = "PaykitClientWrapper"
        
        /**
         * Try to create a wrapper with a real client.
         * Returns a wrapper with null client on failure.
         */
        fun create(): PaykitClientWrapper {
            return try {
                val client = PaykitClient()
                Log.d(TAG, "PaykitClient initialized successfully")
                PaykitClientWrapper(client)
            } catch (e: Exception) {
                Log.e(TAG, "Failed to initialize PaykitClient: ${e.message}")
                PaykitClientWrapper(null)
            }
        }
        
        /**
         * Create a placeholder wrapper for error states.
         */
        fun placeholder(): PaykitClientWrapper = PaykitClientWrapper(null)
    }
    
    val isAvailable: Boolean
        get() = client != null
    
    // MARK: - Payment Methods
    
    fun listMethods(): List<String> {
        return try {
            client?.listMethods() ?: emptyList()
        } catch (e: Exception) {
            Log.e(TAG, "listMethods failed: ${e.message}")
            emptyList()
        }
    }
    
    fun validateEndpoint(methodId: String, endpoint: String): Boolean {
        return try {
            client?.validateEndpoint(methodId, endpoint) ?: false
        } catch (e: Exception) {
            Log.e(TAG, "validateEndpoint failed: ${e.message}")
            false
        }
    }
    
    fun selectMethod(
        methods: List<PaymentMethod>,
        amountSats: ULong,
        preferences: SelectionPreferences?
    ): SelectionResult? {
        return try {
            client?.selectMethod(methods, amountSats, preferences)
        } catch (e: Exception) {
            Log.e(TAG, "selectMethod failed: ${e.message}")
            null
        }
    }
    
    // MARK: - Health
    
    fun checkHealth(): List<HealthCheckResult> {
        return try {
            client?.checkHealth() ?: emptyList()
        } catch (e: Exception) {
            Log.e(TAG, "checkHealth failed: ${e.message}")
            emptyList()
        }
    }
    
    fun getHealthStatus(methodId: String): HealthStatus? {
        return try {
            client?.getHealthStatus(methodId)
        } catch (e: Exception) {
            Log.e(TAG, "getHealthStatus failed: ${e.message}")
            null
        }
    }
    
    fun isMethodUsable(methodId: String): Boolean {
        return try {
            client?.isMethodUsable(methodId) ?: false
        } catch (e: Exception) {
            Log.e(TAG, "isMethodUsable failed: ${e.message}")
            false
        }
    }
    
    // MARK: - Subscriptions
    
    fun createSubscription(
        subscriber: String,
        provider: String,
        terms: SubscriptionTerms
    ): Subscription? {
        return try {
            client?.createSubscription(subscriber, provider, terms)
        } catch (e: Exception) {
            Log.e(TAG, "createSubscription failed: ${e.message}")
            null
        }
    }
    
    fun calculateProration(
        currentAmountSats: Long,
        newAmountSats: Long,
        periodStart: Long,
        periodEnd: Long,
        changeDate: Long
    ): ProrationResult? {
        return try {
            client?.calculateProration(currentAmountSats, newAmountSats, periodStart, periodEnd, changeDate)
        } catch (e: Exception) {
            Log.e(TAG, "calculateProration failed: ${e.message}")
            null
        }
    }
    
    fun daysRemainingInPeriod(periodEnd: Long): UInt {
        return try {
            client?.daysRemainingInPeriod(periodEnd) ?: 0u
        } catch (e: Exception) {
            Log.e(TAG, "daysRemainingInPeriod failed: ${e.message}")
            0u
        }
    }
    
    // MARK: - Payment Requests
    
    fun createPaymentRequest(
        fromPubkey: String,
        toPubkey: String,
        amountSats: Long,
        currency: String,
        methodId: String,
        description: String,
        expiresInSecs: ULong?
    ): PaymentRequest? {
        return try {
            client?.createPaymentRequest(fromPubkey, toPubkey, amountSats, currency, methodId, description, expiresInSecs)
        } catch (e: Exception) {
            Log.e(TAG, "createPaymentRequest failed: ${e.message}")
            null
        }
    }
    
    // MARK: - Receipts
    
    fun createReceipt(
        payer: String,
        payee: String,
        methodId: String,
        amount: String?,
        currency: String?
    ): Receipt? {
        return try {
            client?.createReceipt(payer, payee, methodId, amount, currency)
        } catch (e: Exception) {
            Log.e(TAG, "createReceipt failed: ${e.message}")
            null
        }
    }
    
    fun getPaymentStatus(receiptId: String): PaymentStatusInfo? {
        return try {
            client?.getPaymentStatus(receiptId)
        } catch (e: Exception) {
            Log.e(TAG, "getPaymentStatus failed: ${e.message}")
            null
        }
    }
    
    fun getInProgressPayments(): List<PaymentStatusInfo> {
        return try {
            client?.getInProgressPayments() ?: emptyList()
        } catch (e: Exception) {
            Log.e(TAG, "getInProgressPayments failed: ${e.message}")
            emptyList()
        }
    }
    
    // MARK: - QR Scanning
    
    fun parseScannedQR(data: String): ScannedUri? {
        return try {
            client?.parseScannedQr(data)
        } catch (e: Exception) {
            Log.e(TAG, "parseScannedQR failed: ${e.message}")
            null
        }
    }
    
    fun isPaykitQR(data: String): Boolean {
        return try {
            client?.isPaykitQr(data) ?: false
        } catch (e: Exception) {
            Log.e(TAG, "isPaykitQR failed: ${e.message}")
            false
        }
    }
    
    // MARK: - Directory Operations
    
    /**
     * Create a directory service for fetching contacts and payment endpoints
     */
    fun createDirectoryService(): DirectoryService {
        return DirectoryService()
    }
}

/**
 * Service for interacting with the Pubky directory.
 * Provides access to contacts and payment endpoint discovery.
 */
class DirectoryService {
    companion object {
        private const val TAG = "DirectoryService"
    }
    
    private val directoryOps: DirectoryOperationsAsync
    private val unauthTransport: UnauthenticatedTransportFfi
    
    init {
        // Use mock transport for demo - replace with real transport in production
        unauthTransport = UnauthenticatedTransportFfi.newMock()
        directoryOps = createDirectoryOperationsAsync()
    }
    
    /**
     * Fetch known contacts from a user's Pubky directory
     * @param ownerPubkey The public key of the owner (z-base32 format)
     * @return List of contact public keys
     */
    suspend fun fetchKnownContacts(ownerPubkey: String): List<String> {
        return try {
            directoryOps.fetchKnownContacts(unauthTransport, ownerPubkey)
        } catch (e: Exception) {
            Log.e(TAG, "fetchKnownContacts failed: ${e.message}")
            emptyList()
        }
    }
    
    /**
     * Fetch payment endpoint for a specific method
     * @param ownerPubkey The public key of the payee
     * @param methodId The payment method ID (e.g., "lightning", "onchain")
     * @return The endpoint data if found, null otherwise
     */
    suspend fun fetchPaymentEndpoint(ownerPubkey: String, methodId: String): String? {
        return try {
            directoryOps.fetchPaymentEndpoint(unauthTransport, ownerPubkey, methodId)
        } catch (e: Exception) {
            Log.e(TAG, "fetchPaymentEndpoint failed: ${e.message}")
            null
        }
    }
    
    /**
     * Fetch all supported payment methods for a payee
     * @param ownerPubkey The public key of the payee
     * @return List of payment methods supported by the payee
     */
    suspend fun fetchSupportedPayments(ownerPubkey: String): List<PaymentMethod> {
        return try {
            directoryOps.fetchSupportedPayments(unauthTransport, ownerPubkey)
        } catch (e: Exception) {
            Log.e(TAG, "fetchSupportedPayments failed: ${e.message}")
            emptyList()
        }
    }
}

