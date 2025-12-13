package com.paykit.demo

import android.app.Application

/**
 * Paykit Demo Application
 *
 * Main application class that initializes the app.
 * Storage integration will be added when the SDK is fully implemented.
 */
class PaykitDemoApp : Application() {

    companion object {
        lateinit var instance: PaykitDemoApp
            private set
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
    }
}
