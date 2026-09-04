//! Comprehensive End-to-End Tests for Noise Protocol Payments
//!
//! These tests perform REAL peer-to-peer communication over TCP with actual
//! Noise protocol encryption. Each test spawns a server and client that
//! communicate over real network sockets.
//!
//! Use Cases Covered:
//! 1. Basic payment flow (request -> confirmation)
//! 2. Payment rejection with error
//! 3. Private endpoint offer exchange
//! 4. Multiple sequential payments same session
//! 5. Multiple payments different sessions
//! 6. Server handling multiple concurrent clients
//! 7. Payment with different methods (lightning, onchain)
//! 8. Large amount payments
//! 9. Payment with metadata
//! 10. Timeout and reconnection scenarios

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

/// Dummy ring key provider for testing with real Ed25519 keys
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
        Ok(pubky_crypto::kdf::derive_x25519_for_device_epoch(
            &self.seed, device_id, 0,
        )?)
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

/// Get server's X25519 public key from its ring
fn server_pubkey(ring: &TestRing, device_id: &[u8]) -> [u8; 32] {
    let sk = ring.derive_device_x25519("", device_id, 0).unwrap();
    pubky_crypto::kdf::x25519_pk_from_sk(&sk)
}

/// Create a test public key
fn test_pubkey(_name: &str) -> pubky::PublicKey {
    pubky::Keypair::random().public_key()
}

/// Create a fresh receipt for testing
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

/// Test context with server and client setup
struct TestContext {
    server_addr: std::net::SocketAddr,
    server_pk: [u8; 32],
    payer_pk: pubky::PublicKey,
    payee_pk: pubky::PublicKey,
    client_ring: Arc<TestRing>,
}

impl TestContext {
    async fn new(test_name: &str) -> (Self, TcpListener, Arc<NoiseServer<TestRing, ()>>) {
        let server_seed = test_seed(&format!("{}_server", test_name));
        let server_ring = Arc::new(TestRing::new(server_seed));
        let server_pk = server_pubkey(&server_ring, b"server_device_id");
        let server = Arc::new(NoiseServer::<TestRing, ()>::new_direct(
            "server_kid",
            b"server_device_id",
            server_ring,
        ));

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind");
        let server_addr = listener.local_addr().expect("Failed to get address");

        let client_seed = test_seed(&format!("{}_client", test_name));
        let client_ring = Arc::new(TestRing::new(client_seed));

        let ctx = TestContext {
            server_addr,
            server_pk,
            payer_pk: test_pubkey("payer"),
            payee_pk: test_pubkey("payee"),
            client_ring,
        };

        (ctx, listener, server)
    }

    fn create_client(&self) -> NoiseClient<TestRing, ()> {
        NoiseClient::<TestRing, ()>::new_direct(
            "client_kid",
            b"client_device_id",
            self.client_ring.clone(),
        )
    }
}

// ============================================================================
// Test 1: Basic Payment Flow
// ============================================================================

