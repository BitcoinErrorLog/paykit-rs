//! Tests for encrypted subscription storage
//!
//! These tests verify that subscription proposals, agreements, and cancellations
//! are properly encrypted using Paykit Sealed Blob v1 and can be decrypted
//! with the correct Noise secret key.

use paykit_lib::{MethodId, PublicKey};
use paykit_subscriptions::{
    Amount, PaymentFrequency, Signature, SignedSubscription, Subscription, SubscriptionTerms,
};
use pubky_noise::sealed_blob::{
    is_sealed_blob as check_sealed_blob, sealed_blob_decrypt as decrypt_blob,
    sealed_blob_encrypt as encrypt_blob,
};
use std::str::FromStr;

// ============================================================
// Test Helpers
// ============================================================

fn random_pubkey() -> PublicKey {
    let keypair = pkarr::Keypair::random();
    PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
}

fn random_x25519_keypair() -> ([u8; 32], [u8; 32]) {
    use rand::RngCore;
    let mut sk = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut sk);

    // Derive public key from secret key
    let pk = pubky_noise::kdf::x25519_pk_from_sk(&sk);
    (sk, pk)
}

fn create_test_terms() -> SubscriptionTerms {
    SubscriptionTerms::new(
        Amount::from_sats(10000),
        "SAT".to_string(),
        PaymentFrequency::Monthly { day_of_month: 1 },
        MethodId("lightning".to_string()),
        "Test subscription".to_string(),
    )
}

fn create_test_subscription() -> Subscription {
    let provider = random_pubkey();
    let subscriber = random_pubkey();

    Subscription::new(subscriber, provider, create_test_terms())
}

fn create_test_signature() -> Signature {
    let now = chrono::Utc::now().timestamp();
    Signature::new_ed25519(
        [0u8; 64],
        [0u8; 32],
        [0u8; 32],
        now,
        now + 3600, // expires in 1 hour
    )
}

fn create_test_signed_subscription() -> SignedSubscription {
    let subscription = create_test_subscription();
    SignedSubscription::new(
        subscription,
        create_test_signature(),
        create_test_signature(),
    )
}

// ============================================================
// Sealed Blob Encryption Tests
// ============================================================

#[test]
fn test_subscription_encrypt_decrypt_roundtrip() {
    let subscription = create_test_subscription();
    let (recipient_sk, recipient_pk) = random_x25519_keypair();

    // Serialize subscription
    let plaintext = serde_json::to_vec(&subscription).unwrap();

    // Use canonical /v0/ path format
    let path = format!(
        "/pub/paykit.app/v0/subscriptions/proposals/{}/{}",
        subscription.subscriber, subscription.subscription_id
    );
    // Use canonical AAD format: paykit:v0:subscription_proposal:{path}:{id}
    let aad = format!(
        "paykit:v0:subscription_proposal:{}:{}",
        path, subscription.subscription_id
    );

    let envelope = encrypt_blob(
        &recipient_pk,
        &plaintext,
        &aad,
        Some("subscription_proposal"),
    )
    .expect("Encryption should succeed");

    // Verify it's a sealed blob
    assert!(check_sealed_blob(&envelope), "Should be valid sealed blob");

    // Decrypt
    let decrypted =
        decrypt_blob(&recipient_sk, &envelope, &aad).expect("Decryption should succeed");

    // Parse and verify
    let recovered: Subscription =
        serde_json::from_slice(&decrypted).expect("Should deserialize subscription");

    assert_eq!(recovered.subscription_id, subscription.subscription_id);
    assert_eq!(recovered.provider, subscription.provider);
    assert_eq!(recovered.subscriber, subscription.subscriber);
    assert_eq!(recovered.terms.amount, subscription.terms.amount);
}

#[test]
fn test_signed_subscription_encrypt_decrypt_roundtrip() {
    let signed = create_test_signed_subscription();
    let (recipient_sk, recipient_pk) = random_x25519_keypair();

    // Serialize
    let plaintext = serde_json::to_vec(&signed).unwrap();

    // Use canonical /v0/ path format
    let path = format!(
        "/pub/paykit.app/v0/subscriptions/agreements/{}/{}",
        signed.subscription.subscriber, signed.subscription.subscription_id
    );
    // Use canonical AAD format: paykit:v0:subscription_agreement:{path}:{id}
    let aad = format!(
        "paykit:v0:subscription_agreement:{}:{}",
        path, signed.subscription.subscription_id
    );

    let envelope = encrypt_blob(
        &recipient_pk,
        &plaintext,
        &aad,
        Some("subscription_agreement"),
    )
    .expect("Encryption should succeed");

    // Decrypt
    let decrypted =
        decrypt_blob(&recipient_sk, &envelope, &aad).expect("Decryption should succeed");

    // Parse and verify
    let recovered: SignedSubscription =
        serde_json::from_slice(&decrypted).expect("Should deserialize signed subscription");

    assert_eq!(
        recovered.subscription.subscription_id,
        signed.subscription.subscription_id
    );
}

