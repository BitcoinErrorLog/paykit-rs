// PubkyRingIntegration.kt
// Pubky Ring Integration Protocol
//
// This file defines the integration protocol for communicating with the
// real Pubky Ring app for key derivation. In production, Paykit apps
// request X25519 keys from Pubky Ring rather than storing Ed25519 seeds.
//
// Key Architecture:
//   - Ed25519 (pkarr) keys are "cold" and managed by Pubky Ring
//   - X25519 (Noise) keys are "hot" and derived on-demand by Pubky Ring
//   - Paykit apps request pre-derived keys, never the seed
//
// Intent Action: com.pubky.ring.DERIVE_KEYPAIR
// Response: JSON with secret_key_hex and public_key_hex

package com.paykit.demo.services

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import org.json.JSONObject
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

/**
 * Response from Pubky Ring key derivation
 */
data class PubkyRingKeypairResponse(
    val secretKeyHex: String,
    val publicKeyHex: String
)

/**
 * Exception types for Pubky Ring integration
 */
sealed class PubkyRingException(message: String, val errorCode: String) : Exception(message) {
    object AppNotInstalled : PubkyRingException(
        "Pubky Ring app is not installed. Please install Pubky Ring to use this feature.",
        "app_not_installed"
    )
    class RequestFailed(message: String) : PubkyRingException(
        "Request to Pubky Ring failed: $message",
        "request_failed"
    )
    object InvalidResponse : PubkyRingException(
        "Invalid response from Pubky Ring.",
        "invalid_response"
    )
    class DerivationFailed(message: String) : PubkyRingException(
        "Key derivation failed: $message",
        "derivation_failed"
    )
    object ServiceUnavailable : PubkyRingException(
        "Pubky Ring service is unavailable.",
        "service_unavailable"
    )
    object Timeout : PubkyRingException(
        "Request to Pubky Ring timed out.",
        "timeout"
    )
    object UserCancelled : PubkyRingException(
        "User cancelled the request.",
        "user_cancelled"
    )
}

/**
 * Integration protocol for Pubky Ring app
 *
 * This class handles communication with the Pubky Ring app for key derivation.
 * It uses intents to request X25519 keys derived from the user's Ed25519
 * identity stored in Pubky Ring.
 *
 * **Fallback**: If Pubky Ring is not installed, falls back to MockPubkyRingService.
 */
class PubkyRingIntegration private constructor(private val context: Context) {
    
    companion object {
        private const val PUBKY_RING_PACKAGE = "to.pubky.ring"
        private const val DERIVE_KEYPAIR_ACTION = "com.pubky.ring.DERIVE_KEYPAIR"
        private const val REQUEST_CODE_DERIVE = 1001
        private const val REQUEST_TIMEOUT_MS = 30_000L
        
        @Volatile
        private var instance: PubkyRingIntegration? = null
        
        fun getInstance(context: Context): PubkyRingIntegration {
            return instance ?: synchronized(this) {
                instance ?: PubkyRingIntegration(context.applicationContext).also { instance = it }
            }
        }
    }
    
    /**
     * Whether to use mock service as fallback when Pubky Ring is unavailable
     */
    var useMockFallback: Boolean = true
    
    // Pending callbacks for async operations
    private val pendingCallbacks = mutableMapOf<String, (Result<X25519KeypairResult>) -> Unit>()
    
    /**
     * Check if Pubky Ring app is installed
     */
    val isPubkyRingInstalled: Boolean
        get() {
            return try {
                context.packageManager.getPackageInfo(PUBKY_RING_PACKAGE, 0)
                true
            } catch (e: PackageManager.NameNotFoundException) {
                false
            }
        }
    
    /**
     * Derive X25519 keypair from Pubky Ring
     *
     * This method attempts to request key derivation from Pubky Ring.
     * If Pubky Ring is not installed and `useMockFallback` is true,
     * it falls back to MockPubkyRingService.
     *
     * @param deviceId Unique identifier for this device
     * @param epoch Key rotation epoch (increment to rotate keys)
     * @return Derived X25519 keypair
     * @throws PubkyRingException if derivation fails
     */
    suspend fun deriveX25519Keypair(deviceId: String, epoch: UInt): X25519KeypairResult {
        // Try Pubky Ring first if installed
        if (isPubkyRingInstalled) {
            return requestFromPubkyRing(deviceId, epoch)
        }
        
        // Fall back to mock service
        if (useMockFallback) {
            return useMockService(deviceId, epoch)
        }
        
        throw PubkyRingException.AppNotInstalled
    }
    
    /**
     * Get or derive X25519 keypair with caching
     *
     * This method first checks the NoiseKeyCache, then requests from
     * Pubky Ring if not cached.
     *
     * @param deviceId Unique identifier for this device
     * @param epoch Key rotation epoch
     * @return X25519 keypair (from cache or freshly derived)
     */
    suspend fun getOrDeriveKeypair(deviceId: String, epoch: UInt): X25519KeypairResult {
        // Check cache first
        NoiseKeyCache.getInstance(context).getKey(deviceId, epoch)?.let { cached ->
            return cached
        }
        
        // Derive and cache
        val keypair = deriveX25519Keypair(deviceId, epoch)
        NoiseKeyCache.getInstance(context).setKey(keypair, deviceId, epoch)
        
        return keypair
    }
    
