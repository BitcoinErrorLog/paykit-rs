//! BIP Test Vector Verification
//!
//! This module verifies that the test vectors in `docs/bip-paykit/test-vectors.json`
//! are structurally valid, have correct field sizes, and that encoded values match
//! their hex inputs.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::Value;
use std::fs;

const TEST_VECTORS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/bip-paykit/test-vectors.json");

fn load_test_vectors() -> Value {
    let content = fs::read_to_string(TEST_VECTORS_PATH)
        .expect("Failed to read test-vectors.json");
    serde_json::from_str(&content).expect("Failed to parse test-vectors.json")
}

#[test]
fn test_vectors_file_is_valid_json() {
    let vectors = load_test_vectors();
    assert!(vectors.is_object(), "Test vectors should be a JSON object");
}

#[test]
fn test_vectors_has_required_sections() {
    let vectors = load_test_vectors();
    
    let required_sections = [
        "identity_encoding",
        "directory_paths",
        "endpoint_data",
        "sealed_blob_v2",
        "sealed_blob_v1_compat",
        "message_framing",
        "noise_handshake",
    ];
    
    for section in required_sections {
        assert!(
            vectors.get(section).is_some(),
            "Missing required section: {}",
            section
        );
    }
}

#[test]
fn test_directory_paths_have_correct_prefix() {
    let vectors = load_test_vectors();
    let dir_paths = &vectors["directory_paths"];
    
    let prefix = dir_paths["prefix"].as_str().unwrap();
    assert_eq!(prefix, "/pub/paykit.app/v0/", "Directory path prefix mismatch");
    
    let path_vectors = dir_paths["vectors"].as_array().unwrap();
    for vector in path_vectors {
        let full_path = vector["full_path"].as_str().unwrap();
        assert!(
            full_path.starts_with(prefix),
            "Path {} should start with {}",
            full_path,
            prefix
        );
    }
}

#[test]
fn test_sealed_blob_v2_has_correct_nonce_size() {
    let vectors = load_test_vectors();
    let sealed_v2 = &vectors["sealed_blob_v2"]["vectors"][0];
    
    let nonce_hex = sealed_v2["inputs"]["nonce_hex"].as_str().unwrap();
    // 24 bytes = 48 hex chars
    assert_eq!(
        nonce_hex.len(),
        48,
        "V2 nonce should be 24 bytes (48 hex chars), got {}",
        nonce_hex.len()
    );
    
    let expected_nonce_base64url = sealed_v2["expected_envelope_fields"]["nonce_base64url"]
        .as_str()
        .unwrap();
    // 24 bytes -> ceil(24 * 8 / 6) = 32 base64url chars
    assert_eq!(
        expected_nonce_base64url.len(),
        32,
        "V2 nonce base64url should be 32 chars, got {}",
        expected_nonce_base64url.len()
    );
}

#[test]
fn test_sealed_blob_v1_has_correct_nonce_size() {
    let vectors = load_test_vectors();
    let sealed_v1 = &vectors["sealed_blob_v1_compat"]["vectors"][0];
    
    let nonce_hex = sealed_v1["inputs"]["nonce_hex"].as_str().unwrap();
    // 12 bytes = 24 hex chars
    assert_eq!(
        nonce_hex.len(),
        24,
        "V1 nonce should be 12 bytes (24 hex chars), got {}",
        nonce_hex.len()
    );
    
    let expected_nonce_base64url = sealed_v1["expected_envelope_fields"]["nonce_base64url"]
        .as_str()
        .unwrap();
    // 12 bytes -> ceil(12 * 8 / 6) = 16 base64url chars
    assert_eq!(
        expected_nonce_base64url.len(),
        16,
        "V1 nonce base64url should be 16 chars, got {}",
        expected_nonce_base64url.len()
    );
}

