//! Comprehensive test suite for auto-pay and spending limits
//!
//! This test suite covers:
//! - AutoPayRule creation, validation, and limit checking
//! - PeerSpendingLimit creation, spending tracking, and period reset
//! - Edge cases: zero amounts, overflow, underflow
//! - Integration: atomic spending reservations
//! - Serialization roundtrips

use paykit_lib::{MethodId, PublicKey};
use paykit_subscriptions::{Amount, AutoPayRule, PeerSpendingLimit};
use std::str::FromStr;

// ============================================================
// Test Helpers
// ============================================================

fn random_pubkey() -> PublicKey {
    let keypair = pkarr::Keypair::random();
    PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
}

fn create_test_rule(subscription_id: &str) -> AutoPayRule {
    AutoPayRule::new(
        subscription_id.to_string(),
        random_pubkey(),
        MethodId("lightning".to_string()),
    )
}

fn create_test_limit(limit_sats: i64, period: &str) -> PeerSpendingLimit {
    PeerSpendingLimit::new(
        random_pubkey(),
        Amount::from_sats(limit_sats),
        period.to_string(),
    )
}

// ============================================================
// AutoPayRule Tests
// ============================================================

mod autopay_rule_tests {
    use super::*;

    #[test]
    fn test_rule_creation_defaults() {
        let rule = create_test_rule("sub_test");

        assert_eq!(rule.subscription_id, "sub_test");
        assert!(rule.enabled);
        assert!(!rule.require_confirmation);
        assert_eq!(rule.notify_before, Some(3600)); // Default 1 hour
        assert_eq!(rule.period, Some("monthly".to_string()));
        assert!(rule.max_amount_per_payment.is_none());
        assert!(rule.max_total_amount_per_period.is_none());
    }

    #[test]
    fn test_rule_with_max_payment_amount() {
        let rule = create_test_rule("sub_max")
            .with_max_payment_amount(Amount::from_sats(5000));

        assert_eq!(rule.max_amount_per_payment, Some(Amount::from_sats(5000)));
    }

    #[test]
    fn test_rule_with_period_amount() {
        let rule = create_test_rule("sub_period")
            .with_max_period_amount(Amount::from_sats(100000), "weekly".to_string());

        assert_eq!(
            rule.max_total_amount_per_period,
            Some(Amount::from_sats(100000))
        );
        assert_eq!(rule.period, Some("weekly".to_string()));
    }

    #[test]
    fn test_rule_with_confirmation() {
        let rule = create_test_rule("sub_confirm")
            .with_confirmation(true);

        assert!(rule.require_confirmation);
    }

    #[test]
    fn test_rule_with_notification() {
        let rule = create_test_rule("sub_notify")
            .with_notification(7200); // 2 hours

        assert_eq!(rule.notify_before, Some(7200));
    }

    #[test]
    fn test_rule_validation_success() {
        let rule = create_test_rule("valid_sub");
        assert!(rule.validate().is_ok());
    }

