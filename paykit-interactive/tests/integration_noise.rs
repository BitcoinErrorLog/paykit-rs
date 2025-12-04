//! Integration tests for real Noise protocol handshakes and encrypted channels
//!
//! These tests verify that pubky-noise integration works correctly with actual
//! TCP connections and Noise_IK handshakes (not mocks).

use paykit_interactive::{
    transport::PubkyNoiseChannel, PaykitInteractiveManager, PaykitNoiseChannel, PaykitNoiseMessage,
    PaykitReceipt, PaykitStorage, ReceiptGenerator,
};
use paykit_lib::MethodId;
use pubky_noise::ring::RingKeyProvider;
use pubky_noise::{DummyRing, NoiseClient, NoiseServer};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;


// Use mock implementations for storage and receipt generation
#[path = "mock_implementations.rs"]
mod mock_implementations;
use mock_implementations::{MockReceiptGenerator, MockStorage};

/// Shared seed for all test keys - ensures client and server derive compatible keys
const TEST_SEED: [u8; 32] = [42u8; 32];
const SERVER_DEVICE_ID: &[u8] = b"server_device";
const CLIENT_DEVICE_ID: &[u8] = b"client_device";

/// Helper to get the server's public key (derived from the shared seed)
fn get_server_public_key() -> [u8; 32] {
    let ring = DummyRing::new(TEST_SEED, "server_kid");
    let x_sk = ring
        .derive_device_x25519("server_kid", SERVER_DEVICE_ID)
        .unwrap();
    pubky_noise::kdf::x25519_pk_from_sk(&x_sk)
}

/// Helper to create test public keys
fn create_test_pubkey(_name: &str) -> pubky::PublicKey {
    // Generate a valid keypair and use its public key
    let keypair = pubky::Keypair::random();
    keypair.public_key()
}

#[tokio::test]
async fn test_noise_client_server_handshake() {
    // Test 1: Real Noise_IK handshake over TCP

    // Setup server with shared seed
    let server_pk = get_server_public_key();
    let server_ring = Arc::new(DummyRing::new(TEST_SEED, "server_kid"));
    let server = NoiseServer::<DummyRing>::new_direct("server_kid", SERVER_DEVICE_ID, server_ring);

    // Start TCP server
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let server_addr = listener.local_addr().expect("Failed to get address");

    // Spawn server task
    let server_handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("Failed to accept");

        // Read handshake message
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .expect("Failed to read length");
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut handshake_msg = vec![0u8; len];
        stream
            .read_exact(&mut handshake_msg)
            .await
            .expect("Failed to read handshake");

        // Process handshake - Step 2: Server processes client's message and sends response
        let (mut hs_state, _identity) = server
            .build_responder_read_ik(&handshake_msg)
            .expect("Handshake failed");

        // Generate response message
        let mut response_msg = vec![0u8; 128];
        let n = hs_state
            .write_message(&[], &mut response_msg)
            .expect("Write failed");
        response_msg.truncate(n);

        // Send handshake response
        let len = (response_msg.len() as u32).to_be_bytes();
        stream.write_all(&len).await.expect("Write failed");
        stream.write_all(&response_msg).await.expect("Write failed");

        // Step 3: Complete handshake to get transport mode
        let mut link = pubky_noise::datalink_adapter::server_complete_ik(hs_state)
            .expect("Failed to complete handshake");

        // Send encrypted test message
        let test_msg = b"Hello from server!";
        let ciphertext = link.encrypt(test_msg).expect("Encryption failed");
        let len = (ciphertext.len() as u32).to_be_bytes();
        stream.write_all(&len).await.expect("Write failed");
        stream.write_all(&ciphertext).await.expect("Write failed");

        // Read encrypted response
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .expect("Failed to read length");
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut ciphertext = vec![0u8; len];
        stream
            .read_exact(&mut ciphertext)
            .await
            .expect("Failed to read ciphertext");

        let plaintext = link.decrypt(&ciphertext).expect("Decryption failed");
        assert_eq!(&plaintext, b"Hello from client!");
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Setup client with shared seed
    let client_ring = Arc::new(DummyRing::new(TEST_SEED, "client_kid"));
    let client = NoiseClient::<DummyRing>::new_direct("client_kid", CLIENT_DEVICE_ID, client_ring);

    // Connect and perform handshake
    let mut stream = TcpStream::connect(server_addr)
        .await
        .expect("Failed to connect");

    // Step 1: Client starts handshake
    let (hs_state, handshake_msg) =
        pubky_noise::datalink_adapter::client_start_ik_direct(&client, &server_pk)
            .expect("Handshake build failed");

    // Send handshake
    let len = (handshake_msg.len() as u32).to_be_bytes();
    stream.write_all(&len).await.expect("Write failed");
    stream
        .write_all(&handshake_msg)
        .await
        .expect("Write failed");

    // Step 2: Read server's response
    let mut len_bytes = [0u8; 4];
    stream
        .read_exact(&mut len_bytes)
        .await
        .expect("Failed to read response length");
    let len = u32::from_be_bytes(len_bytes) as usize;

    let mut response_msg = vec![0u8; len];
    stream
        .read_exact(&mut response_msg)
        .await
        .expect("Failed to read response");

    // Step 3: Complete handshake to get transport mode
    let mut link = pubky_noise::datalink_adapter::client_complete_ik(hs_state, &response_msg)
        .expect("Failed to complete handshake");

    // Read encrypted message from server
    let mut len_bytes = [0u8; 4];
    stream
        .read_exact(&mut len_bytes)
        .await
        .expect("Failed to read length");
    let len = u32::from_be_bytes(len_bytes) as usize;

    let mut ciphertext = vec![0u8; len];
    stream
        .read_exact(&mut ciphertext)
        .await
        .expect("Failed to read ciphertext");

    let plaintext = link.decrypt(&ciphertext).expect("Decryption failed");
    assert_eq!(&plaintext, b"Hello from server!");

    // Send encrypted response
    let test_msg = b"Hello from client!";
    let ciphertext = link.encrypt(test_msg).expect("Encryption failed");
    let len = (ciphertext.len() as u32).to_be_bytes();
    stream.write_all(&len).await.expect("Write failed");
    stream.write_all(&ciphertext).await.expect("Write failed");

    // Wait for server to finish
    timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("Server timeout")
        .expect("Server panicked");

    println!("✅ Real Noise_IK handshake test passed");
}

