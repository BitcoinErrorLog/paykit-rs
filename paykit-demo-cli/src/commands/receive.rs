//! Receive command - start payment receiver

use anyhow::{anyhow, bail, Context, Result};
use paykit_demo_core::{
    create_attestation, pkarr_discovery, verify_attestation, AcceptedConnection, DemoStorage,
    Identity, NoisePattern, NoiseServerHelper, Receipt,
};
use paykit_interactive::{transport::PubkyNoiseChannel, PaykitNoiseChannel, PaykitNoiseMessage};
use pubky::Pubky;
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
                ui::info(
                    "  Pattern XX: Trust-on-first-use, static keys exchanged during handshake",
                );
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
        // Run pattern-aware server for the selected pattern
        let selected_pattern = pattern;
        let result = NoiseServerHelper::run_pattern_server(&identity_clone, &bind_addr, |conn| {
            let storage_dir = Arc::clone(&storage_dir_arc);
            let identity_for_conn = identity_clone.clone();
            async move {
                if conn.pattern() != selected_pattern {
                    return Err(anyhow!(
                        "Client requested {:?} but server configured for {:?}",
                        conn.pattern(),
                        selected_pattern
                    ));
                }

                match conn {
                    AcceptedConnection::IK {
                        mut channel,
                        client_identity,
                    } => {
                        let context = ClientContext {
                            description: format!(
                                "authenticated ({})",
                                hex::encode(&client_identity.ed25519_pub[..8])
                            ),
                            attested_ed25519: Some(client_identity.ed25519_pub),
                        };
                        handle_payment_request(&mut channel, &storage_dir, context).await
                    }
                    AcceptedConnection::IKRaw {
                        mut channel,
                        client_x25519_pk,
                    } => {
                        // IK-raw: verify client identity via pkarr lookup
                        handle_ik_raw_with_pkarr_verification(
                            &mut channel,
                            &storage_dir,
                            &client_x25519_pk,
                        )
                        .await
                    }
                    AcceptedConnection::N { mut channel } => {
                        // N pattern is ONE-WAY: client can send, server cannot respond
                        // Handle specially - receive only, no confirmation sent back
                        handle_n_pattern_request(&mut channel, &storage_dir).await
                    }
                    AcceptedConnection::NN {
                        mut channel,
                        client_ephemeral,
                        server_ephemeral,
                    } => {
                        let attested_pk = perform_nn_attestation_server(
                            &mut channel,
                            &identity_for_conn,
                            &client_ephemeral,
                            &server_ephemeral,
                        )
                        .await?;
                        let context = ClientContext {
                            description: format!(
                                "ephemeral ({})",
                                hex::encode(&client_ephemeral[..8])
                            ),
                            attested_ed25519: Some(attested_pk),
                        };
                        handle_payment_request(&mut channel, &storage_dir, context).await
                    }
                    AcceptedConnection::XX {
                        mut channel,
                        client_static_pk,
                    } => {
                        let context = ClientContext {
                            description: format!("TOFU ({})", hex::encode(&client_static_pk[..8])),
                            attested_ed25519: None,
                        };
                        handle_payment_request(&mut channel, &storage_dir, context).await
                    }
                }
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

struct ClientContext {
    description: String,
    attested_ed25519: Option<[u8; 32]>,
}

async fn perform_nn_attestation_server(
    channel: &mut PubkyNoiseChannel<tokio::net::TcpStream>,
    identity: &Identity,
    client_ephemeral: &[u8; 32],
    server_ephemeral: &[u8; 32],
) -> Result<[u8; 32]> {
    ui::info("Verifying NN attestation...");

    let server_signature = create_attestation(
        &identity.keypair.secret_key(),
        server_ephemeral,
        client_ephemeral,
    );
    channel
        .send(PaykitNoiseMessage::Attestation {
            ed25519_pk: hex::encode(identity.public_key().to_bytes()),
            signature: hex::encode(server_signature),
        })
        .await?;

    let (client_pk, client_signature) = recv_attestation(channel).await?;
    if !verify_attestation(
        &client_pk,
        &client_signature,
        client_ephemeral,
        server_ephemeral,
    ) {
        bail!("Client attestation signature invalid");
    }

    Ok(client_pk)
}

async fn recv_attestation(
    channel: &mut PubkyNoiseChannel<tokio::net::TcpStream>,
) -> Result<([u8; 32], [u8; 64])> {
    match channel.recv().await? {
        PaykitNoiseMessage::Attestation {
            ed25519_pk,
            signature,
        } => {
            let pk = decode_hex_array::<32>(&ed25519_pk, "attestation public key")?;
            let sig = decode_hex_array::<64>(&signature, "attestation signature")?;
            Ok((pk, sig))
        }
        other => Err(anyhow!(
            "Expected attestation message, received {:?}",
            other
        )),
    }
}

fn decode_hex_array<const N: usize>(hex_str: &str, label: &str) -> Result<[u8; N]> {
    let bytes =
        hex::decode(hex_str).with_context(|| format!("Invalid {} hex: {}", label, hex_str))?;
    if bytes.len() != N {
        bail!("{} must be {} bytes, got {}", label, N, bytes.len());
    }
    let mut arr = [0u8; N];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Handle IK-raw payment request with pkarr identity verification.
///
/// IK-raw provides channel encryption but defers identity proof to pkarr.
/// This handler:
/// 1. Receives the provisional receipt to learn the claimed payer identity
/// 2. Looks up the payer's X25519 key via pkarr
/// 3. Compares against the client_x25519_pk from the handshake
/// 4. Warns if identity cannot be verified, proceeds with payment handling
async fn handle_ik_raw_with_pkarr_verification(
    channel: &mut PubkyNoiseChannel<tokio::net::TcpStream>,
    storage_dir: &Path,
    client_x25519_pk: &[u8; 32],
) -> Result<()> {
    ui::success(&format!(
        "Accepted cold-key connection (IK-raw: {})",
        hex::encode(&client_x25519_pk[..8])
    ));

    // Receive payment request to learn claimed payer identity
    let msg = channel
        .recv()
        .await
        .context("Failed to receive payment request")?;

    let provisional_receipt = match msg {
        PaykitNoiseMessage::RequestReceipt {
            provisional_receipt,
        } => provisional_receipt,
        other => {
            ui::warning(&format!("Unexpected message type: {:?}", other));
            return Ok(());
        }
    };

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

    // Attempt pkarr verification of client identity
    let pkarr_verified = verify_client_via_pkarr(&provisional_receipt.payer, client_x25519_pk).await;

    match pkarr_verified {
        Ok(true) => {
            ui::success("  ✓ Client identity verified via pkarr");
        }
        Ok(false) => {
            ui::warning("  ✗ Client X25519 key does NOT match pkarr record!");
            ui::warning("    Proceeding with caution - payer identity is NOT verified");
        }
        Err(e) => {
            ui::warning(&format!("  ? Could not verify client via pkarr: {}", e));
            ui::warning("    No pkarr record found - payer identity is NOT verified");
        }
    }

    // Generate and send receipt confirmation
    ui::info("Generating receipt...");
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

    Ok(())
}

/// Verify a client's X25519 key against their pkarr record.
///
/// Returns:
/// - Ok(true) if pkarr record exists and X25519 key matches
/// - Ok(false) if pkarr record exists but X25519 key does NOT match
/// - Err if pkarr lookup fails (no record, network error, etc.)
async fn verify_client_via_pkarr(
    claimed_payer: &pubky::PublicKey,
    client_x25519_pk: &[u8; 32],
) -> Result<bool> {
    // Create Pubky SDK for pkarr lookup
    let sdk = Pubky::new().context("Failed to create Pubky SDK")?;
    let storage = sdk.public_storage();

    // Look up the payer's X25519 key from pkarr
    let expected_x25519 =
        pkarr_discovery::discover_noise_key(&storage, claimed_payer, "default").await?;

    // Compare against the key used in the handshake
    Ok(&expected_x25519 == client_x25519_pk)
}

/// Handle N pattern payment request (ONE-WAY: receive only, no response).
///
/// The N pattern only supports client -> server encryption. The server cannot
/// send encrypted responses back. This handler receives the payment request,
/// logs it, and saves it locally without attempting to send a confirmation.
async fn handle_n_pattern_request(
    channel: &mut PubkyNoiseChannel<tokio::net::TcpStream>,
    storage_dir: &Path,
) -> Result<()> {
    ui::success("Accepted anonymous connection (N pattern - one-way)");
    ui::warning("NOTE: N pattern is one-way. Cannot send confirmation to client.");

    // Receive payment request
    match channel.recv().await {
        Ok(msg) => {
            match msg {
                PaykitNoiseMessage::RequestReceipt {
                    provisional_receipt,
                } => {
                    ui::info(&format!(
                        "Anonymous payment request: {} {} from {}",
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

                    // Save receipt to storage (marked as unconfirmed since we can't ACK)
                    let storage = DemoStorage::new(storage_dir.join("data"));
                    let storage_receipt = Receipt::new(
                        format!("{}-anonymous", provisional_receipt.receipt_id),
                        provisional_receipt.payer,
                        provisional_receipt.payee,
                        provisional_receipt.method_id.0,
                    );

                    if let Err(e) = storage.save_receipt(storage_receipt) {
                        ui::warning(&format!("Failed to save receipt: {}", e));
                    } else {
                        ui::info("Anonymous payment request saved (unconfirmed)");
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

/// Handle a payment request on an established channel.
async fn handle_payment_request(
    channel: &mut PubkyNoiseChannel<tokio::net::TcpStream>,
    storage_dir: &Path,
    context: ClientContext,
) -> Result<()> {
    let ClientContext {
        description,
        attested_ed25519,
    } = context;
    ui::success(&format!("Accepted new connection ({})", description));

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

                    if let Some(expected_pk) = attested_ed25519 {
                        let payer_bytes = provisional_receipt.payer.to_bytes();
                        if payer_bytes != expected_pk {
                            bail!(
                                "Payer identity {} did not match NN attestation",
                                provisional_receipt.payer
                            );
                        }
                    }

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