#[test]
fn test_ephemeral_public_key_sizes() {
    let vectors = load_test_vectors();
    
    // V2
    let v2_epk_hex = vectors["sealed_blob_v2"]["vectors"][0]["inputs"]["ephemeral_pk_hex"]
        .as_str()
        .unwrap();
    assert_eq!(
        v2_epk_hex.len(),
        64,
        "EPK should be 32 bytes (64 hex chars)"
    );
    
    let v2_epk_base64url = vectors["sealed_blob_v2"]["vectors"][0]["expected_envelope_fields"]["epk_base64url"]
        .as_str()
        .unwrap();
    // 32 bytes -> ceil(32 * 8 / 6) = 43 base64url chars
    assert_eq!(
        v2_epk_base64url.len(),
        43,
        "EPK base64url should be 43 chars, got {}",
        v2_epk_base64url.len()
    );
}

#[test]
fn test_message_framing_length_prefix() {
    let vectors = load_test_vectors();
    let framing = &vectors["message_framing"]["vectors"][0];
    
    let message_hex = framing["message_bytes_hex"].as_str().unwrap();
    let length_prefix_hex = framing["length_prefix_hex"].as_str().unwrap();
    let framed_hex = framing["framed_message_hex"].as_str().unwrap();
    
    // Length prefix should be 4 bytes = 8 hex chars
    assert_eq!(length_prefix_hex.len(), 8, "Length prefix should be 4 bytes");
    
    // Framed = prefix + message
    let expected_framed = format!("{}{}", length_prefix_hex, message_hex);
    assert_eq!(
        framed_hex, expected_framed,
        "Framed message should be prefix + message"
    );
    
    // Verify length value matches message length
    let length = u32::from_str_radix(length_prefix_hex, 16).unwrap();
    let message_len = message_hex.len() / 2; // hex chars / 2 = bytes
    assert_eq!(
        length as usize, message_len,
        "Length prefix value should match message length"
    );
}

#[test]
fn test_noise_handshake_patterns() {
    let vectors = load_test_vectors();
    let patterns = vectors["noise_handshake"]["patterns"].as_array().unwrap();
    
    assert!(patterns.len() >= 2, "Should have at least IK and XX patterns");
    
    let pattern_names: Vec<&str> = patterns
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    
    assert!(pattern_names.contains(&"Noise_IK"), "Should include Noise_IK");
    assert!(pattern_names.contains(&"Noise_XX"), "Should include Noise_XX");
}

// ============================================================================
// Encoding Consistency Tests - Verify base64url matches hex inputs
// ============================================================================

#[test]
fn test_sealed_blob_v2_epk_base64url_matches_hex() {
    let vectors = load_test_vectors();
    let sealed_v2 = &vectors["sealed_blob_v2"]["vectors"][0];
    
    let epk_hex = sealed_v2["inputs"]["ephemeral_pk_hex"].as_str().unwrap();
    let epk_base64url = sealed_v2["expected_envelope_fields"]["epk_base64url"]
        .as_str()
        .unwrap();
    
    // Decode base64url and compare to hex bytes
    let epk_from_hex = hex::decode(epk_hex).expect("Invalid hex in ephemeral_pk_hex");
    let epk_from_b64 = URL_SAFE_NO_PAD
        .decode(epk_base64url)
        .expect("Invalid base64url in epk_base64url");
    
    assert_eq!(
        epk_from_hex, epk_from_b64,
        "V2 epk_base64url does not decode to same bytes as ephemeral_pk_hex"
    );
}

#[test]
fn test_sealed_blob_v2_nonce_base64url_matches_hex() {
    let vectors = load_test_vectors();
    let sealed_v2 = &vectors["sealed_blob_v2"]["vectors"][0];
    
    let nonce_hex = sealed_v2["inputs"]["nonce_hex"].as_str().unwrap();
    let nonce_base64url = sealed_v2["expected_envelope_fields"]["nonce_base64url"]
        .as_str()
        .unwrap();
    
    let nonce_from_hex = hex::decode(nonce_hex).expect("Invalid hex in nonce_hex");
    let nonce_from_b64 = URL_SAFE_NO_PAD
        .decode(nonce_base64url)
        .expect("Invalid base64url in nonce_base64url");
    
    assert_eq!(
        nonce_from_hex, nonce_from_b64,
        "V2 nonce_base64url does not decode to same bytes as nonce_hex"
    );
}

