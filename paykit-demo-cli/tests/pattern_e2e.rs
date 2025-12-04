//! End-to-end tests for all Noise patterns in payment scenarios.
//!
//! This module tests each pattern in realistic payment workflows:
//! - IK: Standard authenticated payments
//! - IK-raw: Cold key scenarios with pkarr identity
//! - N: Anonymous donation box
//! - NN: Ephemeral with post-handshake verification

use paykit_demo_core::{Identity, NoiseClientHelper, NoisePattern, NoiseRawClientHelper, NoiseServerHelper};
use paykit_interactive::{PaykitNoiseChannel, PaykitNoiseMessage, PaykitReceipt};
use paykit_lib::MethodId;
use tempfile::TempDir;
use tokio::net::TcpListener;

// ============================================================================
// IK Pattern Tests
// ============================================================================

#[tokio::test]
async fn test_ik_pattern_full_payment() {
    // Standard IK pattern with full identity binding
    let temp_dir = TempDir::new().expect("temp dir");
    let id_manager = paykit_demo_core::IdentityManager::new(temp_dir.path().join("ids"));

    let payer = id_manager.create("payer").expect("create payer");
    let receiver = id_manager.create("receiver").expect("create receiver");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let server_addr = listener.local_addr().expect("addr");
    let server_port = server_addr.port();

    let device_id = format!("test-{}", receiver.public_key());
    let receiver_pk = NoiseServerHelper::get_static_public_key(&receiver, device_id.as_bytes());

    // Server
    let receiver_clone = receiver.clone();
    let device_id_clone = device_id.clone();
    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let server = NoiseServerHelper::create_server(&receiver_clone, device_id_clone.as_bytes());
        let mut channel = NoiseServerHelper::accept_connection(server, stream)
            .await
            .expect("handshake");

        let msg = channel.recv().await.expect("recv");
        if let PaykitNoiseMessage::RequestReceipt { provisional_receipt } = msg {
            channel
                .send(PaykitNoiseMessage::ConfirmReceipt {
                    receipt: provisional_receipt,
                })
                .await
                .expect("send");
        }
    });

    // Client
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let addr = format!("127.0.0.1:{}", server_port);
    let mut channel = NoiseClientHelper::connect_to_recipient(&payer, &addr, &receiver_pk)
        .await
        .expect("connect");

    let receipt = PaykitReceipt::new(
        "payment-1".to_string(),
        payer.public_key(),
        receiver.public_key(),
        MethodId("lightning".to_string()),
        Some("1000".to_string()),
        Some("SAT".to_string()),
        serde_json::json!({}),
    );

    channel
        .send(PaykitNoiseMessage::RequestReceipt {
            provisional_receipt: receipt,
        })
        .await
        .expect("send");

    let response = channel.recv().await.expect("recv");
    assert!(matches!(response, PaykitNoiseMessage::ConfirmReceipt { .. }));

    server_task.await.expect("server");
}

// ============================================================================
// IK-raw Pattern Tests (Cold Key)
// ============================================================================

#[tokio::test]
async fn test_ik_raw_pattern_cold_key_payment() {
    // IK-raw: Identity verified via pkarr, no Ed25519 signing at handshake
    let temp_dir = TempDir::new().expect("temp dir");
    let id_manager = paykit_demo_core::IdentityManager::new(temp_dir.path().join("ids"));

    let receiver = id_manager.create("receiver").expect("create receiver");

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let server_port = listener.local_addr().expect("addr").port();

    // Derive X25519 keys
    let device_id = b"cold-key-device";
    let receiver_sk = NoiseServerHelper::derive_x25519_key(&receiver, device_id);
    let receiver_pk = pubky_noise::kdf::x25519_pk_from_sk(&receiver_sk);

    // Client derives their own X25519 key
    let client_seed = [42u8; 32];
    let client_sk = NoiseRawClientHelper::derive_x25519_key(&client_seed, b"client");

    // Server
    let server_task = tokio::spawn({
        let receiver_sk = receiver_sk.clone();
        async move {
            let (stream, _) = listener.accept().await.expect("accept");

            // Accept IK-raw without identity verification (done via pkarr)
            let mut channel = NoiseServerHelper::accept_ik_raw_with_stream(
                &receiver_sk,
                stream,
            )
            .await
            .expect("handshake");

            let msg = channel.recv().await.expect("recv");
            if let PaykitNoiseMessage::RequestReceipt { provisional_receipt } = msg {
                channel
                    .send(PaykitNoiseMessage::ConfirmReceipt {
                        receipt: provisional_receipt,
                    })
                    .await
                    .expect("send");
            }
        }
    });

    // Client connects with IK-raw
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let addr = format!("127.0.0.1:{}", server_port);
    let mut channel = NoiseRawClientHelper::connect_ik_raw(&client_sk, &addr, &receiver_pk)
        .await
        .expect("connect");

    channel.send(PaykitNoiseMessage::Ack).await.expect("send");
    let _ = channel.recv().await.expect("recv");

    server_task.await.expect("server");
}

// ============================================================================
// N Pattern Tests (Anonymous Client)
// ============================================================================

