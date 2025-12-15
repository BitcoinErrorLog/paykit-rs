//! Payment Execution E2E Tests
//!
//! End-to-end tests for payment execution flows covering:
//! - Full on-chain payment flow
//! - Full Lightning payment flow
//! - Error scenarios
//! - Proof generation

use paykit_mobile::executor_ffi::*;
use paykit_mobile::*;
use std::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// Mock Executor Implementations
// ============================================================================

/// Mock Bitcoin executor that tracks calls and simulates real behavior.
struct MockBitcoinExecutorE2E {
    send_count: AtomicU32,
    should_fail: bool,
    failure_message: String,
}

impl MockBitcoinExecutorE2E {
    fn new() -> Self {
        Self {
            send_count: AtomicU32::new(0),
            should_fail: false,
            failure_message: String::new(),
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            send_count: AtomicU32::new(0),
            should_fail: true,
            failure_message: message.to_string(),
        }
    }
}

impl BitcoinExecutorFFI for MockBitcoinExecutorE2E {
    fn send_to_address(
        &self,
        address: String,
        amount_sats: u64,
        fee_rate: Option<f64>,
    ) -> Result<BitcoinTxResultFFI> {
        if self.should_fail {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }

        let count = self.send_count.fetch_add(1, Ordering::SeqCst);
        let fee = (fee_rate.unwrap_or(1.5) * 140.0) as u64;

        Ok(BitcoinTxResultFFI {
            txid: format!(
                "e2e_txid_{:08x}_{}",
                count,
                &address[..8.min(address.len())]
            ),
            raw_tx: Some(format!("0100000001...{:016x}...000000", amount_sats)),
            vout: 0,
            fee_sats: fee,
            fee_rate: fee_rate.unwrap_or(1.5),
            block_height: None,
            confirmations: 0,
        })
    }

    fn estimate_fee(&self, _address: String, _amount_sats: u64, target_blocks: u32) -> Result<u64> {
        if self.should_fail {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }
        // Higher fee for faster confirmation
        Ok(150 * (10 - target_blocks.min(9)) as u64)
    }

    fn get_transaction(&self, txid: String) -> Result<Option<BitcoinTxResultFFI>> {
        if self.should_fail {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }

        if txid.starts_with("e2e_txid_") {
            Ok(Some(BitcoinTxResultFFI {
                txid,
                raw_tx: None,
                vout: 0,
                fee_sats: 210,
                fee_rate: 1.5,
                block_height: Some(800001),
                confirmations: 1,
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
        if self.should_fail {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }
        Ok(txid.starts_with("e2e_txid_"))
    }
}

/// Mock Lightning executor for E2E tests.
struct MockLightningExecutorE2E {
    pay_count: AtomicU32,
    should_fail: bool,
    failure_message: String,
}

impl MockLightningExecutorE2E {
    fn new() -> Self {
        Self {
            pay_count: AtomicU32::new(0),
            should_fail: false,
            failure_message: String::new(),
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            pay_count: AtomicU32::new(0),
            should_fail: true,
            failure_message: message.to_string(),
        }
    }
}

impl LightningExecutorFFI for MockLightningExecutorE2E {
    fn pay_invoice(
        &self,
        _invoice: String,
        amount_msat: Option<u64>,
        _max_fee_msat: Option<u64>,
    ) -> Result<LightningPaymentResultFFI> {
        if self.should_fail {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }

        let count = self.pay_count.fetch_add(1, Ordering::SeqCst);
        let amount = amount_msat.unwrap_or(1000000);

        Ok(LightningPaymentResultFFI {
            preimage: format!("{:064x}", count + 1),
            payment_hash: format!("{:064x}", count + 1000),
            amount_msat: amount,
            fee_msat: 100,
            hops: 3,
            status: LightningPaymentStatusFFI::Succeeded,
        })
    }

    fn decode_invoice(&self, invoice: String) -> Result<DecodedInvoiceFFI> {
        if self.should_fail {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }

        // Parse amount from invoice prefix if present
        let amount_msat = if invoice.contains("1000n") {
            Some(1000000u64)
        } else {
            Some(100000u64)
        };

        Ok(DecodedInvoiceFFI {
            payment_hash: format!("{:064x}", invoice.len()),
            amount_msat,
            description: Some("E2E Test Invoice".to_string()),
            description_hash: None,
            payee: format!("{:066x}", 1),
            expiry: 3600,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            expired: false,
        })
    }

    fn estimate_fee(&self, _invoice: String) -> Result<u64> {
        if self.should_fail {
            return Err(PaykitMobileError::Transport {
                message: self.failure_message.clone(),
            });
        }
        Ok(100) // 100 msat
    }

    fn get_payment(&self, payment_hash: String) -> Result<Option<LightningPaymentResultFFI>> {
        if self.should_fail {
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
                status: LightningPaymentStatusFFI::Succeeded,
            }))
        } else {
            Ok(None)
        }
    }

    fn verify_preimage(&self, preimage: String, payment_hash: String) -> bool {
        preimage.len() == 64 && payment_hash.len() == 64
    }
}

// ============================================================================
// Full On-Chain Payment Flow Tests
// ============================================================================

#[test]
fn test_e2e_onchain_payment_full_flow() {
    // Step 1: Create client with testnet
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    // Step 2: Register Bitcoin executor
    let executor = Box::new(MockBitcoinExecutorE2E::new());
    client.register_bitcoin_executor(executor).unwrap();
    assert!(client.has_bitcoin_executor());

    // Step 3: Validate endpoint
    let address = "tb1qe2e0test1234567890abcdefghijklmnopqrs";
    let _validation = client.validate_endpoint("onchain".to_string(), address.to_string());
    // Note: validation may fail for mock address, that's expected

    // Step 4: Execute payment
    let result = client
        .execute_payment("onchain".to_string(), address.to_string(), 50000, None)
        .unwrap();

    // Step 5: Verify execution result
    assert!(result.success);
    assert_eq!(result.method_id, "onchain");
    assert_eq!(result.amount_sats, 50000);
    assert!(result.execution_data_json.contains("e2e_txid_"));

    // Step 6: Parse execution data
    let execution_data: serde_json::Value =
        serde_json::from_str(&result.execution_data_json).unwrap();
    assert!(execution_data.get("txid").is_some());
    assert!(execution_data.get("fee_sats").is_some());

    // Step 7: Generate proof
    let proof = client
        .generate_payment_proof("onchain".to_string(), result.execution_data_json.clone())
        .unwrap();

    assert_eq!(proof.proof_type, "bitcoin_txid");
    assert!(proof.proof_data_json.contains("e2e_txid_"));
}

#[test]
fn test_e2e_onchain_payment_with_custom_fee() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    client
        .register_bitcoin_executor(Box::new(MockBitcoinExecutorE2E::new()))
        .unwrap();

