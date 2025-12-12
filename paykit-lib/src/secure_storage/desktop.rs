//! Desktop secure key storage implementation.
//!
//! This implementation provides secure key storage for desktop platforms
//! (macOS, Windows, Linux) using OS-specific secure storage APIs:
//!
//! - **macOS**: Keychain Services (via security-framework crate)
//! - **Windows**: Windows Credential Manager (via windows crate)
//! - **Linux**: Secret Service API (via secret-service crate)
//!
//! # Thread Safety
//!
//! This storage uses `RwLock` for thread-safe access to the fallback storage.
//! Native OS storage is assumed to be thread-safe.
//!
//! # Fallback Behavior
//!
//! When native storage fails or is unavailable, the implementation falls back
//! to in-memory storage. Use `with_fallback_only()` to force fallback mode
//! for testing purposes.

use std::collections::HashMap;
use std::sync::RwLock;

use super::traits::{
    KeyMetadata, SecureKeyStorage, SecureStorageError, SecureStorageErrorCode,
    SecureStorageResult, StoreOptions,
};

/// Desktop secure key storage.
///
/// On desktop platforms, this uses:
/// - **macOS**: Keychain Services (via security-framework crate)
/// - **Windows**: Windows Credential Manager (via windows crate)
/// - **Linux**: Secret Service API (via secret-service crate)
///
/// As a fallback when native APIs fail, it uses in-memory storage.
/// Note: Fallback storage is NOT secure and should only be used for testing.
pub struct DesktopKeyStorage {
    /// Application identifier for namespacing
    app_id: String,
    /// Fallback to in-memory storage (NOT secure, for testing only)
    fallback_storage: RwLock<HashMap<String, StoredKey>>,
    /// Whether to attempt native OS storage first
    use_native: bool,
}

#[derive(Clone)]
struct StoredKey {
    data: Vec<u8>,
    metadata: KeyMetadata,
}

