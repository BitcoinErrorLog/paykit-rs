//! File-based storage for demo data (contacts, receipts, etc.)

use crate::models::{Contact, Receipt};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Simple file-based storage for demo applications
pub struct DemoStorage {
    storage_dir: PathBuf,
}

#[derive(Serialize, Deserialize, Default)]
struct StorageData {
    contacts: HashMap<String, Contact>,
    receipts: HashMap<String, Receipt>,
}

impl DemoStorage {
    /// Create a new storage instance
    pub fn new(storage_dir: impl AsRef<Path>) -> Self {
        Self {
            storage_dir: storage_dir.as_ref().to_path_buf(),
        }
    }

    /// Initialize storage directory
    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.storage_dir).context("Failed to create storage directory")?;
        Ok(())
    }

    /// Add or update a contact
    pub fn save_contact(&self, contact: Contact) -> Result<()> {
        let mut data = self.load_data()?;
        let key = contact.public_key.to_string();
        data.contacts.insert(key, contact);
        self.save_data(&data)?;
        Ok(())
    }

    /// Get a contact by public key
    pub fn get_contact(&self, public_key: &str) -> Result<Option<Contact>> {
        let data = self.load_data()?;
        Ok(data.contacts.get(public_key).cloned())
    }

    /// List all contacts
    pub fn list_contacts(&self) -> Result<Vec<Contact>> {
        let data = self.load_data()?;
        let mut contacts: Vec<_> = data.contacts.values().cloned().collect();
        contacts.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(contacts)
    }

    /// Delete a contact
    pub fn delete_contact(&self, public_key: &str) -> Result<()> {
        let mut data = self.load_data()?;
        data.contacts.remove(public_key);
        self.save_data(&data)?;
        Ok(())
    }

    /// Save a receipt
    pub fn save_receipt(&self, receipt: Receipt) -> Result<()> {
        let mut data = self.load_data()?;
        data.receipts.insert(receipt.id.clone(), receipt);
        self.save_data(&data)?;
        Ok(())
    }

    /// Get a receipt by ID
    pub fn get_receipt(&self, id: &str) -> Result<Option<Receipt>> {
        let data = self.load_data()?;
        Ok(data.receipts.get(id).cloned())
    }

    /// List all receipts
    pub fn list_receipts(&self) -> Result<Vec<Receipt>> {
        let data = self.load_data()?;
        let mut receipts: Vec<_> = data.receipts.values().cloned().collect();
        receipts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(receipts)
    }

    /// Save a receipt as JSON (for interactive protocol receipts)
    pub fn save_receipt_json(&self, id: &str, json: &str) -> Result<()> {
        let receipts_dir = self.storage_dir.join("interactive_receipts");
        std::fs::create_dir_all(&receipts_dir)?;
        let path = receipts_dir.join(format!("{}.json", id));
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Get a receipt JSON by ID
    pub fn get_receipt_json(&self, id: &str) -> Result<Option<String>> {
        let path = self
            .storage_dir
            .join("interactive_receipts")
            .join(format!("{}.json", id));
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&path)?;
        Ok(Some(json))
    }

    /// List all receipt JSONs
    pub fn list_receipt_jsons(&self) -> Result<Vec<String>> {
        let receipts_dir = self.storage_dir.join("interactive_receipts");
        if !receipts_dir.exists() {
            return Ok(vec![]);
        }
        let mut jsons = Vec::new();
        for entry in std::fs::read_dir(&receipts_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    jsons.push(json);
                }
            }
        }
        Ok(jsons)
    }

    fn data_path(&self) -> PathBuf {
        self.storage_dir.join("data.json")
    }

    fn load_data(&self) -> Result<StorageData> {
        let path = self.data_path();
        if !path.exists() {
            return Ok(StorageData::default());
        }

        let json = std::fs::read_to_string(&path)?;
        let data = serde_json::from_str(&json)?;
        Ok(data)
    }

    fn save_data(&self, data: &StorageData) -> Result<()> {
        self.init()?;
        let path = self.data_path();
        let json = serde_json::to_string_pretty(data)?;
        std::fs::write(&path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pubky::Keypair;

    #[test]
    fn test_contact_storage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = DemoStorage::new(temp_dir.path());

        let keypair = Keypair::random();
        let contact = Contact::new(keypair.public_key(), "Alice".to_string());

        storage.save_contact(contact.clone()).unwrap();

        let loaded = storage
            .get_contact(&keypair.public_key().to_string())
            .unwrap()
            .unwrap();
        assert_eq!(loaded.name, "Alice");

        let contacts = storage.list_contacts().unwrap();
        assert_eq!(contacts.len(), 1);
    }
}
