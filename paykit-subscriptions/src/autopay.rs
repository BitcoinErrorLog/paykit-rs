use crate::{Amount, Result, SubscriptionError};
use paykit_lib::{MethodId, PublicKey};
use serde::{Deserialize, Serialize};

/// Auto-pay configuration per subscription
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutoPayRule {
    pub subscription_id: String,
    pub peer: PublicKey,
    pub method_id: MethodId,
    pub enabled: bool,
    pub max_amount_per_payment: Option<Amount>,
    pub max_total_amount_per_period: Option<Amount>,
    pub period: Option<String>, // "daily", "weekly", "monthly"
    pub require_confirmation: bool,
    pub notify_before: Option<u64>, // Seconds before payment
}

impl AutoPayRule {
    /// Create a new auto-pay rule
    pub fn new(subscription_id: String, peer: PublicKey, method_id: MethodId) -> Self {
        Self {
            subscription_id,
            peer,
            method_id,
            enabled: true,
            max_amount_per_payment: None,
            max_total_amount_per_period: None,
            period: Some("monthly".to_string()),
            require_confirmation: false,
            notify_before: Some(3600), // 1 hour default
        }
    }

    /// Set maximum amount per individual payment
    pub fn with_max_payment_amount(mut self, amount: Amount) -> Self {
        self.max_amount_per_payment = Some(amount);
        self
    }

    /// Set maximum total amount per period
    pub fn with_max_period_amount(mut self, amount: Amount, period: String) -> Self {
        self.max_total_amount_per_period = Some(amount);
        self.period = Some(period);
        self
    }

    /// Require manual confirmation before payment
    pub fn with_confirmation(mut self, required: bool) -> Self {
        self.require_confirmation = required;
        self
    }

    /// Set notification time before payment
    pub fn with_notification(mut self, seconds_before: u64) -> Self {
        self.notify_before = Some(seconds_before);
        self
    }

    /// Validate the rule
    pub fn validate(&self) -> Result<()> {
        if self.subscription_id.is_empty() {
            return Err(SubscriptionError::InvalidArgument(
                "Subscription ID cannot be empty".to_string(),
            )
            .into());
        }

        // Amount types are always valid (no validation needed)
        Ok(())
    }

    /// Check if a payment amount is within limits
    pub fn is_amount_within_limit(&self, amount: &Amount) -> bool {
        if let Some(ref max) = self.max_amount_per_payment {
            return amount.is_within_limit(max);
        }
        true // No limit set
    }
}

/// Spending limit per peer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeerSpendingLimit {
    pub peer: PublicKey,
    pub total_amount_limit: Amount,
    pub period: String, // "daily", "weekly", "monthly"
    pub current_spent: Amount,
    pub last_reset: chrono::DateTime<chrono::Utc>,
}

impl PeerSpendingLimit {
    /// Create a new spending limit
    pub fn new(peer: PublicKey, limit: Amount, period: String) -> Self {
        Self {
            peer,
            total_amount_limit: limit,
            period,
            current_spent: Amount::from_sats(0),
            last_reset: chrono::Utc::now(),
        }
    }

    /// Check if adding an amount would exceed the limit
    pub fn would_exceed_limit(&self, amount: &Amount) -> bool {
        if let Some(new_spent) = self.current_spent.checked_add(amount) {
            return !new_spent.is_within_limit(&self.total_amount_limit);
        }
        true // Overflow would occur
    }

    /// Add spent amount
    pub fn add_spent(&mut self, amount: &Amount) -> Result<()> {
        self.current_spent = self
            .current_spent
            .checked_add(amount)
            .ok_or_else(|| SubscriptionError::InvalidArgument("Amount overflow".to_string()))?;
        Ok(())
    }

    /// Check if limit should be reset based on period
    pub fn should_reset(&self) -> bool {
        let now = chrono::Utc::now();
        match self.period.as_str() {
            "daily" => (now - self.last_reset).num_days() >= 1,
            "weekly" => (now - self.last_reset).num_days() >= 7,
            "monthly" => (now - self.last_reset).num_days() >= 30,
            _ => false,
        }
    }

    /// Reset spending counter
    pub fn reset(&mut self) {
        self.current_spent = Amount::from_sats(0);
        self.last_reset = chrono::Utc::now();
    }

    /// Get remaining limit
    pub fn remaining_limit(&self) -> Amount {
        self.total_amount_limit
            .checked_sub(&self.current_spent)
            .unwrap_or(Amount::from_sats(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn test_pubkey() -> PublicKey {
        let keypair = pkarr::Keypair::random();
        PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
    }

    #[test]
    fn test_autopay_rule_creation() {
        let peer = test_pubkey();
        let rule = AutoPayRule::new(
            "sub_123".to_string(),
            peer,
            MethodId("lightning".to_string()),
        );

        assert_eq!(rule.subscription_id, "sub_123");
        assert!(rule.enabled);
        assert!(!rule.require_confirmation);
        assert_eq!(rule.notify_before, Some(3600));
    }

    #[test]
    fn test_autopay_rule_with_limits() {
        let peer = test_pubkey();
        let rule = AutoPayRule::new(
            "sub_123".to_string(),
            peer,
            MethodId("lightning".to_string()),
        )
        .with_max_payment_amount(Amount::from_sats(100))
        .with_max_period_amount(Amount::from_sats(500), "monthly".to_string());

        assert_eq!(rule.max_amount_per_payment, Some(Amount::from_sats(100)));
        assert_eq!(
            rule.max_total_amount_per_period,
            Some(Amount::from_sats(500))
        );
        assert_eq!(rule.period, Some("monthly".to_string()));
    }

    #[test]
    fn test_autopay_rule_amount_check() {
        let peer = test_pubkey();
        let rule = AutoPayRule::new(
            "sub_123".to_string(),
            peer,
            MethodId("lightning".to_string()),
        )
        .with_max_payment_amount(Amount::from_sats(100));

        assert!(rule.is_amount_within_limit(&Amount::from_sats(50)));
        assert!(rule.is_amount_within_limit(&Amount::from_sats(100)));
        assert!(!rule.is_amount_within_limit(&Amount::from_sats(150)));
    }

    #[test]
    fn test_peer_spending_limit() {
        let peer = test_pubkey();
        let mut limit =
            PeerSpendingLimit::new(peer, Amount::from_sats(1000), "monthly".to_string());

        assert_eq!(limit.current_spent, Amount::from_sats(0));
        assert_eq!(limit.remaining_limit(), Amount::from_sats(1000));

        // Add spending
        limit.add_spent(&Amount::from_sats(300)).unwrap();
        assert_eq!(limit.current_spent, Amount::from_sats(300));
        assert_eq!(limit.remaining_limit(), Amount::from_sats(700));

        // Check if would exceed
        assert!(!limit.would_exceed_limit(&Amount::from_sats(600)));
        assert!(limit.would_exceed_limit(&Amount::from_sats(800)));

        // Reset
        limit.reset();
        assert_eq!(limit.current_spent, Amount::from_sats(0));
        assert_eq!(limit.remaining_limit(), Amount::from_sats(1000));
    }

    #[test]
    fn test_spending_limit_period_reset() {
        let peer = test_pubkey();
        let mut limit = PeerSpendingLimit::new(peer, Amount::from_sats(1000), "daily".to_string());

        // Set last reset to 2 days ago
        limit.last_reset = chrono::Utc::now() - chrono::Duration::days(2);
        assert!(limit.should_reset());

        // After reset, should not need reset
        limit.reset();
        assert!(!limit.should_reset());
    }
}
