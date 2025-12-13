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
 * Identity information for display and management
 */
data class IdentityInfo(
    val name: String,
    val publicKeyZ32: String,
    val publicKeyHex: String,
    val nickname: String?,
    val createdAt: Long
)

/**
 * Manages cryptographic keys for Paykit
 *
 * This class handles:
 * - Generating Ed25519 identity keys via Rust FFI
 * - Deriving X25519 device keys for Noise protocol
 * - Secure storage in Android EncryptedSharedPreferences
 * - Key backup/restore with password encryption
 * - Multiple identity management
 */
class KeyManager(context: Context) {
    
    companion object {
        private const val PREFS_NAME = "paykit_keys"
        
        // Legacy single identity keys (for migration)
        private const val KEY_SECRET = "paykit.identity.secret"
        private const val KEY_PUBLIC = "paykit.identity.public"
        private const val KEY_PUBLIC_Z32 = "paykit.identity.public.z32"
        
        // Multiple identity keys (new format)
        private fun secretKey(name: String) = "paykit.identity.$name.secret"
        private fun publicKey(name: String) = "paykit.identity.$name.public"
        private fun publicKeyZ32(name: String) = "paykit.identity.$name.public.z32"
        private fun nickname(name: String) = "paykit.identity.$name.nickname"
        private fun createdAt(name: String) = "paykit.identity.$name.created_at"
        
        // Current identity and list (stored in regular SharedPreferences)
        private const val KEY_CURRENT_IDENTITY = "paykit.current_identity"
        private const val KEY_IDENTITY_LIST = "paykit.identity_list"
        
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
    
    private val _currentIdentityName = MutableStateFlow<String?>(null)
    val currentIdentityName: StateFlow<String?> = _currentIdentityName.asStateFlow()
    
    // Encrypted storage for keys
    private val prefs: SharedPreferences
    
    // Regular SharedPreferences for metadata
    private val regularPrefs: SharedPreferences
    
    private val deviceId: String
    
    init {
        // Create master key for encryption
        val masterKey = MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()
        
        // Create encrypted shared preferences for keys
        prefs = EncryptedSharedPreferences.create(
            context,
            PREFS_NAME,
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )
        
        // Regular SharedPreferences for metadata (current identity, list)
        regularPrefs = context.getSharedPreferences("paykit_metadata", Context.MODE_PRIVATE)
        
        // Get or generate device ID
        deviceId = prefs.getString(KEY_DEVICE_ID, null) ?: run {
            val newId = generateDeviceId()
            prefs.edit().putString(KEY_DEVICE_ID, newId).apply()
            newId
        }
        
        // Migrate single identity to multiple if needed
        migrateSingleIdentity()
        
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
        val currentName = getCurrentIdentityName()
        
        return if (currentName != null) {
            // Load existing identity
            getIdentity(currentName)
        } else {
            // No current identity, create default
            createIdentity("default", null)
        }
    }
    
    /**
     * Get identity by name
     *
     * @param name Identity name
     * @return The Ed25519 keypair
     */
    fun getIdentity(name: String): Ed25519Keypair {
        val secretKey = secretKey(name)
        val secretHex = prefs.getString(secretKey, null)
            ?: throw KeyManagerException.IdentityNotFound(name)
        return ed25519KeypairFromSecret(secretHex)
    }
    
    /**
     * Get identity info without loading secret key
     *
     * @param name Identity name
     * @return IdentityInfo or null if not found
     */
    fun getIdentityInfo(name: String): IdentityInfo? {
        val publicKeyZ32 = prefs.getString(publicKeyZ32(name), null) ?: return null
        val publicKeyHex = prefs.getString(publicKey(name), null) ?: return null
        val nickname = prefs.getString(nickname(name), null)
        val createdAt = prefs.getLong(createdAt(name), 0L)
        
        return IdentityInfo(
            name = name,
            publicKeyZ32 = publicKeyZ32,
            publicKeyHex = publicKeyHex,
            nickname = nickname,
            createdAt = createdAt
        )
    }
    
    /**
     * List all identities
     *
     * @return List of IdentityInfo
     */
    fun listIdentities(): List<IdentityInfo> {
        val identityList = regularPrefs.getString(KEY_IDENTITY_LIST, null)
            ?.split(",")
            ?.filter { it.isNotEmpty() }
            ?: return emptyList()
        
        return identityList.mapNotNull { name ->
            getIdentityInfo(name)
        }
    }
    
    /**
     * Create a new identity
     *
     * @param name Unique name for the identity
     * @param nickname Optional nickname
     * @return The new Ed25519 keypair
     */
    fun createIdentity(name: String, nickname: String?): Ed25519Keypair {
        // Validate name
        if (name.isEmpty()) {
            throw KeyManagerException.InvalidIdentityName("Name cannot be empty")
        }
        
        // Check for duplicates
        if (getIdentityInfo(name) != null) {
            throw KeyManagerException.DuplicateIdentity(name)
        }
        
        // Generate keypair
        val keypair = generateEd25519Keypair()
        
        // Store in encrypted preferences with name prefix
        prefs.edit()
            .putString(secretKey(name), keypair.secretKeyHex)
            .putString(publicKey(name), keypair.publicKeyHex)
            .putString(publicKeyZ32(name), keypair.publicKeyZ32)
            .apply()
        
        if (nickname != null) {
            prefs.edit().putString(this.nickname(name), nickname).apply()
        }
        
        val createdAt = System.currentTimeMillis()
        prefs.edit().putLong(createdAt(name), createdAt).apply()
        
        // Add to identity list
        val identityList = regularPrefs.getString(KEY_IDENTITY_LIST, null)
            ?.split(",")
            ?.filter { it.isNotEmpty() }
            ?.toMutableList()
            ?: mutableListOf()
        
        if (!identityList.contains(name)) {
            identityList.add(name)
            regularPrefs.edit()
                .putString(KEY_IDENTITY_LIST, identityList.joinToString(","))
                .apply()
        }
        
        // If no current identity, set this as current
        if (getCurrentIdentityName() == null) {
            switchIdentity(name)
        }
        
        return keypair
    }
    
    /**
     * Switch to a different identity
     *
     * @param name Identity name to switch to
     */
    fun switchIdentity(name: String) {
        // Validate identity exists
        if (getIdentityInfo(name) == null) {
            throw KeyManagerException.IdentityNotFound(name)
        }
        
        // Update current identity
        regularPrefs.edit()
            .putString(KEY_CURRENT_IDENTITY, name)
            .apply()
        
        _currentIdentityName.value = name
        
        // Reload identity state
        loadIdentityState()
    }
    
    /**
     * Delete an identity
     *
     * @param name Identity name to delete
     */
    fun deleteIdentity(name: String) {
        // Prevent deleting current identity (must switch first)
        if (name == getCurrentIdentityName()) {
            throw KeyManagerException.CannotDeleteCurrentIdentity()
        }
        
        // Delete all encrypted entries for this identity
        prefs.edit()
            .remove(secretKey(name))
            .remove(publicKey(name))
            .remove(publicKeyZ32(name))
            .remove(nickname(name))
            .remove(createdAt(name))
            .apply()
        
        // Remove from identity list
        val identityList = regularPrefs.getString(KEY_IDENTITY_LIST, null)
            ?.split(",")
            ?.filter { it.isNotEmpty() && it != name }
            ?: emptyList()
        
        regularPrefs.edit()
            .putString(KEY_IDENTITY_LIST, identityList.joinToString(","))
            .apply()
        
        // If was current, set first available as current
        if (identityList.isNotEmpty()) {
            switchIdentity(identityList.first())
        } else {
            regularPrefs.edit().remove(KEY_CURRENT_IDENTITY).apply()
            _currentIdentityName.value = null
            loadIdentityState()
        }
    }
    
    /**
     * Get current identity name
     *
     * @return Current identity name or null
     */
    fun getCurrentIdentityName(): String? {
        return regularPrefs.getString(KEY_CURRENT_IDENTITY, null)
    }
    
    /**
     * Generate a new identity (replaces existing if any) - Legacy method for backward compatibility
     *
     * @return The new Ed25519 keypair
     */
    fun generateNewIdentity(): Ed25519Keypair {
        // Use current identity name or "default"
        val name = getCurrentIdentityName() ?: "default"
        
        // Delete existing if any
        if (getIdentityInfo(name) != null) {
            try {
                deleteIdentity(name)
            } catch (e: KeyManagerException.CannotDeleteCurrentIdentity) {
                // If it's current, switch first
                val allIdentities = listIdentities()
                val otherIdentity = allIdentities.firstOrNull { it.name != name }
                if (otherIdentity != null) {
                    switchIdentity(otherIdentity.name)
                    deleteIdentity(name)
                } else {
                    // No other identity, just overwrite
                }
            }
        }
        
        return createIdentity(name, null)
    }
    
    /**
     * Get the current public key in z-base32 format (pkarr format)
     */
    fun getCurrentPublicKeyZ32(): String? {
        val name = getCurrentIdentityName() ?: return null
        return prefs.getString(publicKeyZ32(name), null)
    }
    
    /**
     * Get the current public key in hex format
     */
    fun getCurrentPublicKeyHex(): String? {
        val name = getCurrentIdentityName() ?: return null
        return prefs.getString(publicKey(name), null)
    }
    
    /**
     * Delete the current identity - Legacy method for backward compatibility
     *
     * Warning: This cannot be undone unless you have a backup!
     */
    fun deleteIdentity() {
        val name = getCurrentIdentityName() ?: throw KeyManagerException.NoIdentity()
        deleteIdentity(name)
    }
    
    /**
     * Migrate existing single identity to named system
     */
    private fun migrateSingleIdentity() {
        // Check for old single identity keys
        val oldSecret = prefs.getString(KEY_SECRET, null) ?: return
        
        // Load old identity
        val oldPublic = prefs.getString(KEY_PUBLIC, null) ?: return
        val oldPublicZ32 = prefs.getString(KEY_PUBLIC_Z32, null) ?: return
        
        // Create "default" identity with old keys
        prefs.edit()
            .putString(secretKey("default"), oldSecret)
            .putString(publicKey("default"), oldPublic)
            .putString(publicKeyZ32("default"), oldPublicZ32)
            .putLong(createdAt("default"), System.currentTimeMillis())
            .apply()
        
        // Delete old keys
        prefs.edit()
            .remove(KEY_SECRET)
            .remove(KEY_PUBLIC)
            .remove(KEY_PUBLIC_Z32)
            .apply()
        
        // Set "default" as current
        regularPrefs.edit()
            .putString(KEY_CURRENT_IDENTITY, "default")
            .putString(KEY_IDENTITY_LIST, "default")
            .apply()
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
        val name = getCurrentIdentityName() ?: throw KeyManagerException.NoIdentity()
        val secretKey = secretKey(name)
        val secretHex = prefs.getString(secretKey, null)
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
        val name = getCurrentIdentityName() ?: throw KeyManagerException.NoIdentity()
        val secretKey = secretKey(name)
        val secretHex = prefs.getString(secretKey, null)
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
    
    /**
     * Get secret key as raw bytes (for Noise protocol handshake)
     *
     * @return 32-byte secret key, or null if no identity
     */
    fun getSecretKeyBytes(): ByteArray? {
        val name = getCurrentIdentityName() ?: return null
        val secretKey = secretKey(name)
        val secretHex = prefs.getString(secretKey, null) ?: return null
        
        // Convert hex to bytes
        return try {
            secretHex.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
                .takeIf { it.size == 32 }
        } catch (e: Exception) {
            null
        }
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
        val name = getCurrentIdentityName() ?: throw KeyManagerException.NoIdentity()
        val secretKey = secretKey(name)
        val secretHex = prefs.getString(secretKey, null)
            ?: throw KeyManagerException.NoIdentity()
        
        return exportKeypairToBackup(secretHex, password)
    }
    
    /**
     * Import identity from encrypted backup
     *
     * @param backup The encrypted backup
     * @param password Password to decrypt
     * @param name Name for the imported identity (defaults to backup public key prefix)
     * @return The restored keypair
     */
    fun importBackup(backup: KeyBackup, password: String, name: String? = null): Ed25519Keypair {
        val keypair = importKeypairFromBackup(backup, password)
        
        // Use provided name or generate from public key
        val identityName = name ?: backup.publicKeyZ32.take(8)
        
        // Store in encrypted preferences with name prefix
        prefs.edit()
            .putString(secretKey(identityName), keypair.secretKeyHex)
            .putString(publicKey(identityName), keypair.publicKeyHex)
            .putString(publicKeyZ32(identityName), keypair.publicKeyZ32)
            .putLong(createdAt(identityName), System.currentTimeMillis())
            .apply()
        
        // Add to identity list
        val identityList = regularPrefs.getString(KEY_IDENTITY_LIST, null)
            ?.split(",")
            ?.filter { it.isNotEmpty() }
            ?.toMutableList()
            ?: mutableListOf()
        
        if (!identityList.contains(identityName)) {
            identityList.add(identityName)
            regularPrefs.edit()
                .putString(KEY_IDENTITY_LIST, identityList.joinToString(","))
                .apply()
        }
        
        // If no current identity, set this as current
        if (getCurrentIdentityName() == null) {
            switchIdentity(identityName)
        } else {
            loadIdentityState()
        }
        
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
        val name = getCurrentIdentityName()
        _currentIdentityName.value = name
        
        if (name != null) {
            val pubZ32 = prefs.getString(publicKeyZ32(name), null)
            val pubHex = prefs.getString(publicKey(name), null)
            
            if (pubZ32 != null && pubHex != null) {
                _hasIdentity.value = true
                _publicKeyZ32.value = pubZ32
                _publicKeyHex.value = pubHex
            } else {
                _hasIdentity.value = false
                _publicKeyZ32.value = ""
                _publicKeyHex.value = ""
            }
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
    class IdentityNotFound(val name: String) : KeyManagerException("Identity '$name' not found.")
    class DuplicateIdentity(val name: String) : KeyManagerException("Identity '$name' already exists.")
    class InvalidIdentityName(val message: String) : KeyManagerException("Invalid identity name: $message")
    class CannotDeleteCurrentIdentity : KeyManagerException("Cannot delete current identity. Switch to another identity first.")
}

