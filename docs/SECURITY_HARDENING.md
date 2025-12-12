# Security Hardening Guide

This guide provides security hardening recommendations for production Paykit deployments.

## Table of Contents

1. [Cryptographic Security](#cryptographic-security)
2. [Key Management](#key-management)
3. [Network Security](#network-security)
4. [Application Security](#application-security)
5. [Operational Security](#operational-security)
6. [Compliance Considerations](#compliance-considerations)

---

## Cryptographic Security

### Ed25519 Signature Verification

**Requirement:** Always verify signatures before processing payments.

```rust
use paykit_subscriptions::{verify_signature, Signature, Subscription};

async fn process_payment_request(
    subscription: &Subscription,
    signature: &Signature,
) -> Result<(), SecurityError> {
    // CRITICAL: Verify signature before any processing
    if !verify_signature(subscription, signature)? {
        return Err(SecurityError::InvalidSignature);
    }
    
    // Verify signature is not expired
    if signature.expires_at < chrono::Utc::now().timestamp() {
        return Err(SecurityError::ExpiredSignature);
    }
    
    // Now safe to process
    process_subscription(subscription).await
}
```

### Nonce-Based Replay Protection

**Requirement:** Every signed message must include a unique nonce.

```rust
use paykit_subscriptions::NonceStore;

struct ProductionNonceStore {
    db: DatabasePool,
}

impl NonceStore for ProductionNonceStore {
    fn check_and_mark(&self, nonce: &[u8; 32], expires_at: i64) -> Result<bool> {
        // Use database transaction for atomicity
        let mut tx = self.db.begin()?;
        
        // Check if nonce was already used
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM used_nonces WHERE nonce = $1)",
            nonce.as_slice()
        )
        .fetch_one(&mut tx)
        .await?;
        
        if exists.unwrap_or(false) {
            return Ok(false); // Replay detected!
        }
        
        // Mark nonce as used
        sqlx::query!(
            "INSERT INTO used_nonces (nonce, expires_at) VALUES ($1, $2)",
            nonce.as_slice(),
            expires_at
        )
        .execute(&mut tx)
        .await?;
        
        tx.commit().await?;
        Ok(true)
    }
}
```

### Noise Protocol Configuration

**Requirement:** Use Noise_IK pattern for authenticated connections.

```rust
use pubky_noise::{NoiseServer, RingKeyProvider};

// Server configuration
let key_provider = RingKeyProvider::new(server_secret_key);
let server = NoiseServer::builder()
    .with_key_provider(key_provider)
    .with_pattern("IK")  // Identity-Key pattern
    .build()?;

// Verify peer identity after handshake
let peer_pubkey = channel.remote_public_key();
if !is_authorized_peer(&peer_pubkey) {
    return Err(SecurityError::UnauthorizedPeer);
}
```

---

## Key Management

### Secure Key Generation

```rust
use rand::rngs::OsRng;
use ed25519_dalek::SigningKey;
use zeroize::Zeroizing;

fn generate_signing_key() -> Zeroizing<SigningKey> {
    // Use OS-provided randomness
    let signing_key = SigningKey::generate(&mut OsRng);
    Zeroizing::new(signing_key)
}
```

### Platform-Specific Key Storage

#### iOS (Keychain)

```swift
import Security

class SecureKeyStorage {
    func storeKey(_ keyData: Data, identifier: String) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "com.paykit.keys",
            kSecAttrAccount as String: identifier,
            kSecValueData as String: keyData,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            kSecAttrAccessControl as String: try createAccessControl()
        ]
        
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeyStorageError.storeFailed(status)
        }
    }
    
    private func createAccessControl() throws -> SecAccessControl {
        var error: Unmanaged<CFError>?
        guard let access = SecAccessControlCreateWithFlags(
            kCFAllocatorDefault,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            [.biometryCurrentSet, .devicePasscode],
            &error
        ) else {
            throw KeyStorageError.accessControlFailed
        }
        return access
    }
}
```

#### Android (Keystore)

```kotlin
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.KeyStore
import javax.crypto.KeyGenerator

class SecureKeyStorage(private val context: Context) {
    private val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    
    fun generateKey(alias: String): SecretKey {
        val keyGenerator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            "AndroidKeyStore"
        )
        
        val spec = KeyGenParameterSpec.Builder(
            alias,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setUserAuthenticationRequired(true)
            .setUserAuthenticationValidityDurationSeconds(300)
            .setInvalidatedByBiometricEnrollment(true)
            .build()
        
        keyGenerator.init(spec)
        return keyGenerator.generateKey()
    }
}
```

### Key Rotation

```rust
use paykit_lib::MethodId;
use paykit_interactive::RotationManager;

// Set up rotation callbacks
let rotation_manager = RotationManager::new();

rotation_manager.on_rotation(|method_id, old_endpoint, new_endpoint| {
    tracing::info!(
        method = %method_id.0,
        "Endpoint rotated"
    );
    
    // Notify connected peers
    notify_peers_of_rotation(method_id, new_endpoint);
    
    // Archive old endpoint for audit
    archive_old_endpoint(method_id, old_endpoint);
});

// Schedule regular rotation
rotation_manager.schedule_rotation(
    MethodId("lightning".into()),
    Duration::from_secs(86400), // Daily rotation
);
```

---

## Network Security

### Rate Limiting

```rust
use paykit_interactive::rate_limit::{HandshakeRateLimiter, RateLimitConfig};

// Production configuration
let config = RateLimitConfig {
    max_attempts_per_ip: 5,              // Per IP limit
    global_max_attempts: Some(100),       // Total limit
    window: Duration::from_secs(60),      // 1 minute window
    max_tracked_ips: 100_000,             // Memory limit
};

let limiter = HandshakeRateLimiter::new_shared(config);

// Apply at connection acceptance
async fn accept_connection(stream: TcpStream, limiter: &HandshakeRateLimiter) -> Result<()> {
    let peer_addr = stream.peer_addr()?;
    
    if !limiter.check_and_record(peer_addr.ip()) {
        tracing::warn!(ip = %peer_addr.ip(), "Rate limit exceeded");
        return Err(Error::RateLimited);
    }
    
    // Proceed with handshake
    handle_handshake(stream).await
}
```

### TLS Configuration

```rust
use rustls::{ServerConfig, Certificate, PrivateKey};

fn create_tls_config(cert_path: &str, key_path: &str) -> Result<ServerConfig> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;
    
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()  // Or with_client_cert_verifier for mTLS
        .with_single_cert(certs, key)?;
    
    Ok(config)
}

// Minimum TLS version
const MIN_TLS_VERSION: &rustls::SupportedProtocolVersion = &rustls::version::TLS13;
```

### Connection Timeouts

```rust
use tokio::time::{timeout, Duration};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const IDLE_TIMEOUT: Duration = Duration::from_secs(300);

async fn handle_connection(stream: TcpStream) -> Result<()> {
    // Timeout handshake
    let channel = timeout(HANDSHAKE_TIMEOUT, perform_handshake(stream))
        .await
        .map_err(|_| Error::HandshakeTimeout)??;
    
    // Timeout reads
    loop {
        match timeout(READ_TIMEOUT, channel.recv()).await {
            Ok(Ok(msg)) => process_message(msg).await?,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                // Check if connection is still alive
                if last_activity.elapsed() > IDLE_TIMEOUT {
                    return Err(Error::IdleTimeout);
                }
            }
        }
    }
}
```

---

## Application Security

### Input Validation

```rust
use paykit_lib::{PublicKey, MethodId, EndpointData};

fn validate_payment_request(request: &PaymentRequest) -> Result<(), ValidationError> {
    // Validate public key format
    if request.payer.as_bytes().len() != 32 {
        return Err(ValidationError::InvalidPublicKey);
    }
    
    // Validate amount (must be positive, not overflow)
    if request.amount.is_zero() || request.amount.is_sign_negative() {
        return Err(ValidationError::InvalidAmount);
    }
    
    // Validate method ID (alphanumeric, reasonable length)
    if !is_valid_method_id(&request.method_id) {
        return Err(ValidationError::InvalidMethodId);
    }
    
    // Validate endpoint data doesn't exceed size limit
    if request.endpoint.0.len() > MAX_ENDPOINT_SIZE {
        return Err(ValidationError::EndpointTooLarge);
    }
    
    Ok(())
}

fn is_valid_method_id(method: &MethodId) -> bool {
    !method.0.is_empty() 
        && method.0.len() <= 64
        && method.0.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}
```

### Memory Safety

```rust
use zeroize::Zeroizing;

// Wrap sensitive data
struct SecureContext {
    signing_key: Zeroizing<[u8; 32]>,
    session_key: Zeroizing<[u8; 32]>,
}

impl Drop for SecureContext {
    fn drop(&mut self) {
        // Zeroizing handles secure erasure automatically
        // But we can add logging for audit
        tracing::debug!("Secure context dropped, keys zeroized");
    }
}
```

### Panic Safety

```rust
// Use catch_unwind for FFI boundaries
use std::panic::{catch_unwind, AssertUnwindSafe};

pub extern "C" fn ffi_process_payment(data: *const u8, len: usize) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(|| {
        // Safe Rust code
        let slice = unsafe { std::slice::from_raw_parts(data, len) };
        process_payment_internal(slice)
    }));
    
    match result {
        Ok(Ok(_)) => 0,
        Ok(Err(e)) => {
            tracing::error!(error = %e, "Payment processing failed");
            -1
        }
        Err(_) => {
            tracing::error!("Panic caught at FFI boundary");
            -2
        }
    }
}
```

---

## Operational Security

### Logging

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(signature), fields(subscription_id = %subscription.id))]
async fn verify_and_process(
    subscription: &Subscription,
    signature: &Signature,
) -> Result<Receipt> {
    // Never log sensitive data
    info!("Processing subscription");
    
    if !verify_signature(subscription, signature)? {
        warn!("Invalid signature detected");
        return Err(Error::InvalidSignature);
    }
    
    // Log success with correlation ID
    info!("Subscription processed successfully");
    Ok(receipt)
}
```

### Audit Trail

```rust
#[derive(Serialize)]
struct AuditEvent {
    timestamp: DateTime<Utc>,
    event_type: AuditEventType,
    actor: String,           // Public key of actor
    action: String,
    resource_id: String,
    outcome: AuditOutcome,
    metadata: serde_json::Value,
}

#[derive(Serialize)]
enum AuditEventType {
    PaymentInitiated,
    PaymentCompleted,
    PaymentFailed,
    SubscriptionCreated,
    SignatureVerified,
    SignatureRejected,
    RateLimitTriggered,
    KeyRotated,
}

async fn audit_log(event: AuditEvent) {
    // Write to append-only audit log
    // Consider using a dedicated audit service
    tracing::info!(
        event_type = ?event.event_type,
        actor = %event.actor,
        outcome = ?event.outcome,
        "Audit event"
    );
    
    audit_storage.append(event).await;
}
```

### Monitoring Alerts

| Metric | Warning Threshold | Critical Threshold |
|--------|-------------------|-------------------|
| Signature failures | > 10/min | > 50/min |
| Rate limit triggers | > 100/min | > 500/min |
| Replay attempts | > 1/hour | > 10/hour |
| Key rotation failures | Any | Any |
| Handshake timeouts | > 5% | > 20% |

---

## Compliance Considerations

### Data Retention

```rust
struct RetentionPolicy {
    receipts: Duration,           // Keep transaction records
    nonces: Duration,             // Clean up expired nonces
    audit_logs: Duration,         // Regulatory requirement
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            receipts: Duration::from_days(365 * 7),   // 7 years for financial
            nonces: Duration::from_days(1),           // Just past signature expiry
            audit_logs: Duration::from_days(365 * 7), // Match receipts
        }
    }
}
```

### Privacy

```rust
// Minimize data collection
struct MinimalReceipt {
    id: String,
    timestamp: i64,
    amount_hash: [u8; 32],  // Hash, not actual amount if not needed
    method: MethodId,
    status: PaymentStatus,
    // No PII stored
}

// Log scrubbing
fn scrub_log_entry(entry: &str) -> String {
    // Remove any accidentally logged keys
    let re = regex::Regex::new(r"[a-f0-9]{64}").unwrap();
    re.replace_all(entry, "[REDACTED]").to_string()
}
```

---

## Security Checklist

Before production deployment:

### Cryptography
- [ ] Ed25519 signatures verified on all payment requests
- [ ] Nonces checked for replay protection
- [ ] Expired signatures rejected
- [ ] Noise_IK pattern used for connections

### Key Management
- [ ] Keys stored in platform secure storage (Keychain/Keystore)
- [ ] Key rotation implemented and tested
- [ ] Backup and recovery procedures documented
- [ ] No keys in logs or error messages

### Network
- [ ] Rate limiting configured
- [ ] TLS 1.3 enforced
- [ ] Connection timeouts set
- [ ] DoS protection in place

### Application
- [ ] All inputs validated
- [ ] Sensitive data zeroized on drop
- [ ] Panic boundaries at FFI
- [ ] Error messages don't leak information

### Operations
- [ ] Structured logging configured
- [ ] Audit trail enabled
- [ ] Monitoring alerts set up
- [ ] Incident response plan documented

### Compliance
- [ ] Data retention policy implemented
- [ ] Privacy requirements met
- [ ] Regulatory reporting ready

