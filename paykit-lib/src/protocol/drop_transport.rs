//! S9 Drop transport: bonded dead-drop channels for Paykit protocol traffic.
//!
//! This module moves Paykit request/proposal/ACK traffic off publicly rooted
//! homeserver paths onto S8 Drop channels derived from the pairwise Bond.
//! Payloads are sealed with [`pubky_crypto::molt::seal`] under the send
//! ratchet's message key and the purpose `pubky.molt.paykit.v1`
//! ([`PurposeId::PAYKIT`]); the relay sees only opaque channel ids and
//! ciphertext.
//!
//! - [`DropClient`] speaks to the three S8 HTTP endpoints (`PUT/GET/DELETE
//!   /drop/…`) through a [`DropHttp`] backend. The product backend is
//!   `ReqwestDropHttp` (feature `drop-transport`, native targets only);
//!   tests substitute an in-process stub.
//! - [`BondSession`] bundles one relationship's Bond, both ratchet
//!   directions, and the recovery record.
//! - [`send_bonded`] / [`receive_bonded`] are the v1 send/receive paths.
//!
//! Channel ids are never logged here (nothing in this module logs above
//! `debug`; in fact it does not log at all). Receipts (ACKs) are declared
//! [`Authenticity::ExternallyAuthenticated`]: Paykit signs them inside the
//! body per its own schema and Molt carries the flag untouched. Public
//! method endpoints under `/pub/paykit.app/v0/{method}` are root-anchored
//! and unchanged by this module.

use crate::{PaykitError, Result};
use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use pubky_crypto::molt::{
    self, channel_id, directions_for, epoch_at, Authenticity, Bond, BondRecord, ChannelId, Epoch,
    Header, MoltEnvelope, PeerId, PurposeId, RatchetState,
};

/// The S8 relay's PUT bound on the whole SB2 wire blob ("SB2 ≤ 64 KiB").
pub const MAX_DROP_WIRE_BYTES: usize = 64 * 1024;

/// Exact SB2+Molt sealing overhead over the plaintext body for Drop
/// traffic — computed from the deterministic-CBOR header profile that
/// [`pubky_crypto::molt::seal`] emits (see `encode_molt_header` and
/// `Sb2Header::encode_no_sig` in pubky-crypto), not estimated:
///
/// | Piece | Bytes |
/// |---|---|
/// | SB2 magic + version + u16 header length | 6 |
/// | header map(10) byte | 1 |
/// | key 0 `context_id` bytes(32) | 35 |
/// | key 3 `inbox_kid` bytes(16) | 18 |
/// | key 5 `nonce` bytes(24) | 27 |
/// | key 6 `purpose` text(20) `pubky.molt.paykit.v1` | 22 |
/// | keys 7–9 zeroed peerid/ephemeral bytes(32), ×3 | 105 |
/// | key 20 `dir` uint | 2 |
/// | key 21 `n` uint, worst-case 8-byte CBOR varint | 10 |
/// | key 22 `authenticity` uint | 2 |
/// | XChaCha20-Poly1305 tag | 16 |
///
/// Total 244. The ratchet index `n` is charged its worst-case CBOR varint
/// (1 key byte + 9 value bytes) so the bound holds for every index, and the
/// purpose is the 20-byte `pubky.molt.paykit.v1` every product path seals
/// with; [`send_bonded`] accounts for other purpose lengths exactly via
/// [`drop_wire_size`].
pub const DROP_SEAL_OVERHEAD_BYTES: usize = 244;

/// Maximum plaintext body [`send_bonded`] carries: the largest body whose
/// sealed wire blob stays within the relay's [`MAX_DROP_WIRE_BYTES`] PUT
/// bound ([`MAX_DROP_WIRE_BYTES`] `−` [`DROP_SEAL_OVERHEAD_BYTES`]). Larger
/// bodies are rejected with [`PaykitError::QuotaExceeded`] before sealing
/// and before any network call.
pub const MAX_DROP_BODY_BYTES: usize = MAX_DROP_WIRE_BYTES - DROP_SEAL_OVERHEAD_BYTES;

/// Maximum poll page size (S8: `limit ≤ 50`).
pub const MAX_DROP_POLL_LIMIT: u32 = 50;

/// Defensive upper bound on a poll response body
/// ([`MAX_DROP_POLL_LIMIT`] messages of [`MAX_DROP_WIRE_BYTES`] plus
/// framing slack).
pub const MAX_POLL_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// The `inbox_kid` carried in Drop-traffic envelope headers.
///
/// Drop channels involve no inbox key; the field is zero-filled like every
/// other identity field in a Molt envelope (the relationship is established
/// by the channel id and possession of the ratchet key). See `DECISIONS.md`.
pub const DROP_INBOX_KID: [u8; 16] = [0u8; 16];

/// One message returned by a Drop channel poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropMessage {
    /// Relay-assigned cursor, used for ack-delete.
    pub cursor: u64,
    /// Relay-side unix timestamp of the append.
    pub ts: u64,
    /// The raw (sealed) message bytes.
    pub body: Vec<u8>,
}

/// HTTP backend abstraction for [`DropClient`].
///
/// Product code uses `ReqwestDropHttp`; tests substitute an in-process
/// stub. Keeping the client stateless and the backend injected follows the
/// crate's transport-abstraction convention.
#[async_trait]
pub trait DropHttp: Send + Sync {
    /// `PUT` `body` to `url`; returns the `X-Drop-Cursor` value.
    async fn http_put(&self, url: &str, body: Vec<u8>) -> Result<u64>;

    /// `GET` `url`; returns the raw response body (a CBOR array).
    ///
    /// The backend MUST NOT return more than `max_response_bytes` bytes: it
    /// must reject the response (explicit error) as soon as the cap is known
    /// to be exceeded (a declared `Content-Length` above the cap, or the
    /// streaming body growing past it) rather than buffering an unbounded
    /// body first.
    async fn http_get(&self, url: &str, max_response_bytes: usize) -> Result<Vec<u8>>;

    /// `DELETE` `url`. Success or already-absent both count as `Ok`.
    async fn http_delete(&self, url: &str) -> Result<()>;
}

