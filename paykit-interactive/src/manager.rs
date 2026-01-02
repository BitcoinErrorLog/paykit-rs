use crate::{
    InteractiveError, PaykitNoiseChannel, PaykitNoiseMessage, PaykitReceipt, PaykitStorage, Result,
};
use paykit_lib::{MethodId, PublicKey};
use std::sync::Arc;

/// Trait for generating/finalizing receipts (e.g. creating Lightning invoices).
#[async_trait::async_trait]
pub trait ReceiptGenerator: Send + Sync {
    /// Process a provisional receipt request and return the finalized receipt.
    ///
    /// This is where the application should:
    /// 1. Validate the request (amount, item).
    /// 2. Generate a payment endpoint (e.g. BOLT11 invoice).
    /// 3. specificy the endpoint in the receipt metadata or appropriate fields.
    async fn generate_receipt(&self, request: &PaykitReceipt) -> Result<PaykitReceipt>;
}

/// Manages interactive Paykit flows over a secure channel.
pub struct PaykitInteractiveManager {
    storage: Arc<Box<dyn PaykitStorage>>,
    generator: Arc<Box<dyn ReceiptGenerator>>,
}

impl PaykitInteractiveManager {
    pub fn new(
        storage: Arc<Box<dyn PaykitStorage>>,
        generator: Arc<Box<dyn ReceiptGenerator>>,
    ) -> Self {
        Self { storage, generator }
    }

    /// Initiate a payment flow by requesting a receipt from a peer.
    ///
    /// * `channel`: The established Noise channel to the peer.
    /// * `provisional_receipt`: The receipt request details.
    ///
    /// # Timeout
    /// This function will timeout after 30 seconds if no response is received.
    pub async fn initiate_payment<C: PaykitNoiseChannel>(
        &self,
        channel: &mut C,
        provisional_receipt: PaykitReceipt,
    ) -> Result<PaykitReceipt> {
        use std::time::Duration;

        // 1. Send RequestReceipt
        channel
            .send(PaykitNoiseMessage::RequestReceipt {
                provisional_receipt: provisional_receipt.clone(),
            })
            .await?;

        // 2. Wait for response with timeout (30 seconds)
        #[cfg(feature = "timeout")]
        let msg = {
            tokio::time::timeout(Duration::from_secs(30), channel.recv())
                .await
                .map_err(|_| {
                    InteractiveError::Transport("Receipt confirmation timed out".into())
                })??
        };

        #[cfg(not(feature = "timeout"))]
        let msg = channel.recv().await?;

        match msg {
            PaykitNoiseMessage::ConfirmReceipt { receipt } => {
                // 3. Validate receipt matches request ID
                if receipt.receipt_id != provisional_receipt.receipt_id {
                    return Err(InteractiveError::Protocol("Receipt ID mismatch".into()));
                }

                // 4. Save confirmed receipt
                self.storage.save_receipt(&receipt).await?;
                Ok(receipt)
            }
            PaykitNoiseMessage::Error { code, message } => Err(InteractiveError::Protocol(
                format!("Peer error {}: {}", code, message),
            )),
            msg => Err(InteractiveError::Protocol(format!(
                "Unexpected message: {:?}",
                msg
            ))),
        }
    }

    /// Handle an incoming message from a peer.
    ///
    /// * `msg`: The incoming message.
    /// * `peer`: The public key of the sender.
    /// * `my_pubkey`: The public key of the receiver (self).
    ///
    /// Returns an optional response message to be sent back.
    pub async fn handle_message(
        &self,
        msg: PaykitNoiseMessage,
        peer: &PublicKey,
        my_pubkey: &PublicKey,
    ) -> Result<Option<PaykitNoiseMessage>> {
        match msg {
            PaykitNoiseMessage::OfferPrivateEndpoint {
                method_id,
                endpoint,
            } => {
                // Legacy single endpoint offer - save it
                self.storage
                    .save_private_endpoint(peer, &method_id, &endpoint)
                    .await?;
                Ok(Some(PaykitNoiseMessage::Ack))
            }
            PaykitNoiseMessage::OfferPrivateEndpoints { methods } => {
                // Save all offered endpoints
                let method_ids: Vec<MethodId> =
                    methods.iter().map(|o| o.method_id.clone()).collect();
                for offer in &methods {
                    self.storage
                        .save_private_endpoint(peer, &offer.method_id, &offer.endpoint)
                        .await?;
                }
                // Accept all endpoints (could be made configurable)
                Ok(Some(PaykitNoiseMessage::AcceptPrivateEndpoints {
                    method_ids,
                }))
            }
            PaykitNoiseMessage::AcceptPrivateEndpoints { method_ids: _ } => {
                // Endpoints were accepted - nothing more to do
                // The endpoints are already saved from the offer
                Ok(Some(PaykitNoiseMessage::Ack))
            }
            PaykitNoiseMessage::DeclinePrivateEndpoints { reason: _ } => {
                // Endpoints were declined - log for debugging
                // No action needed, just acknowledge
                Ok(Some(PaykitNoiseMessage::Ack))
            }
            PaykitNoiseMessage::RequestReceipt {
                provisional_receipt,
            } => {
                // 1. Validate request (is it for me?)
                if &provisional_receipt.payee != my_pubkey {
                    return Ok(Some(PaykitNoiseMessage::Error {
                        code: "WRONG_PAYEE".into(),
                        message: "I am not the intended payee".into(),
                    }));
                }

                // 2. Generate receipt using the generator (app logic)
                let confirmed_receipt = self
                    .generator
                    .generate_receipt(&provisional_receipt)
                    .await?;

                // 3. Save locally
                self.storage.save_receipt(&confirmed_receipt).await?;

                // 4. Respond with confirmation
                Ok(Some(PaykitNoiseMessage::ConfirmReceipt {
                    receipt: confirmed_receipt,
                }))
            }
            PaykitNoiseMessage::ConfirmReceipt { receipt } => {
                // Handle unsolicited confirmation or late arrival
                self.storage.save_receipt(&receipt).await?;
                Ok(Some(PaykitNoiseMessage::Ack))
            }
            PaykitNoiseMessage::Ack => {
                // Nothing to do
                Ok(None)
            }
            PaykitNoiseMessage::Error { .. } => {
                // Log error?
                Ok(None)
            }
        }
    }

    /// Send a private endpoint offer to a peer.
    pub async fn offer_private_endpoint<C: PaykitNoiseChannel>(
        &self,
        channel: &mut C,
        method_id: MethodId,
        endpoint: String,
    ) -> Result<()> {
        channel
            .send(PaykitNoiseMessage::OfferPrivateEndpoint {
                method_id,
                endpoint,
            })
            .await
    }
}
