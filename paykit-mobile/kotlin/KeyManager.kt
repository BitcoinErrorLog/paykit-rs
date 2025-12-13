// KeyManager.kt
// Paykit Android Key Management
//
// This file provides secure key management using the Rust FFI functions
// and Android EncryptedSharedPreferences for persistent storage.
//
// Key Architecture:
//   - Ed25519 identity key (pkarr): The user's main identity
//   - X25519 device keys (pubky-noise): Derived for encryption
//
// USAGE:
//   val keyManager = KeyManager(context)
//   val keypair = keyManager.getOrCreateIdentity()
//   Log.d("Paykit", "Your public key: ${keypair.publicKeyZ32}")

package com.paykit.mobile

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.json.JSONObject

/**
 * Manages cryptographic keys for Paykit
 *
 * This class handles:
 * - Generating Ed25519 identity keys via Rust FFI
 * - Deriving X25519 device keys for Noise protocol
 * - Secure storage in Android EncryptedSharedPreferences
 * - Key backup/restore with password encryption
 */
class KeyManager(context: Context) {
    
    companion object {
        private const val PREFS_NAME = "paykit_keys"
        private const val KEY_SECRET = "paykit.identity.secret"
        private const val KEY_PUBLIC = "paykit.identity.public"
        private const val KEY_PUBLIC_Z32 = "paykit.identity.public.z32"
        private const val KEY_DEVICE_ID = "paykit.device.id"
        private const val KEY_EPOCH = "paykit.device.epoch"
    }
    
    // State
    private val _hasIdentity = MutableStateFlow(false)
    val hasIdentity: StateFlow<Boolean> = _hasIdentity.asStateFlow()
    
    private val _publicKeyZ32 = MutableStateFlow("")
    val publicKeyZ32: StateFlow<String> = _publicKeyZ32.asStateFlow()
    
    private val _publicKeyHex = MutableStateFlow("")
    val publicKeyHex: StateFlow<String> = _publicKeyHex.asStateFlow()
    
    // Encrypted storage
    private val prefs: SharedPreferences
    private val deviceId: String
    
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
        
        // Get or generate device ID
        deviceId = prefs.getString(KEY_DEVICE_ID, null) ?: run {
            val newId = generateDeviceId()
            prefs.edit().putString(KEY_DEVICE_ID, newId).apply()
            newId
        }
        
