//! Payment Method Selection
//!
//! This module provides automatic payment method selection based on:
//! - Available methods from the payee
//! - Payment amount
//! - User preferences (cost, speed, privacy)
//! - Method constraints (limits, confirmation times)
//!
//! # Example
//!
//! ```ignore
//! use paykit_lib::selection::{PaymentMethodSelector, SelectionPreferences};
//! use paykit_lib::methods::Amount;
//! use paykit_lib::SupportedPayments;
//!
//! let selector = PaymentMethodSelector::with_defaults();
//! let supported = get_payee_methods().await?;
//! let amount = Amount::sats(10000);
//! let prefs = SelectionPreferences::balanced();
//!
//! let result = selector.select(&supported, &amount, &prefs)?;
//! println!("Selected: {} (reason: {})", result.primary.0, result.reason);
//!
//! // Execute payment with primary method, fallback on failure
//! for method in result.all_methods() {
//!     match execute_payment(&method, &amount).await {
//!         Ok(_) => break,
//!         Err(_) => continue, // Try next fallback
//!     }
//! }
//! ```
//!
//! # Selection Strategies
//!
//! - **Balanced**: Default strategy that considers cost, speed, and privacy
//! - **CostOptimized**: Minimizes transaction fees
//! - **SpeedOptimized**: Prioritizes fastest confirmation
//! - **PrivacyOptimized**: Maximizes privacy (prefers off-chain)
//! - **PriorityList**: Uses methods in user-specified order

mod preferences;
mod selector;

pub use preferences::{AmountThresholds, SelectionPreferences, SelectionStrategy};
pub use selector::{PaymentMethodSelector, SelectionResult};
