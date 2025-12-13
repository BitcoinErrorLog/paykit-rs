//! Noise Protocol benchmarks
//!
//! These benchmarks measure the performance of Noise protocol operations
//! including handshakes and message encryption/decryption.
//!
//! Run with: `cargo bench --bench noise_benchmarks`

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use pubky_noise::datalink_adapter::{
    client_complete_ik, client_start_ik_direct, server_accept_ik, server_complete_ik,
};
use pubky_noise::{DummyRing, NoiseClient, NoiseServer, RingKeyProvider};
use std::sync::Arc;

/// Create test client and server with deterministic keys
fn create_test_peers() -> (
    NoiseClient<DummyRing, ()>,
    NoiseServer<DummyRing, ()>,
    [u8; 32], // Server public key
) {
    let ring_client = Arc::new(DummyRing::new([1u8; 32], "client"));
    let ring_server = Arc::new(DummyRing::new([2u8; 32], "server"));

    let client = NoiseClient::<_, ()>::new_direct("client", b"device", ring_client);
    let server = NoiseServer::<_, ()>::new_direct("server", b"device", ring_server.clone());

    let server_sk = ring_server
        .derive_device_x25519("server", b"device", 0)
        .unwrap();
    let server_pk = pubky_noise::kdf::x25519_pk_from_sk(&server_sk);

    (client, server, server_pk)
}

/// Benchmark Noise_IK handshake initiation (client side)
fn bench_noise_ik_handshake_initiate(c: &mut Criterion) {
    let (client, _server, server_pk) = create_test_peers();

    c.bench_function("noise_ik_handshake_initiate", |b| {
        b.iter(|| {
            let (hs, msg) = client_start_ik_direct(&client, black_box(&server_pk), None).unwrap();
            black_box((hs, msg))
        })
    });
}

/// Benchmark Noise_IK handshake acceptance (server side)
fn bench_noise_ik_handshake_accept(c: &mut Criterion) {
    let (client, server, server_pk) = create_test_peers();
    let (_, first_msg) = client_start_ik_direct(&client, &server_pk, None).unwrap();

    c.bench_function("noise_ik_handshake_accept", |b| {
        b.iter(|| {
            let (hs, identity, response) =
                server_accept_ik(&server, black_box(&first_msg)).unwrap();
            black_box((hs, identity, response))
        })
    });
}

/// Benchmark complete Noise_IK handshake (both sides)
fn bench_noise_ik_handshake_complete(c: &mut Criterion) {
    let ring_client = Arc::new(DummyRing::new([1u8; 32], "client"));
    let ring_server = Arc::new(DummyRing::new([2u8; 32], "server"));

    let server_sk = ring_server
        .derive_device_x25519("server", b"device", 0)
        .unwrap();
    let server_pk = pubky_noise::kdf::x25519_pk_from_sk(&server_sk);

    c.bench_function("noise_ik_handshake_complete", |b| {
        b.iter(|| {
            // Create fresh clients for each iteration (handshake consumes state)
            let client = NoiseClient::<_, ()>::new_direct("client", b"device", ring_client.clone());
            let server = NoiseServer::<_, ()>::new_direct("server", b"device", ring_server.clone());

            // Client initiates
            let (c_hs, first_msg) = client_start_ik_direct(&client, &server_pk, None).unwrap();

            // Server accepts
            let (s_hs, _identity, response) = server_accept_ik(&server, &first_msg).unwrap();

            // Client completes
            let c_link = client_complete_ik(c_hs, &response).unwrap();

            // Server completes
            let s_link = server_complete_ik(s_hs).unwrap();

            black_box((c_link, s_link))
        })
    });
}

