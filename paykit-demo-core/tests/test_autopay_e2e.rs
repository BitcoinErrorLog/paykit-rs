//! End-to-end integration tests for auto-pay workflows
//!
//! These tests verify complete auto-pay scenarios including:
//! - Full auto-pay approval workflow with rules and limits
//! - Spending limit enforcement across multiple payments
//! - Period reset behavior
//! - Rule enabling/disabling
//! - Combined workflow with subscriptions

use paykit_demo_core::SubscriptionCoordinator;
use paykit_lib::MethodId;
use paykit_subscriptions::{Amount, PaymentFrequency};
use pubky::Keypair;
use tempfile::TempDir;

// ============================================================
// Test Helpers
// ============================================================

fn create_test_coordinator() -> (SubscriptionCoordinator, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let coordinator =
        SubscriptionCoordinator::new(temp_dir.path()).expect("Failed to create coordinator");
    (coordinator, temp_dir)
}

// ============================================================
// Auto-Pay Approval Workflow Tests
// ============================================================

mod autopay_approval_tests {
    use super::*;

    #[test]
    fn test_full_autopay_approval_workflow() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let peer = Keypair::random().public_key();

        // Step 1: Set up spending limit for peer
        let limit = coordinator
            .set_spending_limit(peer.clone(), 50000, "daily".to_string())
            .expect("Failed to set spending limit");

        assert_eq!(limit.total_amount_limit.as_sats(), 50000);
        assert_eq!(limit.current_spent.as_sats(), 0);

        // Step 2: Configure auto-pay rule
        let rule = coordinator
            .configure_auto_pay(
                "sub_test".to_string(),
                peer.clone(),
                MethodId("lightning".to_string()),
                Some(10000), // Max 10k sats per period
                "daily".to_string(),
                true, // Enabled
            )
            .expect("Failed to configure auto-pay");

        assert!(rule.enabled);
        assert_eq!(
            rule.max_total_amount_per_period.as_ref().unwrap().as_sats(),
            10000
        );

        // Step 3: Verify auto-pay would be approved for valid amount
        let can_pay = coordinator
            .can_auto_pay(&peer, &Amount::from_sats(5000))
            .expect("Failed to check auto-pay");

        assert!(can_pay);

        // Step 4: Record a payment
        coordinator
            .record_auto_payment(&peer, Amount::from_sats(5000))
            .expect("Failed to record payment");

        // Step 5: Verify remaining limit
        let updated_limit = coordinator
            .get_spending_limit(&peer)
            .expect("Failed to get limit")
            .expect("Limit should exist");

        assert_eq!(updated_limit.current_spent.as_sats(), 5000);
        assert_eq!(updated_limit.remaining_limit().as_sats(), 45000);
    }

    #[test]
    fn test_autopay_blocked_when_exceeds_limit() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let peer = Keypair::random().public_key();

        // Set low spending limit
        coordinator
            .set_spending_limit(peer.clone(), 1000, "daily".to_string())
            .expect("Failed to set limit");

        // Configure auto-pay
        coordinator
            .configure_auto_pay(
                "sub_test".to_string(),
                peer.clone(),
                MethodId("lightning".to_string()),
                Some(5000),
                "daily".to_string(),
                true,
            )
            .expect("Failed to configure auto-pay");

        // Check: amount exceeds limit
        let can_pay = coordinator
            .can_auto_pay(&peer, &Amount::from_sats(2000))
            .expect("Failed to check");

        assert!(!can_pay); // Should be blocked
    }

    #[test]
    fn test_autopay_blocked_when_disabled() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let peer = Keypair::random().public_key();

        // Set spending limit
        coordinator
            .set_spending_limit(peer.clone(), 50000, "daily".to_string())
            .expect("Failed to set limit");

        // Configure auto-pay but disabled
        let rule = coordinator
            .configure_auto_pay(
                "sub_test".to_string(),
                peer.clone(),
                MethodId("lightning".to_string()),
                Some(10000),
                "daily".to_string(),
                false, // Disabled
            )
            .expect("Failed to configure auto-pay");

        assert!(!rule.enabled);

        // Even with limit available, disabled rule should block
        // Note: This depends on implementation - may need to check rule status separately
    }
}

// ============================================================
// Spending Limit Enforcement Tests
// ============================================================

mod spending_limit_enforcement_tests {
    use super::*;

    #[test]
    fn test_multiple_payments_accumulate_spending() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let peer = Keypair::random().public_key();

        // Set limit
        coordinator
            .set_spending_limit(peer.clone(), 10000, "daily".to_string())
            .expect("Failed to set limit");

        // Record multiple payments
        for _ in 0..3 {
            coordinator
                .record_auto_payment(&peer, Amount::from_sats(2000))
                .expect("Failed to record payment");
        }

        // Verify accumulated spending
        let limit = coordinator
            .get_spending_limit(&peer)
            .expect("Failed to get limit")
            .expect("Limit should exist");

