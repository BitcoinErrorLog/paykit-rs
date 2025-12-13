//! Secure key storage abstraction for cross-platform key management.
//!
//! This module provides a platform-agnostic trait for secure key storage,
//! with implementations for:
//!
//! - **In-memory storage** (`InMemoryKeyStorage`): For testing only, NOT secure
//! - **Desktop platforms** (`DesktopKeyStorage`):
//!   - **macOS**: Keychain Services (via security-framework crate)
//!   - **Windows**: Windows Credential Manager (via windows crate)
//!   - **Linux**: Secret Service API (via secret-service crate)
//! - **iOS** (`KeychainStorage`): Keychain (via FFI)
//! - **Android** (`KeystoreStorage`): Android Keystore (via FFI)
//! - **Web** (`WebCryptoStorage`): SubtleCrypto (via wasm-bindgen)
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
//! ## Desktop Secure Storage
//!
//! On desktop platforms, `DesktopKeyStorage` uses OS-native secure storage:
//!
//! ```rust,ignore
//! use paykit_lib::secure_storage::{DesktopKeyStorage, SecureKeyStorage};
//!
//! // Create storage with app identifier
//! let storage = DesktopKeyStorage::new("com.example.myapp");
//!
//! // Store securely in OS keychain/credential manager
//! storage.store_simple("wallet-key", &secret_key_bytes).await?;
//!
//! // For testing, use fallback-only mode (NOT secure!)
//! let test_storage = DesktopKeyStorage::new("test").with_fallback_only();
//! ```
//!
//! ## Security Considerations
//!
//! - Keys are stored with platform-specific encryption via OS keychain APIs
//! - Access control via biometrics/device unlock where supported by the OS
//! - Keys are zeroized from memory after use where possible
//! - No keys are ever logged or serialized to unprotected storage
//! - Fallback in-memory storage is provided for testing but is NOT secure

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
    KeyMetadata, SecureKeyStorage, SecureKeyStorageExt, SecureStorageError, SecureStorageErrorCode,
    SecureStorageResult, StoreOptions,
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
