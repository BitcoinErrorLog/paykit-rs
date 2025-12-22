//! Endpoint rotation policy configuration commands
//!
//! This module provides commands for configuring automatic endpoint rotation
//! policies for privacy-enhancing address reuse prevention.

use anyhow::{Context, Result};
use paykit_lib::rotation::{RotationConfig, RotationPolicy};
use paykit_lib::MethodId;
use std::path::Path;

use crate::ui;

/// Show rotation status for all methods
pub async fn status(storage_dir: &Path, verbose: bool) -> Result<()> {
    ui::header("Endpoint Rotation Status");

    let config = load_rotation_config(storage_dir)?;
    let state = load_rotation_state(storage_dir)?;

    // Default policy
    ui::key_value("Default policy", &format_policy(&config.default_policy));
    ui::key_value(
        "Auto-rotate on payment",
        if config.auto_rotate_on_payment {
            "enabled"
        } else {
            "disabled"
        },
    );

    ui::separator();

    // Per-method policies
    if config.method_policies.is_empty() {
        ui::info("No per-method policies configured (using default)");
    } else {
        ui::info("Per-method policies:");
        for (method, policy) in &config.method_policies {
            ui::key_value(&format!("  {}", method), &format_policy(policy));
        }
    }

    ui::separator();

    // Rotation state
    if let Some(state) = state {
        ui::info("Rotation history:");
        if let Some(obj) = state.as_object() {
            for (method, info) in obj {
                if let Some(rotations) = info.get("rotations").and_then(|v| v.as_u64()) {
                    let last_rotated = info
                        .get("last_rotated")
                        .and_then(|v| v.as_i64())
                        .map(|ts| {
                            chrono::DateTime::from_timestamp(ts, 0)
                                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                .unwrap_or_else(|| "unknown".to_string())
                        })
                        .unwrap_or_else(|| "never".to_string());

                    ui::key_value(
                        &format!("  {}", method),
                        &format!("{} rotations, last: {}", rotations, last_rotated),
                    );

                    if verbose {
                        if let Some(pending) = info.get("pending_endpoint").and_then(|v| v.as_str())
                        {
                            ui::info(&format!("    Pending endpoint: {}", pending));
                        }
                    }
                }
            }
        }
    } else {
        ui::info("No rotation history yet");
    }

    Ok(())
}

/// Set rotation policy for a method
pub async fn set_policy(
    storage_dir: &Path,
    method: &str,
    policy_str: &str,
    _verbose: bool,
) -> Result<()> {
    ui::header("Set Rotation Policy");

    let mut config = load_rotation_config(storage_dir)?;

    let policy = parse_policy(policy_str)?;
    let method_id = method.to_string();

    ui::info(&format!("Setting policy for method: {}", method));
    ui::key_value("New policy", &format_policy(&policy));

    config.method_policies.insert(method_id, policy);
    save_rotation_config(storage_dir, &config)?;

    ui::success("Policy updated successfully");

    Ok(())
}

/// Set the default rotation policy
pub async fn set_default(storage_dir: &Path, policy_str: &str, _verbose: bool) -> Result<()> {
    ui::header("Set Default Rotation Policy");

    let mut config = load_rotation_config(storage_dir)?;

    let policy = parse_policy(policy_str)?;

    ui::key_value("New default policy", &format_policy(&policy));

    config.default_policy = policy;
    save_rotation_config(storage_dir, &config)?;

    ui::success("Default policy updated successfully");

    Ok(())
}

/// Enable or disable auto-rotation
pub async fn auto_rotate(storage_dir: &Path, enable: bool, _verbose: bool) -> Result<()> {
    ui::header("Auto-Rotation Setting");

    let mut config = load_rotation_config(storage_dir)?;

    config.auto_rotate_on_payment = enable;
    save_rotation_config(storage_dir, &config)?;

    if enable {
        ui::success("Auto-rotation enabled");
        ui::info("Endpoints will rotate automatically after payments based on policies");
    } else {
        ui::success("Auto-rotation disabled");
        ui::info("Endpoints will only rotate when manually triggered");
    }

    Ok(())
}

