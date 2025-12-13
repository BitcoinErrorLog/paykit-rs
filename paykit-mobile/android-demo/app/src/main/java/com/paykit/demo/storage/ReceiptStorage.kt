package com.paykit.demo.storage

import android.content.Context
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.paykit.demo.model.PaymentDirection
import com.paykit.demo.model.PaymentStatus
import com.paykit.demo.model.Receipt
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Manages persistent storage of payment receipts using EncryptedSharedPreferences.
 */
class ReceiptStorage(context: Context) {
    
    companion object {
        private const val TAG = "ReceiptStorage"
        private const val PREFS_NAME = "paykit_receipts"
        private const val RECEIPTS_KEY = "receipts_list"
        private const val MAX_RECEIPTS_TO_KEEP = 500
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
            Log.e(TAG, "Failed to create encrypted prefs, falling back to regular prefs", e)
            context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        }
    }
    
    // In-memory cache
    private var receiptsCache: List<Receipt>? = null
    
    // MARK: - CRUD Operations
    
    /**
     * Get all receipts (newest first)
     */
    fun listReceipts(): List<Receipt> {
        receiptsCache?.let { return it }
        
        return try {
            val jsonString = prefs.getString(RECEIPTS_KEY, null) ?: return emptyList()
            val receipts = json.decodeFromString<List<Receipt>>(jsonString)
                .sortedByDescending { it.createdAt }
            receiptsCache = receipts
            receipts
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load receipts: ${e.message}")
            emptyList()
        }
    }
    
    /**
     * Get receipts filtered by status
     */
    fun listReceipts(status: PaymentStatus): List<Receipt> {
        return listReceipts().filter { it.status == status }
    }
    
    /**
     * Get receipts filtered by direction
     */
    fun listReceipts(direction: PaymentDirection): List<Receipt> {
        return listReceipts().filter { it.direction == direction }
    }
    
    /**
     * Get recent receipts (limited count)
     */
    fun recentReceipts(limit: Int = 10): List<Receipt> {
        return listReceipts().take(limit)
    }
    
    /**
     * Get a specific receipt
     */
    fun getReceipt(id: String): Receipt? {
        return listReceipts().find { it.id == id }
    }
    
    /**
     * Add a new receipt
     */
    fun addReceipt(receipt: Receipt) {
        val receipts = listReceipts().toMutableList()
        
        // Add new receipt at the beginning (newest first)
        receipts.add(0, receipt)
        
        // Trim to max size
        val trimmed = if (receipts.size > MAX_RECEIPTS_TO_KEEP) {
            receipts.take(MAX_RECEIPTS_TO_KEEP)
        } else {
            receipts
        }
        
        persistReceipts(trimmed)
    }
    
    /**
     * Update an existing receipt
     */
    fun updateReceipt(receipt: Receipt) {
        val receipts = listReceipts().toMutableList()
        val index = receipts.indexOfFirst { it.id == receipt.id }
        
        if (index >= 0) {
            receipts[index] = receipt
            persistReceipts(receipts)
        }
    }
    
    /**
     * Delete a receipt
     */
    fun deleteReceipt(id: String) {
        val receipts = listReceipts().toMutableList()
        receipts.removeAll { it.id == id }
        persistReceipts(receipts)
    }
    
    /**
     * Search receipts by counterparty or memo
     */
    fun searchReceipts(query: String): List<Receipt> {
        val lowerQuery = query.lowercase()
        return listReceipts().filter { receipt ->
            receipt.displayName.lowercase().contains(lowerQuery) ||
            receipt.counterpartyKey.lowercase().contains(lowerQuery) ||
            (receipt.memo?.lowercase()?.contains(lowerQuery) ?: false)
        }
    }
    
    /**
     * Get receipts for a specific counterparty
     */
    fun receiptsForCounterparty(publicKey: String): List<Receipt> {
        return listReceipts().filter { it.counterpartyKey == publicKey }
    }
    
    /**
     * Clear all receipts
     */
    fun clearAll() {
        persistReceipts(emptyList())
    }
    
    // MARK: - Statistics
    
    /**
     * Total sent amount
     */
    fun totalSent(): Long {
        return listReceipts(PaymentDirection.SENT)
            .filter { it.status == PaymentStatus.COMPLETED }
            .sumOf { it.amountSats }
    }
    
    /**
     * Total received amount
     */
    fun totalReceived(): Long {
        return listReceipts(PaymentDirection.RECEIVED)
            .filter { it.status == PaymentStatus.COMPLETED }
            .sumOf { it.amountSats }
    }
    
    /**
     * Count of completed transactions
     */
    fun completedCount(): Int {
        return listReceipts(PaymentStatus.COMPLETED).size
    }
    
    /**
     * Count of pending transactions
     */
    fun pendingCount(): Int {
        return listReceipts(PaymentStatus.PENDING).size
    }
    
    // MARK: - Private
    
    private fun persistReceipts(receipts: List<Receipt>) {
        try {
            val jsonString = json.encodeToString(receipts)
            prefs.edit().putString(RECEIPTS_KEY, jsonString).apply()
            receiptsCache = receipts
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save receipts: ${e.message}")
        }
    }
}

