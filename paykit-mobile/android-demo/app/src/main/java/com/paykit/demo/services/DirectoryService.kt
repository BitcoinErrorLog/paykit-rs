// DirectoryService.kt
// Directory Service for Noise Endpoint Discovery
//
// This service provides methods for discovering and publishing
// Noise endpoints in the Pubky directory.
//
// Key Functions:
// - Discover noise endpoints for recipients
// - Publish our own noise endpoint
// - Query payment methods from directory

package com.paykit.demo.services

import android.content.Context
import com.paykit.mobile.*
import org.json.JSONObject

/**
 * Noise endpoint published in directory
 */
data class DirectoryNoiseEndpoint(
    val host: String,
    val port: Int,
    val pubkey: String,
    val metadata: String? = null
)

/**
 * Payment method published in directory
 */
data class DirectoryPaymentMethod(
    val methodId: String,
    val endpoint: String
)

/**
 * Exceptions for directory operations
 */
sealed class DirectoryException(message: String) : Exception(message) {
    object NotConfigured : DirectoryException("Directory service not configured")
    class NetworkError(msg: String) : DirectoryException("Network error: $msg")
    class ParseError(msg: String) : DirectoryException("Parse error: $msg")
    class NotFound(resource: String) : DirectoryException("Not found: $resource")
    class PublishFailed(msg: String) : DirectoryException("Publish failed: $msg")
}

/**
 * Service for interacting with the Pubky directory
 */
class DirectoryService private constructor(private val context: Context) {
    
    companion object {
        private const val PAYKIT_PATH_PREFIX = "/pub/paykit.app/v0/"
        private const val NOISE_ENDPOINT_PATH = "/pub/paykit.app/v0/noise"
        
        @Volatile
        private var instance: DirectoryService? = null
        
        fun getInstance(context: Context): DirectoryService {
            return instance ?: synchronized(this) {
                instance ?: DirectoryService(context.applicationContext).also { instance = it }
            }
        }
    }
    
    // Mock storage for demo (in production, uses Pubky SDK)
    private val mockStorage = mutableMapOf<String, MutableMap<String, String>>()
    
    // Whether to use mock mode
    var useMockMode = true
    
    private val keyManager = KeyManager(context)
    
    // PaykitClient instance for FFI operations
    private var paykitClient: PaykitClient? = null
    
    // Unauthenticated transport for public reads
    private var unauthenticatedTransport: UnauthenticatedTransportFfi? = null
    
    // Authenticated transport for writes (requires session)
    private var authenticatedTransport: AuthenticatedTransportFfi? = null
    
    // Homeserver base URL (optional, for direct homeserver access)
    var homeserverBaseURL: String? = null
    
    init {
        // Initialize PaykitClient
        try {
            paykitClient = PaykitClient()
        } catch (e: Exception) {
            android.util.Log.e("DirectoryService", "Failed to initialize PaykitClient: ${e.message}")
        }
    }
    
    /**
     * Configure real Pubky transport
     */
    fun configurePubkyTransport(homeserverBaseURL: String? = null) {
        this.homeserverBaseURL = homeserverBaseURL
        
        // Create unauthenticated storage adapter
        val unauthAdapter = PubkyUnauthenticatedStorageAdapter(homeserverBaseURL)
        unauthenticatedTransport = UnauthenticatedTransportFfi.fromCallback(unauthAdapter)
    }
    
    /**
     * Configure authenticated transport with session
     */
    fun configureAuthenticatedTransport(sessionId: String, ownerPubkey: String, homeserverBaseURL: String? = null) {
        this.homeserverBaseURL = homeserverBaseURL
        
        // Create authenticated storage adapter
        val authAdapter = PubkyAuthenticatedStorageAdapter(sessionId, homeserverBaseURL)
        authenticatedTransport = AuthenticatedTransportFfi.fromCallback(authAdapter, ownerPubkey)
    }
    
    // MARK: - Noise Endpoint Discovery
    
    /**
     * Discover noise endpoint for a recipient
     */
    suspend fun discoverNoiseEndpoint(recipientPubkey: String): NoiseEndpointInfo? {
        return if (useMockMode) {
            discoverNoiseEndpointMock(recipientPubkey)
        } else {
            discoverNoiseEndpointPubky(recipientPubkey)
        }
    }
    
