//! Paykit Demo Core Library
//!
//! Shared business logic for all Paykit demo applications (CLI, Web, Desktop).
//! This crate provides identity management, directory operations, payment flows,
//! and storage abstraction.

pub mod directory;
pub mod identity;
pub mod models;
pub mod payment;
pub mod storage;

pub use directory::DirectoryClient;
pub use identity::{Identity, IdentityManager};
pub use models::{Contact, PaymentMethod, Receipt};
pub use payment::PaymentCoordinator;
pub use storage::DemoStorage;

/// Result type for demo operations
pub type Result<T> = anyhow::Result<T>;