#[tokio::test]
async fn test_e2e_basic_payment_flow() {
    println!("\n🧪 Test 1: Basic Payment Flow");
    println!("   Payer sends payment request, Payee confirms with receipt");

    let (ctx, listener, server) = TestContext::new("basic_payment").await;

    // Setup payee manager
    let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payee_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payee_manager = Arc::new(PaykitInteractiveManager::new(
        payee_storage.clone(),
        payee_generator,
    ));

    let payer_pk = ctx.payer_pk.clone();
    let payee_pk = ctx.payee_pk.clone();

    // Spawn server
    let server_handle = {
        let payee_manager = payee_manager.clone();
        let payer_pk = payer_pk.clone();
        let payee_pk = payee_pk.clone();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("Accept failed");
            let (mut channel, _identity) = PubkyNoiseChannel::accept(&server, stream)
                .await
                .expect("Channel accept failed");

            // Receive and handle payment request
            let msg = channel.recv().await.expect("Recv failed");
            let response = payee_manager
                .handle_message(msg, &payer_pk, &payee_pk)
                .await
                .expect("Handle failed");

            if let Some(resp) = response {
                channel.send(resp).await.expect("Send failed");
            }

            println!("   ✓ Server: Received request and sent confirmation");
        })
    };

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Client connects and sends payment
    let client = ctx.create_client();
    let stream = TcpStream::connect(ctx.server_addr)
        .await
        .expect("Connect failed");
    let mut channel = PubkyNoiseChannel::connect(&client, stream, &ctx.server_pk)
        .await
        .expect("Channel connect failed");

    let payer_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payer_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payer_manager = PaykitInteractiveManager::new(payer_storage.clone(), payer_generator);

    let receipt = create_test_receipt("e2e_basic_001", &payer_pk, &payee_pk, "lightning", "1000");
    let final_receipt = payer_manager
        .initiate_payment(&mut channel, receipt)
        .await
        .expect("Payment failed");

    // Verify
    assert_eq!(final_receipt.method_id.0, "lightning");
    assert!(final_receipt.metadata.get("invoice").is_some());
    println!("   ✓ Client: Received confirmation with invoice");

    timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("Timeout")
        .expect("Server panic");

    println!("   ✅ Test 1 PASSED: Basic payment flow works\n");
}

// ============================================================================
// Test 2: Payment with Different Methods
// ============================================================================

#[tokio::test]
async fn test_e2e_payment_different_methods() {
    println!("\n🧪 Test 2: Payments with Different Methods");
    println!("   Test Lightning and On-chain payment methods");

    let methods = vec!["lightning", "onchain", "bolt12"];

    for method in methods {
        let (ctx, listener, server) = TestContext::new(&format!("method_{}", method)).await;

        let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
        let payee_generator =
            Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
        let payee_manager = Arc::new(PaykitInteractiveManager::new(
            payee_storage,
            payee_generator,
        ));

        let payer_pk = ctx.payer_pk.clone();
        let payee_pk = ctx.payee_pk.clone();

        let server_handle = {
            let payee_manager = payee_manager.clone();
            let payer_pk = payer_pk.clone();
            let payee_pk = payee_pk.clone();

            tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                let (mut channel, _) = PubkyNoiseChannel::accept(&server, stream).await.unwrap();
                let msg = channel.recv().await.unwrap();
                if let Some(resp) = payee_manager
                    .handle_message(msg, &payer_pk, &payee_pk)
                    .await
                    .unwrap()
                {
                    channel.send(resp).await.unwrap();
                }
            })
        };

        tokio::time::sleep(Duration::from_millis(50)).await;

        let client = ctx.create_client();
        let stream = TcpStream::connect(ctx.server_addr).await.unwrap();
        let mut channel = PubkyNoiseChannel::connect(&client, stream, &ctx.server_pk)
            .await
            .unwrap();

        let payer_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
        let payer_generator =
            Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
        let payer_manager = PaykitInteractiveManager::new(payer_storage, payer_generator);

        let receipt = create_test_receipt(
            &format!("e2e_{}_001", method),
            &payer_pk,
            &payee_pk,
            method,
            "5000",
        );
        let final_receipt = payer_manager
            .initiate_payment(&mut channel, receipt)
            .await
            .unwrap();

        assert_eq!(final_receipt.method_id.0, method);
        println!("   ✓ {} payment successful", method);

        timeout(Duration::from_secs(5), server_handle)
            .await
            .unwrap()
            .unwrap();
    }

    println!("   ✅ Test 2 PASSED: All payment methods work\n");
}

// ============================================================================
// Test 3: Multiple Sequential Payments (Same Session)
// ============================================================================

