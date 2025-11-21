//! File-based storage for demo data (contacts, receipts, subscriptions, etc.)
//!
//! This module provides a simple JSON-based file storage system for demo applications.
//!
//! # Security Warning
//!
//! This storage is **NOT suitable for production use**:
//! - No encryption at rest
//! - No atomicity guarantees
//! - No concurrent access protection
//! - No backup/recovery mechanisms
//!
//! For production, use a proper database with encryption, transactions, and access control.
//!
//! # Examples
//!
//! ```no_run
//! use paykit_demo_core::{DemoStorage, Contact};
//! use pubky::Keypair;
//!
//! # fn example() -> anyhow::Result<()> {
//! let storage = DemoStorage::new("./data");
//! storage.init()?;
//!
//! // Save a contact
//! let keypair = Keypair::random();
//! let contact = Contact::new(keypair.public_key(), "Alice".to_string());
//! storage.save_contact(contact)?;
//!
//! // List contacts
//! let contacts = storage.list_contacts()?;
//! println!("Found {} contacts", contacts.len());
//! # Ok(())
//! # }
//! ```

use crate::models::{Contact, Receipt};
use anyhow::{Context, Result};
use paykit_subscriptions::{AutoPayRule, PaymentRequest, Subscription};
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
    subscriptions: HashMap<String, Subscription>,
    payment_requests: HashMap<String, PaymentRequest>,
    auto_pay_rules: HashMap<String, AutoPayRule>,
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

    /// Save a subscription
    pub fn save_subscription(&self, subscription: Subscription) -> Result<()> {
        let mut data = self.load_data()?;
        let key = subscription.subscription_id.clone();
        data.subscriptions.insert(key, subscription);
        self.save_data(&data)?;
        Ok(())
    }

    /// Get a subscription by ID
    pub fn get_subscription(&self, id: &str) -> Result<Option<Subscription>> {
        let data = self.load_data()?;
        Ok(data.subscriptions.get(id).cloned())
    }

    /// List all subscriptions
    pub fn list_subscriptions(&self) -> Result<Vec<Subscription>> {
        let data = self.load_data()?;
        Ok(data.subscriptions.values().cloned().collect())
    }

    /// Save a payment request
    pub fn save_payment_request(&self, request: PaymentRequest) -> Result<()> {
        let mut data = self.load_data()?;
        let key = request.request_id.clone();
        data.payment_requests.insert(key, request);
        self.save_data(&data)?;
        Ok(())
    }

    /// Get a payment request by ID
    pub fn get_payment_request(&self, id: &str) -> Result<Option<PaymentRequest>> {
        let data = self.load_data()?;
        Ok(data.payment_requests.get(id).cloned())
    }

    /// List all payment requests
    pub fn list_payment_requests(&self) -> Result<Vec<PaymentRequest>> {
        let data = self.load_data()?;
        Ok(data.payment_requests.values().cloned().collect())
    }

    /// Save an auto-pay rule
    pub fn save_auto_pay_rule(&self, rule: AutoPayRule) -> Result<()> {
        let mut data = self.load_data()?;
        let key = rule.subscription_id.clone();
        data.auto_pay_rules.insert(key, rule);
        self.save_data(&data)?;
        Ok(())
    }

    /// Get an auto-pay rule by subscription ID
    pub fn get_auto_pay_rule(&self, subscription_id: &str) -> Result<Option<AutoPayRule>> {
        let data = self.load_data()?;
        Ok(data.auto_pay_rules.get(subscription_id).cloned())
    }

    /// List all auto-pay rules
    pub fn list_auto_pay_rules(&self) -> Result<Vec<AutoPayRule>> {
        let data = self.load_data()?;
        Ok(data.auto_pay_rules.values().cloned().collect())
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
