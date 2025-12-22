//! Activity command - unified timeline of all payment activity
//!
//! Provides parity with mobile demo's ActivityListView functionality.
//! Combines receipts, subscriptions, and other events into a single timeline.

use anyhow::Result;
use chrono::{DateTime, Utc};
use colored::Colorize;
use paykit_demo_core::DemoStorage;
use std::path::Path;

use crate::ui;

/// Activity type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityType {
    Payment,
    Subscription,
    Request,
    AutoPay,
}

impl std::fmt::Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Payment => write!(f, "payment"),
            Self::Subscription => write!(f, "subscription"),
            Self::Request => write!(f, "request"),
            Self::AutoPay => write!(f, "autopay"),
        }
    }
}

impl std::str::FromStr for ActivityType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "payment" | "payments" | "p" => Ok(Self::Payment),
            "subscription" | "subscriptions" | "sub" | "s" => Ok(Self::Subscription),
            "request" | "requests" | "req" | "r" => Ok(Self::Request),
            "autopay" | "auto" | "a" => Ok(Self::AutoPay),
            _ => Err(anyhow::anyhow!(
                "Invalid activity type. Use: payment, subscription, request, or autopay"
            )),
        }
    }
}

/// Direction filter
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Direction {
    All,
    Sent,
    Received,
}

impl std::str::FromStr for Direction {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "all" | "a" => Ok(Self::All),
            "sent" | "out" | "s" => Ok(Self::Sent),
            "received" | "in" | "recv" | "r" => Ok(Self::Received),
            _ => Err(anyhow::anyhow!(
                "Invalid direction. Use: all, sent, or received"
            )),
        }
    }
}

/// Unified activity item
#[derive(Debug, Clone)]
struct ActivityItem {
    id: String,
    activity_type: ActivityType,
    timestamp: i64,
    amount: Option<String>,
    currency: Option<String>,
    counterparty: String,
    direction: String,
    status: String,
    method: Option<String>,
}