#[test]
fn test_cancellation_encrypt_decrypt_roundtrip() {
    let signed = create_test_signed_subscription();
    let (recipient_sk, recipient_pk) = random_x25519_keypair();

    // Create cancellation data
    let cancellation = serde_json::json!({
        "subscription_id": signed.subscription.subscription_id,
        "reason": "User requested cancellation",
        "cancelled_at": chrono::Utc::now().timestamp(),
    });
    let plaintext = serde_json::to_vec(&cancellation).unwrap();

    // Use canonical /v0/ path format
    let path = format!(
        "/pub/paykit.app/v0/subscriptions/cancellations/{}/{}",
        signed.subscription.subscriber, signed.subscription.subscription_id
    );
    // Use canonical AAD format: paykit:v0:subscription_cancellation:{path}:{id}
    let aad = format!(
        "paykit:v0:subscription_cancellation:{}:{}",
        path, signed.subscription.subscription_id
    );

    let envelope = encrypt_blob(
        &recipient_pk,
        &plaintext,
        &aad,
        Some("subscription_cancellation"),
    )
    .expect("Encryption should succeed");

    // Decrypt
    let decrypted =
        decrypt_blob(&recipient_sk, &envelope, &aad).expect("Decryption should succeed");

    // Parse and verify
    let recovered: serde_json::Value =
        serde_json::from_slice(&decrypted).expect("Should deserialize cancellation");

    assert_eq!(
        recovered["subscription_id"].as_str().unwrap(),
        signed.subscription.subscription_id
    );
    assert_eq!(
        recovered["reason"].as_str().unwrap(),
        "User requested cancellation"
    );
}

#[test]
fn test_wrong_key_fails_decryption() {
    let subscription = create_test_subscription();
    let (_recipient_sk, recipient_pk) = random_x25519_keypair();
    let (wrong_sk, _wrong_pk) = random_x25519_keypair();

    // Serialize and encrypt
    let plaintext = serde_json::to_vec(&subscription).unwrap();
    // Use canonical /v0/ path format
    let path = format!(
        "/pub/paykit.app/v0/subscriptions/proposals/{}/{}",
        subscription.subscriber, subscription.subscription_id
    );
    // Use canonical AAD format: paykit:v0:subscription_proposal:{path}:{id}
    let aad = format!(
        "paykit:v0:subscription_proposal:{}:{}",
        path, subscription.subscription_id
    );

    let envelope = encrypt_blob(
        &recipient_pk,
        &plaintext,
        &aad,
        Some("subscription_proposal"),
    )
    .expect("Encryption should succeed");

    // Try to decrypt with wrong key
    let result = decrypt_blob(&wrong_sk, &envelope, &aad);
    assert!(result.is_err(), "Decryption with wrong key should fail");
}

#[test]
fn test_wrong_aad_fails_decryption() {
    let subscription = create_test_subscription();
    let (recipient_sk, recipient_pk) = random_x25519_keypair();

    // Serialize and encrypt
    let plaintext = serde_json::to_vec(&subscription).unwrap();
    // Use canonical AAD format
    let aad = "paykit:v0:subscription_proposal:/pub/paykit.app/v0/subscriptions/proposals/scope/correct-id:correct-id";

    let envelope = encrypt_blob(
        &recipient_pk,
        &plaintext,
        aad,
        Some("subscription_proposal"),
    )
    .expect("Encryption should succeed");

    // Try to decrypt with wrong AAD
    let wrong_aad = "paykit:v0:subscription_proposal:/pub/paykit.app/v0/subscriptions/proposals/scope/wrong-id:wrong-id";
    let result = decrypt_blob(&recipient_sk, &envelope, wrong_aad);
    assert!(result.is_err(), "Decryption with wrong AAD should fail");
}

#[test]
fn test_legacy_plaintext_detection() {
    let subscription = create_test_subscription();

    // Plaintext JSON
    let plaintext_json = serde_json::to_string(&subscription).unwrap();
    assert!(
        !check_sealed_blob(&plaintext_json),
        "Plaintext JSON should not be detected as sealed blob"
    );

    // Encrypted envelope
    let (_sk, pk) = random_x25519_keypair();
    let plaintext = serde_json::to_vec(&subscription).unwrap();
    let envelope =
        encrypt_blob(&pk, &plaintext, "test:aad", None).expect("Encryption should succeed");

    assert!(
        check_sealed_blob(&envelope),
        "Encrypted envelope should be detected as sealed blob"
    );
}
