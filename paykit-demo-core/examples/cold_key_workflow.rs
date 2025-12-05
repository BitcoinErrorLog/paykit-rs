//! Complete Cold-Key Workflow Example
//!
//! This example demonstrates the end-to-end flow for using cold Ed25519 keys
//! with Noise protocol connections. The workflow shows:
//!
//! 1. One-time setup: Ed25519 key signs X25519 key binding (offline operation)
//! 2. Publishing X25519 key to pkarr/pubky storage
//! 3. Runtime: Discovering peer's X25519 key via pkarr
//! 4. Runtime: Connecting with IK-raw pattern (no Ed25519 access needed)
//! 5. Encrypted payment message exchange
//!
//! # Key Insight
//!
//! After the initial signing and publishing step, the Ed25519 key can be stored
//! in cold storage (offline, HSM, etc.). All runtime Noise connections only need
//! the derived X25519 key, which can be kept in hot storage (secure enclave, keychain).

use paykit_demo_core::{
    pkarr_discovery::{derive_noise_keypair, prepare_cold_key_publication},
    testing::MockStorage,
    Identity, NoiseRawClientHelper,
};
use paykit_interactive::{PaykitNoiseChannel, PaykitNoiseMessage, PaykitReceipt};
use paykit_lib::MethodId;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use zeroize::Zeroizing;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Paykit Cold-Key Workflow Example ===\n");

    // ====================================================================
    // PHASE 1: ONE-TIME COLD KEY SETUP (Offline Operation)
    // ====================================================================

    println!("Phase 1: Cold Key Setup (happens once, offline)\n");
    println!("  └─ Ed25519 key signs X25519 binding, then goes back to cold storage\n");

    // Simulate retrieving Ed25519 key from cold storage
    let ed25519_seed = [42u8; 32]; // In production: from HSM/offline storage
    let device_id = "mobile-device-001";

    println!("  Step 1a: Derive X25519 keypair from Ed25519 seed");
    let (_x25519_sk, _x25519_pk) = derive_noise_keypair(&ed25519_seed, device_id);
    println!("    ✓ X25519 keypair derived (device: {})", device_id);

    println!("\n  Step 1b: Sign pkarr binding message with Ed25519");
    let (_x25519_sk, _x25519_pk, txt_record) =
        prepare_cold_key_publication(&ed25519_seed, device_id);
    println!("    ✓ Binding signed: {}", &txt_record[..60].to_string());
    println!("    ✓ Ed25519 key can now return to cold storage");

    println!("\n  Step 1c: Publish to pkarr/pubky storage");
    // In production: Use PubkySession to publish
    let mock_storage = MockStorage::new();
    let owner_pk = "test_owner"; // Simplified for demo
    let path = format!("/pub/noise.app/v0/{}", device_id);
    mock_storage.put(owner_pk, &path, txt_record.clone());
    println!("    ✓ Published to pubky storage: {}", path);

    println!("\n  🔒 Ed25519 key is now COLD (offline, HSM, hardware wallet)\n");

    // ====================================================================
    // PHASE 2: RUNTIME CONNECTIONS (Hot X25519 Only)
    // ====================================================================

    println!("\nPhase 2: Runtime Noise Connections (no Ed25519 needed)\n");
    println!("  └─ Uses only X25519 keys, no cold key access required\n");

    // Simulate server (Alice) who has published her X25519 key
    let alice = Identity::generate().with_nickname("Alice");
    let alice_ed25519_seed = [1u8; 32];
    let alice_device = "alice-mobile";
    let (alice_x25519_sk, alice_x25519_pk) = derive_noise_keypair(&alice_ed25519_seed, alice_device);
    let alice_x25519_sk = Zeroizing::new(alice_x25519_sk);

    // Simulate client (Bob) who will discover Alice's key via pkarr
    let bob = Identity::generate().with_nickname("Bob");
    let bob_ed25519_seed = [2u8; 32];
    let bob_device = "bob-mobile";
    let (bob_x25519_sk, _bob_x25519_pk) = derive_noise_keypair(&bob_ed25519_seed, bob_device);
    let bob_x25519_sk = Zeroizing::new(bob_x25519_sk);

    // Bob discovers Alice's X25519 key from pkarr (simulated with mock)
    println!("  Step 2a: Bob discovers Alice's X25519 key via pkarr");
    // In production: use discover_noise_key(&storage, &alice_pubkey, device)
    let discovered_alice_pk = alice_x25519_pk; // Mock discovery
    println!("    ✓ Discovered Alice's X25519 key: {:?}", &discovered_alice_pk[..8]);

    // ====================================================================
    // PHASE 3: IK-RAW HANDSHAKE (No Ed25519 Signing)
    // ====================================================================

    println!("\n  Step 2b: Establish IK-raw Noise connection");
    println!("    └─ Identity proven via pkarr, not handshake signing\n");

    // Start Alice's server
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let server_addr = listener.local_addr()?;
    println!("    Server listening on {}", server_addr);

    // Spawn server task (Alice)
    let alice_sk_clone = alice_x25519_sk.clone();
    let server_task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        println!("    [Server] Accepted connection");

        // Read pattern negotiation byte
        let mut pattern_byte = [0u8; 1];
        stream
            .read_exact(&mut pattern_byte)
            .await
            .expect("read pattern");
        assert_eq!(pattern_byte[0], 0x01); // IK-raw pattern
        println!("    [Server] Received pattern byte: 0x01 (IK-raw)");

        // Read handshake message
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await.expect("read len");
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut first_msg = vec![0u8; len];
        stream
            .read_exact(&mut first_msg)
            .await
            .expect("read handshake");
        println!("    [Server] Received handshake ({} bytes)", len);

        // Process IK-raw handshake
        use pubky_noise::datalink_adapter;
        let (hs, response) = datalink_adapter::accept_ik_raw(&alice_sk_clone, &first_msg)
            .expect("accept ik-raw");
        println!("    [Server] Generated response ({} bytes)", response.len());

        // Send response
        let len = (response.len() as u32).to_be_bytes();
        stream.write_all(&len).await.expect("write len");
        stream.write_all(&response).await.expect("write response");

        // Complete to transport mode
        let session = pubky_noise::NoiseSession::from_handshake(hs).expect("transport");
        println!("    [Server] Handshake complete, entering transport mode");

        // Create channel and receive payment request
        use paykit_interactive::{PaykitNoiseChannel, PaykitNoiseMessage};
        let mut channel = paykit_interactive::transport::PubkyNoiseChannel::new(stream, session);
        println!("    [Server] Transport channel ready");

        let msg = channel.recv().await.expect("recv");
        match msg {
            PaykitNoiseMessage::RequestReceipt {
                provisional_receipt,
            } => {
                println!(
                    "    [Server] Received payment request: {} {}",
                    provisional_receipt.amount.as_deref().unwrap_or("?"),
                    provisional_receipt.currency.as_deref().unwrap_or("SAT")
                );

                // Send confirmation
                channel
                    .send(PaykitNoiseMessage::ConfirmReceipt {
                        receipt: provisional_receipt,
                    })
                    .await
                    .expect("send");
                println!("    [Server] Sent payment confirmation");
            }
            _ => println!("    [Server] Unexpected message type"),
        }
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Connect as client (Bob)
    println!("\n    [Client] Connecting to server...");
    let host = format!("127.0.0.1:{}", server_addr.port());

    // Use NoiseRawClientHelper for IK-raw pattern with negotiation
    let mut channel = NoiseRawClientHelper::connect_ik_raw_with_negotiation(
        &bob_x25519_sk,
        &host,
        &discovered_alice_pk,
    )
    .await?;

    println!("    [Client] IK-raw handshake complete");
    println!("    [Client] ✓ No Ed25519 signing was required!");

    // Send encrypted payment message
    println!("\n  Step 2c: Exchange encrypted messages");
    println!("    [Client] Sending payment request...");

    let receipt = PaykitReceipt::new(
        "cold-key-demo-1".to_string(),
        bob.public_key(),
        alice.public_key(),
        MethodId("lightning".to_string()),
        Some("1000".to_string()),
        Some("SAT".to_string()),
        serde_json::json!({"note": "cold key demo"}),
    );

    channel
        .send(PaykitNoiseMessage::RequestReceipt {
            provisional_receipt: receipt,
        })
        .await?;

    // Receive encrypted response
    let response = channel.recv().await?;
    match response {
        PaykitNoiseMessage::ConfirmReceipt { receipt } => {
            println!("    [Client] Payment confirmed: {}", receipt.receipt_id);
        }
        _ => println!("    [Client] Unexpected response"),
    }

    server_task.await?;

    println!("\n=== Summary ===\n");
    println!("✓ Ed25519 key used ONCE for signing pkarr binding");
    println!("✓ Ed25519 key stored COLD after publishing");
    println!("✓ Runtime connections use ONLY X25519 keys");
    println!("✓ Identity verified via pkarr, not handshake");
    println!("✓ Encrypted payment messages exchanged\n");

    println!("This pattern is ideal for:");
    println!("  - Mobile wallets (Ed25519 in secure enclave, rarely accessed)");
    println!("  - Hardware wallets (Ed25519 offline, X25519 in phone)");
    println!("  - Bitkit integration (cold identity, hot sessions)");

    Ok(())
}

