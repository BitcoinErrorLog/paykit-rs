//! Bonded send/receive round trip against an in-process mock of the three S8
//! Drop endpoints (`PUT`/`GET`/`DELETE /drop/…`). This stub is test-only and
//! lives outside `src/` per the wave-2 rules.

use async_trait::async_trait;
use paykit_lib::protocol::drop_transport::{
    receive_bonded, send_bonded, send_protocol_message, BondSession, DropClient, DropHttp,
    ProtocolMessageKind, MAX_DROP_BODY_BYTES, MAX_POLL_RESPONSE_BYTES,
};
use paykit_lib::{PaykitError, Result};
use pubky_crypto::molt::{
    derive_bond, derive_pair_secret, pair_public, Authenticity, Bond, BondRecord, MoltEnvelope,
    PairPublic, PeerId, PurposeId,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// One stored message: (cursor, timestamp, body).
type StoredMessage = (u64, u64, Vec<u8>);

/// In-process mock of the S8 relay: real URL parsing, real CBOR encoding,
/// in-memory bounded storage.
struct StubRelay {
    channels: Mutex<HashMap<String, Vec<StoredMessage>>>,
    next_cursor: AtomicU64,
    fail_writes: AtomicBool,
    fail_reads: AtomicBool,
    oversize_responses: AtomicBool,
}

impl StubRelay {
    fn new() -> Self {
        StubRelay {
            channels: Mutex::new(HashMap::new()),
            next_cursor: AtomicU64::new(1),
            fail_writes: AtomicBool::new(false),
            fail_reads: AtomicBool::new(false),
            oversize_responses: AtomicBool::new(false),
        }
    }

    fn channel_key(url: &str) -> Result<String> {
        let path = url
            .split("/drop/")
            .nth(1)
            .ok_or_else(|| PaykitError::InvalidData {
                field: "url".into(),
                reason: "missing /drop/ prefix".into(),
            })?;
        let channel = path.split(['?', '/']).next().unwrap_or("");
        // S8: the id must decode to exactly 32 bytes.
        use base64::Engine;
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(channel)
            .map_err(|_| PaykitError::InvalidData {
                field: "channel".into(),
                reason: "invalid base64url".into(),
            })?;
        if decoded.len() != 32 {
            return Err(PaykitError::InvalidData {
                field: "channel".into(),
                reason: "must decode to 32 bytes".into(),
            });
        }
        Ok(channel.to_string())
    }

    fn message_count(&self) -> usize {
        self.channels
            .lock()
            .expect("lock")
            .values()
            .map(Vec::len)
            .sum()
    }

    /// The relay-side key for a channel id (base64url, no padding).
    fn key_of(channel: &pubky_crypto::molt::ChannelId) -> String {
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(channel.0)
    }

    /// Copy the newest message from one channel to another, as a malicious
    /// relay could (cross-channel copy with a fresh cursor).
    fn copy_newest(
        &self,
        from: &pubky_crypto::molt::ChannelId,
        to: &pubky_crypto::molt::ChannelId,
    ) {
        let (from_key, to_key) = (Self::key_of(from), Self::key_of(to));
        let mut channels = self.channels.lock().expect("lock");
        let (_, ts, body) = channels
            .get(&from_key)
            .and_then(|msgs| msgs.last())
            .expect("source channel has a message")
            .clone();
        let cursor = self.next_cursor.fetch_add(1, Ordering::SeqCst);
        channels.entry(to_key).or_default().push((cursor, ts, body));
    }
}

#[async_trait]
impl DropHttp for StubRelay {
    async fn http_put(&self, url: &str, body: Vec<u8>) -> Result<u64> {
        if self.fail_writes.load(Ordering::SeqCst) {
            return Err(PaykitError::Transport("relay write failure".into()));
        }
        let key = Self::channel_key(url)?;
        let cursor = self.next_cursor.fetch_add(1, Ordering::SeqCst);
        let ts = 1_700_000_000u64;
        self.channels
            .lock()
            .expect("lock")
            .entry(key)
            .or_default()
            .push((cursor, ts, body));
        Ok(cursor)
    }

    async fn http_get(&self, url: &str, _max_response_bytes: usize) -> Result<Vec<u8>> {
        if self.fail_reads.load(Ordering::SeqCst) {
            return Err(PaykitError::Transport("relay read failure".into()));
        }
        if self.oversize_responses.load(Ordering::SeqCst) {
            // A misbehaving (or hostile) backend ignoring the cap: the
            // client's own post-read bound must still reject the body.
            return Ok(vec![0u8; MAX_POLL_RESPONSE_BYTES + 1]);
        }
        let key = Self::channel_key(url)?;
        let messages = self
            .channels
            .lock()
            .expect("lock")
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let items: Vec<serde_cbor::Value> = messages
            .into_iter()
            .map(|(cursor, ts, body)| {
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
                            serde_cbor::Value::Bytes(body),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                )
            })
            .collect();
        serde_cbor::to_vec(&serde_cbor::Value::Array(items))
            .map_err(|e| PaykitError::Serialization(e.to_string()))
    }

    async fn http_delete(&self, url: &str) -> Result<()> {
        let key = Self::channel_key(url)?;
        let cursor_str = url.rsplit('/').next().unwrap_or("");
        let cursor: u64 = cursor_str.parse().map_err(|_| PaykitError::InvalidData {
            field: "cursor".into(),
            reason: "must be an unsigned integer".into(),
        })?;
        let mut channels = self.channels.lock().expect("lock");
        if let Some(messages) = channels.get_mut(&key) {
            messages.retain(|(c, _, _)| *c != cursor);
        }
        // Already-absent is not an error (S8 DELETE semantics tolerated).
        Ok(())
    }
}

