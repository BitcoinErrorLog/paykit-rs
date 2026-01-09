# BIP ?: Paykit - Payment Method Discovery and Negotiation Protocol

```
BIP: ?
Title: Paykit - Payment Method Discovery and Negotiation Protocol
Author: John Carvalho <john@synonym.to>
Status: Draft
Type: Specification
Assigned: ?
License: CC0-1.0 OR MIT
Discussion: [To be added after bitcoindev thread]
Version: 0.1.0
Requires: 21
```

## Abstract

Paykit is a payment protocol substrate that abstracts payment method discovery, negotiation, and coordination for Bitcoin payments. It provides a unified interface for on-chain Bitcoin and Lightning Network payments, including subscriptions, receipts, and payment requests. Paykit leverages the Pubky decentralized identity and storage infrastructure for discovery and uses the Noise Protocol Framework for encrypted peer-to-peer communication.

## Copyright

This BIP is dual-licensed under CC0 1.0 Universal and the MIT License.

## Motivation

The Bitcoin ecosystem has multiple incompatible payment protocols:

* **On-chain Bitcoin**: Native Bitcoin transactions with various address formats (legacy, segwit, bech32m)
* **Lightning Network**: BOLT11 invoices, LNURL, and emerging protocols like BOLT12

Each payment method requires:
* Unique discovery mechanisms
* Different negotiation protocols
* Incompatible receipt formats
* Custom integration code

This fragmentation creates barriers:
* Applications must implement support for each payment method individually
* Users must manage multiple payment interfaces
* Cross-method features (subscriptions, auto-pay) are difficult to implement

Paykit addresses this by providing a **payment protocol substrate** that:
* Abstracts payment method discovery through a unified directory protocol
* Provides a common interface for payment negotiation and coordination
* Standardizes receipt exchange and payment proofs
* Enables cross-method features (subscriptions, auto-pay, spending limits)
* Works with Bitcoin on-chain and Lightning Network through method-specific plugins

## Specification

### Conformance Levels

Paykit defines three conformance levels. Implementations MUST support Level 1 and MAY support Levels 2 and 3.

#### Level 1: Directory Protocol (REQUIRED)

* Publish and discover payment endpoints at `/pub/paykit.app/v0/{method}`
* Support `onchain` and `lightning` core methods
* Support endpoint rotation

#### Level 2: Interactive Protocol (OPTIONAL)

* Noise-encrypted negotiation channel (Noise_IK required, Noise_XX recommended)
* Receipt request and confirmation exchange
* Private endpoint sharing

#### Level 3: Subscription Protocol (OPTIONAL)

* Recurring payment agreements with cryptographic signatures
* Payment request creation and response
* Auto-pay rules and spending limits

### Overview

Paykit consists of three protocol layers:

1. **Directory Protocol**: Public payment method discovery via Pubky homeservers
2. **Interactive Protocol**: Encrypted peer-to-peer payment negotiation via Noise Protocol
3. **Subscription Protocol**: Recurring payments, auto-pay, and spending limits

### Assumptions and Dependencies

Paykit assumes the following infrastructure:

* **Pubky Protocol**: Decentralized identity and storage system
  * Ed25519 public keys for identity
  * Pubky homeservers for public data storage
  * PKARR for endpoint discovery and metadata
* **Noise Protocol Framework**: For encrypted peer-to-peer communication
  * Noise_XX and Noise_IK handshake patterns
  * ChaCha20-Poly1305 for encryption
  * X25519 for key exchange
  * BLAKE2s for hashing
* **Transport Layer**: Underlying network transport (TCP, WebSocket, etc.)

### Core Concepts

#### PublicKey

A Paykit participant is identified by their Ed25519 public key, represented as a Pubky URI:

```
pubky://<z-base-32-encoded-public-key>
```

The public key is encoded using z-base-32 (52 characters). This public key serves as:
* Payment recipient identifier
* Directory lookup key
* Authentication credential (via Ed25519 signatures)
* Noise protocol identity binding

Example: `pubky://o1gg96ewuojmopcjbz8895478wdtxtzzuxnfjjz8o8e77csa1ngo`

#### MethodId

A `MethodId` is a string identifier for a payment method. Method IDs are case-sensitive and use lowercase with hyphens.

**Core Methods** (MUST support for Level 1 compliance):
* `"onchain"` - Bitcoin on-chain addresses (BIP21-compatible)
* `"lightning"` - Lightning Network (BOLT11 invoices and LNURL)

