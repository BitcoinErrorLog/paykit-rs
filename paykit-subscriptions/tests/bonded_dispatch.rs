//! W2b: `publish_payment_request_routed` — bonded Drop delivery when a
//! `BondSession` exists for the peer, byte-identical public-outbox behavior
//! otherwise, and no silent fallback from bonded to public.

use async_trait::async_trait;
use paykit_lib::protocol::drop_transport::{
    receive_bonded, BondSession, DropClient, DropHttp, OutboundTransport,
};
use paykit_lib::protocol::owner_peerid_bytes_from_z32;
use paykit_lib::{EndpointData, HomeserverSessionStorage, MethodId, PublicKey};
use paykit_subscriptions::discovery::{
    publish_payment_request, publish_payment_request_routed, PublishedRequest,
};
use paykit_subscriptions::{Amount, PaymentRequest};
use pubky_crypto::molt::{
    derive_bond, derive_pair_secret, pair_public, Bond, BondRecord, PairPublic, PeerId, PurposeId,
};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// Homeserver storage mock recording every `put` (path, content).
struct RecordingStorage {
    puts: Mutex<Vec<(String, String)>>,
}

impl RecordingStorage {
    fn new() -> Self {
        RecordingStorage {
            puts: Mutex::new(Vec::new()),
        }
    }

    fn recorded(&self) -> Vec<(String, String)> {
        self.puts.lock().expect("lock").clone()
    }
}

#[async_trait]
impl HomeserverSessionStorage for RecordingStorage {
    async fn upsert_payment_endpoint(
        &self,
        _method: &MethodId,
        _data: &EndpointData,
    ) -> paykit_lib::Result<()> {
        Ok(())
    }

    async fn remove_payment_endpoint(&self, _method: &MethodId) -> paykit_lib::Result<()> {
        Ok(())
    }

    async fn put(&self, path: &str, content: &str) -> paykit_lib::Result<()> {
        self.puts
            .lock()
            .expect("lock")
            .push((path.to_string(), content.to_string()));
        Ok(())
    }

    async fn get(&self, _path: &str) -> paykit_lib::Result<Option<String>> {
        Ok(None)
    }

    async fn delete(&self, _path: &str) -> paykit_lib::Result<()> {
        Ok(())
    }
}

