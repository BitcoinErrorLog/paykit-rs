//! Integration tests for payment executors.
//!
//! These tests verify the LND and Esplora executors work correctly with
//! mock HTTP servers and (optionally) real testnet nodes.
//!
//! # Running Tests
//!
//! ## Mock tests (default, no network required)
//!
//! ```bash
//! cargo test -p paykit-lib --features http-executor --test executor_integration
//! ```
//!
//! ## Real testnet tests (requires running nodes)
//!
//! These tests are marked `#[ignore]` and require manual setup:
//!
//! ### For LND tests:
//! 1. Run Polar or set up an LND testnet node
//! 2. Export environment variables:
//!    ```bash
//!    export PAYKIT_LND_URL=https://localhost:8081
//!    export PAYKIT_LND_MACAROON=0201036c6e6402...
//!    export PAYKIT_NETWORK=regtest
//!    ```
//! 3. Run with `--ignored`:
//!    ```bash
//!    cargo test -p paykit-lib --features http-executor --test executor_integration -- --ignored
//!    ```
//!
//! ### For Esplora tests (no special setup needed):
//! ```bash
//! cargo test -p paykit-lib --features http-executor esplora_real -- --ignored
//! ```

#![cfg(feature = "http-executor")]

use paykit_lib::executors::{
    testnet::{get_lnd_config_from_env, TestnetConfig},
    EsploraConfig, EsploraExecutor, LndConfig, LndExecutor,
};
use paykit_lib::methods::LightningExecutor;
use wiremock::{
    matchers::{header, method, path, path_regex},
    Mock, MockServer, ResponseTemplate,
};

// ============================================================================
// LND Executor Mock Tests
// ============================================================================

#[tokio::test]
async fn test_lnd_decode_invoice_mock() {
    let mock_server = MockServer::start().await;

    // Mock the decode invoice endpoint with a future timestamp
    let future_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 1000; // 1000 seconds in the future

    Mock::given(method("GET"))
        .and(path_regex(r"/v1/payreq/.*"))
        .and(header("Grpc-Metadata-macaroon", "test_macaroon"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "destination": "03abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab",
            "payment_hash": "abc123def456",
            "num_msat": "100000",
            "timestamp": future_timestamp.to_string(),
            "expiry": "3600",
            "description": "Test payment",
            "description_hash": ""
        })))
        .mount(&mock_server)
        .await;

    let config = LndConfig::new(mock_server.uri(), "test_macaroon");
    let executor = LndExecutor::new(config).unwrap();

    let decoded = executor.decode_invoice("lnbc1u1ptest").await.unwrap();

    assert_eq!(decoded.payment_hash, "abc123def456");
    assert_eq!(decoded.amount_msat, Some(100000));
    assert_eq!(decoded.description, Some("Test payment".to_string()));
    assert!(!decoded.expired);
}

#[tokio::test]
async fn test_lnd_pay_invoice_mock() {
    let mock_server = MockServer::start().await;

    // Mock the pay invoice endpoint
    Mock::given(method("POST"))
        .and(path("/v1/channels/transactions"))
        .and(header("Grpc-Metadata-macaroon", "test_macaroon"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "payment_preimage": "preimage123456",
            "payment_hash": "hash123456",
            "payment_error": "",
            "payment_route": {
                "total_fees_msat": "100",
                "hops": [{"chan_id": "123"}, {"chan_id": "456"}]
            }
        })))
        .mount(&mock_server)
        .await;

    let config = LndConfig::new(mock_server.uri(), "test_macaroon");
    let executor = LndExecutor::new(config).unwrap();

    let result = executor
        .pay_invoice("lnbc1u1ptest", Some(100000), None)
        .await
        .unwrap();

    assert_eq!(result.preimage, "preimage123456");
    assert_eq!(result.payment_hash, "hash123456");
    assert_eq!(result.fee_msat, 100);
    assert_eq!(result.hops, 2);
    assert_eq!(
        result.status,
        paykit_lib::methods::LightningPaymentStatus::Succeeded
    );
}

#[tokio::test]
async fn test_lnd_pay_invoice_error_mock() {
    let mock_server = MockServer::start().await;

    // Mock a payment error
    Mock::given(method("POST"))
        .and(path("/v1/channels/transactions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "payment_preimage": "",
            "payment_hash": "hash123",
            "payment_error": "no route found",
            "payment_route": null
        })))
        .mount(&mock_server)
        .await;

    let config = LndConfig::new(mock_server.uri(), "test_macaroon");
    let executor = LndExecutor::new(config).unwrap();

    let result = executor
        .pay_invoice("lnbc1u1ptest", Some(100000), None)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("no route found"));
}

