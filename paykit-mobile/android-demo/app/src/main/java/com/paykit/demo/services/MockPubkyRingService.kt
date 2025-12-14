// MockPubkyRingService.kt
// Mock Pubky Ring Service for Demo/Testing
//
// This service simulates Pubky Ring for demo and testing purposes.
// In production, Paykit apps would request key derivation from the real
// Pubky Ring app via intents.
//
// Key Architecture:
//   - Ed25519 (pkarr) keys are "cold" and managed by Pubky Ring
//   - X25519 (Noise) keys are "hot" and derived on-demand
//   - This mock service provides Ed25519 seed for demo compatibility
//
// For production, use PubkyRingIntegration instead.

package com.paykit.demo.services

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.paykit.mobile.deriveX25519Keypair
import com.paykit.mobile.ed25519KeypairFromSecret
import com.paykit.mobile.generateEd25519Keypair

/**
 * X25519 keypair for Noise protocol encryption
 */
data class X25519KeypairResult(
    val secretKeyHex: String,
    val publicKeyHex: String,
    val deviceId: String,
    val epoch: UInt
)

/**
 * Exception types for mock Pubky Ring service
 */
sealed class MockPubkyRingException(message: String) : Exception(message) {
    object NoSeedAvailable : MockPubkyRingException("No Ed25519 seed available. Initialize the mock service first.")
    class DerivationFailed(message: String) : MockPubkyRingException("Key derivation failed: $message")
    object InvalidSeedFormat : MockPubkyRingException("Invalid seed format. Seed must be 32 bytes hex-encoded.")
    class StorageError(message: String) : MockPubkyRingException("Storage error: $message")
}

/**
 * Mock Pubky Ring service for demo and testing
 *
 * This service simulates the key derivation functionality that would
 * normally be provided by the Pubky Ring app. It stores an Ed25519
 * seed in encrypted storage and derives X25519 keys on-demand.
 *
 * **DEMO ONLY**: In production, use PubkyRingIntegration to communicate
 * with the real Pubky Ring app.
 */
class MockPubkyRingService private constructor(context: Context) {
    
    companion object {
        private const val PREFS_NAME = "mock_pubkyring_keys"
        private const val KEY_MOCK_SEED = "mock.pubkyring.ed25519.seed"
        private const val KEY_MOCK_PUBLIC = "mock.pubkyring.ed25519.public"
        private const val KEY_MOCK_PUBLIC_Z32 = "mock.pubkyring.ed25519.public.z32"
        
        @Volatile
        private var instance: MockPubkyRingService? = null
        
        fun getInstance(context: Context): MockPubkyRingService {
            return instance ?: synchronized(this) {
                instance ?: MockPubkyRingService(context.applicationContext).also { instance = it }
            }
        }
    }
    
    private val prefs: SharedPreferences
    private var cachedSeedHex: String? = null
    
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
        