impl ActivityItem {
    fn timestamp_display(&self) -> String {
        DateTime::from_timestamp(self.timestamp, 0)
            .map(|dt: DateTime<Utc>| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    fn amount_display(&self) -> String {
        match (&self.amount, &self.currency) {
            (Some(amt), Some(curr)) => format!("{} {}", amt, curr),
            (Some(amt), None) => amt.clone(),
            _ => "N/A".to_string(),
        }
    }

    fn type_icon(&self) -> &str {
        match self.activity_type {
            ActivityType::Payment => "💸",
            ActivityType::Subscription => "🔄",
            ActivityType::Request => "📨",
            ActivityType::AutoPay => "⚡",
        }
    }

    fn direction_icon(&self) -> String {
        match self.direction.as_str() {
            "sent" | "outgoing" => "↑".red().to_string(),
            "received" | "incoming" => "↓".green().to_string(),
            _ => "·".to_string(),
        }
    }
}

/// Run activity command - show unified timeline
#[tracing::instrument(skip(storage_dir))]
pub async fn run(
    storage_dir: &Path,
    filter_type: Option<ActivityType>,
    direction: Direction,
    limit: usize,
    verbose: bool,
) -> Result<()> {
    ui::header("Activity Timeline");

    let storage = DemoStorage::new(storage_dir.join("data"));
    let mut activities: Vec<ActivityItem> = Vec::new();

    // Collect receipts
    if filter_type.is_none() || filter_type == Some(ActivityType::Payment) {
        let receipts = storage.list_receipts()?;
        for receipt in receipts {
            let is_sent = receipt.payer.to_string().contains("self")
                || receipt.metadata.get("direction").and_then(|v| v.as_str()) == Some("sent");

            let direction_str = if is_sent { "sent" } else { "received" };

            activities.push(ActivityItem {
                id: receipt.id.clone(),
                activity_type: ActivityType::Payment,
                timestamp: receipt.timestamp,
                amount: receipt.amount.clone(),
                currency: receipt.currency.clone(),
                counterparty: if is_sent {
                    abbreviate_key(&receipt.payee.to_string())
                } else {
                    abbreviate_key(&receipt.payer.to_string())
                },
                direction: direction_str.to_string(),
                status: if receipt.proof_verified {
                    "verified"
                } else {
                    "pending"
                }
                .to_string(),
                method: Some(receipt.method.clone()),
            });
        }
    }

    // Collect subscription events
    if filter_type.is_none() || filter_type == Some(ActivityType::Subscription) {
        let subscriptions_path = storage_dir.join("data").join("subscriptions.json");
        if subscriptions_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&subscriptions_path) {
                if let Ok(subs) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                    for sub in subs {
                        if let (Some(id), Some(created_at)) =
                            (sub.get("id").and_then(|v| v.as_str()), sub.get("created_at"))
                        {
                            let timestamp = created_at.as_i64().unwrap_or(0);
                            let peer = sub
                                .get("peer")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");

                            activities.push(ActivityItem {
                                id: id.to_string(),
                                activity_type: ActivityType::Subscription,
                                timestamp,
                                amount: sub.get("amount").and_then(|v| v.as_str()).map(String::from),
                                currency: sub
                                    .get("currency")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                counterparty: abbreviate_key(peer),
                                direction: "outgoing".to_string(),
                                status: sub
                                    .get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                                method: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Collect payment requests
    if filter_type.is_none() || filter_type == Some(ActivityType::Request) {
        let requests_path = storage_dir.join("data").join("payment_requests.json");
        if requests_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&requests_path) {
                if let Ok(requests) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                    for req in requests {
                        if let (Some(id), Some(timestamp)) =
                            (req.get("id").and_then(|v| v.as_str()), req.get("timestamp"))
                        {
                            let ts = timestamp.as_i64().unwrap_or(0);

                            activities.push(ActivityItem {
                                id: id.to_string(),
                                activity_type: ActivityType::Request,
                                timestamp: ts,
                                amount: req.get("amount").and_then(|v| v.as_str()).map(String::from),
                                currency: req
                                    .get("currency")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                counterparty: req
                                    .get("from")
                                    .and_then(|v| v.as_str())
                                    .map(|s| abbreviate_key(s))
                                    .unwrap_or_else(|| "unknown".to_string()),
                                direction: "incoming".to_string(),
                                status: req
                                    .get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("pending")
                                    .to_string(),
                                method: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Collect auto-pay events
    if filter_type.is_none() || filter_type == Some(ActivityType::AutoPay) {
        let autopay_path = storage_dir.join("data").join("autopay_history.json");
        if autopay_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&autopay_path) {
                if let Ok(events) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                    for event in events {
                        if let (Some(id), Some(timestamp)) =
                            (event.get("id").and_then(|v| v.as_str()), event.get("timestamp"))
                        {
                            let ts = timestamp.as_i64().unwrap_or(0);

                            activities.push(ActivityItem {
                                id: id.to_string(),
                                activity_type: ActivityType::AutoPay,
                                timestamp: ts,
                                amount: event.get("amount").and_then(|v| v.as_str()).map(String::from),
                                currency: event
                                    .get("currency")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                counterparty: event
                                    .get("peer")
                                    .and_then(|v| v.as_str())
                                    .map(|s| abbreviate_key(s))
                                    .unwrap_or_else(|| "unknown".to_string()),
                                direction: "sent".to_string(),
                                status: event
                                    .get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("completed")
                                    .to_string(),
                                method: event
                                    .get("method")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                            });
                        }
                    }
                }
            }
        }
    }

    // Filter by direction
    let activities: Vec<ActivityItem> = match direction {
        Direction::All => activities,
        Direction::Sent => activities
            .into_iter()
            .filter(|a| a.direction == "sent" || a.direction == "outgoing")
            .collect(),
        Direction::Received => activities
            .into_iter()
            .filter(|a| a.direction == "received" || a.direction == "incoming")
            .collect(),
    };

    // Sort by timestamp (most recent first)
    let mut activities = activities;
    activities.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Apply limit
    let activities: Vec<_> = activities.into_iter().take(limit).collect();

    if activities.is_empty() {
        ui::info("No activity found");

        if filter_type.is_some() {
            ui::info("Try removing the --type filter to see all activity");
        }

        return Ok(());
    }

    ui::info(&format!(
        "Showing {} most recent activit{}",
        activities.len(),
        if activities.len() == 1 { "y" } else { "ies" }
    ));
    ui::separator();

    for activity in &activities {
        // Header line with icon, type, direction, and timestamp
        let header = format!(
            "{} {} {} {} {}",
            activity.type_icon(),
            activity.activity_type.to_string().bold(),
            activity.direction_icon(),
            activity.counterparty.dimmed(),
            activity.timestamp_display().dimmed()
        );
        println!("{}", header);

        // Details line
        println!(
            "   {} | Status: {} | ID: {}",
            activity.amount_display(),
            format_status(&activity.status),
            &activity.id[..activity.id.len().min(12)]
        );

        if verbose {
            if let Some(method) = &activity.method {
                ui::key_value("   Method", method);
            }
            ui::key_value("   Full ID", &activity.id);
        }

        println!();
    }

    // Summary
    ui::separator();
    let payment_count = activities
        .iter()
        .filter(|a| a.activity_type == ActivityType::Payment)
        .count();
    let subscription_count = activities
        .iter()
        .filter(|a| a.activity_type == ActivityType::Subscription)
        .count();
    let request_count = activities
        .iter()
        .filter(|a| a.activity_type == ActivityType::Request)
        .count();
    let autopay_count = activities
        .iter()
        .filter(|a| a.activity_type == ActivityType::AutoPay)
        .count();

    let mut summary = Vec::new();
    if payment_count > 0 {
        summary.push(format!("{} payments", payment_count));
    }
    if subscription_count > 0 {
        summary.push(format!("{} subscriptions", subscription_count));
    }
    if request_count > 0 {
        summary.push(format!("{} requests", request_count));
    }
    if autopay_count > 0 {
        summary.push(format!("{} auto-pays", autopay_count));
    }

    if !summary.is_empty() {
        ui::info(&format!("Summary: {}", summary.join(", ")));
    }

    Ok(())
}

fn abbreviate_key(key: &str) -> String {
    if key.len() > 16 {
        format!("{}...{}", &key[..8], &key[key.len() - 4..])
    } else {
        key.to_string()
    }
}

fn format_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "verified" | "completed" | "accepted" | "active" => status.green().to_string(),
        "pending" | "awaiting" => status.yellow().to_string(),
        "failed" | "declined" | "expired" => status.red().to_string(),
        _ => status.to_string(),
    }
}

