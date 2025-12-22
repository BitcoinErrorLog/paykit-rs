//! End-to-End Tests for Server Mode Message Processing
//!
//! These tests verify that server mode correctly handles incoming connections,
//! performs Noise handshakes, and processes payment messages using the
//! PaykitInteractiveManagerFFI pattern used in mobile apps.

use paykit_interactive::{
    transport::PubkyNoiseChannel, PaykitInteractiveManager, PaykitNoiseChannel, PaykitNoiseMessage,
    PaykitReceipt, PaykitStorage, ReceiptGenerator,
};
use paykit_lib::MethodId;
use pubky_noise::{NoiseClient, NoiseServer, RingKeyProvider};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

// Use mock implementations for storage and receipt generation
#[path = "mock_implementations.rs"]
mod mock_implementations;
use mock_implementations::{MockReceiptGenerator, MockStorage};

// ============================================================================
// Test Infrastructure
// ============================================================================

/// Dummy ring key provider for testing
struct TestRing {
    seed: [u8; 32],
}

impl TestRing {
    fn new(seed: [u8; 32]) -> Self {
        Self { seed }
    }
}

impl RingKeyProvider for TestRing {
    fn derive_device_x25519(
        &self,
        _kid: &str,
        device_id: &[u8],
        _epoch: u32,
    ) -> std::result::Result<[u8; 32], pubky_noise::NoiseError> {
        pubky_noise::kdf::derive_x25519_for_device_epoch(&self.seed, device_id, 0)
    }

    fn ed25519_pubkey(&self, _kid: &str) -> std::result::Result<[u8; 32], pubky_noise::NoiseError> {
        use ed25519_dalek::SigningKey;
        let signing_key = SigningKey::from_bytes(&self.seed);
        Ok(signing_key.verifying_key().to_bytes())
    }

    fn sign_ed25519(
        &self,
        _kid: &str,
        msg: &[u8],
    ) -> std::result::Result<[u8; 64], pubky_noise::NoiseError> {
        use ed25519_dalek::{Signer, SigningKey};
        let signing_key = SigningKey::from_bytes(&self.seed);
        Ok(signing_key.sign(msg).to_bytes())
    }
}

/// Generate deterministic test seed from name
fn test_seed(name: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let result = hasher.finalize();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&result);
    seed
}

/// Get server's X25519 public key
fn server_pubkey(ring: &TestRing, device_id: &[u8]) -> [u8; 32] {
    let sk = ring.derive_device_x25519("", device_id, 0).unwrap();
    pubky_noise::kdf::x25519_pk_from_sk(&sk)
}

/// Create a test public key
fn test_pubkey(_name: &str) -> pubky::PublicKey {
    pubky::Keypair::random().public_key()
}

/// Create a test receipt
fn create_test_receipt(
    id: &str,
    payer: &pubky::PublicKey,
    payee: &pubky::PublicKey,
    method: &str,
    amount: &str,
) -> PaykitReceipt {
    PaykitReceipt::new(
        id.to_string(),
        payer.clone(),
        payee.clone(),
        MethodId(method.to_string()),
        Some(amount.to_string()),
        Some("SAT".to_string()),
        serde_json::json!({"test": true}),
    )
}

// ============================================================================
// Test 1: Server Accepts Connection and Handles Handshake
// ============================================================================

#[tokio::test]
async fn test_server_accepts_connection() {
    println!("\n🧪 Test 1: Server Accepts Connection");
    println!("   Server accepts TCP connection and performs Noise handshake");

    let server_seed = test_seed("server_accept");
    let server_ring = Arc::new(TestRing::new(server_seed));
    let server = Arc::new(NoiseServer::<TestRing, ()>::new_direct(
        "server_kid",
        b"server_device",
        server_ring.clone(),
    ));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let server_addr = listener.local_addr().expect("Failed to get address");

    let client_seed = test_seed("client_accept");
    let client_ring = Arc::new(TestRing::new(client_seed));
    let client = NoiseClient::<TestRing, ()>::new_direct(
        "client_kid",
        b"client_device",
        client_ring,
    );

    let server_pk = server_pubkey(&server_ring, b"server_device");

    // Spawn server accepting connection
    let server_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("Accept failed");
        let (_channel, _identity) = PubkyNoiseChannel::accept(&server, stream)
            .await
            .expect("Channel accept failed");
        println!("   ✓ Server: Accepted connection and completed handshake");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Client connects
    let stream = TcpStream::connect(server_addr).await.expect("Connect failed");
    let _channel = PubkyNoiseChannel::connect(&client, stream, &server_pk)
        .await
        .expect("Channel connect failed");
    println!("   ✓ Client: Connected and completed handshake");

    timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("Timeout")
        .expect("Server panic");

    println!("   ✅ Test 1 PASSED: Server accepts connections\n");
}

// ============================================================================
// Test 2: Server Processes Payment Request
// ============================================================================

