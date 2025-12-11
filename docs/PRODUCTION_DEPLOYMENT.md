# Production Deployment Guide

This guide covers deploying Paykit in production environments.

## Pre-Deployment Checklist

- [ ] Security audit completed
- [ ] Key storage configured (Keychain/Keystore)
- [ ] Payment executor tested
- [ ] Error handling reviewed
- [ ] Logging and monitoring set up
- [ ] Backup and recovery plan
- [ ] Network tested (mainnet vs testnet)

## Configuration

### Network Selection

```rust
use paykit_lib::methods::{
    default_registry,      // Mainnet
    testnet_registry,      // Testnet
    regtest_registry,      // Regtest
};
use paykit_lib::executors::{BitcoinNetwork, EsploraConfig, LndConfig};

// Use the appropriate registry for your environment
let registry = if cfg!(debug_assertions) {
    testnet_registry()
} else {
    default_registry()
};

// Configure executors for the right network
let network = BitcoinNetwork::Mainnet;
let esplora = EsploraConfig::for_network(network);
```

### Executor Configuration

**Lightning (LND):**

```rust
use paykit_lib::executors::{LndExecutor, LndConfig};

let config = LndConfig::new(
    "https://your-lnd-node:8080",
    &std::env::var("LND_MACAROON").expect("LND_MACAROON required"),
)
.with_timeout(30)
.with_tls_cert_path("/path/to/tls.cert");

let executor = LndExecutor::new(config);
```

**On-Chain (Esplora):**

```rust
use paykit_lib::executors::{EsploraExecutor, EsploraConfig};

// Use reliable Esplora endpoints
let config = EsploraConfig::new("https://blockstream.info/api")
    .with_timeout(30);

let executor = EsploraExecutor::new(config);
```

## Security

### Key Storage

**iOS:**
```swift
// Use Keychain with appropriate access control
let query: [String: Any] = [
    kSecClass as String: kSecClassGenericPassword,
    kSecAttrService as String: "com.yourapp.paykit",
    kSecAttrAccount as String: keyId,
    kSecAttrAccessControl as String: accessControl,
    kSecValueData as String: keyData
]
SecItemAdd(query as CFDictionary, nil)
```

**Android:**
```kotlin
// Use Android Keystore
val keyStore = KeyStore.getInstance("AndroidKeyStore")
keyStore.load(null)
val secretKey = KeyGenerator.getInstance(
    KeyProperties.KEY_ALGORITHM_AES,
    "AndroidKeyStore"
).apply {
    init(KeyGenParameterSpec.Builder(keyId, PURPOSE_ENCRYPT or PURPOSE_DECRYPT)
        .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
        .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
        .setUserAuthenticationRequired(true)
        .build())
}.generateKey()
```

### Invoice Validation

Always validate before paying:

```rust
use paykit_lib::methods::LightningExecutor;

async fn safe_pay(executor: &impl LightningExecutor, invoice: &str) -> Result<()> {
    // Decode and validate
    let decoded = executor.decode_invoice(invoice).await?;
    
    // Check expiry
    if decoded.expired {
        return Err(PaykitError::InvoiceExpired { 
            invoice_id: decoded.payment_hash,
            expired_at: decoded.timestamp as i64 + decoded.expiry as i64,
        });
    }
    
    // Check amount
    if let Some(amount) = decoded.amount_msat {
        if amount > MAX_PAYMENT_MSAT {
            return Err(PaykitError::ValidationFailed(
                "Amount exceeds maximum".into()
            ));
        }
    }
    
    // Proceed with payment
    executor.pay_invoice(invoice, None, Some(MAX_FEE_MSAT)).await?;
    Ok(())
}
```

### Rate Limiting

Implement rate limiting on payment endpoints:

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

struct PaymentRateLimiter {
    attempts: HashMap<String, Vec<Instant>>,
    max_per_minute: usize,
}

impl PaymentRateLimiter {
    fn check(&mut self, user_id: &str) -> bool {
        let now = Instant::now();
        let attempts = self.attempts.entry(user_id.to_string()).or_default();
        
        // Remove old attempts
        attempts.retain(|t| now.duration_since(*t) < Duration::from_secs(60));
        
        if attempts.len() >= self.max_per_minute {
            return false;
        }
        
        attempts.push(now);
        true
    }
}
```

## Performance

### Expected Latencies

| Operation | Latency (p50) | Latency (p99) |
|-----------|---------------|---------------|
| Endpoint Discovery | 50ms | 200ms |
| Invoice Decode | 1ms | 5ms |
| Lightning Payment | 1s | 10s |
| On-chain Broadcast | 500ms | 2s |
| Transaction Verification | 100ms | 500ms |

### Caching Strategy

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

struct EndpointCache {
    cache: HashMap<String, (SupportedPayments, Instant)>,
    ttl: Duration,
}

impl EndpointCache {
    fn get(&self, pubkey: &str) -> Option<&SupportedPayments> {
        self.cache.get(pubkey).and_then(|(payments, cached_at)| {
            if cached_at.elapsed() < self.ttl {
                Some(payments)
            } else {
                None
            }
        })
    }
}
```