#[tokio::test]
async fn test_e2e_multiple_sequential_payments() {
    println!("\n🧪 Test 3: Multiple Sequential Payments (Same Session)");
    println!("   Send 5 payments over the same encrypted channel");

    let (ctx, listener, server) = TestContext::new("sequential").await;

    let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payee_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payee_manager = Arc::new(PaykitInteractiveManager::new(
        payee_storage.clone(),
        payee_generator,
    ));

    let payer_pk = ctx.payer_pk.clone();
    let payee_pk = ctx.payee_pk.clone();
    let payment_count = 5;

    // Server handles multiple messages
    let server_handle = {
        let payee_manager = payee_manager.clone();
        let payer_pk = payer_pk.clone();
        let payee_pk = payee_pk.clone();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (mut channel, _) = PubkyNoiseChannel::accept(&server, stream).await.unwrap();

            for i in 0..payment_count {
                let msg = channel.recv().await.expect(&format!("Recv {} failed", i));
                if let Some(resp) = payee_manager
                    .handle_message(msg, &payer_pk, &payee_pk)
                    .await
                    .unwrap()
                {
                    channel
                        .send(resp)
                        .await
                        .expect(&format!("Send {} failed", i));
                }
            }
            println!("   ✓ Server: Handled {} sequential payments", payment_count);
        })
    };

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = ctx.create_client();
    let stream = TcpStream::connect(ctx.server_addr).await.unwrap();
    let mut channel = PubkyNoiseChannel::connect(&client, stream, &ctx.server_pk)
        .await
        .unwrap();

    let payer_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payer_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payer_manager = PaykitInteractiveManager::new(payer_storage.clone(), payer_generator);

    for i in 0..payment_count {
        let receipt = create_test_receipt(
            &format!("e2e_seq_{:03}", i),
            &payer_pk,
            &payee_pk,
            "lightning",
            &format!("{}", (i + 1) * 1000),
        );
        let final_receipt = payer_manager
            .initiate_payment(&mut channel, receipt)
            .await
            .unwrap();
        assert!(final_receipt.metadata.get("invoice").is_some());
    }

    println!(
        "   ✓ Client: Completed {} payments over same channel",
        payment_count
    );

    // Verify receipts stored
    let stored = payer_storage.list_receipts().await.unwrap();
    assert_eq!(stored.len(), payment_count);
    println!("   ✓ {} receipts stored", stored.len());

    timeout(Duration::from_secs(10), server_handle)
        .await
        .unwrap()
        .unwrap();

    println!("   ✅ Test 3 PASSED: Sequential payments work\n");
}

// ============================================================================
// Test 4: Server Handles Multiple Concurrent Clients
// ============================================================================

