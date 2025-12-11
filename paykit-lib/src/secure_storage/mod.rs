//! Secure key storage abstraction for cross-platform key management.
//!
//! This module provides a platform-agnostic trait for secure key storage,
//! with implementations for:
//! - In-memory storage (for testing)
//! - iOS Keychain (via FFI)
//! - Android Keystore (via FFI)
//! - Web SubtleCrypto (via wasm-bindgen)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use paykit_lib::secure_storage::{SecureKeyStorage, InMemoryKeyStorage};
//!
//! let storage = InMemoryKeyStorage::new();
//!
//! // Store a secret key
//! storage.store("my-key-id", b"secret-key-data").await?;
//!
//! // Retrieve the key
//! if let Some(key) = storage.retrieve("my-key-id").await? {
//!     // Use the key...
//! }
//!
//! // Delete when done
//! storage.delete("my-key-id").await?;
//! ```
//!
//! ## Security Considerations
//!
//! - Keys are stored with platform-specific encryption
//! - Access control via biometrics/device unlock where supported
//! - Keys are zeroized from memory after use where possible
//! - No keys are ever logged or serialized to unprotected storage

mod memory;
mod traits;

#[cfg(target_os = "ios")]
mod ios;

#[cfg(target_os = "android")]
mod android;

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(all(
    not(target_os = "ios"),
    not(target_os = "android"),
    not(target_arch = "wasm32")
))]
mod desktop;

pub use memory::InMemoryKeyStorage;
pub use traits::{
    KeyMetadata, SecureKeyStorage, SecureStorageError, SecureStorageErrorCode, SecureStorageResult,
};

#[cfg(target_os = "ios")]
pub use ios::KeychainStorage;

#[cfg(target_os = "android")]
pub use android::KeystoreStorage;

#[cfg(target_arch = "wasm32")]
pub use web::WebCryptoStorage;

#[cfg(all(
    not(target_os = "ios"),
    not(target_os = "android"),
    not(target_arch = "wasm32")
))]
pub use desktop::DesktopKeyStorage;
