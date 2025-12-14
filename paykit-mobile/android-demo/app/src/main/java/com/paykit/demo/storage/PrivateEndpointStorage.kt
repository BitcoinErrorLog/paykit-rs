package com.paykit.demo.storage

import android.content.Context
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.paykit.mobile.PrivateEndpointOffer
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.decodeFromString

/**
 * Manages persistent storage of private payment endpoints using EncryptedSharedPreferences.
 */
class PrivateEndpointStorage(context: Context, private val identityName: String) {
    
    companion object {
        private const val TAG = "PrivateEndpointStorage"
    }
    
    private val PREFS_NAME = "paykit_private_endpoints_$identityName"
    private val ENDPOINTS_KEY = "endpoints_map"
    
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
    private var endpointsCache: Map<String, List<PrivateEndpointOffer>>? = null
    
    // MARK: - CRUD Operations
    
    /**
     * Get all private endpoints for a peer
     */
    fun listForPeer(peerPubkey: String): List<PrivateEndpointOffer> {
        val all = loadAllEndpoints()
        return all[peerPubkey] ?: emptyList()
    }
    
    /**
     * Get a specific endpoint for a peer and method
     */
    fun get(peerPubkey: String, methodId: String): PrivateEndpointOffer? {
        val endpoints = listForPeer(peerPubkey)
        return endpoints.firstOrNull { it.methodId == methodId }
    }
    
    /**
     * Save a private endpoint
     */
    fun save(endpoint: PrivateEndpointOffer, peerPubkey: String) {
        val all = loadAllEndpoints().toMutableMap()
        
        // Get or create list for this peer
        val peerEndpoints = all[peerPubkey]?.toMutableList() ?: mutableListOf()
        
        // Remove existing endpoint for this method if it exists
        peerEndpoints.removeAll { it.methodId == endpoint.methodId }
        
        // Add the new endpoint
        peerEndpoints.add(endpoint)
        
        // Update the map
        all[peerPubkey] = peerEndpoints
        
        persistAllEndpoints(all)
    }
    
    /**
     * Remove a specific endpoint
     */
    fun remove(peerPubkey: String, methodId: String) {
        val all = loadAllEndpoints().toMutableMap()
        
        val peerEndpoints = all[peerPubkey]?.toMutableList() ?: return
        
        peerEndpoints.removeAll { it.methodId == methodId }
        
        if (peerEndpoints.isEmpty) {
            all.remove(peerPubkey)
        } else {
            all[peerPubkey] = peerEndpoints
        }
        
        persistAllEndpoints(all)
    }
    
    /**
     * Remove all endpoints for a peer
     */
    fun removeAllForPeer(peerPubkey: String) {
        val all = loadAllEndpoints().toMutableMap()
        all.remove(peerPubkey)
        persistAllEndpoints(all)
    }
    
    /**
     * List all peers that have private endpoints
     */
    fun listPeers(): List<String> {
        val all = loadAllEndpoints()
        return all.keys.toList()
    }
    
    /**
     * Clean up expired endpoints
     * Note: Expiration checking would need to be added to PrivateEndpointOffer
     * For now, this is a placeholder
     */
    fun cleanupExpired(): Int {
        // TODO: Implement expiration checking when PrivateEndpointOffer includes expiresAt
        return 0
    }
    
    /**
     * Get count of all stored endpoints
     */
    fun count(): Int {
        val all = loadAllEndpoints()
        return all.values.sumOf { it.size }
    }
    
    /**
     * Clear all endpoints
     */
    fun clearAll() {
        persistAllEndpoints(emptyMap())
    }
    
    // MARK: - Private Helpers
    
    private fun loadAllEndpoints(): Map<String, List<PrivateEndpointOffer>> {
        endpointsCache?.let { return it }
        
        return try {
            val jsonString = prefs.getString(ENDPOINTS_KEY, null) ?: return emptyMap()
            val stored = json.decodeFromString<StoredEndpoints>(jsonString)
            
            // Convert stored format to PrivateEndpointOffer
            val result = stored.endpoints.mapValues { (_, storedList) ->
                storedList.map { stored ->
                    PrivateEndpointOffer(
                        methodId = stored.methodId,
                        endpoint = stored.endpoint
                    )
                }
            }
            
            endpointsCache = result
            result
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load endpoints: ${e.message}")
            emptyMap()
        }
    }
    
    private fun persistAllEndpoints(endpoints: Map<String, List<PrivateEndpointOffer>>) {
        try {
            // Convert PrivateEndpointOffer to storable format
            val stored = endpoints.mapValues { (_, offers) ->
                offers.map { offer ->
                    StoredEndpoint(
                        methodId = offer.methodId,
                        endpoint = offer.endpoint
                    )
                }
            }
            
            val storedEndpoints = StoredEndpoints(endpoints = stored)
            val jsonString = json.encodeToString(storedEndpoints)
            
            prefs.edit().putString(ENDPOINTS_KEY, jsonString).apply()
            endpointsCache = endpoints
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save endpoints: ${e.message}")
        }
    }
}

// MARK: - Storage Models

@Serializable
private data class StoredEndpoints(
    val endpoints: Map<String, List<StoredEndpoint>>
)

@Serializable
private data class StoredEndpoint(
    val methodId: String,
    val endpoint: String
)

