# Payment Method Registry

> **Version**: 1.0  
> **Last Updated**: January 2, 2026  
> **Status**: Living Document

This document defines the canonical registry of payment method identifiers and their endpoint formats. All Paykit implementations should use these stable identifiers for interoperability.

**Related Documents**:
- [PAYKIT_PROTOCOL_V0.md](PAYKIT_PROTOCOL_V0.md) - Paykit v0 protocol specification
- [BITKIT_PAYKIT_INTEGRATION_MASTERGUIDE.md](BITKIT_PAYKIT_INTEGRATION_MASTERGUIDE.md) - Bitkit integration guide

---

## Table of Contents

1. [Purpose](#1-purpose)
2. [Method Identifier Rules](#2-method-identifier-rules)
3. [Registered Methods](#3-registered-methods)
4. [Endpoint Schemas](#4-endpoint-schemas)
5. [Adding New Methods](#5-adding-new-methods)

---

## 1. Purpose

Payment method identifiers (`method_id`) are short, stable strings that identify payment protocols. This registry:

- Defines canonical identifiers for well-known payment methods
- Specifies endpoint payload formats for each method
- Prevents naming collisions
- Provides a reference for implementers

---

## 2. Method Identifier Rules

### Naming Conventions

| Rule | Description | Example |
|------|-------------|---------|
| Lowercase | All identifiers are lowercase | `lightning` ✓, `Lightning` ✗ |
| Alphanumeric + hyphen | Only `a-z`, `0-9`, `-` | `btc-lsp` ✓, `btc_lsp` ✗ |
| No version suffix | Versions handled separately | `lightning` ✓, `lightning-v2` ✗ |
| Short and descriptive | Prefer brevity | `onchain` ✓, `bitcoin-onchain-payment` ✗ |
| Max 32 characters | Hard limit | N/A |

### Reserved Prefixes

| Prefix | Reserved For |
|--------|--------------|
| `btc-` | Bitcoin-related methods |
| `ln-` | Lightning-related methods |
| `test-` | Test/mock methods |
| `x-` | Experimental methods (not for production) |

### Collision Avoidance

1. Check this registry before defining a new method ID
2. Use organization prefix for custom methods (e.g., `acme-invoice`)
3. Submit a PR to add well-known methods to this registry

---

## 3. Registered Methods

### Core Methods

| Method ID | Protocol | Status | Spec URI |
|-----------|----------|--------|----------|
| `lightning` | Lightning Network | Stable | [BOLT11](https://github.com/lightning/bolts/blob/master/11-payment-encoding.md) |
| `onchain` | Bitcoin L1 | Stable | [BIP21](https://github.com/bitcoin/bips/blob/master/bip-0021.mediawiki) |

### Extended Methods (Proposed)

| Method ID | Protocol | Status | Spec URI |
|-----------|----------|--------|----------|
| `lnurl` | LNURL | Proposed | [LUD-01](https://github.com/lnurl/luds/blob/luds/01.md) |
| `bolt12` | BOLT12 Offers | Proposed | [BOLT12](https://github.com/lightning/bolts/pull/798) |
| `payjoin` | BIP-78 Payjoin | Proposed | [BIP78](https://github.com/bitcoin/bips/blob/master/bip-0078.mediawiki) |
| `silent-payments` | Silent Payments | Proposed | [BIP352](https://github.com/bitcoin/bips/blob/master/bip-0352.mediawiki) |

---

## 4. Endpoint Schemas

### `lightning`

**Endpoint Data**: A string containing one of:
- BOLT11 invoice (starts with `lnbc`, `lntb`, `lnbcrt`)
- Node URI (`pubkey@host:port`)
- LNURL (starts with `lnurl1` or `LNURL1`)

```json
{
  "method_id": "lightning",
  "endpoint": "lnbc1pvjluezpp5qqqsyq...",
  "enabled": true,
  "updated_at": 1704153600000
}
```

**Validation Rules**:
- BOLT11: Valid bech32, correct network prefix
- Node URI: Valid hex pubkey (66 chars), valid host:port
- LNURL: Valid bech32

**Amount Limits**:
- Minimum: 1 sat
- Maximum: Limited by channel capacity (typically ≤ 0.16 BTC)

**Estimated Confirmation**: < 30 seconds

### `onchain`

**Endpoint Data**: A Bitcoin address or BIP21 URI.

```json
{
  "method_id": "onchain",
  "endpoint": "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
  "enabled": true,
  "updated_at": 1704153600000
}
```

**Validation Rules**:
- P2PKH: Starts with `1` (mainnet) or `m`/`n` (testnet)
- P2SH: Starts with `3` (mainnet) or `2` (testnet)
- Bech32: Starts with `bc1` (mainnet), `tb1` (testnet), `bcrt1` (regtest)
- BIP21: URI format `bitcoin:<address>?amount=...`

**Amount Limits**:
- Minimum: 546 sats (dust limit)
- Maximum: Unlimited

**Estimated Confirmation**: 10-60 minutes (1-6 blocks)

### `lnurl` (Proposed)

**Endpoint Data**: An LNURL string.

```json
{
  "method_id": "lnurl",
  "endpoint": "lnurl1dp68gurn8ghj7um9wfmxjcm99e3k7mf...",
  "enabled": true,
  "updated_at": 1704153600000
}
```

**Validation Rules**:
- Valid bech32 encoding with `lnurl` HRP
- Decodes to valid HTTPS URL

### `bolt12` (Proposed)

**Endpoint Data**: A BOLT12 offer.

```json
{
  "method_id": "bolt12",
  "endpoint": "lno1qgsqvgnwg...",
  "enabled": true,
  "updated_at": 1704153600000
}
```

**Validation Rules**:
- Valid bech32m encoding with `lno` HRP (offer)
- Or `lni` HRP (invoice)

---

## 5. Adding New Methods

### Process

1. **Propose**: Open an issue describing the payment method
2. **Discuss**: Community review for naming and schema
3. **Document**: Add entry to this registry with schema
4. **Implement**: Add plugin to `paykit-lib/src/methods/`
5. **Test**: Add validation and round-trip tests

### Template

```markdown
### `{method-id}` (Proposed)

**Endpoint Data**: Description of what the endpoint contains.

```json
{
  "method_id": "{method-id}",
  "endpoint": "{example-endpoint}",
  "enabled": true,
  "updated_at": 1704153600000
}
```

**Validation Rules**:
- Rule 1
- Rule 2

**Amount Limits**:
- Minimum: X sats
- Maximum: Y sats or unlimited

**Estimated Confirmation**: Time estimate
```

---

## Appendix A: Method File Storage

Each method is stored as a file at:

```
/pub/paykit.app/v0/{method_id}
```

Content is a JSON object with the following structure:

```json
{
  "method_id": "lightning",
  "endpoint": "lnbc...",
  "enabled": true,
  "updated_at": 1704153600000,
  "metadata": {
    "label": "My Lightning Node",
    "description": "Primary payment method"
  }
}
```

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `method_id` | string | Method identifier (must match filename) |
| `endpoint` | string | Payment endpoint data |
| `enabled` | boolean | Whether this method is currently accepting payments |
| `updated_at` | integer | Unix timestamp (milliseconds) of last update |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `metadata` | object | Additional method-specific metadata |
| `metadata.label` | string | Human-readable label |
| `metadata.description` | string | Description for UI display |

---

## Appendix B: Snapshot File (Optional)

For compatibility with clients expecting a single JSON array (PDF-style), an optional snapshot file can be generated at:

```
/pub/paykit.app/v0/supported.json
```

This file contains an array of all supported methods:

```json
[
  {
    "method_id": "lightning",
    "endpoint": "lnbc...",
    "enabled": true,
    "updated_at": 1704153600000
  },
  {
    "method_id": "onchain",
    "endpoint": "bc1q...",
    "enabled": true,
    "updated_at": 1704153600000
  }
]
```

**Note**: This is supplementary to per-method files. The per-method files remain the source of truth for atomic updates.

---

*This registry is maintained in the [BitcoinErrorLog/paykit-rs](https://github.com/BitcoinErrorLog/paykit-rs) repository.*

