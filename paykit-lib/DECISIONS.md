# DECISIONS.md — Molt wave 2 (paykit-rs, `feat/molt-route`)

## W3 review fixes (2026-09-04, against the updated `pubky-crypto`)

18. **`receive_bonded` binds the polled purpose and the receiver's inbox
    kid.** `pubky_crypto::molt::open` is now
    `open(bytes, ratchet, expected_inbox_kid, expected_purpose)` and
    transactional (authenticate before any ratchet commit). `receive_bonded`
    passes `DROP_INBOX_KID` (the receiver's own inbox kid for Drop traffic,
    decision 11) and the `PurposeId` of the channel being polled. A relay
    copying a valid message between a peer's purpose channels is now rejected
    with `PurposeMismatch` before `peek_recv`/AEAD/commit, consuming no
    ratchet state, so the genuine delivery on the correct channel still opens
    at the same index; ack-delete already happened only after a successful
    open (decision 12) and is unchanged. Regression test:
    `receive_bonded_rejects_cross_channel_copy_without_consuming_ratchet_state`.

19. **Poll response size cap enforced during the read, not after.**
    `DropHttp::http_get` now takes `max_response_bytes` and the trait
    contract requires the backend to abort with an explicit error as soon as
    the cap is exceeded. `ReqwestDropHttp` does a `Content-Length` precheck
    (reject before reading a byte) and a streaming `chunk()` read capped at
    `max_response_bytes` (abort on the first chunk that would exceed it);
    `DropClient::poll` passes `MAX_POLL_RESPONSE_BYTES` and keeps the
    post-read check as defense in depth for lenient backends. `chunk()` is
    used instead of `bytes_stream()` so the `reqwest` dependency needs no new
    feature flags. Test: `poll_rejects_oversize_response_body` (stub returns
    `MAX_POLL_RESPONSE_BYTES + 1`).

20. **Poll CBOR decoding is streaming, strict, and duplicate-free.**
    Integer keys `{0: cursor, 1: ts, 2: body}` are primary (the current
    `pubky-core` relay form); text keys remain as a fallback for the earlier
    relay revision. The previous decoder went through
    `serde_cbor::Value::Map`, whose map representation collapses duplicate
    keys last-write-wins *before* the client could see them. Decoding is now
    a single streaming serde pass: key spellings normalize to one canonical
    field, any duplicate canonical field (int/int, text/text, or mixed
    int/text) is rejected, unknown keys are ignored, known fields with the
    wrong type are errors, the array is bounded at `MAX_DROP_POLL_LIMIT`
    *while* decoding, and trailing bytes after the top-level array are
    rejected. The fallback cannot bypass checks because key spelling only
    selects the canonical field — every value check (type, range,
    duplicates, entry count) runs identically for both spellings (tests:
    `poll_response_rejects_duplicate_canonical_fields`,
    `poll_response_text_fallback_cannot_bypass_checks`).

21. **Vector test tracks the two-viewpoint vector format.** The regenerated
    `molt_crypto_v1.json` records each channel id twice (`epochs_alice`,
    `epochs_bob`, with a `role` label). The cross-check in
    `tests/molt_vectors.rs` now asserts the two viewpoints are equal and each
    matches the `BondSession`-derived id — the same assertion as before,
    made stronger. No product code changed.

## W2b — bonded outbound dispatch (2026-09-04)

22. **`OutboundTransport` is an enum with no shared transport inside.**
    `PublicOutbox` and `Bonded { session, client }` carry no cross-variant
    state, so a failed bonded send cannot "fall back" to the public outbox —
    the bonded route holds no public transport at all (fail closed by
    construction). The public variant also holds no transport: the two
    existing outbox write interfaces in use (`HomeserverSessionStorage` in
    `paykit-subscriptions::discovery`, `pubky::PubkySession` in
    `paykit-subscriptions::manager`) are not unifiable without new
    dependencies, so `OutboundTransport::deliver` invokes the caller's own
    unchanged write path as a closure (`public_write`). This keeps the
    public-outbox bytes byte-identical per call site instead of forcing a
    single storage encoding the ecosystem does not currently have (requests/
    proposals store text Sealed Blobs; ACKs are binary SB2). `deliver` is
    generic over the caller's error type (`E: From<PaykitError>`) so
    `anyhow`-based callers need no mapping glue.

23. **ACK storage gets a named entry point, not a format.** `encrypt_ack`
    previously had no in-repo storage caller (the public-outbox write lived
    entirely downstream), so "route ACK storage" is realized as
    `store_encrypted_ack(encrypted_ack, outbound, public_write)` in
    `protocol/ack.rs` — a thin, fully tested dispatch through
    `OutboundTransport::deliver` with `ProtocolMessageKind::Ack`
    (`ExternallyAuthenticated`). No new public storage encoding is invented;
    the caller's existing write path is used on the public route.

