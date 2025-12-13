//! Testnet and development network configurations.
//!
//! This module provides preconfigured executor settings for various Bitcoin
//! test networks, making it easy to run demos and integration tests.
//!
//! # Environment Variables
//!
//! All configurations can be overridden via environment variables:
//!
//! ## LND Configuration
//! - `PAYKIT_LND_URL` - LND REST API URL (e.g., `https://localhost:8080`)
//! - `PAYKIT_LND_MACAROON` - LND macaroon in hex format
//! - `PAYKIT_LND_TLS_CERT` - LND TLS certificate in PEM format (optional)
//!
//! ## Esplora Configuration
//! - `PAYKIT_ESPLORA_URL` - Esplora API URL
//!
//! ## Network Selection
//! - `PAYKIT_NETWORK` - Network name: `mainnet`, `testnet`, `signet`, or `regtest`
//!
//! # Example
//!
//! ```rust,ignore
//! use paykit_lib::executors::testnet::{TestnetConfig, get_lnd_config_from_env};
//!
//! // Use preset configuration
//! let preset = TestnetConfig::polar_regtest();
//!
//! // Or load from environment
//! if let Some(config) = get_lnd_config_from_env() {
//!     let executor = LndExecutor::new(config)?;
//! }
//! ```

use super::config::{BitcoinNetwork, EsploraConfig, LndConfig};

/// Preconfigured testnet settings.
///
/// Provides ready-to-use configurations for common development setups.
#[derive(Clone, Debug)]
pub struct TestnetConfig {
    /// LND configuration (if available).
    pub lnd: Option<LndConfig>,
    /// Esplora configuration.
    pub esplora: EsploraConfig,
    /// Network type.
    pub network: BitcoinNetwork,
}

impl TestnetConfig {
    /// Configuration for Polar regtest environment.
    ///
    /// Polar is a popular Lightning Network development tool that runs
    /// a local regtest network with LND, CLN, and Bitcoin Core nodes.
    ///
    /// Default Polar LND ports:
    /// - Alice: 8081
    /// - Bob: 8082
    /// - Carol: 8083
    ///
    /// Note: You'll need to provide the macaroon separately.
    pub fn polar_regtest() -> Self {
        Self {
            lnd: None, // Macaroon required
            esplora: EsploraConfig::new("http://localhost:3002/api")
                .with_network(BitcoinNetwork::Regtest),
            network: BitcoinNetwork::Regtest,
        }
    }

    /// Configuration for Polar with LND Alice node.
    ///
    /// # Arguments
    ///
    /// * `macaroon_hex` - The admin macaroon in hex format
    pub fn polar_alice(macaroon_hex: impl Into<String>) -> Self {
        Self {
            lnd: Some(
                LndConfig::new("https://localhost:8081", macaroon_hex)
                    .with_network(BitcoinNetwork::Regtest)
                    .with_timeout(60),
            ),
            esplora: EsploraConfig::new("http://localhost:3002/api")
                .with_network(BitcoinNetwork::Regtest),
            network: BitcoinNetwork::Regtest,
        }
    }

    /// Configuration for Bitcoin testnet3 with public APIs.
    ///
    /// Uses Blockstream's testnet Esplora API. LND is not included
    /// as there's no public testnet LND node.
    pub fn testnet3() -> Self {
        Self {
            lnd: None,
            esplora: EsploraConfig::blockstream_testnet(),
            network: BitcoinNetwork::Testnet,
        }
    }

    /// Configuration for Bitcoin testnet with mempool.space API.
    pub fn testnet_mempool() -> Self {
        Self {
            lnd: None,
            esplora: EsploraConfig::mempool_testnet(),
            network: BitcoinNetwork::Testnet,
        }
    }

    /// Configuration for Bitcoin signet with mempool.space API.
    pub fn signet() -> Self {
        Self {
            lnd: None,
            esplora: EsploraConfig::new("https://mempool.space/signet/api")
                .with_network(BitcoinNetwork::Signet),
            network: BitcoinNetwork::Signet,
        }
    }

    /// Configuration for Mutinynet (signet-based Lightning testnet).
    ///
    /// Mutinynet is a public signet with Lightning support.
    pub fn mutinynet() -> Self {
        Self {
            lnd: None, // Would need custom LND setup
            esplora: EsploraConfig::new("https://mutinynet.com/api")
                .with_network(BitcoinNetwork::Signet),
            network: BitcoinNetwork::Signet,
        }
    }

    /// Add LND configuration.
    pub fn with_lnd(mut self, config: LndConfig) -> Self {
        self.lnd = Some(config);
        self
    }

    /// Update Esplora configuration.
    pub fn with_esplora(mut self, config: EsploraConfig) -> Self {
        self.esplora = config;
        self
    }
}

// ============================================================================
// Environment Variable Loading
// ============================================================================

