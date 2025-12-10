//! P2P Payment Example
//!
//! This example demonstrates a peer-to-peer payment flow:
//! - Two-party payment negotiation
//! - Private endpoint exchange
//! - Receipt exchange
//!
//! # Usage
//!
//! ```bash
//! cargo run --example p2p-payment
//! ```

use paykit_interactive::{
    PaykitInteractiveManager, PaykitNoiseChannel, PaykitNoiseMessage, PaykitReceipt,
    PaykitStorage, ReceiptGenerator,
};
use paykit_lib::{MethodId, PublicKey};
use paykit_lib::private_endpoints::{InMemoryStore, PrivateEndpoint, PrivateEndpointManager};
use std::sync::Arc;

// Mock implementations
struct MockStorage;
struct MockGenerator;
struct MockChannel {
    messages: Vec<PaykitNoiseMessage>,
}

#[async_trait::async_trait]
impl PaykitStorage for MockStorage {
    async fn save_receipt(&self, _receipt: &PaykitReceipt) -> paykit_interactive::Result<()> {
        Ok(())
    }
    async fn get_receipt(
        &self,
        _id: &str,
    ) -> paykit_interactive::Result<Option<PaykitReceipt>> {
        Ok(None)
    }
    async fn save_private_endpoint(
        &self,
        _peer: &PublicKey,
        _method: &MethodId,
        _endpoint: &str,
    ) -> paykit_interactive::Result<()> {
        Ok(())
    }
    async fn get_private_endpoint(
        &self,
        _peer: &PublicKey,
        _method: &MethodId,
    ) -> paykit_interactive::Result<Option<String>> {
        Ok(None)
    }
    async fn list_receipts(&self) -> paykit_interactive::Result<Vec<PaykitReceipt>> {
        Ok(Vec::new())
    }
    async fn list_private_endpoints_for_peer(
        &self,
        _peer: &PublicKey,
    ) -> paykit_interactive::Result<Vec<(MethodId, String)>> {
        Ok(Vec::new())
    }
    async fn remove_private_endpoint(
        &self,
        _peer: &PublicKey,
        _method: &MethodId,
    ) -> paykit_interactive::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl ReceiptGenerator for MockGenerator {
    async fn generate_receipt(
        &self,
        request: &PaykitReceipt,
    ) -> paykit_interactive::Result<PaykitReceipt> {
        Ok(request.clone())
    }
}

#[async_trait::async_trait]
impl PaykitNoiseChannel for MockChannel {
    async fn send(&mut self, msg: PaykitNoiseMessage) -> paykit_interactive::Result<()> {
        self.messages.push(msg);
        Ok(())
    }
    async fn recv(&mut self) -> paykit_interactive::Result<PaykitNoiseMessage> {
        if self.messages.is_empty() {
            Ok(PaykitNoiseMessage::Ack)
        } else {
            Ok(self.messages.remove(0))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Paykit P2P Payment Example ===\n");

    // Setup two parties
    let alice_key = PublicKey::from_str("alice_pubkey_123").unwrap();
    let bob_key = PublicKey::from_str("bob_pubkey_456").unwrap();

    println!("Alice: {:?}", alice_key);
    println!("Bob: {:?}\n", bob_key);

    // Initialize managers
    let alice_storage = Arc::new(Box::new(MockStorage) as Box<dyn PaykitStorage>);
    let alice_generator = Arc::new(Box::new(MockGenerator) as Box<dyn ReceiptGenerator>);
    let alice_manager = PaykitInteractiveManager::new(alice_storage, alice_generator);

    let bob_storage = Arc::new(Box::new(MockStorage) as Box<dyn PaykitStorage>);
    let bob_generator = Arc::new(Box::new(MockGenerator) as Box<dyn ReceiptGenerator>);
    let bob_manager = PaykitInteractiveManager::new(bob_storage, bob_generator);

    // Setup private endpoint storage
    let alice_endpoint_store = InMemoryStore::new();
    let alice_endpoint_manager = PrivateEndpointManager::new(alice_endpoint_store);

    let bob_endpoint_store = InMemoryStore::new();
    let bob_endpoint_manager = PrivateEndpointManager::new(bob_endpoint_store);

    // Alice offers a private endpoint to Bob
    println!("Step 1: Alice offers private Lightning endpoint to Bob");
    let alice_endpoint = paykit_lib::EndpointData("lnbc1u1p3alice123...".to_string());
    bob_endpoint_manager
        .store_endpoint(
            alice_key.clone(),
            MethodId("lightning".to_string()),
            alice_endpoint.clone(),
            Some(chrono::Utc::now().timestamp() + 3600),
        )
        .await
        .map_err(|e| format!("Failed to store endpoint: {}", e))?;
    println!("  ✓ Private endpoint stored\n");

    // Bob initiates payment to Alice
    println!("Step 2: Bob initiates payment to Alice");
    let mut bob_channel = MockChannel {
        messages: Vec::new(),
    };

    // In a real scenario, Bob would use the private endpoint
    println!("  Bob uses private endpoint for payment");
    println!("  Payment method: lightning");
    println!("  Amount: 1000 SAT\n");

    // Exchange receipt
    println!("Step 3: Receipt exchange");
    let receipt = PaykitReceipt::new(
        "receipt_123".to_string(),
        bob_key.clone(),
        alice_key.clone(),
        MethodId("lightning".to_string()),
        Some("1000".to_string()),
        Some("SAT".to_string()),
        serde_json::json!({}),
    );
    println!("  Receipt ID: {}", receipt.receipt_id);
    println!("  Payer: {:?}", receipt.payer);
    println!("  Payee: {:?}", receipt.payee);
    println!("  Amount: {} {}", receipt.amount.unwrap(), receipt.currency.unwrap());

    println!("\n=== Example Complete ===");
    Ok(())
}

use std::str::FromStr;