### Connection Pooling

For high-throughput applications:

```rust
// Reuse HTTP clients
use reqwest::Client;

lazy_static! {
    static ref HTTP_CLIENT: Client = Client::builder()
        .pool_max_idle_per_host(10)
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");
}
```

## Monitoring

### Metrics to Track

- Payment success/failure rate
- Payment latency (by method)
- Endpoint discovery latency
- Error rates by type
- Invoice expiry rate
- Fee expenditure

### Health Checks

```rust
struct PaykitHealth {
    lnd_connected: bool,
    esplora_reachable: bool,
    last_payment_success: Option<Instant>,
    error_rate_1h: f64,
}

async fn health_check(
    lnd: &LndExecutor,
    esplora: &EsploraExecutor,
) -> PaykitHealth {
    PaykitHealth {
        lnd_connected: lnd.ping().await.is_ok(),
        esplora_reachable: esplora.get_block_height().await.is_ok(),
        last_payment_success: /* from metrics */,
        error_rate_1h: /* from metrics */,
    }
}
```

### Alerting Thresholds

| Metric | Warning | Critical |
|--------|---------|----------|
| Payment Failure Rate | >5% | >20% |
| Payment Latency (p99) | >10s | >30s |
| LND Connection | Disconnected 1min | Disconnected 5min |
| Esplora Reachable | Unreachable 1min | Unreachable 5min |

## Error Handling

### Retry Strategy

```rust
use paykit_lib::PaykitError;

async fn pay_with_retry(
    executor: &impl LightningExecutor,
    invoice: &str,
    max_retries: u32,
) -> Result<LightningPaymentResult> {
    let mut last_error = None;
    
    for attempt in 0..max_retries {
        match executor.pay_invoice(invoice, None, None).await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() => {
                if let Some(delay) = e.retry_after_ms() {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                last_error = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    
    Err(last_error.unwrap_or_else(|| PaykitError::Internal("Unknown error".into())))
}
```

### Graceful Degradation

```rust
async fn pay_with_fallback(
    lightning: &impl LightningExecutor,
    onchain: &impl BitcoinExecutor,
    amount_sats: u64,
    lightning_invoice: &str,
    fallback_address: &str,
) -> Result<PaymentResult> {
    // Try Lightning first
    match lightning.pay_invoice(lightning_invoice, None, None).await {
        Ok(result) => Ok(PaymentResult::Lightning(result)),
        Err(e) if e.is_retryable() => {
            // Retry Lightning
            Err(e)
        }
        Err(_) => {
            // Fall back to on-chain
            let result = onchain.send_to_address(fallback_address, amount_sats, None).await?;
            Ok(PaymentResult::Onchain(result))
        }
    }
}
```

## Disaster Recovery

### Backup Strategy

1. **Keys** - Secure backup of all payment keys
2. **Pending Payments** - Track in-flight payments
3. **Transaction History** - Export for accounting

### Recovery Procedure

1. Restore keys from backup
2. Verify node connectivity
3. Check for pending payments
4. Reconcile transaction history
5. Resume operations

## Compliance

### Financial Regulations

- Keep transaction records for required period
- Implement AML/KYC where required
- Report as required by jurisdiction

### Audit Trail

```rust
#[derive(Serialize)]
struct PaymentAuditLog {
    timestamp: DateTime<Utc>,
    user_id: String,
    payment_method: String,
    amount_sats: u64,
    payment_hash: String,
    status: String,
    error: Option<String>,
}
```

## Testing

### Pre-Production Testing

```bash
# Run full test suite
cargo test

# Run integration tests against testnet
BITCOIN_NETWORK=testnet cargo test --features integration

# Load testing
cargo run --example load_test -- --rate 100 --duration 60
```

### Smoke Tests

```rust
#[tokio::test]
async fn smoke_test_production() {
    let executor = production_executor();
    
    // Verify connectivity
    assert!(executor.ping().await.is_ok());
    
    // Verify can decode invoices
    let decoded = executor.decode_invoice(TEST_INVOICE).await;
    assert!(decoded.is_ok());
}
```
