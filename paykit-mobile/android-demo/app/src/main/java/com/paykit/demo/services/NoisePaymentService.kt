// NoisePaymentService.kt
// Noise Payment Service for Android
//
// This service coordinates Noise protocol payments, integrating:
// - Key management (PubkyRingIntegration, NoiseKeyCache)
// - Noise handshake (FfiNoiseManager from pubky-noise)
// - Message creation (PaykitMobile FFI)
// - Network transport (Socket)
//
// Key Architecture:
//   - Ed25519 (pkarr) keys are "cold" and managed by Pubky Ring
//   - X25519 (Noise) keys are "hot" and cached locally
//   - All payment messages are encrypted end-to-end

package com.paykit.demo.services

import android.content.Context
import android.os.Build
import com.paykit.mobile.KeyManager
import com.pubky.noise.FfiMobileConfig
import com.pubky.noise.FfiNoiseManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.DataInputStream
import java.io.DataOutputStream
import java.net.InetSocketAddress
import java.net.Socket
import java.util.UUID

/**
 * Information about a recipient's Noise endpoint
 */
data class NoiseEndpointInfo(
    val host: String,
    val port: Int,
    val serverPubkeyHex: String,
    val metadata: String? = null
) {
    val connectionAddress: String
        get() = "$host:$port"
    
    val serverPubkeyBytes: ByteArray
        get() = serverPubkeyHex.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
}

/**
 * A payment request to send over Noise channel
 */
data class NoisePaymentRequest(
    val receiptId: String = "rcpt_${UUID.randomUUID()}",
    val payerPubkey: String,
    val payeePubkey: String,
    val methodId: String,
    val amount: String? = null,
    val currency: String? = null,
    val description: String? = null
)

/**
 * Response from a payment request
 */
data class NoisePaymentResponse(
    val success: Boolean,
    val receiptId: String? = null,
    val confirmedAt: Long? = null,
    val errorCode: String? = null,
    val errorMessage: String? = null
)

/**
 * Exceptions for Noise payment operations
 */
sealed class NoisePaymentException(message: String) : Exception(message) {
    object NoIdentity : NoisePaymentException("No identity configured. Please set up your identity first.")
    class KeyDerivationFailed(msg: String) : NoisePaymentException("Failed to derive encryption keys: $msg")
    object EndpointNotFound : NoisePaymentException("Recipient has no Noise endpoint published.")
    class InvalidEndpoint(msg: String) : NoisePaymentException("Invalid endpoint format: $msg")
    class ConnectionFailed(msg: String) : NoisePaymentException("Connection failed: $msg")
    class HandshakeFailed(msg: String) : NoisePaymentException("Secure handshake failed: $msg")
    class EncryptionFailed(msg: String) : NoisePaymentException("Encryption failed: $msg")
    class DecryptionFailed(msg: String) : NoisePaymentException("Decryption failed: $msg")
    class InvalidResponse(msg: String) : NoisePaymentException("Invalid response: $msg")
    object Timeout : NoisePaymentException("Connection timed out.")
    object Cancelled : NoisePaymentException("Operation was cancelled.")
    class ServerError(code: String, message: String) : NoisePaymentException("Server error [$code]: $message")
}

/**
 * Service for managing Noise protocol payments
 */
class NoisePaymentService private constructor(private val context: Context) {
    
    companion object {
        @Volatile
        private var instance: NoisePaymentService? = null
        
        fun getInstance(context: Context): NoisePaymentService {
            return instance ?: synchronized(this) {
                instance ?: NoisePaymentService(context.applicationContext).also { instance = it }
            }
        }
    }
    
    // State
    private val _isConnected = MutableStateFlow(false)
    val isConnected: StateFlow<Boolean> = _isConnected.asStateFlow()
    
    private val _currentSessionId = MutableStateFlow<String?>(null)
    val currentSessionId: StateFlow<String?> = _currentSessionId.asStateFlow()
    
    // Private properties
    private var noiseManager: FfiNoiseManager? = null
    private var socket: Socket? = null
    private var currentEpoch: UInt = 0u
    
    private val keyManager = KeyManager(context)
    private val keyCache = NoiseKeyCache.getInstance(context)
    private val pubkyRing = PubkyRingIntegration.getInstance(context)
    
    // Configuration
    var connectionTimeoutMs: Int = 30000
    
    // MARK: - Key Management
    
    /**
     * Get or derive X25519 keys for Noise protocol
     */
    suspend fun getOrDeriveKeys(): X25519KeypairResult {
        val deviceId = getDeviceId()
        
        // Try cache first
        keyCache.getKey(deviceId, currentEpoch)?.let { return it }
        
        // Derive via Pubky Ring (or mock)
        return pubkyRing.getOrDeriveKeypair(deviceId, currentEpoch)
    }
    
