//! Interactive Protocol FFI Wrappers
//!
//! This module provides FFI-safe wrappers for the Paykit interactive payment protocol,
//! enabling mobile applications to perform encrypted payment negotiations.
//!
//! # Overview
//!
//! The interactive protocol allows:
//! - Exchange of private payment endpoints over encrypted channels
//! - Receipt negotiation and confirmation
//! - Payment flow coordination
//!
//! # Design
//!
//! Rather than wrapping the full async trait-based protocol, this module provides:
//! - Message serialization/deserialization helpers
//! - Receipt management
//! - Payment status tracking
//!
//! Mobile apps handle the actual Noise channel communication using pubky-noise-main bindings.

use std::sync::Arc;

use crate::{PaykitMobileError, Result};

// ============================================================================
// Message Types
// ============================================================================

/// FFI-safe Paykit message type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum PaykitMessageType {
    /// Offer a private endpoint.
    OfferPrivateEndpoint,
    /// Request a receipt.
    RequestReceipt,
    /// Confirm a receipt.
    ConfirmReceipt,
    /// Acknowledgment.
    Ack,
    /// Error message.
    Error,
}

/// FFI-safe private endpoint offer.
#[derive(Clone, Debug, uniffi::Record)]
pub struct PrivateEndpointOffer {
    pub method_id: String,
    pub endpoint: String,
}

/// FFI-safe receipt request.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ReceiptRequest {
    pub receipt_id: String,
    pub payer: String,
    pub payee: String,
    pub method_id: String,
    pub amount: Option<String>,
    pub currency: Option<String>,
    pub metadata_json: String,
}

/// FFI-safe error message.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ErrorMessage {
    pub code: String,
    pub message: String,
}

/// Parsed Paykit message.
#[derive(Clone, Debug, uniffi::Enum)]
pub enum ParsedMessage {
    OfferPrivateEndpoint { offer: PrivateEndpointOffer },
    RequestReceipt { request: ReceiptRequest },
    ConfirmReceipt { receipt: ReceiptRequest },
    Ack,
    Error { error: ErrorMessage },
}

// ============================================================================
// Message Builder
// ============================================================================

/// Builder for creating Paykit protocol messages.
///
/// Use this to create JSON messages for sending over Noise channels.
#[derive(uniffi::Object)]
pub struct PaykitMessageBuilder {
    _phantom: std::marker::PhantomData<()>,
}

