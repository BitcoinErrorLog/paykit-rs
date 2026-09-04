//! W2b: ACK storage routed through `OutboundTransport` — bonded Drop channel
//! when a `BondSession` exists, the caller's unchanged public-outbox write
//! otherwise, and no silent fallback from bonded to public.

use async_trait::async_trait;
use paykit_lib::protocol::drop_transport::{
    receive_bonded, BondSession, DropClient, DropHttp, OutboundTransport, ProtocolMessageKind,
};
use paykit_lib::protocol::store_encrypted_ack;
use paykit_lib::{PaykitError, Result};
use pubky_crypto::molt::{
    derive_bond, derive_pair_secret, pair_public, Authenticity, Bond, BondRecord, PairPublic,
    PeerId, PurposeId,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// One stored message: (cursor, timestamp, body).
type StoredMessage = (u64, u64, Vec<u8>);

/// In-process mock of the S8 relay (same shape as the one in
/// `molt_drop_roundtrip.rs`): real URL parsing, real CBOR encoding,
/// in-memory storage, switchable write failures.
struct StubRelay {
    channels: Mutex<HashMap<String, Vec<StoredMessage>>>,
    next_cursor: AtomicU64,
    fail_writes: AtomicBool,
}

impl StubRelay {
    fn new() -> Self {
        StubRelay {
            channels: Mutex::new(HashMap::new()),
            next_cursor: AtomicU64::new(1),
            fail_writes: AtomicBool::new(false),
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
}

#[async_trait]
impl DropHttp for StubRelay {
    async fn http_put(&self, url: &str, body: Vec<u8>) -> Result<u64> {
        if self.fail_writes.load(Ordering::SeqCst) {
            return Err(PaykitError::Transport("relay write failure".into()));
        }
        let key = Self::channel_key(url)?;
        let cursor = self.next_cursor.fetch_add(1, Ordering::SeqCst);
        self.channels
            .lock()
            .expect("lock")
            .entry(key)
            .or_default()
            .push((cursor, 1_700_000_000, body));
        Ok(cursor)
    }

    async fn http_get(&self, url: &str, _max_response_bytes: usize) -> Result<Vec<u8>> {
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
                            serde_cbor::Value::Integer(0.into()),
                            serde_cbor::Value::Integer(cursor as i128),
                        ),
                        (
                            serde_cbor::Value::Integer(1.into()),
                            serde_cbor::Value::Integer(ts as i128),
                        ),
                        (
                            serde_cbor::Value::Integer(2.into()),
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
        Ok(())
    }
}

fn alice_bob_sessions() -> (PeerId, PeerId, BondSession, BondSession) {
    let alice = PeerId([0x01; 32]);
    let bob = PeerId([0x02; 32]);
    let sk_a = derive_pair_secret(&[0x11; 32], &bob).expect("pair secret");
    let sk_b = derive_pair_secret(&[0x22; 32], &alice).expect("pair secret");
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

#[tokio::test]
async fn bonded_ack_arrives_via_drop_and_never_touches_public_outbox() {
    let (alice, _bob, mut sa, mut sb) = alice_bob_sessions();
    let c = DropClient::new("http://relay.test", StubRelay::new()).expect("client");

    // The "public outbox" for this test: a closure that records any write.
    // With a BondSession supplied it must never run.
    let public_writes = Mutex::new(Vec::<Vec<u8>>::new());
    let encrypted_ack = b"sb2-signed-ack-bytes".to_vec();

    let mut outbound = OutboundTransport::Bonded {
        session: &mut sa,
        client: &c,
    };
    assert!(outbound.is_bonded());
    store_encrypted_ack(&encrypted_ack, &mut outbound, || async {
        public_writes
            .lock()
            .expect("lock")
            .push(b"public-write".to_vec());
        Ok::<(), PaykitError>(())
    })
    .await
    .expect("bonded ack send");

    // No public write happened; the message is on the relay instead.
    assert!(public_writes.lock().expect("lock").is_empty());
    assert_eq!(c.backend().message_count(), 1);

    // Bob opens it: kind Ack ⇒ ExternallyAuthenticated, body byte-identical.
    let received = receive_bonded(std::slice::from_mut(&mut sb), &[PurposeId::paykit()], &c)
        .await
        .expect("receive");
    assert_eq!(received.len(), 1);
    let (peer, hdr, authenticity, body) = &received[0];
    assert_eq!(peer, &alice);
    assert_eq!(hdr.purpose, PurposeId::paykit());
    assert_eq!(authenticity, &Authenticity::ExternallyAuthenticated);
    assert_eq!(body, &encrypted_ack);
}

#[tokio::test]
async fn public_outbox_route_invokes_only_the_callers_write() {
    let (_alice, _bob, sa, _sb) = alice_bob_sessions();
    let c = DropClient::new("http://relay.test", StubRelay::new()).expect("client");
    let public_writes = Mutex::new(Vec::<Vec<u8>>::new());
    let encrypted_ack = b"sb2-signed-ack-bytes".to_vec();

    let mut outbound: OutboundTransport<'_, StubRelay> = OutboundTransport::PublicOutbox;
    assert!(!outbound.is_bonded());
    store_encrypted_ack(&encrypted_ack, &mut outbound, || async {
        public_writes
            .lock()
            .expect("lock")
            .push(b"public-write".to_vec());
        Ok::<(), PaykitError>(())
    })
    .await
    .expect("public write");

    // The caller's write ran exactly once; nothing reached the relay and the
    // (unused) session ratchet was not advanced.
    assert_eq!(public_writes.lock().expect("lock").len(), 1);
    assert_eq!(c.backend().message_count(), 0);
    assert_eq!(sa.send.next_index(), 0);
}

#[tokio::test]
async fn bonded_failure_fails_closed_without_public_fallback() {
    let (_alice, _bob, mut sa, _sb) = alice_bob_sessions();
    let relay = StubRelay::new();
    relay.fail_writes.store(true, Ordering::SeqCst);
    let c = DropClient::new("http://relay.test", relay).expect("client");
    let public_writes = Mutex::new(Vec::<Vec<u8>>::new());

    let mut outbound = OutboundTransport::Bonded {
        session: &mut sa,
        client: &c,
    };
    let err = store_encrypted_ack(b"ack", &mut outbound, || async {
        public_writes
            .lock()
            .expect("lock")
            .push(b"public-write".to_vec());
        Ok::<(), PaykitError>(())
    })
    .await
    .expect_err("bonded send must fail");
    assert!(matches!(err, PaykitError::Transport(_)));
    // Fail closed: the public outbox was not used as a fallback.
    assert!(public_writes.lock().expect("lock").is_empty());
    assert_eq!(c.backend().message_count(), 0);
}

#[tokio::test]
async fn deliver_routes_request_kind_over_bonded_channel() {
    let (alice, _bob, mut sa, mut sb) = alice_bob_sessions();
    let c = DropClient::new("http://relay.test", StubRelay::new()).expect("client");

    let mut outbound = OutboundTransport::Bonded {
        session: &mut sa,
        client: &c,
    };
    outbound
        .deliver(ProtocolMessageKind::Request, b"request-body", || async {
            panic!("public write must not run on a bonded route");
            #[allow(unreachable_code)]
            Ok::<(), PaykitError>(())
        })
        .await
        .expect("bonded request send");

    let received = receive_bonded(std::slice::from_mut(&mut sb), &[PurposeId::paykit()], &c)
        .await
        .expect("receive");
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].0, alice);
    assert_eq!(received[0].2, Authenticity::SessionAuthenticated);
    assert_eq!(received[0].3, b"request-body");
}
