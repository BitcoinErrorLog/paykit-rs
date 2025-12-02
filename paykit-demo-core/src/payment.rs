//! Payment flow coordination using paykit-interactive
//!
//! This module provides a simplified payment coordinator for demo applications,
//! wrapping the more complex `paykit-interactive` protocol with an easy-to-use API.
//!
//! # Architecture
//!
//! ```text
//! PaymentCoordinator
//!       ↓
//! PaykitInteractiveManager
//!       ↓
//! Noise Protocol Channel
//!       ↓
//! Payer ←→ Payee
//! ```
//!
//! # Examples
//!
//! ## Initiating a payment (payer side)
//!
//! ```ignore
//! use paykit_demo_core::{PaymentCoordinator, DemoPaykitStorage, DemoReceiptGenerator};
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! # use pubky::Keypair;
//! # let channel = todo!(); // Mock channel
//! # let payer = Keypair::random().public_key();
//! # let payee = Keypair::random().public_key();
//! let storage = Arc::new(Box::new(DemoPaykitStorage::new()) as Box<dyn paykit_interactive::PaykitStorage>);
//! let generator = Arc::new(Box::new(DemoReceiptGenerator) as Box<dyn paykit_interactive::ReceiptGenerator>);
//!
//! let coordinator = PaymentCoordinator::new(storage, generator);
//!
//! let receipt = coordinator.initiate_payment(
//!     channel,
//!     payer,
//!     payee,
//!     "lightning".to_string(),
//!     Some("1000".to_string()),
//!     Some("SAT".to_string()),
//! ).await?;
//!
//! println!("Payment initiated: {}", receipt.id);
//! # Ok(())
//! # }
//! ```
//!
//! ## Handling payment requests (payee side)
//!
//! ```ignore
//! # use paykit_demo_core::{PaymentCoordinator, DemoPaykitStorage, DemoReceiptGenerator};
//! # use std::sync::Arc;
//! # async fn example() -> anyhow::Result<()> {
//! # use pubky::Keypair;
//! # let channel = todo!();
//! # let payer = Keypair::random().public_key();
//! # let payee = Keypair::random().public_key();
//! # let storage = Arc::new(Box::new(DemoPaykitStorage::new()) as Box<dyn paykit_interactive::PaykitStorage>);
//! # let generator = Arc::new(Box::new(DemoReceiptGenerator) as Box<dyn paykit_interactive::ReceiptGenerator>);
//! let coordinator = PaymentCoordinator::new(storage, generator);
//!
//! if let Some(receipt) = coordinator.handle_payment_request(
//!     channel,
//!     payer,
//!     payee,
//! ).await? {
//!     println!("Payment received: {}", receipt.id);
//! }
//! # Ok(())
//! # }
//! ```

use crate::models::Receipt;
use anyhow::{Context, Result};
use paykit_interactive::{
    PaykitInteractiveManager, PaykitNoiseChannel, PaykitNoiseMessage, PaykitReceipt, PaykitStorage,
    ReceiptGenerator,
};
use paykit_lib::{MethodId, PublicKey};
use std::sync::Arc;

/// Coordinates payment flows using the interactive protocol
pub struct PaymentCoordinator {
    storage: Arc<Box<dyn PaykitStorage>>,
    receipt_generator: Arc<Box<dyn ReceiptGenerator>>,
}

impl PaymentCoordinator {
    /// Create a new payment coordinator
    pub fn new(
        storage: Arc<Box<dyn PaykitStorage>>,
        receipt_generator: Arc<Box<dyn ReceiptGenerator>>,
    ) -> Self {
        Self {
            storage,
            receipt_generator,
        }
    }

    /// Initiate a payment as the payer (simplified version)
    ///
    /// This directly sends a payment request and waits for a response.
    ///
    /// # Nonce Safety
    /// Receipt IDs are generated using UUID v4 which provides cryptographically
    /// secure random IDs, ensuring uniqueness and preventing replay attacks.
    pub async fn initiate_payment(
        &self,
        mut channel: impl PaykitNoiseChannel,
        payer: PublicKey,
        payee: PublicKey,
        method: String,
        amount: Option<String>,
        currency: Option<String>,
    ) -> Result<Receipt> {
        let manager =
            PaykitInteractiveManager::new(self.storage.clone(), self.receipt_generator.clone());

        // Generate cryptographically secure unique receipt ID using UUID v4
        let receipt_id = format!("receipt_{}", uuid::Uuid::new_v4());

        let provisional_receipt = PaykitReceipt::new(
            receipt_id.clone(),
            payer.clone(),
            payee.clone(),
            MethodId(method.clone()),
            amount.clone(),
            currency.clone(),
            serde_json::json!({}),
        );

        let final_receipt = manager
            .initiate_payment(&mut channel, provisional_receipt)
            .await
            .context("Failed to initiate payment")?;

        Ok(Receipt {
            id: final_receipt.receipt_id,
            payer: final_receipt.payer,
            payee: final_receipt.payee,
            method: final_receipt.method_id.0,
            amount: final_receipt.amount,
            currency: final_receipt.currency,
            timestamp: final_receipt.created_at,
            metadata: final_receipt.metadata,
        })
    }

