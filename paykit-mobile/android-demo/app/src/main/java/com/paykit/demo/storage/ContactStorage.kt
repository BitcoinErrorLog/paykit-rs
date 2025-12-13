package com.paykit.demo.storage

import android.content.Context
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.paykit.demo.model.Contact
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Manages persistent storage of contacts using EncryptedSharedPreferences.
 */
class ContactStorage(context: Context) {
    
    companion object {
        private const val TAG = "ContactStorage"
        private const val PREFS_NAME = "paykit_contacts"
        private const val CONTACTS_KEY = "contacts_list"
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
    private var contactsCache: List<Contact>? = null
    
    // MARK: - CRUD Operations
    
    /**
     * Get all contacts
     */
    fun listContacts(): List<Contact> {
        contactsCache?.let { return it }
        
        return try {
            val jsonString = prefs.getString(CONTACTS_KEY, null) ?: return emptyList()
            val contacts = json.decodeFromString<List<Contact>>(jsonString)
            contactsCache = contacts
            contacts
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load contacts: ${e.message}")
            emptyList()
        }
    }
    
    /**
     * Get a specific contact
     */
    fun getContact(id: String): Contact? {
        return listContacts().find { it.id == id }
    }
    
    /**
     * Save a new contact or update existing
     */
    fun saveContact(contact: Contact) {
        val contacts = listContacts().toMutableList()
        val index = contacts.indexOfFirst { it.id == contact.id }
        
        if (index >= 0) {
            contacts[index] = contact
        } else {
            contacts.add(contact)
        }
        
        persistContacts(contacts)
    }
    
    /**
     * Delete a contact
     */
    fun deleteContact(id: String) {
        val contacts = listContacts().toMutableList()
        contacts.removeAll { it.id == id }
        persistContacts(contacts)
    }
    
    /**
     * Search contacts by name or public key
     */
    fun searchContacts(query: String): List<Contact> {
        val lowerQuery = query.lowercase()
        return listContacts().filter { contact ->
            contact.name.lowercase().contains(lowerQuery) ||
            contact.publicKeyZ32.lowercase().contains(lowerQuery)
        }
    }
    
    /**
     * Record a payment to a contact
     */
    fun recordPayment(contactId: String) {
        val contacts = listContacts().toMutableList()
        val index = contacts.indexOfFirst { it.id == contactId }
        
        if (index >= 0) {
            contacts[index] = contacts[index].recordPayment()
            persistContacts(contacts)
        }
    }
    
    /**
     * Clear all contacts
     */
    fun clearAll() {
        persistContacts(emptyList())
    }
    
    /**
     * Import contacts (merge with existing)
     */
    fun importContacts(newContacts: List<Contact>) {
        val contacts = listContacts().toMutableList()
        
        for (newContact in newContacts) {
            if (contacts.none { it.id == newContact.id }) {
                contacts.add(newContact)
            }
        }
        
        persistContacts(contacts)
    }
    
    /**
     * Export contacts as JSON string
     */
    fun exportContacts(): String {
        return try {
            json.encodeToString(listContacts())
        } catch (e: Exception) {
            Log.e(TAG, "Failed to export contacts: ${e.message}")
            "[]"
        }
    }
    
    // MARK: - Private
    
    private fun persistContacts(contacts: List<Contact>) {
        try {
            val jsonString = json.encodeToString(contacts)
            prefs.edit().putString(CONTACTS_KEY, jsonString).apply()
            contactsCache = contacts
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save contacts: ${e.message}")
        }
    }
}

