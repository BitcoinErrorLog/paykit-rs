//! Payment flow coordination using paykit-interactive

use crate::models::Receipt;
use anyhow::{Context, Result};
use paykit_interactive::{
    PaykitInteractiveManager, PaykitNoiseChannel, PaykitReceipt, PaykitStorage, ReceiptGenerator,
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

    /// Initiate a payment as the payer
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

    /// Handle incoming payment requests as the payee
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

        // Handle it
        let response = manager
            .handle_message(msg, &payer, &payee)
            .await
            .context("Failed to handle payment message")?;

        // Send response if any
        if let Some(response_msg) = response {
            channel
                .send(response_msg)
                .await
                .context("Failed to send response")?;

            // TODO: Extract receipt from response
            // For now, return None as we need to track state better
            Ok(None)
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
