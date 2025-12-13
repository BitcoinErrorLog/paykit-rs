# Paykit Interactive Layer

This crate (`paykit-interactive`) implements the interactive payment protocols and receipt exchange mechanisms for Paykit, designed to run over encrypted channels like Pubky Noise.

It extends the basic public directory functionality of `paykit-lib` to support private, peer-to-peer payment negotiation and proof-of-payment receipts.

## Core Concepts

### 1. PaykitReceipt

A shared, cryptographic receipt that serves as proof of payment. It includes:
- **Payer & Payee**: Public keys of the participants.
- **Method**: The payment method used (e.g., Lightning, Onchain).
- **Metadata**: Arbitrary JSON for invoice details, order IDs, or shipping info.
- **Receipt ID**: Unique identifier for the transaction.

### 2. PaykitNoiseMessage

The wire protocol for exchanging payment information over an encrypted channel:
- **OfferPrivateEndpoint**: Share a private payment address not visible in the public directory.
- **RequestReceipt**: Payer initiates a transaction and requests a receipt.
- **ConfirmReceipt**: Payee validates and signs/confirms the receipt.

### 3. PaykitNoiseChannel

An abstraction for the secure transport layer, implemented using `pubky-noise` to provide end-to-end encryption and mutual authentication using Pubky identities.

See [NOISE_INTEGRATION.md](../docs/NOISE_INTEGRATION.md) for detailed integration documentation.

## Usage Example

```rust
use paykit_interactive::{PaykitInteractiveManager, PaykitReceipt, ReceiptGenerator, PaykitStorage};
use paykit_lib::{MethodId, PublicKey};
use std::sync::Arc;

// 1. Implement storage and generator traits
let storage = Arc::new(Box::new(MyStorage::new()) as Box<dyn PaykitStorage>);
let generator = Arc::new(Box::new(MyReceiptGenerator::new()) as Box<dyn ReceiptGenerator>);

// 2. Create manager
let manager = PaykitInteractiveManager::new(storage, generator);

// 3. Establish Noise channel (using pubky-noise)
let mut channel = PubkyNoiseChannel::connect(&client, stream, &server_pk).await?;

// 4. Create provisional receipt
let receipt = PaykitReceipt::new(
    "receipt_001".to_string(),
    payer_pk,
    payee_pk,
    MethodId("lightning".to_string()),
    Some("1000".to_string()),
    Some("SAT".to_string()),
    json!({"order_id": "ABC123"}),
);

// 5. Initiate payment (payer side)
let final_receipt = manager.initiate_payment(&mut channel, receipt).await?;

// 6. Handle incoming messages (payee side)
let msg = channel.recv().await?;
if let Some(response) = manager.handle_message(msg, &peer_pk, &my_pk).await? {
    channel.send(response).await?;
}
```

For a complete example, see `examples/complete_payment_flow.rs`:
```bash
cargo run --example complete_payment_flow
```

## Message Sequence Diagram

```
Payer                           Payee
  |                              |
  |  OfferPrivateEndpoint        |
  | <----------------------------|
  |                              |
  |  RequestReceipt              |
  |----------------------------->|
  |                              | (generates invoice)
  |  ConfirmReceipt              |
  |<-----------------------------|
  |                              |
  | (both save receipt)          |
```

## Related Components

This crate extends the functionality of other Paykit components:

- **[paykit-lib](../paykit-lib/README.md)** - Uses transport traits from the core library for directory operations
- **[paykit-subscriptions](../paykit-subscriptions/README.md)** - May be used together for subscription-based payments
- **[paykit-demo-core](../paykit-demo-core/)** - Shared demo logic that may use interactive features
- **[paykit-demo-cli](../paykit-demo-cli/README.md)** - CLI demo that demonstrates interactive payment flows

## Integration Plan

This crate is part of the **Paykit Roadmap** (Phase 2 & 3 - ✅ Complete). It is designed to be integrated into Bitkit (via `bitkit-core`) and other Pubky apps to enable:
- Private payment negotiation.
- Interactive checkout flows.
- Standardized transaction history.

See [PAYKIT_ROADMAP.md](../PAYKIT_ROADMAP.md) for the full integration plan.

## Testing

Run the test suite:
```bash
cargo test --all-features
```

## Features

- `timeout` (default): Enables 30-second timeout for receipt negotiations using `tokio::time`
- Disable for environments without tokio runtime

## Modules

### Core Modules

- **transport**: `PubkyNoiseChannel` implementation for encrypted communication (TCP and WebSocket)
- **rate_limit**: `HandshakeRateLimiter` for DoS protection with configurable limits
- **connection_limit**: Connection limiting to prevent resource exhaustion
- **manager**: `PaykitInteractiveManager` for payment flow orchestration
- **storage**: `PaykitStorage` trait for receipt persistence with smart checkout helpers

### Advanced Features

- **metadata**: Metadata validation and parsing for orders, shipping, taxes, and payments
- **proof**: Payment proof generation and verification with multiple proof types
- **status**: Payment status tracking and lifecycle management
- **metrics**: Performance metrics and monitoring for payment flows

### Smart Checkout

The storage module includes smart checkout helpers that automatically select the best payment method:

```rust
use paykit_interactive::storage::{smart_checkout, CheckoutResult};

// Automatically select best method from available options
let result = smart_checkout(
    &supported_payments,
    &amount,
    &user_preferences,
).await?;

match result {
    CheckoutResult::Success { method, endpoint } => {
        // Proceed with payment using selected method
    }
    CheckoutResult::NoMethodAvailable => {
        // Handle no available methods
    }
    CheckoutResult::Error(e) => {
        // Handle error
    }
}
```

### Metadata Validation

Validate and parse structured metadata for payment requests:

```rust
use paykit_interactive::metadata::{OrderMetadata, ShippingMetadata, PaymentMetadata};

let order = OrderMetadata::new("order_123")
    .with_items(vec![/* items */])
    .with_total(Amount::from_sats(10000));

let shipping = ShippingMetadata::new(ShippingAddress::new(/* ... */))
    .with_method(ShippingMethod::Standard);

let payment_metadata = PaymentMetadata::new()
    .with_order(order)
    .with_shipping(shipping);
```

### Payment Proof

Generate and verify cryptographic proofs of payment:

```rust
use paykit_interactive::proof::{PaymentProof, ProofType, ProofVerifier};

// Generate proof after payment
let proof = PaymentProof::new(
    ProofType::LightningPreimage,
    preimage_bytes,
    &receipt,
)?;

// Verify proof
let verifier = ProofVerifierRegistry::default();
let result = verifier.verify(&proof, &receipt)?;
assert!(result.is_valid());
```

### Status Tracking

Track payment status through the lifecycle:

```rust
use paykit_interactive::status::{PaymentStatus, PaymentStatusTracker};

let tracker = PaymentStatusTracker::new();
tracker.update_status(&receipt_id, PaymentStatus::Pending).await?;
tracker.update_status(&receipt_id, PaymentStatus::Confirmed).await?;

let status = tracker.get_status(&receipt_id).await?;
```

## Transport Support

The crate supports multiple transport backends:

- **TCP**: Direct TCP connections with Noise protocol
- **WebSocket**: WebSocket connections for browser and web applications
- **Custom**: Implement `PaykitNoiseChannel` trait for custom transports

## Documentation

- [Build Instructions](BUILD.md)
- [Noise Integration Guide](../docs/NOISE_INTEGRATION.md)
- [Repository Root README](../README.md)
- [Paykit Roadmap](../PAYKIT_ROADMAP.md)

