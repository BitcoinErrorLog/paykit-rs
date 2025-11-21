use async_trait::async_trait;
use paykit_interactive::{
    InteractiveError, PaykitNoiseChannel, PaykitNoiseMessage, PaykitReceipt, PaykitStorage,
    ReceiptGenerator, Result,
};
use paykit_lib::{MethodId, PublicKey};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, Mutex as TokioMutex};

/// Mock storage implementation for testing
#[derive(Default)]
pub struct MockStorage {
    receipts: Arc<Mutex<HashMap<String, PaykitReceipt>>>,
    endpoints: Arc<Mutex<HashMap<(String, String), String>>>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl PaykitStorage for MockStorage {
    async fn save_receipt(&self, receipt: &PaykitReceipt) -> Result<()> {
        let mut receipts = self
            .receipts
            .lock()
            .map_err(|e| InteractiveError::Transport(format!("Mutex poisoned: {}", e)))?;
        receipts.insert(receipt.receipt_id.clone(), receipt.clone());
        Ok(())
    }

    async fn get_receipt(&self, receipt_id: &str) -> Result<Option<PaykitReceipt>> {
        let receipts = self
            .receipts
            .lock()
            .map_err(|e| InteractiveError::Transport(format!("Mutex poisoned: {}", e)))?;
        Ok(receipts.get(receipt_id).cloned())
    }

    async fn save_private_endpoint(
        &self,
        peer: &PublicKey,
        method: &MethodId,
        endpoint: &str,
    ) -> Result<()> {
        let mut endpoints = self
            .endpoints
            .lock()
            .map_err(|e| InteractiveError::Transport(format!("Mutex poisoned: {}", e)))?;
        let key = (format!("{:?}", peer), method.0.clone());
        endpoints.insert(key, endpoint.to_string());
        Ok(())
    }

    async fn get_private_endpoint(
        &self,
        peer: &PublicKey,
        method: &MethodId,
    ) -> Result<Option<String>> {
        let endpoints = self
            .endpoints
            .lock()
            .map_err(|e| InteractiveError::Transport(format!("Mutex poisoned: {}", e)))?;
        let key = (format!("{:?}", peer), method.0.clone());
        Ok(endpoints.get(&key).cloned())
    }
}

/// Mock receipt generator for testing
#[derive(Default)]
pub struct MockReceiptGenerator {
    /// If true, will add invoice metadata to receipts
    pub add_invoice: bool,
}

impl MockReceiptGenerator {
    pub fn new() -> Self {
        Self { add_invoice: true }
    }
}

#[async_trait]
impl ReceiptGenerator for MockReceiptGenerator {
    async fn generate_receipt(&self, request: &PaykitReceipt) -> Result<PaykitReceipt> {
        // Simulate receipt generation by adding invoice data
        let mut receipt = request.clone();

        if self.add_invoice {
            let mut metadata = request.metadata.clone();
            if let Some(obj) = metadata.as_object_mut() {
                obj.insert(
                    "invoice".to_string(),
                    serde_json::Value::String("lnbc1000...".to_string()),
                );
            }
            receipt.metadata = metadata;
        }

        Ok(receipt)
    }
}

/// Mock Noise channel using in-memory message passing
#[allow(dead_code)]
pub struct MockNoiseChannel {
    tx: mpsc::UnboundedSender<PaykitNoiseMessage>,
    rx: Arc<TokioMutex<mpsc::UnboundedReceiver<PaykitNoiseMessage>>>,
}

#[allow(dead_code)]
impl MockNoiseChannel {
    pub fn pair() -> (Self, Self) {
        let (tx1, rx1) = mpsc::unbounded_channel();
        let (tx2, rx2) = mpsc::unbounded_channel();

        let channel1 = Self {
            tx: tx1,
            rx: Arc::new(TokioMutex::new(rx2)),
        };

        let channel2 = Self {
            tx: tx2,
            rx: Arc::new(TokioMutex::new(rx1)),
        };

        (channel1, channel2)
    }
}

#[async_trait]
impl PaykitNoiseChannel for MockNoiseChannel {
    async fn send(&mut self, msg: PaykitNoiseMessage) -> Result<()> {
        self.tx
            .send(msg)
            .map_err(|e| InteractiveError::Transport(format!("Channel send failed: {}", e)))
    }

    async fn recv(&mut self) -> Result<PaykitNoiseMessage> {
        let mut rx = self.rx.lock().await;

        rx.recv()
            .await
            .ok_or_else(|| InteractiveError::Transport("Channel closed".into()))
    }
}