#[tokio::test]
async fn test_lnd_get_payment_mock() {
    let mock_server = MockServer::start().await;

    // Mock the list payments endpoint
    Mock::given(method("GET"))
        .and(path("/v1/payments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "payments": [
                {
                    "payment_hash": "target_hash",
                    "payment_preimage": "preimage123",
                    "value_msat": "50000",
                    "fee_msat": "10",
                    "status": "SUCCEEDED"
                },
                {
                    "payment_hash": "other_hash",
                    "payment_preimage": "preimage456",
                    "value_msat": "100000",
                    "fee_msat": "20",
                    "status": "IN_FLIGHT"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let config = LndConfig::new(mock_server.uri(), "test_macaroon");
    let executor = LndExecutor::new(config).unwrap();

    let payment = executor.get_payment("target_hash").await.unwrap();

    assert!(payment.is_some());
    let p = payment.unwrap();
    assert_eq!(p.payment_hash, "target_hash");
    assert_eq!(p.amount_msat, 50000);
    assert_eq!(
        p.status,
        paykit_lib::methods::LightningPaymentStatus::Succeeded
    );
}

#[tokio::test]
async fn test_lnd_auth_failure_mock() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex(r"/v1/payreq/.*"))
        .respond_with(ResponseTemplate::new(401).set_body_string("permission denied"))
        .mount(&mock_server)
        .await;

    let config = LndConfig::new(mock_server.uri(), "bad_macaroon");
    let executor = LndExecutor::new(config).unwrap();

    let result = executor.decode_invoice("lnbc1test").await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("authentication") || err.to_string().contains("permission denied")
    );
}

// ============================================================================
// Esplora Executor Mock Tests
// ============================================================================

#[tokio::test]
async fn test_esplora_fee_estimates_mock() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/fee-estimates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "1": 50.0,
            "3": 25.0,
            "6": 10.0,
            "144": 1.0
        })))
        .mount(&mock_server)
        .await;

    let config = EsploraConfig::new(mock_server.uri());
    let executor = EsploraExecutor::new(config).unwrap();

    let fees = executor.get_fee_estimates().await.unwrap();

    assert_eq!(fees.get_rate_for_blocks(1), 50.0);
    assert_eq!(fees.get_rate_for_blocks(6), 10.0);
    assert_eq!(fees.get_rate_for_blocks(144), 1.0);
}

#[tokio::test]
async fn test_esplora_address_info_mock() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/address/tb1qtest"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "address": "tb1qtest",
            "chain_stats": {
                "funded_txo_count": 5,
                "funded_txo_sum": 100000,
                "spent_txo_count": 2,
                "spent_txo_sum": 30000,
                "tx_count": 7
            },
            "mempool_stats": {
                "funded_txo_count": 1,
                "funded_txo_sum": 5000,
                "spent_txo_count": 0,
                "spent_txo_sum": 0,
                "tx_count": 1
            }
        })))
        .mount(&mock_server)
        .await;

    let config = EsploraConfig::new(mock_server.uri());
    let executor = EsploraExecutor::new(config).unwrap();

    let info = executor.get_address_info("tb1qtest").await.unwrap();

    assert_eq!(info.address, "tb1qtest");
    assert_eq!(info.confirmed_balance(), 70000); // 100000 - 30000
    assert_eq!(info.unconfirmed_balance(), 5000);
    assert_eq!(info.total_balance(), 75000);
}

#[tokio::test]
async fn test_esplora_utxos_mock() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/address/tb1qtest/utxo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "txid": "abc123",
                "vout": 0,
                "value": 50000,
                "status": {
                    "confirmed": true,
                    "block_height": 100000,
                    "block_hash": "blockhash123",
                    "block_time": 1700000000
                }
            },
            {
                "txid": "def456",
                "vout": 1,
                "value": 25000,
                "status": {
                    "confirmed": false,
                    "block_height": null,
                    "block_hash": null,
                    "block_time": null
                }
            }
        ])))
        .mount(&mock_server)
        .await;

    let config = EsploraConfig::new(mock_server.uri());
    let executor = EsploraExecutor::new(config).unwrap();

    let utxos = executor.get_address_utxos("tb1qtest").await.unwrap();

    assert_eq!(utxos.len(), 2);
    assert_eq!(utxos[0].txid, "abc123");
    assert_eq!(utxos[0].value, 50000);
    assert!(utxos[0].status.confirmed);
    assert_eq!(utxos[1].txid, "def456");
    assert!(!utxos[1].status.confirmed);
}

#[tokio::test]
async fn test_esplora_broadcast_tx_mock() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tx"))
        .respond_with(ResponseTemplate::new(200).set_body_string("txid123456"))
        .mount(&mock_server)
        .await;

    let config = EsploraConfig::new(mock_server.uri());
    let executor = EsploraExecutor::new(config).unwrap();

    let txid = executor.broadcast_tx("0200000001...").await.unwrap();

    assert_eq!(txid, "txid123456");
}