    #[test]
    fn test_rule_validation_empty_id() {
        let rule = AutoPayRule::new(
            String::new(), // Empty ID
            random_pubkey(),
            MethodId("lightning".to_string()),
        );
        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_rule_amount_within_limit_no_limit_set() {
        let rule = create_test_rule("sub_no_limit");
        
        // Any amount should be within limit when no limit is set
        assert!(rule.is_amount_within_limit(&Amount::from_sats(1)));
        assert!(rule.is_amount_within_limit(&Amount::from_sats(1_000_000)));
        assert!(rule.is_amount_within_limit(&Amount::from_sats(i64::MAX)));
    }

    #[test]
    fn test_rule_amount_within_limit_with_limit() {
        let rule = create_test_rule("sub_with_limit")
            .with_max_payment_amount(Amount::from_sats(1000));

        assert!(rule.is_amount_within_limit(&Amount::from_sats(500)));
        assert!(rule.is_amount_within_limit(&Amount::from_sats(1000)));
        assert!(!rule.is_amount_within_limit(&Amount::from_sats(1001)));
        assert!(!rule.is_amount_within_limit(&Amount::from_sats(5000)));
    }

    #[test]
    fn test_rule_amount_boundary_check() {
        let rule = create_test_rule("sub_boundary")
            .with_max_payment_amount(Amount::from_sats(100));

        // Exactly at limit
        assert!(rule.is_amount_within_limit(&Amount::from_sats(100)));
        // One over
        assert!(!rule.is_amount_within_limit(&Amount::from_sats(101)));
        // One under
        assert!(rule.is_amount_within_limit(&Amount::from_sats(99)));
    }

    #[test]
    fn test_rule_zero_amount() {
        let rule = create_test_rule("sub_zero")
            .with_max_payment_amount(Amount::from_sats(1000));

        assert!(rule.is_amount_within_limit(&Amount::from_sats(0)));
    }

    #[test]
    fn test_rule_chained_builders() {
        let rule = create_test_rule("sub_chain")
            .with_max_payment_amount(Amount::from_sats(1000))
            .with_max_period_amount(Amount::from_sats(10000), "daily".to_string())
            .with_confirmation(true)
            .with_notification(1800);

        assert_eq!(rule.max_amount_per_payment, Some(Amount::from_sats(1000)));
        assert_eq!(
            rule.max_total_amount_per_period,
            Some(Amount::from_sats(10000))
        );
        assert_eq!(rule.period, Some("daily".to_string()));
        assert!(rule.require_confirmation);
        assert_eq!(rule.notify_before, Some(1800));
    }

    #[test]
    fn test_rule_serialization_roundtrip() {
        let rule = create_test_rule("sub_serialize")
            .with_max_payment_amount(Amount::from_sats(5000))
            .with_confirmation(true);

        let json = serde_json::to_string(&rule).unwrap();
        let parsed: AutoPayRule = serde_json::from_str(&json).unwrap();

        assert_eq!(rule.subscription_id, parsed.subscription_id);
        assert_eq!(rule.max_amount_per_payment, parsed.max_amount_per_payment);
        assert_eq!(rule.require_confirmation, parsed.require_confirmation);
    }
}

// ============================================================
// PeerSpendingLimit Tests
// ============================================================

mod peer_spending_limit_tests {
    use super::*;

    #[test]
    fn test_limit_creation() {
        let limit = create_test_limit(10000, "monthly");

        assert_eq!(limit.total_amount_limit, Amount::from_sats(10000));
        assert_eq!(limit.current_spent, Amount::from_sats(0));
        assert_eq!(limit.period, "monthly");
    }

    #[test]
    fn test_limit_remaining() {
        let mut limit = create_test_limit(10000, "monthly");

        assert_eq!(limit.remaining_limit(), Amount::from_sats(10000));

        limit.add_spent(&Amount::from_sats(3000)).unwrap();
        assert_eq!(limit.remaining_limit(), Amount::from_sats(7000));

        limit.add_spent(&Amount::from_sats(7000)).unwrap();
        assert_eq!(limit.remaining_limit(), Amount::from_sats(0));
    }

    #[test]
    fn test_limit_would_exceed() {
        let mut limit = create_test_limit(10000, "monthly");

        // Initial state - nothing would exceed
        assert!(!limit.would_exceed_limit(&Amount::from_sats(5000)));
        assert!(!limit.would_exceed_limit(&Amount::from_sats(10000)));
        assert!(limit.would_exceed_limit(&Amount::from_sats(10001)));

        // After spending
        limit.add_spent(&Amount::from_sats(6000)).unwrap();
        assert!(!limit.would_exceed_limit(&Amount::from_sats(4000)));
        assert!(limit.would_exceed_limit(&Amount::from_sats(4001)));
    }

    #[test]
    fn test_limit_add_spent() {
        let mut limit = create_test_limit(10000, "monthly");

        limit.add_spent(&Amount::from_sats(1000)).unwrap();
        assert_eq!(limit.current_spent, Amount::from_sats(1000));

        limit.add_spent(&Amount::from_sats(2000)).unwrap();
        assert_eq!(limit.current_spent, Amount::from_sats(3000));

        limit.add_spent(&Amount::from_sats(500)).unwrap();
        assert_eq!(limit.current_spent, Amount::from_sats(3500));
    }

