//! Private endpoint storage for Web demo.

#[cfg(target_arch = "wasm32")]
mod indexeddb;
#[cfg(not(target_arch = "wasm32"))]
mod memory;

#[cfg(target_arch = "wasm32")]
pub use indexeddb::WasmPrivateEndpointStorage;
#[cfg(not(target_arch = "wasm32"))]
pub use memory::WasmPrivateEndpointStorage;