    // Execute with custom fee rate
    let metadata = serde_json::json!({
        "fee_rate": 10.0
    });

    let result = client
        .execute_payment(
            "onchain".to_string(),
            "tb1qtest123456789012345678901234567890".to_string(),
            100000,
            Some(serde_json::to_string(&metadata).unwrap()),
        )
        .unwrap();

    assert!(result.success);

    // Verify fee is applied (10.0 sat/vB * 140 vB ≈ 1400 sats)
    let execution_data: serde_json::Value =
        serde_json::from_str(&result.execution_data_json).unwrap();
    let fee_sats = execution_data
        .get("fee_sats")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(fee_sats > 1000); // Should be higher with 10 sat/vB
}

// ============================================================================
// Full Lightning Payment Flow Tests
// ============================================================================

#[test]
fn test_e2e_lightning_payment_full_flow() {
    // Step 1: Create client
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    // Step 2: Register Lightning executor
    let executor = Box::new(MockLightningExecutorE2E::new());
    client.register_lightning_executor(executor).unwrap();
    assert!(client.has_lightning_executor());

    // Step 3: Create realistic invoice
    let invoice = format!("lntb1000n1p{}", "0".repeat(200));

    // Step 4: Execute payment
    let result = client
        .execute_payment("lightning".to_string(), invoice.clone(), 1000, None)
        .unwrap();

    // Step 5: Verify execution result
    assert!(result.success);
    assert_eq!(result.method_id, "lightning");
    assert!(result.execution_data_json.len() > 10);

    // Step 6: Parse execution data
    let execution_data: serde_json::Value =
        serde_json::from_str(&result.execution_data_json).unwrap();
    assert!(execution_data.get("preimage").is_some());
    assert!(execution_data.get("payment_hash").is_some());

    // Step 7: Generate proof
    let proof = client
        .generate_payment_proof("lightning".to_string(), result.execution_data_json.clone())
        .unwrap();

    assert_eq!(proof.proof_type, "lightning_preimage");
}

#[test]
fn test_e2e_lightning_payment_with_amount() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    client
        .register_lightning_executor(Box::new(MockLightningExecutorE2E::new()))
        .unwrap();

    // Zero-amount invoice (amount specified separately)
    let invoice = format!("lntb1p{}", "0".repeat(200));

    let result = client
        .execute_payment("lightning".to_string(), invoice, 5000, None)
        .unwrap();

    assert!(result.success);
}

// ============================================================================
// Error Scenario Tests
// ============================================================================