#[tokio::test]
async fn test_pubky_noise_channel_real() {
    // Test 2: PubkyNoiseChannel with real transport

    // Setup server with shared seed
    let server_pk = get_server_public_key();
    let server_ring = Arc::new(DummyRing::new(TEST_SEED, "server_kid"));
    let server = NoiseServer::<DummyRing>::new_direct("server_kid", SERVER_DEVICE_ID, server_ring);

    // Start TCP server
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let server_addr = listener.local_addr().expect("Failed to get address");

    // Spawn server task
    let server_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("Failed to accept");

        // Read handshake message
        let mut len_bytes = [0u8; 4];
        let (mut reader, mut writer) = tokio::io::split(stream);
        reader
            .read_exact(&mut len_bytes)
            .await
            .expect("Failed to read length");
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut handshake_msg = vec![0u8; len];
        reader
            .read_exact(&mut handshake_msg)
            .await
            .expect("Failed to read handshake");

        // Process handshake - Step 2: Server processes client's message and sends response
        let (mut hs_state, _identity) = server
            .build_responder_read_ik(&handshake_msg)
            .expect("Handshake failed");

        // Generate response message
        let mut response_msg = vec![0u8; 128];
        let n = hs_state
            .write_message(&[], &mut response_msg)
            .expect("Write failed");
        response_msg.truncate(n);

        // Send handshake response
        writer
            .write_all(&(response_msg.len() as u32).to_be_bytes())
            .await
            .expect("Write failed");
        writer.write_all(&response_msg).await.expect("Write failed");

        // Step 3: Complete handshake to get transport mode
        let link = pubky_noise::datalink_adapter::server_complete_ik(hs_state)
            .expect("Failed to complete handshake");

        // Reunite stream and create channel
        let stream = reader.unsplit(writer);
        let mut channel = PubkyNoiseChannel::new(stream, link);

        // Receive PaykitNoiseMessage
        let msg = channel.recv().await.expect("Failed to receive message");
        match msg {
            PaykitNoiseMessage::OfferPrivateEndpoint {
                method_id,
                endpoint,
            } => {
                assert_eq!(method_id.0, "lightning");
                assert_eq!(endpoint, "lnbc10000n1...");
            }
            _ => panic!("Unexpected message type"),
        }

        // Send response
        channel
            .send(PaykitNoiseMessage::Ack)
            .await
            .expect("Failed to send response");
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Setup client with shared seed
    let client_ring = Arc::new(DummyRing::new(TEST_SEED, "client_kid"));
    let client = NoiseClient::<DummyRing>::new_direct("client_kid", CLIENT_DEVICE_ID, client_ring);

    // Connect using PubkyNoiseChannel::connect()
    let stream = TcpStream::connect(server_addr)
        .await
        .expect("Failed to connect");

    let mut channel = PubkyNoiseChannel::connect(&client, stream, &server_pk)
        .await
        .expect("Failed to connect channel");

    // Send PaykitNoiseMessage
    channel
        .send(PaykitNoiseMessage::OfferPrivateEndpoint {
            method_id: MethodId("lightning".to_string()),
            endpoint: "lnbc10000n1...".to_string(),
        })
        .await
        .expect("Failed to send message");

    // Receive response
    let msg = channel.recv().await.expect("Failed to receive response");
    assert!(matches!(msg, PaykitNoiseMessage::Ack));

    // Wait for server to finish
    timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("Server timeout")
        .expect("Server panicked");

    println!("✅ PubkyNoiseChannel real transport test passed");
}

