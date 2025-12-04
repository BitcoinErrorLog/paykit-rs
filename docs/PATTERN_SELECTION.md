# Noise Pattern Selection Guide

This guide explains when to use each Noise pattern in Paykit for secure payment communication.

## Pattern Overview

| Pattern | Client Auth | Server Auth | Use Case |
|---------|-------------|-------------|----------|
| **IK** | Ed25519 signed | Static key | Standard payments |
| **IK-raw** | Via pkarr | Via pkarr | Cold key / hardware wallet |
| **N** | Anonymous | Static key | Donations, anonymous tips |
| **NN** | Anonymous | Anonymous | Post-handshake attestation |

## When to Use Each Pattern

### IK (Interactive Key) - Default

Use IK when both parties have Ed25519 keys available at handshake time.

**Security Properties:**
- Full mutual authentication
- Identity binding via Ed25519 signatures
- MITM protection built-in

**Use Cases:**
- Standard payments between identified Pubky users
- Any scenario where both parties can sign with Ed25519

**CLI Usage:**
```bash
# Receiver
paykit-demo receive --port 9735 --pattern ik

# Payer
paykit-demo pay bob --connect 127.0.0.1:9735@<pubkey> --pattern ik
```

**Code Example:**
```rust
use paykit_demo_core::NoiseClientHelper;

let channel = NoiseClientHelper::connect_to_recipient(
    &identity,
    "127.0.0.1:9735",
    &server_static_pk,
).await?;
```

### IK-raw (Cold Key Scenario)

Use IK-raw when Ed25519 keys are kept "cold" (offline) and identity is verified via pkarr.

**Security Properties:**
- Both parties authenticated via external mechanism (pkarr)
- No Ed25519 signing during handshake
- MITM protection depends on pkarr verification

**Use Cases:**
- Hardware wallet integration (Bitkit)
- Cold storage scenarios
- High-security deployments

**How It Works:**
1. Server publishes X25519 key to pkarr (one-time cold signing)
2. Client looks up server's X25519 key from pkarr
3. Client initiates IK-raw handshake (no new signing required)
4. Identity is verified via the pkarr signature chain

**CLI Usage:**
```bash
# Receiver
paykit-demo receive --port 9735 --pattern ik-raw

# Payer
paykit-demo pay bob --connect 127.0.0.1:9735@<pubkey> --pattern ik-raw
```

**Code Example:**
```rust
use paykit_demo_core::NoiseRawClientHelper;
use zeroize::Zeroizing;

// Derive X25519 key (do once, publish to pkarr)
let x25519_sk = NoiseRawClientHelper::derive_x25519_key(&seed, b"device");

// Connect without Ed25519 signing (pattern byte sent automatically)
let channel = NoiseRawClientHelper::connect_ik_raw_with_negotiation(
    &x25519_sk,
    "127.0.0.1:9735",
    &server_pk_from_pkarr,
).await?;
```

**pkarr Integration:**
```
See pubky-noise/docs/COLD_KEY_ARCHITECTURE.md for the pkarr record format.
```

### N (Anonymous Client)

Use N when the client should remain anonymous but needs to verify the server.

**Security Properties:**
- Server authenticated via static key (verify via pkarr)
- Client uses ephemeral key only (anonymous)
- Server cannot identify client

**Use Cases:**
- Donation boxes
- Anonymous tips
- Privacy-first payment scenarios
- Receiving payments without revealing identity

**CLI Usage:**
```bash
# Donation box receiver
paykit-demo receive --port 9735 --pattern n

# Anonymous donor
paykit-demo pay donations --connect 127.0.0.1:9735@<pubkey> --pattern n
```

**Code Example:**
```rust
use paykit_demo_core::NoiseRawClientHelper;

// No identity needed - client is anonymous
let channel = NoiseRawClientHelper::connect_anonymous_with_negotiation(
    "127.0.0.1:9735",
    &server_pk_from_pkarr,
).await?;
```

### NN (Fully Anonymous)

Use NN when neither party should be authenticated during handshake.

**Security Properties:**
- Both parties use ephemeral keys
- No built-in authentication
- **MITM VULNERABLE** without post-handshake attestation

**Use Cases:**
- Post-handshake attestation scenarios
- Testing and development
- Scenarios with external authentication

**Warning:** NN pattern requires explicit post-handshake verification!

**CLI Usage:**
```bash
# Receiver
paykit-demo receive --port 9735 --pattern nn

# Payer
paykit-demo pay <recipient> --connect 127.0.0.1:9735@<pubkey> --pattern nn
```

**Code Example:**
```rust
use paykit_demo_core::NoiseRawClientHelper;

// Connect anonymously
let (mut channel, server_ephemeral) = NoiseRawClientHelper::connect_ephemeral_with_negotiation(
    "127.0.0.1:9735",
).await?;

// IMPORTANT: Verify server identity via post-handshake message
let attestation = channel.recv().await?;
verify_attestation(&attestation, &expected_server_pk)?;
```

## Pattern Selection Flowchart

```
                Start
                  │
                  ▼
        ┌─────────────────────┐
        │ Do you need client  │
        │ authentication?      │
        └──────────┬──────────┘
                   │
           ┌───────┴───────┐
           │               │
          Yes              No
           │               │
           ▼               ▼
    ┌──────────────┐  ┌──────────────┐
    │ Are Ed25519  │  │ Is server    │
    │ keys hot?    │  │ authenticated?│
    └──────┬───────┘  └──────┬───────┘
           │                 │
     ┌─────┴─────┐     ┌─────┴─────┐
     │           │     │           │
    Yes          No   Yes          No
     │           │     │           │
     ▼           ▼     ▼           ▼
   ┌───┐     ┌──────┐ ┌───┐    ┌────┐
   │ IK│     │IK-raw│ │ N │    │ NN │
   └───┘     └──────┘ └───┘    └────┘
```

## Security Comparison

| Property | IK | IK-raw | N | NN |
|----------|-----|--------|---|-----|
| Client MITM Protection | Yes | Via pkarr | No | No |
| Server MITM Protection | Yes | Via pkarr | Yes | No |
| Client Anonymity | No | No | Yes | Yes |
| Server Anonymity | No | No | No | Yes |
| Forward Secrecy | Yes | Yes | Yes | Yes |
| Requires Hot Ed25519 | Yes | No | No | No |

## Best Practices

1. **Default to IK** for standard payments between known parties
2. **Use IK-raw** for hardware wallet / cold key deployments
3. **Use N** for accepting anonymous donations
4. **Avoid NN** unless you implement post-handshake attestation
5. **Always verify server identity** via pkarr before connecting with IK-raw or N
6. **Never trust NN connections** without explicit attestation

## Related Documentation

- [pubky-noise Cold Key Architecture](../pubky-noise/docs/COLD_KEY_ARCHITECTURE.md)
- [pubky-noise README](../pubky-noise/README.md)
- [Demo Scripts](paykit-demo-cli/demos/README.md)