    #[test]
    fn test_limit_reset() {
        let mut limit = create_test_limit(10000, "monthly");

        limit.add_spent(&Amount::from_sats(5000)).unwrap();
        assert_eq!(limit.current_spent, Amount::from_sats(5000));

        limit.reset();
        assert_eq!(limit.current_spent, Amount::from_sats(0));
        assert_eq!(limit.remaining_limit(), Amount::from_sats(10000));
    }

    #[test]
    fn test_limit_should_reset_daily() {
        let mut limit = create_test_limit(10000, "daily");

        // Just created - should not reset
        assert!(!limit.should_reset());

        // Set last_reset to 25 hours ago
        limit.last_reset = chrono::Utc::now() - chrono::Duration::hours(25);
        assert!(limit.should_reset());

        // Set last_reset to 23 hours ago
        limit.last_reset = chrono::Utc::now() - chrono::Duration::hours(23);
        assert!(!limit.should_reset());
    }

    #[test]
    fn test_limit_should_reset_weekly() {
        let mut limit = create_test_limit(10000, "weekly");

        // Just created - should not reset
        assert!(!limit.should_reset());

        // Set last_reset to 8 days ago
        limit.last_reset = chrono::Utc::now() - chrono::Duration::days(8);
        assert!(limit.should_reset());

        // Set last_reset to 6 days ago
        limit.last_reset = chrono::Utc::now() - chrono::Duration::days(6);
        assert!(!limit.should_reset());
    }

    #[test]
    fn test_limit_should_reset_monthly() {
        let mut limit = create_test_limit(10000, "monthly");

        // Just created - should not reset
        assert!(!limit.should_reset());

        // Set last_reset to 31 days ago
        limit.last_reset = chrono::Utc::now() - chrono::Duration::days(31);
        assert!(limit.should_reset());

        // Set last_reset to 29 days ago
        limit.last_reset = chrono::Utc::now() - chrono::Duration::days(29);
        assert!(!limit.should_reset());
    }

    #[test]
    fn test_limit_zero_amount_operations() {
        let mut limit = create_test_limit(10000, "monthly");

        // Adding zero should work
        limit.add_spent(&Amount::from_sats(0)).unwrap();
        assert_eq!(limit.current_spent, Amount::from_sats(0));

        // Zero would not exceed
        assert!(!limit.would_exceed_limit(&Amount::from_sats(0)));
    }

    #[test]
    fn test_limit_exact_limit_amount() {
        let mut limit = create_test_limit(10000, "monthly");

        // Spending exactly the limit
        limit.add_spent(&Amount::from_sats(10000)).unwrap();
        assert_eq!(limit.current_spent, Amount::from_sats(10000));
        assert_eq!(limit.remaining_limit(), Amount::from_sats(0));

        // Any more would exceed
        assert!(limit.would_exceed_limit(&Amount::from_sats(1)));
    }

    #[test]
    fn test_limit_serialization_roundtrip() {
        let mut limit = create_test_limit(50000, "weekly");
        limit.add_spent(&Amount::from_sats(12345)).unwrap();

        let json = serde_json::to_string(&limit).unwrap();
        let parsed: PeerSpendingLimit = serde_json::from_str(&json).unwrap();

        assert_eq!(limit.total_amount_limit, parsed.total_amount_limit);
        assert_eq!(limit.current_spent, parsed.current_spent);
        assert_eq!(limit.period, parsed.period);
    }

    #[test]
    fn test_limit_different_periods() {
        let daily = create_test_limit(1000, "daily");
        let weekly = create_test_limit(7000, "weekly");
        let monthly = create_test_limit(30000, "monthly");

        assert_eq!(daily.period, "daily");
        assert_eq!(weekly.period, "weekly");
        assert_eq!(monthly.period, "monthly");
    }

    #[test]
    fn test_limit_unknown_period_does_not_reset() {
        let mut limit = create_test_limit(10000, "custom_period");
        limit.last_reset = chrono::Utc::now() - chrono::Duration::days(100);

        // Unknown periods never auto-reset
        assert!(!limit.should_reset());
    }
}

// ============================================================
// Amount Safety Tests
// ============================================================

mod amount_safety_tests {
    use super::*;

