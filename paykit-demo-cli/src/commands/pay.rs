//! Pay command - initiate payment
//!
//! Executes real payments when a wallet is configured, otherwise shows simulation.

use anyhow::Result;
use paykit_demo_core::DemoStorage;
use std::path::Path;

use super::wallet::WalletConfig;
use crate::ui;

#[tracing::instrument(skip(storage_dir))]
pub async fn run(
    storage_dir: &Path,
    recipient: &str,
    amount: Option<String>,
    currency: Option<String>,
    method: &str,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    ui::header("Initiate Payment");

    tracing::debug!("Loading current identity");
    // Load current identity
    let identity = super::load_current_identity(storage_dir)?;

    if verbose {
        ui::info(&format!("Payer: {}", identity.pubky_uri()));
        tracing::info!("Payer: {}", identity.pubky_uri());
    }

    // Resolve recipient (could be contact name or URI)
    let payee_uri = resolve_recipient(storage_dir, recipient)?;

    ui::info(&format!("Recipient: {}", payee_uri));
    ui::info(&format!("Method: {}", method));

    let amount_str = if let Some(amt) = &amount {
        if let Some(curr) = &currency {
            ui::info(&format!("Amount: {} {}", amt, curr));
            format!("{} {}", amt, curr)
        } else {
            ui::info(&format!("Amount: {} SAT", amt));
            format!("{} SAT", amt)
        }
    } else {
        ui::info("Amount: Unspecified");
        "unspecified".to_string()
    };

    ui::separator();

    // Check wallet configuration
    let wallet_config = WalletConfig::load(storage_dir)?;

    match method.to_lowercase().as_str() {
        "lightning" | "ln" | "ln-btc" => {
            execute_lightning_payment(
                storage_dir,
                &wallet_config,
                &payee_uri,
                amount.as_deref(),
                dry_run,
                verbose,
            )
            .await?;
        }
        "onchain" | "btc" | "onchain-btc" => {
            execute_onchain_payment(
                storage_dir,
                &wallet_config,
                &payee_uri,
                amount.as_deref(),
                dry_run,
                verbose,
            )
            .await?;
        }
        _ => {
            ui::warning(&format!("Unknown payment method: {}", method));
            ui::info("Supported methods: lightning, onchain");
            return Ok(());
        }
    }

    // Log payment attempt
    if !dry_run {
        log_payment_attempt(storage_dir, &payee_uri, &amount_str, method)?;
    }

    Ok(())
}

async fn execute_lightning_payment(
    _storage_dir: &Path,
    wallet_config: &Option<WalletConfig>,
    payee_uri: &str,
    amount: Option<&str>,
    dry_run: bool,
    _verbose: bool,
) -> Result<()> {
    match wallet_config {
        Some(config) if config.has_lightning() => {
            let lnd_config = config.lnd.as_ref().unwrap();

            if dry_run {
                ui::info("DRY RUN - Would execute Lightning payment:");
                ui::info(&format!("  LND URL: {}", lnd_config.url));
                ui::info(&format!("  Recipient: {}", payee_uri));
                if let Some(amt) = amount {
                    ui::info(&format!("  Amount: {} sats", amt));
                }
                ui::info("  No actual payment will be made");
                return Ok(());
            }

            ui::info("Executing Lightning payment...");

            // Create LND executor
            #[cfg(feature = "http-executor")]
            {
                use paykit_lib::executors::{LndConfig as LibLndConfig, LndExecutor};
                use paykit_lib::methods::LightningExecutor;

                let lib_config = LibLndConfig::new(&lnd_config.url, &lnd_config.macaroon);
                let executor =
                    LndExecutor::new(lib_config).context("Failed to create LND executor")?;

                // For now, we expect the payee_uri to contain an invoice
                // In a full implementation, we'd use Noise to negotiate
                let invoice = if payee_uri.starts_with("ln") {
                    payee_uri.to_string()
                } else {
                    // In a real implementation, we would:
                    // 1. Connect to recipient via Noise
                    // 2. Request a payment invoice
                    // 3. Execute the payment
                    ui::warning("Full Noise negotiation not yet implemented");
                    ui::info("To pay directly, provide a BOLT11 invoice as the recipient");
                    return Ok(());
                };

                // Decode invoice first
                let decoded = executor
                    .decode_invoice(&invoice)
                    .await
                    .context("Failed to decode invoice")?;

                ui::info(&format!("Payment hash: {}", decoded.payment_hash));
                if let Some(amt) = decoded.amount_msat {
                    ui::info(&format!(
                        "Invoice amount: {} msat ({} sats)",
                        amt,
                        amt / 1000
                    ));
                }
                if let Some(desc) = &decoded.description {
                    ui::info(&format!("Description: {}", desc));
                }
                if decoded.expired {
                    ui::error("Invoice has expired!");
                    return Err(anyhow::anyhow!("Invoice expired"));
                }

                // Execute payment
                let amount_msat = amount.and_then(|s| s.parse::<u64>().ok().map(|a| a * 1000));
                let result = executor
                    .pay_invoice(&invoice, amount_msat, None)
                    .await
                    .context("Payment failed")?;

                ui::separator();
                match result.status {
                    paykit_lib::methods::LightningPaymentStatus::Succeeded => {
                        ui::success("Payment succeeded!");
                        ui::info(&format!("Preimage: {}", result.preimage));
                        ui::info(&format!("Fee: {} msat", result.fee_msat));
                        ui::info(&format!("Hops: {}", result.hops));
                    }
                    paykit_lib::methods::LightningPaymentStatus::Pending => {
                        ui::warning("Payment pending...");
                        ui::info(&format!("Payment hash: {}", result.payment_hash));
                    }
                    paykit_lib::methods::LightningPaymentStatus::Failed => {
                        ui::error("Payment failed");
                    }
                }
            }

            #[cfg(not(feature = "http-executor"))]
            {
                ui::warning("http-executor feature not enabled");
                ui::info("Rebuild with: cargo build --features http-executor");
            }
        }
        Some(_) => {
            ui::warning("LND not configured for Lightning payments");
            ui::info(
                "Configure with: paykit-demo wallet configure-lnd --url <url> --macaroon <hex>",
            );
            show_simulation_message();
        }
        None => {
            ui::warning("No wallet configured");
            ui::info("Configure with: paykit-demo wallet status");
            show_simulation_message();
        }
    }

    Ok(())
}

