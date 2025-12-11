//! Endpoint Rotation Management
//!
//! This module provides automatic endpoint rotation for privacy and security.
//! Payment endpoints (addresses, invoices) can be rotated based on configurable policies.
//!
//! # Rotation Policies
//!
//! - **RotateOnUse**: Rotate immediately after each use (best privacy)
//! - **RotateOnThreshold**: Rotate after N uses
//! - **RotatePeriodic**: Rotate on a time interval
//! - **Manual**: No automatic rotation
//!
//! # Example
//!
//! ```ignore
//! use paykit_lib::rotation::{EndpointRotationManager, RotationConfig, RotationPolicy};
//! use paykit_lib::MethodId;
//!
//! // Configure rotation policies
//! let config = RotationConfig::default()
//!     .set_policy(MethodId("onchain".into()), RotationPolicy::after_uses(5))
//!     .set_policy(MethodId("lightning".into()), RotationPolicy::RotateOnUse);
//!
//! let manager = EndpointRotationManager::new(config, registry);
//!
//! // Set initial endpoint
//! manager.set_endpoint(&MethodId("onchain".into()), endpoint);
//!
//! // After payment execution, record usage and rotate if needed
//! if let Some(new_endpoint) = manager.on_payment_executed(&method_id).await {
//!     // Endpoint was rotated, update directory
//!     set_payment_endpoint(transport, method_id, new_endpoint).await?;
//! }
//! ```
//!
//! # Privacy Considerations
//!
//! - On-chain: Rotate addresses after each use to prevent address reuse
//! - Lightning: Rotate invoices for each payment (typically automatic)
//! - Private endpoints: Use unique endpoints per peer

mod manager;
mod policies;

pub use manager::{EndpointRotationManager, RotationCallback, RotationConfig};
pub use policies::{EndpointTracker, RotationPolicy};