/// Load LND configuration from environment variables.
///
/// Required variables:
/// - `PAYKIT_LND_URL` - REST API URL
/// - `PAYKIT_LND_MACAROON` - Macaroon in hex format
///
/// Optional variables:
/// - `PAYKIT_LND_TLS_CERT` - TLS certificate in PEM format
/// - `PAYKIT_LND_TIMEOUT` - Request timeout in seconds (default: 30)
/// - `PAYKIT_LND_MAX_FEE_PERCENT` - Max fee as percentage (default: 1.0)
/// - `PAYKIT_NETWORK` - Network name
///
/// # Example
///
/// ```bash
/// export PAYKIT_LND_URL=https://localhost:8080
/// export PAYKIT_LND_MACAROON=0201036c6e6402...
/// export PAYKIT_NETWORK=testnet
/// ```
pub fn get_lnd_config_from_env() -> Option<LndConfig> {
    let url = std::env::var("PAYKIT_LND_URL").ok()?;
    let macaroon = std::env::var("PAYKIT_LND_MACAROON").ok()?;

    let mut config = LndConfig::new(url, macaroon);

    if let Ok(cert) = std::env::var("PAYKIT_LND_TLS_CERT") {
        config = config.with_tls_cert(cert);
    }

    if let Ok(timeout) = std::env::var("PAYKIT_LND_TIMEOUT") {
        if let Ok(secs) = timeout.parse::<u64>() {
            config = config.with_timeout(secs);
        }
    }

    if let Ok(max_fee) = std::env::var("PAYKIT_LND_MAX_FEE_PERCENT") {
        if let Ok(percent) = max_fee.parse::<f64>() {
            config = config.with_max_fee_percent(percent);
        }
    }

    if let Some(network) = get_network_from_env() {
        config = config.with_network(network);
    }

    Some(config)
}

/// Load Esplora configuration from environment variables.
///
/// Variables:
/// - `PAYKIT_ESPLORA_URL` - API base URL (falls back to testnet Blockstream if not set)
/// - `PAYKIT_ESPLORA_TIMEOUT` - Request timeout in seconds (default: 30)
/// - `PAYKIT_NETWORK` - Network name
///
/// # Example
///
/// ```bash
/// export PAYKIT_ESPLORA_URL=https://mempool.space/testnet/api
/// export PAYKIT_NETWORK=testnet
/// ```
pub fn get_esplora_config_from_env() -> EsploraConfig {
    let url = std::env::var("PAYKIT_ESPLORA_URL")
        .unwrap_or_else(|_| "https://blockstream.info/testnet/api".to_string());

    let mut config = EsploraConfig::new(url);

    if let Ok(timeout) = std::env::var("PAYKIT_ESPLORA_TIMEOUT") {
        if let Ok(secs) = timeout.parse::<u64>() {
            config = config.with_timeout(secs);
        }
    }

    if let Some(network) = get_network_from_env() {
        config = config.with_network(network);
    }

    config
}

/// Get network from environment variable.
///
/// Reads `PAYKIT_NETWORK` and parses it as a network name.
pub fn get_network_from_env() -> Option<BitcoinNetwork> {
    std::env::var("PAYKIT_NETWORK")
        .ok()
        .and_then(|s| parse_network(&s))
}

/// Parse a network name string.
fn parse_network(s: &str) -> Option<BitcoinNetwork> {
    match s.to_lowercase().as_str() {
        "mainnet" | "main" => Some(BitcoinNetwork::Mainnet),
        "testnet" | "testnet3" | "test" => Some(BitcoinNetwork::Testnet),
        "signet" | "sig" => Some(BitcoinNetwork::Signet),
        "regtest" | "reg" | "local" => Some(BitcoinNetwork::Regtest),
        _ => None,
    }
}

/// Load a complete testnet configuration from environment.
///
/// Combines LND (if available) and Esplora configurations.
pub fn get_testnet_config_from_env() -> TestnetConfig {
    let network = get_network_from_env().unwrap_or(BitcoinNetwork::Testnet);

    TestnetConfig {
        lnd: get_lnd_config_from_env(),
        esplora: get_esplora_config_from_env(),
        network,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polar_regtest_config() {
        let config = TestnetConfig::polar_regtest();
        assert!(config.lnd.is_none());
        assert_eq!(config.network, BitcoinNetwork::Regtest);
        assert!(config.esplora.api_url.contains("localhost"));
    }

    #[test]
    fn test_polar_alice_config() {
        let config = TestnetConfig::polar_alice("0201036c6e6402");
        assert!(config.lnd.is_some());
        let lnd = config.lnd.unwrap();
        assert!(lnd.rest_url.contains("8081"));
        assert_eq!(lnd.network, BitcoinNetwork::Regtest);
    }

    #[test]
    fn test_testnet3_config() {
        let config = TestnetConfig::testnet3();
        assert!(config.lnd.is_none());
        assert_eq!(config.network, BitcoinNetwork::Testnet);
        assert!(config.esplora.api_url.contains("testnet"));
    }

    #[test]
    fn test_signet_config() {
        let config = TestnetConfig::signet();
        assert_eq!(config.network, BitcoinNetwork::Signet);
        assert!(config.esplora.api_url.contains("signet"));
    }

    #[test]
    fn test_parse_network() {
        assert_eq!(parse_network("mainnet"), Some(BitcoinNetwork::Mainnet));
        assert_eq!(parse_network("MAINNET"), Some(BitcoinNetwork::Mainnet));
        assert_eq!(parse_network("testnet"), Some(BitcoinNetwork::Testnet));
        assert_eq!(parse_network("testnet3"), Some(BitcoinNetwork::Testnet));
        assert_eq!(parse_network("signet"), Some(BitcoinNetwork::Signet));
        assert_eq!(parse_network("regtest"), Some(BitcoinNetwork::Regtest));
        assert_eq!(parse_network("local"), Some(BitcoinNetwork::Regtest));
        assert_eq!(parse_network("invalid"), None);
    }

    #[test]
    fn test_with_lnd() {
        let config = TestnetConfig::testnet3()
            .with_lnd(LndConfig::new("https://localhost:8080", "macaroon"));
        assert!(config.lnd.is_some());
    }
}
