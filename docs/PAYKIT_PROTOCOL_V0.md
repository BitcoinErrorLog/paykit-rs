# Paykit Protocol v0 Specification

> **Version**: 0.2  
> **Last Updated**: January 20, 2026  
> **Status**: Canonical Specification

This document is the canonical specification for Paykit Protocol v0. All implementations (Rust, Kotlin, Swift, TypeScript) **must** conform to this spec.

## Upstream Specifications

Paykit builds on and implements these upstream specifications:

| Specification | Version | Scope |
|--------------|---------|-------|
| [PUBKY_CRYPTO_SPEC](https://github.com/pubky/pubky-core/blob/main/docs/PUBKY_CRYPTO_SPEC.md) | v2.5 | Sealed Blob SB2, AAD, ContextId, InboxKey/TransportKey |
| [PUBKY_UNIFIED_KEY_DELEGATION_SPEC](https://github.com/pubky/pubky-core/blob/main/docs/PUBKY_UNIFIED_KEY_DELEGATION_SPEC_v0.2.md) | v0.2 | AppCert, KeyBinding, typed signing |

**Related Documents**:
- [SEALED_BLOB_SPEC.md](SEALED_BLOB_SPEC.md) - Encryption envelope format (implements CRYPTO_SPEC SB2)
- [INTEROP_TEST_VECTORS.md](INTEROP_TEST_VECTORS.md) - Cross-platform test vectors
- [SECURE_HANDOFF.md](SECURE_HANDOFF.md) - Bitkit/Ring key provisioning
- [ATOMICITY_INTEGRATION.md](ATOMICITY_INTEGRATION.md) - Atomicity settlement adapter

---

## Table of Contents

1. [Overview](#1-overview)
2. [Directory Layout](#2-directory-layout)
3. [ContextId Derivation](#3-contextid-derivation)
4. [Storage Model](#4-storage-model)
5. [Encryption Requirements](#5-encryption-requirements)
6. [Discovery Algorithm](#6-discovery-algorithm)
7. [Deletion Semantics](#7-deletion-semantics)
8. [AAD Formats](#8-aad-formats)
9. [Payment Methods](#9-payment-methods)
10. [Implementation Requirements](#10-implementation-requirements)
11. [Payload Schemas](#11-payload-schemas)

---

## 1. Overview

Paykit is a decentralized payment coordination protocol built on Pubky. It enables:

- **Payment Method Discovery**: Querying public directories to find how someone accepts payments
- **Encrypted Payment Requests**: Sender-to-recipient encrypted payment solicitations
- **Subscription Proposals**: Recurring payment agreements
- **Secure Key Provisioning**: Cross-app identity and key sharing

### Design Principles

1. **Sender-Storage Model**: Senders store data on their own homeserver, not recipients'
2. **ContextId-Based Directories**: Symmetric peer-pair hashed directories for privacy and discovery
3. **Mandatory Encryption**: All payment requests and proposals use Sealed Blob v2 (XChaCha20-Poly1305)
4. **Owner-Bound AAD**: AAD includes storage owner pubkey to prevent relocation attacks
5. **Decentralized Discovery**: Polling known contacts, not centralized notification

---

## 2. Directory Layout

All Paykit v0 data is stored under `/pub/paykit.app/v0/` on user homeservers.

### Directory Structure

| Path Pattern | Description | Encryption |
|--------------|-------------|------------|
| `/pub/paykit.app/v0/{method_id}` | Supported payment method (e.g., `lightning`) | None (public) |
| `/pub/paykit.app/v0/noise` | Noise endpoint info (X25519 public key) | None (public) |
| `/pub/paykit.app/v0/requests/{context_id}/{request_id}` | Payment request (on sender's storage) | Sealed Blob v2 |
| `/pub/paykit.app/v0/subscriptions/proposals/{context_id}/{proposal_id}` | Subscription proposal (on provider's storage) | Sealed Blob v2 |
| `/pub/paykit.app/v0/acks/{object_type}/{context_id}/{msg_id}` | Encrypted ACK (on receiver's storage) *(specified, not yet implemented)* | Sealed Blob v2 |
| `/pub/paykit.app/v0/handoff/{request_id}` | Secure handoff payload | Sealed Blob v2 |

### Example Directory Tree

```
/pub/paykit.app/v0/
├── lightning                     # Payment method: Lightning
├── onchain                       # Payment method: Bitcoin onchain
├── noise                         # Noise endpoint public key
├── requests/
│   └── 55340b54f9184.../         # ContextId (64 hex chars, symmetric)
│       ├── req_001               # Sealed Blob v2 encrypted request
│       └── req_002               # Sealed Blob v2 encrypted request
├── subscriptions/
│   └── proposals/
│       └── 04dc3323da61.../      # ContextId (64 hex chars, symmetric)
│           └── prop_001          # Sealed Blob v2 encrypted proposal
└── handoff/
    └── f3a7b2c1d4e5f6...         # Sealed Blob v2 encrypted handoff
```

---

## 3. ContextId Derivation

Per **PUBKY_CRYPTO_SPEC v2.5 Section 7.4**, ContextId is a **32-byte random value** chosen by the thread initiator.

### SPEC-Compliant ContextId (v2.5)

For all **new threads**, the thread initiator generates a random ContextId:

```
context_id = random_bytes(32)  # 32 cryptographically random bytes
context_id_hex = hex(context_id)  # 64 lowercase hex characters
```

**Key properties**:
- **Random**: 32 cryptographically random bytes
- **Initiator-chosen**: The thread starter selects the ContextId
- **Non-symmetric**: Different parties initiating threads with the same peer get different ContextIds
- **Included in SB2 headers**: The ContextId is stored in the `context_id` field of SB2 headers

### Properties

| Property | Value |
|----------|-------|
| Length | 32 bytes (64 hex chars) |
| Source | Cryptographically secure random |
| Storage | In SB2 header `context_id` field |
| Discovery | Via bounded directory polling |

### Legacy Pair-Derived ContextId (Deprecated)

For **backward compatibility only**, the legacy symmetric pair-derived ContextId is still supported:

```
context_id = hex(sha256("paykit:v0:context:" + first_z32 + ":" + second_z32))
```

Where `first_z32` and `second_z32` are normalized pubkeys sorted lexicographically.

**Properties of legacy ContextId**:
- Symmetric: `context_id(A, B) == context_id(B, A)`
- Deterministic: Same inputs → same output

**This approach is DEPRECATED**. Use random ContextId for all new implementations.

### Migration Path

1. **New threads**: Always use random ContextId via `generate_context_id()`
2. **Existing threads**: May continue using legacy pair-derived ContextId
3. **Detection**: SB2 headers contain the ContextId; use it for response messages
4. **Bounded discovery**: Recipients poll sender directories with resource limits

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
/pub/paykit.app/v0/requests/{context_id}/{request_id}
```

**Subscription Proposal** (stored on provider's homeserver):
```
/pub/paykit.app/v0/subscriptions/proposals/{context_id}/{proposal_id}
```

**Encrypted ACK** (stored on receiver's homeserver) *(specified, not yet implemented)*:
```
/pub/paykit.app/v0/acks/{object_type}/{context_id}/{msg_id}
```

---

## 5. Encryption Requirements

Per **PUBKY_CRYPTO_SPEC v2.5**, all stored delivery uses **Sealed Blob v2 (SB2)** format.

### Key Separation

| Key Type | Purpose | Usage |
|----------|---------|-------|
| **InboxKey** | Sealed Blob encryption | Stored delivery (payment requests, ACKs) |
| **TransportKey** | Noise sessions | Real-time encrypted channels |

Keys are published via **KeyBinding** (CBOR-encoded) for peer discovery.

### Mandatory Encryption

All payment requests and subscription proposals **MUST** use Sealed Blob v2 encryption.

**Plaintext storage is REJECTED** for security reasons.

### Encryption Flow (SB2)

1. **Fetch recipient's KeyBinding** from PKARR or directory
2. **Select recipient's InboxKey** (X25519 public key for stored delivery)
3. **Generate random ContextId** (32 bytes) for new threads
4. **Construct binary AAD** per CRYPTO_SPEC Section 7.5:
   ```
   aad = "pubky-envelope/v2:" || owner_peerid_bytes || canonical_path_bytes || header_no_sig
   ```
5. **Encrypt** using SB2 (XChaCha20-Poly1305, deterministic CBOR header)
6. **Store** encrypted SB2 blob at the appropriate path

### Decryption Flow (SB2)

1. **Fetch encrypted SB2 blob** from contact's storage
2. **Verify SB2 magic bytes** (0x53 0x42 0x32 = "SB2")
3. **Parse CBOR header** to extract `context_id`, `msg_id`, `inbox_kid`
4. **Select InboxKey secret** matching `inbox_kid` (O(1) lookup)
5. **Construct binary AAD** using header fields
6. **Decrypt** using recipient's InboxKey secret key
7. **Parse** decrypted JSON payload

### Legacy JSON Envelope (Deprecated)

For backward compatibility, readers MAY accept legacy JSON Sealed Blob format:
- Magic: Starts with `{` (JSON object)
- Fields: `v`, `epk`, `nonce`, `ct` (all base64url-encoded)

**New implementations MUST write SB2 format only.**

---

## 6. Discovery Algorithm

### Bounded Discovery

Per CRYPTO_SPEC v2.5, discovery uses **bounded polling** with resource limits:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_entries_per_dir` | 100 | Maximum entries to enumerate per directory |
| `max_blob_size` | 64KB | Maximum SB2 blob size to fetch |
| `poll_interval_secs` | 300 | Minimum seconds between polls |

### For Payment Requests (Recipient)

1. Get list of known contacts (follows, past senders)
2. For each contact `C`:
   ```
   # List ALL context_id directories (bounded)
   requests_path = "pubky://{C}/pub/paykit.app/v0/requests/"
   ctx_dirs = list_directory(requests_path, max_entries=100)
   
   for ctx_id_dir in ctx_dirs:
       entries = list_directory(requests_path + ctx_id_dir, max_entries=100)
       for entry in entries:
           blob = fetch(requests_path + ctx_id_dir + entry)
           if is_sb2(blob):
               # AAD uses owner=C (sender's storage)
               aad = build_binary_aad(C, canonical_path, header_bytes)
               request = sb2_decrypt(blob, my_inbox_sk, aad)
               if can_decrypt(request):  # I'm the intended recipient
                   process(request)
   ```
3. Deduplicate by `request_id` locally
4. Track processed requests to avoid reprocessing
5. Extract `context_id` from SB2 header for response messages

### For Subscription Proposals (Subscriber)

1. Get list of known providers (past subscriptions, follows)
2. For each provider `P`:
   ```
   proposals_path = "pubky://{P}/pub/paykit.app/v0/subscriptions/proposals/"
   ctx_dirs = list_directory(proposals_path, max_entries=100)
   
   for ctx_id_dir in ctx_dirs:
       entries = list_directory(proposals_path + ctx_id_dir, max_entries=100)
   for entry in entries:
       blob = fetch(path + entry)
       if is_sealed_blob(blob):
           aad = subscription_proposal_aad(P, P, my_pubkey, entry)  # owner=P
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

All Sealed Blob v2 encryption uses AAD to bind ciphertext to its storage context and owner.

### Format Pattern (Owner-Bound)

```
paykit:v0:{purpose}:{owner_z32}:{path}:{id}
```

Where `owner_z32` is the normalized z-base-32 pubkey of the storage owner.

### Specific Formats

| Object Type | AAD Format |
|-------------|------------|
| Payment Request | `paykit:v0:request:{owner_z32}:{path}:{request_id}` |
| Subscription Proposal | `paykit:v0:subscription_proposal:{owner_z32}:{path}:{proposal_id}` |
| Encrypted ACK | `paykit:v0:ack_{object_type}:{ack_writer_z32}:{path}:{msg_id}` |
| Secure Handoff | `paykit:v0:handoff:{owner_z32}:{path}:{request_id}` |

### Examples

**Payment Request** (sender is owner):
```
paykit:v0:request:8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo:/pub/paykit.app/v0/requests/a7b8c9d0.../req_001:req_001
```

**Subscription Proposal** (provider is owner):
```
paykit:v0:subscription_proposal:ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u:/pub/paykit.app/v0/subscriptions/proposals/b3c4d5e6.../prop_001:prop_001
```

**Encrypted ACK** (receiver is owner):
```
paykit:v0:ack_request:tj1igr...abc:/pub/paykit.app/v0/acks/request/a7b8c9d0.../req_001:req_001
```

**Secure Handoff** (Ring user is owner):
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

### Snapshot File (Optional)

For clients that prefer a single JSON array (PDF-style compatibility):

**Path**: `/pub/paykit.app/v0/supported.json`

```json
[
  {"method_id": "lightning", "endpoint": "lnbc...", "enabled": true, "updated_at": 1704153600000},
  {"method_id": "onchain", "endpoint": "bc1q...", "enabled": true, "updated_at": 1704153600000}
]
```

- This is **optional** and supplementary to per-method files
- Per-method files remain the source of truth
- See [PAYMENT_METHOD_REGISTRY.md](PAYMENT_METHOD_REGISTRY.md) for full details

---

## 10. Implementation Requirements

### Required Implementations

| Component | Description |
|-----------|-------------|
| `normalize_pubkey_z32` | Normalize pubkey: trim, strip `pubky://` and `pk:` prefixes, lowercase |
| `context_id` | Compute symmetric ContextId for peer pair |
| `payment_request_path` | Build path for payment request (uses ContextId) |
| `subscription_proposal_path` | Build path for subscription proposal (uses ContextId) |
| `ack_path` | Build path for encrypted ACK (uses ContextId) |
| `payment_request_aad` | Build owner-bound AAD for payment request |
| `subscription_proposal_aad` | Build owner-bound AAD for subscription proposal |
| `ack_aad` | Build AAD for encrypted ACK |
| `is_sealed_blob` | Check if content is Sealed Blob v1 or v2 format |

**Deprecated** (legacy compatibility only):
| `recipient_scope` | Legacy single-party scope hash (use `context_id` instead) |

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

## 11. Payload Schemas

This section defines the canonical JSON schemas for encrypted payloads. All implementations **must** use these field names and types for interoperability.

### Payment Request Schema

The plaintext payload (before Sealed Blob encryption):

```json
{
  "schema_version": 1,
  "from_pubkey": "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
  "to_pubkey": "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
  "amount_sats": 10000,
  "description": "Payment for services",
  "method_id": "lightning",
  "created_at": 1704153600000,
  "expires_at": 1704240000000
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `schema_version` | integer | No | Schema version (default: 1) |
| `from_pubkey` | string | Yes | Sender's z-base-32 pubkey |
| `to_pubkey` | string | Yes | Recipient's z-base-32 pubkey |
| `amount_sats` | integer | Yes | Amount in satoshis |
| `description` | string | No | Human-readable memo |
| `method_id` | string | No | Preferred payment method (default: `lightning`) |
| `created_at` | integer | Yes | Unix timestamp in milliseconds |
| `expires_at` | integer | No | Expiry timestamp in milliseconds |

### Subscription Proposal Schema

The plaintext payload (before Sealed Blob encryption):

```json
{
  "schema_version": 1,
  "provider_pubkey": "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
  "provider_name": "Acme Services",
  "amount_sats": 5000,
  "currency": "SAT",
  "frequency": "monthly",
  "description": "Monthly subscription",
  "method_id": "lightning",
  "created_at": 1704153600000,
  "max_payments": null,
  "start_date": null
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `schema_version` | integer | No | Schema version (default: 1) |
| `provider_pubkey` | string | Yes | Provider's z-base-32 pubkey |
| `provider_name` | string | No | Human-readable provider name |
| `amount_sats` | integer | Yes | Amount per payment in satoshis |
| `currency` | string | No | Currency code (default: `SAT`) |
| `frequency` | string | Yes | Payment frequency: `daily`, `weekly`, `monthly`, `yearly` |
| `description` | string | No | Human-readable subscription description |
| `method_id` | string | No | Payment method (default: `lightning`) |
| `created_at` | integer | Yes | Unix timestamp in milliseconds |
| `max_payments` | integer | No | Maximum number of payments (null = unlimited) |
| `start_date` | integer | No | Start date as Unix timestamp in milliseconds |

### Provider Identity Binding

**SECURITY REQUIREMENT**: When polling a provider's storage for subscription proposals, clients **must** verify that:

1. The `provider_pubkey` in the decrypted proposal matches the pubkey of the storage being polled
2. If they don't match, the proposal **must** be rejected

This prevents a malicious provider from publishing proposals that impersonate another provider.

```kotlin
// Example: Kotlin verification
fun verifyProviderBinding(proposal: SubscriptionProposal, polledPubkey: String): Boolean {
    val normalizedProposed = PaykitV0Protocol.normalizePubkeyZ32(proposal.providerPubkey)
    val normalizedPolled = PaykitV0Protocol.normalizePubkeyZ32(polledPubkey)
    return normalizedProposed == normalizedPolled
}
```

### Parsing Guidelines

1. **Unknown fields**: Ignore unknown fields for forward compatibility
2. **Missing optional fields**: Use specified defaults
3. **Missing required fields**: Reject the payload
4. **Type mismatches**: Reject the payload (no coercion)

---

## Appendix A: Reference Implementations

| Language | Location |
|----------|----------|
| Rust | `paykit-rs/paykit-lib/src/protocol/` |
| Kotlin | `bitkit-android/.../paykit/protocol/PaykitV0Protocol.kt` |
| Swift | `bitkit-ios/.../PaykitIntegration/Protocol/PaykitV0Protocol.swift` |

---

## Appendix B: Changelog

### v0.2 (January 8, 2026)
- Migrated from `recipient_scope`/`subscriber_scope` to symmetric `context_id`
- Updated AAD format to include owner binding (`paykit:v0:{purpose}:{owner}:{path}:{id}`)
- Added encrypted ACK path and AAD specs
- Updated all path templates to use `{context_id}`
- Clarified Sealed Blob v2 as current, v1 as legacy
- Added `pubky://` prefix stripping to normalization

### v0.1 (January 2, 2026)
- Initial specification
- Sender-storage model with recipient-scoped directories
- Mandatory Sealed Blob encryption
- SHA-256 scope hashing

---

*This specification is maintained in the [BitcoinErrorLog/paykit-rs](https://github.com/BitcoinErrorLog/paykit-rs) repository.*