#[uniffi::export]
impl PaykitMessageBuilder {
    /// Create a new message builder.
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            _phantom: std::marker::PhantomData,
        })
    }

    /// Create an endpoint offer message.
    ///
    /// # Arguments
    ///
    /// * `method_id` - Payment method identifier (e.g., "lightning", "onchain")
    /// * `endpoint` - The endpoint to offer (e.g., Lightning invoice, Bitcoin address)
    ///
    /// # Returns
    ///
    /// JSON-encoded message ready to send over Noise channel.
    pub fn create_endpoint_offer(&self, method_id: String, endpoint: String) -> Result<String> {
        let msg = serde_json::json!({
            "type": "OfferPrivateEndpoint",
            "payload": {
                "method_id": method_id,
                "endpoint": endpoint
            }
        });
        serde_json::to_string(&msg)
            .map_err(|e| PaykitMobileError::Serialization { message: e.to_string() })
    }

    /// Create a receipt request message.
    ///
    /// # Arguments
    ///
    /// * `request` - The receipt request details
    ///
    /// # Returns
    ///
    /// JSON-encoded message ready to send over Noise channel.
    pub fn create_receipt_request(&self, request: ReceiptRequest) -> Result<String> {
        let metadata: serde_json::Value = serde_json::from_str(&request.metadata_json)
            .unwrap_or(serde_json::json!({}));
        
        let provisional_receipt = serde_json::json!({
            "receipt_id": request.receipt_id,
            "payer": request.payer,
            "payee": request.payee,
            "method_id": request.method_id,
            "amount": request.amount,
            "currency": request.currency,
            "created_at": current_timestamp(),
            "metadata": metadata
        });
        
        let msg = serde_json::json!({
            "type": "RequestReceipt",
            "payload": {
                "provisional_receipt": provisional_receipt
            }
        });
        
        serde_json::to_string(&msg)
            .map_err(|e| PaykitMobileError::Serialization { message: e.to_string() })
    }

    /// Create a receipt confirmation message.
    ///
    /// # Arguments
    ///
    /// * `receipt` - The confirmed receipt details
    ///
    /// # Returns
    ///
    /// JSON-encoded message ready to send over Noise channel.
    pub fn create_receipt_confirm(&self, receipt: ReceiptRequest) -> Result<String> {
        let metadata: serde_json::Value = serde_json::from_str(&receipt.metadata_json)
            .unwrap_or(serde_json::json!({}));
        
        let confirmed_receipt = serde_json::json!({
            "receipt_id": receipt.receipt_id,
            "payer": receipt.payer,
            "payee": receipt.payee,
            "method_id": receipt.method_id,
            "amount": receipt.amount,
            "currency": receipt.currency,
            "created_at": current_timestamp(),
            "metadata": metadata
        });
        
        let msg = serde_json::json!({
            "type": "ConfirmReceipt",
            "payload": {
                "receipt": confirmed_receipt
            }
        });
        
        serde_json::to_string(&msg)
            .map_err(|e| PaykitMobileError::Serialization { message: e.to_string() })
    }

    /// Create an acknowledgment message.
    ///
    /// # Returns
    ///
    /// JSON-encoded message ready to send over Noise channel.
    pub fn create_ack(&self) -> Result<String> {
        let msg = serde_json::json!({
            "type": "Ack",
            "payload": null
        });
        serde_json::to_string(&msg)
            .map_err(|e| PaykitMobileError::Serialization { message: e.to_string() })
    }

    /// Create an error message.
    ///
    /// # Arguments
    ///
    /// * `code` - Error code
    /// * `message` - Error message
    ///
    /// # Returns
    ///
    /// JSON-encoded message ready to send over Noise channel.
    pub fn create_error(&self, code: String, message: String) -> Result<String> {
        let msg = serde_json::json!({
            "type": "Error",
            "payload": {
                "code": code,
                "message": message
            }
        });
        serde_json::to_string(&msg)
            .map_err(|e| PaykitMobileError::Serialization { message: e.to_string() })
    }

    /// Parse a received message.
    ///
    /// # Arguments
    ///
    /// * `message_json` - JSON-encoded message from Noise channel
    ///
    /// # Returns
    ///
    /// Parsed message for processing.
    pub fn parse_message(&self, message_json: String) -> Result<ParsedMessage> {
        let value: serde_json::Value = serde_json::from_str(&message_json)
            .map_err(|e| PaykitMobileError::Validation { 
                message: format!("Invalid message JSON: {}", e) 
            })?;
        
        let msg_type = value.get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| PaykitMobileError::Validation { 
                message: "Missing message type".to_string() 
            })?;
        
        match msg_type {
            "OfferPrivateEndpoint" => {
                let payload = value.get("payload").ok_or_else(|| PaykitMobileError::Validation {
                    message: "Missing payload".to_string()
                })?;
                
                let method_id = payload.get("method_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let endpoint = payload.get("endpoint")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                
                Ok(ParsedMessage::OfferPrivateEndpoint {
                    offer: PrivateEndpointOffer { method_id, endpoint },
                })
            }
            "RequestReceipt" => {
                let payload = value.get("payload").ok_or_else(|| PaykitMobileError::Validation {
                    message: "Missing payload".to_string()
                })?;
                
                let receipt = payload.get("provisional_receipt").ok_or_else(|| PaykitMobileError::Validation {
                    message: "Missing provisional_receipt".to_string()
                })?;
                
                Ok(ParsedMessage::RequestReceipt {
                    request: parse_receipt_from_value(receipt)?,
                })
            }
            "ConfirmReceipt" => {
                let payload = value.get("payload").ok_or_else(|| PaykitMobileError::Validation {
                    message: "Missing payload".to_string()
                })?;
                
                let receipt = payload.get("receipt").ok_or_else(|| PaykitMobileError::Validation {
                    message: "Missing receipt".to_string()
                })?;
                
                Ok(ParsedMessage::ConfirmReceipt {
                    receipt: parse_receipt_from_value(receipt)?,
                })
            }
            "Ack" => Ok(ParsedMessage::Ack),
            "Error" => {
                let payload = value.get("payload").ok_or_else(|| PaykitMobileError::Validation {
                    message: "Missing payload".to_string()
                })?;
                
                let code = payload.get("code")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let message = payload.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                
                Ok(ParsedMessage::Error {
                    error: ErrorMessage { code, message },
                })
            }
            _ => Err(PaykitMobileError::Validation { 
                message: format!("Unknown message type: {}", msg_type) 
            }),
        }
    }

    /// Get the message type from a JSON message.
    ///
    /// # Arguments
    ///
    /// * `message_json` - JSON-encoded message
    ///
    /// # Returns
    ///
    /// The message type.
    pub fn get_message_type(&self, message_json: String) -> Result<PaykitMessageType> {
        let value: serde_json::Value = serde_json::from_str(&message_json)
            .map_err(|e| PaykitMobileError::Validation { 
                message: format!("Invalid message JSON: {}", e) 
            })?;
        
        let msg_type = value.get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| PaykitMobileError::Validation { 
                message: "Missing message type".to_string() 
            })?;
        
        match msg_type {
            "OfferPrivateEndpoint" => Ok(PaykitMessageType::OfferPrivateEndpoint),
            "RequestReceipt" => Ok(PaykitMessageType::RequestReceipt),
            "ConfirmReceipt" => Ok(PaykitMessageType::ConfirmReceipt),
            "Ack" => Ok(PaykitMessageType::Ack),
            "Error" => Ok(PaykitMessageType::Error),
            _ => Err(PaykitMobileError::Validation { 
                message: format!("Unknown message type: {}", msg_type) 
            }),
        }
    }
}