    /**
     * Get device ID for key derivation
     */
    private fun getDeviceId(): String {
        return "${Build.MANUFACTURER}_${Build.MODEL}_${Build.ID}"
    }
    
    /**
     * Increment epoch for key rotation
     */
    fun rotateKeys() {
        currentEpoch++
        keyCache.clearAllKeys(getDeviceId())
    }
    
    // MARK: - Connection Management
    
    /**
     * Discover Noise endpoint for a recipient
     */
    suspend fun discoverEndpoint(recipientPubkey: String): NoiseEndpointInfo {
        // Check environment variable override
        System.getenv("PAYKIT_PAYEE_NOISE_ENDPOINT")?.let { envEndpoint ->
            return parseEndpointString(envEndpoint)
        }
        
        // Query directory service
        val directoryService = DirectoryService.getInstance(context)
        return directoryService.discoverNoiseEndpoint(recipientPubkey)
            ?: throw NoisePaymentException.EndpointNotFound
    }
    
    /**
     * Parse endpoint string in format: host:port:pubkey_hex
     */
    private fun parseEndpointString(str: String): NoiseEndpointInfo {
        val parts = str.split(":")
        require(parts.size >= 3) { "Expected format: host:port:pubkey_hex" }
        
        val host = parts[0]
        val port = parts[1].toIntOrNull() ?: throw NoisePaymentException.InvalidEndpoint("Invalid port")
        val pubkeyHex = parts[2]
        
        return NoiseEndpointInfo(
            host = host,
            port = port,
            serverPubkeyHex = pubkeyHex,
            metadata = null
        )
    }
    
    /**
     * Connect to a Noise endpoint
     */
    suspend fun connect(endpoint: NoiseEndpointInfo) = withContext(Dispatchers.IO) {
        // Ensure we have keys
        val keypair = getOrDeriveKeys()
        
        // Create socket connection
        try {
            socket = Socket().apply {
                soTimeout = connectionTimeoutMs
                connect(InetSocketAddress(endpoint.host, endpoint.port), connectionTimeoutMs)
            }
        } catch (e: Exception) {
            throw NoisePaymentException.ConnectionFailed(e.message ?: "Unknown error")
        }
        
        // Perform Noise handshake
        performHandshake(endpoint.serverPubkeyBytes, keypair)
        
        _isConnected.value = true
    }
    
    /**
     * Perform Noise IK handshake
     */
    private suspend fun performHandshake(
        serverPubkey: ByteArray,
        localKeypair: X25519KeypairResult
    ) = withContext(Dispatchers.IO) {
        // Get seed from mock service for demo
        val seedData = try {
            MockPubkyRingService.getInstance(context).getEd25519SeedBytes()
        } catch (e: Exception) {
            throw NoisePaymentException.NoIdentity
        }
        
        val deviceIdBytes = getDeviceId().toByteArray()
        
        val config = FfiMobileConfig(
            autoReconnect = false,
            maxReconnectAttempts = 0u,
            reconnectDelayMs = 0u,
            batterySaver = false,
            chunkSize = 32768u
        )
        
        try {
            noiseManager = FfiNoiseManager.newClient(
                config = config,
                clientSeed = seedData,
                clientKid = "paykit-android",
                deviceId = deviceIdBytes
            )
        } catch (e: Exception) {
            throw NoisePaymentException.HandshakeFailed("Failed to create Noise manager: ${e.message}")
        }
        
        // Step 1: Initiate connection
        val initResult = try {
            noiseManager!!.initiateConnection(serverPubkey, null)
        } catch (e: Exception) {
            throw NoisePaymentException.HandshakeFailed("Failed to initiate: ${e.message}")
        }
        
        // Step 2: Send first message
        sendRawData(initResult.firstMessage)
        
        // Step 3: Receive server response
        val response = receiveRawData()
        
        // Step 4: Complete handshake
        val sessionId = try {
            noiseManager!!.completeConnection(initResult.sessionId, response)
        } catch (e: Exception) {
            throw NoisePaymentException.HandshakeFailed("Failed to complete: ${e.message}")
        }
        
        _currentSessionId.value = sessionId
    }
    
    /**
     * Disconnect from current peer
     */
    fun disconnect() {
        _currentSessionId.value?.let { sessionId ->
            noiseManager?.removeSession(sessionId)
        }
        
        try {
            socket?.close()
        } catch (e: Exception) {
            // Ignore close errors
        }
        
        socket = null
        noiseManager = null
        _isConnected.value = false
        _currentSessionId.value = null
    }
    