#[tokio::test]
async fn test_server_processes_payment_request() {
    println!("\n🧪 Test 2: Server Processes Payment Request");
    println!("   Server receives RequestReceipt and responds with ConfirmReceipt");

    let server_seed = test_seed("server_process");
    let server_ring = Arc::new(TestRing::new(server_seed));
    let server = Arc::new(NoiseServer::<TestRing, ()>::new_direct(
        "server_kid",
        b"server_device",
        server_ring.clone(),
    ));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let server_addr = listener.local_addr().expect("Failed to get address");

    let client_seed = test_seed("client_process");
    let client_ring = Arc::new(TestRing::new(client_seed));
    let client = NoiseClient::<TestRing, ()>::new_direct(
        "client_kid",
        b"client_device",
        client_ring,
    );

    let server_pk = server_pubkey(&server_ring, b"server_device");
    let payer_pk = test_pubkey("payer");
    let payee_pk = test_pubkey("payee");

    // Setup server manager
    let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payee_generator = Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payee_manager = Arc::new(PaykitInteractiveManager::new(payee_storage, payee_generator));

    // Spawn server handling payment request
    let server_handle = {
        let payee_manager = payee_manager.clone();
        let payer_pk = payer_pk.clone();
        let payee_pk = payee_pk.clone();
        
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("Accept failed");
            let (mut channel, _identity) = PubkyNoiseChannel::accept(&server, stream)
                .await
                .expect("Channel accept failed");

            // Receive payment request
            let msg = channel.recv().await.expect("Recv failed");
            println!("   ✓ Server: Received payment request");

            // Process message
            let response = payee_manager
                .handle_message(msg, &payer_pk, &payee_pk)
                .await
                .expect("Handle failed");

            // Send response
            if let Some(resp) = response {
                channel.send(resp).await.expect("Send failed");
                println!("   ✓ Server: Sent confirmation");
            }
        })
    };

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Client sends payment request
    let stream = TcpStream::connect(server_addr).await.expect("Connect failed");
    let mut channel = PubkyNoiseChannel::connect(&client, stream, &server_pk)
        .await
        .expect("Channel connect failed");

    let receipt = create_test_receipt("server_process_001", &payer_pk, &payee_pk, "lightning", "1000");
    channel
        .send(PaykitNoiseMessage::RequestReceipt {
            provisional_receipt: receipt,
        })
        .await
        .expect("Send failed");

    // Receive confirmation
    let response = channel.recv().await.expect("Recv failed");
    match response {
        PaykitNoiseMessage::ConfirmReceipt { receipt } => {
            assert_eq!(receipt.method_id.0, "lightning");
            assert!(receipt.metadata.get("invoice").is_some());
            println!("   ✓ Client: Received confirmation");
        }
        _ => panic!("Expected ConfirmReceipt"),
    }

    timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("Timeout")
        .expect("Server panic");

    println!("   ✅ Test 2 PASSED: Server processes payment requests\n");
}

// ============================================================================
// Test 3: Server Handles Multiple Concurrent Connections
// ============================================================================

#[tokio::test]
async fn test_server_multiple_concurrent_connections() {
    println!("\n🧪 Test 3: Server Handles Multiple Concurrent Connections");
    println!("   Server accepts and processes multiple clients simultaneously");

    let server_seed = test_seed("server_multi");
    let server_ring = Arc::new(TestRing::new(server_seed));
    let server = Arc::new(NoiseServer::<TestRing, ()>::new_direct(
        "server_kid",
        b"server_device",
        server_ring.clone(),
    ));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let server_addr = listener.local_addr().expect("Failed to get address");

    let server_pk = server_pubkey(&server_ring, b"server_device");
    let payer_pk = test_pubkey("payer");
    let payee_pk = test_pubkey("payee");

    // Setup server manager
    let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payee_generator = Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payee_manager = Arc::new(PaykitInteractiveManager::new(payee_storage, payee_generator));

    // Spawn server accepting multiple connections
    let server_handle = {
        let server = server.clone();
        let payee_manager = payee_manager.clone();
        let payer_pk = payer_pk.clone();
        let payee_pk = payee_pk.clone();
        
        tokio::spawn(async move {
            let mut handles = vec![];
            for i in 0..3 {
                let (stream, _) = listener.accept().await.expect("Accept failed");
                let server = server.clone();
                let payee_manager = payee_manager.clone();
                let payer_pk = payer_pk.clone();
                let payee_pk = payee_pk.clone();
                
                let handle = tokio::spawn(async move {
                    let (mut channel, _identity) = PubkyNoiseChannel::accept(&server, stream)
                        .await
                        .expect("Channel accept failed");

                    let msg = channel.recv().await.expect("Recv failed");
                    let response = payee_manager
                        .handle_message(msg, &payer_pk, &payee_pk)
                        .await
                        .expect("Handle failed");

                    if let Some(resp) = response {
                        channel.send(resp).await.expect("Send failed");
                    }
                    println!("   ✓ Server: Handled connection {}", i + 1);
                });
                handles.push(handle);
            }
            
            for handle in handles {
                handle.await.expect("Connection handler panic");
            }
        })
    };

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Spawn multiple clients
    let mut client_handles = vec![];
    for i in 0..3 {
        let server_addr = server_addr;
        let server_pk = server_pk;
        let payer_pk = payer_pk.clone();
        let payee_pk = payee_pk.clone();
        
        let handle = tokio::spawn(async move {
            let client_seed = test_seed(&format!("client_multi_{}", i));
            let client_ring = Arc::new(TestRing::new(client_seed));
            let client = NoiseClient::<TestRing, ()>::new_direct(
                "client_kid",
                b"client_device",
                client_ring,
            );

            let stream = TcpStream::connect(server_addr).await.expect("Connect failed");
            let mut channel = PubkyNoiseChannel::connect(&client, stream, &server_pk)
                .await
                .expect("Channel connect failed");

            let receipt = create_test_receipt(
                &format!("multi_{}", i),
                &payer_pk,
                &payee_pk,
                "lightning",
                "1000",
            );
            channel
                .send(PaykitNoiseMessage::RequestReceipt {
                    provisional_receipt: receipt,
                })
                .await
                .expect("Send failed");

            let response = channel.recv().await.expect("Recv failed");
            match response {
                PaykitNoiseMessage::ConfirmReceipt { .. } => {
                    println!("   ✓ Client {}: Received confirmation", i + 1);
                }
                _ => panic!("Expected ConfirmReceipt"),
            }
        });
        client_handles.push(handle);
    }

    // Wait for all clients
    for handle in client_handles {
        timeout(Duration::from_secs(10), handle)
            .await
            .expect("Timeout")
            .expect("Client panic");
    }

    timeout(Duration::from_secs(10), server_handle)
        .await
        .expect("Timeout")
        .expect("Server panic");

    println!("   ✅ Test 3 PASSED: Server handles multiple concurrent connections\n");
}

