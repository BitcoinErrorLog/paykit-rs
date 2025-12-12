//! Configuration types for payment executors.

use serde::{Deserialize, Serialize};

/// Bitcoin network selection.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BitcoinNetwork {
    /// Bitcoin mainnet.
    #[default]
    Mainnet,
    /// Bitcoin testnet (testnet3).
    Testnet,
    /// Bitcoin signet.
    Signet,
    /// Bitcoin regtest (local development).
    Regtest,
}

impl BitcoinNetwork {
    /// Get the network name as used by most APIs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mainnet => "mainnet",
            Self::Testnet => "testnet",
            Self::Signet => "signet",
            Self::Regtest => "regtest",
        }
    }

    /// Get the bech32 HRP (human-readable part) for addresses.
    pub fn address_prefix(&self) -> &'static str {
        match self {
            Self::Mainnet => "bc",
            Self::Testnet | Self::Signet | Self::Regtest => "tb",
        }
    }
}

/// Configuration for LND REST API executor.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LndConfig {
    /// REST API endpoint URL (e.g., "https://localhost:8080").
    pub rest_url: String,

    /// Macaroon for authentication (hex-encoded).
    pub macaroon_hex: String,

    /// TLS certificate (PEM format, optional for self-signed).
    pub tls_cert_pem: Option<String>,

    /// Network the node is on.
    #[serde(default)]
    pub network: BitcoinNetwork,

    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Maximum fee allowed as percentage of payment amount.
    #[serde(default = "default_max_fee_percent")]
    pub max_fee_percent: f64,
}

fn default_timeout() -> u64 {
    30
}

fn default_max_fee_percent() -> f64 {
    1.0 // 1% max fee
}

impl LndConfig {
    /// Create a new LND configuration.
    pub fn new(rest_url: impl Into<String>, macaroon_hex: impl Into<String>) -> Self {
        Self {
            rest_url: rest_url.into(),
            macaroon_hex: macaroon_hex.into(),
            tls_cert_pem: None,
            network: BitcoinNetwork::default(),
            timeout_secs: default_timeout(),
            max_fee_percent: default_max_fee_percent(),
        }
    }

    /// Set the TLS certificate.
    pub fn with_tls_cert(mut self, cert_pem: impl Into<String>) -> Self {
        self.tls_cert_pem = Some(cert_pem.into());
        self
    }

    /// Set the network.
    pub fn with_network(mut self, network: BitcoinNetwork) -> Self {
        self.network = network;
        self
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set maximum fee percentage.
    pub fn with_max_fee_percent(mut self, percent: f64) -> Self {
        self.max_fee_percent = percent;
        self
    }
}

/// Configuration for Electrum server executor.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ElectrumConfig {
    /// Electrum server URL (e.g., "ssl://electrum.blockstream.info:50002").
    pub server_url: String,

    /// Network the server is on.
    #[serde(default)]
    pub network: BitcoinNetwork,

    /// Connection timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Number of retry attempts.
    #[serde(default = "default_retries")]
    pub retries: u32,
}

fn default_retries() -> u32 {
    3
}

impl ElectrumConfig {
    /// Create a new Electrum configuration.
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            network: BitcoinNetwork::default(),
            timeout_secs: default_timeout(),
            retries: default_retries(),
        }
    }

    /// Set the network.
    pub fn with_network(mut self, network: BitcoinNetwork) -> Self {
        self.network = network;
        self
    }
}

/// Configuration for Esplora block explorer API executor.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EsploraConfig {
    /// API base URL (e.g., `https://blockstream.info/api`).
    pub api_url: String,

    /// Network the explorer is on.
    #[serde(default)]
    pub network: BitcoinNetwork,

    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Rate limiting: minimum delay between requests in milliseconds.
    #[serde(default = "default_rate_limit")]
    pub rate_limit_ms: u64,
}

fn default_rate_limit() -> u64 {
    100 // 100ms between requests
}

impl EsploraConfig {
    /// Create a new Esplora configuration.
    pub fn new(api_url: impl Into<String>) -> Self {
        Self {
            api_url: api_url.into(),
            network: BitcoinNetwork::default(),
            timeout_secs: default_timeout(),
            rate_limit_ms: default_rate_limit(),
        }
    }

    /// Create config for Blockstream mainnet.
    pub fn blockstream_mainnet() -> Self {
        Self::new("https://blockstream.info/api").with_network(BitcoinNetwork::Mainnet)
    }

    /// Create config for Blockstream testnet.
    pub fn blockstream_testnet() -> Self {
        Self::new("https://blockstream.info/testnet/api").with_network(BitcoinNetwork::Testnet)
    }

    /// Create config for mempool.space mainnet.
    pub fn mempool_mainnet() -> Self {
        Self::new("https://mempool.space/api").with_network(BitcoinNetwork::Mainnet)
    }

    /// Create config for mempool.space testnet.
    pub fn mempool_testnet() -> Self {
        Self::new("https://mempool.space/testnet/api").with_network(BitcoinNetwork::Testnet)
    }

    /// Set the network.
    pub fn with_network(mut self, network: BitcoinNetwork) -> Self {
        self.network = network;
        self
    }

    /// Set the timeout.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// Unified executor configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExecutorConfig {
    /// LND REST API for Lightning.
    Lnd(LndConfig),
    /// Electrum for on-chain.
    Electrum(ElectrumConfig),
    /// Esplora for on-chain.
    Esplora(EsploraConfig),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lnd_config() {
        let config = LndConfig::new("https://localhost:8080", "0201...")
            .with_network(BitcoinNetwork::Testnet)
            .with_timeout(60);

        assert_eq!(config.rest_url, "https://localhost:8080");
        assert_eq!(config.network, BitcoinNetwork::Testnet);
        assert_eq!(config.timeout_secs, 60);
    }

    #[test]
    fn test_esplora_presets() {
        let mainnet = EsploraConfig::blockstream_mainnet();
        assert_eq!(mainnet.network, BitcoinNetwork::Mainnet);
        assert!(mainnet.api_url.contains("blockstream.info"));

        let testnet = EsploraConfig::mempool_testnet();
        assert_eq!(testnet.network, BitcoinNetwork::Testnet);
        assert!(testnet.api_url.contains("mempool.space"));
    }

    #[test]
    fn test_network_address_prefix() {
        assert_eq!(BitcoinNetwork::Mainnet.address_prefix(), "bc");
        assert_eq!(BitcoinNetwork::Testnet.address_prefix(), "tb");
    }
}
