# Noise Pattern Selection Guide

This guide explains when to use each Noise pattern in Paykit for secure payment communication.

> **Implementation Status**: All patterns (IK, IK-raw, N, NN, XX) are fully implemented in
> `pubky-noise` v0.8.0 and available via `paykit-demo-core`. The pkarr-based identity discovery
> is implemented in `paykit_demo_core::pkarr_discovery`.

## Pattern Overview

| Pattern | Client Auth | Server Auth | Bidirectional | Use Case |
|---------|-------------|-------------|---------------|----------|
| **IK** | Ed25519 signed | Static key | ✅ Yes | Standard payments |
| **IK-raw** | Via pkarr | Via pkarr | ✅ Yes | Cold key / hardware wallet |
| **N** | Anonymous | Static key | ⚠️ **ONE-WAY** | Donations, anonymous tips |
| **NN** | Anonymous | Anonymous | ✅ Yes | Post-handshake attestation |
| **XX** | TOFU (learned) | TOFU (learned) | ✅ Yes | Trust-on-first-use |

> ⚠️ **N Pattern Critical Limitation**: The N pattern is **ONE-WAY** only. Clients can send
> encrypted messages to the server, but the server **cannot** send encrypted responses back.
> This is a Noise protocol limitation. For bidirectional anonymous communication, use NN pattern.

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

**pkarr Integration (Now Implemented):**
```rust
use paykit_demo_core::pkarr_discovery::{discover_noise_key, publish_noise_key, setup_cold_key};

// One-time cold key setup
let (x25519_sk, x25519_pk) = setup_cold_key(&session, &ed25519_sk, "device").await?;

// Runtime: discover peer's X25519 key from pubky storage
let server_pk = discover_noise_key(&storage, &peer_pubkey, "default").await?;
```

See pubky-noise/docs/COLD_KEY_ARCHITECTURE.md for the complete architecture.

### N (Anonymous Client)

Use N when the client should remain anonymous but needs to verify the server.

> ⚠️ **ONE-WAY ONLY**: The N pattern only supports encryption from client to server.
> The server **cannot** send encrypted responses. For bidirectional anonymous communication,
> use NN pattern instead.

**Security Properties:**
- Server authenticated via static key (verify via pkarr)
- Client uses ephemeral key only (anonymous)
- Server cannot identify client
- **One-way encryption**: Client → Server only

