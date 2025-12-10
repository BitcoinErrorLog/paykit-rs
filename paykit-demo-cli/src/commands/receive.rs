//! Receive command - start payment receiver

use anyhow::Result;
use std::path::Path;

use crate::ui;

#[tracing::instrument(skip(storage_dir))]
pub async fn run(storage_dir: &Path, port: u16, verbose: bool) -> Result<()> {
    ui::header("Payment Receiver");

    tracing::info!("Starting payment receiver on port {}", port);
    // Load current identity
    let identity = super::load_current_identity(storage_dir)?;

    ui::info(&format!("Identity: {}", identity.pubky_uri()));
    ui::info(&format!("Listening on port: {}", port));

    if verbose {
        ui::info("Receiver mode starting...");
    }

    ui::warning("Full Noise server implementation pending");
    ui::info("In production, this would:");
    ui::info("  1. Start a Noise protocol server");
    ui::info("  2. Listen for incoming payment requests");
    ui::info("  3. Generate receipts for successful payments");
    ui::info("  4. Store receipts locally");

    ui::separator();
    ui::info("Receiver ready (simulation mode)");
    ui::info("Press Ctrl+C to stop");

    // Keep running
    tokio::signal::ctrl_c().await?;

    ui::info("\nReceiver stopped");

    Ok(())
}
