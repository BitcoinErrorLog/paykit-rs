package com.paykit.demo

import android.app.Application
import android.util.Log

/**
 * Paykit Demo Application
 *
 * Main application class that initializes the Paykit SDK.
 */
class PaykitDemoApp : Application() {

    companion object {
        private const val TAG = "PaykitDemoApp"
        
        lateinit var instance: PaykitDemoApp
            private set
            
        /**
         * Singleton PaykitClient wrapper.
         * Lazily initialized on first access.
         * Returns placeholder wrapper if initialization fails.
         */
        val paykitClient: PaykitClientWrapper by lazy {
            Log.d(TAG, "Initializing PaykitClient...")
            try {
                PaykitClientWrapper.create()
            } catch (e: Exception) {
                Log.w(TAG, "PaykitClient initialization failed: ${e.message}, using placeholder")
                PaykitClientWrapper.placeholder()
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
        
        // Don't pre-initialize PaykitClient here to avoid native library loading issues
        // in test environments. Client will be initialized lazily when first accessed.
        Log.d(TAG, "PaykitDemoApp onCreate")
    }
}