#### EndpointData

`EndpointData` is an opaque string payload that encodes payment method-specific data:
* **onchain**: Bitcoin address (e.g., `"bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"`)
* **lightning**: BOLT11 invoice (e.g., `"lnbc10u1p3..."`) or LNURL string (e.g., `"lnurl1dp68gurn..."`)
* **Private endpoints**: Noise protocol connection string (e.g., `"noise://host:port@server_static_key_hex"`)

The format is method-specific and opaque to Paykit core. Paykit stores and retrieves endpoint data without interpretation.

#### SupportedPayments

A collection of payment methods supported by a payee, represented as a map from `MethodId` to `EndpointData`.

### Directory Protocol

The Directory Protocol enables public discovery of payment methods via Pubky homeservers.

#### Storage Path

Payment endpoints are stored under the Pubky path:
```
/pub/paykit.app/v0/{method_id}
```

Where `{method_id}` is the payment method identifier (e.g., `"onchain"`, `"lightning"`).

The path prefix `/pub/paykit.app/v0/` is standardized for interoperability.

#### Publishing Payment Methods

To publish a payment method, the payee:

1. Authenticates with their Pubky homeserver using their Ed25519 keypair
2. Writes the endpoint data to `/pub/paykit.app/v0/{method_id}`
3. Optionally removes old endpoints by deleting the file

The endpoint data is stored as UTF-8 text. Binary data MUST be base64url-encoded.

#### Discovering Payment Methods

To discover a payee's payment methods, the payer:

1. Resolves the payee's Pubky URI to their public key
2. Queries the Pubky homeserver for files under `/pub/paykit.app/v0/`
3. Reads each endpoint file to get the `EndpointData` for each method

#### Endpoint Rotation

Payees SHOULD rotate payment endpoints for privacy:
* **onchain**: Rotate after use to prevent address reuse
* **lightning**: Generate new invoices for each payment
* **Private endpoints**: Update Noise protocol connection strings as needed

Payers MUST fetch the latest endpoint data before each payment.

#### Private Endpoints

In addition to public endpoints, Paykit supports **private endpoints** shared over encrypted channels:
* Private endpoints are not published to the public directory
* They are exchanged via the Interactive Protocol
* They enable per-peer dedicated payment addresses
* They provide enhanced privacy by avoiding public address reuse

### Interactive Protocol

The Interactive Protocol enables encrypted peer-to-peer payment negotiation and receipt exchange.

#### Noise Handshake Patterns

Paykit supports two Noise patterns for different trust scenarios:

##### Noise_XX (First Contact / TOFU)

* **Use when**: Initiator does not have recipient's static public key
* **Pattern**: `Noise_XX_25519_ChaChaPoly_BLAKE2s`
* **Messages**: 3 (full round-trip for key exchange)
* **Trust model**: Trust-on-first-use; static keys exchanged during handshake

##### Noise_IK (Known Peer)

