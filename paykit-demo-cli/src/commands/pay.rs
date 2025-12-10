//! Pay command - initiate payment

use anyhow::Result;
use paykit_demo_core::DemoStorage;
use std::path::Path;

use crate::ui;

#[tracing::instrument(skip(storage_dir))]
pub async fn run(
    storage_dir: &Path,
    recipient: &str,
    amount: Option<String>,
    currency: Option<String>,
    method: &str,
    verbose: bool,
) -> Result<()> {
    ui::header("Initiate Payment");

    tracing::debug!("Loading current identity");
    // Load current identity
    let identity = super::load_current_identity(storage_dir)?;

    if verbose {
        ui::info(&format!("Payer: {}", identity.pubky_uri()));
        tracing::info!("Payer: {}", identity.pubky_uri());
    }

    // Resolve recipient (could be contact name or URI)
    let payee_uri = resolve_recipient(storage_dir, recipient)?;

    ui::info(&format!("Recipient: {}", payee_uri));
    ui::info(&format!("Method: {}", method));

    if let Some(amt) = &amount {
        if let Some(curr) = &currency {
            ui::info(&format!("Amount: {} {}", amt, curr));
        } else {
            ui::info(&format!("Amount: {}", amt));
        }
    }

    ui::separator();
    ui::warning("Full payment flow implementation pending");
    ui::info("In production, this would:");
    ui::info("  1. Connect to recipient via Noise protocol");
    ui::info("  2. Exchange payment method information");
    ui::info("  3. Coordinate payment execution");
    ui::info("  4. Generate and exchange receipts");

    ui::separator();
    ui::success("Payment prepared (simulation mode)");

    Ok(())
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
