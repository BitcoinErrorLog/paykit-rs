# Subscription Nonce Derivation Design

**Status**: Design document (not yet implemented)
**Date**: 2026-02-07
**Origin**: Antoine Theme 7 (PUBKY_CRYPTO_SPEC review), remediation plan audit-remediation-2026-02-07

## Problem Statement

Subscription signatures in `paykit-subscriptions` use random nonces (`rand::random::<[u8; 32]>()`). These nonces are recorded in a `NonceStorage` implementation to prevent replay attacks.

**Current risk**: If the nonce store is lost (device reset, corrupted storage, migration failure), the app cannot reconstruct which nonces were already used. An attacker who captured a valid signed message could replay it, and the fresh nonce store would accept it.

**Current mitigation**: `FileNonceStorage` persists to disk and survives app restarts. Mobile apps use platform-native storage (Room, Core Data) via FFI callbacks. This is adequate for normal operation but does not survive full data loss.

## Design Goals

1. Nonces should be **derivable** from a root secret + monotonic counter, so they can be reconstructed after data loss.
2. The derivation must be **deterministic**: same inputs always produce the same nonce.
3. The counter must be **monotonic and persistent**: the app must never reuse a counter value.
4. The design must be **backward-compatible**: existing nonce stores continue to work; derived nonces are a new generation mode, not a replacement of the storage trait.

## Proposed Scheme

### Nonce Derivation Function

```
nonce = HKDF-SHA256(
    salt = "paykit-subscription-nonce/v1",
    ikm  = root_secret (32 bytes, from Ring or noise_seed),
    info = counter (u64, big-endian)
) → 32 bytes
```

Where:
- `root_secret` is the noise seed (already available to Bitkit after Paykit-Connect handoff) or a dedicated subscription seed derived from Ring.
- `counter` is a monotonically increasing u64, persisted alongside the nonce store.

### Counter Management

| Property | Rule |
|----------|------|
| Initial value | 0 |
| Increment | +1 per nonce generation |
| Persistence | Written to `NonceStorage` or separate counter file before the nonce is used |
| Recovery | After data loss, app sets counter to `last_known_counter + safety_margin` (e.g., +1000) to skip any potentially-used range |
| Upper bound | u64 provides ~18 quintillion nonces; no practical limit |

### Where the Counter Lives

**Decision**: In the app (Bitkit), not in Ring.

Rationale:
- Ring is stateless per call (PUBKY_CRYPTO_SPEC §5). Adding counter state to Ring breaks this invariant.
- The noise_seed is already handed off to Bitkit during Paykit-Connect. Nonce derivation from this seed is a local computation.
- Counter persistence uses the same `NonceStorage` trait that already exists.

### Recovery After Data Loss

1. App detects missing nonce store (empty or absent file).
2. App cannot know the exact last counter value.
3. App sets counter to a **safe floor**: `peer_count * estimated_nonces_per_peer + safety_margin`.
4. Alternatively, the counter value can be stored in a backup (e.g., alongside the encrypted seed backup in Ring's secure handoff).

### Migration Path

1. Existing `NonceStorage` trait gains an optional `get_counter() -> Option<u64>` and `increment_counter() -> u64` method (with default implementations returning `None` / panicking, so existing impls don't break).
2. `SubscriptionManager` checks: if `noise_seed` is available AND counter storage is available, use derived nonces. Otherwise, fall back to `rand::random`.
3. `FileNonceStorage` adds a `counter.json` file alongside `nonces.json`.

### pubky-crypto Changes

If this function is deemed a general-purpose primitive, add to `pubky-crypto/src/kdf.rs`:

```rust
pub fn derive_subscription_nonce(
    seed: &[u8; 32],
    counter: u64,
) -> Result<[u8; 32], CryptoError> {
    let salt = b"paykit-subscription-nonce/v1";
    let hk = Hkdf::<Sha256>::new(Some(salt), seed);
    let mut nonce = [0u8; 32];
    hk.expand(&counter.to_be_bytes(), &mut nonce)
        .map_err(|e| CryptoError::KeyDerivation(format!("HKDF expand failed: {:?}", e)))?;
    Ok(nonce)
}
```

If scoped only to subscriptions, this function can live in `paykit-subscriptions` instead.

## Security Analysis

| Property | Random nonces (current) | Derived nonces (proposed) |
|----------|------------------------|--------------------------|
| Uniqueness | Cryptographically random; collision probability ~2^-128 | Deterministic; unique as long as counter never repeats |
| Replay after data loss | Vulnerable (fresh store accepts replayed nonces) | Resistant (counter skip ensures no overlap) |
| Counter reuse | N/A | MUST be prevented; write counter before signing |
| Seed compromise | N/A | All past and future nonces are computable; but seed compromise already implies full key compromise |

## Open Questions

1. **Where to persist the counter on mobile?** The `NonceStorage` FFI callback interface would need extension. This is a non-trivial mobile SDK change.
2. **Should the counter be per-peer or global?** Global is simpler; per-peer provides better isolation but more state.
3. **Backup integration**: Should the counter be included in Ring's secure handoff backup payload?

## Decision

This design is **deferred to post-MVP**. The current random nonce approach with `FileNonceStorage` (persistent across restarts) is secure for production use. Deterministic nonce derivation is a hardening improvement for the data-loss edge case and should be implemented when the subscription protocol moves to production scale.

## References

- PUBKY_CRYPTO_SPEC v2.5, Section 5 (Ring stateless design)
- IMPLEMENTATION_PLAN.md, Phase 5, "Subscription nonces (Antoine Theme 7)"
- `paykit-subscriptions/src/nonce_store.rs` (current implementation)
- `pubky-crypto/src/kdf.rs` (existing KDF functions)
