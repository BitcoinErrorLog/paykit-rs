//! Cross-check: the `pubky-crypto` Molt crypto vectors
//! (`tests/vectors/molt_crypto_v1.json`) are verified through paykit-lib's
//! own `BondSession` code path — the same derivation a production session
//! uses — so a drift in either crate fails here.

use paykit_lib::protocol::drop_transport::BondSession;
use pubky_crypto::molt::{
    derive_bond, derive_pair_secret, pair_public, BondRecord, PairPublic, PeerId, PurposeId,
};

fn hex32(s: &str) -> [u8; 32] {
    let bytes = hex::decode(s).expect("hex");
    <[u8; 32]>::try_from(bytes.as_slice()).expect("32 bytes")
}

fn load_vectors() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../pubky-crypto/tests/vectors/molt_crypto_v1.json"
    );
    let text = std::fs::read_to_string(path).expect("read molt_crypto_v1.json");
    serde_json::from_str(&text).expect("parse vectors")
}

#[test]
fn channel_id_vectors_match_through_bond_session() {
    let v = load_vectors();
    let inputs = &v["inputs"];

    let alice_seed = hex32(inputs["alice_seed"].as_str().expect("alice_seed"));
    let bob_seed = hex32(inputs["bob_seed"].as_str().expect("bob_seed"));
    let alice_id = PeerId(hex32(inputs["alice_peer_id"].as_str().expect("alice")));
    let bob_id = PeerId(hex32(inputs["bob_peer_id"].as_str().expect("bob")));
    let epoch_secs = inputs["epoch_secs"].as_u64().expect("epoch_secs") as u32;

    // Both sides derive their pair secrets from their own seeds, exactly as
    // a real session would after an intro exchange.
    let sk_a = derive_pair_secret(&alice_seed, &bob_id);
    let sk_b = derive_pair_secret(&bob_seed, &alice_id);
    let pk_a = pair_public(&sk_a);
    let pk_b = pair_public(&sk_b);

    // Sanity: the derived pair publics match the vector file.
    assert_eq!(
        pk_a.0,
        hex32(v["bond"]["alice_pair_public"].as_str().expect("pk_a"))
    );
    assert_eq!(
        pk_b.0,
        hex32(v["bond"]["bob_pair_public"].as_str().expect("pk_b"))
    );

    let bond_a = derive_bond(&alice_id, &sk_a, &bob_id, &pk_b).expect("bond alice");
    let bond_b = derive_bond(&bob_id, &sk_b, &alice_id, &pk_a).expect("bond bob");
    assert_eq!(
        bond_a.as_bytes(),
        &hex32(v["bond"]["bond_from_alice"].as_str().expect("bond"))
    );

    let record = |peer: PeerId, pair_pk_peer: PairPublic| BondRecord {
        peer,
        pair_pk_peer,
        epoch_secs,
        relays: vec![],
    };
    let sa = BondSession::new(&alice_id, bob_id, bond_a, record(bob_id, pk_b));
    let sb = BondSession::new(&bob_id, alice_id, bond_b, record(alice_id, pk_a));

    let channels = v["channels"].as_array().expect("channels array");
    assert_eq!(
        channels.len(),
        4,
        "vector set covers both purposes/directions"
    );
    for entry in channels {
        let purpose =
            PurposeId::parse(entry["purpose"].as_str().expect("purpose")).expect("valid purpose");
        let dir = entry["dir"].as_u64().expect("dir") as u8;
        let epochs = entry["epochs"].as_object().expect("epochs map");
        let mut epochs: Vec<(u32, &str)> = epochs
            .iter()
            .map(|(k, v)| {
                (
                    k.parse::<u32>().expect("epoch key"),
                    v.as_str().expect("channel hex"),
                )
            })
            .collect();
        epochs.sort_by_key(|(e, _)| *e);
        for (e, want_hex) in epochs {
            let want = hex32(want_hex);
            // Route the check through each side's BondSession helpers: the
            // direction byte selects the traffic direction, and both peers
            // must derive the identical channel id for it.
            let from_alice = match dir {
                0 => sa.send_channel(&purpose, e),
                1 => sa.recv_channel(&purpose, e),
                other => panic!("bad dir {other}"),
            };
            let from_bob = match dir {
                0 => sb.recv_channel(&purpose, e),
                1 => sb.send_channel(&purpose, e),
                other => panic!("bad dir {other}"),
            };
            assert_eq!(
                from_alice.0,
                want,
                "channel mismatch: {} dir {dir} epoch {e}",
                purpose.as_str()
            );
            assert_eq!(
                from_bob.0,
                want,
                "bob-derived channel mismatch: {} dir {dir} epoch {e}",
                purpose.as_str()
            );
        }
    }
}

#[test]
fn vector_file_is_present_and_shaped() {
    let v = load_vectors();
    assert_eq!(v["format"].as_str().expect("format"), "molt_crypto_v1");
    // A malformed vector file (missing sections) must fail loudly here
    // rather than silently pass a vacuous cross-check.
    for section in ["inputs", "bond", "directions", "channels"] {
        assert!(!v[section].is_null(), "missing section {section}");
    }
    // Direction vectors: alice sends LoToHi (0), bob sends HiToLo (1).
    assert_eq!(v["directions"]["alice"]["send"].as_u64().expect("d"), 0);
    assert_eq!(v["directions"]["bob"]["send"].as_u64().expect("d"), 1);
}