/// Manually trigger rotation for a method
pub async fn rotate(storage_dir: &Path, method: &str, verbose: bool) -> Result<()> {
    ui::header("Manual Endpoint Rotation");

    let method_id = MethodId::new(method);
    let config = load_rotation_config(storage_dir)?;

    ui::info(&format!("Rotating endpoint for method: {}", method));

    // Create rotation manager
    let registry = paykit_lib::prelude::default_registry();
    let rotation_manager =
        paykit_lib::rotation::EndpointRotationManager::new(config, registry);

    // Trigger rotation
    match rotation_manager.rotate(&method_id).await {
        Ok(new_endpoint) => {
            ui::success("Endpoint rotated successfully!");
            if verbose {
                ui::key_value("New endpoint", &new_endpoint.0);
            }
            ui::warning("Remember to publish the new endpoint to the directory:");
            ui::info(&format!(
                "  paykit-demo publish --{} \"{}\"",
                method, new_endpoint.0
            ));

            // Update rotation state
            update_rotation_state(storage_dir, &method_id, &new_endpoint)?;
        }
        Err(e) => {
            ui::error(&format!("Rotation failed: {}", e));
            ui::info("This may be because no plugin is available for this method");
        }
    }

    Ok(())
}

/// Show rotation history for auditing
pub async fn history(storage_dir: &Path, method: Option<String>, verbose: bool) -> Result<()> {
    ui::header("Rotation History");

    let state = load_rotation_state(storage_dir)?;

    if state.is_none() {
        ui::info("No rotation history recorded yet.");
        ui::info("Rotations are recorded automatically after payments when auto-rotate is enabled.");
        return Ok(());
    }

    let state = state.unwrap();
    let obj = state
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Invalid rotation state format"))?;

    // Filter by method if specified
    let methods: Vec<&String> = if let Some(ref filter_method) = method {
        obj.keys().filter(|k| *k == filter_method).collect()
    } else {
        obj.keys().collect()
    };

    if methods.is_empty() {
        if method.is_some() {
            ui::info(&format!(
                "No rotation history for method: {}",
                method.unwrap()
            ));
        } else {
            ui::info("No rotation history recorded yet.");
        }
        return Ok(());
    }

    // Summary statistics
    let mut total_rotations: u64 = 0;
    for method_key in &methods {
        if let Some(info) = obj.get(*method_key) {
            total_rotations += info.get("rotations").and_then(|v| v.as_u64()).unwrap_or(0);
        }
    }

    ui::key_value("Total rotations", &total_rotations.to_string());
    ui::key_value("Methods tracked", &methods.len().to_string());
    ui::separator();

    // Per-method history
    for method_key in methods {
        if let Some(info) = obj.get(method_key) {
            let rotations = info.get("rotations").and_then(|v| v.as_u64()).unwrap_or(0);
            let last_rotated = info
                .get("last_rotated")
                .and_then(|v| v.as_i64())
                .map(|ts| {
                    chrono::DateTime::from_timestamp(ts, 0)
                        .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                })
                .unwrap_or_else(|| "never".to_string());

            ui::key_value(&format!("Method: {}", method_key), "");
            ui::key_value("  Total rotations", &rotations.to_string());
            ui::key_value("  Last rotated", &last_rotated);

            if verbose {
                if let Some(pending) = info.get("pending_endpoint").and_then(|v| v.as_str()) {
                    ui::key_value("  Pending endpoint", pending);
                }

                // Show recent rotation events if available
                if let Some(history) = info.get("history").and_then(|v| v.as_array()) {
                    if !history.is_empty() {
                        ui::info("  Recent events:");
                        for (i, event) in history.iter().rev().take(5).enumerate() {
                            if let Some(ts) = event.get("timestamp").and_then(|v| v.as_i64()) {
                                let time = chrono::DateTime::from_timestamp(ts, 0)
                                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                    .unwrap_or_else(|| "unknown".to_string());
                                ui::info(&format!("    {}. {}", i + 1, time));
                            }
                        }
                    }
                }
            }

            ui::separator();
        }
    }

    Ok(())
}