/// Benchmark message encryption over established Noise channel
fn bench_noise_message_encrypt(c: &mut Criterion) {
    let ring_client = Arc::new(DummyRing::new([1u8; 32], "client"));
    let ring_server = Arc::new(DummyRing::new([2u8; 32], "server"));

    let client = NoiseClient::<_, ()>::new_direct("client", b"device", ring_client);
    let server = NoiseServer::<_, ()>::new_direct("server", b"device", ring_server.clone());

    let server_sk = ring_server
        .derive_device_x25519("server", b"device", 0)
        .unwrap();
    let server_pk = pubky_noise::kdf::x25519_pk_from_sk(&server_sk);

    // Complete handshake
    let (c_hs, first_msg) = client_start_ik_direct(&client, &server_pk, None).unwrap();
    let (s_hs, _, response) = server_accept_ik(&server, &first_msg).unwrap();
    let mut c_link = client_complete_ik(c_hs, &response).unwrap();

    // Prepare test messages
    let small_message = vec![0u8; 64];
    let medium_message = vec![0u8; 1024];
    let large_message = vec![0u8; 16384];

    let mut group = c.benchmark_group("noise_message_encrypt");

    group.throughput(Throughput::Bytes(64));
    group.bench_function("64_bytes", |b| {
        b.iter(|| {
            let ciphertext = c_link.encrypt(black_box(&small_message)).unwrap();
            black_box(ciphertext)
        })
    });

    group.throughput(Throughput::Bytes(1024));
    group.bench_function("1kb", |b| {
        b.iter(|| {
            let ciphertext = c_link.encrypt(black_box(&medium_message)).unwrap();
            black_box(ciphertext)
        })
    });

    group.throughput(Throughput::Bytes(16384));
    group.bench_function("16kb", |b| {
        b.iter(|| {
            let ciphertext = c_link.encrypt(black_box(&large_message)).unwrap();
            black_box(ciphertext)
        })
    });

    group.finish();
}

/// Benchmark message decryption over established Noise channel
fn bench_noise_message_decrypt(c: &mut Criterion) {
    let ring_client = Arc::new(DummyRing::new([1u8; 32], "client"));
    let ring_server = Arc::new(DummyRing::new([2u8; 32], "server"));

    let client = NoiseClient::<_, ()>::new_direct("client", b"device", ring_client);
    let server = NoiseServer::<_, ()>::new_direct("server", b"device", ring_server.clone());

    let server_sk = ring_server
        .derive_device_x25519("server", b"device", 0)
        .unwrap();
    let server_pk = pubky_noise::kdf::x25519_pk_from_sk(&server_sk);

    // Complete handshake
    let (c_hs, first_msg) = client_start_ik_direct(&client, &server_pk, None).unwrap();
    let (s_hs, _, response) = server_accept_ik(&server, &first_msg).unwrap();
    let mut c_link = client_complete_ik(c_hs, &response).unwrap();
    let mut s_link = server_complete_ik(s_hs).unwrap();

    // Encrypt a message to decrypt
    let message = vec![0u8; 1024];
    let ciphertext = c_link.encrypt(&message).unwrap();

    c.bench_function("noise_message_decrypt_1kb", |b| {
        b.iter(|| {
            let plaintext = s_link.decrypt(black_box(&ciphertext)).unwrap();
            black_box(plaintext)
        })
    });
}

/// Benchmark full message round-trip (encrypt + decrypt)
fn bench_noise_message_roundtrip(c: &mut Criterion) {
    let ring_client = Arc::new(DummyRing::new([1u8; 32], "client"));
    let ring_server = Arc::new(DummyRing::new([2u8; 32], "server"));

    let client = NoiseClient::<_, ()>::new_direct("client", b"device", ring_client);
    let server = NoiseServer::<_, ()>::new_direct("server", b"device", ring_server.clone());

    let server_sk = ring_server
        .derive_device_x25519("server", b"device", 0)
        .unwrap();
    let server_pk = pubky_noise::kdf::x25519_pk_from_sk(&server_sk);

    // Complete handshake
    let (c_hs, first_msg) = client_start_ik_direct(&client, &server_pk, None).unwrap();
    let (s_hs, _, response) = server_accept_ik(&server, &first_msg).unwrap();
    let mut c_link = client_complete_ik(c_hs, &response).unwrap();
    let mut s_link = server_complete_ik(s_hs).unwrap();

    let message = vec![0u8; 1024];

    c.bench_function("noise_message_roundtrip_1kb", |b| {
        b.iter(|| {
            // Client encrypts
            let ciphertext = c_link.encrypt(black_box(&message)).unwrap();
            // Server decrypts
            let plaintext = s_link.decrypt(&ciphertext).unwrap();
            black_box(plaintext)
        })
    });
}

criterion_group!(
    noise_benches,
    bench_noise_ik_handshake_initiate,
    bench_noise_ik_handshake_accept,
    bench_noise_ik_handshake_complete,
    bench_noise_message_encrypt,
    bench_noise_message_decrypt,
    bench_noise_message_roundtrip,
);

criterion_main!(noise_benches);
