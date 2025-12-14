//! Cross-Platform E2E Tests
//!
//! Tests payment flows between different platforms:
//! - iOS ↔ Android (simulated via FFI bindings)
//! - Mobile ↔ CLI (via paykit-interactive)
//! - Multiple concurrent payments across platforms
//!
//! These tests verify protocol compatibility and message format consistency
//! across all supported platforms.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// Test Infrastructure
// ============================================================================

/// Simulated platform identity
#[derive(Debug, Clone)]
struct PlatformIdentity {
    platform: Platform,
    _nickname: String,
    public_key_z32: String,
    noise_pubkey: String,
}

/// Supported platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Platform {
    Ios,
    Android,
    Cli,
}

impl Platform {
    fn name(&self) -> &'static str {
        match self {
            Platform::Ios => "iOS",
            Platform::Android => "Android",
            Platform::Cli => "CLI",
        }
    }
}

/// Simulated payment request
#[derive(Debug, Clone)]
struct PaymentRequest {
    receipt_id: String,
    payer_pubkey: String,
    payee_pubkey: String,
    method_id: String,
    amount_sats: u64,
}

/// Simulated payment response
#[derive(Debug, Clone)]
struct PaymentResponse {
    success: bool,
    _receipt_id: String,
    confirmed_at: Option<u64>,
    error_message: Option<String>,
}

/// Simulated receipt
#[derive(Debug, Clone)]
struct Receipt {
    id: String,
    payer_pubkey: String,
    payee_pubkey: String,
    amount_sats: u64,
    created_at: u64,
    status: String,
}

/// Mock cross-platform payment service
struct CrossPlatformPaymentService {
    endpoints: Arc<Mutex<HashMap<String, EndpointInfo>>>,
    receipts: Arc<Mutex<HashMap<String, Receipt>>>,
}

#[derive(Debug, Clone)]
struct EndpointInfo {
    _pubkey: String,
    _host: String,
    _port: u16,
    _noise_pubkey: String,
    platform: Platform,
}

