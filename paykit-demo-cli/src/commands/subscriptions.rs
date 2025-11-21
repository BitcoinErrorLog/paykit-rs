//! Subscription and payment request commands

use anyhow::{anyhow, Result};
use paykit_lib::{MethodId, PublicKey};
use paykit_subscriptions::{
    request::{PaymentRequest, RequestStatus},
    signing,
    storage::{Direction, FileSubscriptionStorage, RequestFilter, SubscriptionStorage},
    subscription::{PaymentFrequency, Subscription, SubscriptionTerms},
    Amount,
};
use std::{path::Path, str::FromStr};

use crate::ui;

/// Create subscription storage
fn create_subscription_storage(storage_dir: &Path) -> Result<FileSubscriptionStorage> {
    let subs_storage_dir = storage_dir.join("subscriptions");
    FileSubscriptionStorage::new(subs_storage_dir)
        .map_err(|e| anyhow!("Failed to create storage: {}", e))
}

/// Send a payment request to a peer
#[tracing::instrument(skip(storage_dir))]
pub async fn send_request(
    storage_dir: &Path,
    recipient: &str,
    amount: &str,
    currency: &str,
    description: Option<String>,
    expires_in: Option<u64>,
) -> Result<()> {
    let identity = super::load_current_identity(storage_dir)?;

    ui::header("Send Payment Request");

    tracing::debug!("Resolving recipient: {}", recipient);
    // Resolve recipient
    let recipient_pk = resolve_recipient(storage_dir, recipient)?;

    ui::key_value("From", &identity.public_key().to_z32());
    ui::key_value("To", &recipient_pk.to_string());
    ui::key_value("Amount", &format!("{} {}", amount, currency));
    if let Some(desc) = &description {
        ui::key_value("Description", desc);
    }

    let local_pk = PublicKey::from_str(&identity.public_key().to_z32())?;

    // Parse amount as satoshis
    let amount_sats: i64 = amount
        .parse()
        .map_err(|_| anyhow!("Invalid amount: {}", amount))?;

    let mut request = PaymentRequest::new(
        local_pk,
        recipient_pk,
        Amount::from_sats(amount_sats),
        currency.to_string(),
        MethodId(String::from("lightning")),
    );

    if let Some(desc) = description {
        request = request.with_description(desc);
    }

    if let Some(exp) = expires_in {
        let expires_at = chrono::Utc::now().timestamp() + exp as i64;
        request = request.with_expiration(expires_at);
    }

    let storage = create_subscription_storage(storage_dir)?;
    storage.save_request(&request).await?;

    tracing::info!("Request saved: {}", request.request_id);
    ui::success(&format!("Payment request created: {}", request.request_id));
    ui::info("Request saved locally. Connect with recipient to deliver it via Noise channel.");
    ui::info(&format!(
        "Use 'paykit-demo pay {}' to send payment once accepted.",
        recipient
    ));

    Ok(())
}

