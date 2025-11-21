//! Switch command - change active identity

use anyhow::Result;
use paykit_demo_core::IdentityManager;
use std::path::Path;

use crate::ui;

pub async fn run(storage_dir: &Path, name: &str, _verbose: bool) -> Result<()> {
    let identities_dir = storage_dir.join("identities");
    let identity_manager = IdentityManager::new(&identities_dir);

    // Verify identity exists
    let identity = identity_manager.load(name)?;

    // Set as current
    super::set_current_identity(storage_dir, name)?;

    ui::success(&format!("Switched to identity '{}'", name));
    ui::key_value("Pubky URI", &identity.pubky_uri());

    Ok(())
}
