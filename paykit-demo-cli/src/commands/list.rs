//! List command - show all identities

use anyhow::Result;
use colored::Colorize;
use paykit_demo_core::IdentityManager;
use std::path::Path;

use crate::ui;

pub async fn run(storage_dir: &Path, _verbose: bool) -> Result<()> {
    ui::header("Saved Identities");

    let identities_dir = storage_dir.join("identities");
    let identity_manager = IdentityManager::new(&identities_dir);

    let identities = identity_manager.list()?;

    if identities.is_empty() {
        ui::info("No identities found");
        ui::info("Run 'paykit-demo setup' to create one");
        return Ok(());
    }

    let current = super::get_current_identity(storage_dir)?;

    for name in identities {
        // Load to get details
        match identity_manager.load(&name) {
            Ok(identity) => {
                let marker = if current.as_ref() == Some(&name) {
                    "→ ".to_string()
                } else {
                    "  ".to_string()
                };

                println!(
                    "{}{}",
                    marker,
                    if current.as_ref() == Some(&name) {
                        name.green().bold().to_string()
                    } else {
                        name.clone()
                    }
                );

                ui::key_value("  URI", &identity.pubky_uri());
            }
            Err(e) => {
                ui::warning(&format!("Failed to load identity '{}': {}", name, e));
            }
        }
    }

    Ok(())
}