        loadCachedSeed()
    }
    
    /**
     * Check if the mock service has a seed available
     */
    val hasSeed: Boolean
        get() = cachedSeedHex != null
    
    /**
     * Initialize the mock service with a new random seed
     *
     * This generates a new Ed25519 keypair and stores the seed securely.
     * Call this once during demo setup.
     */
    fun initializeWithNewSeed() {
        // Generate new Ed25519 keypair using paykit-mobile FFI
        val keypair = generateEd25519Keypair()
        
        // Store seed securely
        prefs.edit()
            .putString(KEY_MOCK_SEED, keypair.secretKeyHex)
            .putString(KEY_MOCK_PUBLIC, keypair.publicKeyHex)
            .putString(KEY_MOCK_PUBLIC_Z32, keypair.publicKeyZ32)
            .apply()
        
        cachedSeedHex = keypair.secretKeyHex
    }
    
    /**
     * Initialize the mock service with an existing seed
     *
     * @param seedHex 32-byte Ed25519 seed as hex string (64 characters)
     */
    fun initializeWithSeed(seedHex: String) {
        require(seedHex.length == 64) { throw MockPubkyRingException.InvalidSeedFormat }
        
        // Derive public key from seed
        val keypair = ed25519KeypairFromSecret(seedHex)
        
        // Store seed securely
        prefs.edit()
            .putString(KEY_MOCK_SEED, seedHex)
            .putString(KEY_MOCK_PUBLIC, keypair.publicKeyHex)
            .putString(KEY_MOCK_PUBLIC_Z32, keypair.publicKeyZ32)
            .apply()
        
        cachedSeedHex = seedHex
    }
    
    /**
     * Get the Ed25519 public key (for identity display)
     *
     * @return Public key in z-base32 format (pkarr format)
     */
    fun getEd25519PublicKeyZ32(): String {
        return prefs.getString(KEY_MOCK_PUBLIC_Z32, null)
            ?: throw MockPubkyRingException.NoSeedAvailable
    }
    
    /**
     * Get the Ed25519 public key in hex format
     *
     * @return Public key as hex string
     */
    fun getEd25519PublicKeyHex(): String {
        return prefs.getString(KEY_MOCK_PUBLIC, null)
            ?: throw MockPubkyRingException.NoSeedAvailable
    }
    
    /**
     * Derive X25519 keypair for Noise protocol
     *
     * This uses the pubky-noise KDF to derive device-specific encryption keys.
     *
     * @param deviceId Unique identifier for this device
     * @param epoch Key rotation epoch (increment to rotate keys)
     * @return Derived X25519 keypair
     */
    fun deriveX25519Keypair(deviceId: String, epoch: UInt): X25519KeypairResult {
        val seedHex = cachedSeedHex ?: throw MockPubkyRingException.NoSeedAvailable
        
        // Use paykit-mobile FFI for key derivation
        // This uses the same HKDF as pubky-noise
        return try {
            val keypair = deriveX25519Keypair(seedHex, deviceId, epoch.toUInt())
            X25519KeypairResult(
                secretKeyHex = keypair.secretKeyHex,
                publicKeyHex = keypair.publicKeyHex,
                deviceId = deviceId,
                epoch = epoch
            )
        } catch (e: Exception) {
            throw MockPubkyRingException.DerivationFailed(e.message ?: "Unknown error")
        }
    }
    
    /**
     * Get the raw Ed25519 seed (32 bytes)
     *
     * **WARNING**: This exposes the cold key, which defeats the purpose of
     * the cold/hot key architecture. Only use this for demo purposes with
     * FfiNoiseManager which requires the seed.
     *
     * @return Ed25519 seed as raw bytes
     */
    fun getEd25519SeedBytes(): ByteArray {
        val seedHex = cachedSeedHex ?: throw MockPubkyRingException.NoSeedAvailable
        return seedHex.hexToByteArray()
    }
    
    /**
     * Sign a message with Ed25519 (for DHT records, not payments)
     *
     * @param message Message to sign
     * @return 64-byte signature as hex string
     */
    fun signMessage(message: ByteArray): String {
        val seedHex = cachedSeedHex ?: throw MockPubkyRingException.NoSeedAvailable
        return com.paykit.mobile.signMessage(seedHex, message.toList())
    }
    
    /**
     * Clear the mock seed (for testing)
     */
    fun clearSeed() {
        prefs.edit()
            .remove(KEY_MOCK_SEED)
            .remove(KEY_MOCK_PUBLIC)
            .remove(KEY_MOCK_PUBLIC_Z32)
            .apply()
        cachedSeedHex = null
    }
    
    private fun loadCachedSeed() {
        cachedSeedHex = prefs.getString(KEY_MOCK_SEED, null)
    }
}

// Extension to convert hex string to ByteArray
private fun String.hexToByteArray(): ByteArray {
    require(length % 2 == 0) { "Hex string must have even length" }
    return chunked(2)
        .map { it.toInt(16).toByte() }
        .toByteArray()
}

// Extension to convert ByteArray to hex string
private fun ByteArray.toHexString(): String {
    return joinToString("") { "%02x".format(it) }
}