impl DesktopKeyStorage {
    /// Create a new desktop key storage.
    ///
    /// The app_id is used to namespace keys in the OS keychain.
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            fallback_storage: RwLock::new(HashMap::new()),
            use_native: true,
        }
    }

    /// Disable native OS storage and use in-memory fallback only.
    ///
    /// **Warning**: This is NOT secure and should only be used for testing.
    pub fn with_fallback_only(mut self) -> Self {
        self.use_native = false;
        self
    }

    /// Get the application ID.
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// Check if using native OS storage.
    pub fn uses_native(&self) -> bool {
        self.use_native
    }

    /// Build a service name for the keychain entry.
    fn service_name(&self, key_id: &str) -> String {
        format!("{}.{}", self.app_id, key_id)
    }

    // ========================================================================
    // macOS Keychain implementation
    // ========================================================================

    #[cfg(target_os = "macos")]
    fn store_native(&self, key_id: &str, data: &[u8]) -> SecureStorageResult<()> {
        use security_framework::passwords::{set_generic_password, delete_generic_password};
        
        let service = self.service_name(key_id);
        let account = key_id;
        
        // Delete existing entry if present (ignore errors)
        let _ = delete_generic_password(&service, account);
        
        // Store the new password
        set_generic_password(&service, account, data)
            .map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::EncryptionFailed,
                format!("macOS Keychain store failed: {}", e),
            ))
    }

    #[cfg(target_os = "macos")]
    fn retrieve_native(&self, key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        use security_framework::passwords::get_generic_password;
        
        let service = self.service_name(key_id);
        let account = key_id;
        
        match get_generic_password(&service, account) {
            Ok(data) => Ok(Some(data.to_vec())),
            Err(e) => {
                // Check if it's a "not found" error
                let err_str = e.to_string();
                if err_str.contains("not found") || err_str.contains("-25300") {
                    Ok(None)
                } else {
                    Err(SecureStorageError::new(
                        SecureStorageErrorCode::DecryptionFailed,
                        format!("macOS Keychain retrieve failed: {}", e),
                    ))
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn delete_native(&self, key_id: &str) -> SecureStorageResult<()> {
        use security_framework::passwords::delete_generic_password;
        
        let service = self.service_name(key_id);
        let account = key_id;
        
        delete_generic_password(&service, account)
            .map_err(|e| {
                let err_str = e.to_string();
                if err_str.contains("not found") || err_str.contains("-25300") {
                    SecureStorageError::not_found(key_id)
                } else {
                    SecureStorageError::new(
                        SecureStorageErrorCode::Internal,
                        format!("macOS Keychain delete failed: {}", e),
                    )
                }
            })
    }

    // ========================================================================
    // Windows Credential Manager implementation
    // ========================================================================

    #[cfg(target_os = "windows")]
    fn store_native(&self, key_id: &str, data: &[u8]) -> SecureStorageResult<()> {
        use windows::core::PCWSTR;
        use windows::Win32::Security::Credentials::{
            CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        };
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let target_name: Vec<u16> = OsStr::new(&self.service_name(key_id))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let credential = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: PCWSTR(target_name.as_ptr()),
            Comment: PCWSTR::null(),
            LastWritten: Default::default(),
            CredentialBlobSize: data.len() as u32,
            CredentialBlob: data.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: PCWSTR::null(),
            UserName: PCWSTR::null(),
        };

        unsafe {
            CredWriteW(&credential, 0)
                .map_err(|e| SecureStorageError::new(
                    SecureStorageErrorCode::EncryptionFailed,
                    format!("Windows Credential Manager store failed: {}", e),
                ))
        }
    }

    #[cfg(target_os = "windows")]
    fn retrieve_native(&self, key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        use windows::core::PCWSTR;
        use windows::Win32::Security::Credentials::{
            CredReadW, CredFree, CRED_TYPE_GENERIC,
        };
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let target_name: Vec<u16> = OsStr::new(&self.service_name(key_id))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let mut credential_ptr = std::ptr::null_mut();
            match CredReadW(PCWSTR(target_name.as_ptr()), CRED_TYPE_GENERIC, 0, &mut credential_ptr) {
                Ok(()) => {
                    let credential = &*credential_ptr;
                    let data = std::slice::from_raw_parts(
                        credential.CredentialBlob,
                        credential.CredentialBlobSize as usize,
                    ).to_vec();
                    CredFree(credential_ptr as *const std::ffi::c_void);
                    Ok(Some(data))
                }
                Err(e) => {
                    // Error code 1168 (ERROR_NOT_FOUND) means credential not found
                    if e.code().0 as u32 == 1168 {
                        Ok(None)
                    } else {
                        Err(SecureStorageError::new(
                            SecureStorageErrorCode::DecryptionFailed,
                            format!("Windows Credential Manager retrieve failed: {}", e),
                        ))
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn delete_native(&self, key_id: &str) -> SecureStorageResult<()> {
        use windows::core::PCWSTR;
        use windows::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let target_name: Vec<u16> = OsStr::new(&self.service_name(key_id))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            CredDeleteW(PCWSTR(target_name.as_ptr()), CRED_TYPE_GENERIC, 0)
                .map_err(|e| {
                    if e.code().0 as u32 == 1168 {
                        SecureStorageError::not_found(key_id)
                    } else {
                        SecureStorageError::new(
                            SecureStorageErrorCode::Internal,
                            format!("Windows Credential Manager delete failed: {}", e),
                        )
                    }
                })
        }
    }

    // ========================================================================
    // Linux Secret Service implementation
    // ========================================================================

    #[cfg(target_os = "linux")]
    async fn store_native_async(&self, key_id: &str, data: &[u8]) -> SecureStorageResult<()> {
        use secret_service::{EncryptionType, SecretService};
        use std::collections::HashMap as StdHashMap;

        let ss = SecretService::connect(EncryptionType::Dh)
            .await
            .map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                format!("Secret Service connection failed: {}", e),
            ))?;

        let collection = ss.get_default_collection()
            .await
            .map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                format!("Failed to get default collection: {}", e),
            ))?;

        // Unlock collection if necessary
        if collection.is_locked().await.unwrap_or(true) {
            collection.unlock().await.map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::AccessDenied,
                format!("Failed to unlock collection: {}", e),
            ))?;
        }

        let mut attributes: StdHashMap<&str, &str> = StdHashMap::new();
        let service_name = self.service_name(key_id);
        attributes.insert("application", &self.app_id);
        attributes.insert("key_id", key_id);

        // Delete existing secret if present
        let items = collection.search_items(attributes.clone())
            .await
            .unwrap_or_default();
        for item in items {
            let _ = item.delete().await;
        }

        // Create new secret
        collection.create_item(
            &service_name,
            attributes,
            data,
            true, // replace if exists
            "text/plain",
        ).await.map_err(|e| SecureStorageError::new(
            SecureStorageErrorCode::EncryptionFailed,
            format!("Failed to store secret: {}", e),
        ))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn retrieve_native_async(&self, key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        use secret_service::{EncryptionType, SecretService};
        use std::collections::HashMap as StdHashMap;

        let ss = SecretService::connect(EncryptionType::Dh)
            .await
            .map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                format!("Secret Service connection failed: {}", e),
            ))?;

        let collection = ss.get_default_collection()
            .await
            .map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                format!("Failed to get default collection: {}", e),
            ))?;

        // Unlock collection if necessary
        if collection.is_locked().await.unwrap_or(true) {
            collection.unlock().await.map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::AccessDenied,
                format!("Failed to unlock collection: {}", e),
            ))?;
        }

        let mut attributes: StdHashMap<&str, &str> = StdHashMap::new();
        attributes.insert("application", &self.app_id);
        attributes.insert("key_id", key_id);

        let items = collection.search_items(attributes)
            .await
            .map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                format!("Failed to search secrets: {}", e),
            ))?;

        if items.is_empty() {
            return Ok(None);
        }

        let item = &items[0];
        
        // Unlock item if necessary
        if item.is_locked().await.unwrap_or(true) {
            item.unlock().await.map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::AccessDenied,
                format!("Failed to unlock item: {}", e),
            ))?;
        }

        let secret = item.get_secret()
            .await
            .map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::DecryptionFailed,
                format!("Failed to retrieve secret: {}", e),
            ))?;

        Ok(Some(secret))
    }

    #[cfg(target_os = "linux")]
    async fn delete_native_async(&self, key_id: &str) -> SecureStorageResult<()> {
        use secret_service::{EncryptionType, SecretService};
        use std::collections::HashMap as StdHashMap;

        let ss = SecretService::connect(EncryptionType::Dh)
            .await
            .map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                format!("Secret Service connection failed: {}", e),
            ))?;

        let collection = ss.get_default_collection()
            .await
            .map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                format!("Failed to get default collection: {}", e),
            ))?;

        let mut attributes: StdHashMap<&str, &str> = StdHashMap::new();
        attributes.insert("application", &self.app_id);
        attributes.insert("key_id", key_id);

        let items = collection.search_items(attributes)
            .await
            .map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                format!("Failed to search secrets: {}", e),
            ))?;

        if items.is_empty() {
            return Err(SecureStorageError::not_found(key_id));
        }

        for item in items {
            item.delete().await.map_err(|e| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                format!("Failed to delete secret: {}", e),
            ))?;
        }

        Ok(())
    }

    // Linux sync wrappers (native methods are async for secret-service)
    #[cfg(target_os = "linux")]
    fn store_native(&self, _key_id: &str, _data: &[u8]) -> SecureStorageResult<()> {
        // On Linux, we must use the async version
        Err(SecureStorageError::new(
            SecureStorageErrorCode::Internal,
            "Use async store method on Linux".to_string(),
        ))
    }

    #[cfg(target_os = "linux")]
    fn retrieve_native(&self, _key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        // On Linux, we must use the async version
        Err(SecureStorageError::new(
            SecureStorageErrorCode::Internal,
            "Use async retrieve method on Linux".to_string(),
        ))
    }

    #[cfg(target_os = "linux")]
    fn delete_native(&self, _key_id: &str) -> SecureStorageResult<()> {
        // On Linux, we must use the async version
        Err(SecureStorageError::new(
            SecureStorageErrorCode::Internal,
            "Use async delete method on Linux".to_string(),
        ))
    }

    // ========================================================================
    // Fallback implementation (for unsupported platforms or testing)
    // ========================================================================

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    fn store_native(&self, _key_id: &str, _data: &[u8]) -> SecureStorageResult<()> {
        Err(SecureStorageError::unsupported("native secure storage"))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    fn retrieve_native(&self, _key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        Err(SecureStorageError::unsupported("native secure storage"))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    fn delete_native(&self, _key_id: &str) -> SecureStorageResult<()> {
        Err(SecureStorageError::unsupported("native secure storage"))
    }

    /// Store using fallback in-memory storage.
    ///
    /// **Warning**: This is NOT secure and should only be used for testing.
    fn store_fallback(
        &self,
        key_id: &str,
        data: &[u8],
        options: &StoreOptions,
    ) -> SecureStorageResult<()> {
        let mut storage = self
            .fallback_storage
            .write()
            .map_err(|_| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                "Lock poisoned during store_fallback",
            ))?;

        if storage.contains_key(key_id) && !options.overwrite {
            return Err(SecureStorageError::already_exists(key_id));
        }

        let metadata = KeyMetadata::new(key_id, data.len()).with_auth(options.require_auth);

        storage.insert(
            key_id.to_string(),
            StoredKey {
                data: data.to_vec(),
                metadata,
            },
        );

        Ok(())
    }

    /// Retrieve using fallback storage.
    fn retrieve_fallback(&self, key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        let storage = self
            .fallback_storage
            .read()
            .map_err(|_| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                "Lock poisoned during retrieve_fallback",
            ))?;
        Ok(storage.get(key_id).map(|entry| entry.data.clone()))
    }

    /// Delete using fallback storage.
    fn delete_fallback(&self, key_id: &str) -> SecureStorageResult<()> {
        let mut storage = self
            .fallback_storage
            .write()
            .map_err(|_| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                "Lock poisoned during delete_fallback",
            ))?;
        if storage.remove(key_id).is_some() {
            Ok(())
        } else {
            Err(SecureStorageError::not_found(key_id))
        }
    }

    fn exists_fallback(&self, key_id: &str) -> SecureStorageResult<bool> {
        let storage = self
            .fallback_storage
            .read()
            .map_err(|_| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                "Lock poisoned during exists_fallback",
            ))?;
        Ok(storage.contains_key(key_id))
    }

    fn get_metadata_fallback(&self, key_id: &str) -> SecureStorageResult<Option<KeyMetadata>> {
        let storage = self
            .fallback_storage
            .read()
            .map_err(|_| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                "Lock poisoned during get_metadata_fallback",
            ))?;
        Ok(storage.get(key_id).map(|e| e.metadata.clone()))
    }

    fn list_keys_fallback(&self) -> SecureStorageResult<Vec<String>> {
        let storage = self
            .fallback_storage
            .read()
            .map_err(|_| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                "Lock poisoned during list_keys_fallback",
            ))?;
        Ok(storage.keys().cloned().collect())
    }

    fn clear_all_fallback(&self) -> SecureStorageResult<()> {
        self.fallback_storage
            .write()
            .map_err(|_| SecureStorageError::new(
                SecureStorageErrorCode::Internal,
                "Lock poisoned during clear_all_fallback",
            ))?
            .clear();
        Ok(())
    }
}

