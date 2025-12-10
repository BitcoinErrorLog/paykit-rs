mod mock_implementations;

use mock_implementations::{MockNoiseChannel, MockReceiptGenerator, MockStorage};
use paykit_interactive::{
    PaykitInteractiveManager, PaykitNoiseChannel, PaykitNoiseMessage, PaykitReceipt,
};
use paykit_lib::{MethodId, PublicKey};
use serde_json::json;
use std::sync::Arc;

// Helper to create a test PublicKey
fn test_pubkey(_s: &str) -> PublicKey {
    // Generate a random valid key using pubky
    let keypair = pubky::Keypair::random();
    keypair.public_key()
}

#[tokio::test]
async fn test_receipt_negotiation_success() {
    // Setup
    let payer_pk = test_pubkey("payer");
    let payee_pk = test_pubkey("payee");

    let payer_storage =
        Arc::new(Box::new(MockStorage::new()) as Box<dyn paykit_interactive::PaykitStorage>);
    let payer_generator = Arc::new(
        Box::new(MockReceiptGenerator::new()) as Box<dyn paykit_interactive::ReceiptGenerator>
    );
    let payer_manager = PaykitInteractiveManager::new(payer_storage.clone(), payer_generator);

    let payee_storage =
        Arc::new(Box::new(MockStorage::new()) as Box<dyn paykit_interactive::PaykitStorage>);
    let payee_generator = Arc::new(
        Box::new(MockReceiptGenerator::new()) as Box<dyn paykit_interactive::ReceiptGenerator>
    );
    let payee_manager = PaykitInteractiveManager::new(payee_storage.clone(), payee_generator);

    let (mut payer_channel, mut payee_channel) = MockNoiseChannel::pair();

    // Create provisional receipt
    let provisional_receipt = PaykitReceipt::new(
        "receipt_123".to_string(),
        payer_pk.clone(),
        payee_pk.clone(),
        MethodId("lightning".to_string()),
        Some("1000".to_string()),
        Some("SAT".to_string()),
        json!({"order_id": "ABC123"}),
    );

    // Spawn payee to handle incoming request
    let payer_pk_clone = payer_pk.clone();
    let payee_pk_clone = payee_pk.clone();
    let payee_handle = tokio::spawn(async move {
        let msg = payee_channel.recv().await.unwrap();
        let response = payee_manager
            .handle_message(msg, &payer_pk_clone, &payee_pk_clone)
            .await
            .unwrap();
        if let Some(response_msg) = response {
            payee_channel.send(response_msg).await.unwrap();
        }
    });

    // Payer initiates payment
    let final_receipt = payer_manager
        .initiate_payment(&mut payer_channel, provisional_receipt.clone())
        .await
        .unwrap();

    // Wait for payee to finish
    payee_handle.await.unwrap();

    // Verify receipt
    assert_eq!(final_receipt.receipt_id, provisional_receipt.receipt_id);
    assert_eq!(final_receipt.payer, payer_pk);
    assert_eq!(final_receipt.payee, payee_pk);
    assert!(final_receipt.metadata.get("invoice").is_some());

    // Verify receipt was saved on payer side
    let saved_receipt = payer_storage
        .get_receipt(&final_receipt.receipt_id)
        .await
        .unwrap();
    assert!(saved_receipt.is_some());
}

#[tokio::test]
async fn test_private_endpoint_offer() {
    let peer_pk = test_pubkey("peer");
    let my_pk = test_pubkey("me");

    let storage =
        Arc::new(Box::new(MockStorage::new()) as Box<dyn paykit_interactive::PaykitStorage>);
    let generator = Arc::new(
        Box::new(MockReceiptGenerator::new()) as Box<dyn paykit_interactive::ReceiptGenerator>
    );
    let manager = PaykitInteractiveManager::new(storage.clone(), generator);

    // Create offer message
    let offer_msg = PaykitNoiseMessage::OfferPrivateEndpoint {
        method_id: MethodId("lightning".to_string()),
        endpoint: "lnbc1000...".to_string(),
    };

    // Handle the offer
    let response = manager
        .handle_message(offer_msg, &peer_pk, &my_pk)
        .await
        .unwrap();

    // Should get Ack response
    assert!(matches!(response, Some(PaykitNoiseMessage::Ack)));

    // Verify endpoint was saved
    let saved_endpoint = storage
        .get_private_endpoint(&peer_pk, &MethodId("lightning".to_string()))
        .await
        .unwrap();
    assert_eq!(saved_endpoint, Some("lnbc1000...".to_string()));
}