    /**
     * Handle result from Pubky Ring activity
     *
     * Call this from your activity's onActivityResult when receiving
     * a result from Pubky Ring.
     *
     * @param requestCode The request code from onActivityResult
     * @param resultCode The result code from onActivityResult
     * @param data The intent data from onActivityResult
     * @return true if the result was handled
     */
    fun handleActivityResult(requestCode: Int, resultCode: Int, data: Intent?): Boolean {
        if (requestCode != REQUEST_CODE_DERIVE) {
            return false
        }
        
        val requestId = data?.getStringExtra("request_id") ?: return false
        val callback = synchronized(pendingCallbacks) {
            pendingCallbacks.remove(requestId)
        } ?: return false
        
        if (resultCode != Activity.RESULT_OK) {
            val errorCode = data.getStringExtra("error") ?: "user_cancelled"
            val errorMessage = data.getStringExtra("message") ?: "User cancelled"
            callback(Result.failure(mapErrorCode(errorCode, errorMessage)))
            return true
        }
        
        // Parse success response
        val secretKeyHex = data.getStringExtra("secret_key_hex")
        val publicKeyHex = data.getStringExtra("public_key_hex")
        val deviceIdResult = data.getStringExtra("device_id")
        val epochResult = data.getIntExtra("epoch", -1)
        
        if (secretKeyHex == null || publicKeyHex == null || deviceIdResult == null || epochResult < 0) {
            callback(Result.failure(PubkyRingException.InvalidResponse))
            return true
        }
        
        val result = X25519KeypairResult(
            secretKeyHex = secretKeyHex,
            publicKeyHex = publicKeyHex,
            deviceId = deviceIdResult,
            epoch = epochResult.toUInt()
        )
        
        callback(Result.success(result))
        return true
    }
    
    /**
     * Handle callback URI from Pubky Ring
     *
     * Call this from your activity's onCreate or onNewIntent when receiving
     * a callback URI from Pubky Ring.
     *
     * @param uri The callback URI
     * @return true if the URI was handled
     */
    fun handleCallbackUri(uri: Uri): Boolean {
        if (uri.scheme != "paykit" || uri.host != "pubkyring-callback") {
            return false
        }
        
        val requestId = uri.getQueryParameter("request_id") ?: return false
        val callback = synchronized(pendingCallbacks) {
            pendingCallbacks.remove(requestId)
        } ?: return false
        
        // Check for error
        val errorCode = uri.getQueryParameter("error")
        if (errorCode != null) {
            val message = uri.getQueryParameter("message") ?: "Unknown error"
            callback(Result.failure(mapErrorCode(errorCode, message)))
            return true
        }
        
        // Parse success response
        val secretKeyHex = uri.getQueryParameter("secret_key_hex")
        val publicKeyHex = uri.getQueryParameter("public_key_hex")
        val deviceId = uri.getQueryParameter("device_id")
        val epochStr = uri.getQueryParameter("epoch")
        
        if (secretKeyHex == null || publicKeyHex == null || deviceId == null || epochStr == null) {
            callback(Result.failure(PubkyRingException.InvalidResponse))
            return true
        }
        
        val result = X25519KeypairResult(
            secretKeyHex = secretKeyHex,
            publicKeyHex = publicKeyHex,
            deviceId = deviceId,
            epoch = epochStr.toUIntOrNull() ?: 0u
        )
        
        callback(Result.success(result))
        return true
    }
    
    private suspend fun requestFromPubkyRing(deviceId: String, epoch: UInt): X25519KeypairResult {
        val requestId = java.util.UUID.randomUUID().toString()
        
        return withTimeoutOrNull(REQUEST_TIMEOUT_MS) {
            suspendCancellableCoroutine { continuation ->
                // Store callback
                synchronized(pendingCallbacks) {
                    pendingCallbacks[requestId] = { result ->
                        result.fold(
                            onSuccess = { continuation.resume(it) },
                            onFailure = { continuation.resumeWithException(it) }
                        )
                    }
                }
                
                // Build intent
                val intent = Intent(DERIVE_KEYPAIR_ACTION).apply {
                    setPackage(PUBKY_RING_PACKAGE)
                    putExtra("device_id", deviceId)
                    putExtra("epoch", epoch.toInt())
                    putExtra("callback_uri", "paykit://pubkyring-callback")
                    putExtra("request_id", requestId)
                }
                
                // Try to send intent
                try {
                    val resolveInfo = context.packageManager.resolveActivity(intent, 0)
                    if (resolveInfo != null) {
                        // Start activity for result from application context
                        // Note: This requires the caller to handle the result
                        context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                    } else {
                        synchronized(pendingCallbacks) {
                            pendingCallbacks.remove(requestId)
                        }
                        continuation.resumeWithException(PubkyRingException.ServiceUnavailable)
                    }
                } catch (e: Exception) {
                    synchronized(pendingCallbacks) {
                        pendingCallbacks.remove(requestId)
                    }
                    continuation.resumeWithException(
                        PubkyRingException.RequestFailed(e.message ?: "Unknown error")
                    )
                }
                
                continuation.invokeOnCancellation {
                    synchronized(pendingCallbacks) {
                        pendingCallbacks.remove(requestId)
                    }
                }
            }
        } ?: throw PubkyRingException.Timeout
    }
    
    private fun useMockService(deviceId: String, epoch: UInt): X25519KeypairResult {
        val mock = MockPubkyRingService.getInstance(context)
        
        // Initialize mock if needed
        if (!mock.hasSeed) {
            mock.initializeWithNewSeed()
        }
        
        return mock.deriveX25519Keypair(deviceId, epoch)
    }
    
    private fun mapErrorCode(code: String, message: String): PubkyRingException {
        return when (code) {
            "key_not_found" -> PubkyRingException.DerivationFailed("No identity configured in Pubky Ring")
            "derivation_failed" -> PubkyRingException.DerivationFailed(message)
            "invalid_parameters" -> PubkyRingException.RequestFailed("Invalid parameters: $message")
            "service_unavailable" -> PubkyRingException.ServiceUnavailable
            "user_cancelled" -> PubkyRingException.UserCancelled
            else -> PubkyRingException.RequestFailed(message)
        }
    }
}

