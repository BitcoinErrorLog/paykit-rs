// NoisePaymentViewModel.kt
// ViewModel for Noise Protocol Payments
//
// This ViewModel coordinates the payment flow using:
// - NoisePaymentService for encrypted communication
// - PubkyRingIntegration for key management
// - Local storage for receipts

package com.paykit.demo.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.paykit.demo.services.*
import com.paykit.demo.model.PaymentStatus
import com.paykit.demo.storage.ContactStorage
import com.paykit.demo.storage.ReceiptStorage
import com.paykit.demo.storage.StoredReceipt
import com.paykit.mobile.KeyManager
import com.paykit.mobile.NoiseEndpointInfo
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * Payment flow state
 */
enum class NoisePaymentState {
    IDLE,
    RESOLVING_RECIPIENT,
    DERIVING_KEYS,
    DISCOVERING_ENDPOINT,
    CONNECTING,
    HANDSHAKING,
    SENDING_REQUEST,
    AWAITING_CONFIRMATION,
    COMPLETED,
    FAILED,
    CANCELLED;
    
    val isProcessing: Boolean
        get() = when (this) {
            IDLE, COMPLETED, FAILED, CANCELLED -> false
            else -> true
        }
    
    val progress: Float
        get() = when (this) {
            IDLE -> 0f
            RESOLVING_RECIPIENT -> 0.1f
            DERIVING_KEYS -> 0.15f
            DISCOVERING_ENDPOINT -> 0.25f
            CONNECTING -> 0.4f
            HANDSHAKING -> 0.55f
            SENDING_REQUEST -> 0.7f
            AWAITING_CONFIRMATION -> 0.85f
            COMPLETED -> 1f
            FAILED, CANCELLED -> 0f
        }
    
    val description: String
        get() = when (this) {
            IDLE -> "Ready to send payment"
            RESOLVING_RECIPIENT -> "Resolving recipient..."
            DERIVING_KEYS -> "Preparing encryption keys..."
            DISCOVERING_ENDPOINT -> "Discovering payment endpoint..."
            CONNECTING -> "Connecting to recipient..."
            HANDSHAKING -> "Establishing secure channel..."
            SENDING_REQUEST -> "Sending payment request..."
            AWAITING_CONFIRMATION -> "Awaiting confirmation..."
            COMPLETED -> "Payment completed!"
            FAILED -> "Payment failed"
            CANCELLED -> "Payment cancelled"
        }
}

/**
 * ViewModel for sending payments via Noise protocol
 */
class NoisePaymentViewModel(application: Application) : AndroidViewModel(application) {
    
    // Form fields
    private val _recipientInput = MutableStateFlow("")
    val recipientInput: StateFlow<String> = _recipientInput.asStateFlow()
    
    private val _amount = MutableStateFlow("1000")
    val amount: StateFlow<String> = _amount.asStateFlow()
    
    private val _currency = MutableStateFlow("SAT")
    val currency: StateFlow<String> = _currency.asStateFlow()
    
    private val _paymentMethod = MutableStateFlow("lightning")
    val paymentMethod: StateFlow<String> = _paymentMethod.asStateFlow()
    
    private val _memo = MutableStateFlow("")
    val memo: StateFlow<String> = _memo.asStateFlow()
    
    // State
    private val _state = MutableStateFlow(NoisePaymentState.IDLE)
    val state: StateFlow<NoisePaymentState> = _state.asStateFlow()
    
    private val _completedReceiptId = MutableStateFlow<String?>(null)
    val completedReceiptId: StateFlow<String?> = _completedReceiptId.asStateFlow()
    
    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage.asStateFlow()
    
    // Dependencies
    private val context = application.applicationContext
    private val paymentService = NoisePaymentService.getInstance(context)
    private val keyManager = KeyManager(context)
    
    private var cancellationRequested = false
    
    // Computed properties
    val canSend: Boolean
        get() = recipientInput.value.isNotBlank() && 
                amount.value.isNotBlank() && 
                !state.value.isProcessing
    
