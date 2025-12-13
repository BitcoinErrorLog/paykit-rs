//! Dashboard command - show summary statistics

use anyhow::Result;
use paykit_demo_core::DemoStorage;
use std::path::Path;

use crate::ui;

/// Display dashboard with summary statistics
pub async fn run(storage_dir: &Path, verbose: bool) -> Result<()> {
    ui::header("Paykit Dashboard");

    // Load identity
    let identity = super::load_current_identity(storage_dir).await?;

    ui::key_value("Identity", &identity.pubky_uri());
    if let Some(nickname) = &identity.nickname {
        ui::key_value("Nickname", nickname);
    }

    // Setup Checklist Section
    ui::separator();
    ui::info("Setup Checklist:");
    
    // Check identity
    ui::success("  ✅ Identity created");
    
    // Check wallet configuration
    let wallet_config = super::wallet::WalletConfig::load(storage_dir)?;
    let has_lightning = wallet_config.as_ref().map(|c| c.has_lightning()).unwrap_or(false);
    let has_onchain = wallet_config.as_ref().map(|c| c.has_onchain()).unwrap_or(false);
    
    if has_lightning {
        ui::success("  ✅ Lightning wallet configured");
    } else {
        ui::warning("  ⬜ Lightning wallet (run: paykit-demo wallet configure-lnd ...)");
    }
    
    if has_onchain {
        ui::success("  ✅ On-chain wallet configured");
    } else {
        ui::warning("  ⬜ On-chain wallet (run: paykit-demo wallet configure-esplora ...)");
    }
    
    // Check contacts
    let storage = DemoStorage::new(storage_dir.join("data"));
    let contacts = storage.list_contacts().unwrap_or_default();
    if !contacts.is_empty() {
        ui::success(&format!("  ✅ {} contact(s) added", contacts.len()));
    } else {
        ui::warning("  ⬜ Add contacts (run: paykit-demo contacts add ...)");
    }
    
    // Check if methods are published
    // For demo, we just check if wallet is configured as a proxy
    if has_lightning || has_onchain {
        ui::info("  ✅ Ready to publish payment methods");
    } else {
        ui::warning("  ⬜ Configure wallet before publishing");
    }
    
    // Calculate completion percentage
    let total_steps = 4;
    let completed_steps = 1 // identity
        + (if has_lightning { 1 } else { 0 })
        + (if has_onchain { 1 } else { 0 })
        + (if !contacts.is_empty() { 1 } else { 0 });
    
    let completion = (completed_steps * 100) / total_steps;
    ui::info(&format!("\n  Setup Progress: {}% ({}/{})", completion, completed_steps, total_steps));

    ui::separator();

    // Load storage
    let storage = DemoStorage::new(storage_dir.join("data"));

    // Contacts summary
    let contacts = storage.list_contacts().unwrap_or_default();
    ui::key_value("Contacts", &format!("{}", contacts.len()));

    // Receipts summary
    let receipts = storage.list_receipts().unwrap_or_default();
    ui::key_value("Total Receipts", &format!("{}", receipts.len()));

    if !receipts.is_empty() {
        // Calculate statistics
        let mut total_payments = 0u64;
        let mut total_received = 0u64;
        let my_pubkey = identity.public_key().to_string();

        for receipt in &receipts {
            // Parse amount from string if available
            if let Some(amount_str) = &receipt.amount {
                if let Ok(amount) = amount_str.parse::<u64>() {
                    if receipt.payer.to_string().contains(&my_pubkey) {
                        total_payments += amount;
                    } else if receipt.payee.to_string().contains(&my_pubkey) {
                        total_received += amount;
                    }
                }
            }
        }

        if total_payments > 0 || total_received > 0 {
            ui::separator();
            ui::info("Payment Statistics:");
            ui::key_value("  Total Sent", &format!("{} sats", total_payments));
            ui::key_value("  Total Received", &format!("{} sats", total_received));

            let balance = total_received as i64 - total_payments as i64;
            let balance_str = if balance >= 0 {
                format!("+{} sats", balance)
            } else {
                format!("{} sats", balance)
            };
            ui::key_value("  Net Balance", &balance_str);
        }

        // Recent activity
        ui::separator();
        ui::info("Recent Activity:");

        let recent: Vec<_> = receipts.iter().take(5).collect();
        for receipt in recent {
            let payer_str = receipt.payer.to_string();
            let direction = if payer_str.contains(&my_pubkey) {
                "→ sent"
            } else {
                "← received"
            };

            let amount_str = receipt
                .amount
                .as_ref()
                .map(|a| format!("{} sats", a))
                .unwrap_or_else(|| "? sats".to_string());

            let peer = if payer_str.contains(&my_pubkey) {
                receipt.payee.to_string()
            } else {
                payer_str.clone()
            };

            // Try to find contact name for peer
            let peer_display = contacts
                .iter()
                .find(|c| peer.contains(&c.public_key.to_string()))
                .map(|c| c.name.clone())
                .unwrap_or_else(|| {
                    if peer.len() > 16 {
                        format!("{}...", &peer[..16])
                    } else {
                        peer.to_string()
                    }
                });

            let timestamp = chrono::DateTime::from_timestamp(receipt.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            ui::info(&format!(
                "  {} {} {} ({})",
                direction, amount_str, peer_display, timestamp
            ));
        }
    }

    // Wallet status
    ui::separator();
    ui::info("Wallet Configuration:");

    let wallet_config = super::wallet::WalletConfig::load(storage_dir)?;
    match wallet_config {
        Some(config) => {
            if config.has_lightning() {
                ui::success("  ✅ Lightning (LND) configured");
            } else {
                ui::warning("  ❌ Lightning not configured");
            }

            if config.has_onchain() {
                ui::success("  ✅ On-chain (Esplora) configured");
            } else {
                ui::warning("  ❌ On-chain not configured");
            }

            ui::key_value("  Network", &config.network);
        }
        None => {
            ui::warning("  No wallet configured");
            ui::info("  Configure with: paykit-demo wallet status");
        }
    }

    // Interactive receipts
    let interactive_dir = storage_dir.join("data").join("interactive_receipts");
    if interactive_dir.exists() {
        let interactive_count = std::fs::read_dir(&interactive_dir)
            .map(|entries| entries.count())
            .unwrap_or(0);

        if interactive_count > 0 {
            ui::separator();
            ui::key_value("Interactive Receipts", &format!("{}", interactive_count));
        }
    }

    if verbose {
        ui::separator();
        ui::info(&format!("Storage Directory: {}", storage_dir.display()));
    }

    ui::separator();
    ui::info("Quick Actions:");
    ui::info("  paykit-demo discover <pubky_uri> - Find payment methods");
    ui::info("  paykit-demo pay <recipient> - Send payment");
    ui::info("  paykit-demo receive - Start payment receiver");
    ui::info("  paykit-demo wallet health - Check method health");

    Ok(())
}
