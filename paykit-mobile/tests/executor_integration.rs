//! Executor Integration Tests
//!
//! Comprehensive tests for the Bitkit executor integration, covering:
//! - Executor registration
//! - Payment execution flows
//! - Proof generation
//! - Error handling
//! - Network configuration
//! - Thread safety
//!
//! These tests simulate real-world usage patterns from Bitkit iOS/Android.

use paykit_mobile::executor_ffi::*;
use paykit_mobile::*;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

// ============================================================================
// Mock Executor Implementations
// ============================================================================

/// A configurable mock Bitcoin executor for testing various scenarios.
struct MockBitcoinExecutor {
    /// Track number of send calls
    send_count: AtomicU32,
    /// Track total amount sent
    total_sent: AtomicU64,
    /// Whether to simulate failures
    should_fail: AtomicBool,
    /// Failure message to return
    failure_message: String,
    /// Simulated confirmations
    confirmations: u64,
}

impl MockBitcoinExecutor {
    fn new() -> Self {
        Self {
            send_count: AtomicU32::new(0),
            total_sent: AtomicU64::new(0),
            should_fail: AtomicBool::new(false),
            failure_message: "Mock failure".to_string(),
            confirmations: 0,
        }
    }

    fn with_confirmations(confirmations: u64) -> Self {
        Self {
            confirmations,
            ..Self::new()
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            should_fail: AtomicBool::new(true),
            failure_message: message.to_string(),
            ..Self::new()
        }
    }

    fn get_send_count(&self) -> u32 {
        self.send_count.load(Ordering::SeqCst)
    }

    fn get_total_sent(&self) -> u64 {
        self.total_sent.load(Ordering::SeqCst)
    }
}

impl BitcoinExecutorFFI for MockBitcoinExecutor {
    fn send_to_address(
        &self,
        address: String,
        amount_sats: u64,
        fee_rate: Option<f64>,
    ) -> Result<BitcoinTxResultFFI> {
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }

        let count = self.send_count.fetch_add(1, Ordering::SeqCst) + 1;
        self.total_sent.fetch_add(amount_sats, Ordering::SeqCst);

        Ok(BitcoinTxResultFFI {
            txid: format!("txid_{:08x}_{}", count, &address[..8.min(address.len())]),
            raw_tx: None,
            vout: 0,
            fee_sats: (fee_rate.unwrap_or(1.5) * 140.0) as u64,
            fee_rate: fee_rate.unwrap_or(1.5),
            block_height: if self.confirmations > 0 {
                Some(800000)
            } else {
                None
            },
            confirmations: self.confirmations,
        })
    }

    fn estimate_fee(
        &self,
        _address: String,
        _amount_sats: u64,
        target_blocks: u32,
    ) -> Result<u64> {
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }
        // Higher fee for faster confirmation
        Ok(210 * (10 - target_blocks.min(9)) as u64)
    }

    fn get_transaction(&self, txid: String) -> Result<Option<BitcoinTxResultFFI>> {
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }

        if txid.starts_with("txid_") {
            Ok(Some(BitcoinTxResultFFI {
                txid,
                raw_tx: None,
                vout: 0,
                fee_sats: 210,
                fee_rate: 1.5,
                block_height: Some(800000),
                confirmations: self.confirmations,
            }))
        } else {
            Ok(None)
        }
    }

    fn verify_transaction(
        &self,
        txid: String,
        _address: String,
        _amount_sats: u64,
    ) -> Result<bool> {
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }
        Ok(txid.starts_with("txid_"))
    }
}

/// A configurable mock Lightning executor for testing various scenarios.
struct MockLightningExecutor {
    /// Track number of payment calls
    pay_count: AtomicU32,
    /// Track total amount paid (msat)
    total_paid_msat: AtomicU64,
    /// Whether to simulate failures
    should_fail: AtomicBool,
    /// Failure message to return
    failure_message: String,
    /// Simulated payment status
    payment_status: LightningPaymentStatusFFI,
}

