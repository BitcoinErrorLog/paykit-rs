# Atomicity ↔ Paykit Integration Guide

> **Version**: 1.0  
> **Last Updated**: January 20, 2026  
> **Status**: ⚠️ **PENDING UPSTREAM WORK — fork-only, OPTIONAL for Atomicity.** The `paykit_lib::atomicity` module documented here exists only in this BitcoinErrorLog fork, not in the official [`pubky/paykit-rs`](https://github.com/pubky/paykit-rs). Atomicity v1.1 does not require it: settlement maps onto official Payment Requests/Proofs/Receipts per `atomicity-research/ATOMICITY_MESSAGING_PROFILE_PUBKY.md` Section 6.2. This guide is retained as the upstream proposal for a typed settlement adapter.

This document describes how Paykit serves as the settlement layer for the Atomicity decentralized credit protocol.

## Overview

[Atomicity](https://github.com/BitcoinErrorLog/atomicity-research) is a decentralized P2P credit protocol that enables:
- Peer-to-peer IOUs without intermediaries
- Credit routing through trust networks
- Final settlement via Bitcoin/Lightning

**Paykit's Role**: Execute settlements when credit IOUs need to be converted to Bitcoin payments.

## Architecture

```
┌──────────────────┐          ┌──────────────────┐          ┌──────────────────┐
│                  │          │                  │          │                  │
│  Atomicity Node  │──────────│  Paykit Adapter  │──────────│  Payment Layer   │
│                  │          │                  │          │  (LN / On-chain) │
│  - Credit IOUs   │  settle  │  - Settlement    │  execute │                  │
│  - Routing       │  ──────> │    Request       │  ──────> │  - BOLT11        │
│  - Trust Graph   │          │  - Proof Gen     │          │  - Bitcoin TX    │
│                  │          │  - Replay Prev   │          │                  │
└──────────────────┘          └──────────────────┘          └──────────────────┘
```

## Integration Flow

### 1. Settlement Request

When an Atomicity node needs to settle an IOU, it creates a `SettlementRequest`:

```rust
use paykit_lib::atomicity::{SettlementRequest, SettlementMethod};

let request = SettlementRequest::new(
    "iou-abc123",           // IOU identifier
    50000,                  // Amount in satoshis
    "lnbc500u1p...",        // Lightning invoice
    SettlementMethod::Lightning,
);
```

### 2. Settlement Execution

The settlement executor handles payment execution:

```rust
use paykit_lib::atomicity::{SettlementExecutor, SettlementProof};

// Implement the executor for your payment infrastructure
struct MySettlementExecutor {
    lnd_client: LndClient,
}

impl SettlementExecutor for MySettlementExecutor {
    async fn execute(&self, request: &SettlementRequest) -> Result<SettlementProof> {
        match request.method {
            SettlementMethod::Lightning => {
                let preimage = self.lnd_client.pay_invoice(&request.payment_details).await?;
                Ok(SettlementProof::from_lightning_preimage(
                    &request.request_id,
                    &request.iou_id,
                    hex::encode(&preimage),
                ))
            }
            SettlementMethod::Onchain => {
                let txid = self.send_onchain(&request.payment_details, request.amount_sats).await?;
                Ok(SettlementProof::from_bitcoin_tx(
                    &request.request_id,
                    &request.iou_id,
                    txid,
                    0,
                    None,
                ))
            }
        }
    }
}
```

### 3. Proof Verification

Atomicity nodes verify settlement proofs:

```rust
use paykit_lib::atomicity::SettlementProof;

let proof = SettlementProof::from_bytes(&proof_bytes)?;

// Verify cryptographic validity
if proof.verify()? {
    // Proof is valid - IOU can be marked as settled
} else {
    // Proof is invalid - settlement disputed
}
```

## Replay Prevention

The `SettlementNonceStorage` trait prevents double-settlement:

```rust
use paykit_lib::atomicity::SettlementNonceStorage;

struct MyNonceStorage {
    db: Database,
}

impl SettlementNonceStorage for MyNonceStorage {
    fn check_and_mark(&self, nonce: &[u8; 32], expires_at: u64) -> Result<bool> {
        // Check if nonce exists in DB
        // If not, insert and return true
        // If exists, return false
    }
    
    fn is_used(&self, nonce: &[u8; 32]) -> Result<bool> {
        // Check if nonce exists
    }
    
    fn cleanup_expired(&self, before: u64) -> Result<()> {
        // Delete expired nonces
    }
}
```

## Message Schemas

### SettlementRequest

```json
{
  "request_id": "settle_abc123def456...",
  "iou_id": "iou-abc123",
  "amount_sats": 50000,
  "method": "lightning",
  "payment_details": "lnbc500u1p...",
  "created_at": 1705776000,
  "expires_at": 1705779600,
  "nonce": "a1b2c3d4...",
  "metadata": null
}
```

### SettlementProof (Lightning)

```json
{
  "request_id": "settle_abc123def456...",
  "iou_id": "iou-abc123",
  "method": "lightning",
  "proof_data": {
    "type": "lightning",
    "preimage": "0001020304...",
    "payment_hash": "abcdef123..."
  },
  "settled_at": 1705776120
}
```

### SettlementProof (On-chain)

```json
{
  "request_id": "settle_abc123def456...",
  "iou_id": "iou-abc123",
  "method": "onchain",
  "proof_data": {
    "type": "onchain",
    "txid": "abcd1234...",
    "vout": 0,
    "block_height": 800000
  },
  "settled_at": 1705776120
}
```

## Security Considerations

1. **Nonce Uniqueness**: Each settlement request has a unique 32-byte nonce
2. **Expiry Enforcement**: Requests expire after 1 hour by default
3. **Proof Verification**: SHA256(preimage) == payment_hash for Lightning
4. **Replay Prevention**: Nonces are tracked persistently

## Error Handling

| Error | Description | Recovery |
|-------|-------------|----------|
| `ExpiredRequest` | Settlement request has expired | Create new request |
| `NonceReused` | Nonce was already used | Create new request with fresh nonce |
| `InvalidProof` | Proof verification failed | Dispute settlement |
| `ExecutionFailed` | Payment execution failed | Retry or cancel |

## Related Documentation

- [Atomicity Specification v1.0](https://github.com/BitcoinErrorLog/atomicity-research/blob/main/Atomicity%20Specification.md)
- [paykit_lib::atomicity](../paykit-lib/src/atomicity.rs) - Rust implementation
