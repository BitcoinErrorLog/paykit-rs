//! Property-based tests for paykit-demo-cli
//!
//! Uses proptest to verify properties hold for arbitrary inputs

use paykit_demo_cli::commands::pay::{extract_pubkey_from_uri, parse_noise_endpoint};
use proptest::prelude::*;

proptest! {
    /// Test that valid hex strings of length 64 parse correctly as Noise endpoints
    #[test]
    fn test_noise_endpoint_with_valid_hex(
        port in 1024u16..65535u16,
        hex_bytes in prop::collection::vec(any::<u8>(), 32..=32)
    ) {
        let hex_str = hex::encode(hex_bytes);
        let endpoint = format!("noise://127.0.0.1:{}@{}", port, hex_str);

        let result = parse_noise_endpoint(&endpoint);
        prop_assert!(result.is_ok(), "Valid endpoint should parse: {}", endpoint);

        let (host, pk) = result.unwrap();
        prop_assert_eq!(host, format!("127.0.0.1:{}", port));
        prop_assert_eq!(pk.len(), 32);
    }

    /// Test that endpoints without @ symbol fail
    #[test]
    fn test_noise_endpoint_missing_separator(
        port in 1024u16..65535u16
    ) {
        let endpoint = format!("noise://127.0.0.1:{}", port);
        let result = parse_noise_endpoint(&endpoint);
        prop_assert!(result.is_err(), "Endpoint without @ should fail");
    }

    /// Test that endpoints with invalid hex fail
    #[test]
    fn test_noise_endpoint_invalid_hex(
        port in 1024u16..65535u16,
        invalid_hex in "[g-z]+"  // Characters not in hex
    ) {
        let endpoint = format!("noise://127.0.0.1:{}@{}", port, invalid_hex);
        let result = parse_noise_endpoint(&endpoint);
        prop_assert!(result.is_err(), "Invalid hex should fail");
    }

    /// Test that endpoints with wrong-length hex fail
    #[test]
    fn test_noise_endpoint_wrong_length(
        port in 1024u16..65535u16,
        len in 1usize..60usize
    ) {
        // Skip len==32 which is valid
        prop_assume!(len != 32);

        let hex_str = "a".repeat(len * 2); // Hex is 2 chars per byte
        let endpoint = format!("noise://127.0.0.1:{}@{}", port, hex_str);

        let result = parse_noise_endpoint(&endpoint);
        if len != 32 {
            prop_assert!(result.is_err(), "Wrong-length hex should fail");
        }
    }

    /// Test that pubky:// URIs with valid z32 keys parse correctly
    #[test]
    fn test_pubkey_uri_parsing(
        key_bytes in prop::collection::vec(any::<u8>(), 32..=32)
    ) {
        // Convert to z32 (base32) encoding like Pubky does
        use pubky::Keypair;
        let keypair = Keypair::from_secret_key(&key_bytes.try_into().unwrap());
        let pubkey = keypair.public_key();
        let uri = format!("pubky://{}", pubkey);

        let result = extract_pubkey_from_uri(&uri);
        prop_assert!(result.is_ok(), "Valid pubky URI should parse");
    }

    /// Test that pubkey extraction works with and without prefix
    #[test]
    fn test_pubkey_prefix_handling(
        with_prefix in prop::bool::ANY
    ) {
        // Use a known valid key
        let keypair = pubky::Keypair::random();
        let pubkey = keypair.public_key();

        let uri = if with_prefix {
            format!("pubky://{}", pubkey)
        } else {
            pubkey.to_string()
        };

        let result = extract_pubkey_from_uri(&uri);
        prop_assert!(result.is_ok(), "Should parse with or without prefix");
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_noise_endpoint_localhost_variants() {
        let test_cases = vec![
            "noise://localhost:9735@0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "noise://127.0.0.1:9735@0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "noise://0.0.0.0:9735@0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ];

        for endpoint in test_cases {
            let result = parse_noise_endpoint(endpoint);
            assert!(result.is_ok(), "Should parse: {}", endpoint);
        }
    }

    #[test]
    fn test_noise_endpoint_edge_cases() {
        // Empty hex
        assert!(parse_noise_endpoint("noise://127.0.0.1:9735@").is_err());

        // Missing scheme
        assert!(parse_noise_endpoint("127.0.0.1:9735@abc123").is_err());

        // Wrong scheme
        assert!(parse_noise_endpoint("http://127.0.0.1:9735@abc123").is_err());
    }

    #[test]
    fn test_pubkey_uri_edge_cases() {
        // Empty string
        assert!(extract_pubkey_from_uri("").is_err());

        // Just prefix
        assert!(extract_pubkey_from_uri("pubky://").is_err());

        // Invalid characters
        assert!(extract_pubkey_from_uri("pubky://!!!invalid!!!").is_err());
    }
}