    val isProcessing: Boolean
        get() = state.value.isProcessing
    
    // Form setters
    fun setRecipientInput(value: String) { _recipientInput.value = value }
    fun setAmount(value: String) { _amount.value = value }
    fun setCurrency(value: String) { _currency.value = value }
    fun setPaymentMethod(value: String) { _paymentMethod.value = value }
    fun setMemo(value: String) { _memo.value = value }
    
    /**
     * Send a payment
     */
    fun sendPayment() {
        if (!canSend) return
        
        cancellationRequested = false
        
        viewModelScope.launch {
            try {
                // Step 1: Resolve recipient
                _state.value = NoisePaymentState.RESOLVING_RECIPIENT
                val payeePubkey = resolveRecipient(recipientInput.value)
                
                if (cancellationRequested) throw NoisePaymentException.Cancelled
                
                // Step 2: Derive/get encryption keys
                _state.value = NoisePaymentState.DERIVING_KEYS
                paymentService.getOrDeriveKeys()
                
                if (cancellationRequested) throw NoisePaymentException.Cancelled
                
                // Step 3: Discover endpoint
                _state.value = NoisePaymentState.DISCOVERING_ENDPOINT
                val endpoint = discoverEndpoint(payeePubkey)
                
                if (cancellationRequested) throw NoisePaymentException.Cancelled
                
                // Step 4: Connect
                _state.value = NoisePaymentState.CONNECTING
                paymentService.connect(endpoint)
                
                if (cancellationRequested) throw NoisePaymentException.Cancelled
                
                // Step 5: Handshake (done in connect)
                _state.value = NoisePaymentState.HANDSHAKING
                
                if (cancellationRequested) throw NoisePaymentException.Cancelled
                
                // Step 6: Send request
                _state.value = NoisePaymentState.SENDING_REQUEST
                val request = createPaymentRequest(payeePubkey)
                
                _state.value = NoisePaymentState.AWAITING_CONFIRMATION
                val response = paymentService.sendPaymentRequest(request)
                
                // Step 7: Handle response
                if (response.success && response.receiptId != null) {
                    // Save receipt
                    saveReceipt(request, response.confirmedAt ?: System.currentTimeMillis())
                    
                    _completedReceiptId.value = response.receiptId
                    _state.value = NoisePaymentState.COMPLETED
                } else {
                    _errorMessage.value = response.errorMessage ?: "Payment rejected"
                    _state.value = NoisePaymentState.FAILED
                }
                
            } catch (e: NoisePaymentException.Cancelled) {
                _state.value = NoisePaymentState.CANCELLED
            } catch (e: Exception) {
                _errorMessage.value = e.message ?: "Unknown error"
                _state.value = NoisePaymentState.FAILED
            }
            
            // Cleanup
            paymentService.disconnect()
        }
    }
    
    /**
     * Cancel the current payment
     */
    fun cancel() {
        cancellationRequested = true
        paymentService.disconnect()
        _state.value = NoisePaymentState.CANCELLED
    }
    
    /**
     * Reset form to initial state
     */
    fun reset() {
        _recipientInput.value = ""
        _amount.value = "1000"
        _currency.value = "SAT"
        _paymentMethod.value = "lightning"
        _memo.value = ""
        _state.value = NoisePaymentState.IDLE
        _completedReceiptId.value = null
        _errorMessage.value = null
    }
    
    /**
     * Clear error message
     */
    fun clearError() {
        _errorMessage.value = null
    }
    
    // Helper methods
    
    private fun resolveRecipient(input: String): String {
        // Handle pubky:// URI
        if (input.startsWith("pubky://")) {
            return input.removePrefix("pubky://")
        }
        
        // Try to find in contacts
        val identityName = keyManager.currentIdentityName.value ?: "default"
        val storage = ContactStorage(context, identityName)
        val contacts = storage.listContacts()
        
        contacts.find { 
            it.name.equals(input, ignoreCase = true) ||
            it.publicKeyZ32.equals(input, ignoreCase = true)
        }?.let {
            return it.publicKeyZ32
        }
        
        // Assume it's a raw public key
        return input
    }
    