24. **`DropHttp` is implemented for `Box<dyn DropHttp>`.** Lets
    `paykit-subscriptions` store type-erased relay clients in its manager
    registry without making `SubscriptionManager` generic.



Ambiguities in `molt_v11.plan.md` section S9 resolved during implementation,
with reasoning. In every case the more conservative reading was chosen.
Section numbers refer to the v11 plan (the brief named `molt_v10.plan.md`;
only `molt_v11.plan.md` exists — v11 is the frozen, executing spec and its
S9 is what was implemented).

## PeerId conversion

1. **`PeerIdBridge` newtype instead of direct `From` impls.** The brief asks
   for a "From/Into conversion" between `pubky_molt::PeerId` and
   `pubky_crypto::molt::PeerId` inside paykit-lib. Rust's orphan rule forbids
   implementing `From` between two foreign types, so the conversion is
   carried by a local `[u8; 32]` newtype with `From` impls in all four
   directions. Byte-exact, no re-validation (both types are plain 32-byte
   newtypes by design).

## Adapters (S9 table)

2. **IntroAdapter is one type parameterized by `Authenticity`.** The spec's
   two manifests (SessionAuthenticated vs ExternallyAuthenticated) share
   `accepts`/`produces`; a mode selector (`IntroAdapter::session()` /
   `::external()`) picks the manifest and the adapter id
   (`paykit.molt.intro.session` / `paykit.molt.intro.external`). The
   counterparty witness records `ROOT_IDENTITY` on both sides of the hop:
   "learns ROOT_IDENTITY only" is read as *the only Field the counterparty
   learns*, and the counterparty necessarily observes both sides of its own
   intro. Its observation domain is left unknown (empty `domains`), which the
   scorer conservatively reports as `DetachLevel::Unknown` rather than
   assuming independence. The ExternallyAuthenticated homeserver witness
   learns `TIME | CONTENT_SIZE | DEST_ENDPOINT` on the input side only (it
   stores the inbound SB2; it never observes the resulting pairwise
   channel).

3. **`latency_bound_secs: None` on every v1 adapter manifest.** No v1 hop
   has a real latency bound; an unknown bound is conservatively treated as
   *within* the scorer's time window (pubky-molt DECISIONS 6), which
   overstates rather than understates leaks. `Quote.latency_secs` values (5 /
   60 / 10 / 600 s) are illustrative preference numbers only and never feed
   leak counting.