#[tokio::test]
async fn test_complete_payment_flow_encrypted() {
    // Test 3: Complete payment flow over real Noise

    // Setup server (payee) with shared seed
    let server_pk = get_server_public_key();
    let server_ring = Arc::new(DummyRing::new(TEST_SEED, "server_kid"));
    let server = Arc::new(NoiseServer::<DummyRing>::new_direct(
        "server_kid",
        SERVER_DEVICE_ID,
        server_ring,
    ));

    // Setup payee manager
    let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payee_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payee_manager = Arc::new(PaykitInteractiveManager::new(
        payee_storage.clone(),
        payee_generator,
    ));

    // Create test public keys
    let payer_pk = create_test_pubkey("payer");
    let payee_pk = create_test_pubkey("payee");

    // Start TCP server
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let server_addr = listener.local_addr().expect("Failed to get address");

    // Spawn server (payee) task
    let server_handle = {
        let server = server.clone();
        let payee_manager = payee_manager.clone();
        let payer_pk = payer_pk.clone();
        let payee_pk = payee_pk.clone();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("Failed to accept");

            // Read handshake message
            let mut len_bytes = [0u8; 4];
            let (mut reader, mut writer) = tokio::io::split(stream);
            reader
                .read_exact(&mut len_bytes)
                .await
                .expect("Failed to read length");
            let len = u32::from_be_bytes(len_bytes) as usize;

            let mut handshake_msg = vec![0u8; len];
            reader
                .read_exact(&mut handshake_msg)
                .await
                .expect("Failed to read handshake");

            // Process handshake - Step 2: Server processes client's message and sends response
            let (mut hs_state, _identity) = server
                .build_responder_read_ik(&handshake_msg)
                .expect("Handshake failed");

            // Generate response message
            let mut response_msg = vec![0u8; 128];
            let n = hs_state
                .write_message(&[], &mut response_msg)
                .expect("Write failed");
            response_msg.truncate(n);

            // Send handshake response
            writer
                .write_all(&(response_msg.len() as u32).to_be_bytes())
                .await
                .expect("Write failed");
            writer.write_all(&response_msg).await.expect("Write failed");

            // Step 3: Complete handshake to get transport mode
            let link = pubky_noise::datalink_adapter::server_complete_ik(hs_state)
                .expect("Failed to complete handshake");

            // Create channel
            let stream = reader.unsplit(writer);
            let mut channel = PubkyNoiseChannel::new(stream, link);

            // Handle payment request
            let msg = channel.recv().await.expect("Failed to receive");
            let response = payee_manager
                .handle_message(msg, &payer_pk, &payee_pk)
                .await
                .expect("Failed to handle message");

            if let Some(response_msg) = response {
                channel
                    .send(response_msg)
                    .await
                    .expect("Failed to send response");
            }
        })
    };

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Setup client (payer) with shared seed
    let client_ring = Arc::new(DummyRing::new(TEST_SEED, "client_kid"));
    let client = NoiseClient::<DummyRing>::new_direct("client_kid", CLIENT_DEVICE_ID, client_ring);

    let payer_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payer_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payer_manager = PaykitInteractiveManager::new(payer_storage.clone(), payer_generator);

    // Connect using real Noise channel
    let stream = TcpStream::connect(server_addr)
        .await
        .expect("Failed to connect");

    let mut channel = PubkyNoiseChannel::connect(&client, stream, &server_pk)
        .await
        .expect("Failed to connect channel");

    // Create provisional receipt
    let provisional_receipt = PaykitReceipt::new(
        format!(
            "receipt_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ),
        payer_pk.clone(),
        payee_pk.clone(),
        MethodId("lightning".to_string()),
        Some("1000".to_string()),
        Some("SAT".to_string()),
        serde_json::json!({"order_id": "TEST-001"}),
    );

    // Initiate payment over encrypted channel
    let final_receipt = payer_manager
        .initiate_payment(&mut channel, provisional_receipt)
        .await
        .expect("Payment initiation failed");

    // Verify receipt was completed
    assert_eq!(final_receipt.payer, payer_pk);
    assert_eq!(final_receipt.payee, payee_pk);
    assert_eq!(final_receipt.method_id.0, "lightning");
    assert!(final_receipt.metadata.get("invoice").is_some());

    // Wait for server to finish
    timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("Server timeout")
        .expect("Server panicked");

    println!("✅ Complete payment flow over real Noise test passed");
}

