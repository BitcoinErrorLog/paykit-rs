//! Testing utilities for Paykit demo applications.
//!
//! This module provides mock implementations of transport traits that enable
//! reliable CI/CD testing without external network dependencies.
//!
//! # Usage
//!
//! ```rust
//! use paykit_demo_core::testing::{MockStorage, MockAuthenticatedTransport};
//!
//! // Create a mock transport with in-memory storage
//! let storage = MockStorage::new();
//! let transport = MockAuthenticatedTransport::new(storage.clone(), "owner");
//!
//! // Use transport in tests without network calls
//! ```

pub mod mock_transport;

pub use mock_transport::{MockAuthenticatedTransport, MockStorage, MockUnauthenticatedTransport};

