//! Canonical Paykit v0 protocol conventions.
//!
//! This module defines the single source of truth for:
//! - Pubkey normalization and ContextId derivation
//! - Storage path construction
//! - AAD (Additional Authenticated Data) formats for Sealed Blob v2
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
//! | Payment request      | `/pub/paykit.app/v0/requests/{context_id}/{request_id}`          | sender          |
//! | Subscription proposal| `/pub/paykit.app/v0/subscriptions/proposals/{context_id}/{proposal_id}` | provider |
//! | ACK                  | `/pub/paykit.app/v0/acks/{object_type}/{context_id}/{msg_id}`    | receiver        |
//! | Secure handoff       | `/pub/paykit.app/v0/handoff/{request_id}`                        | Ring user       |
//!
//! # ContextId Derivation
//!
//! `context_id = hex(sha256("paykit:v0:context:" + first_z32 + ":" + second_z32))`
//!
//! Where `first_z32` and `second_z32` are normalized pubkeys sorted lexicographically.
//!
//! ContextId is symmetric: `context_id(A, B) == context_id(B, A)`
//!
//! This creates a per-peer-pair directory that:
//! - Is deterministic and collision-resistant
//! - Doesn't leak either pubkey in the path
//! - Works across all platforms (no z32 decode required)
//! - Enables symmetric discovery from either party

mod aad;
mod ack;
mod paths;
mod sb2;
mod scope;

pub use aad::*;
pub use ack::*;
pub use paths::*;
pub use sb2::*;
pub use scope::*;

/// Protocol version string.
pub const PROTOCOL_VERSION: &str = "v0";