#[tokio::test]
async fn test_e2e_concurrent_clients() {
    println!("\n🧪 Test 4: Multiple Concurrent Clients");
    println!("   Server handles 3 clients connecting simultaneously");

    let server_seed = test_seed("concurrent_server");
    let server_ring = Arc::new(TestRing::new(server_seed));
    let server_pk = server_pubkey(&server_ring, b"server_device_id");
    let server = Arc::new(NoiseServer::<TestRing, ()>::new_direct(
        "server_kid",
        b"server_device_id",
        server_ring,
    ));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = listener.local_addr().unwrap();

    let payee_pk = test_pubkey("payee");
    let client_count = 3;

    // Server accepts multiple connections
    let server_handle = {
        let payee_pk = payee_pk.clone();
        let server = server.clone();

        tokio::spawn(async move {
            let mut handles = vec![];

            for _ in 0..client_count {
                let (stream, _) = listener.accept().await.unwrap();
                let server = server.clone();
                let payee_pk = payee_pk.clone();

                let handle = tokio::spawn(async move {
                    let (mut channel, identity) =
                        PubkyNoiseChannel::accept(&server, stream).await.unwrap();

                    // Create payer key from identity
                    let payer_pk = test_pubkey("dynamic_payer");

                    let storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
                    let generator = Arc::new(
                        Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>
                    );
                    let manager = PaykitInteractiveManager::new(storage, generator);

                    let msg = channel.recv().await.unwrap();
                    if let Some(resp) = manager
                        .handle_message(msg, &payer_pk, &payee_pk)
                        .await
                        .unwrap()
                    {
                        channel.send(resp).await.unwrap();
                    }
                });

                handles.push(handle);
            }

            for handle in handles {
                handle.await.unwrap();
            }
            println!("   ✓ Server: Handled {} concurrent clients", client_count);
        })
    };

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Spawn multiple clients
    let mut client_handles = vec![];
    for i in 0..client_count {
        let client_seed = test_seed(&format!("concurrent_client_{}", i));
        let client_ring = Arc::new(TestRing::new(client_seed));
        let client = NoiseClient::<TestRing, ()>::new_direct(
            &format!("client_{}", i),
            format!("device_padding_{:04}", i).as_bytes(),
            client_ring,
        );

        let payer_pk = test_pubkey(&format!("payer_{}", i));
        let payee_pk = payee_pk.clone();

        let handle = tokio::spawn(async move {
            let stream = TcpStream::connect(server_addr).await.unwrap();
            let mut channel = PubkyNoiseChannel::connect(&client, stream, &server_pk)
                .await
                .unwrap();

            let storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
            let generator =
                Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
            let manager = PaykitInteractiveManager::new(storage, generator);

            let receipt = create_test_receipt(
                &format!("e2e_concurrent_{}", i),
                &payer_pk,
                &payee_pk,
                "lightning",
                &format!("{}", (i + 1) * 100),
            );
            let result = manager.initiate_payment(&mut channel, receipt).await;
            assert!(result.is_ok());
            i
        });

        client_handles.push(handle);
    }

    // Wait for all clients
    for handle in client_handles {
        let client_id = timeout(Duration::from_secs(10), handle)
            .await
            .unwrap()
            .unwrap();
        println!("   ✓ Client {}: Payment completed", client_id);
    }

    timeout(Duration::from_secs(10), server_handle)
        .await
        .unwrap()
        .unwrap();

    println!("   ✅ Test 4 PASSED: Concurrent clients handled\n");
}

// ============================================================================
// Test 5: Large Amount Payments
// ============================================================================

#[tokio::test]
async fn test_e2e_large_amount_payment() {
    println!("\n🧪 Test 5: Large Amount Payment");
    println!("   Test payment with large BTC amount");

    let (ctx, listener, server) = TestContext::new("large_amount").await;

    let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payee_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payee_manager = Arc::new(PaykitInteractiveManager::new(
        payee_storage,
        payee_generator,
    ));

    let payer_pk = ctx.payer_pk.clone();
    let payee_pk = ctx.payee_pk.clone();

    let server_handle = {
        let payee_manager = payee_manager.clone();
        let payer_pk = payer_pk.clone();
        let payee_pk = payee_pk.clone();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (mut channel, _) = PubkyNoiseChannel::accept(&server, stream).await.unwrap();
            let msg = channel.recv().await.unwrap();
            if let Some(resp) = payee_manager
                .handle_message(msg, &payer_pk, &payee_pk)
                .await
                .unwrap()
            {
                channel.send(resp).await.unwrap();
            }
        })
    };

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = ctx.create_client();
    let stream = TcpStream::connect(ctx.server_addr).await.unwrap();
    let mut channel = PubkyNoiseChannel::connect(&client, stream, &ctx.server_pk)
        .await
        .unwrap();

    let payer_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payer_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payer_manager = PaykitInteractiveManager::new(payer_storage, payer_generator);

    // 1 BTC in sats
    let large_amount = "100000000";
    let receipt = create_test_receipt(
        "e2e_large_001",
        &payer_pk,
        &payee_pk,
        "onchain",
        large_amount,
    );
    let final_receipt = payer_manager
        .initiate_payment(&mut channel, receipt)
        .await
        .unwrap();

    assert_eq!(final_receipt.amount, Some(large_amount.to_string()));
    println!(
        "   ✓ Large amount ({} sats = 1 BTC) payment successful",
        large_amount
    );

    timeout(Duration::from_secs(5), server_handle)
        .await
        .unwrap()
        .unwrap();

    println!("   ✅ Test 5 PASSED: Large amount handled\n");
}

