# Security Documentation

This document provides comprehensive security documentation for the Paykit payment protocol implementation.

## Table of Contents

1. [Security Model Overview](#security-model-overview)
2. [Cryptographic Decisions](#cryptographic-decisions)
3. [Threat Model](#threat-model)
4. [Security Architecture](#security-architecture)
5. [Key Management](#key-management)
6. [Transport Security](#transport-security)
7. [Storage Security](#storage-security)
8. [Rate Limiting & DoS Protection](#rate-limiting--dos-protection)
9. [Security Best Practices](#security-best-practices)
10. [Audit Preparation](#audit-preparation)

---

## Security Model Overview

Paykit is a payment coordination protocol built on top of Pubky's identity and storage system. The security model relies on:

1. **Identity**: Ed25519 public keys from Pubky for identity
2. **Transport**: Noise Protocol (IK pattern) for encrypted P2P communication
3. **Storage**: Pubky homeservers for persistent data with signature verification
4. **Key Derivation**: HKDF-based key derivation for session keys

### Trust Assumptions

- **Pubky Homeservers**: Semi-trusted for availability, not confidentiality
- **Network**: Untrusted, all communication encrypted
- **Endpoints**: User devices are trusted to protect private keys
- **Time**: Loose time synchronization assumed for nonce/expiry checks

---

## Cryptographic Decisions

### Algorithm Choices

| Purpose | Algorithm | Rationale |
|---------|-----------|-----------|
| Identity Keys | Ed25519 | Industry standard, Pubky compatibility |
| Key Agreement | X25519 | Curve25519 ECDH, Noise Protocol standard |
| Symmetric Encryption | ChaCha20-Poly1305 | AEAD, constant-time, mobile-friendly |
| Hashing | BLAKE2b / SHA-256 | BLAKE2b for KDF, SHA-256 for compatibility |
| Key Derivation | HKDF-SHA256 | RFC 5869 compliant |

### Noise Protocol Configuration

```
Noise_IK_25519_ChaChaPoly_BLAKE2b
```

- **IK Pattern**: Initiator knows responder's static key (from Pubky directory)
- **25519**: X25519 for key agreement
- **ChaChaPoly**: ChaCha20-Poly1305 AEAD
- **BLAKE2b**: Hash function

### Why Noise_IK?

1. **One Round Trip**: Efficient for mobile networks
2. **Identity Hiding**: Initiator identity protected from passive observers
3. **Forward Secrecy**: Ephemeral keys provide PFS
4. **Known Responder**: Directory lookup provides responder's public key

### Key Zeroization

All sensitive key material is zeroized after use:

```rust
use zeroize::Zeroize;

// Private keys implement Zeroize and ZeroizeOnDrop
struct PrivateKey([u8; 32]);

impl Drop for PrivateKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}
```

### Checked Arithmetic

Financial calculations use checked arithmetic to prevent overflow:

```rust
// Amount operations use checked_add/checked_sub
let total = amount1.checked_add(&amount2)
    .ok_or(Error::Overflow)?;
```

---

## Threat Model

### Adversary Capabilities

| Adversary Type | Capabilities |
|----------------|--------------|
| Passive Network | Eavesdrop on all network traffic |
| Active Network | MITM, inject/modify packets, DoS |
| Compromised Homeserver | Read/write public storage, DoS |
| Malicious Peer | Send malformed messages, DoS |

### Assets Protected

1. **Private Keys**: Ed25519 signing keys, X25519 session keys
2. **Payment Endpoints**: Private payment addresses (Lightning invoices, etc.)
3. **Transaction History**: Payment receipts and metadata
4. **Identity Correlation**: Prevent linking payments to real identities

### Threat Categories

#### T1: Key Compromise

**Threat**: Attacker obtains user's private key

**Mitigations**:
- Platform secure storage (Keychain, Keystore, Credential Manager)
- Key zeroization after use
- No key logging or persistence in plaintext

#### T2: Transport Interception

**Threat**: Attacker intercepts/modifies messages in transit

**Mitigations**:
- Noise Protocol encryption with forward secrecy
- AEAD prevents tampering
- Session binding prevents replay across sessions

#### T3: Replay Attacks

**Threat**: Attacker replays old messages

**Mitigations**:
- Monotonic counters for storage-backed messaging
- Session-bound nonces
- Timestamp checks with reasonable tolerance

#### T4: Denial of Service

**Threat**: Attacker overwhelms system resources

**Mitigations**:
- Per-peer rate limiting
- Handshake rate limiting
- Connection limits
- Exponential backoff

#### T5: Information Leakage

**Threat**: Side-channel leaks reveal sensitive data

**Mitigations**:
- Constant-time operations for crypto
- Minimal logging of sensitive data
- Secure memory handling (optional `secure-mem` feature)

---

## Security Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Application Layer                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │   Paykit    │  │  Subscript  │  │   Receipt   │              │
│  │  Directory  │  │   Manager   │  │  Generator  │              │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
└─────────┼────────────────┼────────────────┼─────────────────────┘
          │                │                │
┌─────────┼────────────────┼────────────────┼─────────────────────┐
│         │         Transport Layer         │                      │
│  ┌──────▼──────────────────────────────────▼──────┐             │
│  │              Noise Protocol (IK)               │             │
│  │  ┌─────────────┐  ┌─────────────────────────┐  │             │
│  │  │  Handshake  │  │  Encrypted Transport    │  │             │
│  │  │  (X25519)   │  │  (ChaCha20-Poly1305)    │  │             │
│  │  └─────────────┘  └─────────────────────────┘  │             │
│  └────────────────────────────────────────────────┘             │
└─────────────────────────────────────────────────────────────────┘
          │                                │
┌─────────┼────────────────────────────────┼──────────────────────┐
│         │           Storage Layer        │                       │
│  ┌──────▼──────┐              ┌──────────▼───────┐              │
│  │   Pubky     │              │  Secure Storage  │              │
│  │  Homeserver │              │  (Platform KMS)  │              │
│  └─────────────┘              └──────────────────┘              │
└─────────────────────────────────────────────────────────────────┘
```

### Component Security Responsibilities

| Component | Security Responsibility |
|-----------|------------------------|
| paykit-lib | Core crypto, key management, transport abstraction |
| paykit-interactive | Noise handshake, encrypted messaging |
| paykit-subscriptions | Payment request signing, verification |
| pubky-noise | Low-level Noise implementation |

---

## Key Management

### Key Hierarchy

```
Ed25519 Identity Key (from Pubky)
    │
    ├── X25519 Static Key (derived for Noise)
    │       │
    │       └── Session Keys (ephemeral, per-connection)
    │               │
    │               ├── Encryption Key
    │               └── MAC Key
    │
    └── Signing Key (for receipts, subscriptions)
```

### Key Storage

| Platform | Storage Mechanism | Protection Level |
|----------|-------------------|------------------|
| iOS | Keychain Services | Hardware (Secure Enclave when available) |
| Android | Android Keystore | Hardware (StrongBox when available) |
| macOS | Keychain | Software + hardware options |
| Windows | Credential Manager | Software |
| Linux | Secret Service API | Software |
| Web | IndexedDB + SubtleCrypto | Software |

### Key Rotation

- **Session Keys**: Rotated per connection
- **Static Keys**: User-initiated rotation via Pubky
- **Epoch-based Rotation**: Supported in pubky-noise for forward secrecy

---

## Transport Security

### Noise Protocol Handshake

```
Initiator                                    Responder
    |                                            |
    |  -> e, es, s, ss                           |
    |     (ephemeral, static encrypted)          |
    |                                            |
    |  <- e, ee, se                              |
    |     (ephemeral, key confirmation)          |
    |                                            |
    |  <-> Encrypted Transport                   |
    |                                            |
```

### Session Properties

- **Forward Secrecy**: Compromise of long-term keys doesn't reveal past sessions
- **Identity Hiding**: Initiator's identity encrypted under responder's key
- **Mutual Authentication**: Both parties authenticated by end of handshake

### Message Format

```
┌────────────────────────────────────────────┐
│  Length (4 bytes, big-endian)              │
├────────────────────────────────────────────┤
│  Ciphertext                                │
│  ┌──────────────────────────────────────┐  │
│  │  Plaintext (variable)                │  │
│  ├──────────────────────────────────────┤  │
│  │  Authentication Tag (16 bytes)       │  │
│  └──────────────────────────────────────┘  │
└────────────────────────────────────────────┘
```

---

## Storage Security

### Pubky Storage

Data stored on Pubky homeservers is:
- **Signed**: All writes signed by owner's Ed25519 key
- **Versioned**: Content-addressed for integrity
- **Public/Private**: Access controlled by path conventions

### Local Storage

Sensitive data stored locally uses platform-specific secure storage:

```rust
// Example: Store private key
secure_storage.store("payment_key", &key_bytes, StorageOptions {
    require_authentication: true,
    accessibility: Accessibility::WhenUnlocked,
}).await?;
```

### Storage-Backed Messaging

For asynchronous messaging:
- Messages encrypted with Noise session keys
- Stored in sender's Pubky repository
- Receiver polls and decrypts
- Monotonic counters prevent replay

---

## Rate Limiting & DoS Protection

### Handshake Rate Limiting

```rust
let config = RateLimitConfig {
    max_handshakes_per_minute: 10,
    max_handshakes_per_hour: 100,
    cooldown_seconds: 60,
};

let limiter = HandshakeRateLimiter::new(config);
```

### Per-Peer Limits

| Resource | Limit | Scope |
|----------|-------|-------|
| Handshakes | 10/min | Per IP |
| Messages | 100/min | Per session |
| Subscriptions | 50 active | Per identity |
| Payment Requests | 20/hour | Per peer pair |

### Connection Limits

```rust
// Maximum concurrent connections
const MAX_CONNECTIONS: usize = 1000;
const MAX_CONNECTIONS_PER_IP: usize = 10;
```

---

## Security Best Practices

### For Developers

1. **Never log sensitive data**: Keys, tokens, payment addresses
2. **Use checked arithmetic**: All financial calculations
3. **Validate all inputs**: Especially from network
4. **Handle errors securely**: Don't leak internal state
5. **Keep dependencies updated**: Regular security updates

### For Operators

1. **Enable rate limiting**: Protect against DoS
2. **Monitor for anomalies**: Unusual traffic patterns
3. **Regular key rotation**: Follow Pubky key rotation practices
4. **Secure backups**: Encrypt backup data

### For Users

1. **Protect your device**: Use device encryption, biometrics
2. **Verify recipients**: Check public keys before payment
3. **Review permissions**: Subscription auto-pay limits
4. **Report issues**: Security bugs to maintainers

---

## Audit Preparation

### Audit Scope Recommendations

1. **Critical Path Review**:
   - Noise handshake implementation
   - Key derivation functions
   - Signature verification
   - Amount calculations

2. **Focus Areas**:
   - `pubky-noise/src/datalink_adapter.rs` - Handshake logic
   - `pubky-noise/src/noise_link.rs` - Encryption/decryption
   - `paykit-subscriptions/src/request.rs` - Payment request handling
   - `paykit-lib/src/secure_storage/` - Key storage

3. **Test Vectors**:
   - See `pubky-noise/tests/` for crypto test vectors
   - Noise Protocol test vectors from specification

### Security Checklist

- [x] All crypto operations use vetted libraries (snow, ed25519-dalek)
- [x] Key zeroization implemented
- [x] Checked arithmetic for financial operations
- [x] Rate limiting for handshakes and messages
- [x] Platform secure storage integration
- [x] No plaintext key storage
- [x] Forward secrecy via ephemeral keys
- [x] Replay protection via nonces/counters

### Known Limitations

1. **Session Resumption**: Not implemented, requires full handshake
2. **Key Revocation**: Relies on Pubky key rotation
3. **Metadata Protection**: Message timing/size may leak information

---

## Reporting Security Issues

For security vulnerabilities, please:

1. **Do NOT** open a public issue
2. Contact maintainers privately
3. Provide detailed reproduction steps
4. Allow reasonable time for fix before disclosure

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-01-XX | Initial security documentation |