impl MockLightningExecutor {
    fn new() -> Self {
        Self {
            pay_count: AtomicU32::new(0),
            total_paid_msat: AtomicU64::new(0),
            should_fail: AtomicBool::new(false),
            failure_message: "Mock failure".to_string(),
            payment_status: LightningPaymentStatusFFI::Succeeded,
        }
    }

    fn with_status(status: LightningPaymentStatusFFI) -> Self {
        Self {
            payment_status: status,
            ..Self::new()
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            should_fail: AtomicBool::new(true),
            failure_message: message.to_string(),
            ..Self::new()
        }
    }

    #[allow(dead_code)]
    fn get_pay_count(&self) -> u32 {
        self.pay_count.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    fn get_total_paid_msat(&self) -> u64 {
        self.total_paid_msat.load(Ordering::SeqCst)
    }
}

impl LightningExecutorFFI for MockLightningExecutor {
    fn pay_invoice(
        &self,
        invoice: String,
        amount_msat: Option<u64>,
        _max_fee_msat: Option<u64>,
    ) -> Result<LightningPaymentResultFFI> {
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }

        let count = self.pay_count.fetch_add(1, Ordering::SeqCst) + 1;
        let amount = amount_msat.unwrap_or(1000000);
        self.total_paid_msat.fetch_add(amount, Ordering::SeqCst);

        // Generate deterministic preimage and hash
        let preimage = format!("{:064x}", count);
        let payment_hash = format!("{:064x}", count + 1000);

        Ok(LightningPaymentResultFFI {
            preimage,
            payment_hash,
            amount_msat: amount,
            fee_msat: 100,
            hops: 3,
            status: self.payment_status.clone(),
        })
    }

    fn decode_invoice(&self, invoice: String) -> Result<DecodedInvoiceFFI> {
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }

        // Extract amount from invoice if present (mock parsing)
        let amount_msat = if invoice.contains("1000") {
            Some(1000000u64)
        } else {
            Some(100000u64)
        };

        Ok(DecodedInvoiceFFI {
            payment_hash: format!("{:064x}", invoice.len()),
            amount_msat,
            description: Some("Test invoice".to_string()),
            description_hash: None,
            payee: "mock_payee_pubkey".to_string(),
            expiry: 3600,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            expired: false,
        })
    }

    fn estimate_fee(&self, _invoice: String) -> Result<u64> {
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }
        Ok(100) // 100 msat base fee
    }

    fn get_payment(&self, payment_hash: String) -> Result<Option<LightningPaymentResultFFI>> {
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }

        if payment_hash.len() == 64 {
            Ok(Some(LightningPaymentResultFFI {
                preimage: format!("{:064x}", 1),
                payment_hash,
                amount_msat: 1000000,
                fee_msat: 100,
                hops: 3,
                status: self.payment_status.clone(),
            }))
        } else {
            Ok(None)
        }
    }

    fn verify_preimage(&self, preimage: String, payment_hash: String) -> bool {
        // Simple mock verification - just check lengths
        preimage.len() == 64 && payment_hash.len() == 64
    }
}

// ============================================================================
// Network Configuration Tests
// ============================================================================

#[test]
fn test_create_client_mainnet() {
    let client = PaykitClient::new().unwrap();
    assert_eq!(client.bitcoin_network(), BitcoinNetworkFFI::Mainnet);
    assert_eq!(client.lightning_network(), LightningNetworkFFI::Mainnet);
}

#[test]
fn test_create_client_testnet() {
    let client = PaykitClient::new_with_network(
        BitcoinNetworkFFI::Testnet,
        LightningNetworkFFI::Testnet,
    )
    .unwrap();

    assert_eq!(client.bitcoin_network(), BitcoinNetworkFFI::Testnet);
    assert_eq!(client.lightning_network(), LightningNetworkFFI::Testnet);
}

#[test]
fn test_create_client_regtest() {
    let client = PaykitClient::new_with_network(
        BitcoinNetworkFFI::Regtest,
        LightningNetworkFFI::Regtest,
    )
    .unwrap();

    assert_eq!(client.bitcoin_network(), BitcoinNetworkFFI::Regtest);
    assert_eq!(client.lightning_network(), LightningNetworkFFI::Regtest);
}

