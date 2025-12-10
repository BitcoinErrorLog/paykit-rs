# Auto-Pay and Spending Limits Guide

This guide covers Paykit's auto-pay system for automated recurring payments with configurable spending limits and approval rules.

## Table of Contents

- [Overview](#overview)
- [Core Concepts](#core-concepts)
- [Setting Up Auto-Pay](#setting-up-auto-pay)
- [Spending Limits](#spending-limits)
- [Auto-Pay Rules](#auto-pay-rules)
- [Implementation Examples](#implementation-examples)
- [API Reference](#api-reference)
- [Security Considerations](#security-considerations)

## Overview

Paykit's auto-pay system enables:

- **Automated recurring payments** for subscriptions
- **Peer-specific spending limits** to control exposure
- **Configurable approval rules** per subscription
- **Period-based limit resets** (daily, weekly, monthly)
- **Real-time spending tracking** with atomic operations

## Core Concepts

### Spending Limits

A spending limit controls how much can be auto-paid to a specific peer within a time period:

```
┌─────────────────────────────────────────┐
│          Peer Spending Limit            │
├─────────────────────────────────────────┤
│ Peer: pubky://abc123...                 │
│ Total Limit: 50,000 sats                │
│ Period: monthly                         │
│ Current Spent: 15,000 sats              │
│ Remaining: 35,000 sats                  │
│ Last Reset: 2024-01-01 00:00:00 UTC     │
└─────────────────────────────────────────┘
```

### Auto-Pay Rules

An auto-pay rule defines when a payment can be automatically approved:

```
┌─────────────────────────────────────────┐
│           Auto-Pay Rule                 │
├─────────────────────────────────────────┤
│ Subscription: sub_premium               │
│ Peer: pubky://provider...               │
│ Method: lightning                       │
│ Max Per Payment: 10,000 sats            │
│ Max Per Period: 50,000 sats             │
│ Period: monthly                         │
│ Require Confirmation: false             │
│ Notify Before: 3600 seconds             │
│ Enabled: true                           │
└─────────────────────────────────────────┘
```

### Payment Approval Flow

```
Payment Request
      │
      ▼
┌──────────────────┐
│ Check if Enabled │──No──▶ Manual Approval
└────────┬─────────┘
         │ Yes
         ▼
┌──────────────────┐
│ Within Per-       │──No──▶ Reject/Manual
│ Payment Limit?   │
└────────┬─────────┘
         │ Yes
         ▼
┌──────────────────┐
│ Within Spending   │──No──▶ Reject/Manual
│ Limit?           │
└────────┬─────────┘
         │ Yes
         ▼
┌──────────────────┐
│ Require           │──Yes─▶ Request
│ Confirmation?    │        Confirmation
└────────┬─────────┘
         │ No
         ▼
   Auto-Approve
```

## Setting Up Auto-Pay

### Step 1: Set Spending Limits

First, configure how much you're willing to auto-pay to each peer:

**CLI:**
```bash
# Set daily limit for a peer
paykit-demo subscriptions set-limit <peer_pubkey> 10000 daily

# Set monthly limit
paykit-demo subscriptions set-limit <peer_pubkey> 100000 monthly
```

**Rust:**
```rust
use paykit_subscriptions::{Amount, PeerSpendingLimit};

let limit = PeerSpendingLimit::new(
    peer_public_key,
    Amount::from_sats(100000),  // 100k sats
    "monthly".to_string(),
);

storage.save_peer_limit(&limit).await?;
```

### Step 2: Configure Auto-Pay Rules

Create rules for each subscription you want to automate:

**CLI:**
```bash
# Configure auto-pay with limits
paykit-demo subscriptions add-autopay \
    --subscription sub_premium \
    --max-amount 10000 \
    --period monthly \
    --method lightning
```

**Rust:**
```rust
use paykit_subscriptions::AutoPayRule;

let rule = AutoPayRule::new(
    "sub_premium".to_string(),
    peer_public_key,
    MethodId("lightning".to_string()),
)
.with_max_payment_amount(Amount::from_sats(10000))
.with_max_period_amount(Amount::from_sats(50000), "monthly".to_string())
.with_notification(3600);  // Notify 1 hour before

storage.save_autopay_rule(&rule).await?;
```

### Step 3: Enable/Disable Rules

Toggle auto-pay without deleting the configuration:

**CLI:**
```bash
# Enable
paykit-demo subscriptions toggle-autopay sub_premium --enable

# Disable
paykit-demo subscriptions toggle-autopay sub_premium --disable
```

## Spending Limits

### Period Types

| Period | Reset Frequency | Use Case |
|--------|-----------------|----------|
| `daily` | Every 24 hours | High-frequency small payments |
| `weekly` | Every 7 days | Regular service payments |
| `monthly` | Every 30 days | Subscription payments |

### Limit Operations

**Check Remaining Limit:**
```rust
let limit = storage.get_peer_limit(&peer).await?;
let remaining = limit.remaining_limit();
println!("Remaining: {} sats", remaining.as_sats());
```

**Record a Payment:**
```rust
limit.add_spent(&Amount::from_sats(5000))?;
storage.save_peer_limit(&limit).await?;
```

**Reset Limit:**
```rust
limit.reset();
storage.save_peer_limit(&limit).await?;
```

**Check Auto-Reset:**
```rust
if limit.should_reset() {
    limit.reset();
    storage.save_peer_limit(&limit).await?;
}
```

### Atomic Reservations

For payment processing, use atomic reservations to prevent race conditions:

```rust
// Reserve spending before payment
let token = storage.try_reserve_spending(&peer, &amount).await?;

// Execute payment
match execute_payment(...).await {
    Ok(_) => {
        // Commit the reservation
        storage.commit_spending(token).await?;
    }
    Err(_) => {
        // Rollback on failure
        storage.rollback_spending(token).await?;
    }
}
```

## Auto-Pay Rules

### Rule Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `max_amount_per_payment` | Maximum per single payment | None (unlimited) |
| `max_total_amount_per_period` | Maximum per time period | None (unlimited) |
| `period` | Reset period (daily/weekly/monthly) | monthly |
| `require_confirmation` | Require user confirmation | false |
| `notify_before` | Seconds before payment to notify | 3600 (1 hour) |
| `enabled` | Whether rule is active | true |

### Checking Auto-Pay Eligibility

```rust
use paykit_demo_core::SubscriptionCoordinator;

let coordinator = SubscriptionCoordinator::new(storage_path)?;

// Check if a payment can be auto-approved
if coordinator.can_auto_pay(&peer, &Amount::from_sats(5000))? {
    // Payment can proceed automatically
    coordinator.record_auto_payment(&peer, Amount::from_sats(5000))?;
} else {
    // Requires manual approval
    show_approval_dialog();
}
```

## Implementation Examples

### iOS (Swift)

```swift
import PaykitMobile

// Set up spending limit
let storage = AutoPayStorage(keychain: KeychainStorage.default)
storage.setSpendingLimit(
    peer: peerPubkey,
    limit: 100000,  // sats
    period: .monthly
)

// Configure auto-pay rule
let rule = AutoPayRule(
    subscriptionId: "sub_premium",
    peer: peerPubkey,
    maxPerPayment: 10000,
    maxPerPeriod: 50000,
    period: .monthly,
    enabled: true
)
storage.saveAutoPayRule(rule)

// Check if payment can proceed
if storage.canAutoApprove(peer: peerPubkey, amount: 5000) {
    // Execute payment
    storage.recordPayment(peer: peerPubkey, amount: 5000)
} else {
    // Show manual approval UI
    showApprovalSheet()
}
```

### Android (Kotlin)

```kotlin
import com.paykit.autopay.AutoPayStorage

// Set up spending limit
val storage = AutoPayStorage(context)
storage.setSpendingLimit(
    peer = peerPubkey,
    limit = 100_000L,  // sats
    period = "monthly"
)

// Configure auto-pay rule
val rule = AutoPayRule(
    subscriptionId = "sub_premium",
    peer = peerPubkey,
    maxPerPayment = 10_000L,
    maxPerPeriod = 50_000L,
    period = "monthly",
    enabled = true
)
storage.saveAutoPayRule(rule)

// Check if payment can proceed
if (storage.canAutoApprove(peerPubkey, 5000L)) {
    // Execute payment
    storage.recordPayment(peerPubkey, 5000L)
} else {
    // Show manual approval UI
    showApprovalDialog()
}
```

### Web (JavaScript)

```javascript
// Set up spending limit
const storage = new AutoPayStorage(localStorage);
storage.setGlobalSettings({
    enabled: true,
    dailyLimit: 100000,
    currentUsage: 0
});

// Add peer limit
storage.addPeerLimit({
    peer: peerPubkey,
    limit: 50000,
    period: 'monthly',
    spent: 0
});

// Check and record payment
if (storage.canAutoApprove(peerPubkey, 5000)) {
    storage.recordPayment(peerPubkey, 5000);
    executePayment();
} else {
    showApprovalModal();
}
```

### CLI

```bash
# View current limits
paykit-demo subscriptions list-limits

# View auto-pay rules
paykit-demo subscriptions list-autopay

# View recent auto-payments
paykit-demo subscriptions recent-payments

# Configure global settings
paykit-demo subscriptions configure-global --enable --daily-limit 100000

# Reset a peer's spending
paykit-demo subscriptions reset-limit <peer_pubkey>
```

## API Reference

### PeerSpendingLimit

```rust
pub struct PeerSpendingLimit {
    pub peer: PublicKey,
    pub total_amount_limit: Amount,
    pub period: String,  // "daily", "weekly", "monthly"
    pub current_spent: Amount,
    pub last_reset: DateTime<Utc>,
}

impl PeerSpendingLimit {
    pub fn new(peer: PublicKey, limit: Amount, period: String) -> Self;
    pub fn would_exceed_limit(&self, amount: &Amount) -> bool;
    pub fn add_spent(&mut self, amount: &Amount) -> Result<()>;
    pub fn should_reset(&self) -> bool;
    pub fn reset(&mut self);
    pub fn remaining_limit(&self) -> Amount;
}
```

### AutoPayRule

```rust
pub struct AutoPayRule {
    pub subscription_id: String,
    pub peer: PublicKey,
    pub method_id: MethodId,
    pub enabled: bool,
    pub max_amount_per_payment: Option<Amount>,
    pub max_total_amount_per_period: Option<Amount>,
    pub period: Option<String>,
    pub require_confirmation: bool,
    pub notify_before: Option<u64>,
}

impl AutoPayRule {
    pub fn new(subscription_id: String, peer: PublicKey, method_id: MethodId) -> Self;
    pub fn with_max_payment_amount(self, amount: Amount) -> Self;
    pub fn with_max_period_amount(self, amount: Amount, period: String) -> Self;
    pub fn with_confirmation(self, required: bool) -> Self;
    pub fn with_notification(self, seconds_before: u64) -> Self;
    pub fn validate(&self) -> Result<()>;
    pub fn is_amount_within_limit(&self, amount: &Amount) -> bool;
}
```

### SubscriptionStorage Trait

```rust
#[async_trait]
pub trait SubscriptionStorage {
    // Auto-pay rules
    async fn save_autopay_rule(&self, rule: &AutoPayRule) -> Result<()>;
    async fn get_autopay_rule(&self, subscription_id: &str) -> Result<Option<AutoPayRule>>;
    
    // Spending limits
    async fn save_peer_limit(&self, limit: &PeerSpendingLimit) -> Result<()>;
    async fn get_peer_limit(&self, peer: &PublicKey) -> Result<Option<PeerSpendingLimit>>;
    
    // Atomic operations
    async fn try_reserve_spending(&self, peer: &PublicKey, amount: &Amount) -> Result<SpendingToken>;
    async fn commit_spending(&self, token: SpendingToken) -> Result<()>;
    async fn rollback_spending(&self, token: SpendingToken) -> Result<()>;
}
```

## Security Considerations

### Limit Configuration

- **Start conservative**: Set lower limits initially and increase as trust builds
- **Use per-payment limits**: Prevent single large unauthorized payments
- **Enable notifications**: Get alerts before payments execute

### Storage Security

- **Encrypt limit data**: Use platform-secure storage (Keychain, EncryptedSharedPreferences)
- **Protect rule modifications**: Require authentication to change auto-pay settings
- **Log all auto-payments**: Maintain an audit trail

### Attack Vectors

| Vector | Mitigation |
|--------|------------|
| Limit exhaustion | Per-payment caps, notifications |
| Unauthorized rule changes | Auth required for settings |
| Race conditions | Atomic reservations |
| Replay attacks | Unique payment IDs, nonce tracking |

### Best Practices

1. **Require confirmation** for high-value auto-payments
2. **Set notification thresholds** for unusual activity
3. **Review spending regularly** using the dashboard/CLI
4. **Use separate limits** for each peer/provider
5. **Disable unused rules** rather than deleting them

## Related Documentation

- [Subscriptions Guide](../paykit-subscriptions/README.md)
- [Mobile Integration](./mobile-integration.md)
- [CLI Reference](../paykit-demo-cli/README.md)
- [Web Demo Guide](../paykit-demo-web/README.md)