#[async_trait]
impl DropHttp for Box<dyn DropHttp> {
    async fn http_put(&self, url: &str, body: Vec<u8>) -> Result<u64> {
        (**self).http_put(url, body).await
    }

    async fn http_get(&self, url: &str, max_response_bytes: usize) -> Result<Vec<u8>> {
        (**self).http_get(url, max_response_bytes).await
    }

    async fn http_delete(&self, url: &str) -> Result<()> {
        (**self).http_delete(url).await
    }
}

/// Client for the S8 Drop relay endpoints.
///
/// | Method | Endpoint |
/// |---|---|
/// | `PUT` | `/drop/{channel_b64url}` — append, `201` + `X-Drop-Cursor` |
/// | `GET` | `/drop/{channel}?since={cursor}&limit={n≤50}` — poll, CBOR array |
/// | `DELETE` | `/drop/{channel}/{cursor}` — ack-delete one |
#[derive(Debug, Clone)]
pub struct DropClient<H> {
    base_url: String,
    http: H,
}

impl<H: DropHttp> DropClient<H> {
    /// Create a client for a relay base URL (e.g. `https://relay.example`).
    ///
    /// # Errors
    ///
    /// Returns [`PaykitError::InvalidData`] if the base URL is empty or does
    /// not start with `http://` or `https://`.
    pub fn new(base_url: impl Into<String>, http: H) -> Result<Self> {
        let base = base_url.into();
        let base = base.trim_end_matches('/');
        if base.is_empty() || !(base.starts_with("https://") || base.starts_with("http://")) {
            return Err(PaykitError::InvalidData {
                field: "base_url".into(),
                reason: "must be a non-empty http(s) URL".into(),
            });
        }
        Ok(DropClient {
            base_url: base.to_string(),
            http,
        })
    }

    /// The relay base URL (without trailing slash).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The underlying HTTP backend.
    pub fn backend(&self) -> &H {
        &self.http
    }

    fn channel_url(&self, channel: &ChannelId) -> String {
        format!(
            "{}/drop/{}",
            self.base_url,
            URL_SAFE_NO_PAD.encode(channel.0)
        )
    }

    /// Append a sealed message to `channel`, returning the relay cursor.
    ///
    /// # Errors
    ///
    /// - [`PaykitError::QuotaExceeded`]: wire body larger than
    ///   [`MAX_DROP_WIRE_BYTES`] (rejected client-side, never sent).
    /// - [`PaykitError::Transport`]: the relay rejected or was unreachable.
    pub async fn put(&self, channel: &ChannelId, body: &[u8]) -> Result<u64> {
        if body.len() > MAX_DROP_WIRE_BYTES {
            return Err(PaykitError::QuotaExceeded {
                used: body.len() as u64,
                limit: MAX_DROP_WIRE_BYTES as u64,
            });
        }
        self.http
            .http_put(&self.channel_url(channel), body.to_vec())
            .await
    }

    /// Poll `channel` for messages (optionally after `since`), returning at
    /// most `limit` (clamped to [`MAX_DROP_POLL_LIMIT`]).
    ///
    /// # Errors
    ///
    /// - [`PaykitError::Transport`]: the relay rejected or was unreachable.
    /// - [`PaykitError::Serialization`]: the response was not a well-formed
    ///   Drop message array (oversized, malformed CBOR, or more than
    ///   [`MAX_DROP_POLL_LIMIT`] entries).
    pub async fn poll(
        &self,
        channel: &ChannelId,
        since: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<DropMessage>> {
        let limit = limit
            .unwrap_or(MAX_DROP_POLL_LIMIT)
            .min(MAX_DROP_POLL_LIMIT);
        let mut url = self.channel_url(channel);
        let mut sep = '?';
        if let Some(since) = since {
            url.push_str(&format!("{sep}since={since}"));
            sep = '&';
        }
        url.push_str(&format!("{sep}limit={limit}"));
        // The cap is enforced by the backend *during* the read (see the
        // `DropHttp::http_get` contract); the post-read check stays as
        // defense in depth for backends that honor the contract loosely.
        let bytes = self.http.http_get(&url, MAX_POLL_RESPONSE_BYTES).await?;
        if bytes.len() > MAX_POLL_RESPONSE_BYTES {
            return Err(PaykitError::Serialization(
                "drop poll response exceeds size bound".into(),
            ));
        }
        decode_poll_response(&bytes)
    }

    /// Ack-delete the message at `cursor` on `channel`.
    ///
    /// # Errors
    ///
    /// [`PaykitError::Transport`] if the relay rejected or was unreachable.
    /// An already-absent message is not an error.
    pub async fn ack(&self, channel: &ChannelId, cursor: u64) -> Result<()> {
        let url = format!("{}/{}", self.channel_url(channel), cursor);
        self.http.http_delete(&url).await
    }
}

/// Decode a Drop poll response: a CBOR array of `{cursor, ts, body}` maps.
///
/// Integer keys `{0: cursor, 1: ts, 2: body}` are primary — that is the
/// deterministic form the current `pubky-core` http-relay emits. Text keys
/// (`"cursor"`, `"ts"`, `"body"`, the earlier relay revision) remain
/// accepted as a fallback, but a *canonical field* (`cursor`/`ts`/`body`
/// regardless of key spelling) may appear at most once per map: duplicates
/// are rejected rather than last-write-wins, so the text fallback cannot be
/// used to override or smuggle a second value past the integer form (see
/// `DECISIONS.md`). Unknown keys are ignored; a known field with the wrong
/// type is an error.
///
/// Decoding is a single streaming pass over the CBOR (not a
/// `serde_cbor::Value` round-trip, whose map representation would silently
/// collapse duplicates) and the array is bounded at
/// [`MAX_DROP_POLL_LIMIT`] entries *while* decoding.
fn decode_poll_response(bytes: &[u8]) -> Result<Vec<DropMessage>> {
    use serde::Deserialize;
    let mut de = serde_cbor::Deserializer::from_slice(bytes);
    let response = PollResponse::deserialize(&mut de)
        .map_err(|e| PaykitError::Serialization(format!("invalid drop poll CBOR: {e}")))?;
    // Reject trailing data after the top-level array.
    de.end()
        .map_err(|e| PaykitError::Serialization(format!("invalid drop poll CBOR: {e}")))?;
    Ok(response.0)
}

/// Top-level poll response wrapper with a bounded streaming visitor.
struct PollResponse(Vec<DropMessage>);

impl<'de> serde::Deserialize<'de> for PollResponse {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = PollResponse;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a CBOR array of drop messages")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<PollResponse, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut items = Vec::new();
                while let Some(msg) = seq.next_element::<WireDropMessage>()? {
                    if items.len() >= MAX_DROP_POLL_LIMIT as usize {
                        return Err(serde::de::Error::custom(format!(
                            "drop poll response exceeds {MAX_DROP_POLL_LIMIT} entries"
                        )));
                    }
                    items.push(msg.0);
                }
                Ok(PollResponse(items))
            }
        }
        deserializer.deserialize_seq(V)
    }
}