#[test]
fn test_mixed_network_configuration() {
    // This is unusual but should work (e.g., testnet Bitcoin with mainnet Lightning)
    let client = PaykitClient::new_with_network(
        BitcoinNetworkFFI::Testnet,
        LightningNetworkFFI::Mainnet,
    )
    .unwrap();

    assert_eq!(client.bitcoin_network(), BitcoinNetworkFFI::Testnet);
    assert_eq!(client.lightning_network(), LightningNetworkFFI::Mainnet);
}

// ============================================================================
// Executor Registration Tests
// ============================================================================

#[test]
fn test_register_bitcoin_executor() {
    let client = PaykitClient::new().unwrap();
    let executor = Box::new(MockBitcoinExecutor::new());

    client.register_bitcoin_executor(executor).unwrap();
    assert!(client.has_bitcoin_executor());
}

#[test]
fn test_register_lightning_executor() {
    let client = PaykitClient::new().unwrap();
    let executor = Box::new(MockLightningExecutor::new());

    client.register_lightning_executor(executor).unwrap();
    assert!(client.has_lightning_executor());
}

#[test]
fn test_register_both_executors() {
    let client = PaykitClient::new().unwrap();

    client
        .register_bitcoin_executor(Box::new(MockBitcoinExecutor::new()))
        .unwrap();
    client
        .register_lightning_executor(Box::new(MockLightningExecutor::new()))
        .unwrap();

    assert!(client.has_bitcoin_executor());
    assert!(client.has_lightning_executor());
}

#[test]
fn test_replace_executor() {
    let client = PaykitClient::new().unwrap();

    // Register first executor
    let executor1 = Box::new(MockBitcoinExecutor::new());
    client.register_bitcoin_executor(executor1).unwrap();

    // Replace with second executor
    let executor2 = Box::new(MockBitcoinExecutor::with_confirmations(6));
    client.register_bitcoin_executor(executor2).unwrap();

    // Should still have executor
    assert!(client.has_bitcoin_executor());
}

// ============================================================================
// Bitcoin Payment Execution Tests
// ============================================================================

#[test]
fn test_execute_bitcoin_payment_success() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    client
        .register_bitcoin_executor(Box::new(MockBitcoinExecutor::new()))
        .unwrap();

    let result = client
        .execute_payment(
            "onchain".to_string(),
            "tb1qtest12345678901234567890123456789012345".to_string(),
            50000,
            None,
        )
        .unwrap();

    assert!(result.success);
    assert_eq!(result.method_id, "onchain");
    assert_eq!(result.amount_sats, 50000);
    assert!(result.execution_data_json.contains("txid_"));
    assert!(result.error.is_none());
}

#[test]
fn test_execute_bitcoin_payment_with_fee_rate() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    client
        .register_bitcoin_executor(Box::new(MockBitcoinExecutor::new()))
        .unwrap();

    let metadata = serde_json::json!({
        "fee_rate": 5.0
    });

    let result = client
        .execute_payment(
            "onchain".to_string(),
            "tb1qtest12345678901234567890123456789012345".to_string(),
            50000,
            Some(serde_json::to_string(&metadata).unwrap()),
        )
        .unwrap();

    assert!(result.success);
    // Fee should be ~700 sats (5.0 * 140)
    assert!(result.execution_data_json.contains("fee_sats"));
}

#[test]
fn test_execute_bitcoin_payment_failure() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    client
        .register_bitcoin_executor(Box::new(MockBitcoinExecutor::failing(
            "Insufficient funds",
        )))
        .unwrap();

    let result = client.execute_payment(
        "onchain".to_string(),
        "tb1qtest12345678901234567890123456789012345".to_string(),
        50000,
        None,
    );

    // The payment should fail but not panic
    assert!(result.is_err() || !result.unwrap().success);
}