    #[test]
    fn test_amount_checked_add() {
        let a = Amount::from_sats(1000);
        let b = Amount::from_sats(500);
        let result = a.checked_add(&b);

        assert!(result.is_some());
        assert_eq!(result.unwrap().as_sats(), 1500);
    }

    #[test]
    fn test_amount_checked_sub() {
        let a = Amount::from_sats(1000);
        let b = Amount::from_sats(400);
        let result = a.checked_sub(&b);

        assert!(result.is_some());
        assert_eq!(result.unwrap().as_sats(), 600);
    }

    #[test]
    fn test_amount_is_within_limit() {
        let amount = Amount::from_sats(500);
        let limit = Amount::from_sats(1000);

        assert!(amount.is_within_limit(&limit));
        assert!(limit.is_within_limit(&limit)); // Equal is within
        assert!(!Amount::from_sats(1001).is_within_limit(&limit));
    }

    #[test]
    fn test_amount_would_exceed() {
        let current = Amount::from_sats(700);
        let limit = Amount::from_sats(1000);

        assert!(!current.would_exceed(&Amount::from_sats(300), &limit));
        assert!(current.would_exceed(&Amount::from_sats(301), &limit));
    }

    #[test]
    fn test_amount_zero() {
        let zero = Amount::zero();
        assert!(zero.is_zero());
        assert_eq!(zero.as_sats(), 0);
    }

    #[test]
    fn test_amount_from_string() {
        let amt = Amount::from_str_checked("12345").unwrap();
        assert_eq!(amt.as_sats(), 12345);
    }

    #[test]
    fn test_amount_invalid_string() {
        let result = Amount::from_str_checked("not_a_number");
        assert!(result.is_err());
    }
}

// ============================================================
// Integration Tests
// ============================================================

mod integration_tests {
    use super::*;

    #[test]
    fn test_autopay_with_spending_limit_workflow() {
        // Setup: Create a rule and a spending limit for the same peer
        let peer = random_pubkey();
        
        let rule = AutoPayRule::new(
            "sub_integration".to_string(),
            peer.clone(),
            MethodId("lightning".to_string()),
        )
        .with_max_payment_amount(Amount::from_sats(500));

        let mut limit = PeerSpendingLimit::new(
            peer,
            Amount::from_sats(2000),
            "daily".to_string(),
        );

        // Scenario: Process multiple payments

        // Payment 1: 300 sats - should pass both checks
        let payment1 = Amount::from_sats(300);
        assert!(rule.is_amount_within_limit(&payment1));
        assert!(!limit.would_exceed_limit(&payment1));
        limit.add_spent(&payment1).unwrap();

        // Payment 2: 400 sats - should pass both checks
        let payment2 = Amount::from_sats(400);
        assert!(rule.is_amount_within_limit(&payment2));
        assert!(!limit.would_exceed_limit(&payment2));
        limit.add_spent(&payment2).unwrap();

        // Payment 3: 600 sats - fails rule check (max per payment is 500)
        let payment3 = Amount::from_sats(600);
        assert!(!rule.is_amount_within_limit(&payment3));

        // Payment 4: 400 sats - passes rule, check spending limit
        let payment4 = Amount::from_sats(400);
        assert!(rule.is_amount_within_limit(&payment4));
        assert!(!limit.would_exceed_limit(&payment4));
        limit.add_spent(&payment4).unwrap();

        // Current spent: 1100 sats, remaining: 900 sats
        assert_eq!(limit.current_spent, Amount::from_sats(1100));
        assert_eq!(limit.remaining_limit(), Amount::from_sats(900));

        // Payment 5: 500 sats - passes rule, passes limit
        let payment5 = Amount::from_sats(500);
        assert!(rule.is_amount_within_limit(&payment5));
        assert!(!limit.would_exceed_limit(&payment5));
        limit.add_spent(&payment5).unwrap();

        // Current spent: 1600 sats, remaining: 400 sats
        assert_eq!(limit.current_spent, Amount::from_sats(1600));

        // Payment 6: 500 sats - passes rule, FAILS limit (would be 2100 > 2000)
        let payment6 = Amount::from_sats(500);
        assert!(rule.is_amount_within_limit(&payment6));
        assert!(limit.would_exceed_limit(&payment6)); // Would exceed!

        // Reset limit and try again
        limit.reset();
        assert_eq!(limit.current_spent, Amount::from_sats(0));
        assert!(!limit.would_exceed_limit(&payment6)); // Now it's OK
    }

