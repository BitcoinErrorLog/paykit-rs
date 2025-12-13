//! Publish command - publish payment methods to directory

use anyhow::{Context, Result};
use paykit_demo_core::{DirectoryClient, PaymentMethod};
use std::path::Path;

use crate::ui;

#[tracing::instrument(skip(storage_dir))]
pub async fn run(
    storage_dir: &Path,
    onchain: Option<String>,
    lightning: Option<String>,
    homeserver: &str,
    verbose: bool,
) -> Result<()> {
    ui::header("Publish Payment Methods");

    tracing::debug!("Loading identity for publishing");
    // Load current identity
    let identity = super::load_current_identity(storage_dir)?;

    if verbose {
        ui::info(&format!("Using identity: {}", identity.pubky_uri()));
        ui::info(&format!("Homeserver: {}", homeserver));
        tracing::info!(
            "Identity: {}, Homeserver: {}",
            identity.pubky_uri(),
            homeserver
        );
    }

    // Collect methods to publish
    let mut methods = Vec::new();

    if let Some(addr) = onchain {
        methods.push(PaymentMethod::new("onchain".to_string(), addr, true));
    }

    if let Some(invoice) = lightning {
        methods.push(PaymentMethod::new("lightning".to_string(), invoice, true));
    }

    if methods.is_empty() {
        ui::error("No payment methods specified");
        ui::info("Use --onchain or --lightning to specify methods");
        return Ok(());
    }

    // Show what we'll publish
    ui::info("Publishing methods:");
    for method in &methods {
        ui::key_value(&format!("  {}", method.method_id), &method.endpoint);
    }

    // Create directory client
    let client = DirectoryClient::new(homeserver);

    // Create Pubky session
    let spinner = ui::spinner("Connecting to homeserver...");

    // Determine if we should use testnet (check if homeserver looks like a testnet address)
    // For demo purposes, we default to testnet mode for safety
    let use_testnet = true;

    let session = match client.create_session(&identity.keypair, use_testnet).await {
        Ok(session) => {
            spinner.finish_and_clear();
            tracing::info!("Session created successfully");
            session
        }
        Err(e) => {
            spinner.finish_and_clear();
            ui::error(&format!("Failed to create session: {}", e));
            ui::info("Make sure:");
            ui::info("  1. The homeserver is reachable");
            ui::info("  2. You have network connectivity");
            ui::info("  3. The homeserver public key is valid");
            return Err(e).context("Failed to establish session with homeserver");
        }
    };

    // Publish methods
    let spinner = ui::spinner("Publishing payment methods...");

    match client.publish_methods(&session, &methods).await {
        Ok(()) => {
            spinner.finish_and_clear();
            ui::separator();
            ui::success(&format!(
                "Successfully published {} payment method(s)",
                methods.len()
            ));
            ui::info(&format!("Discoverable at: {}", identity.pubky_uri()));
            tracing::info!(
                "Published {} methods for {}",
                methods.len(),
                identity.pubky_uri()
            );
        }
        Err(e) => {
            spinner.finish_and_clear();
            ui::error(&format!("Failed to publish: {}", e));
            return Err(e).context("Failed to publish payment methods");
        }
    }

    Ok(())
}