/// A Drop message map key: integer keys (primary, current relay form) and
/// text keys (fallback, earlier relay form) normalize to the same canonical
/// field so mixed-spelling duplicates are detected.
enum FieldKey {
    /// `0` / `"cursor"`.
    Cursor,
    /// `1` / `"ts"`.
    Ts,
    /// `2` / `"body"`.
    Body,
    /// Any other key (ignored, value skipped).
    Unknown,
}

impl<'de> serde::Deserialize<'de> for FieldKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V;
        impl serde::de::Visitor<'_> for V {
            type Value = FieldKey;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("integer 0/1/2 or text cursor/ts/body")
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<FieldKey, E> {
                Ok(match v {
                    0 => FieldKey::Cursor,
                    1 => FieldKey::Ts,
                    2 => FieldKey::Body,
                    _ => FieldKey::Unknown,
                })
            }

            fn visit_i64<E>(self, _v: i64) -> std::result::Result<FieldKey, E> {
                // Negative integers are not field keys (and are not valid
                // values for cursor/ts either).
                Ok(FieldKey::Unknown)
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<FieldKey, E> {
                Ok(match v {
                    "cursor" => FieldKey::Cursor,
                    "ts" => FieldKey::Ts,
                    "body" => FieldKey::Body,
                    _ => FieldKey::Unknown,
                })
            }
        }
        deserializer.deserialize_any(V)
    }
}

/// One `{cursor, ts, body}` map with strict field semantics.
struct WireDropMessage(DropMessage);

impl<'de> serde::Deserialize<'de> for WireDropMessage {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = WireDropMessage;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a CBOR map {0: cursor, 1: ts, 2: body}")
            }

            fn visit_map<A>(self, mut map: A) -> std::result::Result<WireDropMessage, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut cursor = None;
                let mut ts = None;
                let mut body = None;
                while let Some(key) = map.next_key::<FieldKey>()? {
                    let slot = match key {
                        FieldKey::Cursor => &mut cursor,
                        FieldKey::Ts => &mut ts,
                        FieldKey::Body => &mut body,
                        FieldKey::Unknown => {
                            // Unknown key: skip its value, whatever it is.
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                            continue;
                        }
                    };
                    if slot.is_some() {
                        return Err(serde::de::Error::custom(
                            "duplicate drop message field (cursor/ts/body)",
                        ));
                    }
                    let value = map.next_value::<serde_cbor::Value>()?;
                    *slot = Some(value);
                }
                fn uint_field(
                    field: &str,
                    value: Option<serde_cbor::Value>,
                ) -> std::result::Result<u64, String> {
                    match value {
                        Some(serde_cbor::Value::Integer(i)) => u64::try_from(i)
                            .map_err(|_| format!("drop message field {field} out of range")),
                        Some(_) => Err(format!("drop message field {field} has unexpected type")),
                        None => Err("drop message missing cursor/ts/body".to_string()),
                    }
                }
                let cursor = uint_field("cursor", cursor).map_err(serde::de::Error::custom)?;
                let ts = uint_field("ts", ts).map_err(serde::de::Error::custom)?;
                let body = match body {
                    Some(serde_cbor::Value::Bytes(b)) => b,
                    Some(_) => {
                        return Err(serde::de::Error::custom(
                            "drop message field body has unexpected type",
                        ))
                    }
                    None => {
                        return Err(serde::de::Error::custom(
                            "drop message missing cursor/ts/body",
                        ))
                    }
                };
                Ok(WireDropMessage(DropMessage { cursor, ts, body }))
            }
        }
        deserializer.deserialize_map(V)
    }
}

/// One relationship's bonded session: the Bond, both ratchet directions, and
/// the recovery record (S9).
///
/// `send` and `recv` are bootstrapped from the canonical traffic directions
/// for `(me, peer)` so both sides of the relationship derive identical
/// channel ids and ratchet roots for a given flow.
pub struct BondSession {
    /// The counterparty's root identity.
    pub peer: PeerId,
    /// The pairwise Bond `K_AB`. Never used as a traffic key directly.
    pub bond: Bond,
    /// Outgoing ratchet (this side's send direction).
    pub send: RatchetState,
    /// Incoming ratchet (this side's receive direction).
    pub recv: RatchetState,
    /// Recovery record (peer pair public, epoch length, relay URLs).
    pub record: BondRecord,
}

impl BondSession {
    /// Bootstrap a session for the relationship `(me, peer)`.
    ///
    /// `record.epoch_secs` scopes channel rotation; it should match the
    /// value negotiated in the intro (default 86400).
    pub fn new(me: &PeerId, peer: PeerId, bond: Bond, record: BondRecord) -> Self {
        let (send_dir, recv_dir) = directions_for(me, &peer);
        BondSession {
            peer,
            send: RatchetState::bootstrap(&bond, send_dir),
            recv: RatchetState::bootstrap(&bond, recv_dir),
            bond,
            record,
        }
    }

    /// The Drop channel id for outgoing traffic at `epoch`.
    pub fn send_channel(&self, purpose: &PurposeId, epoch: Epoch) -> ChannelId {
        channel_id(&self.bond, self.send.direction(), purpose, epoch)
    }

    /// The Drop channel id for incoming traffic at `epoch`.
    pub fn recv_channel(&self, purpose: &PurposeId, epoch: Epoch) -> ChannelId {
        channel_id(&self.bond, self.recv.direction(), purpose, epoch)
    }

