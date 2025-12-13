use crate::{Amount, Result, SubscriptionError};
use paykit_lib::{MethodId, PublicKey};
use serde::{Deserialize, Serialize};

/// A subscription agreement between two parties
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subscription {
    pub subscription_id: String,
    pub subscriber: PublicKey,
    pub provider: PublicKey,
    pub terms: SubscriptionTerms,
    pub metadata: serde_json::Value,
    pub created_at: i64,
    pub starts_at: i64,
    pub ends_at: Option<i64>,
}

impl Subscription {
    /// Create a new subscription
    pub fn new(subscriber: PublicKey, provider: PublicKey, terms: SubscriptionTerms) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            subscription_id: format!("sub_{}", now),
            subscriber,
            provider,
            terms,
            metadata: serde_json::json!({}),
            created_at: now,
            starts_at: now,
            ends_at: None,
        }
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set start time
    pub fn with_starts_at(mut self, starts_at: i64) -> Self {
        self.starts_at = starts_at;
        self
    }

    /// Set end time
    pub fn with_ends_at(mut self, ends_at: i64) -> Self {
        self.ends_at = Some(ends_at);
        self
    }

    /// Check if subscription is currently active
    pub fn is_active(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now >= self.starts_at && self.ends_at.is_none_or(|end| now < end)
    }

    /// Check if subscription has expired
    pub fn is_expired(&self) -> bool {
        self.ends_at
            .is_some_and(|end| chrono::Utc::now().timestamp() >= end)
    }

    /// Validate subscription data
    pub fn validate(&self) -> Result<()> {
        if self.subscription_id.is_empty() {
            return Err(SubscriptionError::InvalidArgument(
                "Subscription ID cannot be empty".to_string(),
            )
            .into());
        }
        if self.subscriber == self.provider {
            return Err(SubscriptionError::InvalidArgument(
                "Subscriber and provider must be different".to_string(),
            )
            .into());
        }
        if self.starts_at < 0 {
            return Err(SubscriptionError::InvalidArgument(
                "Start time cannot be negative".to_string(),
            )
            .into());
        }
        if let Some(end) = self.ends_at {
            if end <= self.starts_at {
                return Err(SubscriptionError::InvalidArgument(
                    "End time must be after start time".to_string(),
                )
                .into());
            }
        }
        self.terms.validate()?;
        Ok(())
    }

    /// Get canonical bytes for signing
    pub fn to_signing_bytes(&self) -> Result<Vec<u8>> {
        // Use deterministic JSON serialization for signing
        let json = serde_json::to_vec(self)?;
        Ok(json)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubscriptionTerms {
    pub amount: Amount,
    pub currency: String,
    pub frequency: PaymentFrequency,
    pub method: MethodId,
    pub max_amount_per_period: Option<Amount>,
    pub description: String,
}

impl SubscriptionTerms {
    /// Create new subscription terms
    pub fn new(
        amount: Amount,
        currency: String,
        frequency: PaymentFrequency,
        method: MethodId,
        description: String,
    ) -> Self {
        Self {
            amount,
            currency,
            frequency,
            method,
            max_amount_per_period: None,
            description,
        }
    }

    /// Set max amount per period
    pub fn with_max_amount(mut self, max_amount: Amount) -> Self {
        self.max_amount_per_period = Some(max_amount);
        self
    }

    /// Validate terms
    pub fn validate(&self) -> Result<()> {
        if self.currency.is_empty() {
            return Err(
                SubscriptionError::InvalidArgument("Currency cannot be empty".to_string()).into(),
            );
        }
        if self.description.is_empty() {
            return Err(SubscriptionError::InvalidArgument(
                "Description cannot be empty".to_string(),
            )
            .into());
        }

        // Amount types are always valid (no validation needed)
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentFrequency {
    Daily,
    Weekly,
    Monthly { day_of_month: u8 },
    Yearly { month: u8, day: u8 },
    Custom { interval_seconds: u64 },
}

impl PaymentFrequency {
    /// Get interval in seconds
    pub fn to_seconds(&self) -> u64 {
        match self {
            PaymentFrequency::Daily => 86400,
            PaymentFrequency::Weekly => 86400 * 7,
            PaymentFrequency::Monthly { .. } => 86400 * 30, // Approximate
            PaymentFrequency::Yearly { .. } => 86400 * 365, // Approximate
            PaymentFrequency::Custom { interval_seconds } => *interval_seconds,
        }
    }

    /// Human-readable description (deprecated, use Display trait)
    pub fn description(&self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for PaymentFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentFrequency::Daily => write!(f, "Daily"),
            PaymentFrequency::Weekly => write!(f, "Weekly"),
            PaymentFrequency::Monthly { day_of_month } => {
                write!(f, "Monthly (day {})", day_of_month)
            }
            PaymentFrequency::Yearly { month, day } => write!(f, "Yearly ({}/{})", month, day),
            PaymentFrequency::Custom { interval_seconds } => {
                write!(f, "Every {} seconds", interval_seconds)
            }
        }
    }
}

/// A subscription signed by both parties (Ed25519 only)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignedSubscription {
    pub subscription: Subscription,
    pub subscriber_signature: crate::signing::Signature,
    pub provider_signature: crate::signing::Signature,
}

impl SignedSubscription {
    /// Create a new signed subscription
    pub fn new(
        subscription: Subscription,
        subscriber_signature: crate::signing::Signature,
        provider_signature: crate::signing::Signature,
    ) -> Self {
        Self {
            subscription,
            subscriber_signature,
            provider_signature,
        }
    }

    /// Verify both Ed25519 signatures
    pub fn verify_signatures(&self) -> Result<bool> {
        // Verify subscriber signature
        let subscriber_valid = crate::signing::verify_signature_ed25519(
            &self.subscription,
            &self.subscriber_signature,
        )?;

        // Verify provider signature
        let provider_valid =
            crate::signing::verify_signature_ed25519(&self.subscription, &self.provider_signature)?;

        Ok(subscriber_valid && provider_valid)
    }

    /// Check if subscription is currently active
    pub fn is_active(&self) -> bool {
        self.subscription.is_active()
    }

    /// Check if subscription has expired
    pub fn is_expired(&self) -> bool {
        self.subscription.is_expired()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn test_pubkey(_name: &str) -> PublicKey {
        let keypair = pkarr::Keypair::random();
        PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
    }

    #[test]
    fn test_subscription_creation() {
        let subscriber = test_pubkey("sub");
        let provider = test_pubkey("prov");
        let terms = SubscriptionTerms::new(
            Amount::from_sats(100),
            "SAT".to_string(),
            PaymentFrequency::Monthly { day_of_month: 1 },
            MethodId("lightning".to_string()),
            "Test subscription".to_string(),
        );

        let sub = Subscription::new(subscriber.clone(), provider.clone(), terms.clone());
        assert!(!sub.subscription_id.is_empty());
        assert_eq!(sub.subscriber, subscriber);
        assert_eq!(sub.provider, provider);
        assert_eq!(sub.terms, terms);
    }

    #[test]
    fn test_subscription_validation() {
        let subscriber = test_pubkey("sub");
        let provider = test_pubkey("prov");
        let terms = SubscriptionTerms::new(
            Amount::from_sats(100),
            "SAT".to_string(),
            PaymentFrequency::Daily,
            MethodId("lightning".to_string()),
            "Test".to_string(),
        );

        let sub = Subscription::new(subscriber.clone(), provider, terms);
        assert!(sub.validate().is_ok());

        // Test same subscriber and provider
        let mut invalid_sub = sub.clone();
        invalid_sub.provider = subscriber.clone();
        assert!(invalid_sub.validate().is_err());

        // Test invalid end time
        let mut invalid_sub = sub.clone();
        invalid_sub.ends_at = Some(invalid_sub.starts_at - 1);
        assert!(invalid_sub.validate().is_err());
    }

    #[test]
    fn test_subscription_active_status() {
        let subscriber = test_pubkey("sub");
        let provider = test_pubkey("prov");
        let terms = SubscriptionTerms::new(
            Amount::from_sats(100),
            "SAT".to_string(),
            PaymentFrequency::Daily,
            MethodId("lightning".to_string()),
            "Test".to_string(),
        );

        let sub = Subscription::new(subscriber, provider, terms);
        assert!(sub.is_active());
        assert!(!sub.is_expired());

        // Test future start
        let mut future_sub = sub.clone();
        future_sub.starts_at = chrono::Utc::now().timestamp() + 3600;
        assert!(!future_sub.is_active());

        // Test expired
        let mut expired_sub = sub.clone();
        expired_sub.ends_at = Some(chrono::Utc::now().timestamp() - 3600);
        assert!(!expired_sub.is_active());
        assert!(expired_sub.is_expired());
    }

    #[test]
    fn test_terms_validation() {
        let terms = SubscriptionTerms::new(
            Amount::from_sats(100),
            "SAT".to_string(),
            PaymentFrequency::Daily,
            MethodId("lightning".to_string()),
            "Test".to_string(),
        );
        assert!(terms.validate().is_ok());

        // Test empty currency
        let mut invalid_terms = terms.clone();
        invalid_terms.currency = "".to_string();
        assert!(invalid_terms.validate().is_err());

        // Test empty description
        let mut invalid_terms = terms.clone();
        invalid_terms.description = "".to_string();
        assert!(invalid_terms.validate().is_err());
    }

    #[test]
    fn test_payment_frequency() {
        assert_eq!(PaymentFrequency::Daily.to_seconds(), 86400);
        assert_eq!(PaymentFrequency::Weekly.to_seconds(), 86400 * 7);
        assert_eq!(
            PaymentFrequency::Custom {
                interval_seconds: 12345
            }
            .to_seconds(),
            12345
        );

        assert_eq!(PaymentFrequency::Daily.to_string(), "Daily");
        assert_eq!(PaymentFrequency::Weekly.to_string(), "Weekly");
        assert_eq!(
            PaymentFrequency::Monthly { day_of_month: 15 }.to_string(),
            "Monthly (day 15)"
        );
    }
}
