//! Receive command - start payment receiver with Noise protocol

use anyhow::{Context, Result};
use paykit_demo_core::DemoStorage;
use paykit_interactive::{
    PaykitInteractiveManager, PaykitNoiseMessage, PaykitReceipt, PaykitStorage, ReceiptGenerator,
};
use pubky_noise::datalink_adapter::{server_accept_ik, server_complete_ik};
use pubky_noise::{DummyRing, NoiseServer, RingKeyProvider};
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::ui;

/// Simple receipt generator for demo purposes
struct DemoReceiptGenerator;

#[async_trait::async_trait]
impl ReceiptGenerator for DemoReceiptGenerator {
    async fn generate_receipt(
        &self,
        request: &PaykitReceipt,
    ) -> paykit_interactive::Result<PaykitReceipt> {
        // For demo, we just confirm the receipt as-is
        // In production, this would generate invoices, validate amounts, etc.
        let mut confirmed = request.clone();
        confirmed.metadata = serde_json::json!({
            "confirmed_at": chrono::Utc::now().timestamp(),
            "original_metadata": request.metadata,
        });
        Ok(confirmed)
    }
}

/// Simple storage adapter for the demo
struct DemoStorageAdapter {
    storage: DemoStorage,
    endpoint_manager: Option<
        paykit_lib::private_endpoints::PrivateEndpointManager<
            paykit_lib::private_endpoints::FileStore,
        >,
    >,
}

#[async_trait::async_trait]
impl PaykitStorage for DemoStorageAdapter {
    async fn save_receipt(&self, receipt: &PaykitReceipt) -> paykit_interactive::Result<()> {
        let json = serde_json::to_string(receipt)
            .map_err(|e| paykit_interactive::InteractiveError::Serialization(e.to_string()))?;
        self.storage
            .save_receipt_json(&receipt.receipt_id, &json)
            .map_err(|e| paykit_interactive::InteractiveError::Transport(e.to_string()))
    }