/// Clear rotation history
pub async fn clear_history(storage_dir: &Path, _verbose: bool) -> Result<()> {
    ui::header("Clear Rotation History");

    let state_path = storage_dir.join("rotation_state.json");

    if state_path.exists() {
        std::fs::remove_file(&state_path).context("Failed to remove rotation state")?;
        ui::success("Rotation history cleared");
    } else {
        ui::info("No rotation history to clear");
    }

    Ok(())
}

// Helper functions

fn load_rotation_config(storage_dir: &Path) -> Result<RotationConfig> {
    let config_path = storage_dir.join("rotation_config.json");

    if config_path.exists() {
        let config_str =
            std::fs::read_to_string(&config_path).context("Failed to read rotation config")?;
        serde_json::from_str(&config_str).context("Failed to parse rotation config")
    } else {
        Ok(RotationConfig::default())
    }
}

fn save_rotation_config(storage_dir: &Path, config: &RotationConfig) -> Result<()> {
    std::fs::create_dir_all(storage_dir)?;
    let config_path = storage_dir.join("rotation_config.json");
    let config_str =
        serde_json::to_string_pretty(config).context("Failed to serialize rotation config")?;
    std::fs::write(&config_path, config_str).context("Failed to write rotation config")
}

fn load_rotation_state(storage_dir: &Path) -> Result<Option<serde_json::Value>> {
    let state_path = storage_dir.join("rotation_state.json");

    if state_path.exists() {
        let state_str =
            std::fs::read_to_string(&state_path).context("Failed to read rotation state")?;
        let state = serde_json::from_str(&state_str).context("Failed to parse rotation state")?;
        Ok(Some(state))
    } else {
        Ok(None)
    }
}

fn update_rotation_state(
    storage_dir: &Path,
    method_id: &MethodId,
    new_endpoint: &paykit_lib::EndpointData,
) -> Result<()> {
    let state_path = storage_dir.join("rotation_state.json");

    let mut state: serde_json::Value = if state_path.exists() {
        let state_str =
            std::fs::read_to_string(&state_path).context("Failed to read rotation state")?;
        serde_json::from_str(&state_str).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let method_key = &method_id.0;
    if !state[method_key].is_object() {
        state[method_key] = serde_json::json!({
            "last_rotated": chrono::Utc::now().timestamp(),
            "rotations": 0,
        });
    }

    state[method_key]["last_rotated"] = serde_json::json!(chrono::Utc::now().timestamp());
    state[method_key]["rotations"] =
        serde_json::json!(state[method_key]["rotations"].as_u64().unwrap_or(0) + 1);
    state[method_key]["pending_endpoint"] = serde_json::json!(new_endpoint.0);

    let state_str =
        serde_json::to_string_pretty(&state).context("Failed to serialize rotation state")?;
    std::fs::write(&state_path, state_str).context("Failed to save rotation state")?;

    Ok(())
}

fn parse_policy(policy_str: &str) -> Result<RotationPolicy> {
    let policy_lower = policy_str.to_lowercase();

    if policy_lower == "on-use" || policy_lower == "onuse" {
        return Ok(RotationPolicy::RotateOnUse);
    }

    if policy_lower == "manual" || policy_lower == "never" {
        return Ok(RotationPolicy::Manual);
    }

    if let Some(count_str) = policy_lower.strip_prefix("after:") {
        let count: u32 = count_str
            .parse()
            .context("Invalid use count in policy")?;
        return Ok(RotationPolicy::after_uses(count));
    }

    if let Some(interval_str) = policy_lower.strip_prefix("periodic:") {
        let seconds: u64 = interval_str
            .parse()
            .context("Invalid interval in policy")?;
        return Ok(RotationPolicy::RotatePeriodic {
            interval_secs: seconds,
        });
    }

    anyhow::bail!(
        "Invalid policy format. Use: on-use, manual, after:<count>, or periodic:<seconds>"
    )
}

fn format_policy(policy: &RotationPolicy) -> String {
    match policy {
        RotationPolicy::RotateOnUse => "Rotate on every use (best privacy)".to_string(),
        RotationPolicy::RotateOnThreshold { threshold } => {
            format!("Rotate after {} uses", threshold)
        }
        RotationPolicy::RotatePeriodic { interval_secs } => {
            format!("Rotate every {} seconds", interval_secs)
        }
        RotationPolicy::Manual => "Manual rotation only".to_string(),
    }
}

