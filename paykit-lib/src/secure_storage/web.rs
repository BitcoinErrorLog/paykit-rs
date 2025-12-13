//! Web SubtleCrypto implementation of secure key storage.
//!
//! This implementation uses the Web Crypto API (SubtleCrypto)
//! combined with IndexedDB for secure key storage in browsers.
//!
//! ## Security Model
//!
//! Uses a hybrid approach:
//! - Random master key generated on first use
//! - Master key encrypted with password-derived key (PBKDF2)
//! - All data encrypted with master key using AES-256-GCM
//! - Encrypted master key stored in IndexedDB
//! - User must provide password to unlock storage

#[cfg(target_arch = "wasm32")]
use super::traits::{
    KeyMetadata, SecureKeyStorage, SecureStorageError, SecureStorageErrorCode, SecureStorageResult,
    StoreOptions,
};
#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Object, Promise, Uint8Array};
#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{
    Crypto, IdbDatabase, IdbFactory, IdbObjectStore, IdbOpenDbRequest, IdbRequest, IdbTransaction,
    IdbTransactionMode, IdbVersionChangeEvent, SubtleCrypto, Window,
};

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

/// Web Crypto-backed secure key storage.
///
/// Provides key storage using:
/// - SubtleCrypto for encryption/decryption
/// - IndexedDB for persistent storage
#[cfg(target_arch = "wasm32")]
pub struct WebCryptoStorage {
    /// Database name for IndexedDB
    db_name: String,
    /// Object store name for keys
    store_name: String,
    /// Object store name for metadata
    metadata_store_name: String,
    /// Cached master key (wrapped for interior mutability)
    master_key: Rc<RefCell<Option<CryptoKey>>>,
    /// Password for unlocking (temporary, should be cleared after use)
    password: Rc<RefCell<Option<Vec<u8>>>>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize, Deserialize)]
struct StoredKey {
    encrypted_data: Vec<u8>, // IV (12 bytes) + ciphertext
    created_at: i64,
    last_used_at: Option<i64>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize, Deserialize)]
struct MasterKeyData {
    encrypted_master_key: Vec<u8>, // IV (12 bytes) + encrypted master key
    salt: Vec<u8>,                 // Salt for PBKDF2
}

#[cfg(target_arch = "wasm32")]
impl WebCryptoStorage {
    /// Create a new Web Crypto storage.
    pub fn new(db_name: impl Into<String>) -> Self {
        Self {
            db_name: db_name.into(),
            store_name: "paykit_keys".to_string(),
            metadata_store_name: "paykit_metadata".to_string(),
            master_key: Rc::new(RefCell::new(None)),
            password: Rc::new(RefCell::new(None)),
        }
    }

    /// Set a custom object store name.
    pub fn with_store_name(mut self, name: impl Into<String>) -> Self {
        self.store_name = name.into();
        self
    }

    /// Set password for unlocking storage.
    /// This should be called before any operations that require the master key.
    pub fn set_password(&self, password: Vec<u8>) {
        *self.password.borrow_mut() = Some(password);
    }

    /// Clear the password from memory.
    pub fn clear_password(&self) {
        *self.password.borrow_mut() = None;
    }

    /// Get the database name.
    pub fn db_name(&self) -> &str {
        &self.db_name
    }

    /// Get the object store name.
    pub fn store_name(&self) -> &str {
        &self.store_name
    }

