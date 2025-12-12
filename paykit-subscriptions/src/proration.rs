//! Prorated Billing
//!
//! This module provides proration calculations for subscription modifications,
//! ensuring fair billing when upgrading or downgrading mid-cycle.
//!
//! # Overview
//!
//! Proration calculates:
//! - Credits for unused time on the old plan
//! - Charges for time on the new plan
//! - Net amount due (or refund)
//!
//! # Example
//!
//! ```ignore
//! use paykit_subscriptions::proration::{ProratedAmount, ProrationCalculator};
//!
//! let calculator = ProrationCalculator::new();
//! let result = calculator.calculate_upgrade(
//!     Amount::from_sats(1000), // old amount
//!     Amount::from_sats(2000), // new amount
//!     billing_cycle_start,
//!     billing_cycle_end,
//!     change_date,
//! );
//!
//! println!("Credit: {}, Charge: {}, Net: {}", result.credit, result.charge, result.net_amount);
//! ```

use crate::modifications::{ModificationRequest, ModificationType};
use crate::{Amount, Result, Subscription, SubscriptionError};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Result of a proration calculation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProratedAmount {
    /// Credit for unused time on old plan.
    pub credit: Amount,
    /// Charge for time on new plan.
    pub charge: Amount,
    /// Net amount (positive = charge, negative = refund).
    pub net_amount: Amount,
    /// Currency of the amounts.
    pub currency: String,
    /// Details of the calculation.
    pub details: ProrationDetails,
}

impl ProratedAmount {
    /// Check if this results in a refund (net negative).
    pub fn is_refund(&self) -> bool {
        self.net_amount.as_decimal() < Decimal::ZERO
    }

    /// Check if this results in a charge (net positive).
    pub fn is_charge(&self) -> bool {
        self.net_amount.as_decimal() > Decimal::ZERO
    }

    /// Check if amounts balance out (net zero).
    pub fn is_neutral(&self) -> bool {
        self.net_amount.as_decimal() == Decimal::ZERO
    }

    /// Get the absolute value of the net amount.
    pub fn net_absolute(&self) -> Amount {
        let decimal = self.net_amount.as_decimal().abs();
        Amount::new(decimal, self.currency.clone())
    }
}

/// Details of a proration calculation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProrationDetails {
    /// Old amount per period.
    pub old_amount: Amount,
    /// New amount per period.
    pub new_amount: Amount,
    /// Days in the billing period.
    pub total_days: u32,
    /// Days used at old rate.
    pub days_at_old_rate: u32,
    /// Days to be charged at new rate.
    pub days_at_new_rate: u32,
    /// Start of the billing period.
    pub period_start: i64,
    /// End of the billing period.
    pub period_end: i64,
    /// Date of the change.
    pub change_date: i64,
}

/// Calculator for prorated amounts.
#[derive(Clone, Debug)]
pub struct ProrationCalculator {
    /// Minimum proration threshold (don't prorate below this).
    pub min_proration_threshold: Amount,
    /// Whether to round to whole units.
    pub round_to_whole_units: bool,
    /// Rounding mode.
    pub rounding_mode: RoundingMode,
}

/// Rounding mode for proration calculations.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoundingMode {
    /// Round to nearest unit.
    Nearest,
    /// Always round up (favor provider).
    Up,
    /// Always round down (favor subscriber).
    Down,
}

impl Default for ProrationCalculator {
    fn default() -> Self {
        Self {
            min_proration_threshold: Amount::from_sats(100),
            round_to_whole_units: true,
            rounding_mode: RoundingMode::Nearest,
        }
    }
}

impl ProrationCalculator {
    /// Create a new calculator with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum proration threshold.
    pub fn with_threshold(mut self, threshold: Amount) -> Self {
        self.min_proration_threshold = threshold;
        self
    }

    /// Set rounding mode.
    pub fn with_rounding(mut self, mode: RoundingMode) -> Self {
        self.rounding_mode = mode;
        self
    }

    /// Calculate proration for an upgrade.
    pub fn calculate_upgrade(
        &self,
        old_amount: &Amount,
        new_amount: &Amount,
        period_start: i64,
        period_end: i64,
        change_date: i64,
        currency: &str,
    ) -> Result<ProratedAmount> {
        self.calculate(
            old_amount,
            new_amount,
            period_start,
            period_end,
            change_date,
            currency,
        )
    }

