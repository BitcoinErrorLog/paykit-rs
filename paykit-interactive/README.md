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

- **transport**: `PubkyNoiseChannel` implementation for encrypted communication
- **rate_limit**: `HandshakeRateLimiter` for DoS protection
- **manager**: `PaykitInteractiveManager` for payment flow orchestration
- **storage**: `PaykitStorage` trait for receipt persistence

## Documentation

- [Build Instructions](BUILD.md) (if exists)
- [Repository Root README](../README.md)
- [Paykit Roadmap](../PAYKIT_ROADMAP.md)