    private suspend fun discoverEndpoint(pubkey: String): NoiseEndpointInfo {
        return try {
            paymentService.discoverEndpoint(pubkey)
        } catch (e: NoisePaymentException.EndpointNotFound) {
            // Check for manual test endpoint
            getTestEndpoint() ?: throw e
        }
    }
    
    private fun getTestEndpoint(): NoiseEndpointInfo? {
        val host = System.getenv("PAYKIT_TEST_HOST") ?: return null
        val port = System.getenv("PAYKIT_TEST_PORT")?.toIntOrNull() ?: return null
        val pubkey = System.getenv("PAYKIT_TEST_PUBKEY") ?: return null
        
        return NoiseEndpointInfo(
            recipientPubkey = "", // Unknown from test environment
            host = host,
            port = port.toUShort(),
            serverNoisePubkey = pubkey,
            metadata = null
        )
    }
    
    private fun createPaymentRequest(payeePubkey: String): NoisePaymentRequest {
        val payerPubkey = keyManager.publicKeyZ32.value
        
        return NoisePaymentRequest(
            payerPubkey = payerPubkey,
            payeePubkey = payeePubkey,
            methodId = paymentMethod.value,
            amount = amount.value,
            currency = currency.value,
            description = memo.value.ifEmpty { null }
        )
    }
    
    private fun saveReceipt(request: NoisePaymentRequest, confirmedAt: Long) {
        val identityName = keyManager.currentIdentityName.value ?: "default"
        val storage = ReceiptStorage(context, identityName)
        
        val receipt = StoredReceipt(
            id = request.receiptId,
            payer = request.payerPubkey,
            payee = request.payeePubkey,
            amount = request.amount?.toLongOrNull() ?: 0,
            currency = request.currency ?: "SAT",
            method = request.methodId,
            timestamp = confirmedAt,
            status = "completed",
            notes = request.description
        )
        
        storage.saveReceipt(receipt)
    }
}

/**
 * ViewModel for receiving payments via Noise protocol
 */
class NoiseReceiveViewModel(application: Application) : AndroidViewModel(application) {
    
    // State
    private val _isListening = MutableStateFlow(false)
    val isListening: StateFlow<Boolean> = _isListening.asStateFlow()
    
    private val _listeningPort = MutableStateFlow<Int?>(null)
    val listeningPort: StateFlow<Int?> = _listeningPort.asStateFlow()
    
    private val _noisePubkeyHex = MutableStateFlow<String?>(null)
    val noisePubkeyHex: StateFlow<String?> = _noisePubkeyHex.asStateFlow()
    
    private val _pendingRequests = MutableStateFlow<List<PendingPaymentRequest>>(emptyList())
    val pendingRequests: StateFlow<List<PendingPaymentRequest>> = _pendingRequests.asStateFlow()
    
    private val _recentReceipts = MutableStateFlow<List<StoredReceipt>>(emptyList())
    val recentReceipts: StateFlow<List<StoredReceipt>> = _recentReceipts.asStateFlow()
    
    private val _activeConnections = MutableStateFlow(0)
    val activeConnections: StateFlow<Int> = _activeConnections.asStateFlow()
    
    // Dependencies
    private val context = getApplication<Application>().applicationContext
    private val paymentService = NoisePaymentService.getInstance(context)
    private val keyManager = KeyManager(context)
    
    init {
        setupCallbacks()
    }
    
    /**
     * Set up callbacks from the payment service
     */
    private fun setupCallbacks() {
        // Handle pending payment requests
        paymentService.onPendingPaymentRequest = { request ->
            viewModelScope.launch {
                val pendingRequest = PendingPaymentRequest(
                    id = request.id,
                    payerPubkey = request.payerPubkey,
                    amount = request.amount,
                    currency = request.currency,
                    methodId = request.methodId,
                    receivedAt = request.receivedAt
                )
                _pendingRequests.value = _pendingRequests.value + pendingRequest
            }
        }
        
        // Handle confirmed receipts
        paymentService.onReceiptConfirmed = { receipt ->
            viewModelScope.launch {
                // Remove from pending
                _pendingRequests.value = _pendingRequests.value.filter { it.id != receipt.receiptId }
                // Reload receipts
                loadRecentReceipts()
            }
        }
    }
    