#[test]
fn test_sealed_blob_v1_epk_base64url_matches_hex() {
    let vectors = load_test_vectors();
    let sealed_v1 = &vectors["sealed_blob_v1_compat"]["vectors"][0];
    
    let epk_hex = sealed_v1["inputs"]["ephemeral_pk_hex"].as_str().unwrap();
    let epk_base64url = sealed_v1["expected_envelope_fields"]["epk_base64url"]
        .as_str()
        .unwrap();
    
    let epk_from_hex = hex::decode(epk_hex).expect("Invalid hex in ephemeral_pk_hex");
    let epk_from_b64 = URL_SAFE_NO_PAD
        .decode(epk_base64url)
        .expect("Invalid base64url in epk_base64url");
    
    assert_eq!(
        epk_from_hex, epk_from_b64,
        "V1 epk_base64url does not decode to same bytes as ephemeral_pk_hex"
    );
}

#[test]
fn test_sealed_blob_v1_nonce_base64url_matches_hex() {
    let vectors = load_test_vectors();
    let sealed_v1 = &vectors["sealed_blob_v1_compat"]["vectors"][0];
    
    let nonce_hex = sealed_v1["inputs"]["nonce_hex"].as_str().unwrap();
    let nonce_base64url = sealed_v1["expected_envelope_fields"]["nonce_base64url"]
        .as_str()
        .unwrap();
    
    let nonce_from_hex = hex::decode(nonce_hex).expect("Invalid hex in nonce_hex");
    let nonce_from_b64 = URL_SAFE_NO_PAD
        .decode(nonce_base64url)
        .expect("Invalid base64url in nonce_base64url");
    
    assert_eq!(
        nonce_from_hex, nonce_from_b64,
        "V1 nonce_base64url does not decode to same bytes as nonce_hex"
    );
}

#[test]
fn test_identity_encoding_all_zeros_is_correct() {
    let vectors = load_test_vectors();
    let id_vectors = &vectors["identity_encoding"]["vectors"];
    
    // Find the all-zeros vector
    for vector in id_vectors.as_array().unwrap() {
        let pk_hex = vector["public_key_hex"].as_str().unwrap();
        let z32 = vector["z_base_32"].as_str().unwrap();
        
        // All zeros should encode to all 'y' (z-base-32 first character)
        if pk_hex == "0000000000000000000000000000000000000000000000000000000000000000" {
            assert_eq!(
                z32,
                "yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy",
                "All-zeros public key should encode to all 'y' in z-base-32"
            );
            assert_eq!(z32.len(), 52, "z-base-32 should be 52 chars for 32 bytes");
        }
    }
}

// ============================================================================
// ContextId Tests - Verify context_id derivation matches test vectors
// ============================================================================

#[test]
fn test_context_id_section_exists() {
    let vectors = load_test_vectors();
    assert!(
        vectors.get("context_id").is_some(),
        "Missing required section: context_id"
    );
}

#[test]
fn test_context_id_vectors_match_implementation() {
    use paykit_lib::protocol::context_id;

    let vectors = load_test_vectors();
    let ctx_vectors = &vectors["context_id"]["vectors"];

    for vector in ctx_vectors.as_array().unwrap() {
        let expected = vector["expected_context_id"]
            .as_str()
            .expect("Vector missing expected_context_id - all vectors must have concrete expected values");
        let pubkey_a: &str = vector["pubkey_a"].as_str().unwrap();
        let pubkey_b: &str = vector["pubkey_b"].as_str().unwrap();

        let computed: String = context_id(pubkey_a, pubkey_b)
            .expect("Failed to compute context_id");

        assert_eq!(
            computed, expected,
            "ContextId mismatch for ({}, {}): expected {}, got {}",
            pubkey_a, pubkey_b, expected, computed
        );
    }
}