    /// Initialize the IndexedDB database.
    async fn init_db(&self) -> SecureStorageResult<IdbDatabase> {
        let window: Window = web_sys::window()
            .ok_or_else(|| SecureStorageError::unsupported("No window object"))?;

        let idb_factory: IdbFactory = window
            .indexed_db()
            .map_err(|_| SecureStorageError::unsupported("IndexedDB not available"))?;

        let open_request: IdbOpenDbRequest = idb_factory
            .open_with_u32(&self.db_name, 1)
            .map_err(|_| SecureStorageError::unsupported("Failed to open database"))?;

        // Set up onupgradeneeded handler to create object stores
        let store_name = self.store_name.clone();
        let metadata_store_name = self.metadata_store_name.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::IdbVersionChangeEvent| {
            let target = event.target().unwrap();
            let request: IdbOpenDbRequest = target.dyn_into().unwrap();
            let db_result = request.result();
            if let Some(db_js) = db_result {
                let db: IdbDatabase = db_js.dyn_into().unwrap();

                // Create object stores if they don't exist
                if !db.object_store_names().contains(&store_name) {
                    let _ = db.create_object_store(&store_name);
                }
                if !db.object_store_names().contains(&metadata_store_name) {
                    let _ = db.create_object_store(&metadata_store_name);
                }
            }
        }) as Box<dyn FnMut(web_sys::IdbVersionChangeEvent)>);

        open_request.set_onupgradeneeded(Some(closure.as_ref().unchecked_ref()));
        closure.forget();

        let promise = JsFuture::from(open_request.into());
        let result = promise
            .await
            .map_err(|_| SecureStorageError::unsupported("Database open failed"))?;

        let db: IdbDatabase = result
            .dyn_into()
            .map_err(|_| SecureStorageError::unsupported("Invalid database object"))?;

        Ok(db)
    }

    /// Get the SubtleCrypto API.
    fn get_crypto(&self) -> SecureStorageResult<SubtleCrypto> {
        let window: Window = web_sys::window()
            .ok_or_else(|| SecureStorageError::unsupported("No window object"))?;

        let crypto: Crypto = window
            .crypto()
            .map_err(|_| SecureStorageError::unsupported("Crypto API not available"))?;

        Ok(crypto.subtle())
    }

    /// Generate a random master key.
    async fn generate_master_key(&self) -> SecureStorageResult<CryptoKey> {
        let crypto = self.get_crypto()?;

        let algorithm = web_sys::AesKeyGenParams::new("AES-GCM", 256)
            .map_err(|e| SecureStorageError::internal(format!("Failed to create key params: {:?}", e)))?;
        let extractable = true;
        let key_usages = Array::new();
        key_usages.push(&JsValue::from_str("encrypt"));
        key_usages.push(&JsValue::from_str("decrypt"));

        let key_promise = crypto
            .generate_key_with_object(&algorithm, extractable, &key_usages)
            .map_err(|e| SecureStorageError::internal(format!("Key generation failed: {:?}", e)))?;

        let key_result = JsFuture::from(key_promise)
            .await
            .map_err(|e| SecureStorageError::internal(format!("Key generation await failed: {:?}", e)))?;

        key_result
            .dyn_into()
            .map_err(|_| SecureStorageError::internal("Invalid key object"))
    }

    /// Derive encryption key from password using PBKDF2.
    /// 
    /// For Web Crypto PBKDF2, the password is imported as an HMAC key,
    /// then PBKDF2 is used to derive an AES-256-GCM key.
    async fn derive_key_from_password(
        &self,
        password: &[u8],
        salt: &[u8],
    ) -> SecureStorageResult<CryptoKey> {
        let crypto = self.get_crypto()?;

        // Import password as HMAC key material (required for PBKDF2)
        let key_data = Uint8Array::from(password);
        let salt_array = Uint8Array::from(salt);

        // Create HMAC import params - password will be used as HMAC key
        let hmac_params = web_sys::HmacImportParams::new("HMAC", "SHA-256")
            .map_err(|e| SecureStorageError::internal(format!("Failed to create HMAC params: {:?}", e)))?;

        let extractable = false;
        let key_usages = Array::new();
        key_usages.push(&JsValue::from_str("deriveKey"));

        // Import password as HMAC key
        let base_key_promise = crypto
            .import_key_with_object("raw", &key_data.into(), &hmac_params, extractable, &key_usages)
            .map_err(|e| SecureStorageError::internal(format!("Key import failed: {:?}", e)))?;

        let base_key = JsFuture::from(base_key_promise)
            .await
            .map_err(|e| SecureStorageError::internal(format!("Key import await failed: {:?}", e)))?;

        // Create PBKDF2 params
        let pbkdf2_params = web_sys::Pbkdf2Params::new("PBKDF2", &salt_array, 100000)
            .map_err(|e| SecureStorageError::internal(format!("Failed to create PBKDF2 params: {:?}", e)))?;

        // Derive AES-256-GCM key using PBKDF2
        let aes_algorithm = web_sys::AesKeyGenParams::new("AES-GCM", 256)
            .map_err(|e| SecureStorageError::internal(format!("Failed to create AES params: {:?}", e)))?;

        let derived_key_usages = Array::new();
        derived_key_usages.push(&JsValue::from_str("encrypt"));
        derived_key_usages.push(&JsValue::from_str("decrypt"));

        let derive_promise = crypto
            .derive_key_with_object(
                &pbkdf2_params,
                &base_key
                    .dyn_into()
                    .map_err(|_| SecureStorageError::internal("Invalid base key"))?,
                &aes_algorithm,
                extractable,
                &derived_key_usages,
            )
            .map_err(|e| SecureStorageError::internal(format!("Key derivation failed: {:?}", e)))?;

        let derived_key = JsFuture::from(derive_promise)
            .await
            .map_err(|e| SecureStorageError::internal(format!("Key derivation await failed: {:?}", e)))?;

        derived_key
            .dyn_into()
            .map_err(|_| SecureStorageError::internal("Invalid derived key"))
    }

    /// Get or create the master key.
    async fn get_master_key_internal(&self) -> SecureStorageResult<CryptoKey> {
        // Check if already cached
        {
            let master_key_guard = self.master_key.borrow();
            if let Some(ref key) = *master_key_guard {
                // Clone the CryptoKey reference (it's a JS object, so this is safe)
                return Ok(key.clone());
            }
        }

        let db = self.init_db().await?;
        let crypto = self.get_crypto()?;

        // Try to load encrypted master key from IndexedDB
        let transaction = db
            .transaction_with_str_and_mode(&self.metadata_store_name, IdbTransactionMode::Readonly)
            .map_err(|_| SecureStorageError::internal("Failed to create transaction"))?;

        let store = transaction
            .object_store(&self.metadata_store_name)
            .map_err(|_| SecureStorageError::internal("Failed to get object store"))?;

        let request = store
            .get(&JsValue::from_str("master_key"))
            .map_err(|_| SecureStorageError::internal("Failed to get master key"))?;

        let promise = JsFuture::from(request.into());
        let result = promise.await;

        match result {
            Ok(js_value) => {
                if !js_value.is_undefined() && !js_value.is_null() {
                    // Master key exists, decrypt it
                    let password_guard = self.password.borrow();
                    let password = password_guard
                        .as_ref()
                        .ok_or_else(|| SecureStorageError::internal("Password required to unlock storage"))?;

                    let master_key_data: MasterKeyData = serde_wasm_bindgen::from_value(js_value)
                        .map_err(|e| SecureStorageError::internal(format!("Deserialization failed: {}", e)))?;

                    drop(password_guard);

                    let password_key = self
                        .derive_key_from_password(password, &master_key_data.salt)
                        .await?;

                    // Decrypt master key
                    if master_key_data.encrypted_master_key.len() < 12 {
                        return Err(SecureStorageError::internal("Invalid encrypted master key"));
                    }

                    let iv = Uint8Array::new_with_length(12);
                    iv.copy_from(&master_key_data.encrypted_master_key[0..12]);

                    let ciphertext_len = master_key_data.encrypted_master_key.len() - 12;
                    let ciphertext = Uint8Array::new_with_length(ciphertext_len);
                    ciphertext.copy_from(&master_key_data.encrypted_master_key[12..]);

                    let algorithm = web_sys::AesGcmParams::new("AES-GCM", &iv)
                        .map_err(|e| SecureStorageError::internal(format!("Failed to create GCM params: {:?}", e)))?;

                    let decrypt_promise = crypto
                        .decrypt_with_object_and_buffer_source(&algorithm, &password_key, &ciphertext.into())
                        .map_err(|e| SecureStorageError::internal(format!("Decrypt failed: {:?}", e)))?;

                    let decrypted = JsFuture::from(decrypt_promise)
                        .await
                        .map_err(|_| SecureStorageError::internal("Decryption failed - wrong password"))?;

                    // Import decrypted master key
                    let key_data = Uint8Array::from(
                        decrypted
                            .dyn_ref::<js_sys::ArrayBuffer>()
                            .ok_or_else(|| SecureStorageError::internal("Invalid decrypted data"))?,
                    );

                    let import_algorithm = web_sys::AesKeyAlgorithm::new("AES-GCM")
                        .map_err(|e| SecureStorageError::internal(format!("Failed to create import params: {:?}", e)))?;

                    let extractable = true;
                    let key_usages = Array::new();
                    key_usages.push(&JsValue::from_str("encrypt"));
                    key_usages.push(&JsValue::from_str("decrypt"));

                    let import_promise = crypto
                        .import_key_with_object("raw", &key_data.into(), &import_algorithm, extractable, &key_usages)
                        .map_err(|e| SecureStorageError::internal(format!("Import failed: {:?}", e)))?;

                    let master_key = JsFuture::from(import_promise)
                        .await
                        .map_err(|_| SecureStorageError::internal("Failed to import master key"))?
                        .dyn_into()
                        .map_err(|_| SecureStorageError::internal("Invalid master key"))?;

                    // Cache the master key
                    *self.master_key.borrow_mut() = Some(master_key.clone());
                    return Ok(master_key);
                }
            }
            Err(_) => {
                // Master key doesn't exist, will create new one
            }
        }

        // Generate new master key
        let master_key = self.generate_master_key().await?;

        // If password provided, encrypt and store master key
        let password_guard = self.password.borrow();
        if let Some(ref pwd) = *password_guard {
            let crypto_for_salt = self.get_crypto()?;
            let mut salt_bytes = vec![0u8; 16];
            let salt_array = Uint8Array::from(salt_bytes.as_mut_slice());
            crypto_for_salt
                .get_random_values_with_u8_array(&mut salt_bytes)
                .map_err(|_| SecureStorageError::internal("Failed to generate salt"))?;
            let salt_vec = salt_bytes;

            let password_key = self.derive_key_from_password(pwd, &salt_vec).await?;

            // Export master key
            let export_promise = crypto
                .export_key("raw", &master_key)
                .map_err(|e| SecureStorageError::internal(format!("Export failed: {:?}", e)))?;

            let exported = JsFuture::from(export_promise)
                .await
                .map_err(|_| SecureStorageError::internal("Failed to export master key"))?;

            let key_data = Uint8Array::from(
                exported
                    .dyn_ref::<js_sys::ArrayBuffer>()
                    .ok_or_else(|| SecureStorageError::internal("Invalid exported key"))?,
            );

            // Encrypt master key
            let mut iv_bytes = vec![0u8; 12];
            let iv_array = Uint8Array::from(iv_bytes.as_mut_slice());
            crypto_for_salt
                .get_random_values_with_u8_array(&mut iv_bytes)
                .map_err(|_| SecureStorageError::internal("Failed to generate IV"))?;
            let iv_vec = iv_bytes.clone();
            let iv_array_final = Uint8Array::from(iv_vec.as_slice());

            let algorithm = web_sys::AesGcmParams::new("AES-GCM", &iv_array_final)
                .map_err(|e| SecureStorageError::internal(format!("Failed to create GCM params: {:?}", e)))?;

            let encrypt_promise = crypto
                .encrypt_with_object_and_buffer_source(&algorithm, &password_key, &key_data.into())
                .map_err(|e| SecureStorageError::internal(format!("Encrypt failed: {:?}", e)))?;

            let encrypted = JsFuture::from(encrypt_promise)
                .await
                .map_err(|_| SecureStorageError::internal("Failed to encrypt master key"))?;

            let encrypted_data = Uint8Array::from(
                encrypted
                    .dyn_ref::<js_sys::ArrayBuffer>()
                    .ok_or_else(|| SecureStorageError::internal("Invalid encrypted data"))?,
            );

            let mut encrypted_vec = iv_vec;
            encrypted_vec.extend_from_slice(&encrypted_data.to_vec());

            // Store encrypted master key
            let master_key_data = MasterKeyData {
                encrypted_master_key: encrypted_vec,
                salt: salt_vec,
            };

            let transaction = db
                .transaction_with_str_and_mode(&self.metadata_store_name, IdbTransactionMode::Readwrite)
                .map_err(|_| SecureStorageError::internal("Failed to create transaction"))?;

            let store = transaction
                .object_store(&self.metadata_store_name)
                .map_err(|_| SecureStorageError::internal("Failed to get object store"))?;

            let value = serde_wasm_bindgen::to_value(&master_key_data)
                .map_err(|e| SecureStorageError::internal(format!("Serialization failed: {}", e)))?;

            store
                .put_with_key(&value, &JsValue::from_str("master_key"))
                .map_err(|_| SecureStorageError::internal("Failed to store master key"))?;

            let promise = JsFuture::from(transaction.into());
            promise
                .await
                .map_err(|_| SecureStorageError::internal("Transaction failed"))?;
        }
        drop(password_guard);

        // Cache the master key
        *self.master_key.borrow_mut() = Some(master_key.clone());
        Ok(master_key)
    }

    /// Encrypt data using the master key.
    async fn encrypt_data(&self, data: &[u8]) -> SecureStorageResult<Vec<u8>> {
        let master_key = self.get_master_key_internal().await?;
        let crypto = self.get_crypto()?;

        // Generate IV
        let mut iv_bytes = vec![0u8; 12];
        let iv_array_temp = Uint8Array::from(iv_bytes.as_mut_slice());
        crypto
            .get_random_values_with_u8_array(&mut iv_bytes)
            .map_err(|_| SecureStorageError::internal("Failed to generate IV"))?;
        let iv_vec = iv_bytes.clone();
        let iv_array = Uint8Array::from(iv_vec.as_slice());

        // Encrypt
        let algorithm = web_sys::AesGcmParams::new("AES-GCM", &iv_array)
            .map_err(|e| SecureStorageError::internal(format!("Failed to create GCM params: {:?}", e)))?;

        let data_array = Uint8Array::from(data);
        let encrypt_promise = crypto
            .encrypt_with_object_and_buffer_source(&algorithm, &master_key, &data_array.into())
            .map_err(|e| SecureStorageError::internal(format!("Encrypt failed: {:?}", e)))?;

        let encrypted = JsFuture::from(encrypt_promise)
            .await
            .map_err(|_| SecureStorageError::internal("Encryption failed"))?;

        let encrypted_data = Uint8Array::from(
            encrypted
                .dyn_ref::<js_sys::ArrayBuffer>()
                .ok_or_else(|| SecureStorageError::internal("Invalid encrypted data"))?,
        );

        // Return IV + ciphertext
        let mut result = iv_vec;
        result.extend_from_slice(&encrypted_data.to_vec());
        Ok(result)
    }

    /// Decrypt data using the master key.
    async fn decrypt_data(&self, encrypted: &[u8]) -> SecureStorageResult<Vec<u8>> {
        if encrypted.len() < 12 {
            return Err(SecureStorageError::internal("Invalid encrypted data length"));
        }

        let master_key = self.get_master_key_internal().await?;
        let crypto = self.get_crypto()?;

        // Extract IV and ciphertext
        let iv = Uint8Array::new_with_length(12);
        iv.copy_from(&encrypted[0..12]);

        let ciphertext = Uint8Array::new_with_length(encrypted.len() - 12);
        ciphertext.copy_from(&encrypted[12..]);

        // Decrypt
        let algorithm = web_sys::AesGcmParams::new("AES-GCM", &iv)
            .map_err(|e| SecureStorageError::internal(format!("Failed to create GCM params: {:?}", e)))?;

        let decrypt_promise = crypto
            .decrypt_with_object_and_buffer_source(&algorithm, &master_key, &ciphertext.into())
            .map_err(|e| SecureStorageError::internal(format!("Decrypt failed: {:?}", e)))?;

        let decrypted = JsFuture::from(decrypt_promise)
            .await
            .map_err(|_| SecureStorageError::internal("Decryption failed"))?;

        Ok(Uint8Array::from(
            decrypted
                .dyn_ref::<js_sys::ArrayBuffer>()
                .ok_or_else(|| SecureStorageError::internal("Invalid decrypted data"))?,
        )
        .to_vec())
    }
}

