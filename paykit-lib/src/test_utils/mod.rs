//! Test utilities for Paykit.
//!
//! This module provides comprehensive testing infrastructure including:
//! - Mock executors with configurable behavior
//! - Test fixtures for common scenarios
//! - Assertion helpers for payment verification
//! - Simulated payment network for E2E testing
//!
//! ## Usage
//!
//! ```rust,ignore
//! use paykit_lib::test_utils::{TestNetwork, TestWallet, PaymentScenario};
//!
//! // Create a test network with two wallets
//! let network = TestNetwork::new();
//! let alice = network.create_wallet("alice");
//! let bob = network.create_wallet("bob");
//!
//! // Fund Alice's wallet
//! alice.fund(100_000); // 100k sats
//!
//! // Execute a payment
//! let result = alice.pay(&bob, 10_000).await?;
//! assert!(result.is_success());
//! ```

mod assertions;
mod fixtures;
mod mock_network;

pub use fixtures::{
    create_test_keypair, random_payment_hash, random_preimage, test_address, test_invoice,
    TestFixtures,
};

pub use mock_network::{MockChannel, NetworkConfig, TestNetwork, TestWallet};

pub use assertions::{
    assert_address_valid, assert_invoice_valid, assert_payment_failed, assert_payment_succeeded,
    PaymentAssertion,
};
