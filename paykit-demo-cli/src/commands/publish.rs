//! Publish command - publish payment methods to directory

use anyhow::Result;
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
    let _client = DirectoryClient::new(homeserver);

    tracing::warn!("Full Pubky session publishing not yet fully implemented");
    // Note: Full implementation requires PubkyClient from pubky crate
    // This will be completed in the test-driven implementation phase
    ui::warning("Session creation for publishing will be completed with end-to-end tests");
    ui::info("In production, this would:");
    ui::info("  1. Create a PubkyClient and session with your keypair");
    ui::info("  2. Publish methods via PubkyAuthenticatedTransport");
    ui::info("  3. Make them discoverable via your Pubky URI");

    // For now, just show what would happen
    ui::separator();
    ui::success("Methods prepared for publishing");
    ui::info(&format!("Discoverable at: {}", identity.pubky_uri()));
    tracing::info!("Publish command completed (stub mode)");

    Ok(())
}
