package com.paykit.demo

import android.app.Application

/**
 * Test Application class that doesn't initialize PaykitClient
 * to avoid native library loading issues in tests.
 */
class TestApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        // No PaykitClient initialization for tests
    }
}
