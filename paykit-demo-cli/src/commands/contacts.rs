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