    /// Simple payment initiation that returns early
    ///
    /// Sends a payment request and returns immediately with a provisional receipt.
    /// Useful for async/fire-and-forget scenarios.
    ///
    /// # Nonce Safety
    /// Receipt IDs use cryptographically secure UUID v4 for uniqueness.
    pub async fn send_payment_request(
        &self,
        mut channel: impl PaykitNoiseChannel,
        payer: PublicKey,
        payee: PublicKey,
        method: String,
        amount: Option<String>,
        currency: Option<String>,
    ) -> Result<Receipt> {
        // Generate cryptographically secure unique receipt ID using UUID v4
        let receipt_id = format!("receipt_{}", uuid::Uuid::new_v4());

        let provisional_receipt = PaykitReceipt::new(
            receipt_id.clone(),
            payer.clone(),
            payee.clone(),
            MethodId(method.clone()),
            amount.clone(),
            currency.clone(),
            serde_json::json!({"status": "pending"}),
        );

        // Send the payment request
        let msg = PaykitNoiseMessage::RequestReceipt {
            provisional_receipt: provisional_receipt.clone(),
        };

        channel
            .send(msg)
            .await
            .context("Failed to send payment request")?;

        Ok(Receipt {
            id: provisional_receipt.receipt_id,
            payer: provisional_receipt.payer,
            payee: provisional_receipt.payee,
            method: provisional_receipt.method_id.0,
            amount: provisional_receipt.amount,
            currency: provisional_receipt.currency,
            timestamp: provisional_receipt.created_at,
            metadata: provisional_receipt.metadata,
        })
    }

    /// Handle incoming payment requests as the payee
    ///
    /// Receives a payment request, processes it through the interactive manager,
    /// and returns the completed receipt if successful.
    pub async fn handle_payment_request(
        &self,
        mut channel: impl PaykitNoiseChannel,
        payer: PublicKey,
        payee: PublicKey,
    ) -> Result<Option<Receipt>> {
        let manager =
            PaykitInteractiveManager::new(self.storage.clone(), self.receipt_generator.clone());

        // Receive the request
        let msg = channel
            .recv()
            .await
            .context("Failed to receive payment request")?;

        // Handle the message and generate response
        let response = manager
            .handle_message(msg, &payer, &payee)
            .await
            .context("Failed to handle payment message")?;

        // Send response if any and extract confirmed receipt
        if let Some(response_msg) = response {
            // Extract receipt from the response message before sending
            let receipt_opt = match &response_msg {
                PaykitNoiseMessage::ConfirmReceipt { receipt } => Some(Receipt {
                    id: receipt.receipt_id.clone(),
                    payer: receipt.payer.clone(),
                    payee: receipt.payee.clone(),
                    method: receipt.method_id.0.clone(),
                    amount: receipt.amount.clone(),
                    currency: receipt.currency.clone(),
                    timestamp: receipt.created_at,
                    metadata: receipt.metadata.clone(),
                }),
                _ => None,
            };

            // Send the response
            channel
                .send(response_msg)
                .await
                .context("Failed to send response")?;

            Ok(receipt_opt)
        } else {
            Ok(None)
        }
    }
}

/// Helper to create a simple demo receipt generator
pub struct DemoReceiptGenerator;

#[async_trait::async_trait]
impl ReceiptGenerator for DemoReceiptGenerator {
    async fn generate_receipt(
        &self,
        request: &PaykitReceipt,
    ) -> paykit_interactive::Result<PaykitReceipt> {
        // Simple implementation: just add an invoice field
        let mut receipt = request.clone();
        let mut metadata = request.metadata.clone();

        if let Some(obj) = metadata.as_object_mut() {
            obj.insert(
                "invoice".to_string(),
                serde_json::Value::String(format!("INV-{}", request.receipt_id)),
            );
        }

        receipt.metadata = metadata;
        Ok(receipt)
    }
}

/// Helper for storage implementation
pub struct DemoPaykitStorage {
    receipts: Arc<tokio::sync::Mutex<std::collections::HashMap<String, PaykitReceipt>>>,
    endpoints: Arc<tokio::sync::Mutex<std::collections::HashMap<(String, String), String>>>,
}

impl DemoPaykitStorage {
    pub fn new() -> Self {
        Self {
            receipts: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            endpoints: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }
}

impl Default for DemoPaykitStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PaykitStorage for DemoPaykitStorage {
    async fn save_receipt(&self, receipt: &PaykitReceipt) -> paykit_interactive::Result<()> {
        let mut receipts = self.receipts.lock().await;
        receipts.insert(receipt.receipt_id.clone(), receipt.clone());
        Ok(())
    }

    async fn get_receipt(
        &self,
        receipt_id: &str,
    ) -> paykit_interactive::Result<Option<PaykitReceipt>> {
        let receipts = self.receipts.lock().await;
        Ok(receipts.get(receipt_id).cloned())
    }

    async fn save_private_endpoint(
        &self,
        peer: &PublicKey,
        method: &MethodId,
        endpoint: &str,
    ) -> paykit_interactive::Result<()> {
        let mut endpoints = self.endpoints.lock().await;
        let key = (format!("{:?}", peer), method.0.clone());
        endpoints.insert(key, endpoint.to_string());
        Ok(())
    }

    async fn get_private_endpoint(
        &self,
        peer: &PublicKey,
        method: &MethodId,
    ) -> paykit_interactive::Result<Option<String>> {
        let endpoints = self.endpoints.lock().await;
        let key = (format!("{:?}", peer), method.0.clone());
        Ok(endpoints.get(&key).cloned())
    }
}