    async fn get_receipt(
        &self,
        receipt_id: &str,
    ) -> paykit_interactive::Result<Option<PaykitReceipt>> {
        match self.storage.get_receipt_json(receipt_id) {
            Ok(Some(json)) => {
                let receipt = serde_json::from_str(&json).map_err(|e| {
                    paykit_interactive::InteractiveError::Serialization(e.to_string())
                })?;
                Ok(Some(receipt))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(paykit_interactive::InteractiveError::Transport(
                e.to_string(),
            )),
        }
    }

    async fn list_receipts(&self) -> paykit_interactive::Result<Vec<PaykitReceipt>> {
        match self.storage.list_receipt_jsons() {
            Ok(jsons) => {
                let mut receipts = Vec::new();
                for json in jsons {
                    if let Ok(receipt) = serde_json::from_str(&json) {
                        receipts.push(receipt);
                    }
                }
                Ok(receipts)
            }
            Err(e) => Err(paykit_interactive::InteractiveError::Transport(
                e.to_string(),
            )),
        }
    }

    async fn save_private_endpoint(
        &self,
        peer: &paykit_lib::PublicKey,
        method: &paykit_lib::MethodId,
        endpoint: &str,
    ) -> paykit_interactive::Result<()> {
        if let Some(ref manager) = self.endpoint_manager {
            let endpoint_data = paykit_lib::EndpointData::new(endpoint.to_string());
            manager
                .store_endpoint(peer.clone(), method.clone(), endpoint_data, None)
                .await
                .map_err(|e| paykit_interactive::InteractiveError::Transport(e.to_string()))?;
            return Ok(());
        }
        // Fallback: store in memory (demo only)
        Ok(())
    }

    async fn get_private_endpoint(
        &self,
        peer: &paykit_lib::PublicKey,
        method: &paykit_lib::MethodId,
    ) -> paykit_interactive::Result<Option<String>> {
        if let Some(ref manager) = self.endpoint_manager {
            if let Some(endpoint_data) = manager
                .get_endpoint(peer, method)
                .await
                .map_err(|e| paykit_interactive::InteractiveError::Transport(e.to_string()))?
            {
                return Ok(Some(endpoint_data.0));
            }
        }
        Ok(None)
    }

    async fn list_private_endpoints_for_peer(
        &self,
        _peer: &paykit_lib::PublicKey,
    ) -> paykit_interactive::Result<Vec<(paykit_lib::MethodId, String)>> {
        // FileStore doesn't expose list_for_peer through manager
        // Would need direct store access
        Ok(vec![])
    }

    async fn remove_private_endpoint(
        &self,
        _peer: &paykit_lib::PublicKey,
        _method: &paykit_lib::MethodId,
    ) -> paykit_interactive::Result<()> {
        // FileStore doesn't expose remove directly through manager
        // Would need to access store directly
        Ok(())
    }
}

#[tracing::instrument(skip(storage_dir))]
pub async fn run(storage_dir: &Path, port: u16, verbose: bool) -> Result<()> {
    ui::header("Payment Receiver");

    tracing::info!("Starting payment receiver on port {}", port);

    // Load current identity
    let identity = super::load_current_identity(storage_dir).await?;
    let my_pubkey = identity.public_key();

    ui::info(&format!("Identity: {}", identity.pubky_uri()));
    ui::info(&format!("Listening on port: {}", port));

    // Setup Noise server using identity's secret key as seed
    let seed = identity.keypair.secret_key();
    let ring = Arc::new(DummyRing::new(seed, "paykit-receiver"));
    let server = NoiseServer::<_, ()>::new_direct("paykit-receiver", b"demo-device", ring.clone());

    // Get server's static public key for clients to connect
    let server_sk = ring
        .derive_device_x25519("paykit-receiver", b"demo-device", 0)
        .context("Failed to derive server key")?;
    let server_static_pk = pubky_noise::kdf::x25519_pk_from_sk(&server_sk);

    ui::info(&format!(
        "Noise public key: {}",
        hex::encode(&server_static_pk[..16])
    ));
    ui::separator();

    // Setup storage and manager
    let demo_storage = DemoStorage::new(storage_dir.join("data"));
    demo_storage.init()?;

    // Setup private endpoint storage with encryption
    let endpoint_manager = {
        use paykit_lib::private_endpoints::{encryption, FileStore, PrivateEndpointManager};

        let endpoints_dir = storage_dir.join("private_endpoints");
        let key_path = storage_dir.join(".endpoint_key");

        // Try to load existing key, or generate new one
        let key = if key_path.exists() {
            let key_bytes =
                std::fs::read(&key_path).context("Failed to read endpoint encryption key")?;
            if key_bytes.len() != 32 {
                anyhow::bail!("Invalid key length");
            }
            let mut key_array = [0u8; 32];
            key_array.copy_from_slice(&key_bytes);
            key_array
        } else {
            // Generate new key
            let key = encryption::generate_key();
            let key_array = *key; // Dereference Zeroizing wrapper
            std::fs::write(&key_path, key_array)
                .context("Failed to save endpoint encryption key")?;
            // Set restrictive permissions (Unix only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).ok();
            }
            key_array
        };

        let store = FileStore::new_encrypted(&endpoints_dir, key)
            .context("Failed to create encrypted endpoint store")?;
        Some(PrivateEndpointManager::new(store))
    };

    let storage_adapter = Arc::new(Box::new(DemoStorageAdapter {
        storage: demo_storage,
        endpoint_manager,
    }) as Box<dyn PaykitStorage>);
    let generator = Arc::new(Box::new(DemoReceiptGenerator) as Box<dyn ReceiptGenerator>);
    let manager = PaykitInteractiveManager::new(storage_adapter, generator);

    // Bind TCP listener
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .context("Failed to bind to port")?;

    ui::success(&format!("Receiver listening on 0.0.0.0:{}", port));
    ui::info("Press Ctrl+C to stop");
    ui::separator();

