//! Whoami command - show current identity

use anyhow::Result;
use std::path::Path;

use crate::ui;

pub async fn run(storage_dir: &Path, _verbose: bool) -> Result<()> {
    match super::load_current_identity(storage_dir).await {
        Ok(identity) => {
            ui::header("Current Identity");
            if let Some(nickname) = &identity.nickname {
                ui::key_value("Name", nickname);
            }
            ui::key_value("Public Key", &identity.public_key().to_string());
            ui::key_value("Pubky URI", &identity.pubky_uri());

            println!();
            ui::qr_code(&identity.pubky_uri())?;
        }
        Err(_) => {
            ui::error("No identity configured");
            ui::info("Run 'paykit-demo setup' to create an identity");
        }
    }

    Ok(())
}
