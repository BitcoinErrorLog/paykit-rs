// NoiseKeyCache.kt
// X25519 Key Cache for Noise Protocol
//
// This file provides caching for derived X25519 keys used in Noise protocol
// communications. Keys are cached both in-memory and persistently in
// EncryptedSharedPreferences to reduce round trips to Pubky Ring.
//
// Key Architecture:
//   - Ed25519 (pkarr) keys are "cold" and managed by Pubky Ring
//   - X25519 (Noise) keys are "hot" and cached locally in Paykit
//   - This cache stores the hot X25519 keys for quick access

package com.paykit.demo.services

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.ConcurrentHashMap

/**
 * Cache for X25519 Noise protocol keys
 *
 * This class provides:
 * - In-memory caching for fast access
 * - Persistent storage in EncryptedSharedPreferences for app restarts
 * - Key rotation support via epoch parameter
 * - Secure clearing of keys when needed
 */
class NoiseKeyCache private constructor(context: Context) {
    
    companion object {
        private const val PREFS_NAME = "noise_key_cache"
        private const val KEY_CACHE_INDEX = "noise.key.cache.index"
        
        private fun cacheKey(deviceId: String, epoch: UInt): String {
            return "noise.key.cache.$deviceId.$epoch"
        }
        
        @Volatile
        private var instance: NoiseKeyCache? = null
        
        fun getInstance(context: Context): NoiseKeyCache {
            return instance ?: synchronized(this) {
                instance ?: NoiseKeyCache(context.applicationContext).also { instance = it }
            }
        }
    }
    
    private val prefs: SharedPreferences
    private val memoryCache = ConcurrentHashMap<String, X25519KeypairResult>()
    
    /**
     * Maximum number of keys to keep in cache (per device)
     */
    var maxCachedEpochs: Int = 5
    
    init {
        // Create master key for encryption
        val masterKey = MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()
        
        // Create encrypted shared preferences
        prefs = EncryptedSharedPreferences.create(
            context,
            PREFS_NAME,
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )
        
        loadCacheIndex()
    }
    
    /**
     * Get a cached key if available
     *
     * @param deviceId Device identifier used for derivation
     * @param epoch Key rotation epoch
     * @return Cached keypair or null if not found
     */
    fun getKey(deviceId: String, epoch: UInt): X25519KeypairResult? {
        val key = cacheKey(deviceId, epoch)
        
        // Check memory cache first (fast path)
        memoryCache[key]?.let { return it }
        
        // Check persistent cache
        loadFromPrefs(key)?.let { keypair ->
            // Populate memory cache
            memoryCache[key] = keypair
            return keypair
        }
        
        return null
    }
    
    /**
     * Store a key in the cache
     *
     * @param keypair The keypair to cache
     * @param deviceId Device identifier used for derivation
     * @param epoch Key rotation epoch
     */
    fun setKey(keypair: X25519KeypairResult, deviceId: String, epoch: UInt) {
        val key = cacheKey(deviceId, epoch)
        
        // Store in memory cache
        memoryCache[key] = keypair
        
        // Store in encrypted prefs
        saveToPrefs(keypair, key)
        
        // Update cache index
        updateCacheIndex(add = key)
        
        // Cleanup old epochs if needed
        cleanupOldEpochs(deviceId, epoch)
    }
    
    /**
     * Get or derive a key
     *
     * This method checks the cache first, then derives a new key if needed.
     * Uses PubkyRingIntegration for derivation.
     *
     * @param context Android context
     * @param deviceId Device identifier
     * @param epoch Key rotation epoch
     * @return Keypair (from cache or freshly derived)
     */
    suspend fun getOrDerive(context: Context, deviceId: String, epoch: UInt): X25519KeypairResult {
        // Check cache
        getKey(deviceId, epoch)?.let { return it }
        
        // Derive new key
        val keypair = withContext(Dispatchers.IO) {
            PubkyRingIntegration.getInstance(context).deriveX25519Keypair(deviceId, epoch)
        }
        
        // Cache it
        setKey(keypair, deviceId, epoch)
        
        return keypair
    }
    
    /**
     * Get the latest cached epoch for a device
     *
     * @param deviceId Device identifier
     * @return Latest epoch number or null if no keys cached
     */
    fun getLatestEpoch(deviceId: String): UInt? {
        val prefix = "noise.key.cache.$deviceId."
        var latestEpoch: UInt? = null
        
        for (key in memoryCache.keys) {
            if (key.startsWith(prefix)) {
                val epochStr = key.substringAfterLast('.')
                val epoch = epochStr.toUIntOrNull()
                if (epoch != null && (latestEpoch == null || epoch > latestEpoch)) {
                    latestEpoch = epoch
                }
            }
        }
        
        return latestEpoch
    }
    