// ============================================================================
// Receipt Storage
// ============================================================================

/// In-memory receipt storage for mobile.
///
/// Stores receipts during a session. For persistence, mobile apps should
/// save receipts to their own storage (Keychain/SharedPreferences).
#[derive(uniffi::Object)]
pub struct ReceiptStore {
    receipts: std::sync::RwLock<std::collections::HashMap<String, ReceiptRequest>>,
    private_endpoints: std::sync::RwLock<std::collections::HashMap<String, PrivateEndpointOffer>>,
}

#[uniffi::export]
impl ReceiptStore {
    /// Create a new receipt store.
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            receipts: std::sync::RwLock::new(std::collections::HashMap::new()),
            private_endpoints: std::sync::RwLock::new(std::collections::HashMap::new()),
        })
    }

    /// Save a receipt.
    pub fn save_receipt(&self, receipt: ReceiptRequest) -> Result<()> {
        let mut receipts = self.receipts.write().map_err(|_| PaykitMobileError::Internal {
            message: "Lock poisoned".to_string(),
        })?;
        receipts.insert(receipt.receipt_id.clone(), receipt);
        Ok(())
    }

    /// Get a receipt by ID.
    pub fn get_receipt(&self, receipt_id: String) -> Result<Option<ReceiptRequest>> {
        let receipts = self.receipts.read().map_err(|_| PaykitMobileError::Internal {
            message: "Lock poisoned".to_string(),
        })?;
        Ok(receipts.get(&receipt_id).cloned())
    }

    /// List all receipts.
    pub fn list_receipts(&self) -> Result<Vec<ReceiptRequest>> {
        let receipts = self.receipts.read().map_err(|_| PaykitMobileError::Internal {
            message: "Lock poisoned".to_string(),
        })?;
        Ok(receipts.values().cloned().collect())
    }

    /// Delete a receipt.
    pub fn delete_receipt(&self, receipt_id: String) -> Result<()> {
        let mut receipts = self.receipts.write().map_err(|_| PaykitMobileError::Internal {
            message: "Lock poisoned".to_string(),
        })?;
        receipts.remove(&receipt_id);
        Ok(())
    }

    /// Save a private endpoint.
    pub fn save_private_endpoint(&self, peer: String, offer: PrivateEndpointOffer) -> Result<()> {
        let mut endpoints = self.private_endpoints.write().map_err(|_| PaykitMobileError::Internal {
            message: "Lock poisoned".to_string(),
        })?;
        let key = format!("{}:{}", peer, offer.method_id);
        endpoints.insert(key, offer);
        Ok(())
    }

    /// Get a private endpoint.
    pub fn get_private_endpoint(&self, peer: String, method_id: String) -> Result<Option<PrivateEndpointOffer>> {
        let endpoints = self.private_endpoints.read().map_err(|_| PaykitMobileError::Internal {
            message: "Lock poisoned".to_string(),
        })?;
        let key = format!("{}:{}", peer, method_id);
        Ok(endpoints.get(&key).cloned())
    }

    /// List all private endpoints for a peer.
    pub fn list_private_endpoints(&self, peer: String) -> Result<Vec<PrivateEndpointOffer>> {
        let endpoints = self.private_endpoints.read().map_err(|_| PaykitMobileError::Internal {
            message: "Lock poisoned".to_string(),
        })?;
        let prefix = format!("{}:", peer);
        Ok(endpoints
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(_, v)| v.clone())
            .collect())
    }

    /// Clear all stored data.
    pub fn clear(&self) -> Result<()> {
        {
            let mut receipts = self.receipts.write().map_err(|_| PaykitMobileError::Internal {
                message: "Lock poisoned".to_string(),
            })?;
            receipts.clear();
        }
        {
            let mut endpoints = self.private_endpoints.write().map_err(|_| PaykitMobileError::Internal {
                message: "Lock poisoned".to_string(),
            })?;
            endpoints.clear();
        }
        Ok(())
    }

    /// Export all receipts as JSON.
    pub fn export_receipts_json(&self) -> Result<String> {
        let receipts = self.receipts.read().map_err(|_| PaykitMobileError::Internal {
            message: "Lock poisoned".to_string(),
        })?;
        
        let list: Vec<_> = receipts.values().map(|r| {
            serde_json::json!({
                "receipt_id": r.receipt_id,
                "payer": r.payer,
                "payee": r.payee,
                "method_id": r.method_id,
                "amount": r.amount,
                "currency": r.currency,
                "metadata": serde_json::from_str::<serde_json::Value>(&r.metadata_json).unwrap_or(serde_json::json!({}))
            })
        }).collect();
        
        serde_json::to_string(&list)
            .map_err(|e| PaykitMobileError::Serialization { message: e.to_string() })
    }

    /// Import receipts from JSON.
    pub fn import_receipts_json(&self, json: String) -> Result<u32> {
        let list: Vec<serde_json::Value> = serde_json::from_str(&json)
            .map_err(|e| PaykitMobileError::Serialization { message: e.to_string() })?;
        
        let mut receipts = self.receipts.write().map_err(|_| PaykitMobileError::Internal {
            message: "Lock poisoned".to_string(),
        })?;
        
        let mut count = 0;
        for value in list {
            if let Ok(receipt) = parse_receipt_from_value(&value) {
                receipts.insert(receipt.receipt_id.clone(), receipt);
                count += 1;
            }
        }
        
        Ok(count)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn parse_receipt_from_value(value: &serde_json::Value) -> Result<ReceiptRequest> {
    Ok(ReceiptRequest {
        receipt_id: value.get("receipt_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        payer: value.get("payer")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        payee: value.get("payee")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        method_id: value.get("method_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        amount: value.get("amount")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        currency: value.get("currency")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        metadata_json: value.get("metadata")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "{}".to_string()),
    })
}

/// Create a new message builder.
#[uniffi::export]
pub fn create_message_builder() -> Arc<PaykitMessageBuilder> {
    PaykitMessageBuilder::new()
}

/// Create a new receipt store.
#[uniffi::export]
pub fn create_receipt_store() -> Arc<ReceiptStore> {
    ReceiptStore::new()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_endpoint_offer() {
        let builder = PaykitMessageBuilder::new();
        
        let result = builder.create_endpoint_offer("lightning".to_string(), "lnbc1...".to_string());
        assert!(result.is_ok());
        
        let json = result.unwrap();
        assert!(json.contains("OfferPrivateEndpoint"));
        assert!(json.contains("lightning"));
        assert!(json.contains("lnbc1"));
    }

    #[test]
    fn test_create_ack() {
        let builder = PaykitMessageBuilder::new();
        
        let result = builder.create_ack();
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Ack"));
    }

    #[test]
    fn test_create_error() {
        let builder = PaykitMessageBuilder::new();
        
        let result = builder.create_error("TEST_ERROR".to_string(), "Test message".to_string());
        assert!(result.is_ok());
        
        let json = result.unwrap();
        assert!(json.contains("Error"));
        assert!(json.contains("TEST_ERROR"));
    }

    #[test]
    fn test_parse_endpoint_offer() {
        let builder = PaykitMessageBuilder::new();
        
        let offer = builder.create_endpoint_offer("lightning".to_string(), "lnbc1...".to_string()).unwrap();
        let parsed = builder.parse_message(offer).unwrap();
        
        match parsed {
            ParsedMessage::OfferPrivateEndpoint { offer } => {
                assert_eq!(offer.method_id, "lightning");
                assert_eq!(offer.endpoint, "lnbc1...");
            }
            _ => panic!("Expected OfferPrivateEndpoint"),
        }
    }

    #[test]
    fn test_get_message_type() {
        let builder = PaykitMessageBuilder::new();
        
        let offer = builder.create_endpoint_offer("lightning".to_string(), "lnbc1...".to_string()).unwrap();
        let msg_type = builder.get_message_type(offer).unwrap();
        assert_eq!(msg_type, PaykitMessageType::OfferPrivateEndpoint);
        
        let ack = builder.create_ack().unwrap();
        let msg_type = builder.get_message_type(ack).unwrap();
        assert_eq!(msg_type, PaykitMessageType::Ack);
    }

    #[test]
    fn test_receipt_store() {
        let store = ReceiptStore::new();
        
        let receipt = ReceiptRequest {
            receipt_id: "test_receipt_1".to_string(),
            payer: "payer_pubkey".to_string(),
            payee: "payee_pubkey".to_string(),
            method_id: "lightning".to_string(),
            amount: Some("1000".to_string()),
            currency: Some("SAT".to_string()),
            metadata_json: "{}".to_string(),
        };
        
        // Save
        store.save_receipt(receipt.clone()).unwrap();
        
        // Get
        let retrieved = store.get_receipt("test_receipt_1".to_string()).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().receipt_id, "test_receipt_1");
        
        // List
        let all = store.list_receipts().unwrap();
        assert_eq!(all.len(), 1);
        
        // Delete
        store.delete_receipt("test_receipt_1".to_string()).unwrap();
        let deleted = store.get_receipt("test_receipt_1".to_string()).unwrap();
        assert!(deleted.is_none());
    }

    #[test]
    fn test_private_endpoint_store() {
        let store = ReceiptStore::new();
        
        let offer = PrivateEndpointOffer {
            method_id: "lightning".to_string(),
            endpoint: "lnbc1...".to_string(),
        };
        
        // Save
        store.save_private_endpoint("peer1".to_string(), offer).unwrap();
        
        // Get
        let retrieved = store.get_private_endpoint("peer1".to_string(), "lightning".to_string()).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().endpoint, "lnbc1...");
        
        // List
        let all = store.list_private_endpoints("peer1".to_string()).unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_export_import_receipts() {
        let store = ReceiptStore::new();
        
        // Add some receipts
        store.save_receipt(ReceiptRequest {
            receipt_id: "r1".to_string(),
            payer: "p1".to_string(),
            payee: "p2".to_string(),
            method_id: "lightning".to_string(),
            amount: Some("1000".to_string()),
            currency: Some("SAT".to_string()),
            metadata_json: "{}".to_string(),
        }).unwrap();
        
        store.save_receipt(ReceiptRequest {
            receipt_id: "r2".to_string(),
            payer: "p3".to_string(),
            payee: "p4".to_string(),
            method_id: "onchain".to_string(),
            amount: Some("5000".to_string()),
            currency: Some("SAT".to_string()),
            metadata_json: "{}".to_string(),
        }).unwrap();
        
        // Export
        let json = store.export_receipts_json().unwrap();
        
        // Clear and import
        store.clear().unwrap();
        assert_eq!(store.list_receipts().unwrap().len(), 0);
        
        let count = store.import_receipts_json(json).unwrap();
        assert_eq!(count, 2);
        assert_eq!(store.list_receipts().unwrap().len(), 2);
    }
}