    /**
     * Mock implementation for demo
     */
    private fun discoverNoiseEndpointMock(recipientPubkey: String): NoiseEndpointInfo? {
        // Check local mock storage
        val userStorage = mockStorage[recipientPubkey] ?: return null
        val endpointJson = userStorage[NOISE_ENDPOINT_PATH] ?: return null
        
        // Parse JSON
        return try {
            val json = JSONObject(endpointJson)
            NoiseEndpointInfo(
                host = json.getString("host"),
                port = json.getInt("port"),
                serverPubkeyHex = json.getString("pubkey"),
                metadata = json.optString("metadata", null)
            )
        } catch (e: Exception) {
            null
        }
    }
    
    /**
     * Pubky SDK implementation
     */
    private suspend fun discoverNoiseEndpointPubky(recipientPubkey: String): NoiseEndpointInfo? {
        val client = paykitClient ?: throw DirectoryException.NotConfigured
        
        // Use configured transport or create a new one
        val transport = unauthenticatedTransport ?: run {
            val adapter = PubkyUnauthenticatedStorageAdapter(homeserverBaseURL)
            UnauthenticatedTransportFfi.fromCallback(adapter).also {
                unauthenticatedTransport = it
            }
        }
        
        return try {
            client.discoverNoiseEndpoint(transport, recipientPubkey)
        } catch (e: Exception) {
            throw DirectoryException.NetworkError(e.message ?: "Unknown error")
        }
    }
    
    // MARK: - Noise Endpoint Publishing
    
    /**
     * Publish our noise endpoint to the directory
     */
    suspend fun publishNoiseEndpoint(
        host: String,
        port: Int,
        noisePubkey: String,
        metadata: String? = null
    ) {
        val entry = DirectoryNoiseEndpoint(
            host = host,
            port = port,
            pubkey = noisePubkey,
            metadata = metadata
        )
        
        if (useMockMode) {
            publishNoiseEndpointMock(entry)
        } else {
            publishNoiseEndpointPubky(entry)
        }
    }
    
    /**
     * Mock implementation
     */
    private fun publishNoiseEndpointMock(entry: DirectoryNoiseEndpoint) {
        val ownerPubkey = keyManager.publicKeyZ32.value
        if (ownerPubkey.isEmpty()) {
            throw DirectoryException.NotConfigured
        }
        
        val json = JSONObject().apply {
            put("host", entry.host)
            put("port", entry.port)
            put("pubkey", entry.pubkey)
            entry.metadata?.let { put("metadata", it) }
        }
        
        if (mockStorage[ownerPubkey] == null) {
            mockStorage[ownerPubkey] = mutableMapOf()
        }
        mockStorage[ownerPubkey]!![NOISE_ENDPOINT_PATH] = json.toString()
    }
    
    /**
     * Pubky SDK implementation
     */
    private suspend fun publishNoiseEndpointPubky(entry: DirectoryNoiseEndpoint) {
        val client = paykitClient ?: throw DirectoryException.NotConfigured
        val transport = authenticatedTransport ?: throw DirectoryException.NotConfigured
        
        try {
            client.publishNoiseEndpoint(
                transport,
                entry.host,
                entry.port.toUInt().toUShort(),
                entry.pubkey,
                entry.metadata
            )
        } catch (e: Exception) {
            throw DirectoryException.PublishFailed(e.message ?: "Unknown error")
        }
    }
    
    /**
     * Remove noise endpoint from directory
     */
    suspend fun removeNoiseEndpoint() {
        if (useMockMode) {
            val ownerPubkey = keyManager.publicKeyZ32.value
            if (ownerPubkey.isEmpty()) {
                throw DirectoryException.NotConfigured
            }
            mockStorage[ownerPubkey]?.remove(NOISE_ENDPOINT_PATH)
        } else {
            val client = paykitClient ?: throw DirectoryException.NotConfigured
            val transport = authenticatedTransport ?: throw DirectoryException.NotConfigured
            
            try {
                client.removeNoiseEndpoint(transport)
            } catch (e: Exception) {
                throw DirectoryException.PublishFailed(e.message ?: "Unknown error")
            }
        }
    }
    
    // MARK: - Payment Method Discovery
    
    /**
     * Discover all payment methods for a recipient
     */
    suspend fun discoverPaymentMethods(recipientPubkey: String): List<DirectoryPaymentMethod> {
        return if (useMockMode) {
            discoverPaymentMethodsMock(recipientPubkey)
        } else {
            discoverPaymentMethodsPubky(recipientPubkey)
        }
    }
    
