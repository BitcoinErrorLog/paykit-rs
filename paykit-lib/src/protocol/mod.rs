//! Canonical Paykit v0 protocol conventions.
//!
//! This module defines the single source of truth for:
//! - Pubkey normalization and scope hashing
//! - Storage path construction
//! - AAD (Additional Authenticated Data) formats for Sealed Blob v1
//!
//! All Paykit clients (Rust, Kotlin, Swift) must implement equivalent logic
//! and pass the same test vectors.
//!
//! # Path Layout (v0)
//!
//! | Object Type          | Path Template                                                    | Stored On       |
//! |----------------------|------------------------------------------------------------------|-----------------|
//! | Supported payment    | `/pub/paykit.app/v0/{method_id}`                                 | payee           |
//! | Noise endpoint       | `/pub/paykit.app/v0/noise`                                       | payee           |
//! | Payment request      | `/pub/paykit.app/v0/requests/{recipient_scope}/{request_id}`     | sender          |
//! | Subscription proposal| `/pub/paykit.app/v0/subscriptions/proposals/{subscriber_scope}/{proposal_id}` | provider |
//! | Secure handoff       | `/pub/paykit.app/v0/handoff/{request_id}`                        | Ring user       |
//!
//! # Scope Derivation
//!
//! `scope = hex(sha256(utf8(normalize(pubkey_z32))))`
//!
//! This creates a per-recipient (or per-subscriber) directory that:
//! - Is deterministic and collision-resistant
//! - Doesn't leak the recipient pubkey in the path
//! - Works across all platforms (no z32 decode required)

mod aad;
mod paths;
mod scope;

pub use aad::*;
pub use paths::*;
pub use scope::*;

/// Protocol version string.
pub const PROTOCOL_VERSION: &str = "v0";