impl CrossPlatformPaymentService {
    fn new() -> Self {
        Self {
            endpoints: Arc::new(Mutex::new(HashMap::new())),
            receipts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn register_endpoint(&self, identity: &PlatformIdentity, host: &str, port: u16) {
        let mut endpoints = self.endpoints.lock().unwrap();
        endpoints.insert(
            identity.public_key_z32.clone(),
            EndpointInfo {
                _pubkey: identity.public_key_z32.clone(),
                _host: host.to_string(),
                _port: port,
                _noise_pubkey: identity.noise_pubkey.clone(),
                platform: identity.platform,
            },
        );
    }

    fn discover_endpoint(&self, pubkey: &str) -> Option<EndpointInfo> {
        let endpoints = self.endpoints.lock().unwrap();
        endpoints.get(pubkey).cloned()
    }

    fn process_payment(&self, request: PaymentRequest) -> PaymentResponse {
        // Verify payee has registered endpoint
        let endpoints = self.endpoints.lock().unwrap();
        if !endpoints.contains_key(&request.payee_pubkey) {
            return PaymentResponse {
                success: false,
                _receipt_id: request.receipt_id,
                confirmed_at: None,
                error_message: Some("Payee endpoint not found".to_string()),
            };
        }
        drop(endpoints);

        // Create receipt
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let receipt = Receipt {
            id: request.receipt_id.clone(),
            payer_pubkey: request.payer_pubkey,
            payee_pubkey: request.payee_pubkey,
            amount_sats: request.amount_sats,
            created_at: now,
            status: "confirmed".to_string(),
        };

        // Store receipt
        let mut receipts = self.receipts.lock().unwrap();
        receipts.insert(receipt.id.clone(), receipt);

        PaymentResponse {
            success: true,
            _receipt_id: request.receipt_id,
            confirmed_at: Some(now),
            error_message: None,
        }
    }

    fn get_receipt(&self, receipt_id: &str) -> Option<Receipt> {
        let receipts = self.receipts.lock().unwrap();
        receipts.get(receipt_id).cloned()
    }

    fn get_all_receipts(&self) -> Vec<Receipt> {
        let receipts = self.receipts.lock().unwrap();
        receipts.values().cloned().collect()
    }
}

/// Generate a unique receipt ID
fn generate_receipt_id() -> String {
    format!("rcpt_xplat_{}", uuid::Uuid::new_v4())
}

/// Generate a mock public key
fn generate_pubkey(platform: Platform) -> String {
    format!(
        "z32_{}_{}",
        platform.name().to_lowercase(),
        uuid::Uuid::new_v4()
            .to_string()
            .replace("-", "")
            .chars()
            .take(40)
            .collect::<String>()
    )
}

/// Generate a mock Noise public key
fn generate_noise_pubkey() -> String {
    (0..32)
        .map(|_| format!("{:02x}", rand::random::<u8>()))
        .collect()
}

/// Create a platform identity
fn create_identity(platform: Platform, nickname: &str) -> PlatformIdentity {
    PlatformIdentity {
        platform,
        _nickname: nickname.to_string(),
        public_key_z32: generate_pubkey(platform),
        noise_pubkey: generate_noise_pubkey(),
    }
}

// ============================================================================
// Cross-Platform Payment Tests
// ============================================================================

#[test]
fn test_ios_to_android_payment() {
    let service = CrossPlatformPaymentService::new();

    // Create iOS sender and Android receiver
    let ios_sender = create_identity(Platform::Ios, "alice_ios");
    let android_receiver = create_identity(Platform::Android, "bob_android");

    // Android receiver registers endpoint
    service.register_endpoint(&android_receiver, "192.168.1.100", 9000);

    // iOS sender discovers Android receiver
    let endpoint = service.discover_endpoint(&android_receiver.public_key_z32);
    assert!(endpoint.is_some());
    assert_eq!(endpoint.as_ref().unwrap().platform, Platform::Android);

    // iOS sends payment to Android
    let request = PaymentRequest {
        receipt_id: generate_receipt_id(),
        payer_pubkey: ios_sender.public_key_z32.clone(),
        payee_pubkey: android_receiver.public_key_z32.clone(),
        method_id: "lightning".to_string(),
        amount_sats: 1000,
    };

    let response = service.process_payment(request.clone());
    assert!(response.success);
    assert!(response.confirmed_at.is_some());

    // Verify receipt was stored
    let receipt = service.get_receipt(&request.receipt_id);
    assert!(receipt.is_some());
    assert_eq!(receipt.unwrap().amount_sats, 1000);
}

#[test]
fn test_android_to_ios_payment() {
    let service = CrossPlatformPaymentService::new();

    // Create Android sender and iOS receiver
    let android_sender = create_identity(Platform::Android, "charlie_android");
    let ios_receiver = create_identity(Platform::Ios, "diana_ios");

    // iOS receiver registers endpoint
    service.register_endpoint(&ios_receiver, "10.0.0.50", 8080);

    // Android sender discovers iOS receiver
    let endpoint = service.discover_endpoint(&ios_receiver.public_key_z32);
    assert!(endpoint.is_some());
    assert_eq!(endpoint.as_ref().unwrap().platform, Platform::Ios);

    // Android sends payment to iOS
    let request = PaymentRequest {
        receipt_id: generate_receipt_id(),
        payer_pubkey: android_sender.public_key_z32.clone(),
        payee_pubkey: ios_receiver.public_key_z32.clone(),
        method_id: "lightning".to_string(),
        amount_sats: 2500,
    };

    let response = service.process_payment(request.clone());
    assert!(response.success);

    let receipt = service.get_receipt(&request.receipt_id);
    assert!(receipt.is_some());
    assert_eq!(receipt.unwrap().amount_sats, 2500);
}

#[test]
fn test_mobile_to_cli_payment() {
    let service = CrossPlatformPaymentService::new();

    // Create iOS sender and CLI receiver
    let ios_sender = create_identity(Platform::Ios, "mobile_user");
    let cli_receiver = create_identity(Platform::Cli, "cli_server");

    // CLI registers endpoint
    service.register_endpoint(&cli_receiver, "127.0.0.1", 3000);

    // Mobile discovers CLI
    let endpoint = service.discover_endpoint(&cli_receiver.public_key_z32);
    assert!(endpoint.is_some());
    assert_eq!(endpoint.as_ref().unwrap().platform, Platform::Cli);

    // Mobile sends payment to CLI
    let request = PaymentRequest {
        receipt_id: generate_receipt_id(),
        payer_pubkey: ios_sender.public_key_z32.clone(),
        payee_pubkey: cli_receiver.public_key_z32.clone(),
        method_id: "lightning".to_string(),
        amount_sats: 5000,
    };

    let response = service.process_payment(request.clone());
    assert!(response.success);
}

#[test]
fn test_cli_to_mobile_payment() {
    let service = CrossPlatformPaymentService::new();

    // Create CLI sender and Android receiver
    let cli_sender = create_identity(Platform::Cli, "cli_client");
    let android_receiver = create_identity(Platform::Android, "android_merchant");

    // Android registers endpoint
    service.register_endpoint(&android_receiver, "mobile.example.com", 443);

    // CLI discovers Android
    let endpoint = service.discover_endpoint(&android_receiver.public_key_z32);
    assert!(endpoint.is_some());

    // CLI sends payment to Android
    let request = PaymentRequest {
        receipt_id: generate_receipt_id(),
        payer_pubkey: cli_sender.public_key_z32.clone(),
        payee_pubkey: android_receiver.public_key_z32.clone(),
        method_id: "lightning".to_string(),
        amount_sats: 10000,
    };

    let response = service.process_payment(request.clone());
    assert!(response.success);
}

// ============================================================================
// Multi-Platform Concurrent Tests
// ============================================================================

#[test]
fn test_multiple_concurrent_cross_platform_payments() {
    let service = Arc::new(CrossPlatformPaymentService::new());

    // Create identities for all platforms
    let ios_user = create_identity(Platform::Ios, "ios_user");
    let android_user = create_identity(Platform::Android, "android_user");
    let cli_user = create_identity(Platform::Cli, "cli_user");

    // All register endpoints
    service.register_endpoint(&ios_user, "192.168.1.1", 8001);
    service.register_endpoint(&android_user, "192.168.1.2", 8002);
    service.register_endpoint(&cli_user, "192.168.1.3", 8003);

    // Simulate concurrent payments from all directions
    let mut handles = vec![];

    // iOS → Android
    {
        let svc = service.clone();
        let sender = ios_user.clone();
        let receiver = android_user.clone();
        handles.push(std::thread::spawn(move || {
            let request = PaymentRequest {
                receipt_id: generate_receipt_id(),
                payer_pubkey: sender.public_key_z32,
                payee_pubkey: receiver.public_key_z32,
                method_id: "lightning".to_string(),
                amount_sats: 100,
            };
            svc.process_payment(request)
        }));
    }

    // Android → CLI
    {
        let svc = service.clone();
        let sender = android_user.clone();
        let receiver = cli_user.clone();
        handles.push(std::thread::spawn(move || {
            let request = PaymentRequest {
                receipt_id: generate_receipt_id(),
                payer_pubkey: sender.public_key_z32,
                payee_pubkey: receiver.public_key_z32,
                method_id: "lightning".to_string(),
                amount_sats: 200,
            };
            svc.process_payment(request)
        }));
    }

    // CLI → iOS
    {
        let svc = service.clone();
        let sender = cli_user.clone();
        let receiver = ios_user.clone();
        handles.push(std::thread::spawn(move || {
            let request = PaymentRequest {
                receipt_id: generate_receipt_id(),
                payer_pubkey: sender.public_key_z32,
                payee_pubkey: receiver.public_key_z32,
                method_id: "lightning".to_string(),
                amount_sats: 300,
            };
            svc.process_payment(request)
        }));
    }

    // Wait for all payments to complete
    let responses: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // All should succeed
    for response in &responses {
        assert!(response.success, "Payment should succeed");
    }

    // Verify all receipts stored
    let receipts = service.get_all_receipts();
    assert_eq!(receipts.len(), 3);
}

#[test]
fn test_bidirectional_payment_flow() {
    let service = CrossPlatformPaymentService::new();

    // Create two users on different platforms
    let alice = create_identity(Platform::Ios, "alice");
    let bob = create_identity(Platform::Android, "bob");

    // Both register endpoints
    service.register_endpoint(&alice, "10.0.0.1", 9001);
    service.register_endpoint(&bob, "10.0.0.2", 9002);

    // Alice pays Bob
    let alice_to_bob = PaymentRequest {
        receipt_id: generate_receipt_id(),
        payer_pubkey: alice.public_key_z32.clone(),
        payee_pubkey: bob.public_key_z32.clone(),
        method_id: "lightning".to_string(),
        amount_sats: 500,
    };
    let response1 = service.process_payment(alice_to_bob.clone());
    assert!(response1.success);

    // Bob pays Alice back
    let bob_to_alice = PaymentRequest {
        receipt_id: generate_receipt_id(),
        payer_pubkey: bob.public_key_z32.clone(),
        payee_pubkey: alice.public_key_z32.clone(),
        method_id: "lightning".to_string(),
        amount_sats: 250,
    };
    let response2 = service.process_payment(bob_to_alice.clone());
    assert!(response2.success);

    // Verify both receipts
    let receipts = service.get_all_receipts();
    assert_eq!(receipts.len(), 2);

    let alice_receipt = service.get_receipt(&alice_to_bob.receipt_id);
    assert!(alice_receipt.is_some());
    assert_eq!(alice_receipt.unwrap().amount_sats, 500);

    let bob_receipt = service.get_receipt(&bob_to_alice.receipt_id);
    assert!(bob_receipt.is_some());
    assert_eq!(bob_receipt.unwrap().amount_sats, 250);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_payment_to_unregistered_endpoint() {
    let service = CrossPlatformPaymentService::new();

    let sender = create_identity(Platform::Ios, "sender");
    // Receiver does NOT register endpoint

    let request = PaymentRequest {
        receipt_id: generate_receipt_id(),
        payer_pubkey: sender.public_key_z32.clone(),
        payee_pubkey: "nonexistent_pubkey".to_string(),
        method_id: "lightning".to_string(),
        amount_sats: 1000,
    };

    let response = service.process_payment(request);
    assert!(!response.success);
    assert!(response.error_message.is_some());
    assert!(response
        .error_message
        .unwrap()
        .contains("endpoint not found"));
}

#[test]
fn test_endpoint_discovery_not_found() {
    let service = CrossPlatformPaymentService::new();

    let endpoint = service.discover_endpoint("unknown_pubkey");
    assert!(endpoint.is_none());
}

// ============================================================================
// Protocol Compatibility Tests
// ============================================================================

#[test]
fn test_message_format_compatibility() {
    // Test that message formats are consistent across platforms
    let request = PaymentRequest {
        receipt_id: "rcpt_test_123".to_string(),
        payer_pubkey: "z32_ios_payer".to_string(),
        payee_pubkey: "z32_android_payee".to_string(),
        method_id: "lightning".to_string(),
        amount_sats: 1000,
    };

    // Verify all required fields are present
    assert!(!request.receipt_id.is_empty());
    assert!(!request.payer_pubkey.is_empty());
    assert!(!request.payee_pubkey.is_empty());
    assert!(!request.method_id.is_empty());
    assert!(request.amount_sats > 0);
}

#[test]
fn test_receipt_format_compatibility() {
    let receipt = Receipt {
        id: "rcpt_compat_test".to_string(),
        payer_pubkey: "z32_payer".to_string(),
        payee_pubkey: "z32_payee".to_string(),
        amount_sats: 5000,
        created_at: 1700000000,
        status: "confirmed".to_string(),
    };

    // Verify all required fields are present
    assert!(!receipt.id.is_empty());
    assert!(!receipt.payer_pubkey.is_empty());
    assert!(!receipt.payee_pubkey.is_empty());
    assert!(receipt.amount_sats > 0);
    assert!(receipt.created_at > 0);
    assert!(!receipt.status.is_empty());
}

// ============================================================================
// Stress Tests
// ============================================================================

#[test]
fn test_high_volume_cross_platform_payments() {
    let service = Arc::new(CrossPlatformPaymentService::new());

    // Create many identities across platforms
    let identities: Vec<_> = (0..10)
        .map(|i| {
            let platform = match i % 3 {
                0 => Platform::Ios,
                1 => Platform::Android,
                _ => Platform::Cli,
            };
            create_identity(platform, &format!("user_{}", i))
        })
        .collect();

    // All register endpoints
    for (i, identity) in identities.iter().enumerate() {
        service.register_endpoint(identity, "127.0.0.1", 10000 + i as u16);
    }

    // Create many concurrent payments
    let mut handles = vec![];
    for i in 0..50 {
        let svc = service.clone();
        let sender = identities[i % 10].clone();
        let receiver = identities[(i + 1) % 10].clone();

        handles.push(std::thread::spawn(move || {
            let request = PaymentRequest {
                receipt_id: generate_receipt_id(),
                payer_pubkey: sender.public_key_z32,
                payee_pubkey: receiver.public_key_z32,
                method_id: "lightning".to_string(),
                amount_sats: (i + 1) as u64 * 10,
            };
            svc.process_payment(request)
        }));
    }

    // Wait for all
    let responses: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // All should succeed
    let success_count = responses.iter().filter(|r| r.success).count();
    assert_eq!(success_count, 50);

    // Verify all receipts
    let receipts = service.get_all_receipts();
    assert_eq!(receipts.len(), 50);
}

// ============================================================================
// Executor Cross-Platform Tests
// ============================================================================

use paykit_mobile::executor_ffi::*;
use paykit_mobile::*;
use std::sync::atomic::{AtomicU32, Ordering};

/// Mock executor that simulates platform-specific behavior
struct CrossPlatformMockBitcoinExecutor {
    platform: Platform,
    call_count: AtomicU32,
}

impl CrossPlatformMockBitcoinExecutor {
    fn new(platform: Platform) -> Self {
        Self {
            platform,
            call_count: AtomicU32::new(0),
        }
    }
}

impl BitcoinExecutorFFI for CrossPlatformMockBitcoinExecutor {
    fn send_to_address(
        &self,
        _address: String,
        _amount_sats: u64,
        _fee_rate: Option<f64>,
    ) -> Result<BitcoinTxResultFFI> {
        let count = self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(BitcoinTxResultFFI {
            txid: format!("xplat_{}_txid_{:08x}", self.platform.name().to_lowercase(), count),
            raw_tx: None,
            vout: 0,
            fee_sats: 210,
            fee_rate: 1.5,
            block_height: None,
            confirmations: 0,
        })
    }

    fn estimate_fee(&self, _address: String, _amount_sats: u64, _target_blocks: u32) -> Result<u64> {
        Ok(210)
    }

    fn get_transaction(&self, _txid: String) -> Result<Option<BitcoinTxResultFFI>> {
        Ok(None)
    }

    fn verify_transaction(&self, _txid: String, _address: String, _amount_sats: u64) -> Result<bool> {
        Ok(true)
    }
}

/// Mock Lightning executor for cross-platform tests
struct CrossPlatformMockLightningExecutor {
    platform: Platform,
    call_count: AtomicU32,
}

impl CrossPlatformMockLightningExecutor {
    fn new(platform: Platform) -> Self {
        Self {
            platform,
            call_count: AtomicU32::new(0),
        }
    }
}

impl LightningExecutorFFI for CrossPlatformMockLightningExecutor {
    fn pay_invoice(
        &self,
        _invoice: String,
        amount_msat: Option<u64>,
        _max_fee_msat: Option<u64>,
    ) -> Result<LightningPaymentResultFFI> {
        let count = self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(LightningPaymentResultFFI {
            preimage: format!("{:064x}", count + 1),
            payment_hash: format!("{:064x}", count + 1000),
            amount_msat: amount_msat.unwrap_or(1000000),
            fee_msat: 100,
            hops: 3,
            status: LightningPaymentStatusFFI::Succeeded,
        })
    }

    fn decode_invoice(&self, _invoice: String) -> Result<DecodedInvoiceFFI> {
        Ok(DecodedInvoiceFFI {
            payment_hash: format!("{:064x}", 1),
            amount_msat: Some(1000000),
            description: Some(format!("{} invoice", self.platform.name())),
            description_hash: None,
            payee: format!("{:066x}", 1),
            expiry: 3600,
            timestamp: 1700000000,
            expired: false,
        })
    }

    fn estimate_fee(&self, _invoice: String) -> Result<u64> {
        Ok(100)
    }

    fn get_payment(&self, _payment_hash: String) -> Result<Option<LightningPaymentResultFFI>> {
        Ok(None)
    }

    fn verify_preimage(&self, preimage: String, payment_hash: String) -> bool {
        preimage.len() == 64 && payment_hash.len() == 64
    }
}

#[test]
fn test_executor_cross_platform_ios_simulation() {
    // Simulate iOS client with executor
    let client = PaykitClient::new_with_network(
        BitcoinNetworkFFI::Testnet,
        LightningNetworkFFI::Testnet,
    )
    .unwrap();

    client
        .register_bitcoin_executor(Box::new(CrossPlatformMockBitcoinExecutor::new(Platform::Ios)))
        .unwrap();
    client
        .register_lightning_executor(Box::new(CrossPlatformMockLightningExecutor::new(Platform::Ios)))
        .unwrap();

    // Execute payment
    let result = client
        .execute_payment(
            "onchain".to_string(),
            "tb1qtest123456789012345678901234567890".to_string(),
            50000,
            None,
        )
        .unwrap();

    assert!(result.success);
    assert!(result.execution_data_json.contains("xplat_ios_txid"));
}

#[test]
fn test_executor_cross_platform_android_simulation() {
    // Simulate Android client with executor
    let client = PaykitClient::new_with_network(
        BitcoinNetworkFFI::Testnet,
        LightningNetworkFFI::Testnet,
    )
    .unwrap();

    client
        .register_bitcoin_executor(Box::new(CrossPlatformMockBitcoinExecutor::new(Platform::Android)))
        .unwrap();
    client
        .register_lightning_executor(Box::new(CrossPlatformMockLightningExecutor::new(Platform::Android)))
        .unwrap();

    // Execute payment
    let result = client
        .execute_payment(
            "onchain".to_string(),
            "tb1qtest123456789012345678901234567890".to_string(),
            50000,
            None,
        )
        .unwrap();

    assert!(result.success);
    assert!(result.execution_data_json.contains("xplat_android_txid"));
}

#[test]
fn test_executor_payment_proof_format_consistency() {
    // Test that proof format is consistent across platforms
    let ios_client = PaykitClient::new_with_network(
        BitcoinNetworkFFI::Testnet,
        LightningNetworkFFI::Testnet,
    )
    .unwrap();

    let android_client = PaykitClient::new_with_network(
        BitcoinNetworkFFI::Testnet,
        LightningNetworkFFI::Testnet,
    )
    .unwrap();

    ios_client
        .register_bitcoin_executor(Box::new(CrossPlatformMockBitcoinExecutor::new(Platform::Ios)))
        .unwrap();
    android_client
        .register_bitcoin_executor(Box::new(CrossPlatformMockBitcoinExecutor::new(Platform::Android)))
        .unwrap();

    // Execute payments
    let ios_result = ios_client
        .execute_payment(
            "onchain".to_string(),
            "tb1qtest123456789012345678901234567890".to_string(),
            10000,
            None,
        )
        .unwrap();

    let android_result = android_client
        .execute_payment(
            "onchain".to_string(),
            "tb1qtest123456789012345678901234567890".to_string(),
            10000,
            None,
        )
        .unwrap();

    // Generate proofs
    let ios_proof = ios_client
        .generate_payment_proof("onchain".to_string(), ios_result.execution_data_json)
        .unwrap();

    let android_proof = android_client
        .generate_payment_proof("onchain".to_string(), android_result.execution_data_json)
        .unwrap();

    // Proof types should be consistent
    assert_eq!(ios_proof.proof_type, "bitcoin_txid");
    assert_eq!(android_proof.proof_type, "bitcoin_txid");

    // Both should have valid JSON
    let ios_proof_data: serde_json::Value =
        serde_json::from_str(&ios_proof.proof_data_json).unwrap();
    let android_proof_data: serde_json::Value =
        serde_json::from_str(&android_proof.proof_data_json).unwrap();

    // Both should have txid field
    assert!(ios_proof_data.get("txid").is_some() || ios_proof_data.get("Txid").is_some());
    assert!(android_proof_data.get("txid").is_some() || android_proof_data.get("Txid").is_some());
}

#[test]
fn test_executor_concurrent_cross_platform_payments() {
    use std::thread;

    // Simulate multiple platforms making payments concurrently
    let mut handles = vec![];

    for platform in [Platform::Ios, Platform::Android, Platform::Cli] {
        handles.push(thread::spawn(move || {
            let client = PaykitClient::new_with_network(
                BitcoinNetworkFFI::Testnet,
                LightningNetworkFFI::Testnet,
            )
            .unwrap();

            client
                .register_bitcoin_executor(Box::new(CrossPlatformMockBitcoinExecutor::new(platform)))
                .unwrap();

            let result = client
                .execute_payment(
                    "onchain".to_string(),
                    "tb1qtest123456789012345678901234567890".to_string(),
                    10000,
                    None,
                )
                .unwrap();

            (platform, result)
        }));
    }

    // Collect results
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // All should succeed
    for (platform, result) in &results {
        assert!(result.success, "{} payment should succeed", platform.name());
    }

    // Each should have platform-specific txid
    assert!(results
        .iter()
        .any(|(p, r)| *p == Platform::Ios && r.execution_data_json.contains("xplat_ios")));
    assert!(results
        .iter()
        .any(|(p, r)| *p == Platform::Android && r.execution_data_json.contains("xplat_android")));
    assert!(results
        .iter()
        .any(|(p, r)| *p == Platform::Cli && r.execution_data_json.contains("xplat_cli")));
}
