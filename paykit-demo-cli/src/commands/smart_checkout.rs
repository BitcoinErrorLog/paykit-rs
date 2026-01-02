//! Smart Checkout command - discover methods and select the best one
//!
//! Provides parity with mobile demo's SmartCheckoutView functionality.

use anyhow::{Context, Result};
use paykit_demo_core::DirectoryClient;
use std::path::Path;

use crate::ui;

/// Selection strategy for choosing payment method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Balance cost, speed, and privacy
    Balanced,
    /// Prefer lowest fees
    LowestFee,
    /// Prefer fastest confirmation
    Fastest,
    /// Prefer maximum privacy
    MostPrivate,
}

impl std::str::FromStr for Strategy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "balanced" | "b" => Ok(Self::Balanced),
            "lowest-fee" | "fee" | "cheap" | "l" => Ok(Self::LowestFee),
            "fastest" | "fast" | "speed" | "f" => Ok(Self::Fastest),
            "private" | "privacy" | "most-private" | "p" => Ok(Self::MostPrivate),
            _ => Err(anyhow::anyhow!(
                "Invalid strategy. Use: balanced, lowest-fee, fastest, or private"
            )),
        }
    }
}

/// Discovered method with scoring
#[derive(Debug, Clone)]
struct ScoredMethod {
    method_id: String,
    endpoint: String,
    // Scores from 0.0 to 1.0
    cost_score: f64,
    speed_score: f64,
    privacy_score: f64,
    is_healthy: bool,
}

impl ScoredMethod {
    fn balanced_score(&self) -> f64 {
        (self.cost_score + self.speed_score + self.privacy_score) / 3.0
    }

    fn score_for_strategy(&self, strategy: Strategy) -> f64 {
        match strategy {
            Strategy::Balanced => self.balanced_score(),
            Strategy::LowestFee => self.cost_score,
            Strategy::Fastest => self.speed_score,
            Strategy::MostPrivate => self.privacy_score,
        }
    }
}

