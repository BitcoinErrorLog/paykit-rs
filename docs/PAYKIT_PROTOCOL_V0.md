# Paykit Protocol v0 Specification

> **Version**: 0.1  
> **Last Updated**: January 2, 2026  
> **Status**: Canonical Specification

This document is the canonical specification for Paykit Protocol v0. All implementations (Rust, Kotlin, Swift, TypeScript) **must** conform to this spec.

**Related Documents**:
- [SEALED_BLOB_V1_SPEC.md](SEALED_BLOB_V1_SPEC.md) - Encryption envelope format
- [INTEROP_TEST_VECTORS.md](INTEROP_TEST_VECTORS.md) - Cross-platform test vectors
- [SECURE_HANDOFF.md](SECURE_HANDOFF.md) - Bitkit/Ring key provisioning

---

## Table of Contents

1. [Overview](#1-overview)
2. [Directory Layout](#2-directory-layout)
3. [Scope Derivation](#3-scope-derivation)
4. [Storage Model](#4-storage-model)
5. [Encryption Requirements](#5-encryption-requirements)
6. [Discovery Algorithm](#6-discovery-algorithm)
7. [Deletion Semantics](#7-deletion-semantics)
8. [AAD Formats](#8-aad-formats)
9. [Payment Methods](#9-payment-methods)
10. [Implementation Requirements](#10-implementation-requirements)

---

## 1. Overview

Paykit is a decentralized payment coordination protocol built on Pubky. It enables:

- **Payment Method Discovery**: Querying public directories to find how someone accepts payments
- **Encrypted Payment Requests**: Sender-to-recipient encrypted payment solicitations
- **Subscription Proposals**: Recurring payment agreements
- **Secure Key Provisioning**: Cross-app identity and key sharing

### Design Principles

1. **Sender-Storage Model**: Senders store data on their own homeserver, not recipients'
2. **Recipient-Scoped Directories**: Per-recipient hashed directories for privacy and discovery
3. **Mandatory Encryption**: All payment requests and proposals use Sealed Blob v1
4. **Decentralized Discovery**: Polling known contacts, not centralized notification

---

## 2. Directory Layout

All Paykit v0 data is stored under `/pub/paykit.app/v0/` on user homeservers.

### Directory Structure

| Path Pattern | Description | Encryption |
|--------------|-------------|------------|
| `/pub/paykit.app/v0/{method_id}` | Supported payment method (e.g., `lightning`) | None (public) |
| `/pub/paykit.app/v0/noise` | Noise endpoint info (X25519 public key) | None (public) |
| `/pub/paykit.app/v0/requests/{recipient_scope}/{request_id}` | Payment request (on sender's storage) | Sealed Blob v1 |
| `/pub/paykit.app/v0/subscriptions/proposals/{subscriber_scope}/{proposal_id}` | Subscription proposal (on provider's storage) | Sealed Blob v1 |
| `/pub/paykit.app/v0/handoff/{request_id}` | Secure handoff payload | Sealed Blob v1 |

### Example Directory Tree

```
/pub/paykit.app/v0/
├── lightning                     # Payment method: Lightning
├── onchain                       # Payment method: Bitcoin onchain
├── noise                         # Noise endpoint public key
├── requests/
│   └── 55340b54f9184.../         # Recipient scope (64 hex chars)
│       ├── req_001               # Encrypted payment request
│       └── req_002               # Encrypted payment request
├── subscriptions/
│   └── proposals/
│       └── 04dc3323da61.../      # Subscriber scope (64 hex chars)
│           └── prop_001          # Encrypted subscription proposal
└── handoff/
    └── f3a7b2c1d4e5f6...         # Encrypted handoff payload
```

---

## 3. Scope Derivation

The `scope` creates per-recipient directories that:
- Hide the recipient's pubkey from directory listing
- Enable efficient discovery by the recipient
- Prevent enumeration attacks

### Algorithm

```
scope = hex(sha256(utf8(normalize(pubkey_z32))))
```

### Normalization

1. Trim whitespace
2. Strip `pk:` prefix if present
3. Lowercase

### Properties

| Property | Value |
|----------|-------|
| Input | z-base-32 encoded Ed25519 pubkey (52 chars) |
| Output | Lowercase hex SHA-256 hash (64 chars) |
| Deterministic | Yes (same input → same output) |
| Collision-resistant | SHA-256 provides 128-bit security |

### Test Vectors

| Input | Output (scope) |
|-------|----------------|
| `ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u` | `55340b54f918470e1f025a80bb3347934fad3f57189eef303d620e65468cde80` |
| `8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo` | `04dc3323da61313c6f5404cf7921af2432ef867afe6cc4c32553858b8ac07f12` |

See [INTEROP_TEST_VECTORS.md](INTEROP_TEST_VECTORS.md) for complete test vectors.

---

## 4. Storage Model

### Sender-Storage Principle

**Payment requests** are stored on the **sender's** homeserver:
- Sender has write access to their own storage
- Recipient discovers by polling sender's storage
- Sender can update or delete their own requests

**Subscription proposals** are stored on the **provider's** homeserver:
- Provider has write access to their own storage
- Subscriber discovers by polling provider's storage
- Provider can update or delete their own proposals

### Why Sender-Storage?

1. **No write access required**: Senders don't need write permission to recipients' storage
2. **Consent-free**: Recipients don't need to pre-authorize senders
3. **Spam resistance**: Recipients only poll known contacts
4. **Atomic operations**: Senders control their own data lifecycle

### Path Construction

**Payment Request** (stored on sender's homeserver):
```
/pub/paykit.app/v0/requests/{recipient_scope}/{request_id}
```

**Subscription Proposal** (stored on provider's homeserver):
```
/pub/paykit.app/v0/subscriptions/proposals/{subscriber_scope}/{proposal_id}
```

---

## 5. Encryption Requirements

### Mandatory Encryption

All payment requests and subscription proposals **MUST** use Sealed Blob v1 encryption.

**Plaintext storage is REJECTED** for security reasons.

### Encryption Flow

1. **Fetch recipient's Noise endpoint**: `GET /pub/paykit.app/v0/noise`
2. **Extract recipient's X25519 public key** from the endpoint
3. **Construct AAD** using the canonical format (see Section 8)
4. **Encrypt** using Sealed Blob v1
5. **Store** encrypted envelope at the appropriate path

### Decryption Flow

1. **Fetch encrypted envelope** from contact's storage
2. **Verify it's a Sealed Blob** (check for `v`, `epk`, `nonce`, `ct` fields)
3. **Construct AAD** matching the encryption context
4. **Decrypt** using recipient's Noise secret key
5. **Parse** decrypted JSON payload

### Legacy Migration

During the transition period:
- Writers: Always encrypt (no plaintext writes)
- Readers: Accept Sealed Blob v1 only, reject plaintext

After transition (hard break):
- Plaintext data is orphaned (not read, not migrated)

---

## 6. Discovery Algorithm

### For Payment Requests (Recipient)

1. Get list of known contacts (follows, past senders)
2. For each contact `C`:
   ```
   my_scope = recipient_scope(my_pubkey)
   path = "pubky://{C}/pub/paykit.app/v0/requests/{my_scope}/"
   entries = list_directory(path)
   for entry in entries:
       blob = fetch(path + entry)
       if is_sealed_blob(blob):
           request = decrypt(blob, my_noise_sk, aad)
           process(request)
   ```
3. Deduplicate by `request_id` locally
4. Track processed requests to avoid reprocessing

### For Subscription Proposals (Subscriber)

1. Get list of known providers (past subscriptions, follows)
2. For each provider `P`:
   ```
   my_scope = subscriber_scope(my_pubkey)
   path = "pubky://{P}/pub/paykit.app/v0/subscriptions/proposals/{my_scope}/"
   entries = list_directory(path)
   for entry in entries:
       blob = fetch(path + entry)
       if is_sealed_blob(blob):
           proposal = decrypt(blob, my_noise_sk, aad)
           process(proposal)
   ```

### Polling Frequency

| Context | Recommended Interval |
|---------|---------------------|
| Foreground (app active) | 30-60 seconds |
| Background (iOS/Android) | 15-60 minutes |
| Push-triggered | Immediate |

---

## 7. Deletion Semantics

### Sender/Provider Deletion

- **Can delete**: Yes, from their own storage
- **Method**: `DELETE /pub/paykit.app/v0/requests/{scope}/{id}`
- **Effect**: Removes request/proposal permanently

### Recipient/Subscriber Deletion

- **Cannot delete**: Recipients cannot delete from sender's storage
- **Deduplication**: Track processed IDs locally
- **Ignore**: Skip already-processed requests when polling

### Lifecycle

1. Sender creates request → stored on sender's homeserver
2. Recipient discovers request → processes and tracks locally
3. Payment completed → sender may delete request
4. OR: Request expires → sender may delete request
5. OR: Request cancelled → sender deletes request

---

## 8. AAD Formats

All Sealed Blob v1 encryption uses AAD to bind ciphertext to its context.

### Format Pattern

```
paykit:v0:{purpose}:{path}:{id}
```

### Specific Formats

| Object Type | AAD Format |
|-------------|------------|
| Payment Request | `paykit:v0:request:{full_path}:{request_id}` |
| Subscription Proposal | `paykit:v0:subscription_proposal:{full_path}:{proposal_id}` |
| Secure Handoff | `paykit:v0:handoff:{owner_pubkey}:{full_path}:{request_id}` |

### Examples

**Payment Request**:
```
paykit:v0:request:/pub/paykit.app/v0/requests/55340b54f918470e1f025a80bb3347934fad3f57189eef303d620e65468cde80/req_001:req_001
```

**Subscription Proposal**:
```
paykit:v0:subscription_proposal:/pub/paykit.app/v0/subscriptions/proposals/04dc3323da61313c6f5404cf7921af2432ef867afe6cc4c32553858b8ac07f12/prop_001:prop_001
```

**Secure Handoff**:
```
paykit:v0:handoff:8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty:/pub/paykit.app/v0/handoff/f3a7b2c1d4e5f6a7b8c9:f3a7b2c1d4e5f6a7b8c9
```

---

## 9. Payment Methods

### Supported Methods

| Method ID | Endpoint Format | Description |
|-----------|-----------------|-------------|
| `lightning` | BOLT11 invoice or node URI | Lightning Network |
| `onchain` | Bitcoin address | Bitcoin on-chain |

### Method File Format

Each method is stored as a JSON file at `/pub/paykit.app/v0/{method_id}`:

```json
{
  "method_id": "lightning",
  "endpoint": "03abc...@node.example.com:9735",
  "enabled": true,
  "updated_at": 1704153600000
}
```

### Discovery

To discover a peer's payment methods:
1. List `/pub/paykit.app/v0/` on their storage
2. Filter for known method IDs
3. Fetch and parse each method file

---

## 10. Implementation Requirements

### Required Implementations

| Component | Description |
|-----------|-------------|
| `normalize_pubkey_z32` | Normalize pubkey: trim, strip prefix, lowercase |
| `recipient_scope` | Compute scope hash: SHA-256 of normalized pubkey |
| `payment_request_path` | Build path for payment request |
| `subscription_proposal_path` | Build path for subscription proposal |
| `payment_request_aad` | Build AAD for payment request encryption |
| `subscription_proposal_aad` | Build AAD for subscription proposal encryption |
| `is_sealed_blob` | Check if content is Sealed Blob v1 format |

### Security Requirements

1. **Never store plaintext** for requests or proposals
2. **Always validate AAD** matches expected context
3. **Reject plaintext** when reading (no legacy fallback in production)
4. **Zeroize secrets** after use (Noise secret key, shared secrets)

### Interoperability

All implementations must:
1. Pass all test vectors in [INTEROP_TEST_VECTORS.md](INTEROP_TEST_VECTORS.md)
2. Produce identical scope hashes for identical pubkeys
3. Produce identical AAD strings for identical inputs
4. Successfully decrypt blobs encrypted by other implementations

---

## Appendix A: Reference Implementations

| Language | Location |
|----------|----------|
| Rust | `paykit-rs/paykit-lib/src/protocol/` |
| Kotlin | `bitkit-android/.../paykit/protocol/PaykitV0Protocol.kt` |
| Swift | `bitkit-ios/.../PaykitIntegration/Protocol/PaykitV0Protocol.swift` |

---

## Appendix B: Changelog

### v0.1 (January 2, 2026)
- Initial specification
- Sender-storage model with recipient-scoped directories
- Mandatory Sealed Blob v1 encryption
- SHA-256 scope hashing

---

*This specification is maintained in the [BitcoinErrorLog/paykit-rs](https://github.com/BitcoinErrorLog/paykit-rs) repository.*

