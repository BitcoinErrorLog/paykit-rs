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

The secure transport layer implemented using `pubky-noise` for end-to-end encryption and mutual authentication using Pubky identities.

### 4. Noise Pattern Support

This crate re-exports `NoisePattern` from `pubky-noise` for selecting the appropriate handshake pattern:

| Pattern | Use Case | Authentication |
|---------|----------|----------------|
| **IK** | Standard payments | Full Ed25519 binding in handshake |
| **IK-raw** | Cold key scenarios | Identity via pkarr lookup |
| **N** | Anonymous donations | Client anonymous, server authenticated |
| **NN** | Ephemeral exchange | Post-handshake attestation required |
| **XX** | Trust-on-first-use | Static keys learned during handshake |

```rust
use paykit_interactive::NoisePattern;

// Parse from string
let pattern: NoisePattern = "ik-raw".parse()?;

// Get negotiation byte for wire protocol
let byte = pattern.negotiation_byte(); // 0x01 for IK-raw
```

### 5. Attestation Message

For NN (ephemeral) connections, use the `Attestation` message type for post-handshake identity verification:

```rust
use paykit_interactive::PaykitNoiseMessage;

// After NN handshake, exchange attestations
channel.send(PaykitNoiseMessage::Attestation {
    ed25519_pk: hex::encode(&my_ed25519_pk),
    signature: hex::encode(&attestation_signature),
}).await?;
```

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

## Documentation

- [Build Instructions](BUILD.md) (if exists)
- [Repository Root README](../README.md)
- [Paykit Roadmap](../PAYKIT_ROADMAP.md)

