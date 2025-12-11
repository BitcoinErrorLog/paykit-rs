# Paykit Integration Guide

This guide covers integrating the Paykit library for Bitcoin payment discovery and coordination.

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
paykit-lib = "0.1"
```

### Basic Usage

```rust
use paykit_lib::prelude::*;

// Create a payment registry
let registry = default_registry();

// Get the Lightning plugin
let lightning = registry.get(&MethodId::lightning()).unwrap();

// Validate an endpoint
let result = lightning.validate_endpoint(&EndpointData::new("lnurl1..."));
assert!(result.valid);
```

## Core Concepts

### Payment Methods

Paykit uses a plugin system for payment methods:

- **OnchainPlugin** - Bitcoin on-chain payments
- **LightningPlugin** - Lightning Network payments

```rust
use paykit_lib::methods::{OnchainPlugin, LightningPlugin, BitcoinNetwork, LightningNetwork};

// Create plugins for specific networks
let onchain = OnchainPlugin::with_network(BitcoinNetwork::Mainnet);
let lightning = LightningPlugin::with_network(LightningNetwork::Mainnet);

// Or use testnet
let testnet_onchain = OnchainPlugin::with_network(BitcoinNetwork::Testnet);
```

### Method IDs and Endpoints

```rust
use paykit_lib::{MethodId, EndpointData};

// Create method IDs
let method = MethodId::new("lightning");
// Or use helpers
let onchain = MethodId::onchain();
let lightning = MethodId::lightning();

// Create endpoint data
let address = EndpointData::new("bc1q...");
let lnurl = EndpointData::new("lnurl1...");

// Access inner values
println!("Method: {}", method.as_str());
println!("Address: {}", address.as_str());
```

### Transport Traits

Paykit uses trait-based dependency injection for transport:

```rust
use paykit_lib::{AuthenticatedTransport, UnauthenticatedTransportRead};

// For reading public payment endpoints
async fn discover_payments<T: UnauthenticatedTransportRead>(
    transport: &T,
    payee: &PublicKey,
) -> Result<SupportedPayments> {
    get_payment_list(transport, payee).await
}

// For publishing your own endpoints
async fn publish_endpoint<T: AuthenticatedTransport>(
    transport: &T,
    method: MethodId,
    data: EndpointData,
) -> Result<()> {
    set_payment_endpoint(transport, method, data).await
}
```

## Error Handling

All errors are typed with error codes for FFI compatibility:

```rust
use paykit_lib::{PaykitError, PaykitErrorCode};

match result {
    Err(e) => {
        let code = e.code();     // Numeric code for FFI
        let msg = e.message();   // Human-readable message
        
        if e.is_retryable() {
            if let Some(delay) = e.retry_after_ms() {
                // Wait and retry
            }
        }
    }
    Ok(value) => { /* success */ }
}
```

### Common Error Types

| Error | Code | Description |
|-------|------|-------------|
| `Transport` | 1001 | Network/transport failure |
| `NotFound` | 2001 | Resource not found |
| `InvalidData` | 3001 | Invalid data format |
| `Payment` | 4001 | Payment failed |
| `InsufficientFunds` | 4002 | Not enough funds |
| `RateLimited` | 5002 | Too many requests |

## Payment Executors

### Lightning Executor (LND)

```rust
use paykit_lib::executors::{LndExecutor, LndConfig};

let config = LndConfig::new(
    "https://localhost:8080",
    "your-macaroon-hex",
)
.with_timeout(30);

let executor = LndExecutor::new(config);

// Pay an invoice
let result = executor.pay_invoice("lnbc1...", None, Some(1000)).await?;
println!("Preimage: {}", result.preimage);
```

### Bitcoin Executor (Esplora)

```rust
use paykit_lib::executors::{EsploraExecutor, EsploraConfig};

let config = EsploraConfig::new("https://blockstream.info/api");
let executor = EsploraExecutor::new(config);

// Get transaction details
let tx = executor.get_transaction("txid...").await?;

// Verify a payment
let verified = executor.verify_transaction(
    "txid...",
    "bc1q...",
    100_000,  // sats
).await?;
```

## Secure Key Storage

### Platform-Agnostic Trait

```rust
use paykit_lib::secure_storage::{SecureKeyStorage, StoreOptions, KeyMetadata};

async fn store_key<S: SecureKeyStorage>(
    storage: &S,
    key_id: &str,
    data: &[u8],
) -> Result<()> {
    let options = StoreOptions::default()
        .requires_auth(true)
        .label("Payment Key");
    
    storage.store(key_id, data, Some(options)).await
}
```

### Platform Implementations

- **iOS**: `KeychainStorage` - Uses iOS Keychain
- **Android**: `KeystoreStorage` - Uses Android Keystore
- **Web**: `WebCryptoStorage` - Uses SubtleCrypto + IndexedDB
- **Desktop**: `DesktopKeyStorage` - Uses OS-native storage

### Testing

Use `InMemoryKeyStorage` for tests:

```rust
use paykit_lib::secure_storage::InMemoryKeyStorage;

let storage = InMemoryKeyStorage::new();
storage.store("test-key", b"secret", None).await?;
```

## URI Parsing

Parse Paykit URIs:

```rust
use paykit_lib::{parse_uri, PaykitUri};

let uri = "paykit://pk1abc123/lightning?amount=10000";
match parse_uri(uri) {
    Ok(PaykitUri { pubkey, method, amount, .. }) => {
        println!("Payee: {}", pubkey);
        println!("Method: {:?}", method);
        println!("Amount: {:?}", amount);
    }
    Err(e) => eprintln!("Invalid URI: {}", e),
}
```

## Testing

### Using Test Utilities

Enable the `test-utils` feature:

```toml
[dev-dependencies]
paykit-lib = { version = "0.1", features = ["test-utils"] }
```

```rust
use paykit_lib::test_utils::{TestNetwork, TestWallet, assert_payment_succeeded};

#[tokio::test]
async fn test_payment_flow() {
    let network = TestNetwork::new();
    let alice = network.create_wallet("alice");
    let bob = network.create_wallet("bob");
    
    alice.fund(100_000);
    
    let result = alice.pay_invoice("lnbc...", Some(10_000), None).await;
    assert_payment_succeeded(&result);
}
```

### Mock Executors

```rust
use paykit_lib::methods::{MockLightningExecutor, MockBitcoinExecutor};

let mock_ln = MockLightningExecutor::new();
let mock_btc = MockBitcoinExecutor::new();
```

## Pubky Integration

When using with Pubky:

```rust
use paykit_lib::{PubkyAuthenticatedTransport, PubkyUnauthenticatedTransport};
use pubky::Session;

// Authenticated transport for publishing
let session = Session::new(/* ... */);
let transport = PubkyAuthenticatedTransport::new(session);

// Unauthenticated for reading
let reader = PubkyUnauthenticatedTransport::new();
```

## Security Considerations

1. **Key Storage**: Use platform-native secure storage, never plaintext.

2. **Invoice Validation**: Always validate invoices before paying.

3. **Amount Verification**: Verify amounts match expected values.

4. **Rate Limiting**: Implement rate limiting on payment endpoints.

5. **Error Handling**: Don't expose internal error details in UI.

## Examples

See the `examples/` directory:

- `p2p_payment.rs` - Peer-to-peer payment flow
- `ecommerce.rs` - E-commerce integration
- `subscription_service.rs` - Subscription payments

## API Reference

See the [API documentation](https://docs.rs/paykit-lib) for complete reference.
