//! Setup command - create a new identity

use anyhow::Result;
use paykit_demo_core::{Identity, IdentityManager};
use std::path::Path;

use crate::ui;

pub async fn run(storage_dir: &Path, name: Option<String>, verbose: bool) -> Result<()> {
    ui::header("Setup New Identity");

    // Get or prompt for name
    let name = if let Some(n) = name {
        n
    } else {
        ui::input("Enter a name for this identity")?
    };

    if verbose {
        ui::info(&format!("Creating identity '{}'...", name));
    }

    // Create storage directory
    let identities_dir = storage_dir.join("identities");
    std::fs::create_dir_all(&identities_dir)?;

    let identity_manager = IdentityManager::new(&identities_dir);

    // Check if identity already exists
    if identity_manager.load(&name).is_ok()
        && !ui::confirm(
            &format!("Identity '{}' already exists. Overwrite?", name),
            false,
        )?
    {
        ui::info("Setup cancelled");
        return Ok(());
    }

    // Generate new identity
    let spinner = ui::spinner("Generating keypair...");
    let identity = Identity::generate().with_nickname(&name);
    spinner.finish_and_clear();

    // Save identity
    identity_manager.save(&identity, &name)?;

    // Set as current
    super::set_current_identity(storage_dir, &name)?;

    ui::success(&format!("Identity '{}' created and activated", name));
    ui::separator();
    ui::key_value("Public Key", &identity.public_key().to_string());
    ui::key_value("Pubky URI", &identity.pubky_uri());

    // Show QR code
    println!();
    ui::info("Scan this QR code to share your Pubky URI:");
    ui::qr_code(&identity.pubky_uri())?;

    ui::info(&format!(
        "Identity saved to: {:?}",
        identities_dir.join(format!("{}.json", name))
    ));

    Ok(())
}