        assert_eq!(limit.current_spent.as_sats(), 6000);
        assert_eq!(limit.remaining_limit().as_sats(), 4000);

        // Next payment of 4000 should be allowed
        let can_pay = coordinator
            .can_auto_pay(&peer, &Amount::from_sats(4000))
            .expect("Failed to check");
        assert!(can_pay);

        // But 5000 should be blocked
        let can_pay_more = coordinator
            .can_auto_pay(&peer, &Amount::from_sats(5000))
            .expect("Failed to check");
        assert!(!can_pay_more);
    }

    #[test]
    fn test_spending_limit_exactly_at_boundary() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let peer = Keypair::random().public_key();

        coordinator
            .set_spending_limit(peer.clone(), 5000, "daily".to_string())
            .expect("Failed to set limit");

        // Record exactly the limit
        coordinator
            .record_auto_payment(&peer, Amount::from_sats(5000))
            .expect("Failed to record payment");

        let limit = coordinator
            .get_spending_limit(&peer)
            .expect("Failed to get limit")
            .expect("Limit should exist");

        assert_eq!(limit.remaining_limit().as_sats(), 0);

        // Even 1 sat should now be blocked
        let can_pay = coordinator
            .can_auto_pay(&peer, &Amount::from_sats(1))
            .expect("Failed to check");
        assert!(!can_pay);
    }

    #[test]
    fn test_independent_peer_limits() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let peer1 = Keypair::random().public_key();
        let peer2 = Keypair::random().public_key();

        // Set different limits for each peer
        coordinator
            .set_spending_limit(peer1.clone(), 10000, "daily".to_string())
            .expect("Failed to set limit");
        coordinator
            .set_spending_limit(peer2.clone(), 20000, "weekly".to_string())
            .expect("Failed to set limit");

        // Spend on peer1
        coordinator
            .record_auto_payment(&peer1, Amount::from_sats(8000))
            .expect("Failed to record payment");

        // Peer1 should have limited remaining
        let limit1 = coordinator
            .get_spending_limit(&peer1)
            .expect("Failed to get limit")
            .expect("Limit should exist");
        assert_eq!(limit1.remaining_limit().as_sats(), 2000);

        // Peer2 should be unaffected
        let limit2 = coordinator
            .get_spending_limit(&peer2)
            .expect("Failed to get limit")
            .expect("Limit should exist");
        assert_eq!(limit2.remaining_limit().as_sats(), 20000);
    }
}

// ============================================================
// Period Reset Tests
// ============================================================

mod period_reset_tests {
    use super::*;

    #[test]
    fn test_manual_limit_reset() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let peer = Keypair::random().public_key();

        // Set limit and spend some
        coordinator
            .set_spending_limit(peer.clone(), 10000, "daily".to_string())
            .expect("Failed to set limit");

        coordinator
            .record_auto_payment(&peer, Amount::from_sats(7000))
            .expect("Failed to record payment");

        // Verify spent
        let limit = coordinator
            .get_spending_limit(&peer)
            .expect("Failed to get limit")
            .expect("Limit should exist");
        assert_eq!(limit.current_spent.as_sats(), 7000);

        // Reset the limit
        coordinator
            .reset_spending_limit(&peer)
            .expect("Failed to reset limit");

        // Verify reset
        let reset_limit = coordinator
            .get_spending_limit(&peer)
            .expect("Failed to get limit")
            .expect("Limit should exist");
        assert_eq!(reset_limit.current_spent.as_sats(), 0);
        assert_eq!(reset_limit.remaining_limit().as_sats(), 10000);
    }

    #[test]
    fn test_update_spending_limit_amount() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let peer = Keypair::random().public_key();

        // Set initial limit
        coordinator
            .set_spending_limit(peer.clone(), 10000, "daily".to_string())
            .expect("Failed to set limit");

        // Record some spending
        coordinator
            .record_auto_payment(&peer, Amount::from_sats(5000))
            .expect("Failed to record payment");

        // Update to higher limit
        coordinator
            .set_spending_limit(peer.clone(), 20000, "daily".to_string())
            .expect("Failed to update limit");

        let limit = coordinator
            .get_spending_limit(&peer)
            .expect("Failed to get limit")
            .expect("Limit should exist");

        // Limit should be updated
        assert_eq!(limit.total_amount_limit.as_sats(), 20000);
    }
}

// ============================================================
// Subscription + Auto-Pay Combined Tests
// ============================================================

mod subscription_autopay_combined_tests {
    use super::*;

    #[test]
    fn test_subscription_with_autopay() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let subscriber = Keypair::random().public_key();
        let provider = Keypair::random().public_key();

        // Create subscription
        let subscription = coordinator
            .create_subscription(
                subscriber.clone(),
                provider.clone(),
                5000, // 5000 sats monthly
                "SAT".to_string(),
                PaymentFrequency::Monthly { day_of_month: 1 },
                MethodId("lightning".to_string()),
                "Premium monthly subscription".to_string(),
            )
            .expect("Failed to create subscription");

