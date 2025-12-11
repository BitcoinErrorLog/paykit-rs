//! Real payment executor implementations.
//!
//! This module provides production-ready executor implementations
//! for Bitcoin on-chain and Lightning Network payments.
//!
//! ## Supported Backends
//!
//! ### Lightning
//! - **LND REST API** - Connect to LND nodes via REST
//! - **CLN (Core Lightning)** - Coming soon
//!
//! ### On-chain
//! - **Electrum** - Connect to Electrum servers
//! - **Esplora** - Block explorer API (Blockstream, mempool.space)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use paykit_lib::executors::{LndConfig, LndExecutor, EsploraConfig, EsploraExecutor};
//!
//! // Lightning via LND
//! let lnd_config = LndConfig::new("https://localhost:8080", "macaroon_hex");
//! let ln_executor = LndExecutor::new(lnd_config)?;
//!
//! // On-chain via Esplora
//! let esplora_config = EsploraConfig::new("https://blockstream.info/api");
//! let btc_executor = EsploraExecutor::new(esplora_config);
//! ```

mod config;
mod esplora;
mod lnd;

pub use config::{BitcoinNetwork, ElectrumConfig, EsploraConfig, ExecutorConfig, LndConfig};
pub use esplora::EsploraExecutor;
pub use lnd::LndExecutor;
