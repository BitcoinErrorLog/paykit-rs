// EncryptedPreferencesStorage.kt
// Paykit Android EncryptedSharedPreferences Adapter
//
// This file provides an implementation of SecureStorage using Android
// EncryptedSharedPreferences from the AndroidX Security library.
//
// USAGE:
//   1. Add this file to your Android project
//   2. Add the AndroidX Security dependency to build.gradle
//   3. Create an EncryptedPreferencesStorage instance
//   4. Pass it to PaykitClient for secure storage operations
//
// Dependencies (add to build.gradle):
//   implementation "androidx.security:security-crypto:1.1.0-alpha06"
//
// Example:
//   val storage = EncryptedPreferencesStorage(context, "paykit_storage")
//   val client = PaykitClient(storage = storage.asPaykitStorage())

package com.paykit.storage

import android.content.Context
import android.content.SharedPreferences
import android.util.Base64
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import java.io.IOException
import java.security.GeneralSecurityException

/**
 * EncryptedSharedPreferences-based secure storage for Android.
 *
 * This class wraps Android's EncryptedSharedPreferences to provide secure storage
 * for sensitive data like private keys and authentication tokens.
 *
 * Security features:
 * - AES-256-GCM encryption for values
 * - AES-256-SIV encryption for keys
 * - Hardware-backed keystore (when available)
 * - Automatic key rotation support
 *
 * @param context Android context (application context recommended)
 * @param fileName Name for the encrypted preferences file
 * @param masterKeyAlias Alias for the master key in Android Keystore
 */
