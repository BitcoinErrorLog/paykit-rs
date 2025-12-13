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
         */
        val paykitClient: PaykitClientWrapper by lazy {
            Log.d(TAG, "Initializing PaykitClient...")
            PaykitClientWrapper.create()
        }
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
        
        // Pre-initialize the client on app start
        Log.d(TAG, "PaykitDemoApp onCreate")
        if (paykitClient.isAvailable) {
            Log.d(TAG, "PaykitClient is available")
        } else {
            Log.w(TAG, "PaykitClient is NOT available - using fallback mode")
        }
    }
}
