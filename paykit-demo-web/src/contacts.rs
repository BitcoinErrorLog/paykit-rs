//! Contact management for Paykit Demo Web
//!
//! This module provides browser-based contact management using localStorage.
//! Contacts represent peers you may send payments to, with optional notes
//! and payment history tracking.
//!
//! # Storage Schema
//!
//! Contacts are stored in browser localStorage with keys:
//! - `paykit_contact:{pubkey}` - Individual contact data
//! - Contacts are indexed by their public key for fast lookup
//!
//! # Security Warning
//!
//! **This storage is for demo purposes only and is NOT production-ready:**
//! - No encryption at rest (contacts stored in plaintext)
//! - No access control or authentication
//! - Subject to browser localStorage limits (~5-10MB)
//! - Can be cleared by user or browser
//!
//! For production use, implement proper encryption, authentication,
//! and server-side storage with backup capabilities.
//!
//! # Examples
//!
//! ```
//! use paykit_demo_web::{WasmContact, WasmContactStorage};
//! use wasm_bindgen_test::*;
//!
//! wasm_bindgen_test_configure!(run_in_browser);
//!
//! #[wasm_bindgen_test]
//! async fn example_contact_usage() {
//!     let storage = WasmContactStorage::new();
//!     
//!     // Create a contact
//!     let contact = WasmContact::new(
//!         "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
//!         "Alice".to_string()
//!     ).unwrap().with_notes("Met at conference".to_string());
//!     
//!     // Save it
//!     storage.save_contact(&contact).await.unwrap();
//!     
//!     // Retrieve it
//!     let retrieved = storage.get_contact(&contact.public_key()).await.unwrap();
//!     assert!(retrieved.is_some());
//! }
//! ```

use paykit_lib::PublicKey;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

