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

## W4 external-audit fixes (2026-09-04, `feat/molt-route`)

6. **SF-2: one registry guard for bonded check-and-dispatch.**
   `propose_subscription` used to check `bonded_outbounds` under a read
   guard, drop it, then re-acquire the write guard and
   `expect("presence checked above")` — a reachable panic if
   `remove_bonded_outbound` raced in between. All bonded delivery now
   funnels through the private `deliver_bonded` helper, which holds ONE
   write guard across the membership check and the dispatch; a route
   removed between the caller's presence check and the send yields a clean
   "no longer registered … no public fallback" error. Sealing stays outside
   the guard (it performs network I/O); the guard is held across the
   dispatch await so registration state cannot change mid-send. Regression
   test: `test_bonded_delivery_route_removed_midflight_fails_closed`
   (all three message kinds).

7. **SF-3: requests, ACKs, and polling are wired through the registry.**
   The manager dispatches all three protocol message kinds through the same
   `BondedRoute` registry the proposals use:
   `SubscriptionManager::publish_payment_request` (bonded delivery when a
   route exists for the recipient, byte-identical legacy
   `discovery::publish_payment_request` otherwise),
   `SubscriptionManager::store_encrypted_ack` (bonded delivery when a route
   exists; the caller's `public_write` closure runs untouched otherwise),
   and `SubscriptionManager::poll_bonded` (the in-repo caller of
   `paykit_lib::protocol::drop_transport::receive_bonded` across all
   registered routes; errors only when every route fails to poll, mirroring
   `receive_bonded`'s partial-failure semantics). All bonded paths fail
   closed — an explicit error, never a silent public fallback.
   `discovery::seal_payment_request` is now `pub(crate)` so the manager
   builds the identical payload on both routes. Tests:
   `test_publish_payment_request_bonded_arrives_via_drop` (storage mock
   records zero `/pub/` writes, request arrives via the Drop stub),
   `test_store_encrypted_ack_bonded_arrives_via_drop` (public write never
   invoked; ACK arrives `ExternallyAuthenticated`),
   `test_publish_payment_request_unbonded_is_byte_identical_legacy`,
   `test_store_encrypted_ack_unbonded_runs_public_write`,
   `test_poll_bonded_receives_across_registered_routes`,
   `test_poll_bonded_errors_when_every_route_fails`.