**Use Cases:**
- Donation boxes (no response needed)
- Anonymous tips (client doesn't need confirmation)
- One-way anonymous data submission
- Privacy-first scenarios where client doesn't need encrypted response

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
let (mut channel, server_ephemeral, client_ephemeral) = 
    NoiseRawClientHelper::connect_ephemeral_with_negotiation("127.0.0.1:9735").await?;

// IMPORTANT: Verify server identity via post-handshake message
let attestation = channel.recv().await?;
verify_attestation(&attestation, &expected_server_pk, &server_ephemeral, &client_ephemeral)?;
```

**Attestation Protocol:**
```rust
use paykit_demo_core::attestation::{sign_attestation, verify_attestation};

// After NN handshake, both parties should exchange attestations:

// Server signs attestation (binding Ed25519 identity to ephemeral key)
let attestation = sign_attestation(&ed25519_sk, &client_ephemeral, &server_ephemeral);
channel.send(PaykitNoiseMessage::Attestation {
    ed25519_pk: hex::encode(ed25519_pk),
    signature: hex::encode(attestation),
}).await?;

// Client verifies server attestation
if let PaykitNoiseMessage::Attestation { ed25519_pk, signature } = channel.recv().await? {
    verify_attestation(
        &hex::decode(ed25519_pk)?,
        &hex::decode(signature)?,
        &server_ephemeral,
        &client_ephemeral,
    )?;
}
```

### XX (Trust-On-First-Use)

Use XX when neither party knows the other's static key beforehand. Both parties learn each other's keys during a 3-message handshake.

**Security Properties:**
- Static keys exchanged during handshake
- Trust-on-first-use model
- **Cache static keys for future IK connections**
- Forward secrecy maintained

**Use Cases:**
- First-time contact with new payment recipients
- Discovery scenarios
- Onboarding new contacts
- Scenarios where pkarr lookup isn't available

**CLI Usage:**
```bash
# Receiver
paykit-demo receive --port 9735 --pattern xx

# First-time payer (learns server's key during handshake)
paykit-demo pay newcontact --connect 127.0.0.1:9735@<any> --pattern xx
```

**Code Example:**
```rust
use paykit_demo_core::NoiseRawClientHelper;

// Derive X25519 key
let x25519_sk = NoiseRawClientHelper::derive_x25519_key(&seed, b"device");

// Connect with XX (learns server's static key during handshake)
let (channel, server_static_pk) = NoiseRawClientHelper::connect_xx_with_negotiation(
    &x25519_sk,
    "127.0.0.1:9735",
).await?;

// Cache server_static_pk for future IK connections
save_contact_key("bob", &server_static_pk);

// Next time, use IK pattern with cached key
```

**Server-Side Handling:**
```rust
let (channel, client_static_pk) = NoiseServerHelper::accept_xx(&x25519_sk, stream).await?;
// Server also learns client's static key - can cache for future
```

## Pattern Selection Flowchart

```
                        Start
                          │
                          ▼
                ┌─────────────────────┐
                │ Do you know the     │
                │ peer's static key?  │
                └──────────┬──────────┘
                           │
                   ┌───────┴───────┐
                   │               │
                  Yes              No
                   │               │
                   ▼               │
          ┌────────────────┐       │
          │ Do you need    │       │
          │ authentication?│       ▼
          └───────┬────────┘  ┌─────────┐
                  │           │   XX    │
            ┌─────┴────┐      │ (TOFU)  │
           Yes        No      └─────────┘
            │          │
            ▼          ▼
    ┌──────────────┐  ┌───┐
    │ Are Ed25519  │  │ N │
    │ keys hot?    │  └───┘
    └──────┬───────┘
           │
     ┌─────┴─────┐
     │           │
    Yes          No
     │           │
     ▼           ▼
   ┌───┐     ┌──────┐
   │ IK│     │IK-raw│
   └───┘     └──────┘
   
   Note: Use NN only for testing or when implementing
   post-handshake attestation (see documentation).
```

## Security Comparison

| Property | IK | IK-raw | N | NN | XX |
|----------|-----|--------|---|----|-----|
| Client MITM Protection | Yes | Via pkarr | No | No | TOFU¹ |
| Server MITM Protection | Yes | Via pkarr | Yes | No | TOFU¹ |
| Client Anonymity | No | No | Yes | Yes | No |
| Server Anonymity | No | No | No | Yes | No |
| Forward Secrecy | Yes | Yes | Yes | Yes | Yes |
| Bidirectional Encryption | Yes | Yes | **NO²** | Yes | Yes |
| Requires Hot Ed25519 | Yes | No | No | No | No |
| Static Key Learning | N/A | N/A | N/A | N/A | Yes |

¹ TOFU = Trust-On-First-Use. First connection is vulnerable; subsequent connections secure if key is cached.  
² **N pattern is ONE-WAY**: Client can encrypt to server, but server cannot encrypt responses.

¹ TOFU = Trust-On-First-Use. First connection is vulnerable; subsequent connections secure if key is cached.

## Best Practices

1. **Default to IK** for standard payments between known parties
2. **Use IK-raw** for hardware wallet / cold key deployments
3. **Use N only for one-way donations** (client sends, server cannot respond)
4. **Use XX for first contact** when you don't know the peer's key, then upgrade to IK
5. **Cache static keys** learned from XX handshakes for future IK connections
6. **Avoid NN** unless you implement post-handshake attestation
7. **Always verify server identity** via pkarr before connecting with IK-raw or N
8. **Never trust NN connections** without explicit attestation
9. **⚠️ N pattern warning**: Remember server cannot send encrypted responses - use NN/IK-raw for bidirectional

## Key Management

### Caching Static Keys

After learning a peer's static key from an XX handshake, cache it for future connections:
- XX (first contact) → **cache static key** → IK-raw (subsequent connections)
- 10x faster connections (no handshake key exchange)
- Recommended cache TTL: 30 days
- Validate against pkarr weekly

**See [KEY_CACHING_STRATEGY.md](KEY_CACHING_STRATEGY.md) for complete caching guide.**

### Key Rotation

X25519 keys should be rotated periodically (recommended: 90 days):
- Derive new X25519 key with different context
- Sign with cold Ed25519 key
- Publish to pkarr
- Old sessions continue, new connections use new key

**See [KEY_ROTATION.md](KEY_ROTATION.md) for rotation procedures.**

## Related Documentation

- [KEY_CACHING_STRATEGY.md](KEY_CACHING_STRATEGY.md) - How to cache and validate keys
- [KEY_ROTATION.md](KEY_ROTATION.md) - Key rotation and revocation procedures
- [pubky-noise Cold Key Architecture](../../pubky-noise/docs/COLD_KEY_ARCHITECTURE.md)
- [pubky-noise README](../../pubky-noise/README.md)
- [Demo Scripts](../paykit-demo-cli/demos/README.md)

