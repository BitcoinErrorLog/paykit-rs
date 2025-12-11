//! Test fixtures and data generators.

use std::time::{SystemTime, UNIX_EPOCH};

/// Collection of commonly used test fixtures.
pub struct TestFixtures;

impl TestFixtures {
    /// Valid mainnet P2WPKH addresses for testing.
    pub const MAINNET_ADDRESSES: &'static [&'static str] = &[
        "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
        "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",
        "bc1q7efa5rjlceuzy34z8g7u7xnr9k6hqfmt9xz9y2",
    ];

    /// Valid testnet P2WPKH addresses for testing.
    pub const TESTNET_ADDRESSES: &'static [&'static str] = &[
        "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx",
        "tb1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q0sl5k7",
    ];

    /// Valid LNURL examples for testing.
    pub const LNURLS: &'static [&'static str] = &[
        "lnurl1dp68gurn8ghj7um9wfmxjcm99e3k7mf0v9cxj0m385ekvcenxc6r2c35xvukxefcv5mkvv34x5ekzd3ev56nyd3hxqurvctk",
    ];

    /// Sample payment amounts in satoshis.
    pub const SAMPLE_AMOUNTS: &'static [u64] = &[
        100,       // Tiny (dust for on-chain)
        1_000,     // Small
        10_000,    // Medium
        100_000,   // Large
        1_000_000, // Very large (0.01 BTC)
    ];

    /// Get a random mainnet address.
    pub fn mainnet_address() -> &'static str {
        let idx = (current_timestamp() as usize) % Self::MAINNET_ADDRESSES.len();
        Self::MAINNET_ADDRESSES[idx]
    }

    /// Get a random testnet address.
    pub fn testnet_address() -> &'static str {
        let idx = (current_timestamp() as usize) % Self::TESTNET_ADDRESSES.len();
        Self::TESTNET_ADDRESSES[idx]
    }

    /// Get a sample amount.
    pub fn sample_amount(index: usize) -> u64 {
        Self::SAMPLE_AMOUNTS[index % Self::SAMPLE_AMOUNTS.len()]
    }
}

/// Create a deterministic test keypair from a seed string.
///
/// Returns (secret_key_hex, public_key_hex).
pub fn create_test_keypair(seed: &str) -> (String, String) {
    // Simple deterministic key generation for testing
    let hash = simple_hash(seed);
    let sk = format!("{:064x}", hash);

    // Derive a mock public key (not cryptographically valid)
    let pk_hash = simple_hash(&sk);
    let pk = format!("{:066x}", pk_hash);

    (sk, pk)
}

/// Generate a random-looking payment hash.
pub fn random_payment_hash() -> String {
    let seed = format!("payment_hash_{}", current_timestamp());
    format!("{:064x}", simple_hash(&seed))
}

/// Generate a random-looking preimage.
pub fn random_preimage() -> String {
    let seed = format!("preimage_{}", current_timestamp());
    format!("{:064x}", simple_hash(&seed))
}

/// Generate a test invoice string.
///
/// Note: This is not a valid BOLT11 invoice, just for testing parsing logic.
pub fn test_invoice(amount_msat: u64, description: &str) -> String {
    format!(
        "lnbc{}n1test{}",
        amount_msat / 1000,
        simple_hash(description) % 10000
    )
}

/// Get a test address for the specified network.
pub fn test_address(mainnet: bool) -> &'static str {
    if mainnet {
        TestFixtures::mainnet_address()
    } else {
        TestFixtures::testnet_address()
    }
}

/// Simple hash function for deterministic test data.
fn simple_hash(data: &str) -> u128 {
    let mut hash: u128 = 5381;
    for byte in data.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u128);
    }
    hash
}

/// Simple hash function returning u64.
pub fn simple_hash_u64(data: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in data.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

/// Get current timestamp in seconds.
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixtures_addresses() {
        assert!(!TestFixtures::MAINNET_ADDRESSES.is_empty());
        assert!(!TestFixtures::TESTNET_ADDRESSES.is_empty());

        // All mainnet addresses should start with "bc1"
        for addr in TestFixtures::MAINNET_ADDRESSES {
            assert!(addr.starts_with("bc1"));
        }

        // All testnet addresses should start with "tb1"
        for addr in TestFixtures::TESTNET_ADDRESSES {
            assert!(addr.starts_with("tb1"));
        }
    }

    #[test]
    fn test_create_keypair() {
        let (sk1, pk1) = create_test_keypair("alice");
        let (sk2, pk2) = create_test_keypair("bob");
        let (sk3, pk3) = create_test_keypair("alice");

        // Different seeds produce different keys
        assert_ne!(sk1, sk2);
        assert_ne!(pk1, pk2);

        // Same seed produces same keys (deterministic)
        assert_eq!(sk1, sk3);
        assert_eq!(pk1, pk3);
    }

    #[test]
    fn test_payment_hash_format() {
        let hash = random_payment_hash();
        assert_eq!(hash.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_test_invoice() {
        let invoice = test_invoice(100_000_000, "test payment");
        assert!(invoice.starts_with("lnbc"));
    }
}