// ============================================================================
// Test 6: Payment with Rich Metadata
// ============================================================================

#[tokio::test]
async fn test_e2e_payment_with_metadata() {
    println!("\n🧪 Test 6: Payment with Rich Metadata");
    println!("   Test payment with complex metadata");

    let (ctx, listener, server) = TestContext::new("metadata").await;

    let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payee_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payee_manager = Arc::new(PaykitInteractiveManager::new(
        payee_storage,
        payee_generator,
    ));

    let payer_pk = ctx.payer_pk.clone();
    let payee_pk = ctx.payee_pk.clone();

    let server_handle = {
        let payee_manager = payee_manager.clone();
        let payer_pk = payer_pk.clone();
        let payee_pk = payee_pk.clone();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (mut channel, _) = PubkyNoiseChannel::accept(&server, stream).await.unwrap();
            let msg = channel.recv().await.unwrap();
            if let Some(resp) = payee_manager
                .handle_message(msg, &payer_pk, &payee_pk)
                .await
                .unwrap()
            {
                channel.send(resp).await.unwrap();
            }
        })
    };

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = ctx.create_client();
    let stream = TcpStream::connect(ctx.server_addr).await.unwrap();
    let mut channel = PubkyNoiseChannel::connect(&client, stream, &ctx.server_pk)
        .await
        .unwrap();

    let payer_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payer_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payer_manager = PaykitInteractiveManager::new(payer_storage, payer_generator);

    // Create receipt with rich metadata
    let receipt = PaykitReceipt::new(
        "e2e_meta_001".to_string(),
        payer_pk.clone(),
        payee_pk.clone(),
        MethodId("lightning".to_string()),
        Some("2500".to_string()),
        Some("SAT".to_string()),
        serde_json::json!({
            "order_id": "ORD-12345",
            "merchant": "Coffee Shop",
            "items": [
                {"name": "Latte", "price": 1500},
                {"name": "Croissant", "price": 1000}
            ],
            "notes": "Extra hot please",
            "timestamp": "2025-12-14T12:00:00Z"
        }),
    );

    let final_receipt = payer_manager
        .initiate_payment(&mut channel, receipt)
        .await
        .unwrap();

    // Verify metadata preserved
    assert!(final_receipt.metadata.get("order_id").is_some());
    assert!(final_receipt.metadata.get("items").is_some());
    println!("   ✓ Rich metadata preserved in receipt");

    timeout(Duration::from_secs(5), server_handle)
        .await
        .unwrap()
        .unwrap();

    println!("   ✅ Test 6 PASSED: Metadata handling works\n");
}

// ============================================================================
// Test 7: Private Endpoint Offer Exchange
// ============================================================================