#[tokio::test]
async fn test_esplora_block_height_mock() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/blocks/tip/height"))
        .respond_with(ResponseTemplate::new(200).set_body_string("800000"))
        .mount(&mock_server)
        .await;

    let config = EsploraConfig::new(mock_server.uri());
    let executor = EsploraExecutor::new(config).unwrap();

    let height = executor.get_block_height().await.unwrap();

    assert_eq!(height, 800000);
}

#[tokio::test]
async fn test_esplora_transaction_mock() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/tx/abc123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "txid": "abc123",
            "version": 2,
            "locktime": 0,
            "size": 225,
            "weight": 573,
            "fee": 450,
            "status": {
                "confirmed": true,
                "block_height": 100000,
                "block_hash": "blockhash",
                "block_time": 1700000000
            },
            "vin": [],
            "vout": [
                {
                    "value": 50000,
                    "scriptpubkey": "0014...",
                    "scriptpubkey_asm": "OP_0 ...",
                    "scriptpubkey_type": "v0_p2wpkh",
                    "scriptpubkey_address": "tb1qtest"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let config = EsploraConfig::new(mock_server.uri());
    let executor = EsploraExecutor::new(config).unwrap();

    let tx = executor.get_tx("abc123").await.unwrap();

    assert_eq!(tx.txid, "abc123");
    assert_eq!(tx.fee, 450);
    assert!(tx.status.confirmed);
    assert_eq!(tx.vout.len(), 1);
    assert_eq!(tx.vout[0].value, 50000);
}

// ============================================================================
// Testnet Configuration Tests
// ============================================================================

#[test]
fn test_testnet_config_presets() {
    let polar = TestnetConfig::polar_regtest();
    assert!(polar.lnd.is_none());
    assert_eq!(
        polar.network,
        paykit_lib::executors::BitcoinNetwork::Regtest
    );

    let testnet = TestnetConfig::testnet3();
    assert_eq!(
        testnet.network,
        paykit_lib::executors::BitcoinNetwork::Testnet
    );

    let signet = TestnetConfig::signet();
    assert_eq!(
        signet.network,
        paykit_lib::executors::BitcoinNetwork::Signet
    );
}

#[test]
fn test_polar_alice_config() {
    let config = TestnetConfig::polar_alice("test_macaroon");
    assert!(config.lnd.is_some());
    let lnd = config.lnd.unwrap();
    assert!(lnd.rest_url.contains("8081"));
}

// ============================================================================
// Real Testnet Tests (require running nodes - marked #[ignore])
// ============================================================================

/// Test decoding a real invoice on testnet.
///
/// Requires:
/// - PAYKIT_LND_URL and PAYKIT_LND_MACAROON environment variables
/// - A running LND node on testnet/regtest
#[tokio::test]
#[ignore = "Requires running LND node - set PAYKIT_LND_URL and PAYKIT_LND_MACAROON"]
async fn test_lnd_real_decode_invoice() {
    let config = get_lnd_config_from_env().expect("LND config from env required");
    let executor = LndExecutor::new(config).expect("Failed to create executor");

    // This is a sample testnet invoice - replace with a real one
    let invoice = std::env::var("TEST_INVOICE").unwrap_or_else(|_| {
        // A sample expired testnet invoice for testing
        "lntb10u1pjtest".to_string()
    });

    let result = executor.decode_invoice(&invoice).await;
    println!("Decode result: {:?}", result);

    // Just verify it doesn't crash - actual result depends on invoice
}

/// Test querying the real Blockstream testnet API.
#[tokio::test]
#[ignore = "Requires network access to Blockstream API"]
async fn test_esplora_real_fee_estimates() {
    let executor = EsploraExecutor::blockstream_testnet().expect("Failed to create executor");

    let fees = executor
        .get_fee_estimates()
        .await
        .expect("Failed to get fee estimates");

    println!("Fee estimates: {:?}", fees);

    // Verify we got some estimates
    assert!(!fees.estimates.is_empty());

    // Verify fee rates are positive
    for rate in fees.estimates.values() {
        assert!(*rate > 0.0);
    }
}

/// Test querying a real address on testnet.
#[tokio::test]
#[ignore = "Requires network access to Blockstream API"]
async fn test_esplora_real_address_info() {
    let executor = EsploraExecutor::blockstream_testnet().expect("Failed to create executor");

    // This is a well-known testnet faucet address
    let address = "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx";

    let info = executor
        .get_address_info(address)
        .await
        .expect("Failed to get address info");

    println!("Address info: {:?}", info);

    // This address should have some transaction history
    assert!(info.chain_stats.tx_count > 0);
}

/// Test getting current block height.
#[tokio::test]
#[ignore = "Requires network access to Blockstream API"]
async fn test_esplora_real_block_height() {
    let executor = EsploraExecutor::blockstream_testnet().expect("Failed to create executor");

    let height = executor
        .get_block_height()
        .await
        .expect("Failed to get block height");

    println!("Current testnet block height: {}", height);

    // Testnet should be at least at this height (as of late 2024)
    assert!(height > 2_500_000);
}