    /**
     * Mock implementation
     */
    private fun discoverPaymentMethodsMock(recipientPubkey: String): List<DirectoryPaymentMethod> {
        val userStorage = mockStorage[recipientPubkey] ?: return emptyList()
        
        return userStorage.entries
            .filter { it.key.startsWith(PAYKIT_PATH_PREFIX) && it.key != NOISE_ENDPOINT_PATH }
            .map { (path, content) ->
                val methodId = path.removePrefix(PAYKIT_PATH_PREFIX)
                DirectoryPaymentMethod(methodId = methodId, endpoint = content)
            }
    }
    
    /**
     * Pubky SDK implementation
     */
    private suspend fun discoverPaymentMethodsPubky(recipientPubkey: String): List<DirectoryPaymentMethod> {
        val client = paykitClient ?: throw DirectoryException.NotConfigured
        
        // Use configured transport or create a new one
        val transport = unauthenticatedTransport ?: run {
            val adapter = PubkyUnauthenticatedStorageAdapter(homeserverBaseURL)
            UnauthenticatedTransportFfi.fromCallback(adapter).also {
                unauthenticatedTransport = it
            }
        }
        
        return try {
            val supportedPayments = client.fetchSupportedPayments(transport, recipientPubkey)
            supportedPayments.entries.map { entry ->
                DirectoryPaymentMethod(methodId = entry.methodId, endpoint = entry.endpoint)
            }
        } catch (e: Exception) {
            throw DirectoryException.NetworkError(e.message ?: "Unknown error")
        }
    }
    
    // MARK: - Payment Method Publishing
    
    /**
     * Publish a payment method to the directory
     */
    suspend fun publishPaymentMethod(methodId: String, endpoint: String) {
        if (useMockMode) {
            val ownerPubkey = keyManager.publicKeyZ32.value
            if (ownerPubkey.isEmpty()) {
                throw DirectoryException.NotConfigured
            }
            
            val path = "$PAYKIT_PATH_PREFIX$methodId"
            
            if (mockStorage[ownerPubkey] == null) {
                mockStorage[ownerPubkey] = mutableMapOf()
            }
            mockStorage[ownerPubkey]!![path] = endpoint
        } else {
            val client = paykitClient ?: throw DirectoryException.NotConfigured
            val transport = authenticatedTransport ?: throw DirectoryException.NotConfigured
            
            val methodIdObj = MethodId(methodId)
            val endpointData = EndpointData(endpoint)
            
            try {
                client.publishPaymentEndpoint(transport, methodIdObj, endpointData)
            } catch (e: Exception) {
                throw DirectoryException.PublishFailed(e.message ?: "Unknown error")
            }
        }
    }
    
    /**
     * Remove a payment method from the directory
     */
    suspend fun removePaymentMethod(methodId: String) {
        if (useMockMode) {
            val ownerPubkey = keyManager.publicKeyZ32.value
            if (ownerPubkey.isEmpty()) {
                throw DirectoryException.NotConfigured
            }
            
            val path = "$PAYKIT_PATH_PREFIX$methodId"
            mockStorage[ownerPubkey]?.remove(path)
        } else {
            val client = paykitClient ?: throw DirectoryException.NotConfigured
            val transport = authenticatedTransport ?: throw DirectoryException.NotConfigured
            
            val methodIdObj = MethodId(methodId)
            
            try {
                client.removePaymentEndpoint(transport, methodIdObj)
            } catch (e: Exception) {
                throw DirectoryException.PublishFailed(e.message ?: "Unknown error")
            }
        }
    }
    
    // MARK: - Demo Helpers
    
    /**
     * Set up demo data for testing
     */
    fun setupDemoData() {
        val demoRecipient = "demo_recipient_pk"
        
        mockStorage[demoRecipient] = mutableMapOf(
            NOISE_ENDPOINT_PATH to """
                {"host":"127.0.0.1","port":8888,"pubkey":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}
            """.trimIndent(),
            "${PAYKIT_PATH_PREFIX}lightning" to "lnbc1...",
            "${PAYKIT_PATH_PREFIX}onchain" to "bc1q..."
        )
    }
    
    /**
     * Clear all mock data
     */
    fun clearMockData() {
        mockStorage.clear()
    }
}

