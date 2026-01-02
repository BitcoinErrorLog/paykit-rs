//! Backup and restore commands for identity management

use anyhow::{Context, Result};
use std::path::Path;

use crate::ui;

/// Export current identity to encrypted backup
pub async fn export(storage_dir: &Path, output: Option<&str>, verbose: bool) -> Result<()> {
    ui::header("Export Identity Backup");

    // Load current identity
    let identity = super::load_current_identity(storage_dir).await?;

    ui::info(&format!("Identity: {}", identity.pubky_uri()));

    // Get password from user
    let password =
        rpassword::prompt_password("Enter backup password: ").context("Failed to read password")?;

    if password.is_empty() {
        anyhow::bail!("Password cannot be empty");
    }

    // Confirm password
    let password_confirm = rpassword::prompt_password("Confirm password: ")
        .context("Failed to read password confirmation")?;

    if password != password_confirm {
        anyhow::bail!("Passwords do not match");
    }

    if verbose {
        ui::info("Creating encrypted backup...");
    }

    // Create backup
    let backup = identity.export_backup(&password)?;

    // Determine output path
    let output_path = if let Some(path) = output {
        Path::new(path).to_path_buf()
    } else {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        Path::new(&format!("paykit_backup_{}.json", timestamp)).to_path_buf()
    };

    // Serialize and write
    let json = backup.to_json()?;
    std::fs::write(&output_path, &json)
        .with_context(|| format!("Failed to write backup to {:?}", output_path))?;

    ui::success(&format!("Backup saved to: {}", output_path.display()));
    ui::info(&format!(
        "Public key (for verification): {}",
        backup.public_key_z32
    ));
    ui::separator();
    ui::warning("IMPORTANT: Store this backup securely and remember your password!");
    ui::warning("Without the password, the backup cannot be restored.");

    Ok(())
}

/// Import identity from encrypted backup
pub async fn import(
    storage_dir: &Path,
    input: &str,
    name: Option<&str>,
    verbose: bool,
) -> Result<()> {
    ui::header("Restore Identity from Backup");

    // Read backup file
    let json = std::fs::read_to_string(input)
        .with_context(|| format!("Failed to read backup file: {}", input))?;

    // Parse backup
    let backup = paykit_demo_core::KeyBackup::from_json(&json)?;

    if verbose {
        ui::info(&format!("Backup version: {}", backup.version));
        ui::info(&format!("Public key: {}", backup.public_key_z32));
    }

    // Get password
    let password =
        rpassword::prompt_password("Enter backup password: ").context("Failed to read password")?;

    // Decrypt and import
    let spinner = ui::spinner("Decrypting backup...");
    let identity = paykit_demo_core::Identity::import_backup(&backup, &password)?;
    spinner.finish_and_clear();

    // Verify public key matches
    let derived_pubkey = identity.public_key().to_string();
    if derived_pubkey != backup.public_key_z32 {
        anyhow::bail!("Public key mismatch - backup may be corrupted");
    }

    ui::success("Backup decrypted successfully!");
    ui::key_value("Public Key", &identity.pubky_uri());

    // Determine identity name
    let identity_name = if let Some(n) = name {
        n.to_string()
    } else {
        // Prompt for name
        print!("Enter a name for this identity: ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            "restored".to_string()
        } else {
            trimmed.to_string()
        }
    };

    // Save identity
    let manager = paykit_demo_core::IdentityManager::new(storage_dir.join("identities"));
    manager.save(&identity, &identity_name)?;

    // Set as current identity
    let current_path = storage_dir.join("current_identity");
    std::fs::write(&current_path, &identity_name)?;

    ui::success(&format!(
        "Identity '{}' restored and set as current",
        identity_name
    ));

    Ok(())
}