async fn execute_onchain_payment(
    _storage_dir: &Path,
    wallet_config: &Option<WalletConfig>,
    payee_uri: &str,
    amount: Option<&str>,
    dry_run: bool,
    _verbose: bool,
) -> Result<()> {
    match wallet_config {
        Some(config) if config.has_onchain() => {
            let esplora_config = config.esplora.as_ref().unwrap();

            if dry_run {
                ui::info("DRY RUN - Would prepare on-chain payment:");
                ui::info(&format!("  Esplora URL: {}", esplora_config.url));
                ui::info(&format!("  Recipient: {}", payee_uri));
                if let Some(amt) = amount {
                    ui::info(&format!("  Amount: {} sats", amt));
                }
                ui::info("  No actual transaction will be created");
                return Ok(());
            }

            ui::info("Preparing on-chain payment...");

            #[cfg(feature = "http-executor")]
            {
                use paykit_lib::executors::{EsploraConfig as LibEsploraConfig, EsploraExecutor};

                let lib_config = LibEsploraConfig::new(&esplora_config.url);
                let executor = EsploraExecutor::new(lib_config)
                    .context("Failed to create Esplora executor")?;

                // Get fee estimates
                let fees = executor
                    .get_fee_estimates()
                    .await
                    .context("Failed to get fee estimates")?;

                ui::info("Current fee estimates:");
                ui::info(&format!(
                    "  Next block: {:.1} sat/vB",
                    fees.get_rate_for_blocks(1)
                ));
                ui::info(&format!(
                    "  ~1 hour: {:.1} sat/vB",
                    fees.get_rate_for_blocks(6)
                ));
                ui::info(&format!(
                    "  ~1 day: {:.1} sat/vB",
                    fees.get_rate_for_blocks(144)
                ));

                ui::separator();
                ui::warning("On-chain payment requires a wallet for signing");
                ui::info("Esplora can verify transactions but cannot create them.");
                ui::info("Use a wallet like Sparrow or Electrum to create the transaction.");
                if let Some(amt) = amount {
                    ui::info(&format!("Send {} sats to: {}", amt, payee_uri));
                } else {
                    ui::info(&format!("Send to: {}", payee_uri));
                }
            }

            #[cfg(not(feature = "http-executor"))]
            {
                ui::warning("http-executor feature not enabled");
                ui::info("Rebuild with: cargo build --features http-executor");
            }
        }
        Some(_) => {
            ui::warning("Esplora not configured for on-chain payments");
            ui::info("Configure with: paykit-demo wallet configure-esplora --url <url>");
            show_simulation_message();
        }
        None => {
            ui::warning("No wallet configured");
            ui::info("Configure with: paykit-demo wallet status");
            show_simulation_message();
        }
    }

    Ok(())
}

fn show_simulation_message() {
    ui::separator();
    ui::info("SIMULATION MODE");
    ui::info("In production, this would:");
    ui::info("  1. Connect to recipient via Noise protocol");
    ui::info("  2. Exchange payment method information");
    ui::info("  3. Coordinate payment execution");
    ui::info("  4. Generate and exchange receipts");
    ui::separator();
}

fn resolve_recipient(storage_dir: &Path, recipient: &str) -> Result<String> {
    // If it looks like a URI, return as-is
    if recipient.starts_with("pubky://")
        || recipient.starts_with("ln")
        || recipient.starts_with("bc1")
        || recipient.starts_with("tb1")
        || recipient.len() > 40
    {
        return Ok(recipient.to_string());
    }

    // Otherwise, try to look up as contact name
    let storage = DemoStorage::new(storage_dir.join("data"));
    let contacts = storage.list_contacts()?;

    for contact in contacts {
        if contact.name == recipient {
            return Ok(contact.pubky_uri());
        }
    }

    // If not found, assume it's a public key
    Ok(format!("pubky://{}", recipient))
}

fn log_payment_attempt(
    storage_dir: &Path,
    recipient: &str,
    amount: &str,
    method: &str,
) -> Result<()> {
    let log_dir = storage_dir.join("logs");
    std::fs::create_dir_all(&log_dir)?;

    let log_file = log_dir.join("payments.log");
    let timestamp = chrono::Utc::now().to_rfc3339();
    let entry = format!(
        "{} | method={} | recipient={} | amount={}\n",
        timestamp, method, recipient, amount
    );

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?
        .write_all(entry.as_bytes())?;

    Ok(())
}

use std::io::Write;