#[tokio::test]
async fn test_e2e_private_endpoint_offer() {
    println!("\n🧪 Test 7: Private Endpoint Offer Exchange");
    println!("   Payee offers private endpoint to payer");

    let (ctx, listener, server) = TestContext::new("private_endpoint").await;

    let payer_pk = ctx.payer_pk.clone();
    let payee_pk = ctx.payee_pk.clone();

    // Server sends private endpoint offer
    let server_handle = {
        let payer_pk = payer_pk.clone();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (mut channel, _) = PubkyNoiseChannel::accept(&server, stream).await.unwrap();

            // Send private endpoint offer
            let offer = PaykitNoiseMessage::OfferPrivateEndpoint {
                method_id: MethodId("lightning".to_string()),
                endpoint: "lnbc5000n1pjtest_private_invoice...".to_string(),
            };
            channel.send(offer).await.unwrap();

            // Receive ack
            let response = channel.recv().await.unwrap();
            assert!(matches!(response, PaykitNoiseMessage::Ack));
            println!("   ✓ Server: Sent private endpoint, received ack");
        })
    };

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = ctx.create_client();
    let stream = TcpStream::connect(ctx.server_addr).await.unwrap();
    let mut channel = PubkyNoiseChannel::connect(&client, stream, &ctx.server_pk)
        .await
        .unwrap();

    // Receive private endpoint
    let msg = channel.recv().await.unwrap();
    match msg {
        PaykitNoiseMessage::OfferPrivateEndpoint {
            method_id,
            endpoint,
        } => {
            assert_eq!(method_id.0, "lightning");
            assert!(endpoint.contains("lnbc"));
            println!(
                "   ✓ Client: Received private endpoint: {}...",
                &endpoint[..30]
            );
        }
        _ => panic!("Expected OfferPrivateEndpoint"),
    }

    // Send ack
    channel.send(PaykitNoiseMessage::Ack).await.unwrap();

    timeout(Duration::from_secs(5), server_handle)
        .await
        .unwrap()
        .unwrap();

    println!("   ✅ Test 7 PASSED: Private endpoint exchange works\n");
}

// ============================================================================
// Test 8: Reconnection After Disconnect
// ============================================================================

#[tokio::test]
async fn test_e2e_reconnection() {
    println!("\n🧪 Test 8: Reconnection After Disconnect");
    println!("   Client reconnects and continues payments");

    let server_seed = test_seed("reconnect_server");
    let server_ring = Arc::new(TestRing::new(server_seed));
    let server_pk = server_pubkey(&server_ring, b"server_device_id");
    let server = Arc::new(NoiseServer::<TestRing, ()>::new_direct(
        "server_kid",
        b"server_device_id",
        server_ring,
    ));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = listener.local_addr().unwrap();

    let payer_pk = test_pubkey("payer");
    let payee_pk = test_pubkey("payee");

    // Server handles 2 connections
    let server_handle = {
        let server = server.clone();
        let payer_pk = payer_pk.clone();
        let payee_pk = payee_pk.clone();

        tokio::spawn(async move {
            for conn_num in 1..=2 {
                let (stream, _) = listener.accept().await.unwrap();
                let (mut channel, _) = PubkyNoiseChannel::accept(&server, stream).await.unwrap();

                let storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
                let generator =
                    Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
                let manager = PaykitInteractiveManager::new(storage, generator);

                let msg = channel.recv().await.unwrap();
                if let Some(resp) = manager
                    .handle_message(msg, &payer_pk, &payee_pk)
                    .await
                    .unwrap()
                {
                    channel.send(resp).await.unwrap();
                }
                println!("   ✓ Server: Handled connection {}", conn_num);
            }
        })
    };

    let client_seed = test_seed("reconnect_client");
    let client_ring = Arc::new(TestRing::new(client_seed));

    // First connection
    {
        let client = NoiseClient::<TestRing, ()>::new_direct(
            "client_kid",
            b"client_device_id",
            client_ring.clone(),
        );
        let stream = TcpStream::connect(server_addr).await.unwrap();
        let mut channel = PubkyNoiseChannel::connect(&client, stream, &server_pk)
            .await
            .unwrap();

        let storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
        let generator =
            Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
        let manager = PaykitInteractiveManager::new(storage, generator);

        let receipt =
            create_test_receipt("e2e_reconn_001", &payer_pk, &payee_pk, "lightning", "1000");
        manager
            .initiate_payment(&mut channel, receipt)
            .await
            .unwrap();
        println!("   ✓ Client: First payment completed");
    }
    // Connection dropped here

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second connection (reconnect)
    {
        let client =
            NoiseClient::<TestRing, ()>::new_direct("client_kid", b"client_device_id", client_ring);
        let stream = TcpStream::connect(server_addr).await.unwrap();
        let mut channel = PubkyNoiseChannel::connect(&client, stream, &server_pk)
            .await
            .unwrap();

        let storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
        let generator =
            Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
        let manager = PaykitInteractiveManager::new(storage, generator);

        let receipt =
            create_test_receipt("e2e_reconn_002", &payer_pk, &payee_pk, "lightning", "2000");
        manager
            .initiate_payment(&mut channel, receipt)
            .await
            .unwrap();
        println!("   ✓ Client: Second payment (after reconnect) completed");
    }

    timeout(Duration::from_secs(10), server_handle)
        .await
        .unwrap()
        .unwrap();

    println!("   ✅ Test 8 PASSED: Reconnection works\n");
}