fn alice_bob_sessions() -> (PeerId, PeerId, BondSession, BondSession) {
    let alice = PeerId([0x01; 32]);
    let bob = PeerId([0x02; 32]);
    let sk_a = derive_pair_secret(&[0x11; 32], &bob);
    let sk_b = derive_pair_secret(&[0x22; 32], &alice);
    let pk_a = pair_public(&sk_a);
    let pk_b = pair_public(&sk_b);
    let bond_a: Bond = derive_bond(&alice, &sk_a, &bob, &pk_b).expect("bond");
    let bond_b: Bond = derive_bond(&bob, &sk_b, &alice, &pk_a).expect("bond");
    let record = |peer: PeerId, pair_pk_peer: PairPublic| BondRecord {
        peer,
        pair_pk_peer,
        epoch_secs: 86_400,
        relays: vec!["http://relay.test".into()],
    };
    (
        alice,
        bob,
        BondSession::new(&alice, bob, bond_a, record(bob, pk_b)),
        BondSession::new(&bob, alice, bond_b, record(alice, pk_a)),
    )
}

fn client(relay: StubRelay) -> DropClient<StubRelay> {
    DropClient::new("http://relay.test", relay).expect("client")
}

#[tokio::test]
async fn bonded_request_ack_round_trip() {
    let (alice, bob, mut sa, mut sb) = alice_bob_sessions();
    let c = client(StubRelay::new());

    // Alice sends a payment request to Bob over the bonded channel.
    let request_body = b"paykit-request-v0-bytes".to_vec();
    send_protocol_message(&mut sa, ProtocolMessageKind::Request, &request_body, &c)
        .await
        .expect("send request");

    // Bob polls his receive channels and opens it.
    let received = receive_bonded(std::slice::from_mut(&mut sb), &[PurposeId::paykit()], &c)
        .await
        .expect("receive request");
    assert_eq!(received.len(), 1);
    let (peer, hdr, authenticity, body) = &received[0];
    assert_eq!(peer, &alice);
    assert_eq!(hdr.n, 0);
    assert_eq!(hdr.purpose, PurposeId::paykit());
    assert_eq!(authenticity, &Authenticity::SessionAuthenticated);
    assert_eq!(body, &request_body);

    // The opened message was ack-deleted: a second poll is empty.
    let again = receive_bonded(std::slice::from_mut(&mut sb), &[PurposeId::paykit()], &c)
        .await
        .expect("second poll");
    assert!(again.is_empty());

    // Bob replies with a receipt (ACK), ExternallyAuthenticated.
    let ack_body = b"paykit-ack-signed-bytes".to_vec();
    send_protocol_message(&mut sb, ProtocolMessageKind::Ack, &ack_body, &c)
        .await
        .expect("send ack");
    let received = receive_bonded(std::slice::from_mut(&mut sa), &[PurposeId::paykit()], &c)
        .await
        .expect("receive ack");
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].2, Authenticity::ExternallyAuthenticated);
    assert_eq!(received[0].3, ack_body);
    assert_eq!(received[0].0, bob);
}

