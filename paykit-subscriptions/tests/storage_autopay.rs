//! Storage tests for auto-pay and spending limits
//!
//! Tests the FileSubscriptionStorage implementation for:
//! - Auto-pay rule persistence
//! - Peer spending limit persistence
//! - Atomic spending reservation operations

use paykit_lib::{MethodId, PublicKey};
use paykit_subscriptions::{
    storage::{FileSubscriptionStorage, SubscriptionStorage},
    Amount, AutoPayRule, PeerSpendingLimit,
};
use std::str::FromStr;
use tempfile::TempDir;

// ============================================================
// Test Helpers
// ============================================================

fn random_pubkey() -> PublicKey {
    let keypair = pkarr::Keypair::random();
    PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
}

/// Creates test storage, returning both storage and temp_dir to keep it alive
fn create_test_storage() -> (FileSubscriptionStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileSubscriptionStorage::new(temp_dir.path().to_path_buf()).unwrap();
    (storage, temp_dir)
}

fn create_test_rule(subscription_id: &str, peer: PublicKey) -> AutoPayRule {
    AutoPayRule::new(
        subscription_id.to_string(),
        peer,
        MethodId("lightning".to_string()),
    )
}

fn create_test_limit(peer: PublicKey, limit_sats: i64, period: &str) -> PeerSpendingLimit {
    PeerSpendingLimit::new(peer, Amount::from_sats(limit_sats), period.to_string())
}

// ============================================================
// Auto-Pay Rule Storage Tests
// ============================================================

mod autopay_rule_storage_tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_get_autopay_rule() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();
        let rule = create_test_rule("sub_test", peer);

        // Save
        storage.save_autopay_rule(&rule).await.unwrap();

        // Get
        let loaded = storage.get_autopay_rule("sub_test").await.unwrap();
        assert!(loaded.is_some());

        let loaded_rule = loaded.unwrap();
        assert_eq!(loaded_rule.subscription_id, "sub_test");
        assert!(loaded_rule.enabled);
    }

    #[tokio::test]
    async fn test_get_nonexistent_autopay_rule() {
        let (storage, _temp_dir) = create_test_storage();

        let loaded = storage.get_autopay_rule("nonexistent").await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_update_autopay_rule() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();

        // Create and save initial rule
        let mut rule = create_test_rule("sub_update", peer);
        storage.save_autopay_rule(&rule).await.unwrap();

        // Update rule
        rule.enabled = false;
        rule.require_confirmation = true;
        storage.save_autopay_rule(&rule).await.unwrap();

        // Verify update
        let loaded = storage.get_autopay_rule("sub_update").await.unwrap().unwrap();
        assert!(!loaded.enabled);
        assert!(loaded.require_confirmation);
    }

    #[tokio::test]
    async fn test_autopay_rule_with_max_amount() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();

        let rule = create_test_rule("sub_max", peer)
            .with_max_payment_amount(Amount::from_sats(5000))
            .with_max_period_amount(Amount::from_sats(50000), "weekly".to_string());

        storage.save_autopay_rule(&rule).await.unwrap();

        let loaded = storage.get_autopay_rule("sub_max").await.unwrap().unwrap();
        assert_eq!(loaded.max_amount_per_payment, Some(Amount::from_sats(5000)));
        assert_eq!(
            loaded.max_total_amount_per_period,
            Some(Amount::from_sats(50000))
        );
        assert_eq!(loaded.period, Some("weekly".to_string()));
    }

    #[tokio::test]
    async fn test_multiple_autopay_rules() {
        let (storage, _temp_dir) = create_test_storage();

        // Create multiple rules
        for i in 0..5 {
            let peer = random_pubkey();
            let rule = create_test_rule(&format!("sub_{}", i), peer);
            storage.save_autopay_rule(&rule).await.unwrap();
        }

        // Verify each can be retrieved
        for i in 0..5 {
            let loaded = storage
                .get_autopay_rule(&format!("sub_{}", i))
                .await
                .unwrap();
            assert!(loaded.is_some());
        }
    }
}

