//! Wallet configuration commands
//!
//! Configure payment execution backends (LND, Esplora) for real payments.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::ui;

/// Wallet configuration stored on disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WalletConfig {
    /// LND configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lnd: Option<LndConfig>,
    /// Esplora configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub esplora: Option<EsploraConfig>,
    /// Network (mainnet, testnet, signet, regtest)
    #[serde(default = "default_network")]
    pub network: String,
}

fn default_network() -> String {
    "testnet".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LndConfig {
    /// REST API URL
    pub url: String,
    /// Macaroon in hex format
    pub macaroon: String,
    /// TLS certificate (optional, PEM format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_cert: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsploraConfig {
    /// API base URL
    pub url: String,
}

impl WalletConfig {
    /// Load wallet configuration from disk
    pub fn load(storage_dir: &Path) -> Result<Option<Self>> {
        let config_path = storage_dir.join("wallet.json");
        if !config_path.exists() {
            return Ok(None);
        }

        let contents =
            std::fs::read_to_string(&config_path).context("Failed to read wallet configuration")?;
        let config: Self =
            serde_json::from_str(&contents).context("Failed to parse wallet configuration")?;
        Ok(Some(config))
    }

    /// Save wallet configuration to disk
    pub fn save(&self, storage_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(storage_dir)?;
        let config_path = storage_dir.join("wallet.json");
        let contents =
            serde_json::to_string_pretty(self).context("Failed to serialize wallet config")?;
        std::fs::write(&config_path, contents).context("Failed to write wallet configuration")?;
        Ok(())
    }

    /// Check if any executor is configured
    #[allow(dead_code)]
    pub fn is_configured(&self) -> bool {
        self.lnd.is_some() || self.esplora.is_some()
    }

    /// Check if Lightning payments are configured
    pub fn has_lightning(&self) -> bool {
        self.lnd.is_some()
    }

    /// Check if on-chain payments are configured
    pub fn has_onchain(&self) -> bool {
        self.esplora.is_some()
    }
}

/// Configure LND for Lightning payments
pub async fn configure_lnd(
    storage_dir: &Path,
    url: &str,
    macaroon: &str,
    tls_cert: Option<&str>,
    network: Option<&str>,
    _verbose: bool,
) -> Result<()> {
    ui::header("Configure LND");

    // Validate inputs
    if url.is_empty() {
        anyhow::bail!("LND URL is required");
    }
    if macaroon.is_empty() {
        anyhow::bail!("Macaroon is required");
    }

    // Validate macaroon is valid hex
    if hex::decode(macaroon).is_err() {
        anyhow::bail!("Macaroon must be valid hexadecimal");
    }

    // Load existing config or create new
    let mut config = WalletConfig::load(storage_dir)?.unwrap_or_default();

    // Update LND config
    config.lnd = Some(LndConfig {
        url: url.to_string(),
        macaroon: macaroon.to_string(),
        tls_cert: tls_cert.map(|s| s.to_string()),
    });

    // Update network if provided
    if let Some(net) = network {
        config.network = net.to_string();
    }

    // Save configuration
    config.save(storage_dir)?;

    ui::success("LND configuration saved");
    ui::info(&format!("URL: {}", url));
    ui::info(&format!("Network: {}", config.network));
    ui::info(&format!(
        "Macaroon: {}...{} ({} chars)",
        &macaroon[..8.min(macaroon.len())],
        if macaroon.len() > 8 {
            &macaroon[macaroon.len() - 4..]
        } else {
            ""
        },
        macaroon.len()
    ));

    Ok(())
}

/// Configure Esplora for on-chain payments
pub async fn configure_esplora(
    storage_dir: &Path,
    url: &str,
    network: Option<&str>,
    _verbose: bool,
) -> Result<()> {
    ui::header("Configure Esplora");

    if url.is_empty() {
        anyhow::bail!("Esplora URL is required");
    }

    // Load existing config or create new
    let mut config = WalletConfig::load(storage_dir)?.unwrap_or_default();

    // Update Esplora config
    config.esplora = Some(EsploraConfig {
        url: url.to_string(),
    });

    // Update network if provided
    if let Some(net) = network {
        config.network = net.to_string();
    }

    // Save configuration
    config.save(storage_dir)?;

    ui::success("Esplora configuration saved");
    ui::info(&format!("URL: {}", url));
    ui::info(&format!("Network: {}", config.network));

    Ok(())
}

/// Show wallet status
pub async fn status(storage_dir: &Path, _verbose: bool) -> Result<()> {
    ui::header("Wallet Status");

    let config = WalletConfig::load(storage_dir)?;

    match config {
        None => {
            ui::warning("No wallet configured");
            ui::info("");
            ui::info("Configure a wallet to enable real payments:");
            ui::info("  paykit-demo wallet configure-lnd --url <url> --macaroon <hex>");
            ui::info("  paykit-demo wallet configure-esplora --url <url>");
            ui::info("");
            ui::info("Or use presets:");
            ui::info("  paykit-demo wallet preset polar --macaroon <hex>");
            ui::info("  paykit-demo wallet preset testnet");
        }
        Some(config) => {
            ui::info(&format!("Network: {}", config.network));
            ui::separator();

            if let Some(lnd) = &config.lnd {
                ui::success("Lightning (LND): Configured");
                ui::info(&format!("  URL: {}", lnd.url));
                ui::info(&format!(
                    "  Macaroon: {}...{}",
                    &lnd.macaroon[..8.min(lnd.macaroon.len())],
                    if lnd.macaroon.len() > 8 {
                        &lnd.macaroon[lnd.macaroon.len() - 4..]
                    } else {
                        ""
                    }
                ));
                if lnd.tls_cert.is_some() {
                    ui::info("  TLS: Custom certificate");
                }
            } else {
                ui::warning("Lightning (LND): Not configured");
            }

            ui::separator();

            if let Some(esplora) = &config.esplora {
                ui::success("On-chain (Esplora): Configured");
                ui::info(&format!("  URL: {}", esplora.url));
            } else {
                ui::warning("On-chain (Esplora): Not configured");
            }
        }
    }

    Ok(())
}

/// Apply a preset configuration
pub async fn apply_preset(
    storage_dir: &Path,
    preset: &str,
    macaroon: Option<&str>,
    _verbose: bool,
) -> Result<()> {
    ui::header(&format!("Apply Preset: {}", preset));

    let mut config = WalletConfig::load(storage_dir)?.unwrap_or_default();

    match preset.to_lowercase().as_str() {
        "polar" => {
            if macaroon.is_none() {
                anyhow::bail!("Polar preset requires --macaroon argument");
            }
            config.network = "regtest".to_string();
            config.lnd = Some(LndConfig {
                url: "https://127.0.0.1:8081".to_string(),
                macaroon: macaroon.unwrap().to_string(),
                tls_cert: None,
            });
            config.esplora = Some(EsploraConfig {
                url: "http://localhost:3002/api".to_string(),
            });
            ui::success("Polar preset applied (regtest)");
            ui::info("LND: https://127.0.0.1:8081 (Alice node)");
            ui::info("Esplora: http://localhost:3002/api");
        }
        "testnet" => {
            config.network = "testnet".to_string();
            config.esplora = Some(EsploraConfig {
                url: "https://blockstream.info/testnet/api".to_string(),
            });
            // LND requires user configuration
            ui::success("Testnet preset applied");
            ui::info("Esplora: https://blockstream.info/testnet/api");
            ui::warning("LND not configured - add manually with --lnd-url if needed");
        }
        "signet" => {
            config.network = "signet".to_string();
            config.esplora = Some(EsploraConfig {
                url: "https://mempool.space/signet/api".to_string(),
            });
            ui::success("Signet preset applied");
            ui::info("Esplora: https://mempool.space/signet/api");
        }
        "mutinynet" => {
            config.network = "signet".to_string();
            config.esplora = Some(EsploraConfig {
                url: "https://mutinynet.com/api".to_string(),
            });
            ui::success("Mutinynet preset applied");
            ui::info("Esplora: https://mutinynet.com/api");
        }
        _ => {
            anyhow::bail!(
                "Unknown preset: {}. Available: polar, testnet, signet, mutinynet",
                preset
            );
        }
    }

    config.save(storage_dir)?;
    Ok(())
}

/// Clear wallet configuration
pub async fn clear(storage_dir: &Path, _verbose: bool) -> Result<()> {
    ui::header("Clear Wallet Configuration");

    let config_path = storage_dir.join("wallet.json");
    if config_path.exists() {
        std::fs::remove_file(&config_path)?;
        ui::success("Wallet configuration cleared");
    } else {
        ui::info("No wallet configuration to clear");
    }

    Ok(())
}

/// Check health of configured payment methods
pub async fn health(storage_dir: &Path, method: Option<String>, verbose: bool) -> Result<()> {
    use paykit_lib::health::{HealthMonitor, LightningHealthChecker, OnchainHealthChecker};
    use paykit_lib::MethodId;

    ui::header("Payment Method Health Check");

    let config = WalletConfig::load(storage_dir)?;

    if config.is_none() {
        ui::warning("No wallet configured");
        ui::info("Configure a wallet first with 'paykit-demo wallet configure-lnd' or 'configure-esplora'");
        return Ok(());
    }

    let config = config.unwrap();

    // Create health monitor with configured checkers
    let mut monitor = HealthMonitor::new();

    if config.lnd.is_some() {
        let lnd_url = config.lnd.as_ref().map(|c| c.url.clone());
        monitor.register(Box::new(LightningHealthChecker::new(lnd_url)));
    }

    if config.esplora.is_some() {
        let esplora_url = config.esplora.as_ref().map(|c| c.url.clone());
        monitor.register(Box::new(OnchainHealthChecker::new(esplora_url)));
    }

    // Check specific method or all
    if let Some(method_id) = method {
        let method = MethodId(method_id.clone());
        let spinner = ui::spinner(&format!("Checking {} health...", method_id));

        if let Some(result) = monitor.check(&method).await {
            spinner.finish_and_clear();
            display_health_result(&result, verbose);
        } else {
            spinner.finish_and_clear();
            ui::warning(&format!("No health checker for method: {}", method_id));
        }
    } else {
        let spinner = ui::spinner("Checking all payment methods...");
        let results = monitor.check_all().await;
        spinner.finish_and_clear();

        if results.is_empty() {
            ui::warning("No payment methods configured to check");
            return Ok(());
        }

        for result in &results {
            display_health_result(result, verbose);
            println!();
        }

        // Summary
        ui::separator();
        let healthy = results.iter().filter(|r| r.status.is_healthy()).count();
        let usable = results.iter().filter(|r| r.status.is_usable()).count();
        let total = results.len();

        if healthy == total {
            ui::success(&format!("All {} methods are healthy", total));
        } else if usable == total {
            ui::warning(&format!(
                "{}/{} methods healthy, all usable",
                healthy, total
            ));
        } else {
            ui::error(&format!(
                "{}/{} methods healthy, {}/{} usable",
                healthy, total, usable, total
            ));
        }
    }

    Ok(())
}

fn display_health_result(result: &paykit_lib::health::HealthCheckResult, verbose: bool) {
    use paykit_lib::health::HealthStatus;

    let status_str = match result.status {
        HealthStatus::Healthy => "✅ Healthy".to_string(),
        HealthStatus::Degraded => "⚠️ Degraded".to_string(),
        HealthStatus::Unavailable => "❌ Unavailable".to_string(),
        HealthStatus::Unknown => "❓ Unknown".to_string(),
    };

    ui::key_value(&result.method_id.0, &status_str);

    if let Some(latency) = result.latency_ms {
        ui::info(&format!("  Latency: {}ms", latency));
    }

    if let Some(error) = &result.error {
        ui::error(&format!("  Error: {}", error));
    }

    if verbose && !result.details.is_null() {
        ui::info(&format!("  Details: {}", result.details));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wallet_config_save_load() {
        let dir = tempdir().unwrap();
        let config = WalletConfig {
            lnd: Some(LndConfig {
                url: "https://localhost:8080".to_string(),
                macaroon: "abc123".to_string(),
                tls_cert: None,
            }),
            esplora: Some(EsploraConfig {
                url: "https://blockstream.info/api".to_string(),
            }),
            network: "testnet".to_string(),
        };

        config.save(dir.path()).unwrap();
        let loaded = WalletConfig::load(dir.path()).unwrap().unwrap();

        assert_eq!(loaded.network, "testnet");
        assert!(loaded.lnd.is_some());
        assert!(loaded.esplora.is_some());
    }

    #[test]
    fn test_wallet_config_is_configured() {
        let config = WalletConfig::default();
        assert!(!config.is_configured());
        assert!(!config.has_lightning());
        assert!(!config.has_onchain());

        let config_with_lnd = WalletConfig {
            lnd: Some(LndConfig {
                url: "https://localhost:8080".to_string(),
                macaroon: "abc123".to_string(),
                tls_cert: None,
            }),
            ..Default::default()
        };
        assert!(config_with_lnd.is_configured());
        assert!(config_with_lnd.has_lightning());
        assert!(!config_with_lnd.has_onchain());
    }
}