    /**
     * Pending payment request from a payer
     */
    data class PendingPaymentRequest(
        val id: String,
        val payerPubkey: String,
        val amount: String?,
        val currency: String?,
        val methodId: String,
        val receivedAt: Long
    )
    
    /**
     * Refresh server status
     */
    fun refreshStatus() {
        val status = paymentService.getServerStatus()
        _isListening.value = status.isRunning
        _listeningPort.value = status.port
        _noisePubkeyHex.value = status.noisePubkeyHex
        _activeConnections.value = status.activeConnections
    }
    
    /**
     * Start listening for payments
     */
    fun startListening(port: Int = 0) {
        viewModelScope.launch {
            try {
                val status = paymentService.startServer(port)
                
                _isListening.value = status.isRunning
                _listeningPort.value = status.port
                _noisePubkeyHex.value = status.noisePubkeyHex
                
            } catch (e: Exception) {
                // Handle error
            }
        }
    }
    
    /**
     * Stop listening
     */
    fun stopListening() {
        paymentService.stopServer()
        _isListening.value = false
        _listeningPort.value = null
    }
    
    /**
     * Get connection info for sharing
     */
    fun getConnectionInfo(): String? {
        val port = listeningPort.value ?: return null
        val pubkey = noisePubkeyHex.value ?: return null
        
        val host = getLocalIPAddress() ?: "localhost"
        
        return "$host:$port:$pubkey"
    }
    
    /**
     * Accept a pending payment request
     */
    fun acceptRequest(request: PendingPaymentRequest) {
        viewModelScope.launch {
            // Would send confirmation back to payer
            _pendingRequests.value = _pendingRequests.value.filter { it.id != request.id }
        }
    }
    
    /**
     * Decline a pending payment request
     */
    fun declineRequest(request: PendingPaymentRequest) {
        viewModelScope.launch {
            // Would send rejection back to payer
            _pendingRequests.value = _pendingRequests.value.filter { it.id != request.id }
        }
    }
    
    /**
     * Load recent receipts
     */
    fun loadRecentReceipts() {
        viewModelScope.launch {
            val identityName = keyManager.currentIdentityName.value ?: "default"
            val storage = ReceiptStorage(context, identityName)
            // Convert Receipt to StoredReceipt for compatibility
            _recentReceipts.value = storage.listReceipts()
                .map { receipt ->
                    StoredReceipt(
                        id = receipt.id,
                        payer = receipt.counterpartyKey, // Approximate - should track payer/payee separately
                        payee = receipt.counterpartyKey,
                        amount = receipt.amountSats,
                        currency = "SAT", // Default
                        method = receipt.paymentMethod,
                        timestamp = receipt.createdAt,
                        status = when (receipt.status) {
                            PaymentStatus.COMPLETED -> "completed"
                            PaymentStatus.PENDING -> "pending"
                            PaymentStatus.FAILED -> "failed"
                            PaymentStatus.REFUNDED -> "refunded"
                        },
                        notes = receipt.memo
                    )
                }
                .sortedByDescending { it.timestamp }
                .take(10)
        }
    }
    
    private fun getLocalIPAddress(): String? {
        try {
            val interfaces = java.net.NetworkInterface.getNetworkInterfaces()
            while (interfaces.hasMoreElements()) {
                val intf = interfaces.nextElement()
                val addrs = intf.inetAddresses
                while (addrs.hasMoreElements()) {
                    val addr = addrs.nextElement()
                    if (!addr.isLoopbackAddress && addr is java.net.Inet4Address) {
                        return addr.hostAddress
                    }
                }
            }
        } catch (e: Exception) {
            // Ignore
        }
        return null
    }
}

