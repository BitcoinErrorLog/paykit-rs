# Paykit Threat Model & Security Architecture

**Project**: paykit-rs v0.2.0  
**Document Version**: 1.0  
**Last Updated**: December 4, 2025

---

## Overview

This document describes the security architecture and threat model for Paykit, a payment coordination protocol built on the Pubky ecosystem. Paykit inherits cryptographic guarantees from [pubky-noise](../../pubky-noise/THREAT_MODEL.md) and adds payment-specific security considerations.

## Security Objectives

1. **Confidentiality** - Payment details encrypted in transit
2. **Authenticity** - Verify payer and payee identities  
3. **Integrity** - Receipts cannot be forged or tampered
4. **Non-repudiation** - Signed receipts provide proof of agreement
5. **Privacy** - Payment patterns not revealed to network observers

---

## System Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Application Layer                          │
│  (CLI, Web Demo, Bitkit integration)                         │
├──────────────────────────────────────────────────────────────┤
│                    paykit-demo-core                           │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Identity Management (Ed25519 keypairs)                │  │
│  │  Pattern Selection (IK, IK-raw, N, NN, XX)            │  │
│  │  NN Attestation Protocol                               │  │
│  └────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────┤
│                    paykit-interactive                         │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Receipt Exchange (RequestReceipt, ConfirmReceipt)     │  │
│  │  Encrypted Channels (PubkyNoiseChannel)                │  │
│  └────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────┤
│                    pubky-noise                                │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Noise Protocol (IK, IK-raw, N, NN, XX patterns)       │  │
│  │  Identity Binding (Ed25519 signatures)                 │  │
│  │  Cold Key Support (pkarr integration)                  │  │
│  └────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────┤
│                    Transport Layer                            │
│  (TCP sockets, WebSocket, HTTP)                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Trust Model

### Trusted Components

- **pubky-noise cryptography** - See [pubky-noise THREAT_MODEL](../../pubky-noise/THREAT_MODEL.md)
- **Ed25519/X25519** - Standard elliptic curve cryptography
- **Pubky SDK** - Homeserver communication and pkarr operations
- **Local key storage** - Application must protect private keys

### Untrusted Components

- **Network** - All traffic assumed hostile
- **Peers** - Until authenticated via pattern-appropriate mechanism
- **Homeservers** - Trust only for availability, not confidentiality

---

## Pattern-Specific Security Properties

### IK Pattern (Standard Payments)

| Property | Status | Notes |
|----------|--------|-------|
| Client Authentication | ✅ Full | Ed25519 signature in handshake |
| Server Authentication | ✅ Full | X25519 static key binding |
| MITM Protection | ✅ Full | Both parties authenticated |
| Privacy | ✅ High | No identity leaked to observers |
| Ed25519 Required | At handshake | Keys must be accessible |

**Use for**: Standard payment flows where both parties have online keys

### IK-raw Pattern (Cold Key)

| Property | Status | Notes |
|----------|--------|-------|
| Client Authentication | ⚠️ Deferred | Via pkarr lookup |
| Server Authentication | ✅ Full | X25519 static key |
| MITM Protection | ✅ With pkarr | Depends on pkarr verification |
| Privacy | ✅ High | No identity in handshake |
| Ed25519 Required | Never (at runtime) | Published to pkarr once |

**Use for**: Cold key scenarios (Bitkit integration)

**Security Requirement**: Caller MUST verify pkarr record before trusting connection

### N Pattern (Anonymous Donations)

| Property | Status | Notes |
|----------|--------|-------|
| Client Authentication | ❌ None | Client is anonymous |
| Server Authentication | ✅ Full | Via pkarr-published key |
| MITM Protection | ✅ Server only | Client cannot detect MITM |
| Privacy | ✅ Client anonymous | Server known |
| Bidirectional | ⚠️ **NO** | One-way encryption only |

**Use for**: Anonymous donation boxes, anonymous tips

**Critical Limitation**: Server cannot send encrypted responses. Use NN for bidirectional anonymous communication.

### NN Pattern (Ephemeral)

| Property | Status | Notes |
|----------|--------|-------|
| Client Authentication | ❌ None (handshake) | Post-handshake attestation required |
| Server Authentication | ❌ None (handshake) | Post-handshake attestation required |
| MITM Protection | ❌ Vulnerable | Until attestation verified |
| Privacy | ✅ Maximum | Only ephemeral keys used |
| Bidirectional | ✅ Yes | Full duplex after handshake |

**Use for**: Scenarios requiring maximum privacy with explicit attestation

**Security Requirement**: MUST implement post-handshake attestation

### XX Pattern (Trust-On-First-Use)

