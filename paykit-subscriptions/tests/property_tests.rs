//! Property-based tests for paykit-subscriptions
//!
//! These tests use proptest to verify invariants across a wide range of inputs.

#[cfg(test)]
mod amount_properties {
    use paykit_subscriptions::Amount;
    use proptest::prelude::*;

    proptest! {
        /// Addition is commutative: a + b = b + a
        #[test]
        fn addition_commutative(a in 0i64..1_000_000i64, b in 0i64..1_000_000i64) {
            let amount_a = Amount::from_sats(a);
            let amount_b = Amount::from_sats(b);

            let sum1 = amount_a.checked_add(&amount_b);
            let sum2 = amount_b.checked_add(&amount_a);

            prop_assert_eq!(sum1.map(|x| x.as_sats()), sum2.map(|x| x.as_sats()));
        }

        /// Addition is associative: (a + b) + c = a + (b + c)
        #[test]
        fn addition_associative(
            a in 0i64..100_000i64,
            b in 0i64..100_000i64,
            c in 0i64..100_000i64
        ) {
            let amount_a = Amount::from_sats(a);
            let amount_b = Amount::from_sats(b);
            let amount_c = Amount::from_sats(c);

            if let (Some(ab), Some(bc)) = (amount_a.checked_add(&amount_b), amount_b.checked_add(&amount_c)) {
                if let (Some(abc1), Some(abc2)) = (ab.checked_add(&amount_c), amount_a.checked_add(&bc)) {
                    prop_assert_eq!(abc1.as_sats(), abc2.as_sats());
                }
            }
        }

        /// Subtraction is the inverse of addition
        #[test]
        fn subtraction_inverts_addition(a in 0i64..1_000_000i64, b in 0i64..1_000_000i64) {
            let amount_a = Amount::from_sats(a.max(b));
            let amount_b = Amount::from_sats(a.min(b));

            if let Some(sum) = amount_a.checked_add(&amount_b) {
                if let Some(diff) = sum.checked_sub(&amount_b) {
                    prop_assert_eq!(diff.as_sats(), amount_a.as_sats());
                }
            }
        }

        /// Saturating addition never fails
        #[test]
        fn saturating_add_never_fails(a in 0i64..1_000_000i64, b in 0i64..1_000_000i64) {
            let amount_a = Amount::from_sats(a);
            let amount_b = Amount::from_sats(b);

            let result = amount_a.saturating_add(&amount_b);
            prop_assert!(result.as_sats() >= a);
            prop_assert!(result.as_sats() >= b);
        }

        /// Round-trip through satoshis preserves value
        #[test]
        fn sats_round_trip(sats in 0i64..1_000_000_000i64) {
            let original = Amount::from_sats(sats);
            let retrieved = original.as_sats();

            prop_assert_eq!(sats, retrieved);
        }

        /// Comparison is consistent with satoshi value
        #[test]
        fn comparison_consistent(a in 0i64..1_000_000i64, b in 0i64..1_000_000i64) {
            let amount_a = Amount::from_sats(a);
            let amount_b = Amount::from_sats(b);

            if a < b {
                prop_assert!(amount_a < amount_b);
            } else if a > b {
                prop_assert!(amount_a > amount_b);
            } else {
                prop_assert_eq!(amount_a, amount_b);
            }
        }

        /// is_within_limit is transitive
        #[test]
        fn limit_check_transitive(a in 0i64..100i64, b in 100i64..200i64, c in 200i64..300i64) {
            let amount_a = Amount::from_sats(a);
            let amount_b = Amount::from_sats(b);
            let amount_c = Amount::from_sats(c);

            prop_assert!(amount_a.is_within_limit(&amount_b));
            prop_assert!(amount_b.is_within_limit(&amount_c));
            prop_assert!(amount_a.is_within_limit(&amount_c));
        }

        /// would_exceed is consistent with addition
        #[test]
        fn would_exceed_consistent(a in 0i64..100_000i64, b in 0i64..100_000i64, c in 200_000i64..300_000i64) {
            let current = Amount::from_sats(a);
            let additional = Amount::from_sats(b);
            let limit = Amount::from_sats(c);

            let would_exceed = current.would_exceed(&additional, &limit);
            if let Some(total) = current.checked_add(&additional) {
                prop_assert_eq!(would_exceed, total > limit);
            }
        }
    }
}