#[test]
fn test_e2e_payment_method_not_found() {
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
fn test_e2e_bitcoin_executor_failure() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    // Register failing executor
    client
        .register_bitcoin_executor(Box::new(MockBitcoinExecutorE2E::failing(
            "Insufficient funds",
        )))
        .unwrap();

    let result = client.execute_payment(
        "onchain".to_string(),
        "tb1qtest123456789012345678901234567890".to_string(),
        100000,
        None,
    );

    // Should fail with executor error
    assert!(result.is_err() || !result.as_ref().unwrap().success);
}

#[test]
fn test_e2e_lightning_executor_failure() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    // Register failing executor
    client
        .register_lightning_executor(Box::new(MockLightningExecutorE2E::failing(
            "No route found",
        )))
        .unwrap();

    let invoice = format!("lntb1000n1p{}", "0".repeat(200));

    let result = client.execute_payment("lightning".to_string(), invoice, 1000, None);

    // Should fail with executor error
    assert!(result.is_err() || !result.as_ref().unwrap().success);
}

#[test]
fn test_e2e_invalid_proof_generation() {
    let client = PaykitClient::new().unwrap();

    // Invalid JSON
    let result = client.generate_payment_proof("onchain".to_string(), "not valid json".to_string());

    assert!(result.is_err());
    match result {
        Err(PaykitMobileError::Serialization { .. }) => {}
        _ => panic!("Expected Serialization error"),
    }
}

#[test]
fn test_e2e_unknown_method_proof_generation() {
    let client = PaykitClient::new().unwrap();

    let result = client.generate_payment_proof("unknown".to_string(), "{}".to_string());

    assert!(result.is_err());
    match result {
        Err(PaykitMobileError::NotFound { .. }) => {}
        _ => panic!("Expected NotFound error"),
    }
}

// ============================================================================
// Multiple Payment Tests
// ============================================================================

#[test]
fn test_e2e_multiple_sequential_payments() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    client
        .register_bitcoin_executor(Box::new(MockBitcoinExecutorE2E::new()))
        .unwrap();

    // Execute 5 payments
    for i in 1..=5 {
        let result = client
            .execute_payment(
                "onchain".to_string(),
                "tb1qtest123456789012345678901234567890".to_string(),
                10000 * i,
                None,
            )
            .unwrap();

        assert!(result.success);
        assert_eq!(result.amount_sats, 10000 * i);
    }
}

#[test]
fn test_e2e_mixed_payment_methods() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    // Register both executors
    client
        .register_bitcoin_executor(Box::new(MockBitcoinExecutorE2E::new()))
        .unwrap();
    client
        .register_lightning_executor(Box::new(MockLightningExecutorE2E::new()))
        .unwrap();

    // Execute on-chain payment
    let btc_result = client
        .execute_payment(
            "onchain".to_string(),
            "tb1qtest123456789012345678901234567890".to_string(),
            50000,
            None,
        )
        .unwrap();
    assert!(btc_result.success);
    assert_eq!(btc_result.method_id, "onchain");

    // Execute Lightning payment
    let ln_invoice = format!("lntb1000n1p{}", "0".repeat(200));
    let ln_result = client
        .execute_payment("lightning".to_string(), ln_invoice, 1000, None)
        .unwrap();
    assert!(ln_result.success);
    assert_eq!(ln_result.method_id, "lightning");

    // Generate proofs for both
    let btc_proof = client
        .generate_payment_proof("onchain".to_string(), btc_result.execution_data_json)
        .unwrap();
    assert_eq!(btc_proof.proof_type, "bitcoin_txid");

    let ln_proof = client
        .generate_payment_proof("lightning".to_string(), ln_result.execution_data_json)
        .unwrap();
    assert_eq!(ln_proof.proof_type, "lightning_preimage");
}

// ============================================================================
// Network Configuration Tests
// ============================================================================

#[test]
fn test_e2e_mainnet_configuration() {
    let client = PaykitClient::new().unwrap(); // Defaults to mainnet

    assert_eq!(client.bitcoin_network(), BitcoinNetworkFFI::Mainnet);
    assert_eq!(client.lightning_network(), LightningNetworkFFI::Mainnet);
}

#[test]
fn test_e2e_testnet_configuration() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Testnet, LightningNetworkFFI::Testnet)
            .unwrap();

    assert_eq!(client.bitcoin_network(), BitcoinNetworkFFI::Testnet);
    assert_eq!(client.lightning_network(), LightningNetworkFFI::Testnet);
}

#[test]
fn test_e2e_regtest_configuration() {
    let client =
        PaykitClient::new_with_network(BitcoinNetworkFFI::Regtest, LightningNetworkFFI::Regtest)
            .unwrap();

    assert_eq!(client.bitcoin_network(), BitcoinNetworkFFI::Regtest);
    assert_eq!(client.lightning_network(), LightningNetworkFFI::Regtest);
}