#[tokio::test]
async fn bonded_multiple_messages_arrive_in_order() {
    let (alice, _bob, mut sa, mut sb) = alice_bob_sessions();
    let c = client(StubRelay::new());
    let purpose = PurposeId::paykit();

    for i in 0..3u8 {
        let body = vec![0xa0 + i];
        let env = MoltEnvelope {
            purpose: &purpose,
            authenticity: Authenticity::SessionAuthenticated,
            body: &body,
        };
        send_bonded(&mut sa, &env, &c).await.expect("send");
    }

    let received = receive_bonded(std::slice::from_mut(&mut sb), &[PurposeId::paykit()], &c)
        .await
        .expect("receive");
    assert_eq!(received.len(), 3);
    for (i, (peer, hdr, _, body)) in received.iter().enumerate() {
        assert_eq!(peer, &alice);
        assert_eq!(hdr.n, i as u64);
        assert_eq!(body, &vec![0xa0 + i as u8]);
    }
}

#[tokio::test]
async fn send_bonded_propagates_relay_failure() {
    let (_alice, _bob, mut sa, _sb) = alice_bob_sessions();
    let relay = StubRelay::new();
    relay.fail_writes.store(true, Ordering::SeqCst);
    let c = client(relay);
    let purpose = PurposeId::paykit();
    let env = MoltEnvelope {
        purpose: &purpose,
        authenticity: Authenticity::SessionAuthenticated,
        body: b"x",
    };
    let err = send_bonded(&mut sa, &env, &c).await.expect_err("must fail");
    assert!(matches!(err, PaykitError::Transport(_)));
}

#[tokio::test]
async fn receive_bonded_errors_when_every_poll_fails() {
    let (_alice, _bob, _sa, mut sb) = alice_bob_sessions();
    let relay = StubRelay::new();
    relay.fail_reads.store(true, Ordering::SeqCst);
    let c = client(relay);
    let err = receive_bonded(std::slice::from_mut(&mut sb), &[PurposeId::paykit()], &c)
        .await
        .expect_err("all polls failed");
    assert!(matches!(err, PaykitError::Transport(_)));

    // No sessions at all is not an error: an empty poll set yields nothing.
    let c2 = client(StubRelay::new());
    let none = receive_bonded(&mut [], &[PurposeId::paykit()], &c2)
        .await
        .expect("empty sessions");
    assert!(none.is_empty());
}

#[tokio::test]
async fn receive_bonded_skips_unauthenticatable_messages() {
    let (_alice, _bob, mut sa, mut sb) = alice_bob_sessions();
    let c = client(StubRelay::new());
    let purpose = PurposeId::paykit();

    // A valid message, then a tampered one (sealed under the wrong ratchet
    // direction), then another valid one.
    let env = |body: &'static [u8]| MoltEnvelope {
        purpose: &purpose,
        authenticity: Authenticity::SessionAuthenticated,
        body,
    };
    send_bonded(&mut sa, &env(b"first"), &c)
        .await
        .expect("send 1");

    // Forge a message onto Bob's receive channel using a *wrong-direction*
    // ratchet (an attacker or a corrupt relay can do no better).
    let mut forged = pubky_crypto::molt::RatchetState::bootstrap(
        &sa.bond,
        pubky_crypto::molt::Direction::HiToLo,
    );
    let (n, mk) = forged.next_send();
    let hdr = pubky_crypto::molt::Header {
        dir: forged.direction(),
        n,
        purpose: purpose.clone(),
        authenticity: Authenticity::SessionAuthenticated,
    };
    let wire = pubky_crypto::molt::seal(&env(b"forged"), &mk, &hdr, &[0u8; 16]).expect("seal");
    let epoch = sb.poll_epochs(now()).expect("epochs")[0];
    let channel = sb.recv_channel(&purpose, epoch);
    c.put(&channel, &wire).await.expect("forge");

    send_bonded(&mut sa, &env(b"second"), &c)
        .await
        .expect("send 2");

    let received = receive_bonded(std::slice::from_mut(&mut sb), &[PurposeId::paykit()], &c)
        .await
        .expect("receive");
    let bodies: Vec<&[u8]> = received.iter().map(|r| r.3.as_slice()).collect();
    assert_eq!(bodies, vec![b"first".as_slice(), b"second".as_slice()]);
    // The forged message was skipped without being acked (left for TTL).
    assert_eq!(c.backend().message_count(), 1);
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs()
}

