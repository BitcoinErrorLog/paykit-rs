//! Subscription Modifications
//!
//! This module provides functionality for modifying existing subscriptions,
//! including upgrades, downgrades, method changes, and billing date changes.
//!
//! # Overview
//!
//! Subscription modifications allow:
//! - **Upgrades**: Increase subscription amount (e.g., higher tier)
//! - **Downgrades**: Decrease subscription amount (e.g., lower tier)
//! - **Method Changes**: Switch payment method (e.g., Lightning to on-chain)
//! - **Billing Date Changes**: Modify when payments are due
//! - **Frequency Changes**: Change payment frequency
//!
//! # Example
//!
//! ```ignore
//! use paykit_subscriptions::modifications::{ModificationRequest, ModificationType};
//!
//! let request = ModificationRequest::upgrade(
//!     &subscription,
//!     Amount::from_sats(2000),
//!     chrono::Utc::now().timestamp(),
//! );
//!
//! let modified = request.apply(&subscription)?;
//! ```

use crate::{Amount, PaymentFrequency, Result, Subscription, SubscriptionError, SubscriptionTerms};
use paykit_lib::MethodId;
use serde::{Deserialize, Serialize};

/// Type of subscription modification.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ModificationType {
    /// Increase subscription amount (upgrade to higher tier).
    Upgrade {
        /// New amount (must be greater than current).
        new_amount: Amount,
        /// When the upgrade takes effect (unix timestamp).
        effective_date: i64,
    },
    /// Decrease subscription amount (downgrade to lower tier).
    Downgrade {
        /// New amount (must be less than current).
        new_amount: Amount,
        /// When the downgrade takes effect (unix timestamp).
        effective_date: i64,
    },
    /// Change the payment method.
    ChangeMethod {
        /// New payment method.
        new_method: MethodId,
    },
    /// Change the billing date.
    ChangeBillingDate {
        /// New day of month (1-28 for safety).
        new_day: u8,
    },
    /// Change the payment frequency.
    ChangeFrequency {
        /// New frequency.
        new_frequency: PaymentFrequency,
    },
    /// Cancel the subscription.
    Cancel {
        /// When cancellation takes effect.
        effective_date: i64,
        /// Reason for cancellation.
        reason: Option<String>,
    },
    /// Pause the subscription.
    Pause {
        /// Resume date (unix timestamp).
        resume_date: i64,
    },
    /// Resume a paused subscription.
    Resume,
}

impl ModificationType {
    /// Check if this modification requires proration.
    pub fn requires_proration(&self) -> bool {
        matches!(
            self,
            ModificationType::Upgrade { .. } | ModificationType::Downgrade { .. }
        )
    }

    /// Get the effective date if applicable.
    pub fn effective_date(&self) -> Option<i64> {
        match self {
            ModificationType::Upgrade { effective_date, .. } => Some(*effective_date),
            ModificationType::Downgrade { effective_date, .. } => Some(*effective_date),
            ModificationType::Cancel { effective_date, .. } => Some(*effective_date),
            ModificationType::Pause { resume_date } => Some(*resume_date),
            _ => None,
        }
    }

    /// Human-readable description.
    pub fn description(&self) -> String {
        match self {
            ModificationType::Upgrade { new_amount, .. } => {
                format!("Upgrade to {}", new_amount)
            }
            ModificationType::Downgrade { new_amount, .. } => {
                format!("Downgrade to {}", new_amount)
            }
            ModificationType::ChangeMethod { new_method } => {
                format!("Change method to {}", new_method.0)
            }
            ModificationType::ChangeBillingDate { new_day } => {
                format!("Change billing date to day {}", new_day)
            }
            ModificationType::ChangeFrequency { new_frequency } => {
                format!("Change frequency to {}", new_frequency.to_string())
            }
            ModificationType::Cancel { reason, .. } => {
                if let Some(r) = reason {
                    format!("Cancel: {}", r)
                } else {
                    "Cancel subscription".to_string()
                }
            }
            ModificationType::Pause { .. } => "Pause subscription".to_string(),
            ModificationType::Resume => "Resume subscription".to_string(),
        }
    }
}

/// A request to modify a subscription.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModificationRequest {
    /// Unique ID for this modification request.
    pub request_id: String,
    /// The subscription being modified.
    pub subscription_id: String,
    /// The type of modification.
    pub modification_type: ModificationType,
    /// When this request was created.
    pub created_at: i64,
    /// Who requested the modification.
    pub requested_by: RequestedBy,
    /// Optional note.
    pub note: Option<String>,
}

