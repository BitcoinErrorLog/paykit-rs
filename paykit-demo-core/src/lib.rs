//! Paykit Demo Core Library
//!
//! Shared business logic for all Paykit demo applications (CLI, Web, Desktop).
//! This crate provides identity management, directory operations, payment flows,
//! subscription management, and storage abstraction.
//!
//! # Architecture
//!
//! This library sits between the protocol crates and demo applications:
//!
//! ```text
//! Demo Apps (CLI, Web, Desktop)
//!       ↓
//! paykit-demo-core (this crate)
//!       ↓
//! Protocol Layer:
//!   - paykit-lib (directory & transport)
//!   - paykit-interactive (payments)
//!   - paykit-subscriptions (recurring payments)
//!   - pubky-noise (encryption)
//! ```
//!
//! # Security Warning
//!
//! This is **demo code** for development and testing. Key security considerations:
//! - Private keys stored in plaintext JSON files
//! - No encryption at rest
//! - No OS-level secure storage
//! - Simplified error handling
//!
//! For production use, implement proper key management, secure storage,
//! and authentication mechanisms.

pub mod directory;
pub mod identity;
pub mod models;
pub mod noise_client;
pub mod noise_server;
pub mod payment;
pub mod session;
pub mod storage;
pub mod subscription;

pub use directory::DirectoryClient;
pub use identity::{Identity, IdentityManager};
pub use models::{Contact, PaymentMethod, Receipt};
pub use noise_client::NoiseClientHelper;
pub use noise_server::NoiseServerHelper;
pub use payment::{DemoPaykitStorage, DemoReceiptGenerator, PaymentCoordinator};
pub use session::SessionManager;
pub use storage::DemoStorage;
pub use subscription::{DemoPaymentRequest, DemoSubscription, SubscriptionCoordinator};

/// Result type for demo operations
pub type Result<T> = anyhow::Result<T>;
