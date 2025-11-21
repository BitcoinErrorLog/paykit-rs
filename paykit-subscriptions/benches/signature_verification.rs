use criterion::{black_box, criterion_group, criterion_main, Criterion};
use paykit_subscriptions::{signing, Amount};

fn signature_verification_benchmark(c: &mut Criterion) {
    // Use pubky keypair instead of raw SigningKey
    let keypair = pubky::Keypair::random();

    let nonce = [42u8; 32];
    let timestamp = chrono::Utc::now().timestamp();
    let lifetime_seconds = 3600;
    let expires_at = timestamp + lifetime_seconds;

    // Create test subscription
    let subscription = paykit_subscriptions::Subscription {
        subscription_id: "test-sub-001".to_string(),
        subscriber: pubky::Keypair::random().public_key(),
        provider: pubky::Keypair::random().public_key(),
        terms: paykit_subscriptions::SubscriptionTerms {
            amount: Amount::from_sats(1000),
            currency: "SAT".to_string(),
            frequency: paykit_subscriptions::PaymentFrequency::Monthly { day_of_month: 1 },
            method: paykit_lib::MethodId("lightning".to_string()),
            max_amount_per_period: None,
            description: "Test subscription".to_string(),
        },
        metadata: serde_json::json!({}),
        created_at: timestamp,
        starts_at: timestamp,
        ends_at: Some(expires_at),
    };

    c.bench_function("sign_subscription", |b| {
        b.iter(|| {
            signing::sign_subscription_ed25519(
                black_box(&subscription),
                black_box(&keypair),
                black_box(&nonce),
                black_box(lifetime_seconds),
            )
        })
    });
}

criterion_group!(benches, signature_verification_benchmark);
criterion_main!(benches);
