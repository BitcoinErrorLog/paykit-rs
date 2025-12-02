//! Noise protocol integration tests
//!
//! Tests the 3-step IK handshake pattern used by pay/receive commands.

use pubky_noise::datalink_adapter::{
    client_complete_ik, client_start_ik_direct, server_accept_ik, server_complete_ik,
};
use pubky_noise::{DummyRing, NoiseClient, NoiseServer, RingKeyProvider};
use std::sync::Arc;

#[tokio::test]
async fn test_noise_3step_handshake() {
    // Test the 3-step IK handshake pattern used by pay/receive
    let ring_client = Arc::new(DummyRing::new([1u8; 32], "client"));
    let ring_server = Arc::new(DummyRing::new([2u8; 32], "server"));

    let client = NoiseClient::<_, ()>::new_direct("client", b"dev", ring_client.clone());
    let server = NoiseServer::<_, ()>::new_direct("server", b"dev", ring_server.clone(), 3);

    // Get server static key
    let server_sk = ring_server
        .derive_device_x25519("server", b"dev", 3)
        .unwrap();
    let server_static_pk = pubky_noise::kdf::x25519_pk_from_sk(&server_sk);

    // Step 1: Client initiates
    let (c_hs, _epoch, first_msg) =
        client_start_ik_direct(&client, &server_static_pk, 3).unwrap();

    // Step 2: Server responds
    let (s_hs, _identity, response) = server_accept_ik(&server, &first_msg).unwrap();

    // Step 3: Both complete
    let mut c_link = client_complete_ik(c_hs, &response).unwrap();
    let mut s_link = server_complete_ik(s_hs).unwrap();

    // Verify session IDs match
    assert_eq!(c_link.session_id(), s_link.session_id());

    // Test encryption/decryption
    let plaintext = b"test payment data";
    let ciphertext = c_link.encrypt(plaintext).unwrap();
    let decrypted = s_link.decrypt(&ciphertext).unwrap();
    assert_eq!(plaintext, &decrypted[..]);
}

#[tokio::test]
async fn test_noise_handshake_with_identity_payload() {
    // Test that identity payload is correctly transmitted during handshake
    let ring_client = Arc::new(DummyRing::new([3u8; 32], "alice"));
    let ring_server = Arc::new(DummyRing::new([4u8; 32], "bob"));

    let client = NoiseClient::<_, ()>::new_direct("alice", b"device1", ring_client.clone());
    let server = NoiseServer::<_, ()>::new_direct("bob", b"device2", ring_server.clone(), 0);

    // Get server static key
    let server_sk = ring_server
        .derive_device_x25519("bob", b"device2", 0)
        .unwrap();
    let server_static_pk = pubky_noise::kdf::x25519_pk_from_sk(&server_sk);

    // Client initiates
    let (c_hs, _epoch, first_msg) =
        client_start_ik_direct(&client, &server_static_pk, 0).unwrap();

    // Server receives and extracts identity
    let (s_hs, identity, response) = server_accept_ik(&server, &first_msg).unwrap();

    // Verify identity payload was received (check for non-zero Ed25519 key)
    assert_ne!(identity.ed25519_pub, [0u8; 32]);
    assert_eq!(identity.epoch, 0); // Verify epoch is transmitted

    // Complete handshake
    let _c_link = client_complete_ik(c_hs, &response).unwrap();
    let _s_link = server_complete_ik(s_hs).unwrap();
}

#[tokio::test]
async fn test_noise_message_exchange() {
    // Test bidirectional message exchange after handshake
    let ring_client = Arc::new(DummyRing::new([5u8; 32], "payer"));
    let ring_server = Arc::new(DummyRing::new([6u8; 32], "payee"));

    let client = NoiseClient::<_, ()>::new_direct("payer", b"wallet", ring_client.clone());
    let server = NoiseServer::<_, ()>::new_direct("payee", b"receiver", ring_server.clone(), 1);

    // Get server static key
    let server_sk = ring_server
        .derive_device_x25519("payee", b"receiver", 1)
        .unwrap();
    let server_static_pk = pubky_noise::kdf::x25519_pk_from_sk(&server_sk);

    // Perform handshake
    let (c_hs, _epoch, first_msg) =
        client_start_ik_direct(&client, &server_static_pk, 1).unwrap();
    let (s_hs, _identity, response) = server_accept_ik(&server, &first_msg).unwrap();
    let mut c_link = client_complete_ik(c_hs, &response).unwrap();
    let mut s_link = server_complete_ik(s_hs).unwrap();

    // Client sends payment request
    let payment_request = b"PAYMENT_REQUEST: 1000 SAT";
    let encrypted_request = c_link.encrypt(payment_request).unwrap();
    let decrypted_request = s_link.decrypt(&encrypted_request).unwrap();
    assert_eq!(payment_request, &decrypted_request[..]);

    // Server sends payment response
    let payment_response = b"PAYMENT_ACCEPTED: receipt_id_12345";
    let encrypted_response = s_link.encrypt(payment_response).unwrap();
    let decrypted_response = c_link.decrypt(&encrypted_response).unwrap();
    assert_eq!(payment_response, &decrypted_response[..]);
}