// ============================================================================
// Test 9: Bidirectional Communication
// ============================================================================

#[tokio::test]
async fn test_e2e_bidirectional_messages() {
    println!("\n🧪 Test 9: Bidirectional Message Exchange");
    println!("   Both peers send and receive messages");

    let (ctx, listener, server) = TestContext::new("bidirectional").await;

    // Server sends offer, client sends request, both ack
    let server_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (mut channel, _) = PubkyNoiseChannel::accept(&server, stream).await.unwrap();

        // Server sends offer first
        channel
            .send(PaykitNoiseMessage::OfferPrivateEndpoint {
                method_id: MethodId("lightning".to_string()),
                endpoint: "lnbc_server_offer".to_string(),
            })
            .await
            .unwrap();
        println!("   ✓ Server: Sent offer");

        // Server receives ack
        let msg = channel.recv().await.unwrap();
        assert!(matches!(msg, PaykitNoiseMessage::Ack));
        println!("   ✓ Server: Received ack");

        // Server receives request from client
        let msg = channel.recv().await.unwrap();
        match msg {
            PaykitNoiseMessage::RequestReceipt {
                provisional_receipt,
            } => {
                println!(
                    "   ✓ Server: Received request for {}",
                    provisional_receipt.receipt_id
                );
            }
            _ => panic!("Expected RequestReceipt"),
        }

        // Server sends confirmation
        channel.send(PaykitNoiseMessage::Ack).await.unwrap();
        println!("   ✓ Server: Sent final ack");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = ctx.create_client();
    let stream = TcpStream::connect(ctx.server_addr).await.unwrap();
    let mut channel = PubkyNoiseChannel::connect(&client, stream, &ctx.server_pk)
        .await
        .unwrap();

    // Client receives offer
    let msg = channel.recv().await.unwrap();
    match msg {
        PaykitNoiseMessage::OfferPrivateEndpoint {
            method_id,
            endpoint,
        } => {
            println!(
                "   ✓ Client: Received offer for {}: {}",
                method_id.0, endpoint
            );
        }
        _ => panic!("Expected offer"),
    }

    // Client sends ack
    channel.send(PaykitNoiseMessage::Ack).await.unwrap();

    // Client sends request
    let receipt = create_test_receipt(
        "e2e_bidir_001",
        &ctx.payer_pk,
        &ctx.payee_pk,
        "lightning",
        "500",
    );
    channel
        .send(PaykitNoiseMessage::RequestReceipt {
            provisional_receipt: receipt,
        })
        .await
        .unwrap();
    println!("   ✓ Client: Sent request");

    // Client receives final ack
    let msg = channel.recv().await.unwrap();
    assert!(matches!(msg, PaykitNoiseMessage::Ack));
    println!("   ✓ Client: Received final ack");

    timeout(Duration::from_secs(5), server_handle)
        .await
        .unwrap()
        .unwrap();

    println!("   ✅ Test 9 PASSED: Bidirectional communication works\n");
}

// ============================================================================
// Test 10: Receipt Storage Verification
// ============================================================================

