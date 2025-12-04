//! Receive command - start payment receiver

use anyhow::Result;
use paykit_demo_core::{AcceptedConnection, DemoStorage, NoisePattern, NoiseServerHelper, Receipt};
use paykit_interactive::{transport::PubkyNoiseChannel, PaykitNoiseChannel, PaykitNoiseMessage};
use std::path::Path;
use std::sync::Arc;

use crate::ui;

#[tracing::instrument(skip(storage_dir))]
pub async fn run(storage_dir: &Path, port: u16, pattern_str: &str, verbose: bool) -> Result<()> {
    ui::header("Payment Receiver");

    // Parse the pattern
    let pattern: NoisePattern = pattern_str.parse()?;
    tracing::info!(
        "Starting payment receiver on port {} with pattern {}",
        port,
        pattern
    );

    // Load current identity
    let identity = super::load_current_identity(storage_dir)?;

    ui::info(&format!("Identity: {}", identity.pubky_uri()));
    ui::info(&format!("Listening on port: {}", port));
    ui::info(&format!("Noise pattern: {}", pattern));

    // Get the server's static public key that clients need to connect
    let device_id = format!("paykit-demo-{}", identity.public_key());
    let static_pk = NoiseServerHelper::get_static_public_key(&identity, device_id.as_bytes());
    let static_pk_hex = hex::encode(static_pk);

    ui::separator();
    ui::success("Server Configuration:");
    ui::info(&format!("  Static Public Key: {}", static_pk_hex));
    ui::info(&format!(
        "  Connect Address: 127.0.0.1:{}@{}",
        port, static_pk_hex
    ));
    ui::info(&format!("  Pattern: {}", pattern));

    if verbose {
        ui::info("");
        ui::info("Clients can connect using:");
        ui::info(&format!(
            "  paykit-demo pay <recipient> --connect 127.0.0.1:{}@{} --pattern {}",
            port, static_pk_hex, pattern_str
        ));

        // Show pattern-specific info
        match pattern {
            NoisePattern::IK => {
                ui::info("  Pattern IK: Client sends signed identity, full authentication");
            }
            NoisePattern::IKRaw => {
                ui::info("  Pattern IK-raw: Client identity verified via pkarr (cold key)");
            }
            NoisePattern::N => {
                ui::info("  Pattern N: Anonymous client, server authenticated (donation box mode)");
            }
            NoisePattern::NN => {
                ui::info(
                    "  Pattern NN: Both parties anonymous (requires post-handshake attestation)",
                );
                ui::warning("  WARNING: NN without attestation is vulnerable to MITM");
            }
            NoisePattern::XX => {
                ui::info("  Pattern XX: Trust-on-first-use, static keys exchanged during handshake");
                ui::info("  Use for TOFU scenarios where keys are cached after first contact");
            }
        }
    }

    ui::separator();
    ui::info("Starting Noise server...");

    let bind_addr = format!("0.0.0.0:{}", port);

    ui::success(&format!("Listening on {}", bind_addr));
    ui::info("Waiting for payment requests...");
    ui::info("Press Ctrl+C to stop");

    // Set up Ctrl+C handler
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tx.send(()).await.ok();
    });

    // Run server in a separate task
    let identity_clone = identity.clone();
    let storage_dir_arc = Arc::new(storage_dir.to_path_buf());
    let server_task = tokio::spawn(async move {
        // For IK pattern, use the standard server
        // For other patterns, use the pattern-aware server
        let result = match pattern {
            NoisePattern::IK => {
                // Standard IK pattern with full identity binding
                NoiseServerHelper::run_server(&identity_clone, &bind_addr, |mut channel| {
                    let storage_dir = Arc::clone(&storage_dir_arc);
                    async move {
                        handle_payment_request(&mut channel, &storage_dir, Some("authenticated"))
                            .await
                    }
                })
                .await
            }
            _ => {
                // Pattern-aware server for IK-raw, N, NN
                NoiseServerHelper::run_pattern_server(&identity_clone, &bind_addr, |conn| {
                    let storage_dir = Arc::clone(&storage_dir_arc);
                    async move {
                        let client_info = match &conn {
                            AcceptedConnection::IK {
                                client_identity, ..
                            } => {
                                format!(
                                    "authenticated ({})",
                                    hex::encode(&client_identity.ed25519_pub[..8])
                                )
                            }
                            AcceptedConnection::IKRaw {
                                client_x25519_pk, ..
                            } => {
                                format!("cold-key ({})", hex::encode(&client_x25519_pk[..8]))
                            }
                            AcceptedConnection::N { .. } => "anonymous".to_string(),
                            AcceptedConnection::NN {
                                client_ephemeral, ..
                            } => {
                                format!("ephemeral ({})", hex::encode(&client_ephemeral[..8]))
                            }
                            AcceptedConnection::XX {
                                client_static_pk, ..
                            } => {
                                format!("TOFU ({})", hex::encode(&client_static_pk[..8]))
                            }
                        };

                        let mut channel = conn.into_channel();
                        handle_payment_request(&mut channel, &storage_dir, Some(&client_info)).await
                    }
                })
                .await
            }
        };

        if let Err(e) = result {
            eprintln!("Server error: {:#}", e);
        }
    });

    // Wait for Ctrl+C
    rx.recv().await;

    ui::info("\nShutting down receiver...");
    server_task.abort();

    ui::success("Receiver stopped");

    Ok(())
}

/// Handle a payment request on an established channel.
async fn handle_payment_request(
    channel: &mut PubkyNoiseChannel<tokio::net::TcpStream>,
    storage_dir: &Path,
    client_info: Option<&str>,
) -> Result<()> {
    let client_desc = client_info.unwrap_or("unknown");
    ui::success(&format!("Accepted new connection ({})", client_desc));

    // Receive payment request
    match channel.recv().await {
        Ok(msg) => {
            match msg {
                PaykitNoiseMessage::RequestReceipt {
                    provisional_receipt,
                } => {
                    ui::info(&format!(
                        "Payment request: {} {} from {}",
                        provisional_receipt
                            .amount
                            .as_ref()
                            .unwrap_or(&"?".to_string()),
                        provisional_receipt
                            .currency
                            .as_ref()
                            .unwrap_or(&"SAT".to_string()),
                        provisional_receipt.payer
                    ));

                    // Generate receipt
                    ui::info("Generating receipt...");
                    // In a real implementation, you'd validate the payment here

                    // Send confirmation
                    let confirm_msg = PaykitNoiseMessage::ConfirmReceipt {
                        receipt: provisional_receipt.clone(),
                    };

                    channel.send(confirm_msg).await?;
                    ui::success("Receipt sent");

                    // Save receipt to storage
                    let storage = DemoStorage::new(storage_dir.join("data"));
                    let storage_receipt = Receipt::new(
                        provisional_receipt.receipt_id,
                        provisional_receipt.payer,
                        provisional_receipt.payee,
                        provisional_receipt.method_id.0,
                    );

                    if let Err(e) = storage.save_receipt(storage_receipt) {
                        ui::warning(&format!("Failed to save receipt: {}", e));
                    } else {
                        ui::info("Receipt saved to storage");
                    }
                }
                _ => {
                    ui::warning(&format!("Unexpected message type: {:?}", msg));
                }
            }
        }
        Err(e) => {
            ui::error(&format!("Failed to receive message: {}", e));
        }
    }

    Ok(())
}
