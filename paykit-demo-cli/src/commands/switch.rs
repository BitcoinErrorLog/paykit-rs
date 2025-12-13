//! Switch command - change active identity

use anyhow::Result;
use paykit_demo_core::{IdentityManager, SecureIdentityManager};
use std::path::Path;

use crate::ui;

pub async fn run(storage_dir: &Path, name: &str, _verbose: bool) -> Result<()> {
    // Try secure storage first
    let metadata_path = storage_dir.join("identities_metadata.json");
    let using_secure_storage = metadata_path.exists();

    // Verify identity exists
    let identity = if using_secure_storage {
        let secure_manager = SecureIdentityManager::new(storage_dir);
        secure_manager.load(name).await?
    } else {
        let identities_dir = storage_dir.join("identities");
        let identity_manager = IdentityManager::new(&identities_dir);
        identity_manager.load(name)?
    };

    // Set as current
    super::set_current_identity(storage_dir, name)?;

    ui::success(&format!("Switched to identity '{}'", name));
    ui::key_value("Pubky URI", &identity.pubky_uri());

    Ok(())
}