// ============================================================
// Peer Spending Limit Storage Tests
// ============================================================

mod peer_spending_limit_storage_tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_get_peer_limit() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();
        let limit = create_test_limit(peer.clone(), 10000, "monthly");

        // Save
        storage.save_peer_limit(&limit).await.unwrap();

        // Get
        let loaded = storage.get_peer_limit(&peer).await.unwrap();
        assert!(loaded.is_some());

        let loaded_limit = loaded.unwrap();
        assert_eq!(loaded_limit.total_amount_limit, Amount::from_sats(10000));
        assert_eq!(loaded_limit.period, "monthly");
    }

    #[tokio::test]
    async fn test_get_nonexistent_peer_limit() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();

        let loaded = storage.get_peer_limit(&peer).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_update_peer_limit_spending() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();

        // Create and save initial limit
        let mut limit = create_test_limit(peer.clone(), 10000, "daily");
        storage.save_peer_limit(&limit).await.unwrap();

        // Update spending
        limit.add_spent(&Amount::from_sats(3000)).unwrap();
        storage.save_peer_limit(&limit).await.unwrap();

        // Verify update
        let loaded = storage.get_peer_limit(&peer).await.unwrap().unwrap();
        assert_eq!(loaded.current_spent, Amount::from_sats(3000));
        assert_eq!(loaded.remaining_limit(), Amount::from_sats(7000));
    }

    #[tokio::test]
    async fn test_multiple_peer_limits() {
        let (storage, _temp_dir) = create_test_storage();

        let mut peers = Vec::new();
        for i in 0..5 {
            let peer = random_pubkey();
            let limit = create_test_limit(peer.clone(), (i + 1) * 10000, "monthly");
            storage.save_peer_limit(&limit).await.unwrap();
            peers.push(peer);
        }

        // Verify each can be retrieved with correct limit
        for (i, peer) in peers.iter().enumerate() {
            let loaded = storage.get_peer_limit(peer).await.unwrap().unwrap();
            assert_eq!(
                loaded.total_amount_limit,
                Amount::from_sats(((i + 1) * 10000) as i64)
            );
        }
    }

    #[tokio::test]
    async fn test_different_periods() {
        let (storage, _temp_dir) = create_test_storage();

        let periods = ["daily", "weekly", "monthly"];
        let mut peers = Vec::new();

        for period in &periods {
            let peer = random_pubkey();
            let limit = create_test_limit(peer.clone(), 10000, period);
            storage.save_peer_limit(&limit).await.unwrap();
            peers.push((peer, period.to_string()));
        }

        for (peer, expected_period) in peers {
            let loaded = storage.get_peer_limit(&peer).await.unwrap().unwrap();
            assert_eq!(loaded.period, expected_period);
        }
    }
}

// ============================================================
// Atomic Spending Reservation Tests
// ============================================================

mod atomic_spending_tests {
    use super::*;

    #[tokio::test]
    async fn test_try_reserve_spending_success() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();

        // Create limit
        let limit = create_test_limit(peer.clone(), 10000, "daily");
        storage.save_peer_limit(&limit).await.unwrap();

        // Reserve
        let token = storage
            .try_reserve_spending(&peer, &Amount::from_sats(5000))
            .await
            .unwrap();

        assert_eq!(token.amount, Amount::from_sats(5000));