#[tokio::test]
async fn test_wrong_payee_error() {
    let payer_pk = test_pubkey("payer");
    let payee_pk = test_pubkey("payee");
    let wrong_pk = test_pubkey("wrong");

    let storage =
        Arc::new(Box::new(MockStorage::new()) as Box<dyn paykit_interactive::PaykitStorage>);
    let generator = Arc::new(
        Box::new(MockReceiptGenerator::new()) as Box<dyn paykit_interactive::ReceiptGenerator>
    );
    let manager = PaykitInteractiveManager::new(storage, generator);

    // Create receipt for different payee
    let receipt = PaykitReceipt::new(
        "receipt_123".to_string(),
        payer_pk.clone(),
        payee_pk.clone(), // Receipt is for payee_pk
        MethodId("lightning".to_string()),
        Some("1000".to_string()),
        Some("SAT".to_string()),
        json!({}),
    );

    let request_msg = PaykitNoiseMessage::RequestReceipt {
        provisional_receipt: receipt,
    };

    // Handle as wrong_pk (not the intended payee)
    let response = manager
        .handle_message(request_msg, &payer_pk, &wrong_pk)
        .await
        .unwrap();

    // Should get error response
    match response {
        Some(PaykitNoiseMessage::Error { code, .. }) => {
            assert_eq!(code, "WRONG_PAYEE");
        }
        _ => panic!("Expected error response"),
    }
}

#[tokio::test]
async fn test_offer_private_endpoint_api() {
    let (mut channel1, mut channel2) = MockNoiseChannel::pair();

    let storage =
        Arc::new(Box::new(MockStorage::new()) as Box<dyn paykit_interactive::PaykitStorage>);
    let generator = Arc::new(
        Box::new(MockReceiptGenerator::new()) as Box<dyn paykit_interactive::ReceiptGenerator>
    );
    let manager = PaykitInteractiveManager::new(storage, generator);

    // Spawn receiver
    let handle = tokio::spawn(async move {
        let msg = channel2.recv().await.unwrap();
        assert!(matches!(
            msg,
            PaykitNoiseMessage::OfferPrivateEndpoint { .. }
        ));
    });

    // Send offer
    manager
        .offer_private_endpoint(
            &mut channel1,
            MethodId("onchain".to_string()),
            "bc1q...".to_string(),
        )
        .await
        .unwrap();

    handle.await.unwrap();
}

#[tokio::test]
async fn test_receipt_id_mismatch_error() {
    let payer_pk = test_pubkey("payer");
    let payee_pk = test_pubkey("payee");

    let storage =
        Arc::new(Box::new(MockStorage::new()) as Box<dyn paykit_interactive::PaykitStorage>);
    let generator = Arc::new(
        Box::new(MockReceiptGenerator::new()) as Box<dyn paykit_interactive::ReceiptGenerator>
    );
    let manager = PaykitInteractiveManager::new(storage, generator);

    let (mut payer_channel, mut payee_channel) = MockNoiseChannel::pair();

    let provisional_receipt = PaykitReceipt::new(
        "receipt_123".to_string(),
        payer_pk.clone(),
        payee_pk.clone(),
        MethodId("lightning".to_string()),
        Some("1000".to_string()),
        Some("SAT".to_string()),
        json!({}),
    );

    // Spawn payee that sends wrong receipt ID
    tokio::spawn(async move {
        payee_channel.recv().await.unwrap();
        let wrong_receipt = PaykitReceipt::new(
            "wrong_id".to_string(), // Wrong ID!
            payer_pk,
            payee_pk,
            MethodId("lightning".to_string()),
            Some("1000".to_string()),
            Some("SAT".to_string()),
            json!({}),
        );
        payee_channel
            .send(PaykitNoiseMessage::ConfirmReceipt {
                receipt: wrong_receipt,
            })
            .await
            .unwrap();
    });

    // Should fail with protocol error
    let result = manager
        .initiate_payment(&mut payer_channel, provisional_receipt)
        .await;

    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Receipt ID mismatch"));
    }
}
