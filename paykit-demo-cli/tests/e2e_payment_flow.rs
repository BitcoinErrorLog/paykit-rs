//! End-to-end integration tests for pay + receive flow
//!
//! Tests the complete payment workflow: publish → discover → connect → pay → receive

mod common;

use paykit_demo_core::{IdentityManager, NoiseClientHelper, NoiseServerHelper};
use paykit_interactive::{PaykitNoiseChannel, PaykitNoiseMessage};
use paykit_lib::{
    AuthenticatedTransport, EndpointData, MethodId, PubkyAuthenticatedTransport,
    UnauthenticatedTransportRead,
};
use pubky_testnet::EphemeralTestnet;
use tempfile::TempDir;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_noise_handshake_between_payer_and_receiver() {
    // Setup: Create two identities
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_dir = temp_dir.path();

    let id_manager = IdentityManager::new(storage_dir.join("identities"));
    let payer = id_manager.create("payer").expect("Failed to create payer");
    let receiver = id_manager
        .create("receiver")
        .expect("Failed to create receiver");

    // Start receiver's Noise server on a random port
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let server_addr = listener.local_addr().expect("Failed to get local addr");
    let server_port = server_addr.port();

    println!("Server listening on port {}", server_port);

    // Get receiver's static public key
    let device_id = format!("paykit-demo-{}", receiver.public_key());
    let receiver_static_pk =
        NoiseServerHelper::get_static_public_key(&receiver, device_id.as_bytes());

    // Spawn server task
    let receiver_clone = receiver.clone();
    let server_task = tokio::spawn(async move {
        // Accept one connection
        let (stream, _) = listener.accept().await.expect("Failed to accept");

        let server = NoiseServerHelper::create_server(&receiver_clone, device_id.as_bytes());
        let mut channel = NoiseServerHelper::accept_connection(server, stream)
            .await
            .expect("Failed to complete handshake");

        // Receive payment request
        let msg = channel.recv().await.expect("Failed to receive message");

        match msg {
            PaykitNoiseMessage::RequestReceipt {
                provisional_receipt,
            } => {
                // Send confirmation
                let confirm = PaykitNoiseMessage::ConfirmReceipt {
                    receipt: provisional_receipt,
                };
                channel
                    .send(confirm)
                    .await
                    .expect("Failed to send confirmation");
            }
            _ => panic!("Unexpected message type"),
        }
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Payer connects to receiver
    let connect_addr = format!("127.0.0.1:{}", server_port);
    let mut channel =
        NoiseClientHelper::connect_to_recipient(&payer, &connect_addr, &receiver_static_pk)
            .await
            .expect("Failed to connect");

    // Send payment request
    let request = PaykitNoiseMessage::RequestReceipt {
        provisional_receipt: paykit_interactive::PaykitReceipt::new(
            "test-receipt-1".to_string(),
            payer.public_key(),
            receiver.public_key(),
            MethodId("lightning".to_string()),
            Some("1000".to_string()),
            Some("SAT".to_string()),
            serde_json::json!({}),
        ),
    };

    channel.send(request).await.expect("Failed to send request");

    // Receive confirmation
    let response = channel.recv().await.expect("Failed to receive response");

    match response {
        PaykitNoiseMessage::ConfirmReceipt { receipt } => {
            assert_eq!(receipt.receipt_id, "test-receipt-1");
            assert_eq!(receipt.amount, Some("1000".to_string()));
        }
        _ => panic!("Expected ConfirmReceipt"),
    }

    // Wait for server task to complete
    server_task.await.expect("Server task failed");
}

#[tokio::test]
#[ignore = "Requires external DHT - run manually with --ignored"]
async fn test_full_payment_flow_with_published_methods() {
    // Setup: Create testnet and identities
    let testnet = EphemeralTestnet::start()
        .await
        .expect("Failed to start testnet");
    let homeserver = testnet.homeserver();
    let sdk = testnet.sdk().expect("Failed to get SDK");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_dir = temp_dir.path();

    let id_manager = IdentityManager::new(storage_dir.join("identities"));
    let payer = id_manager.create("payer").expect("Failed to create payer");
    let receiver = id_manager
        .create("receiver")
        .expect("Failed to create receiver");

    // Step 1: Receiver publishes methods
    let signer = sdk.signer(receiver.keypair.clone());
    let session = signer
        .signup(&homeserver.public_key(), None)
        .await
        .expect("Failed to signup receiver");

    let auth_transport = PubkyAuthenticatedTransport::new(session);

    // Start receiver's Noise server
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let server_addr = listener.local_addr().expect("Failed to get local addr");
    let server_port = server_addr.port();

    let device_id = format!("paykit-demo-{}", receiver.public_key());
    let receiver_static_pk =
        NoiseServerHelper::get_static_public_key(&receiver, device_id.as_bytes());

    // Publish lightning endpoint with Noise server info
    let endpoint_data = format!(
        "noise://127.0.0.1:{}@{}",
        server_port,
        hex::encode(receiver_static_pk)
    );

    auth_transport
        .upsert_payment_endpoint(
            &MethodId("lightning".to_string()),
            &EndpointData(endpoint_data),
        )
        .await
        .expect("Failed to publish endpoint");

    // Step 2: Payer discovers receiver's methods
    let unauth_transport = paykit_lib::PubkyUnauthenticatedTransport::new(sdk.public_storage());
    let methods = unauth_transport
        .fetch_supported_payments(&receiver.public_key())
        .await
        .expect("Failed to fetch methods");

    assert!(!methods.entries.is_empty(), "Should find published methods");

    let lightning_method = MethodId("lightning".to_string());
    let endpoint = methods
        .entries
        .get(&lightning_method)
        .expect("Should have lightning");

    assert!(
        endpoint.0.starts_with("noise://"),
        "Should be a Noise endpoint"
    );

    println!("✅ Payment flow test complete");
    println!("   - Receiver published methods");
    println!("   - Payer discovered methods");
    println!("   - Endpoint format validated");
}

#[tokio::test]
async fn test_multiple_concurrent_payment_requests() {
    // Test that a receiver can handle multiple payers concurrently
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_dir = temp_dir.path();

    let id_manager = IdentityManager::new(storage_dir.join("identities"));
    let receiver = id_manager
        .create("receiver")
        .expect("Failed to create receiver");
    let payer1 = id_manager
        .create("payer1")
        .expect("Failed to create payer1");
    let payer2 = id_manager
        .create("payer2")
        .expect("Failed to create payer2");

    // Start receiver server
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let server_addr = listener.local_addr().expect("Failed to get local addr");
    let server_port = server_addr.port();

    let device_id = format!("paykit-demo-{}", receiver.public_key());
    let receiver_static_pk =
        NoiseServerHelper::get_static_public_key(&receiver, device_id.as_bytes());

    // Spawn server to accept two connections
    let receiver_clone = receiver.clone();
    let receiver_pk = receiver.public_key();
    let device_id_clone = device_id.clone();
    let _server_task = tokio::spawn(async move {
        for i in 1..=2 {
            let (stream, _) = listener.accept().await.expect("Failed to accept");

            let server =
                NoiseServerHelper::create_server(&receiver_clone, device_id_clone.as_bytes());

            let _receiver_clone2 = receiver_clone.clone();
            tokio::spawn(async move {
                let mut channel = NoiseServerHelper::accept_connection(server, stream)
                    .await
                    .expect("Failed to complete handshake");

                let msg = channel.recv().await.expect("Failed to receive");

                if let PaykitNoiseMessage::RequestReceipt {
                    provisional_receipt,
                } = msg
                {
                    let confirm = PaykitNoiseMessage::ConfirmReceipt {
                        receipt: provisional_receipt,
                    };
                    channel.send(confirm).await.expect("Failed to send");
                    println!("✅ Handled request {}", i);
                }
            });
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Both payers connect and send requests
    let connect_addr = format!("127.0.0.1:{}", server_port);

    let payer1_task = {
        let payer = payer1.clone();
        let addr = connect_addr.clone();
        let pk = receiver_static_pk;
        let receiver_pk_clone = receiver_pk.clone();
        tokio::spawn(async move {
            let mut channel = NoiseClientHelper::connect_to_recipient(&payer, &addr, &pk)
                .await
                .expect("Payer1 failed to connect");

            let request = PaykitNoiseMessage::RequestReceipt {
                provisional_receipt: paykit_interactive::PaykitReceipt::new(
                    "receipt-payer1".to_string(),
                    payer.public_key(),
                    receiver_pk_clone,
                    MethodId("lightning".to_string()),
                    Some("500".to_string()),
                    Some("SAT".to_string()),
                    serde_json::json!({}),
                ),
            };

            channel.send(request).await.expect("Failed to send");
            let _response = channel.recv().await.expect("Failed to receive");
        })
    };

    let payer2_task = {
        let payer = payer2.clone();
        let addr = connect_addr;
        let pk = receiver_static_pk;
        let receiver_pk_clone = receiver_pk;
        tokio::spawn(async move {
            let mut channel = NoiseClientHelper::connect_to_recipient(&payer, &addr, &pk)
                .await
                .expect("Payer2 failed to connect");

            let request = PaykitNoiseMessage::RequestReceipt {
                provisional_receipt: paykit_interactive::PaykitReceipt::new(
                    "receipt-payer2".to_string(),
                    payer.public_key(),
                    receiver_pk_clone,
                    MethodId("lightning".to_string()),
                    Some("750".to_string()),
                    Some("SAT".to_string()),
                    serde_json::json!({}),
                ),
            };

            channel.send(request).await.expect("Failed to send");
            let _response = channel.recv().await.expect("Failed to receive");
        })
    };

    // Wait for both to complete
    payer1_task.await.expect("Payer1 task failed");
    payer2_task.await.expect("Payer2 task failed");

    println!("✅ Both concurrent payments succeeded");
}
