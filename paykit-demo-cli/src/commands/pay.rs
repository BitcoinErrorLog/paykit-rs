//! Pay command - initiate payment
//!
//! Supports multiple Noise patterns for different use cases:
//! - IK (default): Mutual authentication with identity binding
//! - IK-raw: Cold key scenario, identity via pkarr
//! - N: Anonymous client, authenticated server (donation box)
//! - NN: Fully anonymous (requires post-handshake attestation)

use anyhow::{anyhow, bail, Context, Result};
use paykit_demo_core::{
    create_attestation, verify_attestation, DemoStorage, Identity, NoiseClientHelper, NoisePattern,
    NoiseRawClientHelper, Receipt,
};
use paykit_interactive::{PaykitNoiseChannel, PaykitNoiseMessage};
use paykit_lib::{MethodId, PubkyUnauthenticatedTransport, UnauthenticatedTransportRead};
use pubky::Pubky;
use std::path::Path;

use crate::ui;

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip(storage_dir))]
pub async fn run(
    storage_dir: &Path,
    recipient: &str,
    amount: Option<String>,
    currency: Option<String>,
    method: &str,
    pattern_str: &str,
    connect: Option<&str>,
    verbose: bool,
) -> Result<()> {
    run_with_sdk(
        storage_dir,
        recipient,
        amount,
        currency,
        method,
        pattern_str,
        connect,
        verbose,
        None,
    )
    .await
}