    // Handle connections
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((mut socket, addr)) => {
                        ui::info(&format!("Connection from: {}", addr));

                        // Read first handshake message
                        let mut first_msg = vec![0u8; 4096];
                        let n = match socket.read(&mut first_msg).await {
                            Ok(0) => {
                                ui::warning("Connection closed before handshake");
                                continue;
                            }
                            Ok(n) => n,
                            Err(e) => {
                                ui::error(&format!("Read error: {}", e));
                                continue;
                            }
                        };
                        first_msg.truncate(n);

                        // Process handshake
                        let (server_hs, client_identity, response) = match server_accept_ik(&server, &first_msg) {
                            Ok(result) => result,
                            Err(e) => {
                                ui::error(&format!("Handshake failed: {}", e));
                                continue;
                            }
                        };

                        if verbose {
                            ui::info(&format!(
                                "Client identity: {}...",
                                hex::encode(&client_identity.ed25519_pub[..8])
                            ));
                        }

                        // Send response
                        if let Err(e) = socket.write_all(&response).await {
                            ui::error(&format!("Write error: {}", e));
                            continue;
                        }

                        // Complete handshake
                        let mut link = match server_complete_ik(server_hs) {
                            Ok(link) => link,
                            Err(e) => {
                                ui::error(&format!("Handshake completion failed: {}", e));
                                continue;
                            }
                        };

                        ui::success(&format!("Session established: {}", link.session_id()));

                        // Handle messages
                        loop {
                            // Read length-prefixed message
                            let mut len_buf = [0u8; 4];
                            match socket.read_exact(&mut len_buf).await {
                                Ok(_) => {}
                                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                                    ui::info("Client disconnected");
                                    break;
                                }
                                Err(e) => {
                                    ui::error(&format!("Read error: {}", e));
                                    break;
                                }
                            }
                            let len = u32::from_be_bytes(len_buf) as usize;

                            let mut ciphertext = vec![0u8; len];
                            if let Err(e) = socket.read_exact(&mut ciphertext).await {
                                ui::error(&format!("Read error: {}", e));
                                break;
                            }

                            // Decrypt
                            let plaintext = match link.decrypt(&ciphertext) {
                                Ok(pt) => pt,
                                Err(e) => {
                                    ui::error(&format!("Decryption failed: {}", e));
                                    break;
                                }
                            };

                            // Parse message
                            let msg: PaykitNoiseMessage = match serde_json::from_slice(&plaintext) {
                                Ok(m) => m,
                                Err(e) => {
                                    ui::error(&format!("Parse error: {}", e));
                                    continue;
                                }
                            };

                            if verbose {
                                ui::info(&format!("Received: {:?}", msg));
                            }

                            // Handle message
                            let peer_pk = client_identity.ed25519_pub.to_vec();
                            let peer_pk_str = hex::encode(&peer_pk);
                            let peer_pubkey: paykit_lib::PublicKey = peer_pk_str.parse().unwrap_or_else(|_| {
                                // Fallback for parsing issues
                                paykit_lib::PublicKey::try_from(peer_pk_str).unwrap()
                            });

                            match manager.handle_message(msg, &peer_pubkey, &my_pubkey).await {
                                Ok(Some(response_msg)) => {
                                    let response_json = serde_json::to_vec(&response_msg)
                                        .expect("Failed to serialize response");
                                    let encrypted = link
                                        .encrypt(&response_json)
                                        .expect("Encryption failed");

                                    let len_bytes = (encrypted.len() as u32).to_be_bytes();
                                    if let Err(e) = socket.write_all(&len_bytes).await {
                                        ui::error(&format!("Write error: {}", e));
                                        break;
                                    }
                                    if let Err(e) = socket.write_all(&encrypted).await {
                                        ui::error(&format!("Write error: {}", e));
                                        break;
                                    }

                                    if matches!(response_msg, PaykitNoiseMessage::ConfirmReceipt { .. }) {
                                        ui::success("Receipt confirmed and sent");
                                    }
                                }
                                Ok(None) => {
                                    // No response needed
                                }
                                Err(e) => {
                                    ui::error(&format!("Message handling error: {}", e));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        ui::error(&format!("Accept error: {}", e));
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                ui::info("\nReceiver stopped");
                break;
            }
        }
    }

    Ok(())
}
