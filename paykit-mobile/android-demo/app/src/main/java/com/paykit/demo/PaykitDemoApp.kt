package com.paykit.demo

import android.app.Application
import com.paykit.storage.AutoPayStorage
import com.paykit.storage.EncryptedPreferencesStorage
import com.paykit.storage.PrivateEndpointStorage
import com.paykit.storage.asAutoPayStorage
import com.paykit.storage.asPrivateEndpointStorage

/**
 * Paykit Demo Application
 *
 * Main application class that initializes secure storage and
 * provides access to Paykit services throughout the app.
 */
class PaykitDemoApp : Application() {

    companion object {
        lateinit var instance: PaykitDemoApp
            private set
    }

    // Lazy-initialized storage instances
    private var _encryptedStorage: EncryptedPreferencesStorage? = null
    private var _autoPayStorage: AutoPayStorage? = null
    private var _endpointStorage: PrivateEndpointStorage? = null

    val encryptedStorage: EncryptedPreferencesStorage
        get() = _encryptedStorage ?: createEncryptedStorage().also { _encryptedStorage = it }

    val autoPayStorage: AutoPayStorage
        get() = _autoPayStorage ?: encryptedStorage.asAutoPayStorage().also { _autoPayStorage = it }

    val endpointStorage: PrivateEndpointStorage
        get() = _endpointStorage ?: encryptedStorage.asPrivateEndpointStorage().also { _endpointStorage = it }

    override fun onCreate() {
        super.onCreate()
        instance = this
    }

    private fun createEncryptedStorage(): EncryptedPreferencesStorage {
        return EncryptedPreferencesStorage.create(
            context = applicationContext,
            fileName = "paykit_demo_storage"
        )
    }

    /**
     * Clear all app data (for testing/reset).
     */
    fun clearAllData() {
        autoPayStorage.resetToDefaults()
        endpointStorage.clear()
        encryptedStorage.clear()
    }
}
