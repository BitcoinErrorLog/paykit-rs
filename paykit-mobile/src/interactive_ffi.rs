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
//! This module provides:
//! - `PaykitInteractiveManagerFFI` - High-level manager for payment flows
//! - `PaykitMessageBuilder` - Message serialization/deserialization helpers
//! - `ReceiptStore` - Receipt management
//! - `ReceiptGeneratorCallback` - Callback interface for mobile receipt generation
//!
//! Mobile apps handle the actual Noise channel communication using pubky-noise-main bindings.
//! The manager processes messages and produces responses to send back.
//!
//! # Example Flow
//!
//! ```ignore
//! // 1. Create manager with receipt generator callback
//! let generator = MyReceiptGenerator() // Implements ReceiptGeneratorCallback
//! let manager = PaykitInteractiveManagerFFI::new(store, generator)
//!
//! // 2. When receiving a message from Noise channel:
//! let response = manager.handleMessage(messageJson, peerPubkey, myPubkey)
//!
//! // 3. Send response back over Noise channel (if not null)
//! if let Some(responseJson) = response {
//!     noiseChannel.send(responseJson)
//! }
//! ```

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
            .map_err(|e| PaykitMobileError::Serialization { msg: e.to_string() })
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
        let metadata: serde_json::Value =
            serde_json::from_str(&request.metadata_json).unwrap_or(serde_json::json!({}));

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
            .map_err(|e| PaykitMobileError::Serialization { msg: e.to_string() })
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
        let metadata: serde_json::Value =
            serde_json::from_str(&receipt.metadata_json).unwrap_or(serde_json::json!({}));

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
            .map_err(|e| PaykitMobileError::Serialization { msg: e.to_string() })
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
            .map_err(|e| PaykitMobileError::Serialization { msg: e.to_string() })
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
            .map_err(|e| PaykitMobileError::Serialization { msg: e.to_string() })
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
        let value: serde_json::Value =
            serde_json::from_str(&message_json).map_err(|e| PaykitMobileError::Validation {
                msg: format!("Invalid message JSON: {}", e),
            })?;

        let msg_type = value.get("type").and_then(|t| t.as_str()).ok_or_else(|| {
            PaykitMobileError::Validation {
                msg: "Missing message type".to_string(),
            }
        })?;

        match msg_type {
            "OfferPrivateEndpoint" => {
                let payload =
                    value
                        .get("payload")
                        .ok_or_else(|| PaykitMobileError::Validation {
                            msg: "Missing payload".to_string(),
                        })?;

                let method_id = payload
                    .get("method_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let endpoint = payload
                    .get("endpoint")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                Ok(ParsedMessage::OfferPrivateEndpoint {
                    offer: PrivateEndpointOffer {
                        method_id,
                        endpoint,
                    },
                })
            }
            "RequestReceipt" => {
                let payload =
                    value
                        .get("payload")
                        .ok_or_else(|| PaykitMobileError::Validation {
                            msg: "Missing payload".to_string(),
                        })?;

                let receipt = payload.get("provisional_receipt").ok_or_else(|| {
                    PaykitMobileError::Validation {
                        msg: "Missing provisional_receipt".to_string(),
                    }
                })?;

                Ok(ParsedMessage::RequestReceipt {
                    request: parse_receipt_from_value(receipt)?,
                })
            }
            "ConfirmReceipt" => {
                let payload =
                    value
                        .get("payload")
                        .ok_or_else(|| PaykitMobileError::Validation {
                            msg: "Missing payload".to_string(),
                        })?;

                let receipt =
                    payload
                        .get("receipt")
                        .ok_or_else(|| PaykitMobileError::Validation {
                            msg: "Missing receipt".to_string(),
                        })?;

                Ok(ParsedMessage::ConfirmReceipt {
                    receipt: parse_receipt_from_value(receipt)?,
                })
            }
            "Ack" => Ok(ParsedMessage::Ack),
            "Error" => {
                let payload =
                    value
                        .get("payload")
                        .ok_or_else(|| PaykitMobileError::Validation {
                            msg: "Missing payload".to_string(),
                        })?;

                let code = payload
                    .get("code")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let message = payload
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                Ok(ParsedMessage::Error {
                    error: ErrorMessage { code, message },
                })
            }
            _ => Err(PaykitMobileError::Validation {
                msg: format!("Unknown message type: {}", msg_type),
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
        let value: serde_json::Value =
            serde_json::from_str(&message_json).map_err(|e| PaykitMobileError::Validation {
                msg: format!("Invalid message JSON: {}", e),
            })?;

        let msg_type = value.get("type").and_then(|t| t.as_str()).ok_or_else(|| {
            PaykitMobileError::Validation {
                msg: "Missing message type".to_string(),
            }
        })?;

        match msg_type {
            "OfferPrivateEndpoint" => Ok(PaykitMessageType::OfferPrivateEndpoint),
            "RequestReceipt" => Ok(PaykitMessageType::RequestReceipt),
            "ConfirmReceipt" => Ok(PaykitMessageType::ConfirmReceipt),
            "Ack" => Ok(PaykitMessageType::Ack),
            "Error" => Ok(PaykitMessageType::Error),
            _ => Err(PaykitMobileError::Validation {
                msg: format!("Unknown message type: {}", msg_type),
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
        let mut receipts = self
            .receipts
            .write()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
            })?;
        receipts.insert(receipt.receipt_id.clone(), receipt);
        Ok(())
    }

    /// Get a receipt by ID.
    pub fn get_receipt(&self, receipt_id: String) -> Result<Option<ReceiptRequest>> {
        let receipts = self
            .receipts
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
            })?;
        Ok(receipts.get(&receipt_id).cloned())
    }

    /// List all receipts.
    pub fn list_receipts(&self) -> Result<Vec<ReceiptRequest>> {
        let receipts = self
            .receipts
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
            })?;
        Ok(receipts.values().cloned().collect())
    }

    /// Delete a receipt.
    pub fn delete_receipt(&self, receipt_id: String) -> Result<()> {
        let mut receipts = self
            .receipts
            .write()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
            })?;
        receipts.remove(&receipt_id);
        Ok(())
    }

    /// Save a private endpoint.
    pub fn save_private_endpoint(&self, peer: String, offer: PrivateEndpointOffer) -> Result<()> {
        let mut endpoints =
            self.private_endpoints
                .write()
                .map_err(|_| PaykitMobileError::Internal {
                    msg: "Lock poisoned".to_string(),
                })?;
        let key = format!("{}:{}", peer, offer.method_id);
        endpoints.insert(key, offer);
        Ok(())
    }

    /// Get a private endpoint.
    pub fn get_private_endpoint(
        &self,
        peer: String,
        method_id: String,
    ) -> Result<Option<PrivateEndpointOffer>> {
        let endpoints = self
            .private_endpoints
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
            })?;
        let key = format!("{}:{}", peer, method_id);
        Ok(endpoints.get(&key).cloned())
    }

    /// List all private endpoints for a peer.
    pub fn list_private_endpoints(&self, peer: String) -> Result<Vec<PrivateEndpointOffer>> {
        let endpoints = self
            .private_endpoints
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
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
            let mut receipts = self
                .receipts
                .write()
                .map_err(|_| PaykitMobileError::Internal {
                    msg: "Lock poisoned".to_string(),
                })?;
            receipts.clear();
        }
        {
            let mut endpoints =
                self.private_endpoints
                    .write()
                    .map_err(|_| PaykitMobileError::Internal {
                        msg: "Lock poisoned".to_string(),
                    })?;
            endpoints.clear();
        }
        Ok(())
    }

    /// Export all receipts as JSON.
    pub fn export_receipts_json(&self) -> Result<String> {
        let receipts = self
            .receipts
            .read()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
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
            .map_err(|e| PaykitMobileError::Serialization { msg: e.to_string() })
    }

    /// Import receipts from JSON.
    pub fn import_receipts_json(&self, json: String) -> Result<u32> {
        let list: Vec<serde_json::Value> = serde_json::from_str(&json)
            .map_err(|e| PaykitMobileError::Serialization { msg: e.to_string() })?;

        let mut receipts = self
            .receipts
            .write()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Lock poisoned".to_string(),
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
        receipt_id: value
            .get("receipt_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        payer: value
            .get("payer")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        payee: value
            .get("payee")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        method_id: value
            .get("method_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        amount: value
            .get("amount")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        currency: value
            .get("currency")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        metadata_json: value
            .get("metadata")
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
// Receipt Generator Callback
// ============================================================================

/// Result type for receipt generation.
///
/// Used to communicate success/failure from mobile callbacks.
#[derive(Clone, Debug, uniffi::Record)]
pub struct ReceiptGenerationResult {
    /// Whether generation succeeded
    pub success: bool,
    /// The generated receipt (if successful)
    pub receipt: Option<ReceiptRequest>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl ReceiptGenerationResult {
    /// Create a successful result.
    pub fn ok(receipt: ReceiptRequest) -> Self {
        Self {
            success: true,
            receipt: Some(receipt),
            error: None,
        }
    }

    /// Create a failed result.
    pub fn err(message: String) -> Self {
        Self {
            success: false,
            receipt: None,
            error: Some(message),
        }
    }
}

/// Callback interface for mobile receipt generation.
///
/// Mobile apps implement this to generate receipts (e.g., create Lightning invoices).
/// When a payment request is received, this callback is invoked to produce
/// the final receipt with payment endpoint.
///
/// # Example (Swift)
///
/// ```swift
/// class MyReceiptGenerator: ReceiptGeneratorCallback {
///     func generateReceipt(request: ReceiptRequest) -> ReceiptGenerationResult {
///         // Create Lightning invoice
///         let invoice = createInvoice(amount: request.amount)
///         
///         // Update receipt with invoice in metadata
///         var receipt = request
///         receipt.metadataJson = "{\"invoice\":\"\(invoice)\"}"
///         
///         return ReceiptGenerationResult.ok(receipt: receipt)
///     }
/// }
/// ```
#[uniffi::export(callback_interface)]
pub trait ReceiptGeneratorCallback: Send + Sync {
    /// Generate a receipt for a payment request.
    ///
    /// # Arguments
    ///
    /// * `request` - The provisional receipt request from the payer
    ///
    /// # Returns
    ///
    /// A `ReceiptGenerationResult` with either the finalized receipt or an error.
    fn generate_receipt(&self, request: ReceiptRequest) -> ReceiptGenerationResult;
}

// ============================================================================
// Interactive Manager FFI
// ============================================================================

/// FFI wrapper for PaykitInteractiveManager.
///
/// This provides a high-level interface for managing interactive payment flows
/// over Noise channels. Mobile apps use this to:
///
/// 1. Process incoming messages and generate responses
/// 2. Initiate payment flows
/// 3. Manage receipts and private endpoints
///
/// # Thread Safety
///
/// This type is thread-safe and can be used from multiple threads.
#[derive(uniffi::Object)]
pub struct PaykitInteractiveManagerFFI {
    /// Receipt storage
    store: Arc<ReceiptStore>,
    /// Receipt generator callback (provided by mobile app)
    generator: std::sync::RwLock<Option<Box<dyn ReceiptGeneratorCallback>>>,
    /// Message builder for serialization
    message_builder: Arc<PaykitMessageBuilder>,
}

#[uniffi::export]
impl PaykitInteractiveManagerFFI {
    /// Create a new interactive manager without a generator.
    ///
    /// Use `set_generator` to set the receipt generator callback.
    ///
    /// # Arguments
    ///
    /// * `store` - Receipt store for persistence
    #[uniffi::constructor]
    pub fn new(store: Arc<ReceiptStore>) -> Arc<Self> {
        Arc::new(Self {
            store,
            generator: std::sync::RwLock::new(None),
            message_builder: PaykitMessageBuilder::new(),
        })
    }

    /// Set the receipt generator callback.
    ///
    /// This must be called before handling receipt requests.
    ///
    /// # Arguments
    ///
    /// * `generator` - Callback for generating receipts (implement in Swift/Kotlin)
    ///
    /// # Errors
    ///
    /// Returns an error if the internal lock is poisoned.
    pub fn set_generator(&self, generator: Box<dyn ReceiptGeneratorCallback>) -> Result<()> {
        let mut guard = self
            .generator
            .write()
            .map_err(|_| PaykitMobileError::Internal {
                msg: "Generator lock poisoned".to_string(),
            })?;
        *guard = Some(generator);
        Ok(())
    }

    /// Handle an incoming message from a peer.
    ///
    /// This processes a JSON message received over a Noise channel and returns
    /// an optional response to send back.
    ///
    /// # Arguments
    ///
    /// * `message_json` - The JSON-encoded message from the Noise channel
    /// * `peer_pubkey` - The public key of the peer who sent the message
    /// * `my_pubkey` - Your own public key
    ///
    /// # Returns
    ///
    /// Optional JSON response to send back over the Noise channel.
    /// Returns `None` for messages that don't require a response (e.g., Ack).
    ///
    /// # Example
    ///
    /// ```ignore
    /// // In Swift/Kotlin
    /// let response = manager.handleMessage(messageJson, peerPubkey, myPubkey)
    /// if let responseJson = response {
    ///     noiseChannel.send(responseJson)
    /// }
    /// ```
    pub fn handle_message(
        &self,
        message_json: String,
        peer_pubkey: String,
        my_pubkey: String,
    ) -> Result<Option<String>> {
        // Parse the message
        let parsed = self.message_builder.parse_message(message_json)?;

        match parsed {
            ParsedMessage::OfferPrivateEndpoint { offer } => {
                // Save the private endpoint offered by the peer
                self.store.save_private_endpoint(peer_pubkey, offer)?;
                // Respond with Ack
                let response = self.message_builder.create_ack()?;
                Ok(Some(response))
            }
            ParsedMessage::RequestReceipt { request } => {
                // Validate request (is it for me?)
                if request.payee != my_pubkey {
                    let response = self.message_builder.create_error(
                        "WRONG_PAYEE".to_string(),
                        "I am not the intended payee".to_string(),
                    )?;
                    return Ok(Some(response));
                }

                // Get the generator
                let generator_guard =
                    self.generator
                        .read()
                        .map_err(|_| PaykitMobileError::Internal {
                            msg: "Lock poisoned".to_string(),
                        })?;

                let generator = match generator_guard.as_ref() {
                    Some(g) => g,
                    None => {
                        let response = self.message_builder.create_error(
                            "NO_GENERATOR".to_string(),
                            "Receipt generator not configured".to_string(),
                        )?;
                        return Ok(Some(response));
                    }
                };

                // Generate receipt using the callback
                let result = generator.generate_receipt(request.clone());
                if result.success {
                    if let Some(confirmed_receipt) = result.receipt {
                        // Save locally
                        self.store.save_receipt(confirmed_receipt.clone())?;
                        // Respond with confirmation
                        let response = self
                            .message_builder
                            .create_receipt_confirm(confirmed_receipt)?;
                        Ok(Some(response))
                    } else {
                        let response = self.message_builder.create_error(
                            "GENERATION_FAILED".to_string(),
                            "No receipt returned".to_string(),
                        )?;
                        Ok(Some(response))
                    }
                } else {
                    let err_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                    let response = self
                        .message_builder
                        .create_error("GENERATION_FAILED".to_string(), err_msg)?;
                    Ok(Some(response))
                }
            }
            ParsedMessage::ConfirmReceipt { receipt } => {
                // Handle confirmation (late arrival or unsolicited)
                self.store.save_receipt(receipt)?;
                let response = self.message_builder.create_ack()?;
                Ok(Some(response))
            }
            ParsedMessage::Ack => {
                // Nothing to do
                Ok(None)
            }
            ParsedMessage::Error { error } => {
                // Log error, no response needed
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "Received error from peer: {} - {}",
                    error.code,
                    error.message
                );
                let _ = error; // Suppress unused warning
                Ok(None)
            }
        }
    }

    /// Create a payment request message to initiate a payment flow.
    ///
    /// Use this to create the initial message for requesting payment from a peer.
    ///
    /// # Arguments
    ///
    /// * `payer` - Your public key (the one paying)
    /// * `payee` - The recipient's public key
    /// * `method_id` - Payment method (e.g., "lightning", "onchain")
    /// * `amount` - Optional amount (as string, e.g., "1000")
    /// * `currency` - Optional currency code (e.g., "SAT")
    /// * `metadata_json` - Optional JSON metadata
    ///
    /// # Returns
    ///
    /// JSON message to send over Noise channel.
    pub fn create_payment_request(
        &self,
        payer: String,
        payee: String,
        method_id: String,
        amount: Option<String>,
        currency: Option<String>,
        metadata_json: Option<String>,
    ) -> Result<String> {
        let receipt_id = generate_receipt_id();
        let request = ReceiptRequest {
            receipt_id,
            payer,
            payee,
            method_id,
            amount,
            currency,
            metadata_json: metadata_json.unwrap_or_else(|| "{}".to_string()),
        };

        // Save provisional receipt
        self.store.save_receipt(request.clone())?;

        // Create message
        self.message_builder.create_receipt_request(request)
    }

    /// Handle a payment confirmation response.
    ///
    /// Call this when you receive a response to your payment request.
    /// It validates the response and saves the confirmed receipt.
    ///
    /// # Arguments
    ///
    /// * `response_json` - The JSON response from the Noise channel
    /// * `original_receipt_id` - The receipt ID from your original request
    ///
    /// # Returns
    ///
    /// The confirmed receipt if successful, or an error.
    pub fn handle_payment_response(
        &self,
        response_json: String,
        original_receipt_id: String,
    ) -> Result<ReceiptRequest> {
        let parsed = self.message_builder.parse_message(response_json)?;

        match parsed {
            ParsedMessage::ConfirmReceipt { receipt } => {
                // Validate receipt matches request ID
                if receipt.receipt_id != original_receipt_id {
                    return Err(PaykitMobileError::Validation {
                        msg: format!(
                            "Receipt ID mismatch: expected {}, got {}",
                            original_receipt_id, receipt.receipt_id
                        ),
                    });
                }

                // Save confirmed receipt
                self.store.save_receipt(receipt.clone())?;
                Ok(receipt)
            }
            ParsedMessage::Error { error } => Err(PaykitMobileError::Transport {
                msg: format!("Payment rejected: {} - {}", error.code, error.message),
            }),
            _ => Err(PaykitMobileError::Validation {
                msg: "Unexpected response type".to_string(),
            }),
        }
    }

    /// Create a private endpoint offer message.
    ///
    /// Use this to offer a private payment endpoint to a peer.
    ///
    /// # Arguments
    ///
    /// * `method_id` - Payment method (e.g., "lightning")
    /// * `endpoint` - The endpoint to offer (e.g., Lightning invoice)
    ///
    /// # Returns
    ///
    /// JSON message to send over Noise channel.
    pub fn create_endpoint_offer(&self, method_id: String, endpoint: String) -> Result<String> {
        self.message_builder
            .create_endpoint_offer(method_id, endpoint)
    }

    /// Get the receipt store.
    pub fn get_store(&self) -> Arc<ReceiptStore> {
        self.store.clone()
    }

    /// Get a receipt by ID.
    pub fn get_receipt(&self, receipt_id: String) -> Result<Option<ReceiptRequest>> {
        self.store.get_receipt(receipt_id)
    }

    /// List all receipts.
    pub fn list_receipts(&self) -> Result<Vec<ReceiptRequest>> {
        self.store.list_receipts()
    }

    /// Get a private endpoint for a peer.
    pub fn get_private_endpoint(
        &self,
        peer: String,
        method_id: String,
    ) -> Result<Option<PrivateEndpointOffer>> {
        self.store.get_private_endpoint(peer, method_id)
    }

    /// List all private endpoints for a peer.
    pub fn list_private_endpoints(&self, peer: String) -> Result<Vec<PrivateEndpointOffer>> {
        self.store.list_private_endpoints(peer)
    }
}

/// Generate a unique receipt ID.
fn generate_receipt_id() -> String {
    let timestamp = current_timestamp();
    let random = rand_suffix();
    format!("rcpt_{}_{}", timestamp, random)
}

/// Generate a random suffix for IDs.
fn rand_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{:08x}", nanos)
}

/// Create a new interactive manager.
#[uniffi::export]
pub fn create_interactive_manager(store: Arc<ReceiptStore>) -> Arc<PaykitInteractiveManagerFFI> {
    PaykitInteractiveManagerFFI::new(store)
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

        let offer = builder
            .create_endpoint_offer("lightning".to_string(), "lnbc1...".to_string())
            .unwrap();
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

        let offer = builder
            .create_endpoint_offer("lightning".to_string(), "lnbc1...".to_string())
            .unwrap();
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
        store
            .save_private_endpoint("peer1".to_string(), offer)
            .unwrap();

        // Get
        let retrieved = store
            .get_private_endpoint("peer1".to_string(), "lightning".to_string())
            .unwrap();
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
        store
            .save_receipt(ReceiptRequest {
                receipt_id: "r1".to_string(),
                payer: "p1".to_string(),
                payee: "p2".to_string(),
                method_id: "lightning".to_string(),
                amount: Some("1000".to_string()),
                currency: Some("SAT".to_string()),
                metadata_json: "{}".to_string(),
            })
            .unwrap();

        store
            .save_receipt(ReceiptRequest {
                receipt_id: "r2".to_string(),
                payer: "p3".to_string(),
                payee: "p4".to_string(),
                method_id: "onchain".to_string(),
                amount: Some("5000".to_string()),
                currency: Some("SAT".to_string()),
                metadata_json: "{}".to_string(),
            })
            .unwrap();

        // Export
        let json = store.export_receipts_json().unwrap();

        // Clear and import
        store.clear().unwrap();
        assert_eq!(store.list_receipts().unwrap().len(), 0);

        let count = store.import_receipts_json(json).unwrap();
        assert_eq!(count, 2);
        assert_eq!(store.list_receipts().unwrap().len(), 2);
    }

    // ========================================================================
    // Interactive Manager Tests
    // ========================================================================

    /// Test receipt generator that just echoes back the request.
    struct EchoReceiptGenerator;

    impl ReceiptGeneratorCallback for EchoReceiptGenerator {
        fn generate_receipt(&self, request: ReceiptRequest) -> ReceiptGenerationResult {
            // Just return the request with metadata updated
            let mut receipt = request;
            receipt.metadata_json = r#"{"invoice":"lnbc1..."}"#.to_string();
            ReceiptGenerationResult::ok(receipt)
        }
    }

    /// Test receipt generator that fails.
    struct FailingReceiptGenerator;

    impl ReceiptGeneratorCallback for FailingReceiptGenerator {
        fn generate_receipt(&self, _request: ReceiptRequest) -> ReceiptGenerationResult {
            ReceiptGenerationResult::err("Invoice generation failed".to_string())
        }
    }

    /// Helper to create manager with generator for tests
    fn create_test_manager(
        generator: Box<dyn ReceiptGeneratorCallback>,
    ) -> Arc<PaykitInteractiveManagerFFI> {
        let store = ReceiptStore::new();
        let manager = PaykitInteractiveManagerFFI::new(store);
        manager.set_generator(generator).unwrap();
        manager
    }

    #[test]
    fn test_manager_handle_endpoint_offer() {
        let manager = create_test_manager(Box::new(EchoReceiptGenerator));

        // Create an endpoint offer message
        let offer_msg = manager
            .create_endpoint_offer("lightning".to_string(), "lnbc1...".to_string())
            .unwrap();

        // Handle it (simulating receiving this message)
        let response = manager
            .handle_message(
                offer_msg,
                "peer_pubkey".to_string(),
                "my_pubkey".to_string(),
            )
            .unwrap();

        // Should respond with Ack
        assert!(response.is_some());
        let response_json = response.unwrap();
        assert!(response_json.contains("Ack"));

        // Endpoint should be saved
        let endpoint = manager
            .get_private_endpoint("peer_pubkey".to_string(), "lightning".to_string())
            .unwrap();
        assert!(endpoint.is_some());
        assert_eq!(endpoint.unwrap().endpoint, "lnbc1...");
    }

    #[test]
    fn test_manager_handle_receipt_request() {
        let manager = create_test_manager(Box::new(EchoReceiptGenerator));

        // Create a receipt request (as if from another manager)
        let builder = PaykitMessageBuilder::new();
        let request = ReceiptRequest {
            receipt_id: "test_receipt".to_string(),
            payer: "payer_pubkey".to_string(),
            payee: "my_pubkey".to_string(),
            method_id: "lightning".to_string(),
            amount: Some("1000".to_string()),
            currency: Some("SAT".to_string()),
            metadata_json: "{}".to_string(),
        };
        let request_msg = builder.create_receipt_request(request).unwrap();

        // Handle the request
        let response = manager
            .handle_message(
                request_msg,
                "payer_pubkey".to_string(),
                "my_pubkey".to_string(),
            )
            .unwrap();

        // Should respond with ConfirmReceipt
        assert!(response.is_some());
        let response_json = response.unwrap();
        assert!(response_json.contains("ConfirmReceipt"));
        assert!(response_json.contains("lnbc1...")); // From generator

        // Receipt should be saved
        let saved = manager.get_receipt("test_receipt".to_string()).unwrap();
        assert!(saved.is_some());
    }

    #[test]
    fn test_manager_handle_wrong_payee() {
        let manager = create_test_manager(Box::new(EchoReceiptGenerator));

        // Create a receipt request for someone else
        let builder = PaykitMessageBuilder::new();
        let request = ReceiptRequest {
            receipt_id: "test_receipt".to_string(),
            payer: "payer_pubkey".to_string(),
            payee: "someone_else".to_string(), // Not me!
            method_id: "lightning".to_string(),
            amount: Some("1000".to_string()),
            currency: Some("SAT".to_string()),
            metadata_json: "{}".to_string(),
        };
        let request_msg = builder.create_receipt_request(request).unwrap();

        // Handle the request
        let response = manager
            .handle_message(
                request_msg,
                "payer_pubkey".to_string(),
                "my_pubkey".to_string(),
            )
            .unwrap();

        // Should respond with Error
        assert!(response.is_some());
        let response_json = response.unwrap();
        assert!(response_json.contains("Error"));
        assert!(response_json.contains("WRONG_PAYEE"));
    }

    #[test]
    fn test_manager_generator_failure() {
        let manager = create_test_manager(Box::new(FailingReceiptGenerator));

        // Create a receipt request
        let builder = PaykitMessageBuilder::new();
        let request = ReceiptRequest {
            receipt_id: "test_receipt".to_string(),
            payer: "payer_pubkey".to_string(),
            payee: "my_pubkey".to_string(),
            method_id: "lightning".to_string(),
            amount: Some("1000".to_string()),
            currency: Some("SAT".to_string()),
            metadata_json: "{}".to_string(),
        };
        let request_msg = builder.create_receipt_request(request).unwrap();

        // Handle the request
        let response = manager
            .handle_message(
                request_msg,
                "payer_pubkey".to_string(),
                "my_pubkey".to_string(),
            )
            .unwrap();

        // Should respond with Error
        assert!(response.is_some());
        let response_json = response.unwrap();
        assert!(response_json.contains("Error"));
        assert!(response_json.contains("GENERATION_FAILED"));
    }

    #[test]
    fn test_manager_no_generator() {
        let store = ReceiptStore::new();
        let manager = PaykitInteractiveManagerFFI::new(store);
        // Don't set generator

        // Create a receipt request
        let builder = PaykitMessageBuilder::new();
        let request = ReceiptRequest {
            receipt_id: "test_receipt".to_string(),
            payer: "payer_pubkey".to_string(),
            payee: "my_pubkey".to_string(),
            method_id: "lightning".to_string(),
            amount: Some("1000".to_string()),
            currency: Some("SAT".to_string()),
            metadata_json: "{}".to_string(),
        };
        let request_msg = builder.create_receipt_request(request).unwrap();

        // Handle the request - should return error because no generator
        let response = manager
            .handle_message(
                request_msg,
                "payer_pubkey".to_string(),
                "my_pubkey".to_string(),
            )
            .unwrap();

        assert!(response.is_some());
        let response_json = response.unwrap();
        assert!(response_json.contains("Error"));
        assert!(response_json.contains("NO_GENERATOR"));
    }

    #[test]
    fn test_manager_create_payment_request() {
        let manager = create_test_manager(Box::new(EchoReceiptGenerator));

        // Create a payment request
        let request_msg = manager
            .create_payment_request(
                "my_pubkey".to_string(),
                "payee_pubkey".to_string(),
                "lightning".to_string(),
                Some("1000".to_string()),
                Some("SAT".to_string()),
                None,
            )
            .unwrap();

        // Should be a valid RequestReceipt message
        assert!(request_msg.contains("RequestReceipt"));
        assert!(request_msg.contains("rcpt_")); // Receipt ID prefix

        // Provisional receipt should be saved
        let receipts = manager.list_receipts().unwrap();
        assert_eq!(receipts.len(), 1);
    }

    #[test]
    fn test_manager_handle_payment_response() {
        let manager = create_test_manager(Box::new(EchoReceiptGenerator));

        // Create a confirmation response
        let builder = PaykitMessageBuilder::new();
        let receipt = ReceiptRequest {
            receipt_id: "expected_id".to_string(),
            payer: "payer".to_string(),
            payee: "payee".to_string(),
            method_id: "lightning".to_string(),
            amount: Some("1000".to_string()),
            currency: Some("SAT".to_string()),
            metadata_json: r#"{"invoice":"lnbc1..."}"#.to_string(),
        };
        let confirm_msg = builder.create_receipt_confirm(receipt).unwrap();

        // Handle response with matching ID
        let result =
            manager.handle_payment_response(confirm_msg.clone(), "expected_id".to_string());
        assert!(result.is_ok());
        let confirmed = result.unwrap();
        assert_eq!(confirmed.receipt_id, "expected_id");

        // Handle response with wrong ID should fail
        let result = manager.handle_payment_response(confirm_msg, "wrong_id".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_handle_ack() {
        let manager = create_test_manager(Box::new(EchoReceiptGenerator));

        let builder = PaykitMessageBuilder::new();
        let ack_msg = builder.create_ack().unwrap();

        let response = manager
            .handle_message(ack_msg, "peer".to_string(), "me".to_string())
            .unwrap();

        // Ack should not produce a response
        assert!(response.is_none());
    }
}