    // MARK: - Payment Operations
    
    /**
     * Send a payment request
     */
    suspend fun sendPaymentRequest(request: NoisePaymentRequest): NoisePaymentResponse = 
        withContext(Dispatchers.IO) {
            val sessionId = _currentSessionId.value
                ?: throw NoisePaymentException.ConnectionFailed("Not connected")
            val manager = noiseManager
                ?: throw NoisePaymentException.ConnectionFailed("Not connected")
            
            // Create message JSON
            val messageJson = JSONObject().apply {
                put("type", "request_receipt")
                put("receipt_id", request.receiptId)
                put("payer", request.payerPubkey)
                put("payee", request.payeePubkey)
                put("method_id", request.methodId)
                request.amount?.let { put("amount", it) }
                request.currency?.let { put("currency", it) }
                request.description?.let { put("description", it) }
                put("created_at", System.currentTimeMillis() / 1000)
            }
            
            val jsonData = messageJson.toString().toByteArray()
            
            // Encrypt
            val ciphertext = try {
                manager.encrypt(sessionId, jsonData)
            } catch (e: Exception) {
                throw NoisePaymentException.EncryptionFailed(e.message ?: "Unknown error")
            }
            
            // Send with length prefix
            sendLengthPrefixedData(ciphertext)
            
            // Receive response
            val responseCiphertext = receiveLengthPrefixedData()
            
            // Decrypt
            val responsePlaintext = try {
                manager.decrypt(sessionId, responseCiphertext)
            } catch (e: Exception) {
                throw NoisePaymentException.DecryptionFailed(e.message ?: "Unknown error")
            }
            
            // Parse response
            parsePaymentResponse(responsePlaintext, request.receiptId)
        }
    
    /**
     * Parse payment response JSON
     */
    private fun parsePaymentResponse(data: ByteArray, expectedReceiptId: String): NoisePaymentResponse {
        val json = try {
            JSONObject(String(data))
        } catch (e: Exception) {
            throw NoisePaymentException.InvalidResponse("Invalid JSON structure")
        }
        
        return when (val msgType = json.optString("type")) {
            "confirm_receipt" -> {
                NoisePaymentResponse(
                    success = true,
                    receiptId = json.optString("receipt_id"),
                    confirmedAt = json.optLong("confirmed_at"),
                    errorCode = null,
                    errorMessage = null
                )
            }
            "error" -> {
                NoisePaymentResponse(
                    success = false,
                    receiptId = null,
                    confirmedAt = null,
                    errorCode = json.optString("code", "unknown"),
                    errorMessage = json.optString("message", "Unknown error")
                )
            }
            else -> throw NoisePaymentException.InvalidResponse("Unexpected message type: $msgType")
        }
    }
    
    // MARK: - Network I/O
    
    private fun sendRawData(data: ByteArray) {
        val sock = socket ?: throw NoisePaymentException.ConnectionFailed("No connection")
        val output = DataOutputStream(sock.getOutputStream())
        output.writeInt(data.size)
        output.write(data)
        output.flush()
    }
    
    private fun receiveRawData(): ByteArray {
        val sock = socket ?: throw NoisePaymentException.ConnectionFailed("No connection")
        val input = DataInputStream(sock.getInputStream())
        val length = input.readInt()
        val data = ByteArray(length)
        input.readFully(data)
        return data
    }
    
    private fun sendLengthPrefixedData(data: ByteArray) {
        sendRawData(data)
    }
    
    private fun receiveLengthPrefixedData(): ByteArray {
        return receiveRawData()
    }
    
    // MARK: - Server Mode Support
    
    /**
     * Server status
     */
    data class ServerStatus(
        val isRunning: Boolean,
        val port: Int?,
        val noisePubkeyHex: String?,
        val activeConnections: Int
    )
    
    /**
     * Start listening for incoming payment requests
     */
    suspend fun startServer(port: Int = 0): ServerStatus {
        // Get our keys for publishing
        val keypair = getOrDeriveKeys()
        
        // Note: Full server implementation would use ServerSocket
        // For demo, we just return the public key that clients need
        
        return ServerStatus(
            isRunning = false, // Would be true with full implementation
            port = port,
            noisePubkeyHex = keypair.publicKeyHex,
            activeConnections = 0
        )
    }
    
    /**
     * Stop the server
     */
    fun stopServer() {
        // Would stop ServerSocket
    }
    
    /**
     * Get current server status
     */
    fun getServerStatus(): ServerStatus {
        return ServerStatus(
            isRunning = false,
            port = null,
            noisePubkeyHex = null,
            activeConnections = 0
        )
    }
}

