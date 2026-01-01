#![cfg(target_arch = "wasm32")]
//! Paykit Web Demo - WebAssembly bindings
//!
//! This crate provides JavaScript bindings for Paykit functionality,
//! enabling browser-based demonstrations.

use wasm_bindgen::prelude::*;

mod contacts;
mod dashboard;
mod directory;
mod identity;
mod payment;
mod payment_methods;
mod storage;
mod storage_migration;
mod private_endpoints;
mod subscriptions;
mod types;
mod utils;
mod wasm_transport;
mod websocket_transport;

pub use contacts::*;
pub use dashboard::*;
pub use directory::*;
pub use identity::{Identity, WasmKeyProvider};
pub use payment::*;
pub use payment_methods::*;
pub use storage::*;
pub use storage_migration::*;
pub use subscriptions::*;
pub use types::*;
pub use utils::{is_valid_public_key, parse_uri, to_paykit_uri, to_pubky_uri, ParsedUri};
pub use wasm_transport::WasmUnauthenticatedTransport;
pub use websocket_transport::*;
pub use private_endpoints::WasmPrivateEndpointStorage;

/// Initialize the WASM module
///
/// This should be called once when the module is loaded.
/// It sets up panic hooks for better error messages in the browser console.
#[wasm_bindgen(start)]
pub fn init() {
    utils::set_panic_hook();
    utils::log("Paykit WASM module initialized");
}

/// Get the version of the Paykit WASM module
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