    /**
     * Clear a specific key from cache
     *
     * @param deviceId Device identifier
     * @param epoch Key rotation epoch
     */
    fun clearKey(deviceId: String, epoch: UInt) {
        val key = cacheKey(deviceId, epoch)
        
        // Remove from memory
        memoryCache.remove(key)
        
        // Remove from prefs
        prefs.edit().remove(key).apply()
        
        // Update index
        updateCacheIndex(remove = key)
    }
    
    /**
     * Clear all keys for a device
     *
     * @param deviceId Device identifier
     */
    fun clearAllKeys(deviceId: String) {
        val prefix = "noise.key.cache.$deviceId."
        val keysToRemove = memoryCache.keys.filter { it.startsWith(prefix) }
        
        for (key in keysToRemove) {
            memoryCache.remove(key)
            prefs.edit().remove(key).apply()
            updateCacheIndex(remove = key)
        }
    }
    
    /**
     * Clear all cached keys
     */
    fun clearAllKeys() {
        memoryCache.clear()
        prefs.edit().clear().apply()
    }
    
    /**
     * Get cache statistics
     *
     * @return Map with cache statistics
     */
    fun getCacheStats(): Map<String, Any> {
        return mapOf(
            "memoryCount" to memoryCache.size,
            "keys" to memoryCache.keys.toList()
        )
    }
    
    private fun loadCacheIndex() {
        val indexJson = prefs.getString(KEY_CACHE_INDEX, null) ?: return
        
        try {
            val index = JSONArray(indexJson)
            for (i in 0 until index.length()) {
                val key = index.getString(i)
                loadFromPrefs(key)?.let { keypair ->
                    memoryCache[key] = keypair
                }
            }
        } catch (e: Exception) {
            // Ignore corrupted index
        }
    }
    
    private fun loadFromPrefs(key: String): X25519KeypairResult? {
        val json = prefs.getString(key, null) ?: return null
        
        return try {
            val obj = JSONObject(json)
            X25519KeypairResult(
                secretKeyHex = obj.getString("secret_key_hex"),
                publicKeyHex = obj.getString("public_key_hex"),
                deviceId = obj.getString("device_id"),
                epoch = obj.getInt("epoch").toUInt()
            )
        } catch (e: Exception) {
            null
        }
    }
    
    private fun saveToPrefs(keypair: X25519KeypairResult, key: String) {
        val json = JSONObject().apply {
            put("secret_key_hex", keypair.secretKeyHex)
            put("public_key_hex", keypair.publicKeyHex)
            put("device_id", keypair.deviceId)
            put("epoch", keypair.epoch.toInt())
        }
        
        prefs.edit().putString(key, json.toString()).apply()
    }
    
    private fun updateCacheIndex(add: String? = null, remove: String? = null) {
        val index = getCacheIndex().toMutableList()
        
        add?.let {
            if (!index.contains(it)) {
                index.add(it)
            }
        }
        
        remove?.let {
            index.remove(it)
        }
        
        saveCacheIndex(index)
    }
    
    private fun getCacheIndex(): List<String> {
        val indexJson = prefs.getString(KEY_CACHE_INDEX, null) ?: return emptyList()
        
        return try {
            val index = JSONArray(indexJson)
            (0 until index.length()).map { index.getString(it) }
        } catch (e: Exception) {
            emptyList()
        }
    }
    
    private fun saveCacheIndex(index: List<String>) {
        val jsonArray = JSONArray(index)
        prefs.edit().putString(KEY_CACHE_INDEX, jsonArray.toString()).apply()
    }
    
    private fun cleanupOldEpochs(deviceId: String, currentEpoch: UInt) {
        val prefix = "noise.key.cache.$deviceId."
        
        // Get all epochs for this device
        val epochs = mutableListOf<Pair<String, UInt>>()
        
        for (key in memoryCache.keys) {
            if (key.startsWith(prefix)) {
                val epochStr = key.substringAfterLast('.')
                val epoch = epochStr.toUIntOrNull()
                if (epoch != null) {
                    epochs.add(key to epoch)
                }
            }
        }
        
        // Sort by epoch descending
        epochs.sortByDescending { it.second }
        
        // Remove old epochs beyond limit
        while (epochs.size > maxCachedEpochs) {
            val oldest = epochs.removeLast()
            memoryCache.remove(oldest.first)
            prefs.edit().remove(oldest.first).apply()
            updateCacheIndex(remove = oldest.first)
        }
    }
}

