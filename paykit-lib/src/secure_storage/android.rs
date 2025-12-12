//! Android Keystore implementation of secure key storage.
//!
//! This implementation uses the Android Keystore system
//! for secure key storage with biometric protection.
//!
//! # Platform Status
//!
//! **STUB IMPLEMENTATION**: This module provides the interface for Android Keystore
//! integration but requires the Kotlin/JNI FFI bridge to be implemented by the Android
//! host application before it can be used in production.
//!
//! The FFI functions (`ffi_store`, `ffi_retrieve`, etc.) currently return errors
//! and must be connected to Kotlin implementations via UniFFI callbacks.

use super::traits::{
    KeyMetadata, SecureKeyStorage, SecureStorageError, SecureStorageResult, StoreOptions,
};

/// Android Keystore-backed secure key storage.
///
/// Provides secure key storage using the Android Keystore system.
/// Supports biometric (fingerprint / face) protection.
///
/// ## Integration
///
/// This type is designed to be called from Kotlin via UniFFI bindings.
/// The actual Keystore operations are performed on the Android side.
pub struct KeystoreStorage {
    /// Key alias prefix for namespacing
    alias_prefix: String,
    /// Whether to use StrongBox if available
    use_strongbox: bool,
}

impl KeystoreStorage {
    /// Create a new Keystore storage with the given alias prefix.
    ///
    /// The alias prefix is used to namespace your keys within the Keystore.
    pub fn new(alias_prefix: impl Into<String>) -> Self {
        Self {
            alias_prefix: alias_prefix.into(),
            use_strongbox: false,
        }
    }

    /// Enable StrongBox (hardware security module) if available.
    ///
    /// StrongBox provides additional security guarantees on devices
    /// with dedicated secure hardware.
    pub fn with_strongbox(mut self) -> Self {
        self.use_strongbox = true;
        self
    }

    /// Get the alias prefix.
    pub fn alias_prefix(&self) -> &str {
        &self.alias_prefix
    }

    /// Check if StrongBox is enabled.
    pub fn uses_strongbox(&self) -> bool {
        self.use_strongbox
    }

    /// Build the full key alias from prefix and key ID.
    fn full_alias(&self, key_id: &str) -> String {
        format!("{}_{}", self.alias_prefix, key_id)
    }

    // NOTE: FFI Bridge Functions
    // These functions are called from Kotlin via UniFFI bindings.
    // The actual Keystore implementation is provided by the Android host app.
    // See paykit-mobile for UniFFI binding definitions.

    /// FFI: Store item in Keystore (to be implemented by Android host)
    #[allow(dead_code)]
    fn ffi_store(&self, _alias: &str, _data: &[u8], _require_auth: bool) -> Result<(), String> {
        Err("FFI not connected - call from Android host".to_string())
    }

    /// FFI: Retrieve item from Keystore (to be implemented by Android host)
    #[allow(dead_code)]
    fn ffi_retrieve(&self, _alias: &str) -> Result<Option<Vec<u8>>, String> {
        Err("FFI not connected - call from Android host".to_string())
    }

    /// FFI: Delete item from Keystore (to be implemented by Android host)
    #[allow(dead_code)]
    fn ffi_delete(&self, _alias: &str) -> Result<(), String> {
        Err("FFI not connected - call from Android host".to_string())
    }

    /// FFI: Check if item exists in Keystore (to be implemented by Android host)
    #[allow(dead_code)]
    fn ffi_exists(&self, _alias: &str) -> Result<bool, String> {
        Err("FFI not connected - call from Android host".to_string())
    }

    /// FFI: List all items in Keystore (to be implemented by Android host)
    #[allow(dead_code)]
    fn ffi_list(&self) -> Result<Vec<String>, String> {
        Err("FFI not connected - call from Android host".to_string())
    }
}

impl SecureKeyStorage for KeystoreStorage {
    async fn store(
        &self,
        key_id: &str,
        key_data: &[u8],
        options: StoreOptions,
    ) -> SecureStorageResult<()> {
        // Check if exists and handle overwrite
        if !options.overwrite {
            if self.exists(key_id).await? {
                return Err(SecureStorageError::already_exists(key_id));
            }
        }

        let _ = (key_data, self.full_alias(key_id));

        // In a real implementation, this would call the FFI bridge
        Err(SecureStorageError::unsupported(
            "Android Keystore FFI not connected - integrate with Kotlin host",
        ))
    }

    async fn retrieve(&self, key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        let _ = self.full_alias(key_id);
        Err(SecureStorageError::unsupported(
            "Android Keystore FFI not connected - integrate with Kotlin host",
        ))
    }

