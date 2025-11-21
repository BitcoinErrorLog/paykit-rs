# paykit-subscriptions

**Subscription management and automated payments for Paykit**

This crate provides subscription agreements, recurring payment requests, and auto-pay automation for the Paykit payment network.

## Features

- **Subscription Agreements**: Create and manage bilateral subscription agreements with cryptographic signatures
- **Payment Requests**: Send and receive payment requests with expiration and metadata
- **Auto-Pay Rules**: Configure automated payment approvals with spending limits
- **Cryptographic Security**: Ed25519 signatures with replay protection and domain separation
- **Financial Safety**: Safe arithmetic with overflow protection and atomic spending limits
- **Concurrency Safe**: Thread-safe nonce tracking and spending limit enforcement

## Security Model

### Cryptographic Primitives

- **Signatures**: Ed25519 ([RFC 8032](https://tools.ietf.org/html/rfc8032))
- **Hashing**: SHA-256 ([FIPS 180-4](https://csrc.nist.gov/publications/detail/fips/180/4/final))
- **Serialization**: Postcard (deterministic binary format)
- **Financial Math**: `rust_decimal` with checked arithmetic

### Replay Protection

All signatures include:
- Unique 32-byte nonce (cryptographically random)
- Timestamp (signature creation time)
- Expiration time (signature validity period)
- Domain separation constant (`PAYKIT_SUBSCRIPTION_V2`)

### Spending Limits

- File-level locking for atomic check-and-reserve operations
- Per-peer spending limits with configurable periods
- Automatic rollback on payment failure

## Usage

### Creating a Subscription

```rust
use paykit_subscriptions::{Subscription, SubscriptionTerms, PaymentFrequency, Amount};
use paykit_lib::{MethodId, PublicKey};

let terms = SubscriptionTerms::new(
    Amount::from_sats(1000),
    "SAT".to_string(),
    PaymentFrequency::Monthly { day_of_month: 1 },
    MethodId("lightning".to_string()),
    "Monthly service subscription".to_string(),
);

let subscription = Subscription::new(
    subscriber_pubkey,
    provider_pubkey,
    terms,
);
```

### Signing a Subscription

```rust
use paykit_subscriptions::signing;
use rand::RngCore;

// Generate unique nonce
let mut nonce = [0u8; 32];
rand::thread_rng().fill_bytes(&mut nonce);

// Sign subscription (valid for 7 days)
let signature = signing::sign_subscription_ed25519(
    &subscription,
    &keypair,
    &nonce,
    3600 * 24 * 7,
)?;
```

### Payment Requests

```rust
use paykit_subscriptions::PaymentRequest;

let request = PaymentRequest::new(
    from_pubkey,
    to_pubkey,
    Amount::from_sats(1000),
    "SAT".to_string(),
    MethodId("lightning".to_string()),
)
.with_description("Monthly subscription payment".to_string())
.with_expiration(timestamp + 3600); // 1 hour expiration
```

### Auto-Pay Configuration

```rust
use paykit_subscriptions::{AutoPayRule, PeerSpendingLimit};

// Create auto-pay rule
let rule = AutoPayRule::new(
    subscription_id,
    provider_pubkey,
    MethodId("lightning".to_string()),
)
.with_max_amount(Amount::from_sats(5000))
.enable();

// Set peer spending limit
let limit = PeerSpendingLimit::new(
    peer_pubkey,
    Amount::from_sats(10000),
    "daily".to_string(),
);
```

## Architecture

### Core Components

- **`Subscription`**: Bilateral agreement with cryptographic signatures
- **`PaymentRequest`**: Asynchronous payment request with metadata
- **`SubscriptionManager`**: Handles subscription lifecycle and auto-pay
- **`NonceStore`**: Thread-safe nonce tracking for replay prevention
- **`Amount`**: Safe financial arithmetic with overflow protection

### Storage Abstraction

The `SubscriptionStorage` trait allows pluggable storage backends:

- **Native**: `FileSubscriptionStorage` with file-level locking
- **WASM**: Storage backed by browser localStorage (planned)

### Transport Integration

Integrates with Paykit transport layer for:
- Pubky directory listing and file fetching
- Real-time payment via Noise protocol channels
- Async request/response messaging

## Testing

The crate includes comprehensive test coverage:

- **Unit Tests**: 44 tests covering core functionality
- **Property Tests**: 12 property-based tests using proptest
- **Concurrency Tests**: 6 stress tests for thread safety
- **Spending Limit Tests**: 7 tests for atomic operations

Run tests:

```bash
cargo test --package paykit-subscriptions
```

Run property-based tests:

```bash
cargo test --test property_tests
```

Run concurrency stress tests:

```bash
cargo test --test concurrency_tests
```

## Platform Support

- **Native** (Linux, macOS, Windows): Full support with file-based storage
- **WASM**: Core functionality supported, storage requires browser APIs

## Security Considerations

1. **Nonce Management**: Always use cryptographically secure random nonces
2. **Key Rotation**: Rotate signing keys periodically
3. **Spending Limits**: Set conservative limits for auto-pay
4. **Signature Verification**: Always verify signatures before processing
5. **Replay Protection**: Maintain nonce database to detect replays

## Dependencies

- `paykit-lib`: Core payment directory operations
- `paykit-interactive`: Real-time payment execution
- `pubky`: Pubky SDK for distributed storage
- `ed25519-dalek`: Ed25519 signature implementation
- `rust_decimal`: Safe decimal arithmetic
- `fs2`: File-level locking (native only)

## License

See root [LICENSE](../LICENSE) file for details.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## Security

For security issues, see [SECURITY.md](../SECURITY.md) for responsible disclosure.

