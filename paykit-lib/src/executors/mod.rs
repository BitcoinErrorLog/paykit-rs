//! Real payment executor implementations.
//!
//! This module provides production-ready executor implementations
//! for Bitcoin on-chain and Lightning Network payments.
//!
//! ## Feature Flags
//!
//! The `http-executor` feature flag must be enabled for actual HTTP requests:
//!
//! ```toml
//! [dependencies]
//! paykit-lib = { version = "1.0", features = ["http-executor"] }
//! ```
//!
//! ## Supported Backends
//!
//! ### Lightning
//! - **LND REST API** - Connect to LND nodes via REST
//! - **CLN (Core Lightning)** - Coming soon
//!
//! ### On-chain
//! - **Electrum** - Connect to Electrum servers (planned)
//! - **Esplora** - Block explorer API (Blockstream, mempool.space)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use paykit_lib::executors::{LndConfig, LndExecutor, EsploraConfig, EsploraExecutor};
//! use paykit_lib::methods::LightningExecutor;
//!
//! // Lightning via LND
//! let lnd_config = LndConfig::new("https://localhost:8080", "macaroon_hex");
//! let ln_executor = LndExecutor::new(lnd_config)?;
//!
//! // Decode an invoice
//! let decoded = ln_executor.decode_invoice("lnbc...").await?;
//! println!("Amount: {:?}", decoded.amount_msat);
//!
//! // On-chain via Esplora
//! let btc_executor = EsploraExecutor::blockstream_testnet()?;
//!
//! // Get fee estimates
//! let fees = btc_executor.get_fee_estimates().await?;
//! println!("Next block: {} sat/vB", fees.get_rate_for_blocks(1));
//! ```
//!
//! ## Testnet Configuration
//!
//! For development and testing, use the `testnet` module:
//!
//! ```rust,ignore
//! use paykit_lib::executors::testnet::{TestnetConfig, get_lnd_config_from_env};
//!
//! // Preset for Polar regtest
//! let config = TestnetConfig::polar_alice("macaroon_hex");
//!
//! // Or load from environment variables
//! let config = get_lnd_config_from_env();
//! ```

mod config;
mod esplora;
mod lnd;
pub mod testnet;

pub use config::{BitcoinNetwork, ElectrumConfig, EsploraConfig, ExecutorConfig, LndConfig};
pub use esplora::{
    AddressInfo, AddressStats, EsploraExecutor, EsploraTx, EsploraTxInput, EsploraTxOutput,
    FeeEstimates, TxStatus, Utxo,
};
pub use lnd::LndExecutor;