* **Use when**: Initiator has recipient's static public key (from prior contact or directory)
* **Pattern**: `Noise_IK_25519_ChaChaPoly_BLAKE2s`
* **Messages**: 2 (recipient's static key pre-known)
* **Trust model**: Server authentication; client sends identity in first message

Implementations MUST support Noise_IK. Support for Noise_XX is RECOMMENDED for first-contact scenarios.

#### Cipher Suite

* **Key Exchange**: X25519
* **Encryption**: ChaCha20-Poly1305
* **Hash**: BLAKE2s

The Noise handshake provides:
* Mutual authentication via Ed25519 identity binding
* Forward secrecy through ephemeral key exchange
* Protection against man-in-the-middle attacks
* Encrypted message transport

#### Message Format

Messages are JSON-encoded and sent over the encrypted Noise channel with length-prefixed framing:

```
[4-byte length (big-endian)][JSON-encoded message]
```

The maximum message size is 1 MB (1,048,576 bytes). The maximum handshake message size is 64 KB (65,536 bytes).

Message types:
* `OfferPrivateEndpoint` - Share a private payment endpoint
* `RequestReceipt` - Request a receipt for a payment
* `ConfirmReceipt` - Confirm and finalize a receipt
* `Ack` - Acknowledge message receipt
* `Error` - Error reporting

Example message:

```json
{
  "type": "RequestReceipt",
  "payload": {
    "provisional_receipt": {
      "receipt_id": "receipt_001",
      "payer": "pubky://o1gg96ewuojmopcjbz8895478wdtxtzzuxnfjjz8o8e77csa1ngo",
      "payee": "pubky://yb4e1n4gxprcmks6ms1uyuykfq5drmjigabdemfgop3pkpeo5sso",
      "method_id": "lightning",
      "amount": "1000",
      "currency": "SAT",
      "created_at": 1704067200,
      "metadata": {}
    }
  }
}
```

#### Payment Flow

**Step 1: Establish Encrypted Channel**
* Payer initiates Noise handshake (Noise_XX for first contact, Noise_IK for known peer)
* Both parties authenticate using Ed25519 identity binding
* Encrypted channel established

**Step 2: Request Receipt**
* Payer sends `RequestReceipt` message with provisional receipt
* Provisional receipt includes: payer, payee, method, amount, currency, metadata
* Payee validates request and generates payment endpoint (e.g., BOLT11 invoice)

**Step 3: Confirm Receipt**
* Payee sends `ConfirmReceipt` message with finalized receipt
* Finalized receipt includes payment endpoint in metadata
* Both parties save receipt for record-keeping
* Payer executes payment using the endpoint (off-protocol)

#### Receipt Format

A `PaykitReceipt` is a record shared between payer and payee:

```json
{
  "receipt_id": "receipt_001",
  "payer": "pubky://o1gg96ewuojmopcjbz8895478wdtxtzzuxnfjjz8o8e77csa1ngo",
  "payee": "pubky://yb4e1n4gxprcmks6ms1uyuykfq5drmjigabdemfgop3pkpeo5sso",
  "method_id": "lightning",
  "amount": "1000",
  "currency": "SAT",
  "created_at": 1704067200,
  "metadata": {
    "bolt11": "lnbc10u1p3...",
    "preimage": "abc123...",
    "order_id": "ABC123"
  }
}
```

### Subscription Protocol

The Subscription Protocol enables recurring payments and automated payment rules.

#### Subscription Agreement

A subscription is a bilateral agreement between a subscriber and provider:

```json
{
  "subscription_id": "sub_1704067200_abc123",
  "subscriber": "pubky://o1gg96ewuojmopcjbz8895478wdtxtzzuxnfjjz8o8e77csa1ngo",
  "provider": "pubky://yb4e1n4gxprcmks6ms1uyuykfq5drmjigabdemfgop3pkpeo5sso",
  "terms": {
    "amount": "1000",
    "currency": "SAT",
    "frequency": "monthly:1",
    "method": "lightning",
    "description": "Monthly service subscription"
  },
  "metadata": {},
  "created_at": 1704067200,
  "starts_at": 1704067200,
  "ends_at": null
}
```

#### Payment Frequency

Payment frequencies are specified as strings:
* `"daily"` - Daily payments
* `"weekly"` - Weekly payments (every 7 days)
* `"monthly:1"` - Monthly on the 1st
* `"monthly:15"` - Monthly on the 15th
* `"yearly:01-01"` - Yearly on January 1st
* `"custom:86400"` - Custom interval in seconds

#### Cryptographic Signatures

Subscriptions are cryptographically signed using Ed25519:

1. Serialize subscription using deterministic binary encoding (postcard format)
2. Create signing message: `SHA-256("PAYKIT_SUBSCRIPTION_V2" || serialized_subscription || nonce || timestamp || expiration)`
3. Sign with subscriber's Ed25519 private key
4. Include signature, nonce, timestamp, and expiration in subscription record

**Implementation Note**: Migration to RFC 8785 JSON Canonicalization Scheme (JCS) is planned for v1.0 to improve cross-language interoperability.

Signature verification:
* Verify Ed25519 signature
* Check nonce hasn't been used (replay protection)
* Validate timestamp is within acceptable range
* Check expiration hasn't passed

#### Payment Requests

Payment requests are asynchronous payment solicitations:

```json
{
  "request_id": "req_1704067200_def456",
  "from": "pubky://yb4e1n4gxprcmks6ms1uyuykfq5drmjigabdemfgop3pkpeo5sso",
  "to": "pubky://o1gg96ewuojmopcjbz8895478wdtxtzzuxnfjjz8o8e77csa1ngo",
  "amount": "1000",
  "currency": "SAT",
  "method": "lightning",
  "description": "Monthly subscription payment",
  "due_date": 1706745600,
  "metadata": {},
  "created_at": 1704067200,
  "expires_at": 1707350400
}
```

Payment requests can be:
* Created by providers to request payment from subscribers
* Stored in Pubky directory for async discovery
* Responded to by subscribers (accept, decline, or propose subscription)

#### Storage Layout (Paykit v0)

Objects are stored using ContextId for peer-pair routing:

| Object Type | Path | Stored On |
|-------------|------|-----------|
| Payment Request | `/pub/paykit.app/v0/requests/{context_id}/{request_id}` | Sender |
| Subscription Proposal | `/pub/paykit.app/v0/subscriptions/proposals/{context_id}/{proposal_id}` | Provider |
| ACK | `/pub/paykit.app/v0/acks/{object_type}/{context_id}/{msg_id}` | Receiver |
| Noise Endpoint | `/pub/paykit.app/v0/noise` | Owner |
| Secure Handoff | `/pub/paykit.app/v0/handoff/{request_id}` | Ring User |

Where:
* `context_id` = `hex(SHA256("paykit:v0:context:" || first_z32 || ":" || second_z32))`
* `first_z32`, `second_z32` = normalized pubkeys sorted lexicographically (see crypto.md)

#### Encrypted ACK Protocol

ACKs confirm receipt of async messages. See crypto.md for the full protocol specification.

#### Auto-Pay Rules

Auto-pay rules enable automated payment approval:

```json
{
  "rule_id": "rule_1704067200_ghi789",
  "subscription_id": "sub_1704067200_abc123",
  "peer": "pubky://yb4e1n4gxprcmks6ms1uyuykfq5drmjigabdemfgop3pkpeo5sso",
  "method": "lightning",
  "max_amount": "5000",
  "currency": "SAT",
  "enabled": true,
  "require_confirmation": false
}
```

#### Spending Limits

Spending limits restrict total spending per peer over a time period:

```json
{
  "peer": "pubky://yb4e1n4gxprcmks6ms1uyuykfq5drmjigabdemfgop3pkpeo5sso",
  "max_amount": "10000",
  "currency": "SAT",
  "period": "monthly",
  "current_spending": "5000",
  "period_start": 1704067200,
  "period_end": 1706745600
}
```

### Payment Method Plugins

Paykit abstracts payment methods through a plugin architecture.

#### Core Methods

**On-chain Bitcoin** (`"onchain"`):
* Endpoint: Bitcoin address (BIP21-compatible URI or raw address)
* Formats: legacy (1...), P2SH (3...), bech32 (bc1q...), bech32m (bc1p...)
* Execution: Create Bitcoin transaction to address
* Proof: Transaction ID (txid) and optional block height
* Rotation: Generate new address after use (BIP32 derivation recommended)

**Lightning Network** (`"lightning"`):
* Endpoint: BOLT11 invoice or LNURL string
* Formats:
  - BOLT11: `lnbc...` (mainnet), `lntb...` (testnet), `lnbcrt...` (regtest)
  - LNURL: `lnurl1...` (bech32-encoded URL, per LUD-01)
* Execution: Pay BOLT11 invoice directly, or follow LNURL flow (LUD-01 through LUD-21)
* Proof: Payment preimage and payment hash
* Rotation: Generate new invoice for each payment
* Reference: [LNURL specs](https://github.com/lnurl/luds)

#### Custom Methods

Applications MAY define custom payment methods by:
1. Defining a unique `MethodId`
2. Implementing payment execution logic
3. Publishing endpoint data in Paykit directory
4. Handling payment proofs appropriately

### Payment Proofs

Paykit provides a unified proof format:

```json
{
  "receipt_id": "receipt_001",
  "method": "lightning",
  "proof": {
    "type": "lightning_preimage",
    "preimage": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "payment_hash": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
  }
}
```

Proof types:
* **onchain**: `{"type": "bitcoin_txid", "txid": "...", "block_height": 123456, "confirmations": 6}`
* **lightning**: `{"type": "lightning_preimage", "preimage": "...", "payment_hash": "..."}`

## Rationale

### Why Pubky for Identity and Storage?

Pubky provides censorship-resistant, self-sovereign identity and storage:
* **Ed25519 keys**: Widely supported, fast, secure
* **Homeserver model**: Users control their data
* **PKARR**: Decentralized endpoint discovery without central registries
* **No KYC/account requirements**: Permissionless participation

### Why Noise Protocol Framework?

Noise provides proven security properties:
* **Mutual authentication**: Both parties prove identity
* **Forward secrecy**: Compromise of long-term keys doesn't expose past sessions
* **Established standard**: Extensively analyzed, widely implemented
* **Flexibility**: Multiple patterns for different trust scenarios

### Why z-base-32 Encoding?

z-base-32 is the Pubky ecosystem standard for public key encoding:
* **Human-friendly**: Avoids ambiguous characters (0/O, 1/l)
* **Case-insensitive**: Reduces transcription errors
* **URL-safe**: No encoding needed in URIs
* **Compact**: 52 characters for 32-byte keys

### Why Plugin Architecture?

Extensibility without protocol changes:
* **New payment methods**: Add support without updating core protocol
* **Minimal core**: Core protocol remains simple and stable
* **Local implementation**: Payment method details handled locally

## Backward Compatibility

This BIP introduces a new protocol layer and does not modify existing Bitcoin protocols. Paykit is designed to complement rather than replace:

* **BIP21** (Bitcoin URI scheme): Paykit `onchain` endpoints MAY contain BIP21-compatible URIs. Paykit-unaware wallets can extract and use the Bitcoin address directly.

* **BOLT11** (Lightning invoices): Paykit `lightning` endpoints contain standard BOLT11 strings. Any Lightning wallet can pay Paykit-discovered invoices.

* **LNURL**: Paykit `lnurl` endpoints contain standard LNURL strings. LNURL-supporting wallets can process these independently.

No changes to consensus rules, transaction formats, or existing wallet behavior are required. Existing payment infrastructure continues to work; Paykit provides an optional discovery and coordination layer.

## Security Considerations

### Identity Authentication

* All participants authenticated via Ed25519 public keys
* Noise protocol provides mutual authentication
* Identity binding prevents man-in-the-middle attacks

### Encryption

* All interactive communication encrypted via Noise Protocol
* Forward secrecy through ephemeral key exchange
* ChaCha20-Poly1305 provides authenticated encryption
* Private endpoints not exposed in public directory

### Replay Protection

* Subscription signatures include unique 32-byte nonces
* Implementations MUST maintain nonce database to prevent replay
* Timestamp validation prevents old signature reuse
* Expiration times limit signature validity window

### Spending Limits

* Atomic check-and-reserve operations required
* Automatic rollback on payment failure
* Period-based limits prevent unbounded spending

### Key Management

* Private keys MUST be stored securely (hardware security modules, secure enclaves)
* Key rotation policies RECOMMENDED
* X25519 keys derived from Ed25519 for Noise key exchange

## Privacy Considerations

### Information Leakage

* Public directory reveals which payment methods a payee supports
* Directory queries reveal which payees a payer queries
* Mitigations: Use private endpoints, rotate addresses, consider Tor

### Private Endpoints

Private endpoints reduce information leakage:
* Per-peer dedicated addresses
* No public address reuse
* Negotiated over encrypted channel

## Test Vectors

See auxiliary file: `bip-paykit/test-vectors.json`

## Reference Implementation

Reference implementation available at:
* Repository: https://github.com/BitcoinErrorLog/paykit-rs
* Language: Rust
* License: MIT

Crates:
* `paykit-lib`: Core directory protocol
* `paykit-interactive`: Interactive protocol with Noise
* `paykit-subscriptions`: Subscription protocol

## Acknowledgments

* Pubky Protocol team for decentralized identity and storage
* Noise Protocol Framework authors for secure communication
* Bitcoin and Lightning Network communities

## References

* [Pubky Protocol](https://pubky.org)
* [Noise Protocol Framework](https://noiseprotocol.org/)
* [BIP21: Bitcoin URI Scheme](https://github.com/bitcoin/bips/blob/master/bip-0021.mediawiki)
* [BOLT11: Lightning Invoice Format](https://github.com/lightning/bolts/blob/master/11-payment-encoding.md)
* [LNURL Documents](https://github.com/lnurl/luds)
* [RFC 8032: Ed25519](https://datatracker.ietf.org/doc/html/rfc8032)
* [RFC 7748: X25519](https://datatracker.ietf.org/doc/html/rfc7748)
* [RFC 8439: ChaCha20-Poly1305](https://datatracker.ietf.org/doc/html/rfc8439)
* [RFC 7693: BLAKE2](https://datatracker.ietf.org/doc/html/rfc7693)
* [Pubky Cryptographic Specification](https://github.com/pubky/pubky-core/blob/main/docs/PUBKY_CRYPTO_SPEC.md)

## Changelog

### Version 0.1.0

* Initial draft specification
* Core methods: `onchain`, `lightning` (includes LNURL support)
* Directory, Interactive, and Subscription protocols defined
* Conformance levels specified
* BIP3-compliant format

