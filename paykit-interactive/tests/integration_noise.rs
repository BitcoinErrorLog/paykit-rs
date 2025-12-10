//! Integration tests for real Noise protocol handshakes and encrypted channels
//!
//! These tests verify that pubky-noise integration works correctly with actual
//! TCP connections and Noise_IK handshakes (not mocks).

use ed25519_dalek::{Signer, SigningKey};
use paykit_interactive::{
    transport::PubkyNoiseChannel, PaykitInteractiveManager, PaykitNoiseChannel, PaykitNoiseMessage,
    PaykitReceipt, PaykitStorage, ReceiptGenerator,
};
use paykit_lib::MethodId;
use pubky_noise::{NoiseClient, NoiseServer, RingKeyProvider};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

// Use mock implementations for storage and receipt generation
#[path = "mock_implementations.rs"]
mod mock_implementations;
use mock_implementations::{MockReceiptGenerator, MockStorage};

/// Dummy ring key provider for testing with real Ed25519 keys
struct DummyRing {
    seed: [u8; 32],
}

impl DummyRing {
    fn new(seed: [u8; 32]) -> Self {
        Self { seed }
    }
}

impl RingKeyProvider for DummyRing {
    fn derive_device_x25519(
        &self,
        _kid: &str,
        device_id: &[u8],
        _epoch: u32,
    ) -> std::result::Result<[u8; 32], pubky_noise::NoiseError> {
        // Use pubky-noise's KDF for proper key derivation
        Ok(pubky_noise::kdf::derive_x25519_for_device_epoch(
            &self.seed, device_id, 0,
        ))
    }

    fn ed25519_pubkey(&self, _kid: &str) -> std::result::Result<[u8; 32], pubky_noise::NoiseError> {
        // Derive Ed25519 public key from seed
        use ed25519_dalek::SigningKey;
        let signing_key = SigningKey::from_bytes(&self.seed);
        Ok(signing_key.verifying_key().to_bytes())
    }

    fn sign_ed25519(
        &self,
        _kid: &str,
        msg: &[u8],
    ) -> std::result::Result<[u8; 64], pubky_noise::NoiseError> {
        // Use real Ed25519 signing
        use ed25519_dalek::{Signer, SigningKey};
        let signing_key = SigningKey::from_bytes(&self.seed);
        Ok(signing_key.sign(msg).to_bytes())
    }
}

/// Helper to generate deterministic test seed
fn generate_test_seed(name: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let result = hasher.finalize();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&result);
    seed
}

/// Helper to get server's X25519 public key from its ring
fn get_server_pubkey(ring: &DummyRing, device_id: &[u8]) -> [u8; 32] {
    let sk = ring.derive_device_x25519("", device_id, 0).unwrap();
    pubky_noise::kdf::x25519_pk_from_sk(&sk)
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

    // Setup server with deterministic seed
    let server_seed = generate_test_seed("server_test_1");
    let server_ring = Arc::new(DummyRing::new(server_seed));
    let server_pk = get_server_pubkey(&server_ring, b"server_device_id");
    let server =
        NoiseServer::<DummyRing, ()>::new_direct("server_kid", b"server_device_id", server_ring);

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
        let mut response = vec![0u8; 128];
        let n = hs_state
            .write_message(&[], &mut response)
            .expect("Failed to write response");
        response.truncate(n);

        // Send handshake response
        let len = (response.len() as u32).to_be_bytes();
        stream.write_all(&len).await.expect("Write failed");
        stream.write_all(&response).await.expect("Write failed");

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

    // Setup client with deterministic seed
    let client_seed = generate_test_seed("client_test_1");
    let client_ring = Arc::new(DummyRing::new(client_seed));
    let client =
        NoiseClient::<DummyRing, ()>::new_direct("client_kid", b"client_device_id", client_ring);

    // Connect and perform handshake
    let mut stream = TcpStream::connect(server_addr)
        .await
        .expect("Failed to connect");

    // Step 1: Client starts handshake
    let (hs_state, handshake_msg) =
        pubky_noise::datalink_adapter::client_start_ik_direct(&client, &server_pk, None)
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
    // Test 2: PubkyNoiseChannel with real transport using high-level accept() API

    // Setup server with deterministic seed
    let server_seed = generate_test_seed("server_test_2");
    let server_ring = Arc::new(DummyRing::new(server_seed));
    let server_pk = get_server_pubkey(&server_ring, b"server_device_id");
    let server = Arc::new(NoiseServer::<DummyRing, ()>::new_direct(
        "server_kid2",
        b"server_device_id",
        server_ring,
    ));

    // Start TCP server
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let server_addr = listener.local_addr().expect("Failed to get address");

    // Spawn server task using the new PubkyNoiseChannel::accept() API
    let server_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("Failed to accept");

        // Use the new high-level accept() API
        let (mut channel, identity) = PubkyNoiseChannel::accept(&server, stream)
            .await
            .expect("Failed to accept connection");

        // Verify we got client identity
        println!(
            "Accepted connection from client with Ed25519 pub: {:?}",
            &identity.ed25519_pub[..8]
        );

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

    // Setup client with deterministic seed
    let client_seed = generate_test_seed("client_test_2");
    let client_ring = Arc::new(DummyRing::new(client_seed));
    let client =
        NoiseClient::<DummyRing, ()>::new_direct("client_kid", b"client_device_id", client_ring);

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

    println!("✅ PubkyNoiseChannel real transport test passed (using accept() API)");
}

#[tokio::test]
async fn test_complete_payment_flow_encrypted() {
    // Test 3: Complete payment flow over real Noise using high-level accept() API

    // Setup server (payee) with deterministic seed
    let server_seed = generate_test_seed("payee_server_test_3");
    let server_ring = Arc::new(DummyRing::new(server_seed));
    let server_pk = get_server_pubkey(&server_ring, b"payee_device");
    let server = Arc::new(NoiseServer::<DummyRing, ()>::new_direct(
        "payee_kid",
        b"payee_device",
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

    // Spawn server (payee) task using the new high-level accept() API
    let server_handle = {
        let server = server.clone();
        let payee_manager = payee_manager.clone();
        let payer_pk = payer_pk.clone();
        let payee_pk = payee_pk.clone();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("Failed to accept");

            // Use the new high-level accept() API
            let (mut channel, _identity) = PubkyNoiseChannel::accept(&server, stream)
                .await
                .expect("Failed to accept connection");

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

    // Setup client (payer) with deterministic seed
    let client_seed = generate_test_seed("payer_client_test_3");
    let client_ring = Arc::new(DummyRing::new(client_seed));
    let client =
        NoiseClient::<DummyRing, ()>::new_direct("payer_kid", b"payer_device", client_ring);

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
