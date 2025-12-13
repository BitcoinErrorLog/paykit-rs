//! Contacts command - manage contact list

use anyhow::{Context, Result};
use colored::Colorize;
use paykit_demo_core::{Contact, DemoStorage};
use std::path::Path;

use crate::ui;

pub async fn add(
    storage_dir: &Path,
    name: &str,
    uri: &str,
    notes: Option<&str>,
    verbose: bool,
) -> Result<()> {
    ui::header("Add Contact");

    // Parse public key from URI
    let key_str = uri.strip_prefix("pubky://").unwrap_or(uri);
    let public_key: pubky::PublicKey = key_str.parse().context("Invalid Pubky URI")?;

    // Create contact
    let mut contact = Contact::new(public_key, name.to_string());
    if let Some(n) = notes {
        contact = contact.with_notes(n.to_string());
    }

    if verbose {
        ui::info(&format!("Adding contact: {}", name));
        ui::info(&format!("URI: {}", uri));
    }

    // Save to storage
    let storage = DemoStorage::new(storage_dir.join("data"));
    storage.init()?;
    storage.save_contact(contact)?;

    ui::success(&format!("Contact '{}' added", name));

    Ok(())
}

pub async fn list(storage_dir: &Path, _verbose: bool) -> Result<()> {
    ui::header("Contacts");

    let storage = DemoStorage::new(storage_dir.join("data"));
    let contacts = storage.list_contacts()?;

    if contacts.is_empty() {
        ui::info("No contacts found");
        ui::info("Use 'paykit-demo contacts add' to add contacts");
        return Ok(());
    }

    for contact in contacts {
        println!("\n{}", contact.name.bold());
        ui::key_value("  URI", &contact.pubky_uri());
        if let Some(notes) = &contact.notes {
            ui::key_value("  Notes", notes);
        }
        ui::key_value(
            "  Added",
            &chrono::DateTime::from_timestamp(contact.added_at, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
        );
    }

    Ok(())
}

pub async fn remove(storage_dir: &Path, name: &str, verbose: bool) -> Result<()> {
    if verbose {
        ui::info(&format!("Removing contact: {}", name));
    }

    let storage = DemoStorage::new(storage_dir.join("data"));

    // Try to find contact by name first
    let contacts = storage.list_contacts()?;
    let contact = contacts
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| anyhow::anyhow!("Contact '{}' not found", name))?;

    let public_key_str = contact.public_key.to_string();
    storage.delete_contact(&public_key_str)?;

    ui::success(&format!("Contact '{}' removed", name));

    Ok(())
}

pub async fn show(storage_dir: &Path, name: &str, _verbose: bool) -> Result<()> {
    ui::header(&format!("Contact: {}", name));

    let storage = DemoStorage::new(storage_dir.join("data"));
    let contacts = storage.list_contacts()?;

    let contact = contacts
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| anyhow::anyhow!("Contact '{}' not found", name))?;

    ui::key_value("Name", &contact.name);
    ui::key_value("Public Key", &contact.public_key.to_string());
    ui::key_value("Pubky URI", &contact.pubky_uri());

    if let Some(notes) = &contact.notes {
        ui::key_value("Notes", notes);
    }

    ui::key_value(
        "Added",
        &chrono::DateTime::from_timestamp(contact.added_at, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
    );

    // Show QR code
    println!();
    ui::qr_code(&contact.pubky_uri())?;

    Ok(())
}

/// Discover contacts from Pubky follows directory
pub async fn discover(
    storage_dir: &Path,
    import: bool,
    _homeserver: &str,
    verbose: bool,
) -> Result<()> {
    use paykit_demo_core::DirectoryClient;

    ui::header("Discover Contacts");

    // Load current identity
    let identity = super::load_current_identity(storage_dir)?;

    ui::info(&format!(
        "Discovering contacts for: {}",
        identity.pubky_uri()
    ));

    let spinner = ui::spinner("Querying follows directory...");

    // Use the directory client to discover known contacts
    let _client = DirectoryClient::new("8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo");

    // Try to get known contacts from the follows path
    // This uses the UnauthenticatedTransport to query the public directory
    let storage = pubky::PublicStorage::new().context("Failed to create PublicStorage")?;
    let transport = paykit_lib::PubkyUnauthenticatedTransport::new(storage);

    let discovered = paykit_lib::get_known_contacts(&transport, &identity.public_key())
        .await
        .context("Failed to query follows directory")?;

    spinner.finish_and_clear();

    if discovered.is_empty() {
        ui::info("No contacts found in follows directory");
        ui::info("");
        ui::info("To add contacts manually:");
        ui::info("  paykit-demo contacts add <name> <pubky_uri>");
        return Ok(());
    }

    ui::success(&format!("Found {} contact(s)", discovered.len()));
    ui::separator();

    let demo_storage = DemoStorage::new(storage_dir.join("data"));
    demo_storage.init()?;
    let existing_contacts = demo_storage.list_contacts()?;

    let mut imported_count = 0;
    let mut already_exists_count = 0;

    for pubkey in &discovered {
        let pubkey_str = pubkey.to_string();
        let already_exists = existing_contacts
            .iter()
            .any(|c| c.public_key.to_string() == pubkey_str);

        if already_exists {
            if verbose {
                ui::info(&format!("  [exists] {}", pubkey_str));
            }
            already_exists_count += 1;
            continue;
        }

        let uri = format!("pubky://{}", pubkey_str);

        if import {
            // Auto-import with a default name
            let name = format!("contact_{}", &pubkey_str[..8.min(pubkey_str.len())]);
            let contact = Contact::new(pubkey.clone(), name.clone());
            demo_storage.save_contact(contact)?;
            ui::success(&format!("  [imported] {} as '{}'", pubkey_str, name));
            imported_count += 1;
        } else {
            ui::info(&format!("  [new] {}", uri));
        }
    }

    ui::separator();

    if import {
        ui::success(&format!(
            "Imported {} new contact(s), {} already existed",
            imported_count, already_exists_count
        ));
    } else {
        let new_count = discovered.len() - already_exists_count;
        if new_count > 0 {
            ui::info(&format!(
                "Found {} new contact(s), {} already in your list",
                new_count, already_exists_count
            ));
            ui::info("");
            ui::info("To import automatically, use --import flag:");
            ui::info("  paykit-demo contacts discover --import");
        } else {
            ui::info("All discovered contacts are already in your list");
        }
    }

    Ok(())
}
