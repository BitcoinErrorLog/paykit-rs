//! Receipts command - show payment receipts

use anyhow::{Context, Result};
use colored::Colorize;
use paykit_demo_core::DemoStorage;
use paykit_interactive::proof::{PaymentProof, ProofType, ProofVerifier};
use paykit_interactive::proof::verifiers::{RealBitcoinProofVerifier, RealLightningProofVerifier};
#[cfg(feature = "http-executor")]
use paykit_lib::executors::{EsploraConfig, EsploraExecutor};
use std::path::Path;

use crate::ui;

pub async fn run(storage_dir: &Path, verbose: bool) -> Result<()> {
    ui::header("Payment Receipts");

    let storage = DemoStorage::new(storage_dir.join("data"));
    let receipts = storage.list_receipts()?;

    if receipts.is_empty() {
        ui::info("No receipts found");
        ui::info("Receipts will appear here after completing payments");
        return Ok(());
    }

    for receipt in receipts {
        println!("\n{}", format!("Receipt: {}", receipt.id).bold());
        ui::key_value("  Method", &receipt.method);

        if let Some(amount) = &receipt.amount {
            if let Some(currency) = &receipt.currency {
                ui::key_value("  Amount", &format!("{} {}", amount, currency));
            } else {
                ui::key_value("  Amount", amount);
            }
        }

        ui::key_value(
            "  Timestamp",
            &chrono::DateTime::from_timestamp(receipt.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
        );

        // Show proof status
        if let Some(proof) = &receipt.proof {
            let status_icon = if receipt.proof_verified {
                "✓".green()
            } else {
                "⚠".yellow()
            };
            println!("  Proof: {} {}", status_icon, if receipt.proof_verified { "Verified" } else { "Unverified" });
            
            if verbose {
                println!("  Proof details:");
                ui::json(proof);
                if let Some(verified_at) = receipt.proof_verified_at {
                    ui::key_value(
                        "  Verified at",
                        &chrono::DateTime::from_timestamp(verified_at, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| "Unknown".to_string()),
                    );
                }
            }
        } else {
            println!("  Proof: {}", "✗".red().to_string() + " None");
        }

        if verbose {
            ui::key_value("  Payer", &receipt.payer.to_string());
            ui::key_value("  Payee", &receipt.payee.to_string());

            if !receipt.metadata.is_null() {
                println!("  Metadata:");
                ui::json(&receipt.metadata);
            }
        }
    }

    Ok(())
}

pub async fn show(storage_dir: &Path, receipt_id: &str, verbose: bool) -> Result<()> {
    ui::header(&format!("Receipt: {}", receipt_id));

    let storage = DemoStorage::new(storage_dir.join("data"));
    let receipt = storage
        .get_receipt(receipt_id)
        .context("Failed to load receipt")?
        .ok_or_else(|| anyhow::anyhow!("Receipt not found: {}", receipt_id))?;

    ui::key_value("ID", &receipt.id);
    ui::key_value("Method", &receipt.method);
    
    if let Some(amount) = &receipt.amount {
        if let Some(currency) = &receipt.currency {
            ui::key_value("Amount", &format!("{} {}", amount, currency));
        } else {
            ui::key_value("Amount", amount);
        }
    }

    ui::key_value(
        "Timestamp",
        &chrono::DateTime::from_timestamp(receipt.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
    );

    ui::key_value("Payer", &receipt.payer.to_string());
    ui::key_value("Payee", &receipt.payee.to_string());

    // Show proof details
    ui::separator();
    if let Some(proof_json) = &receipt.proof {
        println!("{}", "Proof:".bold());
        ui::json(proof_json);
        
        let status_icon = if receipt.proof_verified {
            "✓".green()
        } else {
            "⚠".yellow()
        };
        println!("Status: {} {}", status_icon, if receipt.proof_verified { "Verified" } else { "Unverified" });
        
        if let Some(verified_at) = receipt.proof_verified_at {
            ui::key_value(
                "Verified at",
                &chrono::DateTime::from_timestamp(verified_at, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string()),
            );
        }
    } else {
        println!("{}", "Proof: ".red().to_string() + "None");
    }

    if verbose && !receipt.metadata.is_null() {
        ui::separator();
        println!("{}", "Metadata:".bold());
        ui::json(&receipt.metadata);
    }

    Ok(())
}

pub async fn verify_proof(storage_dir: &Path, receipt_id: &str, verbose: bool) -> Result<()> {
    ui::header(&format!("Verify Proof: {}", receipt_id));

    let storage = DemoStorage::new(storage_dir.join("data"));
    let mut receipt = storage
        .get_receipt(receipt_id)
        .context("Failed to load receipt")?
        .ok_or_else(|| anyhow::anyhow!("Receipt not found: {}", receipt_id))?;

    let proof_json = receipt.proof.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Receipt has no proof to verify"))?;

    // Parse proof from JSON
    let proof: PaymentProof = serde_json::from_value(proof_json.clone())
        .context("Failed to parse proof JSON")?;

    ui::info("Verifying proof...");
    let spinner = ui::spinner("Verifying");

    let verification_result = match proof.proof_type {
        ProofType::BitcoinTxid { .. } => {
            #[cfg(feature = "http-executor")]
            {
                // Check wallet config for Esplora URL
                let wallet_config = super::wallet::WalletConfig::load(storage_dir)?;
                let esplora_config = if let Some(config) = wallet_config {
                    if let Some(esplora) = config.esplora {
                        EsploraConfig::new(&esplora.url)
                    } else {
                        EsploraConfig::blockstream_mainnet()
                    }
                } else {
                    EsploraConfig::blockstream_mainnet()
                };

                let verifier = RealBitcoinProofVerifier::with_config(esplora_config)
                    .with_min_confirmations(1);
                verifier.verify(&proof).await
            }
            #[cfg(not(feature = "http-executor"))]
            {
                ui::warning("http-executor feature not enabled");
                ui::info("Rebuild with: cargo build --features http-executor");
                return Ok(());
            }
        }
        ProofType::LightningPreimage { .. } => {
            let verifier = RealLightningProofVerifier::new();
            verifier.verify(&proof).await
        }
        ProofType::Custom { .. } => {
            ui::warning("Custom proof type - verification not implemented");
            return Ok(());
        }
    };

    spinner.finish_and_clear();

    if verification_result.valid {
        ui::success("Proof verification succeeded!");
        
        // Update receipt
        receipt.proof_verified = true;
        receipt.proof_verified_at = Some(chrono::Utc::now().timestamp());
        storage.save_receipt(receipt)?;
        
        if let Some(details) = verification_result.details {
            if verbose {
                println!("\nVerification details:");
                ui::json(&details);
            }
        }
    } else {
        ui::error("Proof verification failed!");
        for error in &verification_result.errors {
            ui::error(&format!("  - {}", error));
        }
    }

    Ok(())
}