    /// Calculate proration for a downgrade.
    pub fn calculate_downgrade(
        &self,
        old_amount: &Amount,
        new_amount: &Amount,
        period_start: i64,
        period_end: i64,
        change_date: i64,
        currency: &str,
    ) -> Result<ProratedAmount> {
        self.calculate(
            old_amount,
            new_amount,
            period_start,
            period_end,
            change_date,
            currency,
        )
    }

    /// Calculate proration for any amount change.
    pub fn calculate(
        &self,
        old_amount: &Amount,
        new_amount: &Amount,
        period_start: i64,
        period_end: i64,
        change_date: i64,
        currency: &str,
    ) -> Result<ProratedAmount> {
        // Validate dates
        if change_date < period_start || change_date > period_end {
            return Err(SubscriptionError::InvalidArgument(
                "Change date must be within the billing period".to_string(),
            )
            .into());
        }

        if period_end <= period_start {
            return Err(SubscriptionError::InvalidArgument(
                "Period end must be after period start".to_string(),
            )
            .into());
        }

        // Calculate days
        let total_seconds = period_end - period_start;
        let total_days = (total_seconds / 86400) as u32;
        if total_days == 0 {
            return Err(SubscriptionError::InvalidArgument(
                "Billing period must be at least one day".to_string(),
            )
            .into());
        }

        let days_at_old_rate = ((change_date - period_start) / 86400) as u32;
        let days_at_new_rate = total_days - days_at_old_rate;

        // Calculate prorated amounts
        let old_decimal = old_amount.as_decimal();
        let new_decimal = new_amount.as_decimal();
        let total_days_decimal = Decimal::from(total_days);

        // Credit = (old_amount / total_days) * days_remaining
        let daily_old = old_decimal / total_days_decimal;
        let credit = daily_old * Decimal::from(days_at_new_rate);

        // Charge = (new_amount / total_days) * days_at_new_rate
        let daily_new = new_decimal / total_days_decimal;
        let charge = daily_new * Decimal::from(days_at_new_rate);

        // Net = charge - credit
        let net = charge - credit;

        // Apply rounding
        let (credit_rounded, charge_rounded, net_rounded) =
            self.apply_rounding(credit, charge, net);

        Ok(ProratedAmount {
            credit: Amount::new(credit_rounded, currency.to_string()),
            charge: Amount::new(charge_rounded, currency.to_string()),
            net_amount: Amount::new(net_rounded, currency.to_string()),
            currency: currency.to_string(),
            details: ProrationDetails {
                old_amount: old_amount.clone(),
                new_amount: new_amount.clone(),
                total_days,
                days_at_old_rate,
                days_at_new_rate,
                period_start,
                period_end,
                change_date,
            },
        })
    }

    /// Calculate proration from a modification request.
    pub fn calculate_from_modification(
        &self,
        subscription: &Subscription,
        request: &ModificationRequest,
        period_start: i64,
        period_end: i64,
    ) -> Result<Option<ProratedAmount>> {
        match &request.modification_type {
            ModificationType::Upgrade {
                new_amount,
                effective_date,
            } => Ok(Some(self.calculate(
                &subscription.terms.amount,
                new_amount,
                period_start,
                period_end,
                *effective_date,
                &subscription.terms.currency,
            )?)),
            ModificationType::Downgrade {
                new_amount,
                effective_date,
            } => Ok(Some(self.calculate(
                &subscription.terms.amount,
                new_amount,
                period_start,
                period_end,
                *effective_date,
                &subscription.terms.currency,
            )?)),
            // Other modification types don't require proration
            _ => Ok(None),
        }
    }

    /// Apply rounding based on configuration.
    fn apply_rounding(
        &self,
        credit: Decimal,
        charge: Decimal,
        net: Decimal,
    ) -> (Decimal, Decimal, Decimal) {
        if !self.round_to_whole_units {
            return (credit, charge, net);
        }

        let round_fn = match self.rounding_mode {
            RoundingMode::Nearest => |d: Decimal| d.round(),
            RoundingMode::Up => |d: Decimal| d.ceil(),
            RoundingMode::Down => |d: Decimal| d.floor(),
        };

        (round_fn(credit), round_fn(charge), round_fn(net))
    }
}