/// Who requested the modification.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RequestedBy {
    /// Subscriber requested the change.
    Subscriber,
    /// Provider requested the change.
    Provider,
    /// System-initiated (e.g., auto-downgrade on failed payment).
    System,
}

impl ModificationRequest {
    /// Create a new modification request.
    pub fn new(
        subscription_id: String,
        modification_type: ModificationType,
        requested_by: RequestedBy,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            request_id: format!("mod_{}_{}", subscription_id, now),
            subscription_id,
            modification_type,
            created_at: now,
            requested_by,
            note: None,
        }
    }

    /// Create an upgrade request.
    pub fn upgrade(
        subscription: &Subscription,
        new_amount: Amount,
        effective_date: i64,
    ) -> Self {
        Self::new(
            subscription.subscription_id.clone(),
            ModificationType::Upgrade {
                new_amount,
                effective_date,
            },
            RequestedBy::Subscriber,
        )
    }

    /// Create a downgrade request.
    pub fn downgrade(
        subscription: &Subscription,
        new_amount: Amount,
        effective_date: i64,
    ) -> Self {
        Self::new(
            subscription.subscription_id.clone(),
            ModificationType::Downgrade {
                new_amount,
                effective_date,
            },
            RequestedBy::Subscriber,
        )
    }

    /// Create a method change request.
    pub fn change_method(subscription: &Subscription, new_method: MethodId) -> Self {
        Self::new(
            subscription.subscription_id.clone(),
            ModificationType::ChangeMethod { new_method },
            RequestedBy::Subscriber,
        )
    }

    /// Create a billing date change request.
    pub fn change_billing_date(subscription: &Subscription, new_day: u8) -> Self {
        Self::new(
            subscription.subscription_id.clone(),
            ModificationType::ChangeBillingDate { new_day },
            RequestedBy::Subscriber,
        )
    }

    /// Create a cancellation request.
    pub fn cancel(subscription: &Subscription, effective_date: i64, reason: Option<String>) -> Self {
        Self::new(
            subscription.subscription_id.clone(),
            ModificationType::Cancel {
                effective_date,
                reason,
            },
            RequestedBy::Subscriber,
        )
    }

    /// Add a note to the request.
    pub fn with_note(mut self, note: String) -> Self {
        self.note = Some(note);
        self
    }

    /// Validate the modification request against a subscription.
    pub fn validate(&self, subscription: &Subscription) -> Result<()> {
        if self.subscription_id != subscription.subscription_id {
            return Err(SubscriptionError::InvalidArgument(
                "Subscription ID mismatch".to_string(),
            )
            .into());
        }

        match &self.modification_type {
            ModificationType::Upgrade { new_amount, .. } => {
                if new_amount <= &subscription.terms.amount {
                    return Err(SubscriptionError::InvalidArgument(
                        "Upgrade amount must be greater than current amount".to_string(),
                    )
                    .into());
                }
            }
            ModificationType::Downgrade { new_amount, .. } => {
                if new_amount >= &subscription.terms.amount {
                    return Err(SubscriptionError::InvalidArgument(
                        "Downgrade amount must be less than current amount".to_string(),
                    )
                    .into());
                }
            }
            ModificationType::ChangeBillingDate { new_day } => {
                if *new_day == 0 || *new_day > 28 {
                    return Err(SubscriptionError::InvalidArgument(
                        "Billing day must be between 1 and 28".to_string(),
                    )
                    .into());
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Apply the modification to create a new subscription.
    pub fn apply(&self, subscription: &Subscription) -> Result<Subscription> {
        self.validate(subscription)?;

        let mut modified = subscription.clone();
        let now = chrono::Utc::now().timestamp();

        match &self.modification_type {
            ModificationType::Upgrade { new_amount, effective_date } => {
                if now >= *effective_date {
                    modified.terms.amount = new_amount.clone();
                }
            }
            ModificationType::Downgrade { new_amount, effective_date } => {
                if now >= *effective_date {
                    modified.terms.amount = new_amount.clone();
                }
            }
            ModificationType::ChangeMethod { new_method } => {
                modified.terms.method = new_method.clone();
            }
            ModificationType::ChangeBillingDate { new_day } => {
                modified.terms.frequency = match &modified.terms.frequency {
                    PaymentFrequency::Monthly { .. } => {
                        PaymentFrequency::Monthly { day_of_month: *new_day }
                    }
                    other => other.clone(),
                };
            }
            ModificationType::ChangeFrequency { new_frequency } => {
                modified.terms.frequency = new_frequency.clone();
            }
            ModificationType::Cancel { effective_date, .. } => {
                modified.ends_at = Some(*effective_date);
            }
            ModificationType::Pause { resume_date } => {
                // Store pause info in metadata
                let mut meta = modified.metadata.clone();
                if let Some(obj) = meta.as_object_mut() {
                    obj.insert("paused_until".to_string(), serde_json::json!(resume_date));
                }
                modified.metadata = meta;
            }
            ModificationType::Resume => {
                // Remove pause info from metadata
                let mut meta = modified.metadata.clone();
                if let Some(obj) = meta.as_object_mut() {
                    obj.remove("paused_until");
                }
                modified.metadata = meta;
            }
        }

        Ok(modified)
    }
}

/// History of modifications to a subscription.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModificationHistory {
    /// Subscription ID.
    pub subscription_id: String,
    /// All modifications in chronological order.
    pub modifications: Vec<ModificationRecord>,
}

impl ModificationHistory {
    /// Create a new empty history.
    pub fn new(subscription_id: String) -> Self {
        Self {
            subscription_id,
            modifications: Vec::new(),
        }
    }

    /// Add a modification to the history.
    pub fn record(&mut self, request: &ModificationRequest, success: bool) {
        self.modifications.push(ModificationRecord {
            request_id: request.request_id.clone(),
            modification_type: request.modification_type.clone(),
            requested_by: request.requested_by.clone(),
            applied_at: chrono::Utc::now().timestamp(),
            success,
        });
    }

    /// Get the most recent modification.
    pub fn latest(&self) -> Option<&ModificationRecord> {
        self.modifications.last()
    }

    /// Get all successful modifications.
    pub fn successful(&self) -> Vec<&ModificationRecord> {
        self.modifications.iter().filter(|m| m.success).collect()
    }
}

/// Record of a single modification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModificationRecord {
    /// The modification request ID.
    pub request_id: String,
    /// The type of modification.
    pub modification_type: ModificationType,
    /// Who requested it.
    pub requested_by: RequestedBy,
    /// When it was applied.
    pub applied_at: i64,
    /// Whether it succeeded.
    pub success: bool,
}

/// Version tracking for subscription changes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscriptionVersion {
    /// Version number (increments with each modification).
    pub version: u32,
    /// When this version was created.
    pub created_at: i64,
    /// The terms at this version.
    pub terms: SubscriptionTerms,
    /// What modification created this version.
    pub modification_request_id: Option<String>,
}

