//! CLI command implementations

pub mod contacts;
pub mod discover;
pub mod list;
pub mod pay;
pub mod publish;
pub mod receipts;
pub mod receive;
pub mod setup;
pub mod subscriptions;
pub mod switch;
pub mod whoami;

/// Get the path to the current identity marker file
pub fn current_identity_path(storage_dir: &std::path::Path) -> std::path::PathBuf {
    storage_dir.join(".current_identity")
}

/// Get the current identity name
pub fn get_current_identity(storage_dir: &std::path::Path) -> anyhow::Result<Option<String>> {
    let path = current_identity_path(storage_dir);
    if !path.exists() {
        return Ok(None);
    }
    let name = std::fs::read_to_string(path)?;
    Ok(Some(name.trim().to_string()))
}

/// Set the current identity
pub fn set_current_identity(storage_dir: &std::path::Path, name: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(storage_dir)?;
    let path = current_identity_path(storage_dir);
    std::fs::write(path, name)?;
    Ok(())
}

/// Load the current identity
pub fn load_current_identity(
    storage_dir: &std::path::Path,
) -> anyhow::Result<paykit_demo_core::Identity> {
    let name = get_current_identity(storage_dir)?
        .ok_or_else(|| anyhow::anyhow!("No identity configured. Run 'paykit-demo setup' first."))?;

    let identity_manager = paykit_demo_core::IdentityManager::new(storage_dir.join("identities"));
    identity_manager.load(&name)
}
