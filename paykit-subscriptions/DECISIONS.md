# DECISIONS.md — paykit-subscriptions

## W2b — bonded outbound dispatch (2026-09-04, `feat/molt-route`)

Ambiguities in the Molt W2b follow-up (bonded delivery of Paykit protocol
messages) resolved during implementation. The dispatch primitive itself
(`OutboundTransport`, `ProtocolMessageKind`, `send_protocol_message`) lives
in `paykit-lib::protocol::drop_transport`; see `paykit-lib/DECISIONS.md`
items 22–24 for its design.

1. **Selection rule is per call site and explicit.** A message is bonded
   **if and only if** the caller supplied a `BondSession` for that peer:

   - `discovery::publish_payment_request_routed` takes an
     `&mut OutboundTransport` parameter; the legacy
     `publish_payment_request` is unchanged and keeps its exact previous
     behavior (the encryption was extracted into a private
     `seal_payment_request` helper shared by both entry points — same path,
     same blob, same errors).
   - `SubscriptionManager` gains a small registry
     (`add_bonded_outbound` / `remove_bonded_outbound` /
     `has_bonded_outbound`) keyed by peer z32. `propose_subscription`
     delivers bonded when the subscriber is registered, otherwise writes
     the public outbox exactly as before (the encryption was extracted into
     a private `seal_subscription_proposal` helper shared by both paths).
   - ACK storage is routed in `paykit-lib` (`store_encrypted_ack`), not
     here.

2. **No silent fallback from bonded to public.** The bonded branch of
   `propose_subscription` returns the `send_protocol_message` error
   (wrapped with "no public fallback" context) and never attempts the
   public write; structurally it cannot, because the bonded branch runs
   instead of — not before — the public-outbox branch. The same holds in
   `publish_payment_request_routed` via `OutboundTransport::deliver`.

3. **The payload is identical on both routes.** The bonded body is the same
   Sealed Blob (same canonical path in the AAD) that the public path would
   have stored, so a recipient can verify/decrypt it with the unchanged
   schema regardless of how it arrived. Molt carries the blob unparsed; the
   `/pub/` path remains only as the AAD's canonical-path binding, never as
   a write target on the bonded route.

4. **The registry holds `DropClient<Box<dyn DropHttp>>`.**
   `SubscriptionManager` is a concrete struct used across FFI; making it
   generic over the relay backend would ripple through every consumer.
   Type erasure costs one pointer indirection per relay call, nothing more.
   The registry lives behind a `RwLock` because `propose_subscription`
   takes `&self`.

5. **Tests use in-process doubles only.** The Drop relay stub and the
   recording `HomeserverSessionStorage` mock are test-only (a
   `#[cfg(test)]` module stub in `manager.rs`, plus
   `tests/bonded_dispatch.rs`); `base64` and `serde_cbor` were added as
   dev-dependencies for URL-key parsing and CBOR encoding in those stubs.
   No production code path was mocked.