    async fn delete(&self, key_id: &str) -> SecureStorageResult<()> {
        let _ = self.full_alias(key_id);
        Err(SecureStorageError::unsupported(
            "Android Keystore FFI not connected - integrate with Kotlin host",
        ))
    }

    async fn exists(&self, key_id: &str) -> SecureStorageResult<bool> {
        let _ = self.full_alias(key_id);
        Err(SecureStorageError::unsupported(
            "Android Keystore FFI not connected - integrate with Kotlin host",
        ))
    }

    async fn get_metadata(&self, key_id: &str) -> SecureStorageResult<Option<KeyMetadata>> {
        let _ = self.full_alias(key_id);
        Err(SecureStorageError::unsupported(
            "Android Keystore FFI not connected - integrate with Kotlin host",
        ))
    }

    async fn list_keys(&self) -> SecureStorageResult<Vec<String>> {
        Err(SecureStorageError::unsupported(
            "Android Keystore FFI not connected - integrate with Kotlin host",
        ))
    }

    async fn clear_all(&self) -> SecureStorageResult<()> {
        Err(SecureStorageError::unsupported(
            "Android Keystore FFI not connected - integrate with Kotlin host",
        ))
    }
}

/// Kotlin integration example code.
///
/// Add this to your Android project to implement the Keystore bridge:
///
/// ```kotlin
/// import android.security.keystore.KeyGenParameterSpec
/// import android.security.keystore.KeyProperties
/// import java.security.KeyStore
/// import javax.crypto.Cipher
/// import javax.crypto.KeyGenerator
/// import javax.crypto.SecretKey
/// import javax.crypto.spec.GCMParameterSpec
///
/// class PaykitKeystoreBridge(
///     private val aliasPrefix: String,
///     private val useStrongbox: Boolean = false
/// ) {
///     private val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
///     
///     fun store(keyId: String, data: ByteArray, requireAuth: Boolean) {
///         val alias = "${aliasPrefix}_${keyId}"
///         
///         // Generate or get encryption key
///         val secretKey = getOrCreateKey(alias, requireAuth)
///         
///         // Encrypt data
///         val cipher = Cipher.getInstance("AES/GCM/NoPadding")
///         cipher.init(Cipher.ENCRYPT_MODE, secretKey)
///         val iv = cipher.iv
///         val encrypted = cipher.doFinal(data)
///         
///         // Store IV + encrypted data in EncryptedSharedPreferences
///         // (Keystore alone can't store arbitrary data)
///         storeEncrypted(alias, iv + encrypted)
///     }
///     
///     fun retrieve(keyId: String): ByteArray? {
///         val alias = "${aliasPrefix}_${keyId}"
///         
///         val stored = getEncrypted(alias) ?: return null
///         val iv = stored.sliceArray(0..11)
///         val encrypted = stored.sliceArray(12 until stored.size)
///         
///         val secretKey = keyStore.getKey(alias, null) as? SecretKey
///             ?: return null
///         
///         val cipher = Cipher.getInstance("AES/GCM/NoPadding")
///         cipher.init(Cipher.DECRYPT_MODE, secretKey, GCMParameterSpec(128, iv))
///         return cipher.doFinal(encrypted)
///     }
///     
///     private fun getOrCreateKey(alias: String, requireAuth: Boolean): SecretKey {
///         keyStore.getKey(alias, null)?.let { return it as SecretKey }
///         
///         val keyGenerator = KeyGenerator.getInstance(
///             KeyProperties.KEY_ALGORITHM_AES,
///             "AndroidKeyStore"
///         )
///         
///         val builder = KeyGenParameterSpec.Builder(
///             alias,
///             KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
///         )
///             .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
///             .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
///             .setKeySize(256)
///         
///         if (requireAuth) {
///             builder.setUserAuthenticationRequired(true)
///                 .setUserAuthenticationParameters(30, KeyProperties.AUTH_BIOMETRIC_STRONG)
///         }
///         
///         if (useStrongbox) {
///             builder.setIsStrongBoxBacked(true)
///         }
///         
///         keyGenerator.init(builder.build())
///         return keyGenerator.generateKey()
///     }
/// }
/// ```
#[allow(dead_code)]
const _KOTLIN_EXAMPLE: () = ();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keystore_storage_creation() {
        let storage = KeystoreStorage::new("com.example.app").with_strongbox();

        assert_eq!(storage.alias_prefix(), "com.example.app");
        assert!(storage.uses_strongbox());
    }

    #[test]
    fn test_full_alias() {
        let storage = KeystoreStorage::new("paykit");
        assert_eq!(storage.full_alias("my-key"), "paykit_my-key");
    }
}