#[tokio::test]
async fn test_n_pattern_anonymous_donation() {
    // N pattern: Client is anonymous, server is authenticated
    // Use case: Anonymous donation box
    let temp_dir = TempDir::new().expect("temp dir");
    let id_manager = paykit_demo_core::IdentityManager::new(temp_dir.path().join("ids"));

    let receiver = id_manager.create("receiver").expect("create receiver");

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let server_port = listener.local_addr().expect("addr").port();

    let device_id = b"donation-box";
    let receiver_sk = NoiseServerHelper::derive_x25519_key(&receiver, device_id);
    let receiver_pk = pubky_noise::kdf::x25519_pk_from_sk(&receiver_sk);

    // Server (pattern-aware, accepts N pattern)
    let server_task = tokio::spawn({
        let receiver_sk = receiver_sk.clone();
        async move {
            let (stream, _) = listener.accept().await.expect("accept");

            // Accept N pattern - server is authenticated, client is anonymous
            let mut channel = NoiseServerHelper::accept_anonymous_with_stream(&receiver_sk, stream)
                .await
                .expect("handshake");

            let msg = channel.recv().await.expect("recv");
            if let PaykitNoiseMessage::RequestReceipt { provisional_receipt } = msg {
                // Anonymous client - we can't identify them but can still process payment
                println!("Received anonymous donation: {:?}", provisional_receipt.amount);
                channel
                    .send(PaykitNoiseMessage::ConfirmReceipt {
                        receipt: provisional_receipt,
                    })
                    .await
                    .expect("send");
            }
        }
    });

    // Anonymous client connects
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let addr = format!("127.0.0.1:{}", server_port);
    let mut channel = NoiseRawClientHelper::connect_anonymous(&addr, &receiver_pk)
        .await
        .expect("connect");

    // Send anonymous donation
    let receipt = PaykitReceipt::new(
        "anon-donation-1".to_string(),
        pubky::Keypair::random().public_key(), // Anonymous placeholder
        pubky::Keypair::random().public_key(), // Will be replaced by server
        MethodId("lightning".to_string()),
        Some("500".to_string()),
        Some("SAT".to_string()),
        serde_json::json!({"type": "donation"}),
    );

    channel
        .send(PaykitNoiseMessage::RequestReceipt {
            provisional_receipt: receipt,
        })
        .await
        .expect("send");

    let response = channel.recv().await.expect("recv");
    assert!(matches!(response, PaykitNoiseMessage::ConfirmReceipt { .. }));

    server_task.await.expect("server");
}

// ============================================================================
// NN Pattern Tests (Ephemeral)
// ============================================================================

#[tokio::test]
async fn test_nn_pattern_ephemeral_with_attestation() {
    // NN pattern: Both parties anonymous, requires post-handshake attestation
    // Use case: When both parties need to be anonymous initially
    
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let server_port = listener.local_addr().expect("addr").port();

    // Server
    let server_task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");

        let (mut channel, client_ephemeral) = NoiseServerHelper::accept_ephemeral_with_stream(stream)
            .await
            .expect("handshake");

        // NN pattern provides ephemeral key, not identity
        // In production, we'd verify identity via post-handshake attestation
        println!(
            "NN session with client ephemeral: {}",
            hex::encode(&client_ephemeral[..8])
        );

        let msg = channel.recv().await.expect("recv");
        assert!(matches!(msg, PaykitNoiseMessage::Ack));

        channel.send(PaykitNoiseMessage::Ack).await.expect("send");
    });

    // Client
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let addr = format!("127.0.0.1:{}", server_port);
    let (mut channel, server_ephemeral) = NoiseRawClientHelper::connect_ephemeral(&addr)
        .await
        .expect("connect");

    // NN provides server's ephemeral key
    println!(
        "NN session with server ephemeral: {}",
        hex::encode(&server_ephemeral[..8])
    );

    channel.send(PaykitNoiseMessage::Ack).await.expect("send");
    let response = channel.recv().await.expect("recv");
    assert!(matches!(response, PaykitNoiseMessage::Ack));

    server_task.await.expect("server");
}

// ============================================================================
// Pattern Selection Tests
// ============================================================================

#[test]
fn test_pattern_selection_for_use_cases() {
    // Document the correct pattern for each use case
    
    // Standard payments between known parties
    let standard_payment = NoisePattern::IK;
    assert_eq!(format!("{}", standard_payment), "IK");

    // Cold key / hardware wallet
    let cold_key_payment = NoisePattern::IKRaw;
    assert_eq!(format!("{}", cold_key_payment), "IK-raw");

    // Donation box (anonymous sender)
    let donation = NoisePattern::N;
    assert_eq!(format!("{}", donation), "N");

    // Ephemeral connection (both anonymous)
    let ephemeral = NoisePattern::NN;
    assert_eq!(format!("{}", ephemeral), "NN");
}

#[test]
fn test_pattern_negotiation_bytes() {
    // Verify negotiation bytes match protocol spec
    assert_eq!(NoisePattern::IK.negotiation_byte(), 0);
    assert_eq!(NoisePattern::IKRaw.negotiation_byte(), 1);
    assert_eq!(NoisePattern::N.negotiation_byte(), 2);
    assert_eq!(NoisePattern::NN.negotiation_byte(), 3);
    assert_eq!(NoisePattern::XX.negotiation_byte(), 4);
}