/// Internal version that accepts an optional SDK (for testing)
#[allow(clippy::too_many_arguments)]
pub async fn run_with_sdk(
    storage_dir: &Path,
    recipient: &str,
    amount: Option<String>,
    currency: Option<String>,
    method: &str,
    pattern_str: &str,
    connect: Option<&str>,
    verbose: bool,
    sdk: Option<&Pubky>,
) -> Result<()> {
    ui::header("Initiate Payment");

    // Parse the pattern
    let pattern: NoisePattern = pattern_str.parse()?;

    tracing::debug!("Loading current identity");
    // Load current identity
    let identity = super::load_current_identity(storage_dir)?;

    if verbose {
        ui::info(&format!("Payer: {}", identity.pubky_uri()));
        ui::info(&format!("Noise pattern: {}", pattern));
        tracing::info!("Payer: {}", identity.pubky_uri());
    }

    // Handle direct connection vs discovery
    let (host, static_pk) = if let Some(connect_addr) = connect {
        // Direct connection mode - skip discovery
        ui::info(&format!("Direct connection to: {}", connect_addr));
        ui::info(&format!("Pattern: {}", pattern));

        // Parse address: host:port@pubkey_hex
        let (parsed_host, pk) = NoiseClientHelper::parse_recipient_address(connect_addr)?;
        let pk =
            pk.ok_or_else(|| anyhow!("Direct connection requires pubkey: host:port@pubkey_hex"))?;

        (parsed_host, pk)
    } else {
        // Discovery mode - resolve recipient and query endpoints
        let payee_uri = resolve_recipient(storage_dir, recipient)?;
        let payee_pubkey: pubky::PublicKey = extract_pubkey_from_uri(&payee_uri)?;

        ui::info(&format!("Recipient: {}", payee_uri));
        ui::info(&format!("Method: {}", method));
        ui::info(&format!("Pattern: {}", pattern));

        if let Some(amt) = &amount {
            if let Some(curr) = &currency {
                ui::info(&format!("Amount: {} {}", amt, curr));
            } else {
                ui::info(&format!("Amount: {}", amt));
            }
        }

        ui::separator();
        ui::info("Discovering recipient's payment endpoints...");

        // Create or use provided SDK
        let default_sdk;
        let sdk_ref = if let Some(s) = sdk {
            s
        } else {
            default_sdk = Pubky::new().context("Failed to create Pubky SDK")?;
            &default_sdk
        };

        // Query recipient's published methods
        let unauth_transport = PubkyUnauthenticatedTransport::new(sdk_ref.public_storage());
        let methods = unauth_transport
            .fetch_supported_payments(&payee_pubkey)
            .await
            .context("Failed to query recipient's payment methods")?;

        if methods.entries.is_empty() {
            ui::error("Recipient has not published any payment methods");
            ui::info("They need to run 'paykit-demo publish' first");
            return Ok(());
        }

        let method_id_lookup = MethodId(method.to_string());
        let endpoint_data = methods
            .entries
            .get(&method_id_lookup)
            .ok_or_else(|| anyhow!("Recipient does not support '{}' method", method))?;

        ui::success(&format!("Found {} endpoint: {}", method, endpoint_data.0));

        // Check if this is a Noise endpoint that we can connect to
        let endpoint_str = &endpoint_data.0;
        if !endpoint_str.starts_with("noise://") {
            // Not a Noise endpoint - show what was discovered
            ui::separator();
            ui::info("Payment endpoint discovered (non-interactive):");
            ui::info(&format!("  Endpoint: {}", endpoint_data.0));
            ui::info("");
            ui::info("This appears to be a static payment endpoint.");
            ui::info("For interactive payments, the recipient should:");
            ui::info("  1. Run: paykit-demo receive --port <PORT>");
            ui::info("  2. Publish with the Noise endpoint format:");
            ui::info("     paykit-demo publish --endpoint 'noise://host:port@pubkey'");
            return Ok(());
        }

        // Parse Noise endpoint: noise://host:port@static_pubkey
        parse_noise_endpoint(endpoint_str)?
    };

    // Resolve recipient for payment messages (even in direct mode)
    let payee_uri = resolve_recipient(storage_dir, recipient)?;
    let payee_pubkey: pubky::PublicKey = extract_pubkey_from_uri(&payee_uri)?;
    let method_id = MethodId(method.to_string());

    // This is an interactive endpoint - connect via Noise
    ui::separator();
    ui::info("Connecting to recipient via Noise protocol...");
    ui::info(&format!("  Pattern: {}", pattern));

    tracing::debug!("Connecting to {} with static key", host);

    // Connect using the appropriate pattern
    let mut channel = match pattern {
        NoisePattern::IK => {
            NoiseClientHelper::connect_to_recipient_with_negotiation(&identity, &host, &static_pk)
                .await
                .context("Failed to establish Noise connection (IK)")?
        }
        NoisePattern::IKRaw => {
            let device_context = format!("paykit-demo-{}", identity.public_key());
            let x25519_sk = NoiseRawClientHelper::derive_x25519_key(
                &identity.keypair.secret_key(),
                device_context.as_bytes(),
            );

            NoiseRawClientHelper::connect_ik_raw_with_negotiation(&x25519_sk, &host, &static_pk)
                .await
                .context("Failed to establish Noise connection (IK-raw)")?
        }
        NoisePattern::N => {
            ui::info("  (Anonymous mode - your identity is not revealed)");
            NoiseRawClientHelper::connect_anonymous_with_negotiation(&host, &static_pk)
                .await
                .context("Failed to establish Noise connection (N)")?
        }
        NoisePattern::NN => {
            ui::warning("  (Ephemeral mode - requires post-handshake attestation)");
            let (mut channel, server_ephemeral, client_ephemeral) =
                NoiseRawClientHelper::connect_ephemeral_with_negotiation(&host)
                    .await
                    .context("Failed to establish Noise connection (NN)")?;
            ui::info(&format!(
                "  Server ephemeral: {}",
                hex::encode(&server_ephemeral[..8])
            ));
            perform_nn_attestation_client(
                &mut channel,
                &identity,
                &payee_pubkey,
                &client_ephemeral,
                &server_ephemeral,
            )
            .await?;
            channel
        }
        NoisePattern::XX => {
            ui::info("  (Trust-on-first-use - keys exchanged during handshake)");
            let device_context = format!("paykit-demo-{}", identity.public_key());
            let x25519_sk = NoiseRawClientHelper::derive_x25519_key(
                &identity.keypair.secret_key(),
                device_context.as_bytes(),
            );
            let (channel, server_static_pk) =
                NoiseRawClientHelper::connect_xx_with_negotiation(&x25519_sk, &host)
                    .await
                    .context("Failed to establish Noise connection (XX)")?;
            ui::info(&format!(
                "  Server static key (cache for future): {}",
                hex::encode(&server_static_pk[..8])
            ));
            channel
        }
    };

    ui::success("Noise connection established");

    // Create and send payment request
    ui::info("Sending payment request...");

    let provisional_receipt = paykit_interactive::PaykitReceipt::new(
        format!("receipt-{}", uuid::Uuid::new_v4()),
        identity.public_key(),
        payee_pubkey,
        method_id.clone(),
        amount.clone(),
        currency.clone(),
        serde_json::json!({}),
    );

    let request = PaykitNoiseMessage::RequestReceipt {
        provisional_receipt,
    };

    channel
        .send(request)
        .await
        .context("Failed to send payment request")?;

    ui::success("Payment request sent");

    // Wait for confirmation
    ui::info("Waiting for recipient confirmation...");

    let response = channel.recv().await.context("Failed to receive response")?;

    match response {
        PaykitNoiseMessage::ConfirmReceipt { receipt } => {
            ui::success("Payment confirmed!");
            ui::separator();
            ui::info("Receipt Details:");
            ui::info(&format!("  ID: {}", receipt.receipt_id));
            ui::info(&format!("  Payer: {}", receipt.payer));
            ui::info(&format!("  Payee: {}", receipt.payee));
            ui::info(&format!("  Method: {}", receipt.method_id.0));
            if let Some(amt) = &receipt.amount {
                if let Some(curr) = &receipt.currency {
                    ui::info(&format!("  Amount: {} {}", amt, curr));
                }
            }

            // Save receipt to storage
            let storage = DemoStorage::new(storage_dir.join("data"));

            // Convert PaykitReceipt to storage Receipt
            let storage_receipt = Receipt::new(
                receipt.receipt_id.clone(),
                receipt.payer.clone(),
                receipt.payee.clone(),
                receipt.method_id.0.clone(),
            );

            storage
                .save_receipt(storage_receipt)
                .context("Failed to save receipt")?;

            ui::success("Receipt saved");
        }
        _ => {
            ui::error("Unexpected response from recipient");
            return Ok(());
        }
    }

    ui::separator();
    ui::success("Payment completed successfully");

    Ok(())
}