    /// The epochs a receiver polls: `{e-1, e, e+1}` for the current wall
    /// clock (S2), deduplicated at the edges of the epoch counter.
    ///
    /// # Errors
    ///
    /// [`PaykitError::Crypto`] if the epoch cannot be computed (zero epoch
    /// length or counter overflow).
    pub fn poll_epochs(&self, now_unix_secs: u64) -> Result<Vec<Epoch>> {
        let e = current_epoch(now_unix_secs, self.record.epoch_secs)?;
        let mut epochs = vec![e];
        let prev = e.saturating_sub(1);
        if prev != e {
            epochs.push(prev);
        }
        let next = e.saturating_add(1);
        if next != e {
            epochs.push(next);
        }
        Ok(epochs)
    }
}

fn current_epoch(now_unix_secs: u64, epoch_secs: u32) -> Result<Epoch> {
    epoch_at(now_unix_secs, epoch_secs).map_err(|e| PaykitError::Crypto {
        operation: "molt_epoch".into(),
        details: e.to_string(),
    })
}

fn now_unix_secs() -> Result<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| PaykitError::Internal("system clock is before the unix epoch".into()))
}

/// Exact worst-case sealed wire size for `body_len` plaintext bytes under
/// `purpose`: [`DROP_SEAL_OVERHEAD_BYTES`] minus its 22-byte paykit-purpose
/// entry, plus the exact CBOR entry for this purpose (key byte + string
/// header + string bytes), plus the body. The ratchet index stays charged
/// at its worst-case 9-byte varint, so the result is an upper bound of the
/// actual wire size for every index.
fn drop_wire_size(purpose: &PurposeId, body_len: usize) -> usize {
    /// All wire bytes except the body and the purpose entry; see the
    /// itemization on [`DROP_SEAL_OVERHEAD_BYTES`].
    const FIXED_WITHOUT_PURPOSE: usize = DROP_SEAL_OVERHEAD_BYTES - 22;
    let purpose_len = purpose.as_str().len();
    // Mirrors `CborWriter::write_str` in pubky-crypto.
    let str_header = if purpose_len < 24 {
        1
    } else if purpose_len < 256 {
        2
    } else {
        3
    };
    FIXED_WITHOUT_PURPOSE + 1 + str_header + purpose_len + body_len
}

/// Seal `env` with the session's next send-side message key and append it to
/// the current epoch's Drop channel (S9).
///
/// The header `{dir, n, purpose, authenticity}` is bound in the envelope
/// AAD by [`pubky_crypto::molt::seal`]. A body whose sealed wire would
/// exceed the relay's [`MAX_DROP_WIRE_BYTES`] PUT bound is rejected
/// **before** the ratchet index is consumed and before any network call
/// (bodies up to [`MAX_DROP_BODY_BYTES`] always fit under the paykit
/// purpose; longer purposes shrink the allowance exactly). Otherwise the
/// ratchet index is consumed even if the relay append fails; the caller may
/// retry with a fresh envelope (re-sends use a fresh index, and receivers
/// reject replays).
///
/// # Errors
///
/// - [`PaykitError::Crypto`]: sealing or epoch derivation failed.
/// - [`PaykitError::QuotaExceeded`]: the sealed body would exceed the S8
///   wire bound.
/// - [`PaykitError::Transport`]: the relay append failed.
pub async fn send_bonded<H: DropHttp>(
    s: &mut BondSession,
    env: &MoltEnvelope<'_>,
    c: &DropClient<H>,
) -> Result<()> {
    let wire_len = drop_wire_size(env.purpose, env.body.len());
    if wire_len > MAX_DROP_WIRE_BYTES {
        return Err(PaykitError::QuotaExceeded {
            used: wire_len as u64,
            limit: MAX_DROP_WIRE_BYTES as u64,
        });
    }
    let (n, mk) = s.send.next_send();
    let hdr = Header {
        dir: s.send.direction(),
        n,
        purpose: env.purpose.clone(),
        authenticity: env.authenticity,
    };
    let wire = molt::seal(env, &mk, &hdr, &DROP_INBOX_KID).map_err(|e| PaykitError::Crypto {
        operation: "molt_seal".into(),
        details: e.to_string(),
    })?;
    let epoch = current_epoch(now_unix_secs()?, s.record.epoch_secs)?;
    let channel = s.send_channel(env.purpose, epoch);
    c.put(&channel, &wire).await?;
    Ok(())
}

/// Poll every session's receive channels ({e-1, e, e+1} per purpose) and
/// open every envelope that authenticates (S9).
///
/// Returns one `(peer, header, authenticity, body)` tuple per opened
/// envelope; the body is returned untouched (Molt never parses bodies) and
/// the authenticity is exactly the AAD-bound declaration. Every envelope is
/// opened against the receiver's own inbox kid and the `PurposeId` of the
/// channel being polled, so a message a relay copied between a peer's
/// purpose channels is rejected (`PurposeMismatch`) without consuming
/// ratchet state; the genuine delivery on the correct channel still opens
/// at the same index. Successfully opened messages are ack-deleted — and
/// only those: messages that fail to open are skipped and left on the relay
/// for TTL expiry (they may belong to a post-mix future the receiver has
/// not scheduled yet). Ack failures are tolerated: a redelivered duplicate
/// is rejected by ratchet replay protection.
///
/// # Errors
///
/// [`PaykitError::Transport`] only when **every** channel poll failed — if
/// at least one channel answered, the messages collected so far are
/// returned and the failed channels are retried on the caller's next poll,
/// so consumed ratchet indices are never lost to a discarded error.
pub async fn receive_bonded<H: DropHttp>(
    sessions: &mut [BondSession],
    purposes: &[PurposeId],
    c: &DropClient<H>,
) -> Result<Vec<(PeerId, Header, Authenticity, Vec<u8>)>> {
    let now = now_unix_secs()?;
    let mut out = Vec::new();
    let mut first_error: Option<PaykitError> = None;
    let mut polls_ok = 0usize;
    let mut polls_total = 0usize;

    for s in sessions.iter_mut() {
        let epochs = s.poll_epochs(now)?;
        for purpose in purposes {
            for epoch in &epochs {
                let channel = s.recv_channel(purpose, *epoch);
                polls_total += 1;
                let messages = match c.poll(&channel, None, None).await {
                    Ok(messages) => {
                        polls_ok += 1;
                        messages
                    }
                    Err(e) => {
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                        continue;
                    }
                };
                for msg in messages {
                    // Bind the receiver's own inbox kid and the purpose of
                    // the channel being polled: the ratchet is per direction
                    // but shared across purposes, so a relay copying a valid
                    // message between a peer's purpose channels must be
                    // rejected with PurposeMismatch BEFORE any ratchet state
                    // is consumed (transactional open in pubky-crypto).
                    match molt::open(&msg.body, &mut s.recv, &DROP_INBOX_KID, purpose) {
                        Ok((hdr, body)) => {
                            let authenticity = hdr.authenticity;
                            out.push((s.peer, hdr, authenticity, body));
                            // Best-effort ack: failure only means a later
                            // redelivery, which the ratchet rejects.
                            let _ = c.ack(&channel, msg.cursor).await;
                        }
                        Err(_) => {
                            // Unauthenticatable (tampered, replay, or
                            // awaiting a scheduled mix): skip without
                            // acking; TTL purges it.
                        }
                    }
                }
            }
        }
    }

    if polls_total > 0 && polls_ok == 0 {
        return Err(first_error
            .unwrap_or_else(|| PaykitError::Transport("every drop channel poll failed".into())));
    }
    Ok(out)
}