#[cfg(target_arch = "wasm32")]
impl SecureKeyStorage for WebCryptoStorage {
    async fn store(
        &self,
        key_id: &str,
        key_data: &[u8],
        options: StoreOptions,
    ) -> SecureStorageResult<()> {
        if !options.overwrite {
            if self.exists(key_id).await? {
                return Err(SecureStorageError::already_exists(key_id));
            }
        }

        // Encrypt the key data
        let encrypted = self.encrypt_data(key_data).await?;

        // Store in IndexedDB
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
            .map_err(|_| SecureStorageError::internal("Failed to create transaction"))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|_| SecureStorageError::internal("Failed to get object store"))?;

        let stored_key = StoredKey {
            encrypted_data: encrypted,
            created_at: js_sys::Date::now() as i64 / 1000,
            last_used_at: None,
        };

        let value = serde_wasm_bindgen::to_value(&stored_key)
            .map_err(|e| SecureStorageError::internal(format!("Serialization failed: {}", e)))?;

        store
            .put_with_key(&value, &JsValue::from_str(key_id))
            .map_err(|_| SecureStorageError::internal("Failed to store key"))?;

        let promise = JsFuture::from(transaction.into());
        promise
            .await
            .map_err(|_| SecureStorageError::internal("Transaction failed"))?;

