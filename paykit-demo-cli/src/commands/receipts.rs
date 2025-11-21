//! Receipts command - show payment receipts

use anyhow::Result;
use colored::Colorize;
use paykit_demo_core::DemoStorage;
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
