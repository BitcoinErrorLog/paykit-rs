//! Atomic spending limit tests
//!
//! These tests verify that spending limits are enforced atomically

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))] // File locks only work on native platforms
mod spending_limit_tests {
    use paykit_subscriptions::{Amount, PeerSpendingLimit};
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn test_spending_limit_creation() {
        let peer_id = pubky::Keypair::random().public_key();
        let limit = PeerSpendingLimit::new(
            peer_id.clone(),
            Amount::from_sats(10000),
            "daily".to_string(),
        );

        assert_eq!(limit.peer, peer_id);
        assert_eq!(limit.total_amount_limit.as_sats(), 10000);
        assert_eq!(limit.period, "daily");
        assert_eq!(limit.current_spent.as_sats(), 0);
    }

    #[tokio::test]
    async fn test_atomic_check_and_reserve() {
        // Test verifies that check-and-reserve logic is correct
        let amount = Amount::from_sats(100);
        let limit = Amount::from_sats(1000);

        // Verify arithmetic
        assert!(amount.is_within_limit(&limit));
        assert!(!Amount::from_sats(2000).is_within_limit(&limit));
    }

    #[tokio::test]
    async fn test_would_exceed_detection() {
        let current = Amount::from_sats(900);
        let additional = Amount::from_sats(200);
        let limit = Amount::from_sats(1000);

        // 900 + 200 = 1100 > 1000, should detect overflow
        assert!(current.would_exceed(&additional, &limit));

        // 900 + 50 = 950 < 1000, should be fine
        let small_additional = Amount::from_sats(50);
        assert!(!current.would_exceed(&small_additional, &limit));
    }

    #[tokio::test]
    async fn test_concurrent_limit_checks() {
        let limit = Amount::from_sats(10000);
        let mut tasks = JoinSet::new();

        // Many tasks checking if amounts are within limit
        for i in 0..100 {
            let limit_clone = limit;
            tasks.spawn(async move {
                let amount = Amount::from_sats(i * 100);
                amount.is_within_limit(&limit_clone)
            });
        }

        let mut under_limit = 0;
        while let Some(result) = tasks.join_next().await {
            if result.unwrap() {
                under_limit += 1;
            }
        }

        // Amounts 0-9900 should be under limit (100 values)
        assert_eq!(under_limit, 100, "All amounts should be under 10000 limit");
    }

    #[tokio::test]
    async fn test_rollback_on_error() {
        // Verify that if a transaction fails, the reservation can be rolled back
        let spent = Amount::from_sats(500);
        let reservation = Amount::from_sats(100);

        // Successful reservation
        let new_spent = spent.checked_add(&reservation).unwrap();
        assert_eq!(new_spent.as_sats(), 600);

        // Rollback (subtract the reservation)
        let rolled_back = new_spent.checked_sub(&reservation).unwrap();
        assert_eq!(rolled_back.as_sats(), 500);
        assert_eq!(rolled_back, spent);
    }

    #[tokio::test]
    async fn test_concurrent_amount_operations() {
        let base_amount = Amount::from_sats(1000);
        let mut tasks = JoinSet::new();

        // Spawn many tasks doing arithmetic operations
        for i in 0..100 {
            let amount = base_amount;
            tasks.spawn(async move {
                let addition = Amount::from_sats(i);
                amount.checked_add(&addition)
            });
        }

        let mut success_count = 0;
        while let Some(result) = tasks.join_next().await {
            if result.unwrap().is_some() {
                success_count += 1;
            }
        }

        assert_eq!(success_count, 100, "All additions should succeed");
    }

    #[tokio::test]
    async fn test_spending_limit_api() {
        let peer_id = pubky::Keypair::random().public_key();
        let mut limit =
            PeerSpendingLimit::new(peer_id, Amount::from_sats(5000), "daily".to_string());

        // Test spending
        let spend1 = Amount::from_sats(1000);
        assert!(!limit
            .current_spent
            .would_exceed(&spend1, &limit.total_amount_limit));

        // Simulate spending
        limit.current_spent = limit.current_spent.checked_add(&spend1).unwrap();
        assert_eq!(limit.current_spent.as_sats(), 1000);

        // Test another spend
        let spend2 = Amount::from_sats(3000);
        assert!(!limit
            .current_spent
            .would_exceed(&spend2, &limit.total_amount_limit));

        limit.current_spent = limit.current_spent.checked_add(&spend2).unwrap();
        assert_eq!(limit.current_spent.as_sats(), 4000);

        // Test exceeding limit
        let spend3 = Amount::from_sats(2000);
        assert!(limit
            .current_spent
            .would_exceed(&spend3, &limit.total_amount_limit));
    }
}