/// In-process mock of the S8 Drop relay: real URL parsing, real CBOR
/// encoding, in-memory storage, switchable write failures.
/// One stored relay message: (cursor, timestamp, body).
type StoredMessage = (u64, u64, Vec<u8>);

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

    fn channel_key(url: &str) -> paykit_lib::Result<String> {
        let path =
            url.split("/drop/")
                .nth(1)
                .ok_or_else(|| paykit_lib::PaykitError::InvalidData {
                    field: "url".into(),
                    reason: "missing /drop/ prefix".into(),
                })?;
        let channel = path.split(['?', '/']).next().unwrap_or("");
        use base64::Engine;
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(channel)
            .map_err(|_| paykit_lib::PaykitError::InvalidData {
                field: "channel".into(),
                reason: "invalid base64url".into(),
            })?;
        if decoded.len() != 32 {
            return Err(paykit_lib::PaykitError::InvalidData {
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
    async fn http_put(&self, url: &str, body: Vec<u8>) -> paykit_lib::Result<u64> {
        if self.fail_writes.load(Ordering::SeqCst) {
            return Err(paykit_lib::PaykitError::Transport(
                "relay write failure".into(),
            ));
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

    async fn http_get(&self, url: &str, _max_response_bytes: usize) -> paykit_lib::Result<Vec<u8>> {
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
            .map_err(|e| paykit_lib::PaykitError::Serialization(e.to_string()))
    }

    async fn http_delete(&self, url: &str) -> paykit_lib::Result<()> {
        let key = Self::channel_key(url)?;
        let cursor_str = url.rsplit('/').next().unwrap_or("");
        let cursor: u64 = cursor_str
            .parse()
            .map_err(|_| paykit_lib::PaykitError::InvalidData {
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

fn bonded_pair() -> (PeerId, PeerId, BondSession, BondSession) {
    let sender = PeerId([0x01; 32]);
    let recipient = PeerId([0x02; 32]);
    let sk_a = derive_pair_secret(&[0x11; 32], &recipient).expect("pair secret");
    let sk_b = derive_pair_secret(&[0x22; 32], &sender).expect("pair secret");
    let pk_a = pair_public(&sk_a);
    let pk_b = pair_public(&sk_b);
    let bond_a: Bond = derive_bond(&sender, &sk_a, &recipient, &pk_b).expect("bond");
    let bond_b: Bond = derive_bond(&recipient, &sk_b, &sender, &pk_a).expect("bond");
    let record = |peer: PeerId, pair_pk_peer: PairPublic| BondRecord {
        peer,
        pair_pk_peer,
        epoch_secs: 86_400,
        relays: vec!["http://relay.test".into()],
    };
    (
        sender,
        recipient,
        BondSession::new(&sender, recipient, bond_a, record(recipient, pk_b)),
        BondSession::new(&recipient, sender, bond_b, record(sender, pk_a)),
    )
}

fn test_pubkey() -> PublicKey {
    let keypair = pkarr::Keypair::random();
    PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
}

fn test_request(from: &PublicKey, to: &PublicKey) -> PaymentRequest {
    PaymentRequest::new(
        from.clone(),
        to.clone(),
        Amount::from_sats(1000),
        "SAT".to_string(),
        MethodId("lightning".to_string()),
    )
}

#[tokio::test]
async fn bonded_publish_never_touches_pub_paths_and_arrives_via_drop() {
    let (sender_peer, _recipient_peer, mut sender_session, mut recipient_session) = bonded_pair();
    let c = DropClient::new("http://relay.test", StubRelay::new()).expect("client");
    let storage = RecordingStorage::new();

    let from = test_pubkey();
    let to = test_pubkey();
    let request = test_request(&from, &to);
    let (noise_sk, noise_pk) = pubky_crypto::sealed_blob::x25519_generate_keypair();

    let mut outbound = OutboundTransport::Bonded {
        session: &mut sender_session,
        client: &c,
    };
    publish_payment_request_routed(
        &storage,
        &mut outbound,
        &from.to_string(),
        &request,
        &noise_pk,
    )
    .await
    .expect("bonded publish");

    // Not a single storage write — bonded delivery touches no `/pub/` path.
    assert!(storage.recorded().is_empty());
    assert_eq!(c.backend().message_count(), 1);

    // The recipient opens the request over its own Drop channel; the body
    // is the same encrypted blob the public path would have stored.
    let received = receive_bonded(
        std::slice::from_mut(&mut recipient_session),
        &[PurposeId::paykit()],
        &c,
    )
    .await
    .expect("receive");
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].0, sender_peer);
    let body = String::from_utf8(received[0].3.clone()).expect("blob is text");
    assert!(pubky_crypto::sealed_blob::is_sealed_blob(&body));

    let path = paykit_lib::protocol::payment_request_path(
        &from.to_string(),
        &to.to_string(),
        &request.request_id,
    )
    .expect("path");
    let owner_bytes = owner_peerid_bytes_from_z32(&from.to_string()).expect("owner");
    let plaintext = pubky_crypto::sealed_blob::sealed_blob_decrypt_with_context(
        &noise_sk,
        &body,
        &owner_bytes,
        &path,
    )
    .expect("decrypt request");
    let published: PublishedRequest = serde_json::from_slice(&plaintext).expect("decode");
    assert_eq!(published.request.request_id, request.request_id);
    assert!(published.active);
}

#[tokio::test]
async fn bonded_publish_failure_fails_closed_without_public_fallback() {
    let (_s, _r, mut sender_session, _recipient_session) = bonded_pair();
    let relay = StubRelay::new();
    relay.fail_writes.store(true, Ordering::SeqCst);
    let c = DropClient::new("http://relay.test", relay).expect("client");
    let storage = RecordingStorage::new();

    let from = test_pubkey();
    let to = test_pubkey();
    let request = test_request(&from, &to);
    let (_sk, noise_pk) = pubky_crypto::sealed_blob::x25519_generate_keypair();

    let mut outbound = OutboundTransport::Bonded {
        session: &mut sender_session,
        client: &c,
    };
    let err = publish_payment_request_routed(
        &storage,
        &mut outbound,
        &from.to_string(),
        &request,
        &noise_pk,
    )
    .await
    .expect_err("bonded send must fail");
    // Fail closed: the error surfaced and the public outbox was not used.
    assert!(
        format!("{err:#}").contains("relay write failure"),
        "unexpected: {err:#}"
    );
    assert!(storage.recorded().is_empty());
}

#[tokio::test]
async fn public_outbox_route_is_byte_identical_to_legacy_publish() {
    let from = test_pubkey();
    let to = test_pubkey();
    let request = test_request(&from, &to);
    let (_sk, noise_pk) = pubky_crypto::sealed_blob::x25519_generate_keypair();

    // Legacy entry point.
    let legacy_storage = RecordingStorage::new();
    publish_payment_request(&legacy_storage, &from.to_string(), &request, &noise_pk)
        .await
        .expect("legacy publish");

    // Routed entry point with the public variant (no BondSession supplied).
    let routed_storage = RecordingStorage::new();
    let c = DropClient::new("http://relay.test", StubRelay::new()).expect("client");
    let mut outbound: OutboundTransport<'_, StubRelay> = OutboundTransport::PublicOutbox;
    publish_payment_request_routed(
        &routed_storage,
        &mut outbound,
        &from.to_string(),
        &request,
        &noise_pk,
    )
    .await
    .expect("routed public publish");

    let legacy = legacy_storage.recorded();
    let routed = routed_storage.recorded();
    assert_eq!(legacy.len(), 1);
    assert_eq!(routed.len(), 1);
    // Same canonical path under /pub/paykit.app/v0/requests/ ...
    assert_eq!(legacy[0].0, routed[0].0);
    assert!(legacy[0].0.starts_with("/pub/paykit.app/v0/requests/"));
    // ... and a sealed blob at it (bodies differ only by random nonce).
    assert!(pubky_crypto::sealed_blob::is_sealed_blob(&legacy[0].1));
    assert!(pubky_crypto::sealed_blob::is_sealed_blob(&routed[0].1));
    // Nothing reached the Drop relay.
    assert_eq!(c.backend().message_count(), 0);
}
