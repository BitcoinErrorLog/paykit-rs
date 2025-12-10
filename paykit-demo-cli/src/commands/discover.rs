//! Discover command - query payment methods from a Pubky URI

use anyhow::{Context, Result};
use paykit_demo_core::DirectoryClient;
use std::path::Path;

use crate::ui;

#[tracing::instrument(skip(_storage_dir))]
pub async fn run(_storage_dir: &Path, uri: &str, homeserver: &str, verbose: bool) -> Result<()> {
    ui::header("Discover Payment Methods");

    tracing::debug!("Parsing URI: {}", uri);
    // Parse URI
    let public_key = parse_pubky_uri(uri)?;

    if verbose {
        ui::info(&format!("Querying: {}", uri));
        ui::info(&format!("Homeserver: {}", homeserver));
        tracing::info!("Querying {} via homeserver {}", uri, homeserver);
    }

    // Create directory client
    let client = DirectoryClient::new(homeserver);

    // Query methods
    let spinner = ui::spinner("Querying directory...");

    match client.query_methods(&public_key).await {
        Ok(methods) => {
            spinner.finish_and_clear();

            if methods.is_empty() {
                ui::info("No payment methods found");
            } else {
                ui::success(&format!("Found {} payment method(s)", methods.len()));
                ui::separator();

                for method in methods {
                    ui::key_value(&method.method_id, &method.endpoint);
                }
            }
        }
        Err(e) => {
            spinner.finish_and_clear();
            ui::error(&format!("Failed to query directory: {}", e));

            if verbose {
                ui::info(&format!("Error details: {:?}", e));
            }
        }
    }

    Ok(())
}

fn parse_pubky_uri(uri: &str) -> Result<pubky::PublicKey> {
    // Remove pubky:// prefix if present
    let key_str = uri.strip_prefix("pubky://").unwrap_or(uri);

    // Parse as public key
    key_str.parse().context("Invalid Pubky URI format")
}