        // Verify limit was updated
        let loaded = storage.get_peer_limit(&peer).await.unwrap().unwrap();
        assert_eq!(loaded.current_spent, Amount::from_sats(5000));
    }

    #[tokio::test]
    async fn test_try_reserve_spending_exceeds_limit() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();

        // Create limit
        let limit = create_test_limit(peer.clone(), 10000, "daily");
        storage.save_peer_limit(&limit).await.unwrap();

        // Try to reserve more than limit
        let result = storage
            .try_reserve_spending(&peer, &Amount::from_sats(15000))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_try_reserve_spending_no_limit() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();

        // No limit set - should fail
        let result = storage
            .try_reserve_spending(&peer, &Amount::from_sats(1000))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_commit_spending() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();

        // Create limit
        let limit = create_test_limit(peer.clone(), 10000, "daily");
        storage.save_peer_limit(&limit).await.unwrap();

        // Reserve and commit
        let token = storage
            .try_reserve_spending(&peer, &Amount::from_sats(3000))
            .await
            .unwrap();

        storage.commit_spending(token).await.unwrap();

        // Spending should remain
        let loaded = storage.get_peer_limit(&peer).await.unwrap().unwrap();
        assert_eq!(loaded.current_spent, Amount::from_sats(3000));
    }

    #[tokio::test]
    async fn test_rollback_spending() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();

        // Create limit
        let limit = create_test_limit(peer.clone(), 10000, "daily");
        storage.save_peer_limit(&limit).await.unwrap();

        // Reserve
        let token = storage
            .try_reserve_spending(&peer, &Amount::from_sats(5000))
            .await
            .unwrap();

        // Verify reserved
        let loaded = storage.get_peer_limit(&peer).await.unwrap().unwrap();
        assert_eq!(loaded.current_spent, Amount::from_sats(5000));

        // Rollback
        storage.rollback_spending(token).await.unwrap();

        // Verify rolled back
        let loaded = storage.get_peer_limit(&peer).await.unwrap().unwrap();
        assert_eq!(loaded.current_spent, Amount::from_sats(0));
    }

    #[tokio::test]
    async fn test_multiple_reservations() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();

        // Create limit
        let limit = create_test_limit(peer.clone(), 10000, "daily");
        storage.save_peer_limit(&limit).await.unwrap();

        // Multiple reservations
        let _token1 = storage
            .try_reserve_spending(&peer, &Amount::from_sats(2000))
            .await
            .unwrap();

        let _token2 = storage
            .try_reserve_spending(&peer, &Amount::from_sats(3000))
            .await
            .unwrap();

        let loaded = storage.get_peer_limit(&peer).await.unwrap().unwrap();
        assert_eq!(loaded.current_spent, Amount::from_sats(5000));

        // Third reservation that would exceed limit
        let result = storage
            .try_reserve_spending(&peer, &Amount::from_sats(6000))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reservation_exactly_at_limit() {
        let (storage, _temp_dir) = create_test_storage();
        let peer = random_pubkey();

        // Create limit of exactly 1000
        let limit = create_test_limit(peer.clone(), 1000, "daily");
        storage.save_peer_limit(&limit).await.unwrap();

        // Reserve exactly the limit
        let token = storage
            .try_reserve_spending(&peer, &Amount::from_sats(1000))
            .await
            .unwrap();

        assert_eq!(token.amount, Amount::from_sats(1000));

        // Verify at limit
        let loaded = storage.get_peer_limit(&peer).await.unwrap().unwrap();
        assert_eq!(loaded.remaining_limit(), Amount::from_sats(0));

        // Any more should fail
        let result = storage
            .try_reserve_spending(&peer, &Amount::from_sats(1))
            .await;

        assert!(result.is_err());
    }
}

// ============================================================
// Concurrent Access Tests
// ============================================================

mod concurrent_tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_concurrent_reads() {
        let (storage, _temp_dir) = create_test_storage();
        let storage = Arc::new(storage);
        let peer = random_pubkey();

        // Create limit
        let limit = create_test_limit(peer.clone(), 10000, "daily");
        storage.save_peer_limit(&limit).await.unwrap();

        // Concurrent reads
        let mut handles = vec![];
        for _ in 0..10 {
            let storage_clone = Arc::clone(&storage);
            let peer_clone = peer.clone();
            handles.push(tokio::spawn(async move {
                let loaded = storage_clone
                    .get_peer_limit(&peer_clone)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(loaded.total_amount_limit, Amount::from_sats(10000));
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }

    // Note: Full concurrent reservation testing would require more complex
    // synchronization, but the file-locking mechanism in try_reserve_spending
    // handles this at the OS level
}