        Ok(())
    }

    async fn retrieve(&self, key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readonly)
            .map_err(|_| SecureStorageError::internal("Failed to create transaction"))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|_| SecureStorageError::internal("Failed to get object store"))?;

        let request = store
            .get(&JsValue::from_str(key_id))
            .map_err(|_| SecureStorageError::internal("Failed to get key"))?;

        let promise = JsFuture::from(request.into());
        let result = promise.await;

        match result {
            Ok(js_value) => {
                if js_value.is_undefined() || js_value.is_null() {
                    return Ok(None);
                }

                let stored_key: StoredKey = serde_wasm_bindgen::from_value(js_value)
                    .map_err(|e| SecureStorageError::internal(format!("Deserialization failed: {}", e)))?;

                // Update last_used_at
                let mut updated_key = stored_key.clone();
                updated_key.last_used_at = Some(js_sys::Date::now() as i64 / 1000);

                // Save updated metadata
                let update_transaction = db
                    .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
                    .map_err(|_| SecureStorageError::internal("Failed to create transaction"))?;

                let update_store = update_transaction
                    .object_store(&self.store_name)
                    .map_err(|_| SecureStorageError::internal("Failed to get object store"))?;

                let update_value = serde_wasm_bindgen::to_value(&updated_key)
                    .map_err(|e| SecureStorageError::internal(format!("Serialization failed: {}", e)))?;

                update_store
                    .put_with_key(&update_value, &JsValue::from_str(key_id))
                    .map_err(|_| SecureStorageError::internal("Failed to update key"))?;

                let update_promise = JsFuture::from(update_transaction.into());
                update_promise
                    .await
                    .map_err(|_| SecureStorageError::internal("Transaction failed"))?;

                // Decrypt and return
                let decrypted = self.decrypt_data(&stored_key.encrypted_data).await?;
                Ok(Some(decrypted))
            }
            Err(_) => Ok(None),
        }
    }

    async fn delete(&self, key_id: &str) -> SecureStorageResult<()> {
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
            .map_err(|_| SecureStorageError::internal("Failed to create transaction"))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|_| SecureStorageError::internal("Failed to get object store"))?;

        store
            .delete(&JsValue::from_str(key_id))
            .map_err(|_| SecureStorageError::internal("Failed to delete key"))?;

        let promise = JsFuture::from(transaction.into());
        promise
            .await
            .map_err(|_| SecureStorageError::internal("Transaction failed"))?;

        Ok(())
    }

    async fn exists(&self, key_id: &str) -> SecureStorageResult<bool> {
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readonly)
            .map_err(|_| SecureStorageError::internal("Failed to create transaction"))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|_| SecureStorageError::internal("Failed to get object store"))?;

        let request = store
            .get(&JsValue::from_str(key_id))
            .map_err(|_| SecureStorageError::internal("Failed to check key"))?;

        let promise = JsFuture::from(request.into());
        let result = promise.await;

        match result {
            Ok(js_value) => Ok(!js_value.is_undefined() && !js_value.is_null()),
            Err(_) => Ok(false),
        }
    }

    async fn get_metadata(&self, key_id: &str) -> SecureStorageResult<Option<KeyMetadata>> {
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readonly)
            .map_err(|_| SecureStorageError::internal("Failed to create transaction"))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|_| SecureStorageError::internal("Failed to get object store"))?;

        let request = store
            .get(&JsValue::from_str(key_id))
            .map_err(|_| SecureStorageError::internal("Failed to get key"))?;

        let promise = JsFuture::from(request.into());
        let result = promise.await;

        match result {
            Ok(js_value) => {
                if js_value.is_undefined() || js_value.is_null() {
                    return Ok(None);
                }

                let stored_key: StoredKey = serde_wasm_bindgen::from_value(js_value)
                    .map_err(|e| SecureStorageError::internal(format!("Deserialization failed: {}", e)))?;

                Ok(Some(KeyMetadata {
                    created_at: stored_key.created_at,
                    last_used_at: stored_key.last_used_at,
                }))
            }
            Err(_) => Ok(None),
        }
    }

    async fn list_keys(&self) -> SecureStorageResult<Vec<String>> {
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readonly)
            .map_err(|_| SecureStorageError::internal("Failed to create transaction"))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|_| SecureStorageError::internal("Failed to get object store"))?;

        // Use getAllKeys to get all key IDs
        let request = store
            .get_all_keys()
            .map_err(|_| SecureStorageError::internal("Failed to list keys"))?;

        let promise = JsFuture::from(request.into());
        let result = promise
            .await
            .map_err(|_| SecureStorageError::internal("List operation failed"))?;

        let array = result
            .dyn_into::<js_sys::Array>()
            .map_err(|_| SecureStorageError::internal("Invalid result array"))?;

        let mut keys = Vec::new();
        for i in 0..array.length() {
            let key_js = array.get(i);
            if let Some(key) = key_js.as_string() {
                keys.push(key);
            } else if key_js.is_number() {
                // IndexedDB can use numeric keys
                keys.push(key_js.as_f64().unwrap().to_string());
            }
        }

        Ok(keys)
    }

    async fn clear_all(&self) -> SecureStorageResult<()> {
        let db = self.init_db().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
            .map_err(|_| SecureStorageError::internal("Failed to create transaction"))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|_| SecureStorageError::internal("Failed to get object store"))?;

        let request = store
            .clear()
            .map_err(|_| SecureStorageError::internal("Failed to clear store"))?;

        let promise = JsFuture::from(request.into());
        promise
            .await
            .map_err(|_| SecureStorageError::internal("Clear operation failed"))?;

        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