#[tokio::test]
async fn put_rejects_oversized_body_client_side() {
    let c = client(StubRelay::new());
    let channel = pubky_crypto::molt::ChannelId([7u8; 32]);
    let big = vec![0u8; MAX_DROP_BODY_BYTES + 1];
    let err = c.put(&channel, &big).await.expect_err("oversized");
    assert!(matches!(err, PaykitError::QuotaExceeded { .. }));
}

#[tokio::test]
async fn receive_bonded_rejects_cross_channel_copy_without_consuming_ratchet_state() {
    let (alice, _bob, mut sa, mut sb) = alice_bob_sessions();
    let c = client(StubRelay::new());
    let paykit = PurposeId::paykit();
    let hello = PurposeId::hello();

    // Alice seals a genuine PAYKIT request; it lands on Bob's PAYKIT recv
    // channel.
    send_protocol_message(
        &mut sa,
        ProtocolMessageKind::Request,
        b"genuine-request",
        &c,
    )
    .await
    .expect("send");

    // A relay copies the sealed message onto Bob's HELLO channel (same
    // receive direction, different purpose).
    let epoch = sb.poll_epochs(now()).expect("epochs")[0];
    let paykit_channel = sb.recv_channel(&paykit, epoch);
    let hello_channel = sb.recv_channel(&hello, epoch);
    c.backend().copy_newest(&paykit_channel, &hello_channel);
    assert_eq!(c.backend().message_count(), 2);

    // Polling the HELLO channel rejects the copy with PurposeMismatch inside
    // `open` — before any ratchet state is consumed.
    let state_before = format!("{:?}", sb.recv);
    let got = receive_bonded(
        std::slice::from_mut(&mut sb),
        std::slice::from_ref(&hello),
        &c,
    )
    .await
    .expect("hello poll");
    assert!(got.is_empty(), "cross-channel copy must not be delivered");
    assert_eq!(
        state_before,
        format!("{:?}", sb.recv),
        "rejection must leave the ratchet untouched"
    );
    assert_eq!(sb.recv.next_index(), 0);
    assert_eq!(sb.recv.skipped_len(), 0);
    // And it must not be ack-deleted: an unopened message stays on the relay.
    assert_eq!(c.backend().message_count(), 2);

    // The genuine delivery on the correct channel still opens at the same
    // ratchet index (no replay rejection).
    let got = receive_bonded(std::slice::from_mut(&mut sb), &[paykit], &c)
        .await
        .expect("paykit poll");
    assert_eq!(got.len(), 1);
    let (peer, hdr, authenticity, body) = &got[0];
    assert_eq!(peer, &alice);
    assert_eq!(hdr.n, 0);
    assert_eq!(hdr.purpose, PurposeId::paykit());
    assert_eq!(authenticity, &Authenticity::SessionAuthenticated);
    assert_eq!(body, b"genuine-request");
    assert_eq!(sb.recv.next_index(), 1);
}

#[tokio::test]
async fn poll_rejects_oversize_response_body() {
    let (_alice, _bob, _sa, mut sb) = alice_bob_sessions();
    let relay = StubRelay::new();
    relay.oversize_responses.store(true, Ordering::SeqCst);
    let c = client(relay);
    let err = receive_bonded(std::slice::from_mut(&mut sb), &[PurposeId::paykit()], &c)
        .await
        .expect_err("oversize body must be rejected");
    assert!(
        matches!(err, PaykitError::Serialization(ref m) if m.contains("size bound")),
        "unexpected error: {err}"
    );
    // No ratchet state was consumed by the rejected response.
    assert_eq!(sb.recv.next_index(), 0);
    assert_eq!(sb.recv.skipped_len(), 0);
}