/// The Paykit protocol message kinds routed over bonded Drop channels.
///
/// Requests and proposals vouch only for the live session; receipts (ACKs)
/// carry their own application-level signature inside the body and are
/// therefore declared [`Authenticity::ExternallyAuthenticated`] so the
/// evidence stays independently verifiable after session keys are gone
/// (S9: "receipts are ExternallyAuthenticated").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolMessageKind {
    /// A payment request (`/pub/paykit.app/v0/requests/…` payload).
    Request,
    /// A subscription proposal
    /// (`/pub/paykit.app/v0/subscriptions/proposals/…` payload).
    Proposal,
    /// An ACK / receipt (`/pub/paykit.app/v0/acks/…` payload).
    Ack,
}

impl ProtocolMessageKind {
    /// The authenticity mode declared for this message kind.
    pub fn authenticity(&self) -> Authenticity {
        match self {
            ProtocolMessageKind::Request | ProtocolMessageKind::Proposal => {
                Authenticity::SessionAuthenticated
            }
            ProtocolMessageKind::Ack => Authenticity::ExternallyAuthenticated,
        }
    }
}

/// Route an existing Paykit protocol message (request, proposal, or ACK)
/// through [`send_bonded`] with purpose `pubky.molt.paykit.v1`.
///
/// This is the bonded path used when a [`BondSession`] exists with the
/// counterparty; the public, root-anchored storage paths are untouched.
/// `body` is the already-serialized protocol payload (SB2 or ACK bytes);
/// Molt carries it unparsed.
///
/// # Errors
///
/// Same as [`send_bonded`].
pub async fn send_protocol_message<H: DropHttp>(
    s: &mut BondSession,
    kind: ProtocolMessageKind,
    body: &[u8],
    c: &DropClient<H>,
) -> Result<()> {
    let purpose = PurposeId::paykit();
    let env = MoltEnvelope {
        purpose: &purpose,
        authenticity: kind.authenticity(),
        body,
    };
    send_bonded(s, &env, c).await
}

/// Outbound route for a Paykit protocol message (request, proposal, ACK)
/// when a [`BondSession`] may exist with the counterparty (W2b).
///
/// Selection rule: the caller chooses the variant — bonded **if and only
/// if** it holds a [`BondSession`] for that peer. The bonded variant carries
/// no public transport and [`OutboundTransport::deliver`] never retries a
/// failed bonded send on the public path, so falling back from bonded to
/// public is structurally impossible (fail closed).
///
/// Neither variant holds the public transport: the two existing outbox
/// write interfaces in use (`HomeserverSessionStorage`,
/// `pubky::PubkySession`) differ, so on the public route the caller's own
/// unchanged write path runs (the `public_write` closure). See
/// `DECISIONS.md`.
pub enum OutboundTransport<'a, H: DropHttp> {
    /// The legacy public, root-anchored homeserver outbox
    /// (`/pub/paykit.app/v0/…`).
    PublicOutbox,
    /// A bonded Drop channel to the counterparty (purpose
    /// `pubky.molt.paykit.v1`).
    Bonded {
        /// The bonded session with the counterparty.
        session: &'a mut BondSession,
        /// The Drop relay client.
        client: &'a DropClient<H>,
    },
}

impl<'a, H: DropHttp> OutboundTransport<'a, H> {
    /// `true` when this route is bonded.
    pub fn is_bonded(&self) -> bool {
        matches!(self, Self::Bonded { .. })
    }

    /// Deliver an already-serialized protocol payload for `kind`.
    ///
    /// - Bonded: sealed with the session's next send key and appended to the
    ///   peer's Drop channel under purpose `pubky.molt.paykit.v1` via
    ///   [`send_protocol_message`]. Any error is returned and the public
    ///   outbox is never touched as a fallback.
    /// - Public: `public_write()` runs — the caller's existing outbox write,
    ///   unchanged (byte-identical to the pre-W2b behavior).
    ///
    /// # Errors
    ///
    /// Bonded route: `E::from(PaykitError)` from the seal/send. Public
    /// route: whatever `public_write` returns.
    pub async fn deliver<F, Fut, E>(
        &mut self,
        kind: ProtocolMessageKind,
        body: &[u8],
        public_write: F,
    ) -> std::result::Result<(), E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<(), E>>,
        E: From<PaykitError>,
    {
        match self {
            OutboundTransport::Bonded { session, client } => {
                send_protocol_message(session, kind, body, client).await?;
                Ok(())
            }
            OutboundTransport::PublicOutbox => public_write().await,
        }
    }
}

/// Reqwest-backed [`DropHttp`] for native targets (feature
/// `drop-transport`). Not available on WASM, matching the crate's existing
/// `http-executor` constraint.
#[cfg(feature = "drop-transport")]
#[derive(Debug, Clone, Default)]
pub struct ReqwestDropHttp {
    client: reqwest::Client,
}

