//! Private endpoint management commands
//!
//! This module provides commands for managing private endpoints that have been
//! exchanged with peers during Noise protocol handshakes.

use anyhow::{Context, Result};
use paykit_lib::private_endpoints::{
    encryption, FileStore, PrivateEndpointManager, PrivateEndpointStore,
};
use paykit_lib::{MethodId, PublicKey};
use std::path::Path;
use std::str::FromStr;

use crate::ui;

/// List all private endpoints
pub async fn list(storage_dir: &Path, verbose: bool) -> Result<()> {
    ui::header("Private Endpoints");

    let manager = load_endpoint_manager(storage_dir)?;
    let store = manager.store();

    // Get all peers
    let peers = store.list_peers().await.context("Failed to list peers")?;

    if peers.is_empty() {
        ui::info("No private endpoints stored.");
        ui::info("Private endpoints are exchanged during Noise protocol handshakes.");
        return Ok(());
    }

    ui::success(&format!(
        "Found {} peer(s) with private endpoints",
        peers.len()
    ));
    ui::separator();

    for peer in peers {
        let endpoints = store
            .list_for_peer(&peer)
            .await
            .context("Failed to list endpoints for peer")?;

        ui::key_value("Peer", &peer.to_string());

        for endpoint in endpoints {
            let status = if endpoint.is_expired() {
                "EXPIRED"
            } else if let Some(expires) = endpoint.expires_at {
                let remaining = expires - chrono::Utc::now().timestamp();
                if remaining < 3600 {
                    "expiring soon"
                } else {
                    "active"
                }
            } else {
                "active"
            };

            if verbose {
                ui::key_value(
                    &format!("  {} [{}]", endpoint.method_id.0, status),
                    &endpoint.endpoint.0,
                );
                if let Some(expires) = endpoint.expires_at {
                    let dt = chrono::DateTime::from_timestamp(expires, 0)
                        .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    ui::info(&format!("    Expires: {}", dt));
                }
                ui::info(&format!("    Use count: {}", endpoint.use_count));
            } else {
                ui::key_value(&format!("  {}", endpoint.method_id.0), status);
            }
        }
        ui::separator();
    }

    // Show summary
    let total_count = store.count().await.unwrap_or(0);

    // Count expired endpoints by iterating through all peers
    let mut expired_count = 0;
    let peers_for_count = store.list_peers().await.unwrap_or_default();
    for peer in &peers_for_count {
        let endpoints = store.list_for_peer(peer).await.unwrap_or_default();
        expired_count += endpoints.iter().filter(|e| e.is_expired()).count();
    }

    ui::info(&format!("Total endpoints: {}", total_count));
    if expired_count > 0 {
        ui::warning(&format!(
            "Expired endpoints: {} (run 'endpoints cleanup' to remove)",
            expired_count
        ));
    }

    Ok(())
}