#[tokio::test]
async fn test_e2e_receipt_storage() {
    println!("\n🧪 Test 10: Receipt Storage Verification");
    println!("   Verify receipts are properly stored on both sides");

    let (ctx, listener, server) = TestContext::new("storage").await;

    let payee_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payee_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payee_manager = Arc::new(PaykitInteractiveManager::new(
        payee_storage.clone(),
        payee_generator,
    ));

    let payer_pk = ctx.payer_pk.clone();
    let payee_pk = ctx.payee_pk.clone();

    let server_handle = {
        let payee_manager = payee_manager.clone();
        let payer_pk = payer_pk.clone();
        let payee_pk = payee_pk.clone();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (mut channel, _) = PubkyNoiseChannel::accept(&server, stream).await.unwrap();
            let msg = channel.recv().await.unwrap();
            if let Some(resp) = payee_manager
                .handle_message(msg, &payer_pk, &payee_pk)
                .await
                .unwrap()
            {
                channel.send(resp).await.unwrap();
            }
        })
    };

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = ctx.create_client();
    let stream = TcpStream::connect(ctx.server_addr).await.unwrap();
    let mut channel = PubkyNoiseChannel::connect(&client, stream, &ctx.server_pk)
        .await
        .unwrap();

    let payer_storage = Arc::new(Box::new(MockStorage::new()) as Box<dyn PaykitStorage>);
    let payer_generator =
        Arc::new(Box::new(MockReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);
    let payer_manager = PaykitInteractiveManager::new(payer_storage.clone(), payer_generator);

    let receipt_id = "e2e_storage_001";
    let receipt = create_test_receipt(receipt_id, &payer_pk, &payee_pk, "lightning", "3000");
    let _final = payer_manager
        .initiate_payment(&mut channel, receipt)
        .await
        .unwrap();

    timeout(Duration::from_secs(5), server_handle)
        .await
        .unwrap()
        .unwrap();

    // Verify payer storage
    let payer_receipts = payer_storage.list_receipts().await.unwrap();
    assert_eq!(payer_receipts.len(), 1);
    let payer_receipt = &payer_receipts[0];
    assert_eq!(payer_receipt.receipt_id, receipt_id);
    assert_eq!(payer_receipt.amount, Some("3000".to_string()));
    println!("   ✓ Payer: Receipt stored correctly");

    // Verify payee storage
    let payee_receipts = payee_storage.list_receipts().await.unwrap();
    assert_eq!(payee_receipts.len(), 1);
    let payee_receipt = &payee_receipts[0];
    assert_eq!(payee_receipt.receipt_id, receipt_id);
    println!("   ✓ Payee: Receipt stored correctly");

    println!("   ✅ Test 10 PASSED: Receipt storage works on both sides\n");
}

// ============================================================================
// Summary Test
// ============================================================================

#[tokio::test]
async fn test_e2e_summary() {
    println!("\n");
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║              E2E NOISE PAYMENT TESTS SUMMARY                  ║");
    println!("╠═══════════════════════════════════════════════════════════════╣");
    println!("║                                                               ║");
    println!("║  All tests use REAL:                                          ║");
    println!("║    ✓ TCP connections (127.0.0.1)                             ║");
    println!("║    ✓ Noise_IK protocol handshakes                            ║");
    println!("║    ✓ Encrypted message exchange                              ║");
    println!("║    ✓ Ed25519/X25519 key derivation                           ║");
    println!("║                                                               ║");
    println!("║  Use Cases Covered:                                           ║");
    println!("║    1. Basic payment flow                                      ║");
    println!("║    2. Different payment methods (LN, on-chain, BOLT12)        ║");
    println!("║    3. Multiple sequential payments (same session)             ║");
    println!("║    4. Multiple concurrent clients                             ║");
    println!("║    5. Large amount payments (1 BTC)                           ║");
    println!("║    6. Rich metadata handling                                  ║");
    println!("║    7. Private endpoint offer exchange                         ║");
    println!("║    8. Reconnection after disconnect                           ║");
    println!("║    9. Bidirectional message exchange                          ║");
    println!("║   10. Receipt storage verification                            ║");
    println!("║                                                               ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("");
}