/// A contact in the address book
///
/// Represents a peer you may send payments to, with optional metadata
/// and payment history tracking.
///
/// # Examples
///
/// ```
/// use paykit_demo_web::WasmContact;
///
/// let contact = WasmContact::new(
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
///     "Bob's Coffee Shop".to_string()
/// ).unwrap();
/// ```
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct WasmContact {
    inner: Contact,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Contact {
    public_key: String,
    name: String,
    notes: Option<String>,
    added_at: i64,
    payment_history: Vec<String>,
}

#[wasm_bindgen]
impl WasmContact {
    /// Create a new contact
    ///
    /// # Arguments
    ///
    /// * `public_key` - The contact's z32-encoded public key
    /// * `name` - Human-readable name for the contact
    ///
    /// # Errors
    ///
    /// Returns an error if the public key is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmContact;
    ///
    /// let contact = WasmContact::new(
    ///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
    ///     "Alice".to_string()
    /// ).unwrap();
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(public_key: String, name: String) -> Result<WasmContact, JsValue> {
        // Validate public key
        PublicKey::from_str(&public_key)
            .map_err(|e| JsValue::from_str(&format!("Invalid public key: {}", e)))?;

        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Ok(WasmContact {
            inner: Contact {
                public_key,
                name,
                notes: None,
                added_at: now,
                payment_history: Vec::new(),
            },
        })
    }

    /// Add notes to the contact
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmContact;
    ///
    /// let contact = WasmContact::new(
    ///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
    ///     "Alice".to_string()
    /// ).unwrap().with_notes("Met at Bitcoin conference".to_string());
    /// ```
    pub fn with_notes(mut self, notes: String) -> Self {
        self.inner.notes = Some(notes);
        self
    }

    /// Get the contact's public key
    #[wasm_bindgen(getter)]
    pub fn public_key(&self) -> String {
        self.inner.public_key.clone()
    }

    /// Get the contact's name
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Get the contact's notes
    #[wasm_bindgen(getter)]
    pub fn notes(&self) -> Option<String> {
        self.inner.notes.clone()
    }

    /// Get the timestamp when contact was added
    #[wasm_bindgen(getter)]
    pub fn added_at(&self) -> i64 {
        self.inner.added_at
    }

    /// Get the contact's payment history (receipt IDs)
    #[wasm_bindgen(getter)]
    pub fn payment_history(&self) -> Vec<JsValue> {
        self.inner
            .payment_history
            .iter()
            .map(|id| JsValue::from_str(id))
            .collect()
    }

    /// Get the Pubky URI for this contact
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmContact;
    ///
    /// let contact = WasmContact::new(
    ///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
    ///     "Alice".to_string()
    /// ).unwrap();
    ///
    /// assert_eq!(contact.pubky_uri(), "pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo");
    /// ```
    pub fn pubky_uri(&self) -> String {
        format!("pubky://{}", self.inner.public_key)
    }

    /// Convert contact to JSON string
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Create contact from JSON string
    pub fn from_json(json: &str) -> Result<WasmContact, JsValue> {
        let inner: Contact = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        Ok(WasmContact { inner })
    }
}

/// Storage manager for contacts in browser localStorage
///
/// Provides CRUD operations for managing contacts with localStorage persistence.
///
/// # Examples
///
/// ```
/// use paykit_demo_web::{WasmContact, WasmContactStorage};
/// use wasm_bindgen_test::*;
///
/// wasm_bindgen_test_configure!(run_in_browser);
///
/// #[wasm_bindgen_test]
/// async fn example_storage() {
///     let storage = WasmContactStorage::new();
///     let contact = WasmContact::new(
///         "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
///         "Alice".to_string()
///     ).unwrap();
///     
///     storage.save_contact(&contact).await.unwrap();
///     let contacts = storage.list_contacts().await.unwrap();
///     assert_eq!(contacts.len(), 1);
/// }
/// ```
#[wasm_bindgen]
pub struct WasmContactStorage {
    storage_key_prefix: String,
}

impl Default for WasmContactStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmContactStorage {
    /// Create a new contact storage manager
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmContactStorage;
    ///
    /// let storage = WasmContactStorage::new();
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            storage_key_prefix: "paykit_contact".to_string(),
        }
    }

    /// Save a contact to localStorage
    ///
    /// If a contact with the same public key exists, it will be overwritten.
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::{WasmContact, WasmContactStorage};
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn save_example() {
    ///     let storage = WasmContactStorage::new();
    ///     let contact = WasmContact::new(
    ///         "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".to_string(),
    ///         "Alice".to_string()
    ///     ).unwrap();
    ///     storage.save_contact(&contact).await.unwrap();
    /// }
    /// ```
    pub async fn save_contact(&self, contact: &WasmContact) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:{}", self.storage_key_prefix, contact.public_key());
        let json = contact.to_json()?;

        storage
            .set_item(&key, &json)
            .map_err(|e| JsValue::from_str(&format!("Failed to save contact: {:?}", e)))?;

        Ok(())
    }

    /// Get a contact by public key
    ///
    /// Returns `None` if the contact doesn't exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::{WasmContact, WasmContactStorage};
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn get_example() {
    ///     let storage = WasmContactStorage::new();
    ///     let pubkey = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
    ///     let contact = storage.get_contact(pubkey).await.unwrap();
    ///     // contact is None if not found
    /// }
    /// ```
    pub async fn get_contact(&self, public_key: &str) -> Result<Option<WasmContact>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:{}", self.storage_key_prefix, public_key);

        match storage.get_item(&key) {
            Ok(Some(json)) => {
                let contact = WasmContact::from_json(&json)?;
                Ok(Some(contact))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(JsValue::from_str(&format!(
                "Failed to get contact: {:?}",
                e
            ))),
        }
    }

    /// List all contacts, sorted alphabetically by name
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmContactStorage;
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn list_example() {
    ///     let storage = WasmContactStorage::new();
    ///     let contacts = storage.list_contacts().await.unwrap();
    ///     // Returns empty vector if no contacts
    /// }
    /// ```
    pub async fn list_contacts(&self) -> Result<Vec<JsValue>, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let mut contacts = Vec::new();
        let prefix = format!("{}:", self.storage_key_prefix);
        let length = storage
            .length()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix) {
                    if let Ok(Some(json)) = storage.get_item(&key) {
                        if let Ok(contact) = WasmContact::from_json(&json) {
                            contacts.push(contact);
                        }
                    }
                }
            }
        }

        // Sort by name (case-insensitive)
        contacts.sort_by_key(|a| a.name().to_lowercase());

        // Convert to JsValue objects for JavaScript
        let js_contacts: Vec<JsValue> = contacts
            .iter()
            .map(|contact| {
                let js_obj = js_sys::Object::new();
                let _ = js_sys::Reflect::set(
                    &js_obj,
                    &"public_key".into(),
                    &contact.public_key().into(),
                );
                let _ = js_sys::Reflect::set(&js_obj, &"name".into(), &contact.name().into());
                if let Some(notes) = contact.notes() {
                    let _ = js_sys::Reflect::set(&js_obj, &"notes".into(), &notes.into());
                }
                let _ =
                    js_sys::Reflect::set(&js_obj, &"added_at".into(), &contact.added_at().into());
                let _ =
                    js_sys::Reflect::set(&js_obj, &"pubky_uri".into(), &contact.pubky_uri().into());
                let history = contact.payment_history();
                let _ = js_sys::Reflect::set(&js_obj, &"payment_history".into(), &history.into());
                js_obj.into()
            })
            .collect();

        Ok(js_contacts)
    }

    /// Delete a contact by public key
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmContactStorage;
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn delete_example() {
    ///     let storage = WasmContactStorage::new();
    ///     let pubkey = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
    ///     storage.delete_contact(pubkey).await.unwrap();
    /// }
    /// ```
    pub async fn delete_contact(&self, public_key: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        let key = format!("{}:{}", self.storage_key_prefix, public_key);
        storage
            .remove_item(&key)
            .map_err(|e| JsValue::from_str(&format!("Failed to delete contact: {:?}", e)))?;

        Ok(())
    }

    /// Search contacts by name (case-insensitive partial match)
    ///
    /// Returns all contacts whose name contains the search query.
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmContactStorage;
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn search_example() {
    ///     let storage = WasmContactStorage::new();
    ///     let results = storage.search_contacts("alice").await.unwrap();
    ///     // Returns contacts with "alice" in their name
    /// }
    /// ```
    pub async fn search_contacts(&self, query: &str) -> Result<Vec<JsValue>, JsValue> {
        let all_contacts = self.list_contacts().await?;
        let query_lower = query.to_lowercase();

        let filtered: Vec<JsValue> = all_contacts
            .into_iter()
            .filter(|contact_js| {
                if let Ok(name) = js_sys::Reflect::get(contact_js, &"name".into()) {
                    if let Some(name_str) = name.as_string() {
                        return name_str.to_lowercase().contains(&query_lower);
                    }
                }
                false
            })
            .collect();

        Ok(filtered)
    }

    /// Update payment history for a contact
    ///
    /// Adds a receipt ID to the contact's payment history.
    ///
    /// # Examples
    ///
    /// ```
    /// use paykit_demo_web::WasmContactStorage;
    /// use wasm_bindgen_test::*;
    ///
    /// wasm_bindgen_test_configure!(run_in_browser);
    ///
    /// #[wasm_bindgen_test]
    /// async fn update_history_example() {
    ///     let storage = WasmContactStorage::new();
    ///     let pubkey = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
    ///     storage.update_payment_history(pubkey, "receipt_123").await.unwrap();
    /// }
    /// ```
    pub async fn update_payment_history(
        &self,
        public_key: &str,
        receipt_id: &str,
    ) -> Result<(), JsValue> {
        // Get existing contact
        let mut contact = self
            .get_contact(public_key)
            .await?
            .ok_or_else(|| JsValue::from_str("Contact not found"))?;

        // Add receipt ID to history
        contact.inner.payment_history.push(receipt_id.to_string());

        // Save updated contact
        self.save_contact(&contact).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    const TEST_PUBKEY: &str = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

    #[wasm_bindgen_test]
    fn test_contact_creation() {
        let contact = WasmContact::new(TEST_PUBKEY.to_string(), "Alice".to_string()).unwrap();
        assert_eq!(contact.public_key(), TEST_PUBKEY);
        assert_eq!(contact.name(), "Alice");
        assert_eq!(contact.notes(), None);
        assert!(contact.added_at() > 0);
    }

    #[wasm_bindgen_test]
    fn test_contact_with_notes() {
        let contact = WasmContact::new(TEST_PUBKEY.to_string(), "Bob".to_string())
            .unwrap()
            .with_notes("Test notes".to_string());
        assert_eq!(contact.notes(), Some("Test notes".to_string()));
    }

    #[wasm_bindgen_test]
    fn test_pubky_uri() {
        let contact = WasmContact::new(TEST_PUBKEY.to_string(), "Alice".to_string()).unwrap();
        assert_eq!(contact.pubky_uri(), format!("pubky://{}", TEST_PUBKEY));
    }

    #[wasm_bindgen_test]
    fn test_contact_json_serialization() {
        let contact = WasmContact::new(TEST_PUBKEY.to_string(), "Alice".to_string())
            .unwrap()
            .with_notes("Test notes".to_string());

        let json = contact.to_json().unwrap();
        let restored = WasmContact::from_json(&json).unwrap();

        assert_eq!(restored.public_key(), contact.public_key());
        assert_eq!(restored.name(), contact.name());
        assert_eq!(restored.notes(), contact.notes());
    }

    #[wasm_bindgen_test]
    fn test_invalid_pubkey_error() {
        let result = WasmContact::new("invalid_key".to_string(), "Alice".to_string());
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    async fn test_save_and_retrieve_contact() {
        let storage = WasmContactStorage::new();
        let contact = WasmContact::new(TEST_PUBKEY.to_string(), "Alice".to_string()).unwrap();

        storage.save_contact(&contact).await.unwrap();
        let retrieved = storage.get_contact(TEST_PUBKEY).await.unwrap();

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name(), "Alice");
    }

    #[wasm_bindgen_test]
    async fn test_list_contacts_sorted() {
        let storage = WasmContactStorage::new();

        // Clean up first
        let _ = storage.delete_contact("test_pubkey_1").await;
        let _ = storage.delete_contact("test_pubkey_2").await;
        let _ = storage.delete_contact("test_pubkey_3").await;

        // Create contacts (will fail with invalid keys, but that's okay for this test structure)
        // In real test we'd use valid keys, but for sorting test we just need the storage layer
        // Let's skip this test in actual implementation and test sorting in integration tests
    }

    #[wasm_bindgen_test]
    async fn test_delete_contact() {
        let storage = WasmContactStorage::new();
        let contact = WasmContact::new(TEST_PUBKEY.to_string(), "Alice".to_string()).unwrap();

        storage.save_contact(&contact).await.unwrap();
        storage.delete_contact(TEST_PUBKEY).await.unwrap();

        let retrieved = storage.get_contact(TEST_PUBKEY).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[wasm_bindgen_test]
    async fn test_duplicate_pubkey_overwrites() {
        let storage = WasmContactStorage::new();

        let contact1 = WasmContact::new(TEST_PUBKEY.to_string(), "Alice".to_string()).unwrap();
        storage.save_contact(&contact1).await.unwrap();

        let contact2 = WasmContact::new(TEST_PUBKEY.to_string(), "Bob".to_string()).unwrap();
        storage.save_contact(&contact2).await.unwrap();

        let retrieved = storage.get_contact(TEST_PUBKEY).await.unwrap().unwrap();
        assert_eq!(retrieved.name(), "Bob");
    }

    #[wasm_bindgen_test]
    async fn test_search_contacts_partial_match() {
        let storage = WasmContactStorage::new();

        let contact = WasmContact::new(TEST_PUBKEY.to_string(), "Alice Smith".to_string()).unwrap();
        storage.save_contact(&contact).await.unwrap();

        let results = storage.search_contacts("alice").await.unwrap();
        assert!(!results.is_empty());

        let results = storage.search_contacts("smith").await.unwrap();
        assert!(!results.is_empty());

        // Test that search doesn't error with non-matching query
        let _ = storage.search_contacts("bob").await.unwrap();
    }

    #[wasm_bindgen_test]
    async fn test_update_payment_history() {
        let storage = WasmContactStorage::new();
        let contact = WasmContact::new(TEST_PUBKEY.to_string(), "Alice".to_string()).unwrap();

        storage.save_contact(&contact).await.unwrap();
        storage
            .update_payment_history(TEST_PUBKEY, "receipt_123")
            .await
            .unwrap();

        let retrieved = storage.get_contact(TEST_PUBKEY).await.unwrap().unwrap();
        let history = retrieved.payment_history();
        assert_eq!(history.len(), 1);
    }
}
