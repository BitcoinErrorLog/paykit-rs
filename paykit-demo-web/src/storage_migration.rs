//! Migration from localStorage to WebCryptoStorage

use paykit_lib::secure_storage::{SecureKeyStorage, StoreOptions, WebCryptoStorage};
use wasm_bindgen::prelude::*;

use crate::utils;

/// Migration status
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MigrationStatus {
    /// No migration needed (already using secure storage)
    NotNeeded,
    /// Migration available (localStorage data detected)
    Available,
    /// Migration in progress
    InProgress,
    /// Migration completed
    Completed,
    /// Migration failed
    Failed,
}

/// Migration manager for moving from localStorage to WebCryptoStorage
#[wasm_bindgen]
pub struct StorageMigration {
    status: MigrationStatus,
}

#[wasm_bindgen]
impl StorageMigration {
    /// Create a new migration manager
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            status: MigrationStatus::NotNeeded,
        }
    }

    /// Check if migration is needed
    #[wasm_bindgen(js_name = checkMigrationNeeded)]
    pub fn check_migration_needed(&self) -> Result<bool, JsValue> {
        // Check if localStorage has Paykit data
        let window = web_sys::window().ok_or_else(|| utils::js_error("No window"))?;
        let storage = window
            .local_storage()?
            .ok_or_else(|| utils::js_error("No localStorage"))?;

        let length = storage
            .length()
            .map_err(|_| utils::js_error("Failed to get length"))?;

        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with("paykit_") {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Migrate data from localStorage to WebCryptoStorage
    ///
    /// # Arguments
    /// * `password` - Password to encrypt the secure storage (required for first-time setup)
    ///
    /// # Returns
    /// Number of items migrated
    #[wasm_bindgen(js_name = migrate)]
    pub async fn migrate(&mut self, password: &str) -> Result<u32, JsValue> {
        self.status = MigrationStatus::InProgress;

        // Get localStorage
        let window = web_sys::window().ok_or_else(|| utils::js_error("No window"))?;
        let storage = window
            .local_storage()?
            .ok_or_else(|| utils::js_error("No localStorage"))?;

        // Create WebCryptoStorage
        let secure_storage = WebCryptoStorage::new("paykit_secure");
        secure_storage.set_password(password.as_bytes().to_vec());

        // Collect all Paykit keys from localStorage
        let length = storage
            .length()
            .map_err(|_| utils::js_error("Failed to get length"))?;

        let mut keys_to_migrate = Vec::new();
        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with("paykit_") {
                    if let Ok(Some(value)) = storage.get_item(&key) {
                        keys_to_migrate.push((key, value));
                    }
                }
            }
        }

        if keys_to_migrate.is_empty() {
            self.status = MigrationStatus::NotNeeded;
            return Ok(0);
        }

        // Migrate each key
        let mut migrated_count = 0u32;
        for (key, value) in keys_to_migrate {
            // Store in secure storage
            let key_data = value.as_bytes();
            secure_storage
                .store(
                    &key,
                    key_data,
                    StoreOptions {
                        overwrite: true,
                        require_auth: false,
                        tags: vec![],
                    },
                )
                .await
                .map_err(|e| utils::js_error(&format!("Failed to store {}: {:?}", key, e)))?;

            migrated_count += 1;
        }

        // Clear localStorage after successful migration
        for (key, _) in keys_to_migrate {
            storage
                .remove_item(&key)
                .map_err(|_| utils::js_error(&format!("Failed to remove {}", key)))?;
        }

        // Mark migration as complete in secure storage
        secure_storage
            .store(
                "paykit_migration_complete",
                b"true",
                StoreOptions {
                    overwrite: true,
                    require_auth: false,
                    tags: vec![],
                },
            )
            .await
            .map_err(|e| utils::js_error(&format!("Failed to mark migration complete: {:?}", e)))?;

        self.status = MigrationStatus::Completed;
        Ok(migrated_count)
    }

    /// Get migration status
    #[wasm_bindgen(js_name = getStatus)]
    pub fn get_status(&self) -> String {
        match self.status {
            MigrationStatus::NotNeeded => "not_needed",
            MigrationStatus::Available => "available",
            MigrationStatus::InProgress => "in_progress",
            MigrationStatus::Completed => "completed",
            MigrationStatus::Failed => "failed",
        }
        .to_string()
    }
}

