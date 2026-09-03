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

/// Maximum Drop message body (S8: SB2 ≤ 64 KiB).
pub const MAX_DROP_BODY_BYTES: usize = 64 * 1024;

/// Maximum poll page size (S8: `limit ≤ 50`).
pub const MAX_DROP_POLL_LIMIT: u32 = 50;

/// Defensive upper bound on a poll response body
/// ([`MAX_DROP_POLL_LIMIT`] messages of [`MAX_DROP_BODY_BYTES`] plus
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
    async fn http_get(&self, url: &str) -> Result<Vec<u8>>;

    /// `DELETE` `url`. Success or already-absent both count as `Ok`.
    async fn http_delete(&self, url: &str) -> Result<()>;
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
    /// - [`PaykitError::QuotaExceeded`]: body larger than
    ///   [`MAX_DROP_BODY_BYTES`] (rejected client-side, never sent).
    /// - [`PaykitError::Transport`]: the relay rejected or was unreachable.
    pub async fn put(&self, channel: &ChannelId, body: &[u8]) -> Result<u64> {
        if body.len() > MAX_DROP_BODY_BYTES {
            return Err(PaykitError::QuotaExceeded {
                used: body.len() as u64,
                limit: MAX_DROP_BODY_BYTES as u64,
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
        let bytes = self.http.http_get(&url).await?;
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
/// The deployed `http-relay` encodes text keys via `serde_cbor` (see the
/// wave-1 review flag in plan v11); the spec's stated form uses integer keys
/// `{0: cursor, 1: ts, 2: body}`. Both are accepted so the client tracks
/// either relay revision (see `DECISIONS.md`).
fn decode_poll_response(bytes: &[u8]) -> Result<Vec<DropMessage>> {
    let value: serde_cbor::Value = serde_cbor::from_slice(bytes)
        .map_err(|e| PaykitError::Serialization(format!("invalid drop poll CBOR: {e}")))?;
    let items = match value {
        serde_cbor::Value::Array(items) => items,
        _ => {
            return Err(PaykitError::Serialization(
                "drop poll response is not an array".into(),
            ))
        }
    };
    if items.len() > MAX_DROP_POLL_LIMIT as usize {
        return Err(PaykitError::Serialization(format!(
            "drop poll response has {} entries (max {MAX_DROP_POLL_LIMIT})",
            items.len()
        )));
    }
    items.iter().map(decode_drop_message).collect()
}

fn decode_drop_message(value: &serde_cbor::Value) -> Result<DropMessage> {
    let entries = match value {
        serde_cbor::Value::Map(entries) => entries,
        _ => {
            return Err(PaykitError::Serialization(
                "drop message is not a map".into(),
            ))
        }
    };
    let mut cursor = None;
    let mut ts = None;
    let mut body = None;
    for (key, val) in entries {
        let field = match key {
            serde_cbor::Value::Text(t) => t.as_str(),
            serde_cbor::Value::Integer(i) => match *i {
                0 => "cursor",
                1 => "ts",
                2 => "body",
                _ => continue,
            },
            _ => continue,
        };
        match (field, val) {
            ("cursor", serde_cbor::Value::Integer(i)) => {
                cursor =
                    Some(u64::try_from(*i).map_err(|_| {
                        PaykitError::Serialization("drop cursor out of range".into())
                    })?);
            }
            ("ts", serde_cbor::Value::Integer(i)) => {
                ts = Some(u64::try_from(*i).map_err(|_| {
                    PaykitError::Serialization("drop timestamp out of range".into())
                })?);
            }
            ("body", serde_cbor::Value::Bytes(b)) => {
                body = Some(b.clone());
            }
            // Unknown key or mismatched type: ignore unknown keys, but a
            // known key with the wrong type makes the entry unusable.
            ("cursor" | "ts" | "body", _) => {
                return Err(PaykitError::Serialization(format!(
                    "drop message field {field} has unexpected type"
                )))
            }
            _ => {}
        }
    }
    match (cursor, ts, body) {
        (Some(cursor), Some(ts), Some(body)) => Ok(DropMessage { cursor, ts, body }),
        _ => Err(PaykitError::Serialization(
            "drop message missing cursor/ts/body".into(),
        )),
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

/// Seal `env` with the session's next send-side message key and append it to
/// the current epoch's Drop channel (S9).
///
/// The header `{dir, n, purpose, authenticity}` is bound in the envelope
/// AAD by [`pubky_crypto::molt::seal`]. The ratchet index is consumed even
/// if the relay append fails; the caller may retry with a fresh envelope
/// (re-sends use a fresh index, and receivers reject replays).
///
/// # Errors
///
/// - [`PaykitError::Crypto`]: sealing or epoch derivation failed.
/// - [`PaykitError::QuotaExceeded`]: the sealed body exceeds the S8 bound.
/// - [`PaykitError::Transport`]: the relay append failed.
pub async fn send_bonded<H: DropHttp>(
    s: &mut BondSession,
    env: &MoltEnvelope<'_>,
    c: &DropClient<H>,
) -> Result<()> {
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
/// the authenticity is exactly the AAD-bound declaration. Successfully
/// opened messages are ack-deleted; messages that fail to open are skipped
/// and left on the relay for TTL expiry (they may belong to a post-mix
/// future the receiver has not scheduled yet). Ack failures are tolerated:
/// a redelivered duplicate is rejected by ratchet replay protection.
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
                    match molt::open(&msg.body, &mut s.recv) {
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

    async fn http_get(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self
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
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| transport_err("drop GET body", e))?;
        Ok(bytes.to_vec())
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
        let sk_a = derive_pair_secret(&[0x11; 32], &bob);
        let pk_b = pair_public(&derive_pair_secret(&[0x22; 32], &alice));
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
            async fn http_get(&self, _url: &str) -> Result<Vec<u8>> {
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
    }

    #[test]
    fn bond_session_channels_match_between_peers() {
        let (alice, bob, bond_a) = alice_bob();
        // Bob derives the same bond from his side.
        let sk_b = derive_pair_secret(&[0x22; 32], &alice);
        let pk_a = pair_public(&derive_pair_secret(&[0x11; 32], &bob));
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
                &derive_pair_secret(&[0x11; 32], &bob),
                &bob,
                &pair_public(&derive_pair_secret(&[0x22; 32], &alice)),
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
}
