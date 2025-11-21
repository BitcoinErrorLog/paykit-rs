//! Receive command - start payment receiver

use anyhow::Result;
use paykit_demo_core::{DemoStorage, NoiseServerHelper, Receipt};
use paykit_interactive::{PaykitNoiseChannel, PaykitNoiseMessage};
use std::path::Path;
use std::sync::Arc;

use crate::ui;

#[tracing::instrument(skip(storage_dir))]
pub async fn run(storage_dir: &Path, port: u16, verbose: bool) -> Result<()> {
    ui::header("Payment Receiver");

    tracing::info!("Starting payment receiver on port {}", port);
    // Load current identity
    let identity = super::load_current_identity(storage_dir)?;

    ui::info(&format!("Identity: {}", identity.pubky_uri()));
    ui::info(&format!("Listening on port: {}", port));

    // Get the server's static public key that clients need to connect
    let device_id = format!("paykit-demo-{}", identity.public_key());
    let static_pk = NoiseServerHelper::get_static_public_key(&identity, device_id.as_bytes(), 0);
    let static_pk_hex = hex::encode(static_pk);

    ui::separator();
    ui::success("Server Configuration:");
    ui::info(&format!("  Static Public Key: {}", static_pk_hex));
    ui::info(&format!(
        "  Connect Address: 127.0.0.1:{}@{}",
        port, static_pk_hex
    ));

    if verbose {
        ui::info("");
        ui::info("Clients can connect using:");
        ui::info(&format!(
            "  paykit-demo pay <recipient> --connect 127.0.0.1:{}@{}",
            port, static_pk_hex
        ));
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
        let result = NoiseServerHelper::run_server(&identity_clone, &bind_addr, |mut channel| {
            let storage_dir = Arc::clone(&storage_dir_arc);
            async move {
                ui::success("Accepted new connection");

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
        })
        .await;

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