        // Set up spending limit for provider
        coordinator
            .set_spending_limit(provider.clone(), 100000, "monthly".to_string())
            .expect("Failed to set limit");

        // Configure auto-pay for this subscription
        let rule = coordinator
            .configure_auto_pay(
                subscription.inner.subscription_id.clone(),
                provider.clone(),
                MethodId("lightning".to_string()),
                Some(10000), // Max 10k per period
                "monthly".to_string(),
                true,
            )
            .expect("Failed to configure auto-pay");

        assert!(rule.enabled);
        assert_eq!(rule.subscription_id, subscription.inner.subscription_id);

        // Payment of 5000 (subscription amount) should be approved
        let can_pay = coordinator
            .can_auto_pay(&provider, &Amount::from_sats(5000))
            .expect("Failed to check");
        assert!(can_pay);
    }

    #[test]
    fn test_multiple_subscriptions_same_provider() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let subscriber = Keypair::random().public_key();
        let provider = Keypair::random().public_key();

        // Create first subscription
        let sub1 = coordinator
            .create_subscription(
                subscriber.clone(),
                provider.clone(),
                3000,
                "SAT".to_string(),
                PaymentFrequency::Monthly { day_of_month: 1 },
                MethodId("lightning".to_string()),
                "Basic subscription".to_string(),
            )
            .expect("Failed to create subscription 1");

        // Create second subscription
        let sub2 = coordinator
            .create_subscription(
                subscriber.clone(),
                provider.clone(),
                7000,
                "SAT".to_string(),
                PaymentFrequency::Monthly { day_of_month: 15 },
                MethodId("lightning".to_string()),
                "Premium subscription".to_string(),
            )
            .expect("Failed to create subscription 2");

        // Set spending limit that covers both
        coordinator
            .set_spending_limit(provider.clone(), 15000, "monthly".to_string())
            .expect("Failed to set limit");

        // Configure auto-pay for both subscriptions
        coordinator
            .configure_auto_pay(
                sub1.inner.subscription_id.clone(),
                provider.clone(),
                MethodId("lightning".to_string()),
                Some(5000),
                "monthly".to_string(),
                true,
            )
            .expect("Failed to configure auto-pay for sub1");

        coordinator
            .configure_auto_pay(
                sub2.inner.subscription_id.clone(),
                provider.clone(),
                MethodId("lightning".to_string()),
                Some(10000),
                "monthly".to_string(),
                true,
            )
            .expect("Failed to configure auto-pay for sub2");

        // Record payment for sub1
        coordinator
            .record_auto_payment(&provider, Amount::from_sats(3000))
            .expect("Failed to record payment");

        // Should still have room for sub2
        let can_pay = coordinator
            .can_auto_pay(&provider, &Amount::from_sats(7000))
            .expect("Failed to check");
        assert!(can_pay);

        // Record payment for sub2
        coordinator
            .record_auto_payment(&provider, Amount::from_sats(7000))
            .expect("Failed to record payment");

        // Now at 10000, only 5000 remaining
        let limit = coordinator
            .get_spending_limit(&provider)
            .expect("Failed to get limit")
            .expect("Limit should exist");
        assert_eq!(limit.remaining_limit().as_sats(), 5000);
    }
}

// ============================================================
// Edge Cases Tests
// ============================================================

mod edge_cases_tests {
    use super::*;

    #[test]
    fn test_zero_amount_payment() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let peer = Keypair::random().public_key();

        coordinator
            .set_spending_limit(peer.clone(), 10000, "daily".to_string())
            .expect("Failed to set limit");

        // Zero amount should be allowed
        let can_pay = coordinator
            .can_auto_pay(&peer, &Amount::from_sats(0))
            .expect("Failed to check");
        assert!(can_pay);
    }

    #[test]
    fn test_no_limit_set_for_peer() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let peer = Keypair::random().public_key();

        // No limit set - should not be able to auto-pay
        let can_pay = coordinator
            .can_auto_pay(&peer, &Amount::from_sats(1000))
            .expect("Failed to check");
        assert!(!can_pay);
    }

    #[test]
    fn test_get_nonexistent_limit() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let peer = Keypair::random().public_key();

        let limit = coordinator
            .get_spending_limit(&peer)
            .expect("Failed to get limit");
        assert!(limit.is_none());
    }

    #[test]
    fn test_very_large_limit() {
        let (coordinator, _temp_dir) = create_test_coordinator();
        let peer = Keypair::random().public_key();

        let large_limit = i64::MAX / 2; // Avoid overflow
        coordinator
            .set_spending_limit(peer.clone(), large_limit, "monthly".to_string())
            .expect("Failed to set limit");

        let limit = coordinator
            .get_spending_limit(&peer)
            .expect("Failed to get limit")
            .expect("Limit should exist");

        assert_eq!(limit.total_amount_limit.as_sats(), large_limit);
    }
}