#[test]
fn test_execute_bitcoin_payment_dust_limit() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    client
        .register_bitcoin_executor(Box::new(MockBitcoinExecutor::new()))
        .unwrap();

    // Try to send below dust limit (546 sats)
    let result = client.execute_payment(
        "onchain".to_string(),
        "tb1qtest12345678901234567890123456789012345".to_string(),
        100, // Below dust
        None,
    );

    // Should fail validation
    assert!(result.is_err());
}

// ============================================================================
// Lightning Payment Execution Tests
// ============================================================================

#[test]
fn test_execute_lightning_payment_success() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    client
        .register_lightning_executor(Box::new(MockLightningExecutor::new()))
        .unwrap();

    // Use a realistic-length invoice
    let invoice = format!("lntb1000n1p{}", "0".repeat(200));

    let result = client
        .execute_payment("lightning".to_string(), invoice, 1000, None)
        .unwrap();

    assert!(result.success);
    assert_eq!(result.method_id, "lightning");
    assert!(result.execution_data_json.contains("preimage"));
    assert!(result.error.is_none());
}

#[test]
fn test_execute_lightning_payment_with_amount() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    client
        .register_lightning_executor(Box::new(MockLightningExecutor::new()))
        .unwrap();

    let invoice = format!("lntb1p{}", "0".repeat(200)); // Zero-amount invoice

    let result = client
        .execute_payment("lightning".to_string(), invoice, 5000, None)
        .unwrap();

    assert!(result.success);
}

#[test]
fn test_execute_lightning_payment_failure() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    client
        .register_lightning_executor(Box::new(MockLightningExecutor::failing("No route found")))
        .unwrap();

    let invoice = format!("lntb1000n1p{}", "0".repeat(200));

    let result = client.execute_payment("lightning".to_string(), invoice, 1000, None);

    assert!(result.is_err() || !result.unwrap().success);
}

#[test]
fn test_execute_lightning_payment_pending() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    client
        .register_lightning_executor(Box::new(MockLightningExecutor::with_status(
            LightningPaymentStatusFFI::Pending,
        )))
        .unwrap();

    let invoice = format!("lntb1000n1p{}", "0".repeat(200));

    let result = client
        .execute_payment("lightning".to_string(), invoice, 1000, None)
        .unwrap();

    // Payment initiated - the executor returned a result (pending is still a valid result)
    // The execution_data should contain status information
    assert!(result.execution_data_json.len() > 2); // Has some data
}

// ============================================================================
// Proof Generation Tests
// ============================================================================

#[test]
fn test_generate_bitcoin_proof() {
    let client = PaykitClient::new().unwrap();

    let execution_data = serde_json::json!({
        "txid": "abc123def456789012345678901234567890123456789012345678901234",
        "address": "bc1qtest",
        "amount_sats": 10000,
        "vout": 0,
        "fee_sats": 210
    });

    let proof = client
        .generate_payment_proof(
            "onchain".to_string(),
            serde_json::to_string(&execution_data).unwrap(),
        )
        .unwrap();

    assert_eq!(proof.proof_type, "bitcoin_txid");
    assert!(proof.proof_data_json.contains("abc123def456"));
}