impl SecureKeyStorage for DesktopKeyStorage {
    async fn store(
        &self,
        key_id: &str,
        key_data: &[u8],
        options: StoreOptions,
    ) -> SecureStorageResult<()> {
        if self.use_native {
            // Try native storage first
            #[cfg(target_os = "linux")]
            {
                match self.store_native_async(key_id, key_data).await {
                    Ok(()) => return Ok(()),
                    Err(_) => {} // Fall through to fallback
                }
            }
            
            #[cfg(not(target_os = "linux"))]
            {
                // Try native storage, fall through to fallback on error
                if self.store_native(key_id, key_data).is_ok() {
                    return Ok(());
                }
            }
        }

        // Use fallback
        self.store_fallback(key_id, key_data, &options)
    }

    async fn retrieve(&self, key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        if self.use_native {
            // Try native storage first
            #[cfg(target_os = "linux")]
            {
                match self.retrieve_native_async(key_id).await {
                    Ok(data) => return Ok(data),
                    Err(_) => {} // Fall through to fallback
                }
            }
            
            #[cfg(not(target_os = "linux"))]
            {
                // Try native storage, fall through to fallback on error
                if let Ok(data) = self.retrieve_native(key_id) {
                    return Ok(data);
                }
            }
        }

        // Use fallback
        self.retrieve_fallback(key_id)
    }