/// Show endpoints for a specific peer
pub async fn show(storage_dir: &Path, peer: &str, verbose: bool) -> Result<()> {
    ui::header("Peer Endpoints");

    let peer_pk = PublicKey::from_str(peer).context("Invalid peer public key")?;
    let manager = load_endpoint_manager(storage_dir)?;
    let store = manager.store();

    let endpoints = store
        .list_for_peer(&peer_pk)
        .await
        .context("Failed to list endpoints for peer")?;

    if endpoints.is_empty() {
        ui::info(&format!("No private endpoints found for peer: {}", peer));
        return Ok(());
    }

    ui::key_value("Peer", peer);
    ui::separator();

    for endpoint in endpoints {
        ui::key_value("Method", &endpoint.method_id.0);
        ui::key_value("Endpoint", &endpoint.endpoint.0);

        let status = if endpoint.is_expired() {
            "❌ EXPIRED"
        } else {
            "✓ Active"
        };
        ui::key_value("Status", status);

        if let Some(expires) = endpoint.expires_at {
            let dt = chrono::DateTime::from_timestamp(expires, 0)
                .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            ui::key_value("Expires", &dt);

            if !endpoint.is_expired() {
                let remaining = expires - chrono::Utc::now().timestamp();
                let hours = remaining / 3600;
                let minutes = (remaining % 3600) / 60;
                ui::key_value("Remaining", &format!("{}h {}m", hours, minutes));
            }
        } else {
            ui::key_value("Expires", "Never");
        }

        ui::key_value("Use count", &endpoint.use_count.to_string());

        if verbose {
            let created = chrono::DateTime::from_timestamp(endpoint.created_at, 0)
                .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            ui::key_value("Created", &created);

            if let Some(last_used) = endpoint.last_used_at {
                let last = chrono::DateTime::from_timestamp(last_used, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                ui::key_value("Last used", &last);
            }
        }

        ui::separator();
    }

    Ok(())
}

/// Remove a specific endpoint
pub async fn remove(storage_dir: &Path, peer: &str, method: &str, verbose: bool) -> Result<()> {
    ui::header("Remove Private Endpoint");

    let peer_pk = PublicKey::from_str(peer).context("Invalid peer public key")?;
    let method_id = MethodId::new(method);
    let manager = load_endpoint_manager(storage_dir)?;
    let store = manager.store();

    // Check if endpoint exists
    let existing = store
        .get(&peer_pk, &method_id)
        .await
        .context("Failed to check endpoint")?;

    if existing.is_none() {
        ui::warning(&format!(
            "No endpoint found for peer {} method {}",
            peer, method
        ));
        return Ok(());
    }

    if verbose {
        ui::info(&format!("Removing endpoint for peer: {}", peer));
        ui::info(&format!("Method: {}", method));
    }

    store
        .remove(&peer_pk, &method_id)
        .await
        .context("Failed to remove endpoint")?;

    ui::success("Endpoint removed successfully");

    Ok(())
}

/// Remove all endpoints for a peer
pub async fn remove_peer(storage_dir: &Path, peer: &str, verbose: bool) -> Result<()> {
    ui::header("Remove All Peer Endpoints");

    let peer_pk = PublicKey::from_str(peer).context("Invalid peer public key")?;
    let manager = load_endpoint_manager(storage_dir)?;
    let store = manager.store();

    // Check how many endpoints exist
    let endpoints = store
        .list_for_peer(&peer_pk)
        .await
        .context("Failed to list endpoints")?;

    if endpoints.is_empty() {
        ui::warning(&format!("No endpoints found for peer: {}", peer));
        return Ok(());
    }

    if verbose {
        ui::info(&format!(
            "Removing {} endpoint(s) for peer: {}",
            endpoints.len(),
            peer
        ));
    }

    store
        .remove_all_for_peer(&peer_pk)
        .await
        .context("Failed to remove endpoints")?;

    ui::success(&format!("Removed {} endpoint(s) for peer", endpoints.len()));

    Ok(())
}

/// Cleanup expired endpoints
pub async fn cleanup(storage_dir: &Path, verbose: bool) -> Result<()> {
    ui::header("Cleanup Expired Endpoints");

    let manager = load_endpoint_manager(storage_dir)?;
    let store = manager.store();

    // Count expired endpoints
    let peers = store.list_peers().await.context("Failed to list peers")?;
    let mut expired_count = 0;
    for peer in &peers {
        let endpoints = store.list_for_peer(peer).await.unwrap_or_default();
        expired_count += endpoints.iter().filter(|e| e.is_expired()).count();
    }

    if expired_count == 0 {
        ui::info("No expired endpoints to clean up.");
        return Ok(());
    }

    if verbose {
        ui::info(&format!("Found {} expired endpoint(s)", expired_count));
    }

    let removed = store
        .cleanup_expired()
        .await
        .context("Failed to cleanup expired endpoints")?;

    ui::success(&format!("Removed {} expired endpoint(s)", removed));

    Ok(())
}

/// Show endpoint statistics
pub async fn stats(storage_dir: &Path, _verbose: bool) -> Result<()> {
    ui::header("Private Endpoint Statistics");

    let manager = load_endpoint_manager(storage_dir)?;
    let store = manager.store();

    let total = store.count().await.unwrap_or(0);
    let peers_list = store.list_peers().await.unwrap_or_default();
    let peers = peers_list.len();

    // Count expired endpoints
    let mut expired = 0;
    for peer in &peers_list {
        let endpoints = store.list_for_peer(peer).await.unwrap_or_default();
        expired += endpoints.iter().filter(|e| e.is_expired()).count();
    }
    let active = total.saturating_sub(expired);

    ui::key_value("Total endpoints", &total.to_string());
    ui::key_value("Active endpoints", &active.to_string());
    ui::key_value("Expired endpoints", &expired.to_string());
    ui::key_value("Unique peers", &peers.to_string());

    if total > 0 {
        ui::separator();

        // Count by method
        let mut method_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for peer in &peers_list {
            let endpoints = store.list_for_peer(peer).await.unwrap_or_default();
            for ep in endpoints {
                *method_counts.entry(ep.method_id.0.clone()).or_insert(0) += 1;
            }
        }

        ui::info("Endpoints by method:");
        for (method, count) in method_counts {
            ui::key_value(&format!("  {}", method), &count.to_string());
        }
    }

    Ok(())
}

/// Load the private endpoint manager
fn load_endpoint_manager(storage_dir: &Path) -> Result<PrivateEndpointManager<FileStore>> {
    let endpoints_dir = storage_dir.join("private_endpoints");
    let key_path = storage_dir.join(".endpoint_key");

    // Load or generate encryption key
    let key = if key_path.exists() {
        let key_bytes =
            std::fs::read(&key_path).context("Failed to read endpoint encryption key")?;
        if key_bytes.len() != 32 {
            anyhow::bail!("Invalid endpoint encryption key size");
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        key
    } else {
        // Generate new key and save
        let key = encryption::generate_key();
        std::fs::create_dir_all(storage_dir)?;
        std::fs::write(&key_path, *key).context("Failed to save endpoint encryption key")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).ok();
        }
        *key
    };

    let store =
        FileStore::new_encrypted(&endpoints_dir, key).context("Failed to create endpoint store")?;
    Ok(PrivateEndpointManager::new(store))
}