#[test]
fn test_generate_lightning_proof() {
    let client = PaykitClient::new().unwrap();

    let execution_data = serde_json::json!({
        "preimage": "0000000000000000000000000000000000000000000000000000000000000001",
        "payment_hash": "0000000000000000000000000000000000000000000000000000000000000002",
        "invoice": "lnbc1000n1...",
        "amount_msat": 1000000
    });

    let proof = client
        .generate_payment_proof(
            "lightning".to_string(),
            serde_json::to_string(&execution_data).unwrap(),
        )
        .unwrap();

    assert_eq!(proof.proof_type, "lightning_preimage");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_execute_payment_unknown_method() {
    let client = PaykitClient::new().unwrap();

    let result = client.execute_payment(
        "unknown_method".to_string(),
        "some_endpoint".to_string(),
        1000,
        None,
    );

    assert!(result.is_err());
    match result {
        Err(PaykitMobileError::NotFound { message }) => {
            assert!(message.contains("unknown_method"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[test]
fn test_generate_proof_invalid_json() {
    let client = PaykitClient::new().unwrap();

    let result = client.generate_payment_proof("onchain".to_string(), "not valid json".to_string());

    assert!(result.is_err());
    match result {
        Err(PaykitMobileError::Serialization { .. }) => {}
        _ => panic!("Expected Serialization error"),
    }
}

#[test]
fn test_generate_proof_unknown_method() {
    let client = PaykitClient::new().unwrap();

    let result = client.generate_payment_proof("unknown".to_string(), "{}".to_string());

    assert!(result.is_err());
    match result {
        Err(PaykitMobileError::NotFound { .. }) => {}
        _ => panic!("Expected NotFound error"),
    }
}

// ============================================================================
// Full Payment Flow Tests (E2E)
// ============================================================================

#[test]
fn test_full_bitcoin_payment_flow() {
    // Simulate complete Bitkit integration flow
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    // Step 1: Register executor (Bitkit wallet)
    client
        .register_bitcoin_executor(Box::new(MockBitcoinExecutor::with_confirmations(1)))
        .unwrap();

    // Step 2: Validate the endpoint
    let address = "tb1qtest12345678901234567890123456789012345";
    let is_valid = client.validate_endpoint("onchain".to_string(), address.to_string());
    assert!(is_valid.is_ok());

    // Step 3: Execute payment
    let result = client
        .execute_payment("onchain".to_string(), address.to_string(), 100000, None)
        .unwrap();

    assert!(result.success);

    // Step 4: Generate proof
    let proof = client
        .generate_payment_proof("onchain".to_string(), result.execution_data_json.clone())
        .unwrap();

    assert_eq!(proof.proof_type, "bitcoin_txid");

    // Step 5: Verify we can parse the execution data
    let execution_data: serde_json::Value =
        serde_json::from_str(&result.execution_data_json).unwrap();
    assert!(execution_data.get("txid").is_some());
    assert!(execution_data.get("fee_sats").is_some());
}

#[test]
fn test_full_lightning_payment_flow() {
    // Simulate complete Bitkit integration flow
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    // Step 1: Register executor (Bitkit Lightning node)
    client
        .register_lightning_executor(Box::new(MockLightningExecutor::new()))
        .unwrap();

    // Step 2: Create a realistic invoice
    let invoice = format!("lntb1000n1p{}", "0".repeat(200));

    // Step 3: Execute payment
    let result = client
        .execute_payment("lightning".to_string(), invoice.clone(), 1000, None)
        .unwrap();

    assert!(result.success);

    // Step 4: Generate proof
    let proof = client
        .generate_payment_proof("lightning".to_string(), result.execution_data_json.clone())
        .unwrap();

    assert_eq!(proof.proof_type, "lightning_preimage");

    // Step 5: Verify we can parse the execution data
    let execution_data: serde_json::Value =
        serde_json::from_str(&result.execution_data_json).unwrap();
    assert!(execution_data.get("preimage").is_some());
    assert!(execution_data.get("payment_hash").is_some());
}

#[test]
fn test_multiple_payments_sequential() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    let executor = Arc::new(MockBitcoinExecutor::new());

    // Use a simple wrapper since we can't clone Arc directly
    struct ExecutorWrapper(Arc<MockBitcoinExecutor>);

    impl BitcoinExecutorFFI for ExecutorWrapper {
        fn send_to_address(
            &self,
            address: String,
            amount_sats: u64,
            fee_rate: Option<f64>,
        ) -> Result<BitcoinTxResultFFI> {
            self.0.send_to_address(address, amount_sats, fee_rate)
        }

        fn estimate_fee(
            &self,
            address: String,
            amount_sats: u64,
            target_blocks: u32,
        ) -> Result<u64> {
            self.0.estimate_fee(address, amount_sats, target_blocks)
        }

        fn get_transaction(&self, txid: String) -> Result<Option<BitcoinTxResultFFI>> {
            self.0.get_transaction(txid)
        }

        fn verify_transaction(
            &self,
            txid: String,
            address: String,
            amount_sats: u64,
        ) -> Result<bool> {
            self.0.verify_transaction(txid, address, amount_sats)
        }
    }

    client
        .register_bitcoin_executor(Box::new(ExecutorWrapper(executor.clone())))
        .unwrap();

    // Execute multiple payments
    for i in 0..5 {
        let result = client
            .execute_payment(
                "onchain".to_string(),
                "tb1qtest12345678901234567890123456789012345".to_string(),
                10000 * (i + 1),
                None,
            )
            .unwrap();
        assert!(result.success);
    }

    // Verify all payments were tracked
    assert_eq!(executor.get_send_count(), 5);
    assert_eq!(executor.get_total_sent(), 10000 + 20000 + 30000 + 40000 + 50000);
}

// ============================================================================
// Thread Safety Tests
// ============================================================================

#[test]
fn test_concurrent_executor_registration() {
    use std::thread;

    let client = Arc::new(
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap(),
    );

    let mut handles = vec![];

    // Spawn multiple threads that register executors
    for _ in 0..10 {
        let client_clone = client.clone();
        handles.push(thread::spawn(move || {
            client_clone
                .register_bitcoin_executor(Box::new(MockBitcoinExecutor::new()))
                .unwrap();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Should still have a valid executor
    assert!(client.has_bitcoin_executor());
}

#[test]
fn test_executor_called_from_multiple_threads() {
    use std::thread;

    let client = Arc::new(
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap(),
    );

    client
        .register_bitcoin_executor(Box::new(MockBitcoinExecutor::new()))
        .unwrap();

    let mut handles = vec![];

    // Spawn multiple threads that execute payments
    for i in 0..5 {
        let client_clone = client.clone();
        handles.push(thread::spawn(move || {
            let result = client_clone.execute_payment(
                "onchain".to_string(),
                "tb1qtest12345678901234567890123456789012345".to_string(),
                10000 + i * 1000,
                None,
            );
            assert!(result.is_ok());
            assert!(result.unwrap().success);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

// ============================================================================
// Result Type Tests
// ============================================================================

#[test]
fn test_payment_execution_result_structure() {
    let result = PaymentExecutionResult {
        execution_id: "exec_12345678".to_string(),
        method_id: "lightning".to_string(),
        endpoint: "lnbc...".to_string(),
        amount_sats: 1000,
        success: true,
        executed_at: 1700000000,
        execution_data_json: r#"{"preimage":"abc"}"#.to_string(),
        error: None,
    };

    assert!(result.execution_id.starts_with("exec_"));
    assert_eq!(result.method_id, "lightning");
    assert!(result.success);
    assert!(result.error.is_none());
}

#[test]
fn test_payment_execution_result_with_error() {
    let result = PaymentExecutionResult {
        execution_id: "exec_12345678".to_string(),
        method_id: "onchain".to_string(),
        endpoint: "bc1q...".to_string(),
        amount_sats: 1000,
        success: false,
        executed_at: 1700000000,
        execution_data_json: "{}".to_string(),
        error: Some("Insufficient funds".to_string()),
    };

    assert!(!result.success);
    assert!(result.error.is_some());
    assert_eq!(result.error.unwrap(), "Insufficient funds");
}

#[test]
fn test_payment_proof_result_bitcoin() {
    let proof = PaymentProofResult {
        proof_type: "bitcoin_txid".to_string(),
        proof_data_json: r#"{"txid":"abc123","confirmations":6}"#.to_string(),
    };

    assert_eq!(proof.proof_type, "bitcoin_txid");
    assert!(proof.proof_data_json.contains("abc123"));
}

#[test]
fn test_payment_proof_result_lightning() {
    let proof = PaymentProofResult {
        proof_type: "lightning_preimage".to_string(),
        proof_data_json: r#"{"preimage":"0000...","payment_hash":"1111..."}"#.to_string(),
    };

    assert_eq!(proof.proof_type, "lightning_preimage");
    assert!(proof.proof_data_json.contains("preimage"));
}
