# Paykit Mobile Changelog

All notable changes to the paykit-mobile crate are documented in this file.

## [Unreleased]

### Added - Bitkit Executor FFI Integration

This release adds comprehensive support for integrating external wallet implementations (like Bitkit) through FFI callback interfaces.

#### Phase 1: Executor FFI Bindings (`executor_ffi.rs`)

**New FFI Callback Interfaces:**
- `BitcoinExecutorFFI` - Interface for on-chain Bitcoin wallet operations
  - `send_to_address()` - Send Bitcoin to an address
  - `estimate_fee()` - Estimate transaction fee
  - `get_transaction()` - Retrieve transaction details
  - `verify_transaction()` - Verify transaction matches expected parameters
  
- `LightningExecutorFFI` - Interface for Lightning Network node operations
  - `pay_invoice()` - Pay a BOLT11 invoice
  - `decode_invoice()` - Decode invoice without paying
  - `estimate_fee()` - Estimate routing fee
  - `get_payment()` - Get payment status
  - `verify_preimage()` - Verify preimage matches payment hash

**New FFI Result Types:**
- `BitcoinTxResultFFI` - Bitcoin transaction result with txid, fees, confirmations
- `LightningPaymentResultFFI` - Lightning payment result with preimage, fees, status
- `DecodedInvoiceFFI` - Decoded BOLT11 invoice details
- `LightningPaymentStatusFFI` - Payment status enum (Pending/Succeeded/Failed)

**New Network Configuration:**
- `BitcoinNetworkFFI` - Network enum (Mainnet/Testnet/Regtest)
- `LightningNetworkFFI` - Network enum (Mainnet/Testnet/Regtest)

**Executor Bridges:**
- `BitcoinExecutorBridge` - Adapts FFI callbacks to Rust trait
- `LightningExecutorBridge` - Adapts FFI callbacks to Rust trait

#### Phase 2: PaykitClient Extensions (`lib.rs`)

**New Constructors:**
- `PaykitClient::new_with_network(bitcoin_network, lightning_network)` - Create client with specific network configuration

**New Registration Methods:**
- `register_bitcoin_executor(executor)` - Register Bitcoin wallet implementation
- `register_lightning_executor(executor)` - Register Lightning node implementation
- `has_bitcoin_executor()` - Check if Bitcoin executor is registered
- `has_lightning_executor()` - Check if Lightning executor is registered

**New Payment Methods:**
- `execute_payment(method_id, endpoint, amount_sats, metadata)` - Execute a payment
- `generate_payment_proof(method_id, execution_data)` - Generate proof of payment
- `validate_endpoint(method_id, endpoint)` - Validate payment destination

**New Network Accessors:**
- `bitcoin_network()` - Get configured Bitcoin network
- `lightning_network()` - Get configured Lightning network

**New Result Types:**
- `PaymentExecutionResult` - Result of payment execution
- `PaymentProofResult` - Generated payment proof

#### Phase 3: Example Implementations & Documentation

**New Example Files:**
- `swift/BitkitExecutorExample.swift` - Complete Swift implementation example
  - `BitkitBitcoinExecutor` class
  - `BitkitLightningExecutor` class
  - `BitkitPaykitIntegration` helper

- `kotlin/BitkitExecutorExample.kt` - Complete Kotlin implementation example
  - `BitkitBitcoinExecutor` class
  - `BitkitLightningExecutor` class
  - Usage examples

**New Documentation:**
- `BITKIT_INTEGRATION_GUIDE.md` - Comprehensive integration guide
  - Architecture overview
  - Quick start for iOS and Android
  - Interface reference
  - Error handling
  - Thread safety
  - Payment flow
  - Production checklist

#### Phase 4: Integration Tests

**New Test File: `tests/executor_integration.rs`**

30 comprehensive integration tests covering:

| Category | Count | Description |
|----------|-------|-------------|
| Network Configuration | 4 | Mainnet, testnet, regtest, mixed |
| Executor Registration | 4 | Bitcoin, Lightning, both, replace |
| Bitcoin Payment | 4 | Success, fee rate, failure, dust |
| Lightning Payment | 4 | Success, amount, failure, pending |
| Proof Generation | 2 | Bitcoin txid, Lightning preimage |
| Error Handling | 3 | Unknown method, invalid JSON, unknown proof |
| E2E Payment Flow | 2 | Complete Bitcoin/Lightning flows |
| Sequential Payments | 1 | Multiple payment tracking |
| Thread Safety | 2 | Concurrent operations |
| Result Types | 4 | Structure validation |

#### Phase 5: Documentation & Polish

**New Documentation:**
- `CHANGELOG.md` - This changelog
- `API_REFERENCE.md` - Complete API reference for executor FFI

**Updated Files:**
- Main `CHANGELOG.md` updated with Bitkit integration

### Dependencies

- Added `async-trait = "0.1"` for async trait implementations in executor bridges

### Test Results

- **151 unit tests** passing (121 lib + 30 integration)
- All builds passing
- Clippy clean

---

## [0.1.0] - Initial Release

### Added

- Core PaykitClient implementation
- Transport FFI bindings
- Key management (export/import)
- Payment endpoint publishing
- Noise protocol integration
- Health monitoring
- Mobile identity management