        // Load identity state
        loadIdentityState()
    }
    
    // ============================================================================
    // Identity Management
    // ============================================================================
    
    /**
     * Get the current identity or create a new one
     *
     * @return The Ed25519 keypair
     */
    fun getOrCreateIdentity(): Ed25519Keypair {
        val existingSecret = prefs.getString(KEY_SECRET, null)
        return if (existingSecret != null) {
            // Restore from storage
            ed25519KeypairFromSecret(existingSecret)
        } else {
            // Generate new identity
            generateNewIdentity()
        }
    }
    
    /**
     * Generate a new identity (replaces existing if any)
     *
     * @return The new Ed25519 keypair
     */
    fun generateNewIdentity(): Ed25519Keypair {
        val keypair = generateEd25519Keypair()
        
        // Store in encrypted preferences
        prefs.edit()
            .putString(KEY_SECRET, keypair.secretKeyHex)
            .putString(KEY_PUBLIC, keypair.publicKeyHex)
            .putString(KEY_PUBLIC_Z32, keypair.publicKeyZ32)
            .apply()
        
        // Update state
        _hasIdentity.value = true
        _publicKeyZ32.value = keypair.publicKeyZ32
        _publicKeyHex.value = keypair.publicKeyHex
        
        return keypair
    }
    
    /**
     * Get the current public key in z-base32 format (pkarr format)
     */
    fun getCurrentPublicKeyZ32(): String? = prefs.getString(KEY_PUBLIC_Z32, null)
    
    /**
     * Get the current public key in hex format
     */
    fun getCurrentPublicKeyHex(): String? = prefs.getString(KEY_PUBLIC, null)
    
    /**
     * Delete the current identity
     *
     * Warning: This cannot be undone unless you have a backup!
     */
    fun deleteIdentity() {
        prefs.edit()
            .remove(KEY_SECRET)
            .remove(KEY_PUBLIC)
            .remove(KEY_PUBLIC_Z32)
            .apply()
        
        _hasIdentity.value = false
        _publicKeyZ32.value = ""
        _publicKeyHex.value = ""
    }
    
    // ============================================================================
    // X25519 Device Keys
    // ============================================================================
    
    /**
     * Get or derive X25519 device key for Noise protocol
     *
     * @param epoch Key rotation epoch (default 0)
     * @return The X25519 keypair for this device and epoch
     */
    fun getDeviceX25519Key(epoch: UInt = 0u): X25519Keypair {
        val secretHex = prefs.getString(KEY_SECRET, null)
            ?: throw KeyManagerException.NoIdentity()
        
        return deriveX25519Keypair(
            ed25519SecretHex = secretHex,
            deviceId = deviceId,
            epoch = epoch
        )
    }
    
    /**
     * Get the current device ID
     */
    fun getDeviceId(): String = deviceId
    
    /**
     * Get the current key epoch
     */
    fun getCurrentEpoch(): UInt = prefs.getInt(KEY_EPOCH, 0).toUInt()
    
    /**
     * Increment the key epoch (for key rotation)
     */
    fun incrementEpoch(): UInt {
        val newEpoch = getCurrentEpoch() + 1u
        prefs.edit().putInt(KEY_EPOCH, newEpoch.toInt()).apply()
        return newEpoch
    }
    
    // ============================================================================
    // Signing
    // ============================================================================
    
    /**
     * Sign data with the identity key
     *
     * @param data Data to sign
     * @return Hex-encoded signature
     */
    fun sign(data: ByteArray): String {
        val secretHex = prefs.getString(KEY_SECRET, null)
            ?: throw KeyManagerException.NoIdentity()
        
        return signMessage(secretHex, data)
    }
    
    /**
     * Verify a signature
     *
     * @param publicKeyHex Signer's public key in hex
     * @param data Original data
     * @param signatureHex Signature to verify
     * @return true if valid
     */
    fun verify(publicKeyHex: String, data: ByteArray, signatureHex: String): Boolean {
        return verifySignature(publicKeyHex, data, signatureHex)
    }
    
    // ============================================================================
    // Backup & Restore
    // ============================================================================
    
    /**
     * Export identity to encrypted backup
     *
     * @param password Password to encrypt the backup
     * @return The encrypted backup
     */
    fun exportBackup(password: String): KeyBackup {
        val secretHex = prefs.getString(KEY_SECRET, null)
            ?: throw KeyManagerException.NoIdentity()
        
        return exportKeypairToBackup(secretHex, password)
    }
    
    /**
     * Import identity from encrypted backup
     *
     * @param backup The encrypted backup
     * @param password Password to decrypt
     * @return The restored keypair
     */
    fun importBackup(backup: KeyBackup, password: String): Ed25519Keypair {
        val keypair = importKeypairFromBackup(backup, password)
        
        // Store in encrypted preferences
        prefs.edit()
            .putString(KEY_SECRET, keypair.secretKeyHex)
            .putString(KEY_PUBLIC, keypair.publicKeyHex)
            .putString(KEY_PUBLIC_Z32, keypair.publicKeyZ32)
            .apply()
        
        // Update state
        _hasIdentity.value = true
        _publicKeyZ32.value = keypair.publicKeyZ32
        _publicKeyHex.value = keypair.publicKeyHex
        
        return keypair
    }
    
    /**
     * Convert backup to shareable string (JSON)
     */
    fun backupToString(backup: KeyBackup): String {
        return JSONObject().apply {
            put("version", backup.version)
            put("encrypted_data", backup.encryptedDataHex)
            put("salt", backup.saltHex)
            put("nonce", backup.nonceHex)
            put("public_key", backup.publicKeyZ32)
        }.toString(2)
    }
    
    /**
     * Parse backup from string (JSON)
     */
    fun backupFromString(string: String): KeyBackup {
        val json = JSONObject(string)
        return KeyBackup(
            version = json.getInt("version").toUInt(),
            encryptedDataHex = json.getString("encrypted_data"),
            saltHex = json.getString("salt"),
            nonceHex = json.getString("nonce"),
            publicKeyZ32 = json.getString("public_key")
        )
    }
    
    // ============================================================================
    // Private Helpers
    // ============================================================================
    
    private fun loadIdentityState() {
        val pubZ32 = prefs.getString(KEY_PUBLIC_Z32, null)
        val pubHex = prefs.getString(KEY_PUBLIC, null)
        
        if (pubZ32 != null && pubHex != null) {
            _hasIdentity.value = true
            _publicKeyZ32.value = pubZ32
            _publicKeyHex.value = pubHex
        } else {
            _hasIdentity.value = false
            _publicKeyZ32.value = ""
            _publicKeyHex.value = ""
        }
    }
}

// ============================================================================
// Exceptions
// ============================================================================

sealed class KeyManagerException(message: String) : Exception(message) {
    class NoIdentity : KeyManagerException("No identity key exists. Generate or import one first.")
    class InvalidBackup : KeyManagerException("Invalid backup format.")
    class BackupDecryptionFailed : KeyManagerException("Failed to decrypt backup. Check your password.")
}