// Note: NN pattern with attestation is comprehensively tested in paykit-demo-cli/tests/pattern_e2e.rs
// That test covers the full flow: NN handshake → attestation exchange → payment
/*
    // Test NN pattern with post-handshake attestation flow

    // Server identity (for attestation)
    let server_ed25519_sk = [1u8; 32];
    let server_signing_key = SigningKey::from_bytes(&server_ed25519_sk);
    let server_ed25519_pk = server_signing_key.verifying_key().to_bytes();

    // Client identity (for attestation)
    let client_ed25519_sk = [2u8; 32];
    let client_signing_key = SigningKey::from_bytes(&client_ed25519_sk);
    let client_ed25519_pk = client_signing_key.verifying_key().to_bytes();

    // Start TCP server
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let server_addr = listener.local_addr().expect("Failed to get address");

    // Spawn server task
    let server_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("Failed to accept");

        // Perform NN handshake (no authentication yet)
        let (hs, first_msg) = pubky_noise::datalink_adapter::start_nn().expect("start_nn");
        let server_ephemeral: [u8; 32] = first_msg[..32]
            .try_into()
            .expect("Invalid first message length");

        let (mut reader, mut writer) = tokio::io::split(stream);

        // Send first message
        let len = (first_msg.len() as u32).to_be_bytes();
        writer.write_all(&len).await.expect("write len");
        writer.write_all(&first_msg).await.expect("write msg");

        // Read client response
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes).await.expect("read len");
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut response = vec![0u8; len];
        reader.read_exact(&mut response).await.expect("read response");

        let client_ephemeral: [u8; 32] = response[..32]
            .try_into()
            .expect("Invalid response length");

        // Complete handshake
        let session = pubky_noise::datalink_adapter::complete_raw(hs, &response).expect("complete");

        // Reunite stream
        let stream = reader.unsplit(writer);
        let mut channel = PubkyNoiseChannel::new(stream, session);

        // Post-handshake attestation: Server sends first
        let mut hasher = Sha256::new();
        hasher.update(b"pubky-noise-nn-attestation-v1:");
        hasher.update(&server_ephemeral);
        hasher.update(&client_ephemeral);
        let attestation_msg = hasher.finalize();

        let server_signature = server_signing_key.sign(&attestation_msg);

        channel
            .send(PaykitNoiseMessage::Attestation {
                ed25519_pk: encode_hex_32(&server_ed25519_pk),
                signature: encode_hex_64(&server_signature.to_bytes()),
            })
            .await
            .expect("send attestation");

        // Receive client's attestation
        let msg = channel.recv().await.expect("recv client attestation");
        match msg {
            PaykitNoiseMessage::Attestation {
                ed25519_pk: pk_hex,
                signature: sig_hex,
            } => {
                // Parse hex manually
                let pk_bytes: [u8; 32] = decode_hex_32(&pk_hex).expect("decode pk");
                let sig_bytes: [u8; 64] = decode_hex_64(&sig_hex).expect("decode sig");

                assert_eq!(pk_bytes, client_ed25519_pk);

                // Verify client attestation
                let mut hasher = Sha256::new();
                hasher.update(b"pubky-noise-nn-attestation-v1:");
                hasher.update(&client_ephemeral);
                hasher.update(&server_ephemeral);
                let client_attestation_msg = hasher.finalize();

                let verifying_key = VerifyingKey::from_bytes(&pk_bytes).expect("valid pk");
                let sig = Signature::from_bytes(&sig_bytes);
                verifying_key
                    .verify(&client_attestation_msg, &sig)
                    .expect("Valid client attestation");
            }
            _ => panic!("Expected Attestation message"),
        }

        // Now authenticated - normal message exchange
        let msg = channel.recv().await.expect("recv payment");
        assert!(matches!(msg, PaykitNoiseMessage::Ack));
        channel.send(PaykitNoiseMessage::Ack).await.expect("send ack");
    });

    // Client task
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut stream = TcpStream::connect(server_addr)
        .await
        .expect("Failed to connect");

    // Read server's first message
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await.expect("read len");
    let len = u32::from_be_bytes(len_bytes) as usize;

    let mut server_msg = vec![0u8; len];
    stream.read_exact(&mut server_msg).await.expect("read msg");

    let server_ephemeral: [u8; 32] = server_msg[..32]
        .try_into()
        .expect("Invalid server message length");

    // Send client response
    let (hs, client_msg) = pubky_noise::datalink_adapter::start_nn().expect("start_nn");
    let client_ephemeral: [u8; 32] = client_msg[..32]
        .try_into()
        .expect("Invalid client message length");

    let len = (client_msg.len() as u32).to_be_bytes();
    stream.write_all(&len).await.expect("write len");
    stream.write_all(&client_msg).await.expect("write msg");

    // Complete handshake
    let session = pubky_noise::datalink_adapter::complete_raw(hs, &server_msg).expect("complete");
    let mut channel = PubkyNoiseChannel::new(stream, session);

    // Receive server's attestation
    let msg = channel.recv().await.expect("recv server attestation");
    match msg {
        PaykitNoiseMessage::Attestation {
            ed25519_pk: pk_hex,
            signature: sig_hex,
        } => {
            let pk_bytes: [u8; 32] = decode_hex_32(&pk_hex).expect("decode pk");
            let sig_bytes: [u8; 64] = decode_hex_64(&sig_hex).expect("decode sig");

            assert_eq!(pk_bytes, server_ed25519_pk);

            // Verify server attestation
            let mut hasher = Sha256::new();
            hasher.update(b"pubky-noise-nn-attestation-v1:");
            hasher.update(&server_ephemeral);
            hasher.update(&client_ephemeral);
            let server_attestation_msg = hasher.finalize();

            let verifying_key = VerifyingKey::from_bytes(&pk_bytes).expect("valid pk");
            let sig = Signature::from_bytes(&sig_bytes);
            verifying_key
                .verify(&server_attestation_msg, &sig)
                .expect("Valid server attestation");
        }
        _ => panic!("Expected Attestation message"),
    }

    // Send client's attestation
    let mut hasher = Sha256::new();
    hasher.update(b"pubky-noise-nn-attestation-v1:");
    hasher.update(&client_ephemeral);
    hasher.update(&server_ephemeral);
    let client_attestation_msg = hasher.finalize();

    let client_signature = client_signing_key.sign(&client_attestation_msg);

    channel
        .send(PaykitNoiseMessage::Attestation {
            ed25519_pk: encode_hex_32(&client_ed25519_pk),
            signature: encode_hex_64(&client_signature.to_bytes()),
        })
        .await
        .expect("send attestation");

    // Now authenticated - send payment
    channel.send(PaykitNoiseMessage::Ack).await.expect("send payment");
    let response = channel.recv().await.expect("recv ack");
    assert!(matches!(response, PaykitNoiseMessage::Ack));

    // Wait for server to finish
    timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("Server timeout")
        .expect("Server panicked");

    println!("✅ NN pattern with attestation test passed");
}
*/

// Helper functions (unused but kept for reference)
/*
// Helper functions to encode/decode hex strings without the hex crate
fn encode_hex_32(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn encode_hex_64(bytes: &[u8; 64]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn decode_hex_32(hex_str: &str) -> Result<[u8; 32], String> {
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        let start = i * 2;
        if start + 2 > hex_str.len() {
            return Err("Hex string too short".to_string());
        }
        bytes[i] = u8::from_str_radix(&hex_str[start..start + 2], 16)
            .map_err(|e| format!("Invalid hex: {}", e))?;
    }
    Ok(bytes)
}

fn decode_hex_64(hex_str: &str) -> Result<[u8; 64], String> {
    let mut bytes = [0u8; 64];
    for i in 0..64 {
        let start = i * 2;
        if start + 2 > hex_str.len() {
            return Err("Hex string too short".to_string());
        }
        bytes[i] = u8::from_str_radix(&hex_str[start..start + 2], 16)
            .map_err(|e| format!("Invalid hex: {}", e))?;
    }
    Ok(bytes)
}
*/