// Stub implementation for non-WASM targets
pub struct WebCryptoStorage {
    db_name: String,
    store_name: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl WebCryptoStorage {
    pub fn new(db_name: impl Into<String>) -> Self {
        Self {
            db_name: db_name.into(),
            store_name: "paykit_keys".to_string(),
        }
    }

    pub fn with_store_name(mut self, name: impl Into<String>) -> Self {
        self.store_name = name.into();
        self
    }

    pub fn db_name(&self) -> &str {
        &self.db_name
    }

    pub fn store_name(&self) -> &str {
        &self.store_name
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SecureKeyStorage for WebCryptoStorage {
    async fn store(
        &self,
        _key_id: &str,
        _key_data: &[u8],
        _options: StoreOptions,
    ) -> SecureStorageResult<()> {
        Err(SecureStorageError::unsupported(
            "WebCryptoStorage only available on wasm32 target",
        ))
    }

    async fn retrieve(&self, _key_id: &str) -> SecureStorageResult<Option<Vec<u8>>> {
        Err(SecureStorageError::unsupported(
            "WebCryptoStorage only available on wasm32 target",
        ))
    }

    async fn delete(&self, _key_id: &str) -> SecureStorageResult<()> {
        Err(SecureStorageError::unsupported(
            "WebCryptoStorage only available on wasm32 target",
        ))
    }

    async fn exists(&self, _key_id: &str) -> SecureStorageResult<bool> {
        Err(SecureStorageError::unsupported(
            "WebCryptoStorage only available on wasm32 target",
        ))
    }

    async fn get_metadata(&self, _key_id: &str) -> SecureStorageResult<Option<KeyMetadata>> {
        Err(SecureStorageError::unsupported(
            "WebCryptoStorage only available on wasm32 target",
        ))
    }

    async fn list_keys(&self) -> SecureStorageResult<Vec<String>> {
        Err(SecureStorageError::unsupported(
            "WebCryptoStorage only available on wasm32 target",
        ))
    }

    async fn clear_all(&self) -> SecureStorageResult<()> {
        Err(SecureStorageError::unsupported(
            "WebCryptoStorage only available on wasm32 target",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_crypto_storage_creation() {
        let storage = WebCryptoStorage::new("my-app-db").with_store_name("secrets");

        assert_eq!(storage.db_name(), "my-app-db");
        assert_eq!(storage.store_name(), "secrets");
    }
}