    async fn delete(&self, key_id: &str) -> SecureStorageResult<()> {
        if self.use_native {
            // Try native storage first
            #[cfg(target_os = "linux")]
            {
                match self.delete_native_async(key_id).await {
                    Ok(()) => return Ok(()),
                    Err(e) if e.code == SecureStorageErrorCode::NotFound => {
                        return Err(e);
                    }
                    Err(_) => {} // Fall through to fallback for other errors
                }
            }
            
            #[cfg(not(target_os = "linux"))]
            {
                match self.delete_native(key_id) {
                    Ok(()) => return Ok(()),
                    Err(e) if e.code == SecureStorageErrorCode::NotFound => {
                        return Err(e);
                    }
                    Err(_) => {} // Fall through to fallback for other errors
                }
            }
        }

        // Use fallback
        self.delete_fallback(key_id)
    }

    async fn exists(&self, key_id: &str) -> SecureStorageResult<bool> {
        if self.use_native {
            // Check native first, then fallback
            #[cfg(target_os = "linux")]
            {
                if let Ok(Some(_)) = self.retrieve_native_async(key_id).await {
                    return Ok(true);
                }
            }
            
            #[cfg(not(target_os = "linux"))]
            {
                if let Ok(Some(_)) = self.retrieve_native(key_id) {
                    return Ok(true);
                }
            }
        }

        // Check fallback
        self.exists_fallback(key_id)
    }

