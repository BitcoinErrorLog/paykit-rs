// PrivateEndpointStorage.kt
// Paykit Android Private Endpoint Storage
//
// This file provides specialized storage for private payment endpoints
// using EncryptedSharedPreferences.
//
// USAGE:
//   val storage = EncryptedPreferencesStorage.create(context)
//   val endpointStorage = PrivateEndpointStorage(storage)
//   
//   // Save a private endpoint
//   endpointStorage.save(PrivateEndpoint(...))
//   
//   // Retrieve endpoints for a peer
//   val endpoints = endpointStorage.listForPeer(peerPubkey)

package com.paykit.storage

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject

/**
 * Storage for private payment endpoints.
 *
 * Private endpoints are payment addresses or channels that are shared only
 * with specific peers for enhanced privacy and payment routing.
 *
 * All data is stored encrypted using the underlying EncryptedPreferencesStorage.
 *
 * @param storage The underlying encrypted storage
 */
class PrivateEndpointStorage(
    private val storage: EncryptedPreferencesStorage
) {
    companion object {
        private const val KEY_PREFIX = "private_endpoint."
        private const val KEY_PEERS = "private_endpoint.peers"
    }

    // MARK: - Endpoint Operations

    /**
     * Save a private endpoint.
     *
     * If an endpoint for this peer/method already exists, it will be replaced.
     */
    fun save(endpoint: PrivateEndpoint) {
        val key = endpointKey(endpoint.peerPubkey, endpoint.methodId)
        storage.store(key, endpoint.toJson().toString())
        
        // Track known peers
        val peers = getKnownPeers().toMutableSet()
        peers.add(endpoint.peerPubkey)
        savePeers(peers)
    }

    /**
     * Get a specific endpoint.
     *
     * @param peerPubkey The peer's public key
     * @param methodId The payment method ID
     * @return The endpoint, or null if not found
     */
    fun get(peerPubkey: String, methodId: String): PrivateEndpoint? {
        val key = endpointKey(peerPubkey, methodId)
        val json = storage.retrieveString(key) ?: return null
        
        return try {
            val endpoint = PrivateEndpoint.fromJson(JSONObject(json))
            // Check expiration
            if (endpoint.isExpired()) {
                remove(peerPubkey, methodId)
                null
            } else {
                endpoint
            }
        } catch (e: Exception) {
            null
        }
    }

    /**
     * List all endpoints for a peer.
     */
    fun listForPeer(peerPubkey: String): List<PrivateEndpoint> {
        val endpoints = mutableListOf<PrivateEndpoint>()
        val keys = storage.listKeys()
        
        for (key in keys) {
            if (key.startsWith("$KEY_PREFIX$peerPubkey:")) {
                val json = storage.retrieveString(key) ?: continue
                try {
                    val endpoint = PrivateEndpoint.fromJson(JSONObject(json))
                    if (!endpoint.isExpired()) {
                        endpoints.add(endpoint)
                    } else {
                        // Clean up expired endpoint
                        storage.delete(key)
                    }
                } catch (e: Exception) {
                    // Skip invalid entries
                }
            }
        }
        
        return endpoints
    }

    /**
     * List all known peers with private endpoints.
     */
    fun listPeers(): List<String> {
        return getKnownPeers().filter { peer ->
            listForPeer(peer).isNotEmpty()
        }
    }

    /**
     * Remove a specific endpoint.
     */
    fun remove(peerPubkey: String, methodId: String) {
        val key = endpointKey(peerPubkey, methodId)
        storage.delete(key)
        
        // Clean up peer if no more endpoints
        if (listForPeer(peerPubkey).isEmpty()) {
            val peers = getKnownPeers().toMutableSet()
            peers.remove(peerPubkey)
            savePeers(peers)
        }
    }

    /**
     * Remove all endpoints for a peer.
     */
    fun removeAllForPeer(peerPubkey: String) {
        val endpoints = listForPeer(peerPubkey)
        for (endpoint in endpoints) {
            remove(peerPubkey, endpoint.methodId)
        }
    }

    /**
     * Clean up all expired endpoints.
     *
     * @return Number of endpoints removed
     */
    fun cleanupExpired(): Int {
        var count = 0
        
        for (peer in getKnownPeers()) {
            val keys = storage.listKeys()
            for (key in keys) {
                if (key.startsWith("$KEY_PREFIX$peer:")) {
                    val json = storage.retrieveString(key) ?: continue
                    try {
                        val endpoint = PrivateEndpoint.fromJson(JSONObject(json))
                        if (endpoint.isExpired()) {
                            storage.delete(key)
                            count++
                        }
                    } catch (e: Exception) {
                        // Skip
                    }
                }
            }
        }
        
        return count
    }

    /**
     * Get the best endpoint for a peer, preferring unexpired ones.
     */
    fun getBestEndpoint(peerPubkey: String, methodId: String): PrivateEndpoint? {
        return get(peerPubkey, methodId)
    }

    /**
     * Clear all private endpoints.
     */
    fun clear() {
        for (peer in getKnownPeers()) {
            removeAllForPeer(peer)
        }
        storage.delete(KEY_PEERS)
    }

    // MARK: - Statistics

    /**
     * Get count of all stored endpoints.
     */
    fun count(): Int {
        var total = 0
        for (peer in getKnownPeers()) {
            total += listForPeer(peer).size
        }
        return total
    }

    /**
     * Get count of expired endpoints (not yet cleaned up).
     */
    fun expiredCount(): Int {
        var count = 0
        for (peer in getKnownPeers()) {
            val keys = storage.listKeys()
            for (key in keys) {
                if (key.startsWith("$KEY_PREFIX$peer:")) {
                    val json = storage.retrieveString(key) ?: continue
                    try {
                        val endpoint = PrivateEndpoint.fromJson(JSONObject(json))
                        if (endpoint.isExpired()) {
                            count++
                        }
                    } catch (e: Exception) {
                        // Skip
                    }
                }
            }
        }
        return count
    }

    // MARK: - Private Helpers

    private fun endpointKey(peerPubkey: String, methodId: String): String {
        return "$KEY_PREFIX$peerPubkey:$methodId"
    }

    private fun getKnownPeers(): Set<String> {
        val json = storage.retrieveString(KEY_PEERS) ?: return emptySet()
        return try {
            val array = JSONArray(json)
            (0 until array.length()).map { array.getString(it) }.toSet()
        } catch (e: Exception) {
            emptySet()
        }
    }

    private fun savePeers(peers: Set<String>) {
        val array = JSONArray()
        peers.forEach { array.put(it) }
        storage.store(KEY_PEERS, array.toString())
    }
}