#[cfg(feature = "drop-transport")]
impl ReqwestDropHttp {
    /// Create a backend with a default reqwest client.
    pub fn new() -> Self {
        ReqwestDropHttp {
            client: reqwest::Client::new(),
        }
    }
}

#[cfg(feature = "drop-transport")]
fn transport_err(context: &str, e: impl std::fmt::Display) -> PaykitError {
    PaykitError::Transport(format!("{context}: {e}"))
}

#[cfg(feature = "drop-transport")]
#[async_trait]
impl DropHttp for ReqwestDropHttp {
    async fn http_put(&self, url: &str, body: Vec<u8>) -> Result<u64> {
        let resp = self
            .client
            .put(url)
            .body(body)
            .send()
            .await
            .map_err(|e| transport_err("drop PUT", e))?;
        let status = resp.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(PaykitError::RateLimited {
                retry_after_ms: 1_000,
            });
        }
        if !status.is_success() {
            return Err(PaykitError::Transport(format!(
                "drop PUT rejected with status {status}"
            )));
        }
        let cursor = resp
            .headers()
            .get(reqwest::header::HeaderName::from_static("x-drop-cursor"))
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| {
                PaykitError::Serialization("drop PUT response missing X-Drop-Cursor".into())
            })?;
        Ok(cursor)
    }

    async fn http_get(&self, url: &str, max_response_bytes: usize) -> Result<Vec<u8>> {
        let mut resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| transport_err("drop GET", e))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(PaykitError::Transport(format!(
                "drop GET rejected with status {status}"
            )));
        }
        // Content-Length precheck: reject before reading a single byte when
        // the declared length already exceeds the cap.
        if let Some(len) = resp.content_length() {
            if len > max_response_bytes as u64 {
                return Err(PaykitError::Transport(format!(
                    "drop GET response declares {len} bytes (max {max_response_bytes})"
                )));
            }
        }
        // Streaming read capped at `max_response_bytes`: abort with an
        // explicit error as soon as the body grows past the cap instead of
        // buffering it whole first.
        let mut body = Vec::new();
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| transport_err("drop GET body", e))?
        {
            if body.len() + chunk.len() > max_response_bytes {
                return Err(PaykitError::Transport(format!(
                    "drop GET response exceeds {max_response_bytes} bytes"
                )));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }

    async fn http_delete(&self, url: &str) -> Result<()> {
        let resp = self
            .client
            .delete(url)
            .send()
            .await
            .map_err(|e| transport_err("drop DELETE", e))?;
        let status = resp.status();
        if status.is_success() || status == reqwest::StatusCode::NOT_FOUND {
            Ok(())
        } else {
            Err(PaykitError::Transport(format!(
                "drop DELETE rejected with status {status}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pubky_crypto::molt::{derive_bond, derive_pair_secret, pair_public, Direction, PairPublic};

    fn alice_bob() -> (PeerId, PeerId, Bond) {
        let alice = PeerId([0x01; 32]);
        let bob = PeerId([0x02; 32]);
        let sk_a = derive_pair_secret(&[0x11; 32], &bob).expect("pair secret");
        let pk_b = pair_public(&derive_pair_secret(&[0x22; 32], &alice).expect("pair secret"));
        let bond = derive_bond(&alice, &sk_a, &bob, &pk_b).expect("bond");
        (alice, bob, bond)
    }

    fn record(peer: PeerId) -> BondRecord {
        BondRecord {
            peer,
            pair_pk_peer: PairPublic([0u8; 32]),
            epoch_secs: 86_400,
            relays: vec![],
        }
    }

    #[test]
    fn drop_client_validates_base_url() {
        // Positive: trailing slashes are normalized.
        // (No network I/O happens in these tests; the backend is never
        // called for construction-only assertions.)
        struct Never;
        #[async_trait]
        impl DropHttp for Never {
            async fn http_put(&self, _url: &str, _body: Vec<u8>) -> Result<u64> {
                unreachable!()
            }
            async fn http_get(&self, _url: &str, _max_response_bytes: usize) -> Result<Vec<u8>> {
                unreachable!()
            }
            async fn http_delete(&self, _url: &str) -> Result<()> {
                unreachable!()
            }
        }
        let c = DropClient::new("https://relay.example/", Never).expect("valid https");
        assert_eq!(c.base_url(), "https://relay.example");
        for bad in ["", "ftp://relay.example", "relay.example"] {
            assert!(DropClient::new(bad, Never).is_err(), "accepted {bad:?}");
        }
    }

    fn text_key_message(cursor: u64, ts: u64, body: &[u8]) -> serde_cbor::Value {
        serde_cbor::Value::Map(
            [
                (
                    serde_cbor::Value::Text("cursor".into()),
                    serde_cbor::Value::Integer(cursor as i128),
                ),
                (
                    serde_cbor::Value::Text("ts".into()),
                    serde_cbor::Value::Integer(ts as i128),
                ),
                (
                    serde_cbor::Value::Text("body".into()),
                    serde_cbor::Value::Bytes(body.to_vec()),
                ),
            ]
            .into_iter()
            .collect(),
        )
    }

    #[test]
    fn poll_response_decodes_text_and_integer_keys() {
        // Text keys: the deployed http-relay form (serde_cbor of structs).
        let text_form = serde_cbor::to_vec(&serde_cbor::Value::Array(vec![text_key_message(
            7,
            1_700_000_000,
            &[1, 2, 3],
        )]))
        .expect("encode");
        let msgs = decode_poll_response(&text_form).expect("text-key decode");
        assert_eq!(
            msgs,
            vec![DropMessage {
                cursor: 7,
                ts: 1_700_000_000,
                body: vec![1, 2, 3]
            }]
        );

        // Integer keys: the spec's stated form {0: cursor, 1: ts, 2: body}.
        let int_form = serde_cbor::to_vec(&serde_cbor::Value::Array(vec![serde_cbor::Value::Map(
            [
                (
                    serde_cbor::Value::Integer(0.into()),
                    serde_cbor::Value::Integer(9.into()),
                ),
                (
                    serde_cbor::Value::Integer(1.into()),
                    serde_cbor::Value::Integer(42.into()),
                ),
                (
                    serde_cbor::Value::Integer(2.into()),
                    serde_cbor::Value::Bytes(vec![0xaa]),
                ),
            ]
            .into_iter()
            .collect(),
        )]))
        .expect("encode");
        let msgs = decode_poll_response(&int_form).expect("integer-key decode");
        assert_eq!(
            msgs,
            vec![DropMessage {
                cursor: 9,
                ts: 42,
                body: vec![0xaa]
            }]
        );

        // Empty array is a valid empty poll.
        let empty = serde_cbor::to_vec(&serde_cbor::Value::Array(vec![])).expect("enc");
        assert_eq!(decode_poll_response(&empty).expect("empty"), vec![]);
    }

    #[test]
    fn poll_response_rejects_malformed_and_oversized() {
        assert!(decode_poll_response(b"not cbor at all").is_err());
        // Not an array.
        let map = serde_cbor::to_vec(&serde_cbor::Value::Map(Default::default())).expect("enc");
        assert!(decode_poll_response(&map).is_err());
        // Missing fields.
        let incomplete =
            serde_cbor::to_vec(&serde_cbor::Value::Array(vec![serde_cbor::Value::Map(
                [(
                    serde_cbor::Value::Text("cursor".into()),
                    serde_cbor::Value::Integer(1.into()),
                )]
                .into_iter()
                .collect(),
            )]))
            .expect("enc");
        assert!(decode_poll_response(&incomplete).is_err());
        // Wrong type for a known key.
        let wrong_type =
            serde_cbor::to_vec(&serde_cbor::Value::Array(vec![serde_cbor::Value::Map(
                [
                    (
                        serde_cbor::Value::Text("cursor".into()),
                        serde_cbor::Value::Text("seven".into()),
                    ),
                    (
                        serde_cbor::Value::Text("ts".into()),
                        serde_cbor::Value::Integer(1.into()),
                    ),
                    (
                        serde_cbor::Value::Text("body".into()),
                        serde_cbor::Value::Bytes(vec![0]),
                    ),
                ]
                .into_iter()
                .collect(),
            )]))
            .expect("enc");
        assert!(decode_poll_response(&wrong_type).is_err());
        // More than MAX_DROP_POLL_LIMIT entries.
        let many = serde_cbor::Value::Array((0..51).map(|i| text_key_message(i, 0, &[])).collect());
        let many = serde_cbor::to_vec(&many).expect("enc");
        assert!(decode_poll_response(&many).is_err());
        // Trailing bytes after the top-level array.
        let mut trailing = serde_cbor::to_vec(&serde_cbor::Value::Array(vec![])).expect("enc");
        trailing.push(0x00);
        assert!(decode_poll_response(&trailing).is_err());
    }

    /// Hand-encoded CBOR: `[{0: cursor, 1: ts, 2: h'body'}]` with optional
    /// extra map entries appended (used to craft duplicate keys, which
    /// `serde_cbor::Value::Map` cannot represent).
    fn cbor_int_message(extra_entries: &[(u8, u8)]) -> Vec<u8> {
        let map_len = 3 + extra_entries.len() as u8;
        let mut w = vec![
            0x81,
            0xa0 | map_len,
            0x00,
            0x07,
            0x01,
            0x18,
            0x2a,
            0x02,
            0x43,
            0xaa,
            0xbb,
            0xcc,
        ];
        for (k, v) in extra_entries {
            w.extend_from_slice(&[*k, *v]);
        }
        w
    }

    #[test]
    fn poll_response_rejects_duplicate_canonical_fields() {
        // Positive: the plain integer-keyed form decodes.
        let ok = cbor_int_message(&[]);
        let msgs = decode_poll_response(&ok).expect("plain decode");
        assert_eq!(
            msgs,
            vec![DropMessage {
                cursor: 7,
                ts: 42,
                body: vec![0xaa, 0xbb, 0xcc]
            }]
        );

        // Duplicate integer key 0 (cursor): map(4) = {0:7, 1:42, 2:bytes, 0:9}.
        let dup_int = cbor_int_message(&[(0x00, 0x09)]);
        let err = decode_poll_response(&dup_int).expect_err("duplicate int key");
        assert!(
            matches!(err, PaykitError::Serialization(ref m) if m.contains("duplicate")),
            "unexpected error: {err}"
        );

        // Mixed-spelling duplicate: integer key 0 then text key "cursor" —
        // the text fallback must not override the integer form.
        // map(4) = {0:7, 1:42, 2:bytes, "cursor":9}.
        let mut mixed = cbor_int_message(&[]);
        mixed[1] = 0xa4;
        mixed.extend_from_slice(b"\x66cursor\x09");
        let err = decode_poll_response(&mixed).expect_err("mixed-spelling duplicate");
        assert!(
            matches!(err, PaykitError::Serialization(ref m) if m.contains("duplicate")),
            "unexpected error: {err}"
        );

        // Text-only duplicate: {"cursor":7, "cursor":9, "ts":42, "body":bytes}.
        let text_dup = vec![
            0x81, 0xa4, 0x66, b'c', b'u', b'r', b's', b'o', b'r', 0x07, 0x66, b'c', b'u', b'r',
            b's', b'o', b'r', 0x09, 0x62, b't', b's', 0x18, 0x2a, 0x64, b'b', b'o', b'd', b'y',
            0x41, 0xff,
        ];
        assert!(decode_poll_response(&text_dup).is_err());
    }

    #[test]
    fn poll_response_text_fallback_cannot_bypass_checks() {
        // The text-key fallback decodes to exactly the same message as the
        // integer-keyed primary form...
        let text_form = serde_cbor::to_vec(&serde_cbor::Value::Array(vec![text_key_message(
            7,
            42,
            &[0xaa, 0xbb, 0xcc],
        )]))
        .expect("encode");
        let int_form = cbor_int_message(&[]);
        assert_eq!(
            decode_poll_response(&text_form).expect("text"),
            decode_poll_response(&int_form).expect("int"),
            "key spelling must not change the decoded value"
        );

        // ...and every check applies identically through the fallback: a
        // wrong-typed text field is rejected just like the integer form.
        let wrong_type_int = {
            // map(3) = {0:7, 1:42, 2:"not-bytes"}
            vec![
                0x81, 0xa3, 0x00, 0x07, 0x01, 0x18, 0x2a, 0x02, 0x69, b'n', b'o', b't', b'-', b'b',
                b'y', b't', b'e', b's',
            ]
        };
        assert!(decode_poll_response(&wrong_type_int).is_err());
    }

    #[test]
    fn bond_session_channels_match_between_peers() {
        let (alice, bob, bond_a) = alice_bob();
        // Bob derives the same bond from his side.
        let sk_b = derive_pair_secret(&[0x22; 32], &alice).expect("pair secret");
        let pk_a = pair_public(&derive_pair_secret(&[0x11; 32], &bob).expect("pair secret"));
        let bond_b = derive_bond(&bob, &sk_b, &alice, &pk_a).expect("bond");
        assert_eq!(bond_a.as_bytes(), bond_b.as_bytes());

        let sa = BondSession::new(&alice, bob, bond_a, record(bob));
        let sb = BondSession::new(&bob, alice, bond_b, record(alice));
        assert_eq!(sa.send.direction(), Direction::LoToHi);
        assert_eq!(sb.send.direction(), Direction::HiToLo);

        let purpose = PurposeId::paykit();
        for e in [0u32, 19_999, 20_000] {
            // Alice's send channel is Bob's receive channel, and vice versa.
            assert_eq!(sa.send_channel(&purpose, e), sb.recv_channel(&purpose, e));
            assert_eq!(sa.recv_channel(&purpose, e), sb.send_channel(&purpose, e));
            // Directions and epochs give distinct channels.
            assert_ne!(sa.send_channel(&purpose, e), sa.recv_channel(&purpose, e));
        }
        assert_ne!(sa.send_channel(&purpose, 0), sa.send_channel(&purpose, 1));
    }

    #[test]
    fn poll_epochs_returns_e_minus_1_e_e_plus_1() {
        let (_, bob, bond) = alice_bob();
        let alice = PeerId([0x01; 32]);
        let s = BondSession::new(&alice, bob, bond, record(bob));
        // Day boundary: unix 86_400 * 20_000 is epoch 20_000.
        let epochs = s.poll_epochs(86_400 * 20_000).expect("epochs");
        assert_eq!(epochs, vec![20_000, 19_999, 20_001]);
        // Epoch 0 has no predecessor (deduplicated).
        let epochs0 = s.poll_epochs(0).expect("epochs");
        assert_eq!(epochs0, vec![0, 1]);
        // Zero epoch length is rejected.
        let mut bad = record(bob);
        bad.epoch_secs = 0;
        let sbad = BondSession::new(
            &alice,
            bob,
            derive_bond(
                &alice,
                &derive_pair_secret(&[0x11; 32], &bob).expect("pair secret"),
                &bob,
                &pair_public(&derive_pair_secret(&[0x22; 32], &alice).expect("pair secret")),
            )
            .expect("bond"),
            bad,
        );
        assert!(sbad.poll_epochs(1_700_000_000).is_err());
    }

    #[test]
    fn protocol_message_kind_authenticity() {
        assert_eq!(
            ProtocolMessageKind::Request.authenticity(),
            Authenticity::SessionAuthenticated
        );
        assert_eq!(
            ProtocolMessageKind::Proposal.authenticity(),
            Authenticity::SessionAuthenticated
        );
        assert_eq!(
            ProtocolMessageKind::Ack.authenticity(),
            Authenticity::ExternallyAuthenticated
        );
    }

    #[test]
    fn drop_wire_size_pins_overhead_computation() {
        // The itemized overhead (see the constant's doc table) must sum to
        // the relay-bound derivation exactly.
        assert_eq!(DROP_SEAL_OVERHEAD_BYTES, 244);
        assert_eq!(MAX_DROP_BODY_BYTES, 65_292);
        assert_eq!(MAX_DROP_BODY_BYTES + DROP_SEAL_OVERHEAD_BYTES, 65_536);

        let paykit = PurposeId::paykit();
        // A maximal body fits the relay bound exactly; one byte more does
        // not — this is the boundary `send_bonded` enforces.
        assert_eq!(
            drop_wire_size(&paykit, MAX_DROP_BODY_BYTES),
            MAX_DROP_WIRE_BYTES
        );
        assert_eq!(
            drop_wire_size(&paykit, MAX_DROP_BODY_BYTES + 1),
            MAX_DROP_WIRE_BYTES + 1
        );
        // Longer purposes shrink the body allowance byte for byte (plus a
        // CBOR string-header byte at the 24-char threshold): this 32-char
        // purpose costs a 35-byte entry vs the paykit purpose's 22.
        let long_purpose = PurposeId::parse("pubky.molt.abcdefghijklmnopqr.v1").expect("purpose");
        assert_eq!(long_purpose.as_str().len(), 32);
        assert_eq!(
            drop_wire_size(&long_purpose, MAX_DROP_BODY_BYTES),
            MAX_DROP_WIRE_BYTES + 13
        );
    }

    #[test]
    fn drop_wire_size_bounds_actual_seal() {
        let (_alice, _bob, bond) = alice_bob();
        let purpose = PurposeId::paykit();
        let mut ratchet = RatchetState::bootstrap(&bond, Direction::LoToHi);

        for body_len in [0usize, 1, 1024, MAX_DROP_BODY_BYTES] {
            let body = vec![0xabu8; body_len];
            let (n, mk) = ratchet.next_send();
            let hdr = Header {
                dir: ratchet.direction(),
                n,
                purpose: purpose.clone(),
                authenticity: Authenticity::SessionAuthenticated,
            };
            let env = MoltEnvelope {
                purpose: &purpose,
                authenticity: Authenticity::SessionAuthenticated,
                body: &body,
            };
            let wire = molt::seal(&env, &mk, &hdr, &DROP_INBOX_KID).expect("seal");
            // The computation is an upper bound for every ratchet index...
            assert!(
                wire.len() <= drop_wire_size(&purpose, body_len),
                "wire {} exceeds computed bound {}",
                wire.len(),
                drop_wire_size(&purpose, body_len)
            );
            // ...and exact up to the worst-case `n` varint it charges: the
            // first indices encode in one value byte, not nine.
            assert_eq!(wire.len(), drop_wire_size(&purpose, body_len) - 8);
            // The maximal body stays within the relay's PUT bound.
            assert!(wire.len() <= MAX_DROP_WIRE_BYTES);
        }
    }
}