#[cfg(test)]
mod serialization_properties {
    use paykit_lib::MethodId;
    use paykit_subscriptions::{Amount, PaymentFrequency, Subscription, SubscriptionTerms};
    use proptest::prelude::*;

    prop_compose! {
        fn arb_subscription()(
            subscription_id in "[a-z]{8}-[0-9]{4}",
            amount_sats in 1000i64..1_000_000i64,
        ) -> Subscription {
            // Create a valid test keypair and use its public key
            let keypair = pubky::Keypair::random();
            let subscriber = keypair.public_key();
            let provider = keypair.public_key();

            Subscription {
                subscription_id,
                subscriber,
                provider,
                terms: SubscriptionTerms {
                    amount: Amount::from_sats(amount_sats),
                    currency: "SAT".to_string(),
                    frequency: PaymentFrequency::Monthly { day_of_month: 1 },
                    method: MethodId("lightning".to_string()),
                    max_amount_per_period: None,
                    description: "Test subscription".to_string(),
                },
                metadata: serde_json::json!({}),
                created_at: 1700000000,
                starts_at: 1700000000,
                ends_at: Some(1800000000),
            }
        }
    }

    proptest! {
        /// JSON serialization round-trip preserves data
        #[test]
        fn json_round_trip(subscription in arb_subscription()) {
            let serialized = serde_json::to_string(&subscription)
                .expect("serialization should succeed");
            let deserialized: Subscription = serde_json::from_str(&serialized)
                .expect("deserialization should succeed");

            prop_assert_eq!(subscription.subscription_id, deserialized.subscription_id);
            prop_assert_eq!(subscription.subscriber, deserialized.subscriber);
            prop_assert_eq!(subscription.provider, deserialized.provider);
            prop_assert_eq!(subscription.terms.amount, deserialized.terms.amount);
        }

        /// JSON serialization is deterministic (same input produces same output)
        #[test]
        fn json_deterministic(subscription in arb_subscription()) {
            let serialized1 = serde_json::to_string(&subscription)
                .expect("serialization should succeed");
            let serialized2 = serde_json::to_string(&subscription)
                .expect("serialization should succeed");

            prop_assert_eq!(serialized1, serialized2);
        }
    }
}

#[cfg(test)]
mod nonce_properties {
    use paykit_subscriptions::NonceStore;
    use proptest::prelude::*;
    use std::sync::Arc;

    proptest! {
        /// Nonces are properly tracked
        #[test]
        fn nonce_tracking(nonce_bytes in prop::collection::vec(any::<u8>(), 32)) {
            let mut nonce = [0u8; 32];
            nonce.copy_from_slice(&nonce_bytes);

            let store = Arc::new(NonceStore::new());
            let expires_at = chrono::Utc::now().timestamp() + 3600;

            // First use should return true (nonce is new)
            let result1 = store.check_and_mark(&nonce, expires_at);
            prop_assert!(result1.is_ok());
            prop_assert_eq!(result1.unwrap(), true);

            // Immediate reuse should return false (nonce already used)
            let result2 = store.check_and_mark(&nonce, expires_at);
            prop_assert!(result2.is_ok());
            prop_assert_eq!(result2.unwrap(), false);
        }

        /// Different nonces don't interfere
        #[test]
        fn nonce_independence(
            nonce1_bytes in prop::collection::vec(any::<u8>(), 32),
            nonce2_bytes in prop::collection::vec(any::<u8>(), 32)
        ) {
            let mut nonce1 = [0u8; 32];
            let mut nonce2 = [0u8; 32];
            nonce1.copy_from_slice(&nonce1_bytes);
            nonce2.copy_from_slice(&nonce2_bytes);

            if nonce1 != nonce2 {
                let store = Arc::new(NonceStore::new());
                let expires_at = chrono::Utc::now().timestamp() + 3600;

                // Use first nonce
                let result1 = store.check_and_mark(&nonce1, expires_at);
                prop_assert!(result1.is_ok());
                prop_assert_eq!(result1.unwrap(), true);

                // Second nonce should still work
                let result2 = store.check_and_mark(&nonce2, expires_at);
                prop_assert!(result2.is_ok());
                prop_assert_eq!(result2.unwrap(), true);
            }
        }
    }
}