// MARK: - Data Classes

/**
 * Endpoint policy for access control.
 */
data class EndpointPolicy(
    val maxPayments: Int?,
    val maxAmountPerPayment: Long?,
    val maxTotalAmount: Long?,
    val allowedMethodIds: List<String>?
) {
    fun toJson(): JSONObject {
        return JSONObject().apply {
            maxPayments?.let { put("maxPayments", it) }
            maxAmountPerPayment?.let { put("maxAmountPerPayment", it) }
            maxTotalAmount?.let { put("maxTotalAmount", it) }
            allowedMethodIds?.let { 
                val arr = JSONArray()
                it.forEach { m -> arr.put(m) }
                put("allowedMethodIds", arr)
            }
        }
    }

    companion object {
        fun fromJson(json: JSONObject): EndpointPolicy {
            return EndpointPolicy(
                maxPayments = if (json.has("maxPayments")) json.getInt("maxPayments") else null,
                maxAmountPerPayment = if (json.has("maxAmountPerPayment")) json.getLong("maxAmountPerPayment") else null,
                maxTotalAmount = if (json.has("maxTotalAmount")) json.getLong("maxTotalAmount") else null,
                allowedMethodIds = if (json.has("allowedMethodIds")) {
                    val arr = json.getJSONArray("allowedMethodIds")
                    (0 until arr.length()).map { arr.getString(it) }
                } else null
            )
        }

        val UNRESTRICTED = EndpointPolicy(null, null, null, null)
    }
}

