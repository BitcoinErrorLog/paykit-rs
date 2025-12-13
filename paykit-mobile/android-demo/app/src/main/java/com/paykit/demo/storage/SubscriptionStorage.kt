package com.paykit.demo.storage

import android.content.Context
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.paykit.demo.model.StoredSubscription
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Manages persistent storage of subscriptions using EncryptedSharedPreferences.
 */
class SubscriptionStorage(context: Context) {
    
    companion object {
        private const val TAG = "SubscriptionStorage"
        private const val PREFS_NAME = "paykit_subscriptions"
        private const val SUBSCRIPTIONS_KEY = "subscriptions_list"
    }
    
    private val json = Json { 
        ignoreUnknownKeys = true 
        encodeDefaults = true
    }
    
    private val prefs by lazy {
        try {
            val masterKey = MasterKey.Builder(context)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build()
            
            EncryptedSharedPreferences.create(
                context,
                PREFS_NAME,
                masterKey,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
            )
        } catch (e: Exception) {
            Log.e(TAG, "Failed to create encrypted prefs, falling back", e)
            context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        }
    }
    
    private var subscriptionsCache: List<StoredSubscription>? = null
    
    fun listSubscriptions(): List<StoredSubscription> {
        subscriptionsCache?.let { return it }
        
        return try {
            val jsonString = prefs.getString(SUBSCRIPTIONS_KEY, null) ?: return emptyList()
            val subscriptions = json.decodeFromString<List<StoredSubscription>>(jsonString)
            subscriptionsCache = subscriptions
            subscriptions
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load subscriptions: ${e.message}")
            emptyList()
        }
    }
    
    fun getSubscription(id: String): StoredSubscription? {
        return listSubscriptions().find { it.id == id }
    }
    
    fun saveSubscription(subscription: StoredSubscription) {
        val subscriptions = listSubscriptions().toMutableList()
        val index = subscriptions.indexOfFirst { it.id == subscription.id }
        
        if (index >= 0) {
            subscriptions[index] = subscription
        } else {
            subscriptions.add(subscription)
        }
        
        persistSubscriptions(subscriptions)
    }
    
    fun deleteSubscription(id: String) {
        val subscriptions = listSubscriptions().toMutableList()
        subscriptions.removeAll { it.id == id }
        persistSubscriptions(subscriptions)
    }
    
    fun toggleActive(id: String) {
        val subscriptions = listSubscriptions().toMutableList()
        val index = subscriptions.indexOfFirst { it.id == id }
        if (index >= 0) {
            subscriptions[index] = subscriptions[index].copy(isActive = !subscriptions[index].isActive)
            persistSubscriptions(subscriptions)
        }
    }
    
    fun recordPayment(subscriptionId: String) {
        val subscriptions = listSubscriptions().toMutableList()
        val index = subscriptions.indexOfFirst { it.id == subscriptionId }
        if (index >= 0) {
            subscriptions[index] = subscriptions[index].recordPayment()
            persistSubscriptions(subscriptions)
        }
    }
    
    fun activeSubscriptions(): List<StoredSubscription> {
        return listSubscriptions().filter { it.isActive }
    }
    
    fun clearAll() {
        persistSubscriptions(emptyList())
    }
    
    private fun persistSubscriptions(subscriptions: List<StoredSubscription>) {
        try {
            val jsonString = json.encodeToString(subscriptions)
            prefs.edit().putString(SUBSCRIPTIONS_KEY, jsonString).apply()
            subscriptionsCache = subscriptions
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save subscriptions: ${e.message}")
        }
    }
}