4. **RecoverySemantics choices** (spec is silent): intro `Idempotent`
   (re-delivery re-derives the same bond); Drop transport `BestEffort` (the
   relay offers TTL-bounded, no-guarantee storage; retried PUT duplicates are
   rejected by the receiver's ratchet replay protection); bolt11 `Atomic`
   (matches the S6 `LightningPath` fixture); onchain `BestEffort` (a
   broadcast transaction has no protocol-level recovery).

5. **The relay manifest JSON is embedded verbatim but not serde-parsed into
   a `Manifest`.** `pubky-core`'s `drop_relay_manifest()` encodes `Field`
   sets as JSON arrays of names, while `pubky-molt`'s human-readable `Field`
   form is the bitflags text form (`"A | B"`). The executing adapter builds
   the equivalent `Manifest` programmatically and a test
   (`drop_manifest_matches_relay_json`) pins field-for-field equivalence
   with the embedded JSON. The adapter keeps the relay's own adapter id
   `http-relay.drop.v1` so routes name the actual witness operator class.

6. **DropTransportAdapter accepts any `endpoint_kind` on the
   `pubky-storage` network.** The S9 table constrains only network and
   holder (`Transport{pubky-storage, Self}`); being stricter would invent a
   constraint the spec does not state. It produces
   `Transport{drop, opaque-channel, Self}` (`opaque-channel` matches the
   pubky-molt comparison fixtures).

7. **`PaymentNetworkKind::Other` manifest.** The bridge wraps *any*
   `PaymentMethodPlugin`, but S9 defines manifests only for onchain and
   bolt11. Other methods get a deliberately conservative manifest
   (counterparty learns `RELATIONSHIP_IDENTITY | AMOUNT | TIME`, amount/time
   preserved, no segments, `BestEffort`) — honest about an unprofiled method
   instead of silently declaring no witnesses. `"lightning"` and `"bolt11"`
   method ids both map to the Lightning network; the value-network names are
   `"bitcoin"` / `"lightning"`, matching the S6 fixtures. The adapter id for
   `Other` is `paykit.molt.payment_bridge.{method_id}`.

8. **Empty `Quote.costs`.** Fees are method- and traffic-dependent and
   unknown at planning time; quoting a fabricated number would be less
   honest than quoting none. `SingleAsset(BTC, sat)` reduces empty costs to
   `0.0`.

## Drop transport (`protocol/drop_transport.rs`)

9. **`DropClient` is generic over an injected `DropHttp` backend.** The
   crate's existing HTTP client dependency is `reqwest`, but it is optional
   and native-only (WASM builds of the library must keep working). The
   product backend `ReqwestDropHttp` is gated behind a new `drop-transport`
   feature (`dep:reqwest`); the client core, `BondSession`, and both bonded
   functions stay feature-free and testable. This mirrors the crate's
   transport-abstraction convention (trait injection instead of concrete SDK
   types). The test stub implementing `DropHttp` lives in `tests/`, never in
   `src/`.

10. **Poll responses accept both text-key and integer-key CBOR maps.**
    The spec's S8 table says integer keys; the deployed relay serializes
    Rust structs with text keys (`{"cursor", "ts", "body"}` — the wave-1
    review flag in the plan). Accepting both keeps the client correct
    against either relay revision; types are checked strictly and unknown
    keys ignored.

11. **`inbox_kid` is zero for Drop traffic** (`DROP_INBOX_KID`). Drop
    channels involve no inbox key, and every other identity field in a Molt
    envelope is already zero-filled for anti-correlation; the relationship is
    established by the channel id and ratchet key possession.

12. **Ack-before-skip, never ack-on-failure.** `receive_bonded` ack-deletes
    only successfully opened messages. Messages that fail to open (tampered,
    replay, or a post-mix index awaiting a scheduled mix) are left on the
    relay for TTL expiry — deleting them could destroy evidence a future
    scheduled mix would make readable. Ack failures are tolerated: a
    redelivered duplicate is rejected by ratchet replay protection.

13. **`receive_bonded` fails only when *every* channel poll fails.**
    Returning `Err` after some channels were already drained would discard
    consumed ratchet indices (a redelivery is a `Replay` error, so the
    message would be lost to the application). Delivering partial results
    and letting the caller's next poll retry the failed channels loses
    nothing; a total outage still surfaces as `Err(Transport)`.

14. **Receipts = ACKs for the authenticity mapping.** S9 says "receipts are
    ExternallyAuthenticated (Paykit signs them inside the body per its own
    schema)". Of the three routed message kinds, the ACK is the receipt-like
    object (the receiver's signed statement about a request/proposal), so
    `ProtocolMessageKind::Ack` maps to `ExternallyAuthenticated` and
    requests/proposals to `SessionAuthenticated`. Molt only carries the
    declaration; the signature lives inside the body, which Molt never
    parses.

15. **Client-side bounds mirror the relay's.** Bodies > 64 KiB are rejected
    before sending (`QuotaExceeded`); poll `limit` is clamped to 50; poll
    responses > 4 MiB or with > 50 entries are rejected as malformed. No
    remote input is ever `unwrap()`ed.

## `select_route`

16. **Score-rejected routes are omitted, not returned.** The spec fixes the
    signature `-> Vec<(Route, RouteScore)>`; `ScoreError` means "cannot be
    ranked at all" (ineligible, not a bad score), so ineligible routes are
    filtered out. The planner's `PlanResult.rejected` already carries
    diagnostics for planning failures. Results are sorted ascending by
    `RouteScore::total(weights)` — presentation, not routing logic.

## Gate remediation (pre-existing breakage in scope)

17. **`cargo fmt --check` and `cargo clippy --all-targets --all-features`
    were already red at baseline** (60 fmt diffs workspace-wide; the
    `crypto_benchmarks` bench did not compile; ~30 lint warnings across
    paykit-lib targets). Within the allowed scope (`paykit-lib/`), these were
    normalized: `cargo fmt -p paykit-lib`; mechanical `clippy --fix`
    applications; `ed25519-dalek` dev-dependency gained its `rand_core`
    feature so the existing bench compiles; two lints were `allow`ed with
    in-code justification (`AckObjectType::from_str` kept for API
    compatibility; deprecated `context_id` exercised intentionally by the
    BIP compatibility vectors). No logic changed. Crates outside paykit-lib
    (`paykit-demo-*`, `paykit-interactive`, `paykit-mobile`,
    `paykit-subscriptions`) still have pre-existing rustfmt diffs; the brief
    forbids touching them, so the bare workspace `cargo fmt --check` remains
    red there — flagged for the parent's wave-5 integration.

## Notes

- The plan's wave-5 `rg` proof gate for `v0/requests/` still matches
  existing path constants and doc comments: the brief mandates that all
  existing tests stay green and the public endpoints stay unchanged, so the
  legacy paths remain (the bonded Drop path is additive). Flagged for the
  parent.