async fn perform_nn_attestation_client<C: PaykitNoiseChannel>(
    channel: &mut C,
    identity: &Identity,
    expected_server_pk: &pubky::PublicKey,
    client_ephemeral: &[u8; 32],
    server_ephemeral: &[u8; 32],
) -> Result<()> {
    ui::info("  Authenticating NN session via attestation...");

    let (server_pk_bytes, server_signature) = recv_attestation(channel).await?;
    if server_pk_bytes != expected_server_pk.to_bytes() {
        bail!("Server attestation did not match expected recipient");
    }

    if !verify_attestation(
        &server_pk_bytes,
        &server_signature,
        server_ephemeral,
        client_ephemeral,
    ) {
        bail!("Server attestation signature invalid");
    }

    let client_signature = create_attestation(
        &identity.keypair.secret_key(),
        client_ephemeral,
        server_ephemeral,
    );
    channel
        .send(PaykitNoiseMessage::Attestation {
            ed25519_pk: hex::encode(identity.public_key().to_bytes()),
            signature: hex::encode(client_signature),
        })
        .await
        .context("Failed to send client attestation")?;

    Ok(())
}

async fn recv_attestation<C: PaykitNoiseChannel>(channel: &mut C) -> Result<([u8; 32], [u8; 64])> {
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

fn resolve_recipient(storage_dir: &Path, recipient: &str) -> Result<String> {
    // If it looks like a URI, return as-is
    if recipient.starts_with("pubky://") || recipient.len() > 40 {
        return Ok(recipient.to_string());
    }

    // Otherwise, try to look up as contact name
    let storage = DemoStorage::new(storage_dir.join("data"));
    let contacts = storage.list_contacts()?;

    for contact in contacts {
        if contact.name == recipient {
            return Ok(contact.pubky_uri());
        }
    }

    // If not found, assume it's a public key
    Ok(format!("pubky://{}", recipient))
}

/// Extract the public key from a pubky:// URI
pub fn extract_pubkey_from_uri(uri: &str) -> Result<pubky::PublicKey> {
    let pk_str = uri.strip_prefix("pubky://").unwrap_or(uri);

    pk_str
        .parse::<pubky::PublicKey>()
        .with_context(|| format!("Invalid public key in URI: {}", uri))
}

/// Parse a Noise endpoint string: noise://host:port@static_pubkey
pub fn parse_noise_endpoint(endpoint: &str) -> Result<(String, [u8; 32])> {
    let without_prefix = endpoint
        .strip_prefix("noise://")
        .ok_or_else(|| anyhow::anyhow!("Endpoint must start with 'noise://'"))?;

    if let Some((host, pk_hex)) = without_prefix.split_once('@') {
        // Decode the public key from hex
        let pk_bytes =
            hex::decode(pk_hex).with_context(|| format!("Invalid hex public key: {}", pk_hex))?;

        if pk_bytes.len() != 32 {
            anyhow::bail!("Public key must be 32 bytes, got {}", pk_bytes.len());
        }

        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(&pk_bytes);

        Ok((host.to_string(), pk_array))
    } else {
        anyhow::bail!("Invalid Noise endpoint format. Expected: noise://host:port@pubkey_hex")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pubkey_from_uri() {
        let uri = "pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
        let result = extract_pubkey_from_uri(uri);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_pubkey_without_prefix() {
        let uri = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
        let result = extract_pubkey_from_uri(uri);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_noise_endpoint() {
        let endpoint = "noise://127.0.0.1:9735@0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let result = parse_noise_endpoint(endpoint);
        assert!(result.is_ok());
        let (host, pk) = result.unwrap();
        assert_eq!(host, "127.0.0.1:9735");
        assert_eq!(pk.len(), 32);
    }

    #[test]
    fn test_parse_noise_endpoint_invalid_format() {
        let endpoint = "noise://127.0.0.1:9735"; // Missing @ and pubkey
        let result = parse_noise_endpoint(endpoint);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_noise_endpoint_invalid_hex() {
        let endpoint = "noise://127.0.0.1:9735@xyz";
        let result = parse_noise_endpoint(endpoint);
        assert!(result.is_err());
    }
}