| Property | Status | Notes |
|----------|--------|-------|
| Client Authentication | ⚠️ TOFU | Key learned during handshake |
| Server Authentication | ⚠️ TOFU | Key learned during handshake |
| MITM Protection | ❌ First contact | ✅ After key cached |
| Privacy | ✅ Moderate | Keys exchanged encrypted |
| Key Learning | ✅ Yes | Cache keys for future IK |

**Use for**: First contact scenarios, key discovery

---

## Attestation Protocol Security

For NN pattern connections, Paykit implements post-handshake attestation:

### Protocol

1. Both parties complete NN handshake (ephemeral keys only)
2. Server signs `SHA256(domain || server_ephemeral || client_ephemeral)`
3. Server sends `Attestation { ed25519_pk, signature }`
4. Client verifies signature matches expected server identity (from pkarr/contact)
5. Client signs with swapped ephemeral order
6. Client sends `Attestation { ed25519_pk, signature }`
7. Server verifies client signature

### Security Properties

- **Replay Prevention**: Each handshake has unique ephemeral keys
- **Binding**: Signature over both ephemerals binds to specific session
- **Identity Proof**: Valid signature proves Ed25519 key possession
- **Order Sensitivity**: Different signing order prevents reflection attacks

### Domain Separator

```
pubky-noise-nn-attestation-v1:
```

---

## Payment-Specific Threats

### 1. Receipt Forgery

**Threat**: Attacker creates fake receipt for payment not made

**Mitigations**:
- Receipts include both payer and payee public keys
- Application-layer signatures over receipts
- Noise channel authenticates both parties

**Residual Risk**: LOW (with proper implementation)

### 2. Double Spending Coordination

**Threat**: Payer coordinates payment with multiple parties simultaneously

**Mitigations**:
- Out of scope for Paykit (handled by payment rail - Bitcoin/Lightning)
- Receipts are coordination, not payment execution

### 3. Payment Amount Manipulation

**Threat**: MITM modifies payment amount during coordination

**Mitigations**:
- All messages encrypted and authenticated via Noise
- Receipt includes amount; tampering breaks AEAD

### 4. Identity Spoofing

**Threat**: Attacker claims to be legitimate payment recipient

**Mitigations**:
- IK pattern: Ed25519 signature required
- IK-raw pattern: pkarr record verification
- N/NN patterns: Explicit attestation or anonymous by design

### 5. Replay Attacks

**Threat**: Old payment requests replayed to extract funds

**Mitigations**:
- Noise ephemeral keys unique per session
- Receipt IDs should be unique (application responsibility)
- Timestamps recommended in application layer

---

## Cold Key Architecture Security

For Bitkit integration with cold Ed25519 keys:

### Key Publication (One-Time, Offline Possible)

1. Derive X25519 from Ed25519: `x25519_sk = HKDF-SHA512(ed25519_sk, device_id)`
2. Sign binding: `sig = Ed25519.sign(ed25519_sk, "pubky-noise-pkarr-key-v1:" || x25519_pk || device_id)`
3. Publish to pubky: `/pub/pubky-noise/keys/noise/x25519/{device_id}.txt`
4. Store Ed25519 key offline

### Runtime Verification (No Ed25519 Needed)

1. Lookup pkarr record for peer
2. Verify Ed25519 signature over X25519 binding
3. Check timestamp freshness (recommended: < 7 days)
4. Connect with IK-raw using verified X25519 key

### Threat: Stale Key Compromise

**Threat**: Old X25519 key compromised, attacker impersonates user

**Mitigations**:
- Timestamp in pkarr record enables freshness checking
- `parse_and_verify_with_expiry` enforces max age
- Recommendation: Re-publish keys periodically (e.g., weekly)

---

## Demo Application Security

### Limitations (NOT FOR PRODUCTION)

The demo applications (`paykit-demo-cli`, `paykit-demo-web`) have intentional security limitations:

1. **Plaintext key storage** - Private keys in JSON files
2. **No secure enclave** - No HSM/TEE integration
3. **Simplified authentication** - No rate limiting or CAPTCHA
4. **Local storage** - Browser localStorage for web demo

### Production Requirements

For production deployment:

1. Use OS-level secure storage (Keychain, Credential Manager)
2. Consider HSM for high-value keys
3. Implement rate limiting
4. Add audit logging
5. Use TLS for homeserver communication
6. Implement proper session management

---

## Related Documentation

- [pubky-noise Threat Model](../../pubky-noise/THREAT_MODEL.md)
- [Cold Key Architecture](../../pubky-noise/docs/COLD_KEY_ARCHITECTURE.md)
- [Pattern Selection Guide](PATTERN_SELECTION.md)
- [Bitkit Integration](BITKIT_INTEGRATION.md)
- [Security Guide](../SECURITY.md)

