//! Profile command - fetch and publish Pubky profiles
//!
//! Provides parity with mobile demo's ProfileImportView functionality.

use anyhow::{Context, Result};
use paykit_demo_core::DirectoryClient;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::ui;

/// Profile data from Pubky directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
}

impl Profile {
    pub fn new(name: String) -> Self {
        Self {
            name: Some(name),
            bio: None,
            image: None,
            status: None,
            links: Vec::new(),
        }
    }
}

/// Fetch a profile from the Pubky directory
#[tracing::instrument(skip(_storage_dir))]
pub async fn fetch(
    _storage_dir: &Path,
    uri: &str,
    homeserver: &str,
    output_json: bool,
    verbose: bool,
) -> Result<()> {
    if !output_json {
        ui::header("Fetch Profile");
    }

    tracing::debug!("Parsing URI: {}", uri);
    let public_key = parse_pubky_uri(uri)?;

    if verbose {
        ui::info(&format!("Querying: {}", uri));
        ui::info(&format!("Homeserver: {}", homeserver));
    }

    let client = DirectoryClient::new(homeserver);
    let spinner = if !output_json {
        Some(ui::spinner("Fetching profile..."))
    } else {
        None
    };

    // Query profile.json from the directory
    let profile_path = "/pub/pubky.app/profile.json";

    match client.get_raw(&public_key, profile_path).await {
        Ok(Some(data)) => {
            if let Some(s) = spinner {
                s.finish_and_clear();
            }

            match serde_json::from_str::<Profile>(&data) {
                Ok(profile) => {
                    if output_json {
                        println!("{}", serde_json::to_string_pretty(&profile)?);
                    } else {
                        display_profile(&profile, &public_key.to_string());
                    }
                }
                Err(e) => {
                    if output_json {
                        println!("{{\"error\": \"Failed to parse profile: {}\"}}", e);
                    } else {
                        ui::error(&format!("Failed to parse profile: {}", e));
                        if verbose {
                            ui::info(&format!("Raw data: {}", data));
                        }
                    }
                }
            }
        }
        Ok(None) => {
            if let Some(s) = spinner {
                s.finish_and_clear();
            }
            if output_json {
                println!("{{\"error\": \"Profile not found\"}}");
            } else {
                ui::info("No profile found for this public key");
            }
        }
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_and_clear();
            }
            if output_json {
                println!("{{\"error\": \"{}\"}}", e);
            } else {
                ui::error(&format!("Failed to fetch profile: {}", e));
            }
        }
    }

    Ok(())
}

/// Publish a profile to the Pubky directory
#[tracing::instrument(skip(storage_dir))]
pub async fn publish(
    storage_dir: &Path,
    name: String,
    bio: Option<String>,
    image: Option<String>,
    links: Vec<String>,
    homeserver: &str,
    verbose: bool,
) -> Result<()> {
    ui::header("Publish Profile");

    tracing::debug!("Loading identity for publishing");
    let identity = super::load_current_identity(storage_dir).await?;

    if verbose {
        ui::info(&format!("Using identity: {}", identity.pubky_uri()));
        ui::info(&format!("Homeserver: {}", homeserver));
    }

    // Build profile
    let mut profile = Profile::new(name.clone());
    profile.bio = bio.clone();
    profile.image = image.clone();
    profile.links = links.clone();

    ui::info("Profile to publish:");
    ui::key_value("  Name", &name);
    if let Some(ref b) = bio {
        ui::key_value("  Bio", b);
    }
    if let Some(ref i) = image {
        ui::key_value("  Image", i);
    }
    if !links.is_empty() {
        ui::key_value("  Links", &links.join(", "));
    }
    ui::separator();

    let client = DirectoryClient::new(homeserver);

    // Create session
    let spinner = ui::spinner("Connecting to homeserver...");
    let use_testnet = true;

    let session = match client.create_session(&identity.keypair, use_testnet).await {
        Ok(session) => {
            spinner.finish_and_clear();
            tracing::info!("Session created successfully");
            session
        }
        Err(e) => {
            spinner.finish_and_clear();
            ui::error(&format!("Failed to create session: {}", e));
            return Err(e).context("Failed to establish session with homeserver");
        }
    };

    // Publish profile
    let spinner = ui::spinner("Publishing profile...");
    let profile_json = serde_json::to_string(&profile)?;
    let profile_path = "/pub/pubky.app/profile.json";

    match client.put_raw(&session, profile_path, &profile_json).await {
        Ok(()) => {
            spinner.finish_and_clear();
            ui::separator();
            ui::success("Profile published successfully!");
            ui::info(&format!("Discoverable at: {}", identity.pubky_uri()));
        }
        Err(e) => {
            spinner.finish_and_clear();
            ui::error(&format!("Failed to publish profile: {}", e));
            return Err(e).context("Failed to publish profile");
        }
    }

    Ok(())
}