#[test]
fn test_context_id_is_symmetric() {
    use paykit_lib::protocol::context_id;

    let vectors = load_test_vectors();
    let ctx_vectors = &vectors["context_id"]["vectors"];

    for vector in ctx_vectors.as_array().unwrap() {
        let pubkey_a = vector["pubkey_a"].as_str().unwrap();
        let pubkey_b = vector["pubkey_b"].as_str().unwrap();

        let ctx_ab = context_id(pubkey_a, pubkey_b)
            .expect("Failed to compute context_id A->B");
        let ctx_ba = context_id(pubkey_b, pubkey_a)
            .expect("Failed to compute context_id B->A");

        assert_eq!(
            ctx_ab, ctx_ba,
            "ContextId should be symmetric: ({}, {}) != ({}, {})",
            pubkey_a, pubkey_b, pubkey_b, pubkey_a
        );
    }
}

#[test]
fn test_context_id_preimage_format() {
    let vectors = load_test_vectors();
    let ctx_vectors = &vectors["context_id"]["vectors"];

    for vector in ctx_vectors.as_array().unwrap() {
        let preimage = vector["preimage"]
            .as_str()
            .expect("Vector missing preimage - all vectors must have concrete values");
        let first_z32 = vector["first_z32"].as_str().unwrap();
        let second_z32 = vector["second_z32"].as_str().unwrap();

        // Verify preimage format matches spec
        let expected_preimage = format!("paykit:v0:context:{}:{}", first_z32, second_z32);
        assert_eq!(
            preimage, expected_preimage,
            "Preimage format mismatch"
        );

        // Verify first < second lexicographically (normalization requirement)
        assert!(
            first_z32 <= second_z32,
            "first_z32 should be <= second_z32 lexicographically"
        );
    }
}

// ============================================================================
// ACK Path and AAD Tests - Verify ack_path and ack_aad match test vectors
// ============================================================================

#[test]
fn test_ack_paths_section_exists() {
    let vectors = load_test_vectors();
    assert!(
        vectors.get("ack_paths").is_some(),
        "Missing required section: ack_paths"
    );
}

#[test]
fn test_ack_path_vectors_match_implementation() {
    use paykit_lib::protocol::ack_path;

    let vectors = load_test_vectors();
    let ack_vectors = &vectors["ack_paths"]["vectors"];

    for vector in ack_vectors.as_array().unwrap() {
        let object_type = vector["object_type"].as_str().unwrap();
        let sender_z32 = vector["sender_z32"].as_str().unwrap();
        let recipient_z32 = vector["recipient_z32"].as_str().unwrap();
        let msg_id = vector["msg_id"].as_str().unwrap();
        let expected_path = vector["expected_path"]
            .as_str()
            .expect("Vector missing expected_path - all vectors must have concrete values");

        let computed = ack_path(object_type, sender_z32, recipient_z32, msg_id)
            .expect("Failed to compute ack_path");

        assert_eq!(
            computed, expected_path,
            "ACK path mismatch for {} {}: expected {}, got {}",
            object_type, msg_id, expected_path, computed
        );
    }
}

#[test]
fn test_ack_aad_vectors_match_implementation() {
    use paykit_lib::protocol::ack_aad;

    let vectors = load_test_vectors();
    let ack_vectors = &vectors["ack_paths"]["vectors"];

    for vector in ack_vectors.as_array().unwrap() {
        let object_type = vector["object_type"].as_str().unwrap();
        let sender_z32 = vector["sender_z32"].as_str().unwrap();
        let recipient_z32 = vector["recipient_z32"].as_str().unwrap();
        let msg_id = vector["msg_id"].as_str().unwrap();
        let expected_aad = vector["expected_aad"]
            .as_str()
            .expect("Vector missing expected_aad - all vectors must have concrete values");

        // ack_writer is the recipient (who writes the ACK)
        let computed = ack_aad(object_type, recipient_z32, sender_z32, recipient_z32, msg_id)
            .expect("Failed to compute ack_aad");

        assert_eq!(
            computed, expected_aad,
            "ACK AAD mismatch for {} {}: expected {}, got {}",
            object_type, msg_id, expected_aad, computed
        );
    }
}
