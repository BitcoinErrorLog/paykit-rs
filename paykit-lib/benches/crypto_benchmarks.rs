//! Cryptographic operation benchmarks
//!
//! These benchmarks measure the performance of critical cryptographic operations
//! used in Paykit's payment protocol.
//!
//! Run with: `cargo bench --bench crypto_benchmarks`

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

/// Benchmark Ed25519 key generation
fn bench_keypair_generation(c: &mut Criterion) {
    c.bench_function("ed25519_keypair_generation", |b| {
        b.iter(|| {
            let keypair = pubky::Keypair::random();
            black_box(keypair)
        })
    });
}

/// Benchmark Ed25519 signature creation
fn bench_ed25519_signing(c: &mut Criterion) {
    use ed25519_dalek::{Signer, SigningKey};

    let signing_key = SigningKey::generate(&mut rand::thread_rng());
    let message = b"Test message for signing benchmark - this is a typical payment receipt";

    c.bench_function("ed25519_sign", |b| {
        b.iter(|| {
            let signature = signing_key.sign(black_box(message));
            black_box(signature)
        })
    });
}

/// Benchmark Ed25519 signature verification
fn bench_ed25519_verification(c: &mut Criterion) {
    use ed25519_dalek::{Signer, SigningKey, Verifier};

    let signing_key = SigningKey::generate(&mut rand::thread_rng());
    let verifying_key = signing_key.verifying_key();
    let message = b"Test message for signing benchmark - this is a typical payment receipt";
    let signature = signing_key.sign(message);

    c.bench_function("ed25519_verify", |b| {
        b.iter(|| {
            let result = verifying_key.verify(black_box(message), black_box(&signature));
            black_box(result)
        })
    });
}

/// Benchmark X25519 key agreement (Diffie-Hellman)
fn bench_x25519_key_agreement(c: &mut Criterion) {
    use x25519_dalek::{EphemeralSecret, PublicKey};

    // Generate Bob's public key for DH computation
    let bob_secret = EphemeralSecret::random_from_rng(rand::thread_rng());
    let bob_public = PublicKey::from(&bob_secret);

    c.bench_function("x25519_key_agreement", |b| {
        b.iter(|| {
            // Generate fresh ephemeral secret each iteration (includes key gen time)
            let alice_secret = EphemeralSecret::random_from_rng(rand::thread_rng());
            let shared = alice_secret.diffie_hellman(black_box(&bob_public));
            black_box(shared)
        })
    });
}

/// Benchmark HKDF key derivation
fn bench_hkdf_derive(c: &mut Criterion) {
    use hkdf::Hkdf;
    use sha2::Sha256;

    let ikm = [0u8; 32]; // Input key material
    let salt = [1u8; 32];
    let info = b"paykit-session-key";

    c.bench_function("hkdf_sha256_derive_32bytes", |b| {
        b.iter(|| {
            let hk = Hkdf::<Sha256>::new(Some(&salt), &ikm);
            let mut okm = [0u8; 32];
            hk.expand(black_box(info), &mut okm).unwrap();
            black_box(okm)
        })
    });
}

/// Benchmark ChaCha20-Poly1305 encryption
fn bench_chacha20_poly1305_encrypt(c: &mut Criterion) {
    use chacha20poly1305::{
        aead::{Aead, KeyInit},
        ChaCha20Poly1305, Nonce,
    };

    let key = [0u8; 32];
    let cipher = ChaCha20Poly1305::new_from_slice(&key).unwrap();
    let nonce = Nonce::from_slice(b"unique nonce"); // 12-byte nonce

    // Typical message sizes
    let small_message = vec![0u8; 64]; // Small: 64 bytes
    let medium_message = vec![0u8; 1024]; // Medium: 1KB
    let large_message = vec![0u8; 16384]; // Large: 16KB

    let mut group = c.benchmark_group("chacha20_poly1305_encrypt");

    group.throughput(Throughput::Bytes(64));
    group.bench_function("64_bytes", |b| {
        b.iter(|| {
            let ciphertext = cipher
                .encrypt(nonce, black_box(small_message.as_slice()))
                .unwrap();
            black_box(ciphertext)
        })
    });

    group.throughput(Throughput::Bytes(1024));
    group.bench_function("1kb", |b| {
        b.iter(|| {
            let ciphertext = cipher
                .encrypt(nonce, black_box(medium_message.as_slice()))
                .unwrap();
            black_box(ciphertext)
        })
    });

    group.throughput(Throughput::Bytes(16384));
    group.bench_function("16kb", |b| {
        b.iter(|| {
            let ciphertext = cipher
                .encrypt(nonce, black_box(large_message.as_slice()))
                .unwrap();
            black_box(ciphertext)
        })
    });

    group.finish();
}

/// Benchmark ChaCha20-Poly1305 decryption
fn bench_chacha20_poly1305_decrypt(c: &mut Criterion) {
    use chacha20poly1305::{
        aead::{Aead, KeyInit},
        ChaCha20Poly1305, Nonce,
    };

    let key = [0u8; 32];
    let cipher = ChaCha20Poly1305::new_from_slice(&key).unwrap();
    let nonce = Nonce::from_slice(b"unique nonce");

    let message = vec![0u8; 1024];
    let ciphertext = cipher.encrypt(nonce, message.as_slice()).unwrap();

    c.bench_function("chacha20_poly1305_decrypt_1kb", |b| {
        b.iter(|| {
            let plaintext = cipher
                .decrypt(nonce, black_box(ciphertext.as_slice()))
                .unwrap();
            black_box(plaintext)
        })
    });
}

/// Benchmark BLAKE2b hashing
fn bench_blake2b_hash(c: &mut Criterion) {
    use blake2::{Blake2b512, Digest};

    let data = vec![0u8; 1024];

    c.bench_function("blake2b_hash_1kb", |b| {
        b.iter(|| {
            let mut hasher = Blake2b512::new();
            hasher.update(black_box(&data));
            let result = hasher.finalize();
            black_box(result)
        })
    });
}

/// Benchmark SHA-256 hashing
fn bench_sha256_hash(c: &mut Criterion) {
    use sha2::{Digest, Sha256};

    let data = vec![0u8; 1024];

    c.bench_function("sha256_hash_1kb", |b| {
        b.iter(|| {
            let mut hasher = Sha256::new();
            hasher.update(black_box(&data));
            let result = hasher.finalize();
            black_box(result)
        })
    });
}

criterion_group!(
    crypto_benches,
    bench_keypair_generation,
    bench_ed25519_signing,
    bench_ed25519_verification,
    bench_x25519_key_agreement,
    bench_hkdf_derive,
    bench_chacha20_poly1305_encrypt,
    bench_chacha20_poly1305_decrypt,
    bench_blake2b_hash,
    bench_sha256_hash,
);

criterion_main!(crypto_benches);