/// Import a profile from another pubkey and publish as your own
#[tracing::instrument(skip(storage_dir))]
pub async fn import(
    storage_dir: &Path,
    from_uri: &str,
    homeserver: &str,
    verbose: bool,
) -> Result<()> {
    ui::header("Import Profile");

    tracing::debug!("Parsing source URI: {}", from_uri);
    let source_pubkey = parse_pubky_uri(from_uri)?;

    if verbose {
        ui::info(&format!("Importing from: {}", from_uri));
    }

    let client = DirectoryClient::new(homeserver);

    // Fetch source profile
    let spinner = ui::spinner("Fetching source profile...");
    let profile_path = "/pub/pubky.app/profile.json";

    let profile = match client.get_raw(&source_pubkey, profile_path).await {
        Ok(Some(data)) => {
            spinner.finish_and_clear();
            serde_json::from_str::<Profile>(&data).context("Failed to parse source profile")?
        }
        Ok(None) => {
            spinner.finish_and_clear();
            ui::error("Source profile not found");
            return Ok(());
        }
        Err(e) => {
            spinner.finish_and_clear();
            ui::error(&format!("Failed to fetch source profile: {}", e));
            return Err(e).context("Failed to fetch source profile");
        }
    };

    ui::success("Found profile:");
    display_profile(&profile, &source_pubkey.to_string());
    ui::separator();

    // Load our identity
    let identity = super::load_current_identity(storage_dir).await?;
    ui::info(&format!("Will publish to: {}", identity.pubky_uri()));

    // Create session and publish
    let spinner = ui::spinner("Connecting to homeserver...");
    let use_testnet = true;

    let session = match client.create_session(&identity.keypair, use_testnet).await {
        Ok(session) => {
            spinner.finish_and_clear();
            session
        }
        Err(e) => {
            spinner.finish_and_clear();
            ui::error(&format!("Failed to create session: {}", e));
            return Err(e).context("Failed to establish session with homeserver");
        }
    };

    let spinner = ui::spinner("Publishing imported profile...");
    let profile_json = serde_json::to_string(&profile)?;

    match client.put_raw(&session, profile_path, &profile_json).await {
        Ok(()) => {
            spinner.finish_and_clear();
            ui::separator();
            ui::success("Profile imported and published successfully!");
            ui::info(&format!("Now discoverable at: {}", identity.pubky_uri()));
        }
        Err(e) => {
            spinner.finish_and_clear();
            ui::error(&format!("Failed to publish profile: {}", e));
            return Err(e).context("Failed to publish profile");
        }
    }

    Ok(())
}

fn display_profile(profile: &Profile, pubkey: &str) {
    ui::separator();
    if let Some(ref name) = profile.name {
        ui::key_value("Name", name);
    } else {
        ui::key_value("Name", "(not set)");
    }

    ui::key_value(
        "Pubkey",
        &format!("{}...{}", &pubkey[..8], &pubkey[pubkey.len() - 8..]),
    );

    if let Some(ref bio) = profile.bio {
        ui::key_value("Bio", bio);
    }

    if let Some(ref image) = profile.image {
        ui::key_value("Image", image);
    }

    if let Some(ref status) = profile.status {
        ui::key_value("Status", status);
    }

    if !profile.links.is_empty() {
        ui::info("Links:");
        for link in &profile.links {
            ui::info(&format!("  - {}", link));
        }
    }
}

fn parse_pubky_uri(uri: &str) -> Result<pubky::PublicKey> {
    let key_str = uri.strip_prefix("pubky://").unwrap_or(uri);
    // Remove any path components
    let key_str = key_str.split('/').next().unwrap_or(key_str);
    key_str.parse().context("Invalid Pubky URI format")
}
