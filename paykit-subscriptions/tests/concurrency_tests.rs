//! Concurrency stress tests for NonceStore
//!
//! These tests verify thread-safety under high contention

#[cfg(test)]
mod concurrency_tests {
    use paykit_subscriptions::NonceStore;
    use std::sync::Arc;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn test_concurrent_nonce_checking() {
        let store = Arc::new(NonceStore::new());
        let mut tasks = JoinSet::new();

        // Spawn 100 concurrent tasks all trying to use the same nonce
        let nonce = [42u8; 32];
        let expires_at = chrono::Utc::now().timestamp() + 3600;

        for _ in 0..100 {
            let store_clone = Arc::clone(&store);
            tasks.spawn(async move { store_clone.check_and_mark(&nonce, expires_at) });
        }

        // Collect results
        let mut success_count = 0;
        let mut failure_count = 0;

        while let Some(result) = tasks.join_next().await {
            match result.unwrap() {
                Ok(true) => success_count += 1,  // Nonce was new
                Ok(false) => failure_count += 1, // Nonce was already used
                Err(_) => panic!("Unexpected error"),
            }
        }

        // Exactly one task should succeed, the rest should fail
        assert_eq!(
            success_count, 1,
            "Exactly one task should successfully mark the nonce as new"
        );
        assert_eq!(
            failure_count, 99,
            "All other tasks should see the nonce as already used"
        );
    }

    #[tokio::test]
    async fn test_concurrent_different_nonces() {
        let store = Arc::new(NonceStore::new());
        let mut tasks = JoinSet::new();
        let expires_at = chrono::Utc::now().timestamp() + 3600;

        // Spawn 100 tasks each using a unique nonce
        for i in 0..100u8 {
            let store_clone = Arc::clone(&store);
            let mut nonce = [0u8; 32];
            nonce[0] = i;

            tasks.spawn(async move { store_clone.check_and_mark(&nonce, expires_at) });
        }

        // All should succeed since they're all unique nonces
        let mut success_count = 0;
        while let Some(result) = tasks.join_next().await {
            match result.unwrap() {
                Ok(true) => success_count += 1,
                Ok(false) => panic!("Unique nonce should not be marked as used"),
                Err(_) => panic!("Unexpected error"),
            }
        }

        assert_eq!(success_count, 100, "All unique nonces should succeed");
    }

    #[tokio::test]
    async fn test_high_contention_stress() {
        let store = Arc::new(NonceStore::new());
        let mut tasks = JoinSet::new();
        let expires_at = chrono::Utc::now().timestamp() + 3600;

        // Create 50 different nonces, each will be attempted by 10 tasks
        for nonce_id in 0..50u8 {
            for _ in 0..10 {
                let store_clone = Arc::clone(&store);
                let mut nonce = [0u8; 32];
                nonce[0] = nonce_id;

                tasks.spawn(
                    async move { (nonce_id, store_clone.check_and_mark(&nonce, expires_at)) },
                );
            }
        }

        // Count successes per nonce
        use std::collections::HashMap;
        let mut nonce_successes: HashMap<u8, usize> = HashMap::new();

        while let Some(result) = tasks.join_next().await {
            let (nonce_id, check_result) = result.unwrap();
            if let Ok(true) = check_result {
                *nonce_successes.entry(nonce_id).or_insert(0) += 1;
            }
        }

        // Each nonce should have exactly one success
        assert_eq!(
            nonce_successes.len(),
            50,
            "All 50 nonces should have been checked"
        );
        for (nonce_id, count) in nonce_successes {
            assert_eq!(
                count, 1,
                "Nonce {} should have exactly one success",
                nonce_id
            );
        }
    }

    #[tokio::test]
    async fn test_no_deadlock_under_load() {
        let store = Arc::new(NonceStore::new());
        let mut tasks = JoinSet::new();
        let expires_at = chrono::Utc::now().timestamp() + 3600;

        // Spawn 1000 tasks doing random operations
        for i in 0..1000u16 {
            let store_clone = Arc::clone(&store);
            let mut nonce = [0u8; 32];
            // Use i to create some duplicate nonces
            nonce[0] = (i % 100) as u8;
            nonce[1] = (i / 100) as u8;

            tasks.spawn(async move {
                // Each task attempts the check twice to increase contention
                let _ = store_clone.check_and_mark(&nonce, expires_at);
                tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
                store_clone.check_and_mark(&nonce, expires_at)
            });
        }

        // Just verify all tasks complete without deadlocking
        let mut completed = 0;
        while let Some(result) = tasks.join_next().await {
            result.unwrap().unwrap(); // Unwrap join handle and Result
            completed += 1;
        }

        assert_eq!(completed, 1000, "All tasks should complete");
    }

    #[tokio::test]
    async fn test_concurrent_expired_nonce_cleanup() {
        let store = Arc::new(NonceStore::new());
        let now = chrono::Utc::now().timestamp();

        // Add some expired nonces
        let nonce1 = [1u8; 32];
        let nonce2 = [2u8; 32];

        // These should expire immediately
        store.check_and_mark(&nonce1, now - 1).unwrap();
        store.check_and_mark(&nonce2, now - 1).unwrap();

        // Clean up expired nonces
        store.cleanup_expired(now).unwrap();

        // Try to use them again - should succeed since they've been cleaned up
        let result1 = store.check_and_mark(&nonce1, now + 3600);
        let result2 = store.check_and_mark(&nonce2, now + 3600);

        assert!(result1.is_ok());
        assert!(result1.unwrap(), "Cleaned up nonce should be reusable");
        assert!(result2.is_ok());
        assert!(result2.unwrap(), "Cleaned up nonce should be reusable");
    }

    #[tokio::test]
    async fn test_concurrent_mixed_operations() {
        let store = Arc::new(NonceStore::new());
        let mut tasks = JoinSet::new();
        let base_time = chrono::Utc::now().timestamp();

        // Mix of operations: new nonces, duplicates, and expired nonces
        for i in 0..200u8 {
            let store_clone = Arc::clone(&store);
            let mut nonce = [0u8; 32];
            nonce[0] = i % 50; // Creates duplicates

            let expires_at = if i % 10 == 0 {
                // Every 10th nonce expires immediately
                base_time - 1
            } else {
                base_time + 3600
            };

            tasks.spawn(async move { store_clone.check_and_mark(&nonce, expires_at) });
        }

        // Just verify no panics or errors
        let mut total = 0;
        while let Some(result) = tasks.join_next().await {
            result.unwrap().unwrap();
            total += 1;
        }

        assert_eq!(total, 200, "All operations should complete");
    }
}