/**
 * Private payment endpoint.
 */
data class PrivateEndpoint(
    val peerPubkey: String,
    val methodId: String,
    val endpoint: String,
    val policy: EndpointPolicy?,
    val createdAt: Long,
    val expiresAt: Long?,
    val metadata: Map<String, String>?
) {
    /**
     * Check if this endpoint has expired.
     */
    fun isExpired(): Boolean {
        val exp = expiresAt ?: return false
        return System.currentTimeMillis() > exp
    }

    /**
     * Get remaining time until expiration in milliseconds.
     */
    fun remainingTimeMs(): Long? {
        val exp = expiresAt ?: return null
        return maxOf(0, exp - System.currentTimeMillis())
    }

    fun toJson(): JSONObject {
        return JSONObject().apply {
            put("peerPubkey", peerPubkey)
            put("methodId", methodId)
            put("endpoint", endpoint)
            policy?.let { put("policy", it.toJson()) }
            put("createdAt", createdAt)
            expiresAt?.let { put("expiresAt", it) }
            metadata?.let {
                val metaObj = JSONObject()
                it.forEach { (k, v) -> metaObj.put(k, v) }
                put("metadata", metaObj)
            }
        }
    }

    companion object {
        fun fromJson(json: JSONObject): PrivateEndpoint {
            return PrivateEndpoint(
                peerPubkey = json.getString("peerPubkey"),
                methodId = json.getString("methodId"),
                endpoint = json.getString("endpoint"),
                policy = if (json.has("policy")) EndpointPolicy.fromJson(json.getJSONObject("policy")) else null,
                createdAt = json.getLong("createdAt"),
                expiresAt = if (json.has("expiresAt")) json.getLong("expiresAt") else null,
                metadata = if (json.has("metadata")) {
                    val metaObj = json.getJSONObject("metadata")
                    metaObj.keys().asSequence().associateWith { metaObj.getString(it) }
                } else null
            )
        }

        /**
         * Create a new endpoint with default settings.
         */
        fun create(
            peerPubkey: String,
            methodId: String,
            endpoint: String,
            expiresInMs: Long? = null
        ): PrivateEndpoint {
            val now = System.currentTimeMillis()
            return PrivateEndpoint(
                peerPubkey = peerPubkey,
                methodId = methodId,
                endpoint = endpoint,
                policy = null,
                createdAt = now,
                expiresAt = expiresInMs?.let { now + it },
                metadata = null
            )
        }
    }
}

// MARK: - Coroutine Extensions

suspend fun PrivateEndpointStorage.saveAsync(endpoint: PrivateEndpoint) =
    withContext(Dispatchers.IO) { save(endpoint) }

suspend fun PrivateEndpointStorage.getAsync(peerPubkey: String, methodId: String): PrivateEndpoint? =
    withContext(Dispatchers.IO) { get(peerPubkey, methodId) }

suspend fun PrivateEndpointStorage.listForPeerAsync(peerPubkey: String): List<PrivateEndpoint> =
    withContext(Dispatchers.IO) { listForPeer(peerPubkey) }

suspend fun PrivateEndpointStorage.removeAsync(peerPubkey: String, methodId: String) =
    withContext(Dispatchers.IO) { remove(peerPubkey, methodId) }

suspend fun PrivateEndpointStorage.cleanupExpiredAsync(): Int =
    withContext(Dispatchers.IO) { cleanupExpired() }

/**
 * Extension function to create PrivateEndpointStorage from EncryptedPreferencesStorage.
 */
fun EncryptedPreferencesStorage.asPrivateEndpointStorage(): PrivateEndpointStorage {
    return PrivateEndpointStorage(this)
}