/// Calculate the current billing period for a subscription.
pub fn current_billing_period(subscription: &Subscription) -> (i64, i64) {
    let now = chrono::Utc::now().timestamp();
    let interval_seconds = subscription.terms.frequency.to_seconds() as i64;

    // Calculate how many periods have passed since start
    let elapsed = now - subscription.starts_at;
    let periods_passed = elapsed / interval_seconds;

    let period_start = subscription.starts_at + (periods_passed * interval_seconds);
    let period_end = period_start + interval_seconds;

    (period_start, period_end)
}

/// Calculate the next billing date for a subscription.
pub fn next_billing_date(subscription: &Subscription) -> i64 {
    let (_, period_end) = current_billing_period(subscription);
    period_end
}

/// Calculate days remaining in the current billing period.
pub fn days_remaining_in_period(subscription: &Subscription) -> u32 {
    let now = chrono::Utc::now().timestamp();
    let (_, period_end) = current_billing_period(subscription);
    let remaining_seconds = period_end - now;
    if remaining_seconds <= 0 {
        return 0;
    }
    (remaining_seconds / 86400) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PaymentFrequency;
    use std::str::FromStr;

    fn test_pubkey() -> paykit_lib::PublicKey {
        let keypair = pkarr::Keypair::random();
        paykit_lib::PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
    }

    fn test_subscription() -> Subscription {
        use crate::SubscriptionTerms;
        use paykit_lib::MethodId;

        Subscription::new(
            test_pubkey(),
            test_pubkey(),
            SubscriptionTerms::new(
                Amount::from_sats(3000), // 3000 sats/month
                "SAT".to_string(),
                PaymentFrequency::Monthly { day_of_month: 1 },
                MethodId("lightning".to_string()),
                "Test subscription".to_string(),
            ),
        )
    }

    #[test]
    fn test_upgrade_proration() {
        let calc = ProrationCalculator::new();

        // 30-day month, upgrade on day 10
        let period_start = 0;
        let period_end = 30 * 86400;
        let change_date = 10 * 86400;

        let result = calc
            .calculate_upgrade(
                &Amount::from_sats(3000),
                &Amount::from_sats(6000),
                period_start,
                period_end,
                change_date,
                "SAT",
            )
            .unwrap();

        // 20 days remaining
        // Credit: 3000/30 * 20 = 2000
        // Charge: 6000/30 * 20 = 4000
        // Net: 4000 - 2000 = 2000
        assert_eq!(result.details.days_at_old_rate, 10);
        assert_eq!(result.details.days_at_new_rate, 20);
        assert!(result.is_charge());
        assert_eq!(result.net_amount, Amount::from_sats(2000));
    }

    #[test]
    fn test_downgrade_proration() {
        let calc = ProrationCalculator::new();

        // 30-day month, downgrade on day 15
        let period_start = 0;
        let period_end = 30 * 86400;
        let change_date = 15 * 86400;

        let result = calc
            .calculate_downgrade(
                &Amount::from_sats(6000),
                &Amount::from_sats(3000),
                period_start,
                period_end,
                change_date,
                "SAT",
            )
            .unwrap();

        // 15 days remaining
        // Credit: 6000/30 * 15 = 3000
        // Charge: 3000/30 * 15 = 1500
        // Net: 1500 - 3000 = -1500 (refund)
        assert!(result.is_refund());
        assert_eq!(result.net_amount.as_decimal().abs(), Decimal::from(1500));
    }

    #[test]
    fn test_proration_at_period_start() {
        let calc = ProrationCalculator::new();

        let period_start = 0;
        let period_end = 30 * 86400;
        let change_date = 0; // Start of period

        let result = calc
            .calculate(
                &Amount::from_sats(3000),
                &Amount::from_sats(6000),
                period_start,
                period_end,
                change_date,
                "SAT",
            )
            .unwrap();

        // Full period at new rate
        // Credit: 3000/30 * 30 = 3000
        // Charge: 6000/30 * 30 = 6000
        // Net: 6000 - 3000 = 3000
        assert_eq!(result.details.days_at_old_rate, 0);
        assert_eq!(result.details.days_at_new_rate, 30);
        assert_eq!(result.net_amount, Amount::from_sats(3000));
    }

    #[test]
    fn test_proration_at_period_end() {
        let calc = ProrationCalculator::new();

        let period_start = 0;
        let period_end = 30 * 86400;
        let change_date = 30 * 86400; // End of period

        let result = calc
            .calculate(
                &Amount::from_sats(3000),
                &Amount::from_sats(6000),
                period_start,
                period_end,
                change_date,
                "SAT",
            )
            .unwrap();

        // Full period at old rate
        assert_eq!(result.details.days_at_old_rate, 30);
        assert_eq!(result.details.days_at_new_rate, 0);
        // Credit and charge are both 0 for new rate
        assert!(result.is_neutral());
    }

    #[test]
    fn test_invalid_dates() {
        let calc = ProrationCalculator::new();

        // Change date before period
        let result = calc.calculate(
            &Amount::from_sats(3000),
            &Amount::from_sats(6000),
            100,
            200,
            50, // Before period start
            "SAT",
        );
        assert!(result.is_err());

        // Change date after period
        let result = calc.calculate(
            &Amount::from_sats(3000),
            &Amount::from_sats(6000),
            100,
            200,
            250, // After period end
            "SAT",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_rounding_modes() {
        // Test with amounts that don't divide evenly
        let period_start = 0;
        let period_end = 7 * 86400; // 7 days
        let change_date = 3 * 86400; // Day 3

        let calc_up = ProrationCalculator::new().with_rounding(RoundingMode::Up);
        let calc_down = ProrationCalculator::new().with_rounding(RoundingMode::Down);

        // 1000/7 = 142.857... per day
        // 4 days at new rate
        // New: 2000/7 * 4 = 1142.857...
        // Old: 1000/7 * 4 = 571.428...

        let result_up = calc_up
            .calculate(
                &Amount::from_sats(1000),
                &Amount::from_sats(2000),
                period_start,
                period_end,
                change_date,
                "SAT",
            )
            .unwrap();

        let result_down = calc_down
            .calculate(
                &Amount::from_sats(1000),
                &Amount::from_sats(2000),
                period_start,
                period_end,
                change_date,
                "SAT",
            )
            .unwrap();

        // Up should round charges up
        // Down should round charges down
        assert!(result_up.charge.as_decimal() >= result_down.charge.as_decimal());
    }

    #[test]
    fn test_calculate_from_modification() {
        let calc = ProrationCalculator::new();
        let subscription = test_subscription();

        let now = chrono::Utc::now().timestamp();
        let period_start = now - (15 * 86400); // Started 15 days ago
        let period_end = period_start + (30 * 86400);

        let request = ModificationRequest::upgrade(&subscription, Amount::from_sats(6000), now);

        let result = calc
            .calculate_from_modification(&subscription, &request, period_start, period_end)
            .unwrap();

        assert!(result.is_some());
        let proration = result.unwrap();
        assert!(proration.is_charge());
    }

    #[test]
    fn test_no_proration_for_method_change() {
        let calc = ProrationCalculator::new();
        let subscription = test_subscription();

        let now = chrono::Utc::now().timestamp();
        let period_start = now - (15 * 86400);
        let period_end = period_start + (30 * 86400);

        let request = ModificationRequest::change_method(
            &subscription,
            paykit_lib::MethodId("onchain".to_string()),
        );

        let result = calc
            .calculate_from_modification(&subscription, &request, period_start, period_end)
            .unwrap();

        // Method changes don't require proration
        assert!(result.is_none());
    }

    #[test]
    fn test_current_billing_period() {
        let mut subscription = test_subscription();
        subscription.starts_at = chrono::Utc::now().timestamp() - (45 * 86400); // 45 days ago
                                                                                // With monthly frequency (~30 days), we should be in the second period

        let (start, end) = current_billing_period(&subscription);
        assert!(start < end);
        assert!(start >= subscription.starts_at);
    }

    #[test]
    fn test_days_remaining() {
        let mut subscription = test_subscription();
        // Start at beginning of current period
        let interval = subscription.terms.frequency.to_seconds() as i64;
        subscription.starts_at = chrono::Utc::now().timestamp() - (interval / 2);

        let remaining = days_remaining_in_period(&subscription);
        // Should be roughly half the period
        assert!(remaining > 0);
        assert!(remaining < 30);
    }
}