// ============================================================================
// Test 4: Server Handles Error Messages
// ============================================================================

#[tokio::test]
async fn test_server_handles_errors() {
    println!("\n🧪 Test 4: Server Handles Error Messages");
    println!("   Server responds with error for invalid requests");

    let server_seed = test_seed("server_error");
    let server_ring = Arc::new(TestRing::new(server_seed));
    let server = Arc::new(NoiseServer::<TestRing, ()>::new_direct(
        "server_kid",
        b"server_device",
        server_ring.clone(),
    ));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let server_addr = listener.local_addr().expect("Failed to get address");

    let client_seed = test_seed("client_error");
    let client_ring = Arc::new(TestRing::new(client_seed));
    let client = NoiseClient::<TestRing, ()>::new_direct(
        "client_kid",
        b"client_device",
        client_ring,
    );

    let server_pk = server_pubkey(&server_ring, b"server_device");
    let payer_pk = test_pubkey("payer");
    let wrong_payee_pk = test_pubkey("wrong_payee");
    let correct_payee_pk = test_pubkey("correct_payee");

    // Setup server manager
    let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payee_generator = Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payee_manager = Arc::new(PaykitInteractiveManager::new(payee_storage, payee_generator));

    // Spawn server
    let server_handle = {
        let payee_manager = payee_manager.clone();
        let payer_pk = payer_pk.clone();
        let correct_payee_pk = correct_payee_pk.clone();
        
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("Accept failed");
            let (mut channel, _identity) = PubkyNoiseChannel::accept(&server, stream)
                .await
                .expect("Channel accept failed");

            // Receive request with wrong payee
            let msg = channel.recv().await.expect("Recv failed");
            let response = payee_manager
                .handle_message(msg, &payer_pk, &correct_payee_pk)
                .await
                .expect("Handle failed");

            // Should return error
            if let Some(resp) = response {
                channel.send(resp).await.expect("Send failed");
            }
            println!("   ✓ Server: Sent error response");
        })
    };

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Client sends request with wrong payee
    let stream = TcpStream::connect(server_addr).await.expect("Connect failed");
    let mut channel = PubkyNoiseChannel::connect(&client, stream, &server_pk)
        .await
        .expect("Channel connect failed");

    let receipt = create_test_receipt(
        "error_test_001",
        &payer_pk,
        &wrong_payee_pk, // Wrong payee!
        "lightning",
        "1000",
    );
    channel
        .send(PaykitNoiseMessage::RequestReceipt {
            provisional_receipt: receipt,
        })
        .await
        .expect("Send failed");

    // Receive error
    let response = channel.recv().await.expect("Recv failed");
    match response {
        PaykitNoiseMessage::Error { code, .. } => {
            assert_eq!(code, "WRONG_PAYEE");
            println!("   ✓ Client: Received error response");
        }
        _ => panic!("Expected Error message"),
    }

    timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("Timeout")
        .expect("Server panic");

    println!("   ✅ Test 4 PASSED: Server handles errors correctly\n");
}