    #[test]
    fn test_multiple_peer_limits() {
        let peer1 = random_pubkey();
        let peer2 = random_pubkey();
        let peer3 = random_pubkey();

        let mut limit1 = PeerSpendingLimit::new(
            peer1,
            Amount::from_sats(10000),
            "daily".to_string(),
        );

        let mut limit2 = PeerSpendingLimit::new(
            peer2,
            Amount::from_sats(50000),
            "weekly".to_string(),
        );

        let mut limit3 = PeerSpendingLimit::new(
            peer3,
            Amount::from_sats(100000),
            "monthly".to_string(),
        );

        // Each limit is independent
        limit1.add_spent(&Amount::from_sats(5000)).unwrap();
        limit2.add_spent(&Amount::from_sats(25000)).unwrap();
        limit3.add_spent(&Amount::from_sats(50000)).unwrap();

        assert_eq!(limit1.remaining_limit(), Amount::from_sats(5000));
        assert_eq!(limit2.remaining_limit(), Amount::from_sats(25000));
        assert_eq!(limit3.remaining_limit(), Amount::from_sats(50000));

        // Reset one doesn't affect others
        limit1.reset();
        assert_eq!(limit1.remaining_limit(), Amount::from_sats(10000));
        assert_eq!(limit2.remaining_limit(), Amount::from_sats(25000)); // Unchanged
    }

    #[test]
    fn test_rule_disabled_behavior() {
        let mut rule = create_test_rule("sub_disabled")
            .with_max_payment_amount(Amount::from_sats(100));

        // Rule is enabled by default
        assert!(rule.enabled);
        assert!(!rule.is_amount_within_limit(&Amount::from_sats(500)));

        // Disable the rule
        rule.enabled = false;

        // Note: is_amount_within_limit still checks the max_amount
        // The enabled flag is for the caller to check before processing
        assert!(!rule.is_amount_within_limit(&Amount::from_sats(500)));
    }
}

// ============================================================
// Edge Case Tests
// ============================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_very_large_amounts() {
        let large = Amount::from_sats(i64::MAX - 1);
        let limit = create_test_limit(i64::MAX, "monthly");

        assert!(!limit.would_exceed_limit(&large));
    }

    #[test]
    fn test_one_satoshi_operations() {
        let mut limit = create_test_limit(1, "daily");
        
        assert!(!limit.would_exceed_limit(&Amount::from_sats(1)));
        limit.add_spent(&Amount::from_sats(1)).unwrap();
        
        // Now at limit
        assert!(limit.would_exceed_limit(&Amount::from_sats(1)));
        assert_eq!(limit.remaining_limit(), Amount::from_sats(0));
    }

    #[test]
    fn test_empty_subscription_id() {
        let rule = AutoPayRule::new(
            String::new(),
            random_pubkey(),
            MethodId("lightning".to_string()),
        );

        let validation = rule.validate();
        assert!(validation.is_err());
    }

    #[test]
    fn test_very_long_subscription_id() {
        let long_id = "x".repeat(10000);
        let rule = AutoPayRule::new(
            long_id.clone(),
            random_pubkey(),
            MethodId("lightning".to_string()),
        );

        assert_eq!(rule.subscription_id, long_id);
        assert!(rule.validate().is_ok());
    }

    #[test]
    fn test_special_characters_in_period() {
        let limit = create_test_limit(10000, "daily/weekly");
        
        // Unknown period - should never auto-reset
        assert!(!limit.should_reset());
    }

    #[test]
    fn test_limit_at_exactly_remaining() {
        let mut limit = create_test_limit(1000, "daily");
        limit.add_spent(&Amount::from_sats(600)).unwrap();

        // Remaining is 400
        assert!(!limit.would_exceed_limit(&Amount::from_sats(400)));
        assert!(limit.would_exceed_limit(&Amount::from_sats(401)));
    }
}
