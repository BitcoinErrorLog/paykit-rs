//! Migrate command - migrate identities from plaintext to secure storage

use anyhow::Result;
use paykit_demo_core::{IdentityManager, SecureIdentityManager};
use std::path::Path;

use crate::ui;

pub async fn run(storage_dir: &Path, verbose: bool) -> Result<()> {
    ui::header("Migrate to Secure Storage");

    // Check if already using secure storage
    let metadata_path = storage_dir.join("identities_metadata.json");
    if metadata_path.exists() {
        ui::info("Already using secure storage. No migration needed.");
        return Ok(());
    }

    // Check for old plaintext identities
    let identities_dir = storage_dir.join("identities");
    if !identities_dir.exists() {
        ui::info("No identities found. Nothing to migrate.");
        return Ok(());
    }

    let old_manager = IdentityManager::new(&identities_dir);
    let old_identities = old_manager.list()?;

    if old_identities.is_empty() {
        ui::info("No identities found. Nothing to migrate.");
        return Ok(());
    }

    ui::info(&format!(
        "Found {} identity(ies) to migrate",
        old_identities.len()
    ));

    if !ui::confirm("Migrate identities to secure storage?", true)? {
        ui::info("Migration cancelled");
        return Ok(());
    }

    // Create secure manager
    let secure_manager = SecureIdentityManager::new(storage_dir);

    // Migrate each identity
    let mut migrated = 0;
    let mut failed = Vec::new();

    for name in &old_identities {
        if verbose {
            ui::info(&format!("Migrating identity '{}'...", name));
        }

        match old_manager.load(name) {
            Ok(identity) => match secure_manager.save(&identity, name).await {
                Ok(()) => {
                    migrated += 1;
                    if verbose {
                        ui::success(&format!("Migrated '{}'", name));
                    }
                }
                Err(e) => {
                    failed.push((name.clone(), e.to_string()));
                    ui::warning(&format!("Failed to migrate '{}': {}", name, e));
                }
            },
            Err(e) => {
                failed.push((name.clone(), e.to_string()));
                ui::warning(&format!("Failed to load '{}': {}", name, e));
            }
        }
    }

    ui::separator();
    ui::success(&format!(
        "Migrated {}/{} identities",
        migrated,
        old_identities.len()
    ));

    if !failed.is_empty() {
        ui::warning(&format!("{} identities failed to migrate:", failed.len()));
        for (name, error) in &failed {
            ui::error(&format!("  {}: {}", name, error));
        }
        ui::info("Old identity files are preserved. You can retry migration later.");
    } else {
        // Ask if user wants to delete old files
        if ui::confirm("Delete old plaintext identity files?", false)? {
            for name in &old_identities {
                if let Err(e) = old_manager.delete(name) {
                    ui::warning(&format!("Failed to delete old file for '{}': {}", name, e));
                }
            }
            ui::success("Old files deleted");
        } else {
            ui::info("Old files preserved for backup");
        }
    }

    Ok(())
}