class EncryptedPreferencesStorage private constructor(
    private val preferences: SharedPreferences
) {

    /**
     * Error types for storage operations.
     */
    sealed class StorageException(message: String, cause: Throwable? = null) : Exception(message, cause) {
        class InitializationError(message: String, cause: Throwable? = null) : 
            StorageException("Initialization error: $message", cause)
        class EncryptionError(message: String, cause: Throwable? = null) : 
            StorageException("Encryption error: $message", cause)
        class AccessDenied(message: String) : 
            StorageException("Access denied: $message")
        class NotFound(key: String) : 
            StorageException("Key not found: $key")
        class InvalidData(message: String) : 
            StorageException("Invalid data: $message")
    }

    companion object {
        private const val DEFAULT_FILE_NAME = "paykit_secure_storage"
        private const val DEFAULT_KEY_ALIAS = "paykit_master_key"

        /**
         * Create a new EncryptedPreferencesStorage instance.
         *
         * @param context Android context (application context recommended)
         * @param fileName Name for the encrypted preferences file
         * @param masterKeyAlias Alias for the master key in Android Keystore
         * @return EncryptedPreferencesStorage instance
         * @throws StorageException.InitializationError if initialization fails
         */
        @JvmStatic
        @JvmOverloads
        fun create(
            context: Context,
            fileName: String = DEFAULT_FILE_NAME,
            masterKeyAlias: String = DEFAULT_KEY_ALIAS
        ): EncryptedPreferencesStorage {
            try {
                // Create or retrieve the master key from Android Keystore
                val masterKey = MasterKey.Builder(context, masterKeyAlias)
                    .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                    .build()

                // Create encrypted shared preferences
                val preferences = EncryptedSharedPreferences.create(
                    context,
                    fileName,
                    masterKey,
                    EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                    EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
                )

                return EncryptedPreferencesStorage(preferences)
            } catch (e: GeneralSecurityException) {
                throw StorageException.InitializationError(
                    "Failed to initialize encrypted storage", e
                )
            } catch (e: IOException) {
                throw StorageException.InitializationError(
                    "Failed to create encrypted preferences file", e
                )
            }
        }

        /**
         * Create storage with biometric authentication requirement.
         *
         * Note: This requires additional setup in the calling application
         * to handle biometric prompts.
         *
         * @param context Android context
         * @param fileName Preferences file name
         * @param masterKeyAlias Keystore key alias
         * @return EncryptedPreferencesStorage with biometric protection
         */
        @JvmStatic
        fun createWithBiometrics(
            context: Context,
            fileName: String = DEFAULT_FILE_NAME,
            masterKeyAlias: String = "${DEFAULT_KEY_ALIAS}_biometric"
        ): EncryptedPreferencesStorage {
            try {
                val masterKey = MasterKey.Builder(context, masterKeyAlias)
                    .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                    .setUserAuthenticationRequired(true)
                    .setRequestStrongBoxBacked(true) // Use StrongBox if available
                    .build()

                val preferences = EncryptedSharedPreferences.create(
                    context,
                    fileName,
                    masterKey,
                    EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                    EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
                )

                return EncryptedPreferencesStorage(preferences)
            } catch (e: GeneralSecurityException) {
                throw StorageException.InitializationError(
                    "Failed to initialize biometric storage", e
                )
            } catch (e: IOException) {
                throw StorageException.InitializationError(
                    "Failed to create encrypted preferences file", e
                )
            }
        }
    }

    // MARK: - Public Methods

    /**
     * Store data securely in encrypted preferences.
     *
     * @param key Unique identifier for the data
     * @param data The data to store (will be Base64 encoded)
     * @throws StorageException if storage fails
     */
    @Throws(StorageException::class)
    fun store(key: String, data: ByteArray) {
        try {
            val encoded = Base64.encodeToString(data, Base64.NO_WRAP)
            preferences.edit().putString(key, encoded).apply()
        } catch (e: Exception) {
            throw StorageException.EncryptionError("Failed to store data for key: $key", e)
        }
    }

    /**
     * Store a string securely in encrypted preferences.
     *
     * @param key Unique identifier for the string
     * @param value The string to store
     * @throws StorageException if storage fails
     */
    @Throws(StorageException::class)
    fun store(key: String, value: String) {
        store(key, value.toByteArray(Charsets.UTF_8))
    }

    /**
     * Retrieve data from encrypted preferences.
     *
     * @param key The key to retrieve
     * @return The stored data, or null if not found
     * @throws StorageException on decryption errors
     */
    @Throws(StorageException::class)
    fun retrieve(key: String): ByteArray? {
        return try {
            val encoded = preferences.getString(key, null) ?: return null
            Base64.decode(encoded, Base64.NO_WRAP)
        } catch (e: IllegalArgumentException) {
            throw StorageException.InvalidData("Failed to decode data for key: $key")
        } catch (e: Exception) {
            throw StorageException.EncryptionError("Failed to retrieve data for key: $key", e)
        }
    }

    /**
     * Retrieve a string from encrypted preferences.
     *
     * @param key The key to retrieve
     * @return The stored string, or null if not found
     * @throws StorageException on decryption errors
     */
    @Throws(StorageException::class)
    fun retrieveString(key: String): String? {
        val data = retrieve(key) ?: return null
        return String(data, Charsets.UTF_8)
    }

    /**
     * Delete data from encrypted preferences.
     *
     * @param key The key to delete
     */
    fun delete(key: String) {
        preferences.edit().remove(key).apply()
    }

    /**
     * List all keys stored in encrypted preferences.
     *
     * @return Set of all stored keys
     */
    fun listKeys(): Set<String> {
        return preferences.all.keys.toSet()
    }

    /**
     * Check if a key exists in encrypted preferences.
     *
     * @param key The key to check
     * @return true if the key exists
     */
    fun contains(key: String): Boolean {
        return preferences.contains(key)
    }

    /**
     * Clear all data from encrypted preferences.
     */
    fun clear() {
        preferences.edit().clear().apply()
    }

    /**
     * Get the number of stored items.
     *
     * @return Number of stored key-value pairs
     */
    fun count(): Int {
        return preferences.all.size
    }

    // MARK: - Synchronous Commit Operations

    /**
     * Store data with synchronous commit (blocks until written).
     *
     * Use this for critical data that must be persisted immediately.
     *
     * @param key Unique identifier for the data
     * @param data The data to store
     * @return true if the commit succeeded
     */
    fun storeSync(key: String, data: ByteArray): Boolean {
        return try {
            val encoded = Base64.encodeToString(data, Base64.NO_WRAP)
            preferences.edit().putString(key, encoded).commit()
        } catch (e: Exception) {
            false
        }
    }

    /**
     * Delete with synchronous commit.
     *
     * @param key The key to delete
     * @return true if the commit succeeded
     */
    fun deleteSync(key: String): Boolean {
        return preferences.edit().remove(key).commit()
    }

    /**
     * Clear all with synchronous commit.
     *
     * @return true if the commit succeeded
     */
    fun clearSync(): Boolean {
        return preferences.edit().clear().commit()
    }
}

