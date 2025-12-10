use paykit_interactive::{PaykitNoiseMessage, PaykitReceipt};
use paykit_lib::{MethodId, PublicKey};
use serde_json::json;

// Helper to get a test key
fn get_test_key(_s: &str) -> PublicKey {
    // Generate a random valid key using pubky
    let keypair = pubky::Keypair::random();
    keypair.public_key()
}

#[test]
fn test_receipt_serialization() {
    // If pubky feature is on, we need a valid key string.
    let key_str = "8".repeat(52);
    let payer = get_test_key(&key_str);
    let payee = get_test_key(&key_str);
    let method = MethodId("lightning".to_string());

    let receipt = PaykitReceipt::new(
        "receipt_123".to_string(),
        payer.clone(),
        payee.clone(),
        method.clone(),
        Some("1000".to_string()),
        Some("SAT".to_string()),
        json!({"invoice": "lnbc1..."}),
    );

    let serialized = serde_json::to_string(&receipt).unwrap();
    println!("Serialized Receipt: {}", serialized);

    let deserialized: PaykitReceipt = serde_json::from_str(&serialized).unwrap();
    assert_eq!(receipt.receipt_id, deserialized.receipt_id);
    assert_eq!(receipt.payer, deserialized.payer);
    assert_eq!(receipt.payee, deserialized.payee);
    assert_eq!(receipt.method_id, deserialized.method_id);
    assert_eq!(receipt.amount, deserialized.amount);
    assert_eq!(receipt.currency, deserialized.currency);
    assert_eq!(receipt.metadata, deserialized.metadata);
    // created_at might differ slightly if we re-created it, but here we deserialized exact value
    assert_eq!(receipt.created_at, deserialized.created_at);
}

#[test]
fn test_message_serialization() {
    let method = MethodId("onchain".to_string());
    let msg = PaykitNoiseMessage::OfferPrivateEndpoint {
        method_id: method,
        endpoint: "http://private.endpoint".to_string(),
    };

    let serialized = serde_json::to_string(&msg).unwrap();
    println!("Serialized Message: {}", serialized);

    // Verify structure matches expectations (tag="type", content="payload")
    let value: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(value["type"], "OfferPrivateEndpoint");
    assert_eq!(value["payload"]["method_id"], "onchain");
    assert_eq!(value["payload"]["endpoint"], "http://private.endpoint");

    let deserialized: PaykitNoiseMessage = serde_json::from_str(&serialized).unwrap();
    if let PaykitNoiseMessage::OfferPrivateEndpoint {
        method_id,
        endpoint,
    } = deserialized
    {
        assert_eq!(method_id.0, "onchain");
        assert_eq!(endpoint, "http://private.endpoint");
    } else {
        panic!("Wrong variant deserialized");
    }
}