impl SubscriptionVersion {
    /// Create the initial version.
    pub fn initial(terms: SubscriptionTerms) -> Self {
        Self {
            version: 1,
            created_at: chrono::Utc::now().timestamp(),
            terms,
            modification_request_id: None,
        }
    }

    /// Create a new version from a modification.
    pub fn from_modification(
        previous_version: u32,
        terms: SubscriptionTerms,
        request_id: String,
    ) -> Self {
        Self {
            version: previous_version + 1,
            created_at: chrono::Utc::now().timestamp(),
            terms,
            modification_request_id: Some(request_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn test_pubkey() -> paykit_lib::PublicKey {
        let keypair = pkarr::Keypair::random();
        paykit_lib::PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
    }

    fn test_subscription() -> Subscription {
        Subscription::new(
            test_pubkey(),
            test_pubkey(),
            SubscriptionTerms::new(
                Amount::from_sats(1000),
                "SAT".to_string(),
                PaymentFrequency::Monthly { day_of_month: 1 },
                MethodId("lightning".to_string()),
                "Test subscription".to_string(),
            ),
        )
    }

    #[test]
    fn test_modification_types() {
        let upgrade = ModificationType::Upgrade {
            new_amount: Amount::from_sats(2000),
            effective_date: 1000000,
        };
        assert!(upgrade.requires_proration());
        assert!(upgrade.effective_date().is_some());

        let change_method = ModificationType::ChangeMethod {
            new_method: MethodId("onchain".to_string()),
        };
        assert!(!change_method.requires_proration());
        assert!(change_method.effective_date().is_none());
    }

    #[test]
    fn test_upgrade_request() {
        let subscription = test_subscription();
        let request = ModificationRequest::upgrade(
            &subscription,
            Amount::from_sats(2000),
            chrono::Utc::now().timestamp(),
        );

        assert!(request.validate(&subscription).is_ok());

        // Invalid: upgrade to same amount
        let invalid_request = ModificationRequest::upgrade(
            &subscription,
            Amount::from_sats(1000),
            chrono::Utc::now().timestamp(),
        );
        assert!(invalid_request.validate(&subscription).is_err());
    }

    #[test]
    fn test_downgrade_request() {
        let subscription = test_subscription();
        let request = ModificationRequest::downgrade(
            &subscription,
            Amount::from_sats(500),
            chrono::Utc::now().timestamp(),
        );

        assert!(request.validate(&subscription).is_ok());

        // Invalid: downgrade to same amount
        let invalid_request = ModificationRequest::downgrade(
            &subscription,
            Amount::from_sats(1000),
            chrono::Utc::now().timestamp(),
        );
        assert!(invalid_request.validate(&subscription).is_err());
    }

    #[test]
    fn test_apply_upgrade() {
        let subscription = test_subscription();
        let now = chrono::Utc::now().timestamp();
        let request = ModificationRequest::upgrade(
            &subscription,
            Amount::from_sats(2000),
            now - 1, // Already effective
        );

        let modified = request.apply(&subscription).unwrap();
        assert_eq!(modified.terms.amount, Amount::from_sats(2000));
    }

    #[test]
    fn test_apply_change_method() {
        let subscription = test_subscription();
        let request =
            ModificationRequest::change_method(&subscription, MethodId("onchain".to_string()));

        let modified = request.apply(&subscription).unwrap();
        assert_eq!(modified.terms.method.0, "onchain");
    }

    #[test]
    fn test_apply_cancel() {
        let subscription = test_subscription();
        let cancel_date = chrono::Utc::now().timestamp() + 86400; // Tomorrow
        let request = ModificationRequest::cancel(
            &subscription,
            cancel_date,
            Some("Switching providers".to_string()),
        );

        let modified = request.apply(&subscription).unwrap();
        assert_eq!(modified.ends_at, Some(cancel_date));
    }

    #[test]
    fn test_change_billing_date() {
        let subscription = test_subscription();
        let request = ModificationRequest::change_billing_date(&subscription, 15);

        let modified = request.apply(&subscription).unwrap();
        match modified.terms.frequency {
            PaymentFrequency::Monthly { day_of_month } => {
                assert_eq!(day_of_month, 15);
            }
            _ => panic!("Expected monthly frequency"),
        }

        // Invalid: day 0
        let invalid_request = ModificationRequest::change_billing_date(&subscription, 0);
        assert!(invalid_request.validate(&subscription).is_err());

        // Invalid: day 29
        let invalid_request = ModificationRequest::change_billing_date(&subscription, 29);
        assert!(invalid_request.validate(&subscription).is_err());
    }

    #[test]
    fn test_modification_history() {
        let subscription = test_subscription();
        let mut history = ModificationHistory::new(subscription.subscription_id.clone());

        let request = ModificationRequest::upgrade(
            &subscription,
            Amount::from_sats(2000),
            chrono::Utc::now().timestamp(),
        );

        history.record(&request, true);
        assert_eq!(history.modifications.len(), 1);
        assert!(history.latest().unwrap().success);
        assert_eq!(history.successful().len(), 1);
    }

    #[test]
    fn test_subscription_version() {
        let terms = SubscriptionTerms::new(
            Amount::from_sats(1000),
            "SAT".to_string(),
            PaymentFrequency::Monthly { day_of_month: 1 },
            MethodId("lightning".to_string()),
            "Test".to_string(),
        );

        let v1 = SubscriptionVersion::initial(terms.clone());
        assert_eq!(v1.version, 1);
        assert!(v1.modification_request_id.is_none());

        let v2_terms = SubscriptionTerms {
            amount: Amount::from_sats(2000),
            ..terms
        };
        let v2 = SubscriptionVersion::from_modification(v1.version, v2_terms, "mod_123".to_string());
        assert_eq!(v2.version, 2);
        assert_eq!(v2.modification_request_id, Some("mod_123".to_string()));
    }
}