// MARK: - Paykit Integration

/**
 * Adapter that bridges EncryptedPreferencesStorage to Paykit's expected interface.
 *
 * Usage:
 * ```kotlin
 * val storage = EncryptedPreferencesStorage.create(context)
 * val paykitStorage = storage.asPaykitStorage()
 * val client = PaykitClient(storage = paykitStorage)
 * ```
 */
class PaykitSecureStorageAdapter(
    private val storage: EncryptedPreferencesStorage
) {

    /**
     * Result type for storage operations.
     */
    sealed class StorageResult<out T> {
        data class Success<T>(val value: T) : StorageResult<T>()
        data class Failure(val error: Exception) : StorageResult<Nothing>()

        fun isSuccess(): Boolean = this is Success
        fun isFailure(): Boolean = this is Failure

        fun getOrNull(): T? = when (this) {
            is Success -> value
            is Failure -> null
        }

        fun getOrThrow(): T = when (this) {
            is Success -> value
            is Failure -> throw error
        }

        fun <R> map(transform: (T) -> R): StorageResult<R> = when (this) {
            is Success -> Success(transform(value))
            is Failure -> this
        }
    }

    /**
     * Store data securely.
     *
     * @param key Unique identifier
     * @param value Data to store
     * @return Result indicating success or failure
     */
    fun store(key: String, value: ByteArray): StorageResult<Unit> {
        return try {
            storage.store(key, value)
            StorageResult.Success(Unit)
        } catch (e: Exception) {
            StorageResult.Failure(e)
        }
    }

    /**
     * Retrieve data from secure storage.
     *
     * @param key The key to retrieve
     * @return Result containing the data or null if not found
     */
    fun retrieve(key: String): StorageResult<ByteArray?> {
        return try {
            val data = storage.retrieve(key)
            StorageResult.Success(data)
        } catch (e: Exception) {
            StorageResult.Failure(e)
        }
    }

    /**
     * Delete data from secure storage.
     *
     * @param key The key to delete
     * @return Result indicating success or failure
     */
    fun delete(key: String): StorageResult<Unit> {
        return try {
            storage.delete(key)
            StorageResult.Success(Unit)
        } catch (e: Exception) {
            StorageResult.Failure(e)
        }
    }

    /**
     * List all stored keys.
     *
     * @return Result containing the list of keys
     */
    fun listKeys(): StorageResult<List<String>> {
        return try {
            val keys = storage.listKeys().toList()
            StorageResult.Success(keys)
        } catch (e: Exception) {
            StorageResult.Failure(e)
        }
    }

    /**
     * Check if a key exists.
     *
     * @param key The key to check
     * @return Result indicating if the key exists
     */
    fun contains(key: String): StorageResult<Boolean> {
        return try {
            StorageResult.Success(storage.contains(key))
        } catch (e: Exception) {
            StorageResult.Failure(e)
        }
    }

    /**
     * Clear all stored data.
     *
     * @return Result indicating success or failure
     */
    fun clear(): StorageResult<Unit> {
        return try {
            storage.clear()
            StorageResult.Success(Unit)
        } catch (e: Exception) {
            StorageResult.Failure(e)
        }
    }
}

/**
 * Extension function to create a Paykit-compatible storage adapter.
 */
fun EncryptedPreferencesStorage.asPaykitStorage(): PaykitSecureStorageAdapter {
    return PaykitSecureStorageAdapter(this)
}

// MARK: - Coroutine Extensions

/**
 * Coroutine-friendly extensions for async storage operations.
 *
 * These extensions allow using the storage with Kotlin coroutines,
 * running blocking operations on an appropriate dispatcher.
 */
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

suspend fun EncryptedPreferencesStorage.storeAsync(key: String, data: ByteArray) {
    withContext(Dispatchers.IO) {
        store(key, data)
    }
}

suspend fun EncryptedPreferencesStorage.retrieveAsync(key: String): ByteArray? {
    return withContext(Dispatchers.IO) {
        retrieve(key)
    }
}

suspend fun EncryptedPreferencesStorage.deleteAsync(key: String) {
    withContext(Dispatchers.IO) {
        delete(key)
    }
}

suspend fun EncryptedPreferencesStorage.clearAsync() {
    withContext(Dispatchers.IO) {
        clear()
    }
}
