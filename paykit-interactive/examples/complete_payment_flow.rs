//! Complete Payment Flow Example
//!
//! This example demonstrates a full payment negotiation between a payer and payee
//! using the interactive Paykit protocol over Noise-encrypted channels.

use paykit_interactive::{
    PaykitInteractiveManager, PaykitNoiseChannel, PaykitNoiseMessage, PaykitReceipt, PaykitStorage,
    ReceiptGenerator,
};
use paykit_lib::{MethodId, PublicKey};
use serde_json::json;
use std::sync::Arc;

// Import mock implementations for the example
#[path = "../tests/mock_implementations.rs"]
mod mock_implementations;
use mock_implementations::{MockNoiseChannel, MockReceiptGenerator, MockStorage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Paykit Interactive Payment Flow Example ===\n");

    // Step 1: Setup identities
    println!("Step 1: Setting up payer and payee identities...");
    let payer_pk = create_test_pubkey("payer_pubkey_abc123");
    let payee_pk = create_test_pubkey("payee_pubkey_xyz789");
    println!("  Payer: {:?}", payer_pk);
    println!("  Payee: {:?}\n", payee_pk);

    // Step 2: Initialize managers
    println!("Step 2: Initializing Paykit managers...");
    let payer_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payer_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payer_manager = PaykitInteractiveManager::new(payer_storage.clone(), payer_generator);

    let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payee_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payee_manager = PaykitInteractiveManager::new(payee_storage.clone(), payee_generator);
    println!("  ✓ Managers initialized\n");

    // Step 3: Establish encrypted channel
    println!("Step 3: Establishing Noise-encrypted channel...");
    let (mut payer_channel, mut payee_channel) = MockNoiseChannel::pair();
    println!("  ✓ Secure channel established\n");

    // Step 4: (Optional) Offer private endpoint
    println!("Step 4: Payee offers private Lightning endpoint...");
    payee_manager
        .offer_private_endpoint(
            &mut payee_channel,
            MethodId("lightning".to_string()),
            "lnbc10000n1...".to_string(), // Private invoice
        )
        .await?;

    // Payer receives and stores the offer
    if let PaykitNoiseMessage::OfferPrivateEndpoint {
        method_id,
        endpoint,
    } = payer_channel.recv().await?
    {
        println!("  Received offer for method: {}", method_id.0);
        println!("  Endpoint: {}...\n", &endpoint[..20]);
        payer_storage
            .save_private_endpoint(&payee_pk, &method_id, &endpoint)
            .await?;
    }

    // Step 5: Create provisional receipt
    println!("Step 5: Payer creates provisional receipt...");
    let provisional_receipt = PaykitReceipt::new(
        format!("receipt_{}", chrono::Utc::now().timestamp()),
        payer_pk.clone(),
        payee_pk.clone(),
        MethodId("lightning".to_string()),
        Some("1000".to_string()),
        Some("SAT".to_string()),
        json!({
            "order_id": "ORDER-2025-001",
            "description": "Premium Widget",
            "shipping": {
                "address": "123 Main St",
                "city": "Bitcoinville"
            }
        }),
    );
    println!("  Receipt ID: {}", provisional_receipt.receipt_id);
    println!(
        "  Amount: {} {}",
        provisional_receipt.amount.as_ref().unwrap(),
        provisional_receipt.currency.as_ref().unwrap()
    );
    println!("  Metadata: {}\n", provisional_receipt.metadata);

    // Step 6: Spawn payee handler
    println!("Step 6: Starting payment negotiation...");
    let payee_pk_clone = payee_pk.clone();
    let payer_pk_clone = payer_pk.clone();
    let payee_handle = tokio::spawn(async move {
        // Payee receives request
        let msg = payee_channel.recv().await.unwrap();
        println!("  [Payee] Received payment request");

        // Process and generate invoice
        let response = payee_manager
            .handle_message(msg, &payer_pk_clone, &payee_pk_clone)
            .await
            .unwrap();

        if let Some(response_msg) = response {
            println!("  [Payee] Sending confirmation with invoice");
            payee_channel.send(response_msg).await.unwrap();
        }
    });

    // Step 7: Payer initiates payment
    let final_receipt = payer_manager
        .initiate_payment(&mut payer_channel, provisional_receipt)
        .await?;

    payee_handle.await?;
    println!("  ✓ Negotiation complete\n");

    // Step 8: Display final receipt
    println!("Step 8: Final receipt details:");
    println!("  Receipt ID: {}", final_receipt.receipt_id);
    println!("  Payer: {:?}", final_receipt.payer);
    println!("  Payee: {:?}", final_receipt.payee);
    println!("  Method: {}", final_receipt.method_id.0);
    println!(
        "  Amount: {} {}",
        final_receipt.amount.as_ref().unwrap(),
        final_receipt.currency.as_ref().unwrap()
    );
    println!("  Created: {}", final_receipt.created_at);
    println!("  Metadata: {}", final_receipt.metadata);

    // Step 9: Verify receipt is stored
    println!("\nStep 9: Verifying receipt persistence...");
    let stored_receipt = payer_storage.get_receipt(&final_receipt.receipt_id).await?;

    if let Some(receipt) = stored_receipt {
        println!("  ✓ Receipt successfully stored");
        println!("  ✓ Invoice: {}", receipt.metadata.get("invoice").unwrap());
    }

    println!("\n=== Payment Flow Complete ===");
    Ok(())
}

fn create_test_pubkey(_s: &str) -> PublicKey {
    // Generate a random valid key using pubky
    let keypair = pubky::Keypair::random();
    keypair.public_key()
}

// Additional helper for displaying timestamps
mod chrono {
    pub struct Utc;
    impl Utc {
        pub fn now() -> Timestamp {
            Timestamp
        }
    }
    pub struct Timestamp;
    impl Timestamp {
        pub fn timestamp(&self) -> i64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        }
    }
}