    async fn get_metadata(&self, key_id: &str) -> SecureStorageResult<Option<KeyMetadata>> {
        // Metadata is only stored in fallback (native stores don't track custom metadata)
        self.get_metadata_fallback(key_id)
    }

    async fn list_keys(&self) -> SecureStorageResult<Vec<String>> {
        // List is only available from fallback (native stores don't support enumeration easily)
        self.list_keys_fallback()
    }

    async fn clear_all(&self) -> SecureStorageResult<()> {
        // Only clear fallback storage - native storage would need enumeration
        self.clear_all_fallback()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secure_storage::traits::SecureKeyStorageExt;

    #[test]
    fn test_desktop_storage_creation() {
        let storage = DesktopKeyStorage::new("com.example.app");
        assert_eq!(storage.app_id(), "com.example.app");
        assert!(storage.uses_native());
    }

    #[tokio::test]
    async fn test_fallback_storage() {
        let storage = DesktopKeyStorage::new("test").with_fallback_only();
        assert!(!storage.uses_native());

        // Test basic operations
        storage.store_simple("key1", b"data1").await.unwrap();
        let retrieved = storage.retrieve("key1").await.unwrap();
        assert_eq!(retrieved, Some(b"data1".to_vec()));

        storage.delete("key1").await.unwrap();
        assert!(!storage.exists("key1").await.unwrap());
    }

    #[test]
    fn test_service_name() {
        let storage = DesktopKeyStorage::new("com.example.app");
        assert_eq!(storage.service_name("my-key"), "com.example.app.my-key");
    }
}