/// List payment requests
#[tracing::instrument(skip(storage_dir))]
pub async fn list_requests(
    storage_dir: &Path,
    filter_type: &str,
    peer: Option<String>,
) -> Result<()> {
    let storage = create_subscription_storage(storage_dir)?;

    ui::header("Payment Requests");

    let direction = match filter_type {
        "incoming" => Some(Direction::Incoming),
        "outgoing" => Some(Direction::Outgoing),
        _ => None,
    };

    let peer_pk = if let Some(p) = peer {
        Some(resolve_recipient(storage_dir, &p)?)
    } else {
        None
    };

    let filter = RequestFilter {
        peer: peer_pk,
        status: None,
        direction,
    };

    let requests = storage.list_requests(filter).await?;

    if requests.is_empty() {
        ui::info("No payment requests found.");
        return Ok(());
    }

    for req in requests {
        ui::key_value("Request ID", &req.request_id[..8.min(req.request_id.len())]);
        ui::key_value(
            "From",
            &req.from.to_string()[..20.min(req.from.to_string().len())],
        );
        ui::key_value(
            "To",
            &req.to.to_string()[..20.min(req.to.to_string().len())],
        );
        ui::key_value("Amount", &format!("{} {}", req.amount, req.currency));

        let created_dt =
            chrono::DateTime::from_timestamp(req.created_at, 0).unwrap_or_else(chrono::Utc::now);
        ui::key_value(
            "Created",
            &created_dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        if let Some(exp) = req.expires_at {
            let exp_dt = chrono::DateTime::from_timestamp(exp, 0).unwrap_or_else(chrono::Utc::now);
            ui::key_value("Expires", &exp_dt.format("%Y-%m-%d %H:%M:%S").to_string());

            if chrono::Utc::now().timestamp() > exp {
                ui::warning("⚠ EXPIRED");
            }
        }

        ui::separator();
    }

    Ok(())
}

/// Respond to a payment request
pub async fn respond_to_request(
    storage_dir: &Path,
    request_id: &str,
    action: &str,
    reason: Option<String>,
) -> Result<()> {
    let storage = create_subscription_storage(storage_dir)?;

    ui::header("Respond to Payment Request");

    // Load request
    let request = storage
        .get_request(request_id)
        .await?
        .ok_or_else(|| anyhow!("Request {} not found", request_id))?;

    ui::key_value("Request ID", request_id);
    ui::key_value("From", &request.from.to_string());
    ui::key_value(
        "Amount",
        &format!("{} {}", request.amount, request.currency),
    );

    let new_status = match action {
        "accept" => {
            ui::success("Accepting payment request");
            RequestStatus::Accepted
        }
        "decline" => {
            ui::warning("Declining payment request");
            if let Some(r) = reason {
                ui::key_value("Reason", &r);
            }
            RequestStatus::Declined
        }
        _ => {
            return Err(anyhow!(
                "Invalid action: {}. Use 'accept' or 'decline'",
                action
            ))
        }
    };

    storage
        .update_request_status(request_id, new_status)
        .await?;

    ui::success(&format!(
        "Request {} updated to {:?}",
        request_id, new_status
    ));

    if matches!(new_status, RequestStatus::Accepted) {
        ui::info("To complete payment, use:");
        ui::info(&format!(
            "  paykit-demo pay {} --amount {} --currency {}",
            request.from,
            request.amount.as_sats(),
            request.currency
        ));
    }

    Ok(())
}

/// Show request details
pub async fn show_request(storage_dir: &Path, request_id: &str) -> Result<()> {
    let storage = create_subscription_storage(storage_dir)?;

    ui::header("Payment Request Details");

    let request = storage
        .get_request(request_id)
        .await?
        .ok_or_else(|| anyhow!("Request {} not found", request_id))?;

    ui::key_value("Request ID", &request.request_id);
    ui::key_value("From", &request.from.to_string());
    ui::key_value("To", &request.to.to_string());
    ui::key_value(
        "Amount",
        &format!("{} {}", request.amount, request.currency),
    );
    ui::key_value("Method", &request.method.0);

    let created_dt =
        chrono::DateTime::from_timestamp(request.created_at, 0).unwrap_or_else(chrono::Utc::now);
    ui::key_value(
        "Created",
        &created_dt.format("%Y-%m-%d %H:%M:%S").to_string(),
    );

    if let Some(exp) = request.expires_at {
        let exp_dt = chrono::DateTime::from_timestamp(exp, 0).unwrap_or_else(chrono::Utc::now);
        ui::key_value("Expires", &exp_dt.format("%Y-%m-%d %H:%M:%S").to_string());

        if chrono::Utc::now().timestamp() > exp {
            ui::warning("⚠ This request has expired");
        }
    }

    if let Some(desc) = &request.description {
        ui::key_value("Description", desc);
    }

    ui::separator();
    ui::info("Commands:");
    ui::info(&format!(
        "  Accept:  paykit-demo subscriptions respond {} --action accept",
        request_id
    ));
    ui::info(&format!(
        "  Decline: paykit-demo subscriptions respond {} --action decline",
        request_id
    ));

    Ok(())
}

/// Helper: resolve recipient from contact name or Pubky URI
fn resolve_recipient(storage_dir: &Path, recipient: &str) -> Result<PublicKey> {
    // Try as Pubky URI first
    if recipient.starts_with("pubky://") || recipient.starts_with("paykit:") {
        let uri_str = recipient
            .strip_prefix("pubky://")
            .or_else(|| recipient.strip_prefix("paykit:"))
            .unwrap_or(recipient);
        return PublicKey::from_str(uri_str).map_err(|e| anyhow!("Invalid public key: {}", e));
    }

    // Try as direct z32 public key
    if let Ok(pk) = PublicKey::from_str(recipient).map_err(|e| anyhow!("{}", e)) {
        return Ok(pk);
    }

    // Try as contact name
    let contacts_file = storage_dir.join("contacts.json");
    if contacts_file.exists() {
        let data = std::fs::read_to_string(contacts_file)?;
        let contacts: serde_json::Value = serde_json::from_str(&data)?;
        if let Some(contact) = contacts.get(recipient) {
            if let Some(uri) = contact.get("uri").and_then(|v| v.as_str()) {
                let uri_str = uri
                    .strip_prefix("pubky://")
                    .or_else(|| uri.strip_prefix("paykit:"))
                    .unwrap_or(uri);
                return PublicKey::from_str(uri_str)
                    .map_err(|e| anyhow!("Invalid public key: {}", e));
            }
        }
    }

    Err(anyhow!("Could not resolve recipient: {}", recipient))
}

// ============================================================
// Phase 2: Subscription Agreements
// ============================================================

/// Propose a subscription agreement to a peer
#[tracing::instrument(skip(storage_dir))]
pub async fn propose_subscription(
    storage_dir: &Path,
    recipient: &str,
    amount: &str,
    currency: &str,
    frequency: &str,
    description: &str,
) -> Result<()> {
    let identity = super::load_current_identity(storage_dir)?;

    ui::header("Propose Subscription");

    // Resolve recipient
    let recipient_pk = resolve_recipient(storage_dir, recipient)?;

    let subscriber_pk = PublicKey::from_str(&identity.public_key().to_z32())?;

    // Parse frequency
    let payment_frequency = parse_frequency(frequency)?;

    // Parse amount as satoshis
    let amount_sats: i64 = amount
        .parse()
        .map_err(|_| anyhow!("Invalid amount: {}", amount))?;

    // Create subscription terms
    let terms = SubscriptionTerms::new(
        Amount::from_sats(amount_sats),
        currency.to_string(),
        payment_frequency,
        MethodId("lightning".to_string()),
        description.to_string(),
    );

    // Create subscription
    let subscription = Subscription::new(subscriber_pk.clone(), recipient_pk.clone(), terms);

    // Validate
    subscription.validate()?;

    // Sign the proposal (Ed25519 only)
    let nonce = rand::random::<[u8; 32]>();
    let _signature = signing::sign_subscription_ed25519(
        &subscription,
        &identity.keypair,
        &nonce,
        60 * 60 * 24 * 7, // 7 days validity
    )?;

    // Save locally
    let storage = create_subscription_storage(storage_dir)?;
    storage.save_subscription(&subscription).await?;

    tracing::info!(
        "Subscription proposal saved: {}",
        subscription.subscription_id
    );
    // Display details
    ui::key_value("Subscription ID", &subscription.subscription_id);
    ui::key_value("Subscriber (You)", &subscriber_pk.to_z32());
    ui::key_value("Provider", &recipient_pk.to_z32());
    ui::key_value("Amount", &format!("{} {}", amount, currency));
    ui::key_value("Frequency", &subscription.terms.frequency.to_string());
    ui::key_value("Description", description);

    ui::success("Subscription proposal created and signed");
    ui::info("To send to recipient, establish a Noise connection and transmit the proposal.");
    ui::info(&format!(
        "Subscription ID: {}",
        subscription.subscription_id
    ));

    Ok(())
}

/// Accept a subscription proposal
#[tracing::instrument(skip(storage_dir))]
pub async fn accept_subscription(storage_dir: &Path, subscription_id: &str) -> Result<()> {
    let identity = super::load_current_identity(storage_dir)?;
    let storage = create_subscription_storage(storage_dir)?;

    ui::header("Accept Subscription");

    // Load subscription
    let subscription = storage
        .get_subscription(subscription_id)
        .await?
        .ok_or_else(|| anyhow!("Subscription {} not found", subscription_id))?;

    // Display details
    ui::key_value("Subscription ID", subscription_id);
    ui::key_value("Subscriber", &subscription.subscriber.to_z32());
    ui::key_value("Provider", &subscription.provider.to_z32());
    ui::key_value(
        "Amount",
        &format!(
            "{} {}",
            subscription.terms.amount, subscription.terms.currency
        ),
    );
    ui::key_value("Frequency", &subscription.terms.frequency.to_string());
    ui::key_value("Description", &subscription.terms.description);

    // Sign as acceptor (Ed25519 only)
    let nonce_acceptor = rand::random::<[u8; 32]>();
    let acceptor_signature = signing::sign_subscription_ed25519(
        &subscription,
        &identity.keypair,
        &nonce_acceptor,
        60 * 60 * 24 * 7, // 7 days validity
    )?;

    // For CLI demo, we'll create a dummy proposer signature
    // In real usage, this would come from the actual proposer
    let nonce_proposer = rand::random::<[u8; 32]>();
    let proposer_signature = signing::sign_subscription_ed25519(
        &subscription,
        &identity.keypair,
        &nonce_proposer,
        60 * 60 * 24 * 7, // 7 days validity
    )?;

    // Create signed subscription (SigningKeyInfo removed from v0.2)
    let signed = paykit_subscriptions::SignedSubscription::new(
        subscription,
        proposer_signature,
        acceptor_signature,
    );

    // Verify signatures (method renamed to just verify_signatures in v0.2)
    if !signed.verify_signatures()? {
        return Err(anyhow!("Signature verification failed"));
    }

    // Save signed subscription
    storage.save_signed_subscription(&signed).await?;

    tracing::info!("Subscription accepted and signed: {}", subscription_id);
    ui::success("Subscription accepted and saved");
    ui::info(&format!("Subscription {} is now active", subscription_id));

    Ok(())
}

/// List subscription agreements
pub async fn list_subscriptions(
    storage_dir: &Path,
    peer: Option<String>,
    active_only: bool,
) -> Result<()> {
    let storage = create_subscription_storage(storage_dir)?;

    ui::header("Subscription Agreements");

    let subscriptions = if let Some(p) = peer {
        let peer_pk = resolve_recipient(storage_dir, &p)?;
        storage.list_subscriptions_with_peer(&peer_pk).await?
    } else if active_only {
        storage.list_active_subscriptions().await?
    } else {
        // List all signed subscriptions
        storage.list_active_subscriptions().await? // For now, same as active
    };

    if subscriptions.is_empty() {
        ui::info("No subscription agreements found.");
        return Ok(());
    }

    for sub in subscriptions {
        let subscription = &sub.subscription;
        ui::key_value(
            "Subscription ID",
            &subscription.subscription_id[..8.min(subscription.subscription_id.len())],
        );
        ui::key_value(
            "Subscriber",
            &subscription.subscriber.to_z32()[..20.min(subscription.subscriber.to_z32().len())],
        );
        ui::key_value(
            "Provider",
            &subscription.provider.to_z32()[..20.min(subscription.provider.to_z32().len())],
        );
        ui::key_value(
            "Amount",
            &format!(
                "{} {}",
                subscription.terms.amount, subscription.terms.currency
            ),
        );
        ui::key_value("Frequency", &subscription.terms.frequency.to_string());

        let created_dt = chrono::DateTime::from_timestamp(subscription.created_at, 0)
            .unwrap_or_else(chrono::Utc::now);
        ui::key_value(
            "Created",
            &created_dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        if sub.is_active() {
            ui::success("✓ ACTIVE");
        } else if sub.is_expired() {
            ui::warning("⚠ EXPIRED");
        } else {
            ui::info("○ PENDING START");
        }

        ui::separator();
    }

    Ok(())
}

/// Show subscription details
pub async fn show_subscription(storage_dir: &Path, subscription_id: &str) -> Result<()> {
    let storage = create_subscription_storage(storage_dir)?;

    ui::header("Subscription Details");

    // Try signed subscription first
    if let Some(signed) = storage.get_signed_subscription(subscription_id).await? {
        let subscription = &signed.subscription;

        ui::key_value("Subscription ID", &subscription.subscription_id);
        ui::key_value("Subscriber", &subscription.subscriber.to_z32());
        ui::key_value("Provider", &subscription.provider.to_z32());
        ui::key_value(
            "Amount",
            &format!(
                "{} {}",
                subscription.terms.amount, subscription.terms.currency
            ),
        );
        ui::key_value("Frequency", &subscription.terms.frequency.to_string());
        ui::key_value("Method", &subscription.terms.method.0);
        ui::key_value("Description", &subscription.terms.description);

        if let Some(max) = &subscription.terms.max_amount_per_period {
            ui::key_value("Max Amount Per Period", &max.to_string());
        }

        let created_dt = chrono::DateTime::from_timestamp(subscription.created_at, 0)
            .unwrap_or_else(chrono::Utc::now);
        ui::key_value(
            "Created",
            &created_dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        let starts_dt = chrono::DateTime::from_timestamp(subscription.starts_at, 0)
            .unwrap_or_else(chrono::Utc::now);
        ui::key_value("Starts", &starts_dt.format("%Y-%m-%d %H:%M:%S").to_string());

        if let Some(end) = subscription.ends_at {
            let ends_dt = chrono::DateTime::from_timestamp(end, 0).unwrap_or_else(chrono::Utc::now);
            ui::key_value("Ends", &ends_dt.format("%Y-%m-%d %H:%M:%S").to_string());
        }

        ui::separator();
        ui::info("Signature Info:");
        ui::key_value("Signature Type", "Ed25519 (v0.2)");

        if signed.is_active() {
            ui::success("✓ This subscription is ACTIVE");
        } else if signed.is_expired() {
            ui::warning("⚠ This subscription has EXPIRED");
        }
    } else if let Some(subscription) = storage.get_subscription(subscription_id).await? {
        // Unsigned proposal
        ui::warning("⚠ This is an unsigned proposal (not yet accepted)");
        ui::key_value("Subscription ID", &subscription.subscription_id);
        ui::key_value("Subscriber", &subscription.subscriber.to_z32());
        ui::key_value("Provider", &subscription.provider.to_z32());
        ui::key_value(
            "Amount",
            &format!(
                "{} {}",
                subscription.terms.amount, subscription.terms.currency
            ),
        );
        ui::key_value("Frequency", &subscription.terms.frequency.to_string());
        ui::key_value("Description", &subscription.terms.description);

        ui::separator();
        ui::info("Commands:");
        ui::info(&format!(
            "  Accept: paykit-demo subscriptions accept {}",
            subscription_id
        ));
    } else {
        return Err(anyhow!("Subscription {} not found", subscription_id));
    }

    Ok(())
}

/// Helper: parse frequency string
fn parse_frequency(freq: &str) -> Result<PaymentFrequency> {
    match freq.to_lowercase().as_str() {
        "daily" => Ok(PaymentFrequency::Daily),
        "weekly" => Ok(PaymentFrequency::Weekly),
        freq_str if freq_str.starts_with("monthly") => {
            // Parse "monthly" or "monthly:15" (day of month)
            let day = if let Some(day_str) = freq_str.strip_prefix("monthly:") {
                day_str.parse::<u8>()
                    .map_err(|_| anyhow!("Invalid day of month: {}", day_str))?
            } else {
                1 // Default to 1st of month
            };
            if day == 0 || day > 31 {
                return Err(anyhow!("Day of month must be between 1 and 31"));
            }
            Ok(PaymentFrequency::Monthly { day_of_month: day })
        }
        freq_str if freq_str.starts_with("yearly") => {
            // Parse "yearly:6:15" (month:day)
            let parts: Vec<&str> = freq_str.split(':').collect();
            if parts.len() != 3 {
                return Err(anyhow!("Yearly frequency must be in format 'yearly:MONTH:DAY'"));
            }
            let month = parts[1].parse::<u8>()
                .map_err(|_| anyhow!("Invalid month: {}", parts[1]))?;
            let day = parts[2].parse::<u8>()
                .map_err(|_| anyhow!("Invalid day: {}", parts[2]))?;
            if month == 0 || month > 12 {
                return Err(anyhow!("Month must be between 1 and 12"));
            }
            if day == 0 || day > 31 {
                return Err(anyhow!("Day must be between 1 and 31"));
            }
            Ok(PaymentFrequency::Yearly { month, day })
        }
        freq_str if freq_str.starts_with("custom:") => {
            // Parse "custom:3600" (interval in seconds)
            let interval_str = freq_str.strip_prefix("custom:")
                .ok_or_else(|| anyhow!("Invalid custom frequency format"))?;
            let interval = interval_str.parse::<u64>()
                .map_err(|_| anyhow!("Invalid interval: {}", interval_str))?;
            Ok(PaymentFrequency::Custom { interval_seconds: interval })
        }
        _ => Err(anyhow!("Invalid frequency: {}. Use daily, weekly, monthly[:DAY], yearly:MONTH:DAY, or custom:SECONDS", freq)),
    }
}

// ============================================================
// Phase 3: Auto-Pay Commands
// ============================================================

/// Enable auto-pay for a subscription
#[tracing::instrument(skip(storage_dir))]
pub async fn enable_autopay(
    storage_dir: &Path,
    subscription_id: &str,
    max_amount: Option<String>,
    require_confirmation: bool,
) -> Result<()> {
    let storage = create_subscription_storage(storage_dir)?;

    ui::header("Enable Auto-Pay");

    // Load subscription
    let subscription = storage
        .get_signed_subscription(subscription_id)
        .await?
        .ok_or_else(|| anyhow!("Subscription {} not found", subscription_id))?;

    ui::key_value("Subscription ID", subscription_id);
    ui::key_value("Provider", &subscription.subscription.provider.to_z32());
    ui::key_value(
        "Amount",
        &format!(
            "{} {}",
            subscription.subscription.terms.amount, subscription.subscription.terms.currency
        ),
    );

    // Create auto-pay rule
    let mut rule = paykit_subscriptions::AutoPayRule::new(
        subscription_id.to_string(),
        subscription.subscription.provider.clone(),
        subscription.subscription.terms.method.clone(),
    );

    if let Some(max) = max_amount {
        let max_sats: i64 = max
            .parse()
            .map_err(|_| anyhow!("Invalid max amount: {}", max))?;
        rule = rule.with_max_payment_amount(Amount::from_sats(max_sats));
    }

    rule = rule.with_confirmation(require_confirmation);

    // Validate
    rule.validate()?;

    // Save
    storage.save_autopay_rule(&rule).await?;

    tracing::info!("Auto-pay rule saved for subscription: {}", subscription_id);
    ui::success("Auto-pay enabled");
    ui::key_value(
        "Manual Confirmation Required",
        &require_confirmation.to_string(),
    );

    if let Some(ref max) = rule.max_amount_per_payment {
        ui::key_value("Max Payment Amount", &max.to_string());
    }

    Ok(())
}

/// Disable auto-pay for a subscription
pub async fn disable_autopay(storage_dir: &Path, subscription_id: &str) -> Result<()> {
    let storage = create_subscription_storage(storage_dir)?;

    ui::header("Disable Auto-Pay");

    // Load rule
    if let Some(mut rule) = storage.get_autopay_rule(subscription_id).await? {
        rule.enabled = false;
        storage.save_autopay_rule(&rule).await?;
        ui::success(&format!(
            "Auto-pay disabled for subscription {}",
            subscription_id
        ));
    } else {
        ui::warning(&format!(
            "No auto-pay rule found for subscription {}",
            subscription_id
        ));
    }

    Ok(())
}

/// Show auto-pay status
pub async fn show_autopay_status(storage_dir: &Path, subscription_id: &str) -> Result<()> {
    let storage = create_subscription_storage(storage_dir)?;

    ui::header("Auto-Pay Status");

    if let Some(rule) = storage.get_autopay_rule(subscription_id).await? {
        ui::key_value("Subscription ID", subscription_id);
        ui::key_value("Enabled", &rule.enabled.to_string());
        ui::key_value("Peer", &rule.peer.to_z32());
        ui::key_value("Method", &rule.method_id.0);

        if let Some(ref max) = rule.max_amount_per_payment {
            ui::key_value("Max Payment Amount", &max.to_string());
        }

        if let Some(ref period_max) = rule.max_total_amount_per_period {
            ui::key_value("Max Per Period", &period_max.to_string());
            if let Some(ref period) = rule.period {
                ui::key_value("Period", period);
            }
        }

        ui::key_value(
            "Requires Confirmation",
            &rule.require_confirmation.to_string(),
        );

        if let Some(notify) = rule.notify_before {
            ui::key_value("Notify Before (seconds)", &notify.to_string());
        }
    } else {
        ui::info(&format!(
            "No auto-pay rule configured for subscription {}",
            subscription_id
        ));
    }

    Ok(())
}

/// Set spending limit for a peer
#[tracing::instrument(skip(storage_dir))]
pub async fn set_peer_limit(
    storage_dir: &Path,
    peer: &str,
    limit: &str,
    period: &str,
) -> Result<()> {
    let peer_pk = resolve_recipient(storage_dir, peer)?;
    let storage = create_subscription_storage(storage_dir)?;

    ui::header("Set Spending Limit");

    // Validate period
    let valid_periods = ["daily", "weekly", "monthly"];
    if !valid_periods.contains(&period) {
        return Err(anyhow!("Period must be one of: daily, weekly, monthly"));
    }

    // Parse limit as satoshis
    let limit_sats: i64 = limit
        .parse()
        .map_err(|_| anyhow!("Invalid limit: {}", limit))?;

    // Create limit
    let spending_limit = paykit_subscriptions::PeerSpendingLimit::new(
        peer_pk.clone(),
        Amount::from_sats(limit_sats),
        period.to_string(),
    );

    // Save
    storage.save_peer_limit(&spending_limit).await?;

    tracing::info!("Spending limit set for peer: {}", peer);
    ui::success("Spending limit set");
    ui::key_value("Peer", peer);
    ui::key_value("Limit", &format!("{} per {}", limit, period));

    Ok(())
}

/// Show spending limits
pub async fn show_peer_limits(storage_dir: &Path, peer: Option<String>) -> Result<()> {
    let storage = create_subscription_storage(storage_dir)?;

    ui::header("Spending Limits");

    if let Some(p) = peer {
        // Show specific peer
        let peer_pk = resolve_recipient(storage_dir, &p)?;

        if let Some(limit) = storage.get_peer_limit(&peer_pk).await? {
            ui::key_value("Peer", &limit.peer.to_z32());
            ui::key_value(
                "Total Limit",
                &format!("{} per {}", limit.total_amount_limit, limit.period),
            );
            ui::key_value("Current Spent", &limit.current_spent.to_string());
            ui::key_value("Remaining", &limit.remaining_limit().to_string());
            ui::key_value("Last Reset", &limit.last_reset.to_string());
        } else {
            ui::info(&format!("No spending limit set for peer {}", p));
        }
    } else {
        ui::info("Use --peer <peer> to view specific peer limits");
    }

    Ok(())
}
