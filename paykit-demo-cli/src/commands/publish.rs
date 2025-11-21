//! Publish command - publish payment methods to directory

use anyhow::{Context, Result};
use paykit_demo_core::{PaymentMethod, SessionManager};
use paykit_lib::{AuthenticatedTransport, EndpointData, MethodId};
use pubky::Pubky;
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
    run_with_sdk(storage_dir, onchain, lightning, homeserver, verbose, None).await
}

/// Internal version that accepts an optional SDK (for testing)
pub async fn run_with_sdk(
    storage_dir: &Path,
    onchain: Option<String>,
    lightning: Option<String>,
    homeserver: &str,
    verbose: bool,
    sdk: Option<&Pubky>,
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

    tracing::info!("Resolving homeserver and creating session");
    ui::info("Connecting to homeserver...");

    // Parse homeserver public key from URL or pubky URI
    let homeserver_pubkey = parse_homeserver_pubkey(homeserver)
        .with_context(|| format!("Invalid homeserver: {}", homeserver))?;

    // Create or use provided SDK
    let default_sdk;
    let sdk_ref = if let Some(s) = sdk {
        s
    } else {
        default_sdk = Pubky::new().context("Failed to create Pubky SDK")?;
        &default_sdk
    };

    // Create authenticated transport
    tracing::debug!("Creating authenticated session");
    let auth_transport = SessionManager::create_with_sdk(sdk_ref, &identity, &homeserver_pubkey)
        .await
        .context("Failed to create authenticated session")?;

    ui::success("Session created");

    // Publish each method
    tracing::info!("Publishing {} methods", methods.len());
    for method in &methods {
        let method_id = MethodId(method.method_id.clone());
        let endpoint_data = EndpointData(method.endpoint.clone());

        tracing::info!("Publishing method: {}", method.method_id);

        auth_transport
            .upsert_payment_endpoint(&method_id, &endpoint_data)
            .await
            .with_context(|| format!("Failed to publish {}", method.method_id))?;

        ui::success(&format!(
            "Published {}: {}",
            method.method_id, method.endpoint
        ));
    }

    ui::separator();
    ui::success("All methods published successfully");
    ui::info(&format!("Discoverable at: {}", identity.pubky_uri()));
    tracing::info!("Publish command completed successfully");

    Ok(())
}

/// Parse homeserver public key from URL or pubky URI
fn parse_homeserver_pubkey(homeserver: &str) -> Result<pubky::PublicKey> {
    // Try to parse as a direct public key (z32 encoded)
    if let Ok(pk) = homeserver.parse::<pubky::PublicKey>() {
        return Ok(pk);
    }

    // Try to extract from pubky:// URI
    if let Some(pk_str) = homeserver.strip_prefix("pubky://") {
        if let Ok(pk) = pk_str.parse::<pubky::PublicKey>() {
            return Ok(pk);
        }
    }

    // For HTTP(S) URLs, we would need to resolve via pkarr DNS
    // For now, return a helpful error
    anyhow::bail!(
        "Homeserver must be specified as a public key (z32 format) or pubky:// URI. \
         HTTP(S) URLs are not yet supported in this demo. \
         Example: pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_homeserver_pubkey_direct() {
        let result =
            parse_homeserver_pubkey("8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_homeserver_pubkey_uri() {
        let result =
            parse_homeserver_pubkey("pubky://8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_homeserver_pubkey_http_unsupported() {
        let result = parse_homeserver_pubkey("https://demo.httprelay.io");
        assert!(result.is_err());
    }
}