/// Run smart checkout - discover and select best payment method
#[tracing::instrument(skip(_storage_dir))]
pub async fn run(
    _storage_dir: &Path,
    recipient: &str,
    amount: Option<u64>,
    strategy: Strategy,
    homeserver: &str,
    execute: bool,
    verbose: bool,
) -> Result<()> {
    ui::header("Smart Checkout");

    tracing::debug!("Parsing recipient: {}", recipient);
    let public_key = parse_pubky_uri(recipient)?;

    ui::key_value(
        "Recipient",
        &format!(
            "{}...{}",
            &recipient[..12.min(recipient.len())],
            &recipient[recipient.len().saturating_sub(8)..]
        ),
    );
    if let Some(amt) = amount {
        ui::key_value("Amount", &format!("{} sats", amt));
    }
    ui::key_value("Strategy", &format!("{:?}", strategy));
    ui::separator();

    let client = DirectoryClient::new(homeserver);

    // Discover methods
    let spinner = ui::spinner("Discovering payment methods...");

    let methods = match client.query_methods(&public_key).await {
        Ok(m) => {
            spinner.finish_and_clear();
            m
        }
        Err(e) => {
            spinner.finish_and_clear();
            ui::error(&format!("Failed to discover methods: {}", e));
            return Err(e).context("Method discovery failed");
        }
    };

    if methods.is_empty() {
        ui::error("No payment methods found for this recipient");
        ui::info("The recipient may not have published any payment methods");
        return Ok(());
    }

    ui::success(&format!("Found {} payment method(s)", methods.len()));

    // Score methods
    let spinner = ui::spinner("Evaluating methods...");
    let scored_methods: Vec<ScoredMethod> = methods
        .iter()
        .map(|m| score_method(&m.method_id, &m.endpoint))
        .collect();
    spinner.finish_and_clear();

    // Filter healthy methods
    let healthy_methods: Vec<&ScoredMethod> =
        scored_methods.iter().filter(|m| m.is_healthy).collect();

    if healthy_methods.is_empty() {
        ui::warning("No healthy payment methods available");
        ui::info("All discovered methods failed health checks");

        if verbose {
            ui::separator();
            ui::info("Unhealthy methods:");
            for m in &scored_methods {
                ui::key_value(&format!("  {}", m.method_id), "(unhealthy)");
            }
        }
        return Ok(());
    }

    // Sort by strategy score
    let mut sorted: Vec<&ScoredMethod> = healthy_methods;
    sorted.sort_by(|a, b| {
        b.score_for_strategy(strategy)
            .partial_cmp(&a.score_for_strategy(strategy))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Display ranked methods
    ui::separator();
    ui::info("Ranked methods:");
    for (i, method) in sorted.iter().enumerate() {
        let rank = if i == 0 { "★ " } else { "  " };
        let score = method.score_for_strategy(strategy);
        ui::info(&format!(
            "{}{}: {} (score: {:.2})",
            rank, method.method_id, method.endpoint, score
        ));

        if verbose {
            ui::info(&format!(
                "     cost: {:.2}, speed: {:.2}, privacy: {:.2}",
                method.cost_score, method.speed_score, method.privacy_score
            ));
        }
    }

    // Recommend best method
    if let Some(best) = sorted.first() {
        ui::separator();
        ui::success(&format!(
            "Recommended: {} via {}",
            best.method_id, best.endpoint
        ));

        if execute {
            ui::separator();
            ui::info("Executing payment...");

            // In a real implementation, this would call the pay command
            // For now, show what would happen
            if let Some(amt) = amount {
                ui::info(&format!(
                    "Would pay {} sats to {} using {}",
                    amt, best.endpoint, best.method_id
                ));
            } else {
                ui::warning("No amount specified. Use --amount to execute payment.");
            }
        } else {
            ui::info("Use --execute to proceed with payment");
        }
    }

    Ok(())
}

/// Score a payment method based on its characteristics
fn score_method(method_id: &str, endpoint: &str) -> ScoredMethod {
    // Determine scores based on method type
    let (cost_score, speed_score, privacy_score) = match method_id.to_lowercase().as_str() {
        "lightning" => {
            // Lightning: fast, low fees, moderate privacy
            (0.9, 1.0, 0.7)
        }
        "onchain" | "bitcoin" => {
            // On-chain: slower, variable fees, lower privacy
            (0.6, 0.3, 0.5)
        }
        "noise" => {
            // Noise: fast, no fees, high privacy
            (1.0, 0.9, 1.0)
        }
        _ => {
            // Unknown: moderate scores
            (0.5, 0.5, 0.5)
        }
    };

    // Check if endpoint looks valid (basic health check)
    let is_healthy = !endpoint.is_empty() && is_endpoint_valid(method_id, endpoint);

    ScoredMethod {
        method_id: method_id.to_string(),
        endpoint: endpoint.to_string(),
        cost_score,
        speed_score,
        privacy_score,
        is_healthy,
    }
}

/// Basic endpoint validation
fn is_endpoint_valid(method_id: &str, endpoint: &str) -> bool {
    match method_id.to_lowercase().as_str() {
        "lightning" => {
            // Lightning invoice should start with ln
            endpoint.starts_with("ln") || endpoint.starts_with("LN")
        }
        "onchain" | "bitcoin" => {
            // Bitcoin address validation (basic)
            endpoint.starts_with("bc1") 
                || endpoint.starts_with("1") 
                || endpoint.starts_with("3")
                || endpoint.starts_with("tb1") // testnet
                || endpoint.starts_with("m") // testnet
                || endpoint.starts_with("n") // testnet
                || endpoint.starts_with("2") // testnet
        }
        "noise" => {
            // Noise endpoint should be host:port or pubkey
            endpoint.contains(':') || endpoint.len() >= 32
        }
        _ => true, // Accept unknown methods
    }
}

fn parse_pubky_uri(uri: &str) -> Result<pubky::PublicKey> {
    let key_str = uri.strip_prefix("pubky://").unwrap_or(uri);
    let key_str = key_str.split('/').next().unwrap_or(key_str);
    key_str.parse().context("Invalid Pubky URI format")
}
