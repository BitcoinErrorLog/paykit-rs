// PubkyStorageAdapter.kt
// Adapter for Pubky Storage Operations
//
// This adapter implements the Pubky storage callback interfaces
// to enable real Pubky directory operations in Paykit mobile apps.
//
// It uses HTTP requests to communicate with Pubky homeservers.

package com.paykit.demo.services

import android.content.Context
import com.paykit.mobile.*
import kotlinx.coroutines.runBlocking
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.io.IOException
import java.util.concurrent.TimeUnit

/**
 * Adapter for unauthenticated (read-only) Pubky storage operations.
 *
 * This adapter makes HTTP requests to Pubky homeservers to read
 * public data from other users' storage.
 */
class PubkyUnauthenticatedStorageAdapter(
    private val homeserverBaseURL: String? = null
) : PubkyUnauthenticatedStorageCallback {
    
    private val client: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(30, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .writeTimeout(60, TimeUnit.SECONDS)
        .build()
    
    override fun get(ownerPubkey: String, path: String): StorageGetResult {
        // Construct URL: https://_pubky.{ownerPubkey}{path}
        val urlString = if (homeserverBaseURL != null) {
            "$homeserverBaseURL/pubky$ownerPubkey$path"
        } else {
            "https://_pubky.$ownerPubkey$path"
        }
        
        val request = Request.Builder()
            .url(urlString)
            .get()
            .build()
        
        return try {
            val response = client.newCall(request).execute()
            
            when {
                response.code == 404 -> StorageGetResult.ok(content = null)
                response.code in 200..299 -> {
                    val body = response.body?.string()
                    StorageGetResult.ok(content = body)
                }
                else -> StorageGetResult.err(
                    message = "HTTP ${response.code}"
                )
            }
        } catch (e: IOException) {
            StorageGetResult.err(message = "Network error: ${e.message}")
        } catch (e: Exception) {
            StorageGetResult.err(message = "Error: ${e.message}")
        }
    }
    
    override fun list(ownerPubkey: String, prefix: String): StorageListResult {
        val urlString = if (homeserverBaseURL != null) {
            "$homeserverBaseURL/pubky$ownerPubkey$prefix?shallow=true"
        } else {
            "https://_pubky.$ownerPubkey$prefix?shallow=true"
        }
        
        val request = Request.Builder()
            .url(urlString)
            .get()
            .build()
        
        return try {
            val response = client.newCall(request).execute()
            
            when {
                response.code == 404 -> StorageListResult.ok(entries = emptyList())
                response.code in 200..299 -> {
                    val body = response.body?.string()
                    if (body.isNullOrEmpty()) {
                        StorageListResult.ok(entries = emptyList())
                    } else {
                        try {
                            // Parse JSON array of resources
                            val jsonArray = JSONArray(body)
                            val entries = mutableListOf<String>()
                            for (i in 0 until jsonArray.length()) {
                                val item = jsonArray.getJSONObject(i)
                                entries.add(item.getString("path"))
                            }
                            StorageListResult.ok(entries = entries)
                        } catch (e: Exception) {
                            // Try parsing as simple string array
                            try {
                                val jsonArray = JSONArray(body)
                                val entries = mutableListOf<String>()
                                for (i in 0 until jsonArray.length()) {
                                    entries.add(jsonArray.getString(i))
                                }
                                StorageListResult.ok(entries = entries)
                            } catch (e2: Exception) {
                                StorageListResult.err(
                                    message = "Failed to parse response: ${e2.message}"
                                )
                            }
                        }
                    }
                }
                else -> StorageListResult.err(message = "HTTP ${response.code}")
            }
        } catch (e: IOException) {
            StorageListResult.err(message = "Network error: ${e.message}")
        } catch (e: Exception) {
            StorageListResult.err(message = "Error: ${e.message}")
        }
    }
}

/**
 * Adapter for authenticated Pubky storage operations.
 *
 * This adapter makes HTTP requests to Pubky homeservers with
 * session authentication to read/write the owner's storage.
 */
class PubkyAuthenticatedStorageAdapter(
    private val sessionId: String,
    private val homeserverBaseURL: String? = null
) : PubkyAuthenticatedStorageCallback {
    
    private val client: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(30, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .writeTimeout(60, TimeUnit.SECONDS)
        .cookieJar(object : CookieJar {
            private val cookies = mutableListOf<Cookie>()
            
            override fun saveFromResponse(url: HttpUrl, cookies: List<Cookie>) {
                this.cookies.addAll(cookies)
            }
            
            override fun loadForRequest(url: HttpUrl): List<Cookie> {
                return this.cookies
            }
        })
        .build()
    
    override fun put(path: String, content: String): StorageOperationResult {
        val urlString = if (homeserverBaseURL != null) {
            "$homeserverBaseURL$path"
        } else {
            "https://homeserver.pubky.app$path"
        }
        
        val mediaType = "application/json".toMediaType()
        val requestBody = content.toRequestBody(mediaType)
        
        val request = Request.Builder()
            .url(urlString)
            .put(requestBody)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Cookie", "session=$sessionId")
            .build()
        
        return try {
            val response = client.newCall(request).execute()
            
            if (response.code in 200..299) {
                StorageOperationResult.ok()
            } else {
                StorageOperationResult.err(message = "HTTP ${response.code}")
            }
        } catch (e: IOException) {
            StorageOperationResult.err(message = "Network error: ${e.message}")
        } catch (e: Exception) {
            StorageOperationResult.err(message = "Error: ${e.message}")
        }
    }
    
    override fun get(path: String): StorageGetResult {
        val urlString = if (homeserverBaseURL != null) {
            "$homeserverBaseURL$path"
        } else {
            "https://homeserver.pubky.app$path"
        }
        
        val request = Request.Builder()
            .url(urlString)
            .get()
            .header("Accept", "application/json")
            .header("Cookie", "session=$sessionId")
            .build()
        
        return try {
            val response = client.newCall(request).execute()
            
            when {
                response.code == 404 -> StorageGetResult.ok(content = null)
                response.code in 200..299 -> {
                    val body = response.body?.string()
                    StorageGetResult.ok(content = body)
                }
                else -> StorageGetResult.err(message = "HTTP ${response.code}")
            }
        } catch (e: IOException) {
            StorageGetResult.err(message = "Network error: ${e.message}")
        } catch (e: Exception) {
            StorageGetResult.err(message = "Error: ${e.message}")
        }
    }
    
    override fun delete(path: String): StorageOperationResult {
        val urlString = if (homeserverBaseURL != null) {
            "$homeserverBaseURL$path"
        } else {
            "https://homeserver.pubky.app$path"
        }
        
        val request = Request.Builder()
            .url(urlString)
            .delete()
            .header("Cookie", "session=$sessionId")
            .build()
        
        return try {
            val response = client.newCall(request).execute()
            
            // 204 No Content or 200 OK are both valid for DELETE
            if (response.code in 200..299 || response.code == 404) {
                StorageOperationResult.ok()
            } else {
                StorageOperationResult.err(message = "HTTP ${response.code}")
            }
        } catch (e: IOException) {
            StorageOperationResult.err(message = "Network error: ${e.message}")
        } catch (e: Exception) {
            StorageOperationResult.err(message = "Error: ${e.message}")
        }
    }
    
    override fun list(prefix: String): StorageListResult {
        val urlString = if (homeserverBaseURL != null) {
            "$homeserverBaseURL$prefix?shallow=true"
        } else {
            "https://homeserver.pubky.app$prefix?shallow=true"
        }
        
        val request = Request.Builder()
            .url(urlString)
            .get()
            .header("Accept", "application/json")
            .header("Cookie", "session=$sessionId")
            .build()
        
        return try {
            val response = client.newCall(request).execute()
            
            when {
                response.code == 404 -> StorageListResult.ok(entries = emptyList())
                response.code in 200..299 -> {
                    val body = response.body?.string()
                    if (body.isNullOrEmpty()) {
                        StorageListResult.ok(entries = emptyList())
                    } else {
                        try {
                            val jsonArray = JSONArray(body)
                            val entries = mutableListOf<String>()
                            for (i in 0 until jsonArray.length()) {
                                val item = jsonArray.getJSONObject(i)
                                entries.add(item.getString("path"))
                            }
                            StorageListResult.ok(entries = entries)
                        } catch (e: Exception) {
                            try {
                                val jsonArray = JSONArray(body)
                                val entries = mutableListOf<String>()
                                for (i in 0 until jsonArray.length()) {
                                    entries.add(jsonArray.getString(i))
                                }
                                StorageListResult.ok(entries = entries)
                            } catch (e2: Exception) {
                                StorageListResult.err(
                                    message = "Failed to parse response: ${e2.message}"
                                )
                            }
                        }
                    }
                }
                else -> StorageListResult.err(message = "HTTP ${response.code}")
            }
        } catch (e: IOException) {
            StorageListResult.err(message = "Network error: ${e.message}")
        } catch (e: Exception) {
            StorageListResult.err(message = "Error: ${e.message}")
        }
    }
}

