//! AAD (Additional Authenticated Data) builders for Sealed Blob v2.
//!
//! AAD binds the ciphertext to its storage context and owner, preventing relocation attacks.
//! All Paykit clients must use identical AAD formats.
//!
//! # AAD Format (Owner-Bound)
//!
//! `paykit:v0:{purpose}:{owner_z32}:{path}:{id}`
//!
//! Where:
//! - `purpose` is the object type (e.g., "request", "subscription_proposal", "handoff", "ack_request")
//! - `owner_z32` is the normalized z-base-32 pubkey of the storage owner
//! - `path` is the full storage path
//! - `id` is the object identifier

use super::paths::{ack_path, payment_request_path, secure_handoff_path, subscription_proposal_path};
use crate::Result;

/// AAD prefix for all Paykit v0 sealed blobs.
pub const AAD_PREFIX: &str = "paykit:v0";

/// Purpose label for payment requests.
pub const PURPOSE_REQUEST: &str = "request";

/// Purpose label for subscription proposals.
pub const PURPOSE_SUBSCRIPTION_PROPOSAL: &str = "subscription_proposal";

/// Purpose label for secure handoff payloads.
pub const PURPOSE_HANDOFF: &str = "handoff";

/// Purpose label for ACK (acknowledgment) messages.
pub const PURPOSE_ACK: &str = "ack";

/// Build AAD for a payment request.
///
/// Format: `paykit:v0:request:{owner_z32}:{path}:{request_id}`
///
/// # Arguments
///
/// * `owner_pubkey_z32` - The storage owner's z-base-32 encoded pubkey (sender)
/// * `sender_pubkey_z32` - The sender's z-base-32 encoded pubkey
/// * `recipient_pubkey_z32` - The recipient's z-base-32 encoded pubkey
/// * `request_id` - Unique identifier for this request
///
/// # Returns
///
/// The AAD string to use with Sealed Blob v2 encryption.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::payment_request_aad;
///
/// let aad = payment_request_aad(
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "req-123"
/// ).unwrap();
/// assert!(aad.starts_with("paykit:v0:request:"));
/// ```
pub fn payment_request_aad(
    owner_pubkey_z32: &str,
    sender_pubkey_z32: &str,
    recipient_pubkey_z32: &str,
    request_id: &str,
) -> Result<String> {
    let path = payment_request_path(sender_pubkey_z32, recipient_pubkey_z32, request_id)?;
    Ok(format!(
        "{}:{}:{}:{}:{}",
        AAD_PREFIX, PURPOSE_REQUEST, owner_pubkey_z32, path, request_id
    ))
}

/// Build AAD for a subscription proposal.
///
/// Format: `paykit:v0:subscription_proposal:{owner_z32}:{path}:{proposal_id}`
///
/// # Arguments
///
/// * `owner_pubkey_z32` - The storage owner's z-base-32 encoded pubkey (provider)
/// * `provider_pubkey_z32` - The provider's z-base-32 encoded pubkey
/// * `subscriber_pubkey_z32` - The subscriber's z-base-32 encoded pubkey
/// * `proposal_id` - Unique identifier for this proposal
///
/// # Returns
///
/// The AAD string to use with Sealed Blob v2 encryption.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::subscription_proposal_aad;
///
/// let aad = subscription_proposal_aad(
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "prop-456"
/// ).unwrap();
/// assert!(aad.starts_with("paykit:v0:subscription_proposal:"));
/// ```
pub fn subscription_proposal_aad(
    owner_pubkey_z32: &str,
    provider_pubkey_z32: &str,
    subscriber_pubkey_z32: &str,
    proposal_id: &str,
) -> Result<String> {
    let path = subscription_proposal_path(provider_pubkey_z32, subscriber_pubkey_z32, proposal_id)?;
    Ok(format!(
        "{}:{}:{}:{}:{}",
        AAD_PREFIX, PURPOSE_SUBSCRIPTION_PROPOSAL, owner_pubkey_z32, path, proposal_id
    ))
}

/// Build AAD for a secure handoff payload.
///
/// Format: `paykit:v0:handoff:{owner_z32}:{path}:{request_id}`
///
/// # Arguments
///
/// * `owner_pubkey_z32` - The Ring user's z-base-32 encoded pubkey
/// * `request_id` - Unique identifier for this handoff
///
/// # Returns
///
/// The AAD string to use with Sealed Blob v2 encryption.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::secure_handoff_aad;
///
/// let aad = secure_handoff_aad(
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "handoff-789"
/// );
/// assert!(aad.starts_with("paykit:v0:handoff:"));
/// ```
pub fn secure_handoff_aad(owner_pubkey_z32: &str, request_id: &str) -> String {
    let path = secure_handoff_path(request_id);
    format!(
        "{}:{}:{}:{}:{}",
        AAD_PREFIX, PURPOSE_HANDOFF, owner_pubkey_z32, path, request_id
    )
}

/// Build AAD for an ACK.
///
/// Format: `paykit:v0:ack_{object_type}:{ack_writer_z32}:{path}:{msg_id}`
///
/// # Arguments
///
/// * `object_type` - Type of object being ACKed (e.g., "request", "subscription_proposal")
/// * `ack_writer_pubkey_z32` - The ACK writer's z-base-32 encoded pubkey (receiver)
/// * `sender_pubkey_z32` - The original sender's z-base-32 encoded pubkey
/// * `recipient_pubkey_z32` - The recipient's z-base-32 encoded pubkey
/// * `msg_id` - The original message's identifier
///
/// # Returns
///
/// The AAD string to use with Sealed Blob v2 encryption.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::ack_aad;
///
/// let aad = ack_aad(
///     "request",
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "req_001"
/// ).unwrap();
/// assert!(aad.starts_with("paykit:v0:ack_request:"));
/// ```
pub fn ack_aad(
    object_type: &str,
    ack_writer_pubkey_z32: &str,
    sender_pubkey_z32: &str,
    recipient_pubkey_z32: &str,
    msg_id: &str,
) -> Result<String> {
    let path = ack_path(object_type, sender_pubkey_z32, recipient_pubkey_z32, msg_id)?;
    Ok(format!(
        "{}:ack_{}:{}:{}:{}",
        AAD_PREFIX, object_type, ack_writer_pubkey_z32, path, msg_id
    ))
}

/// Build AAD from explicit owner, path and ID.
///
/// This is the low-level builder for cases where you already have the path.
///
/// Format: `paykit:v0:{purpose}:{owner_z32}:{path}:{id}`
///
/// # Arguments
///
/// * `purpose` - The object type (use constants like `PURPOSE_REQUEST`)
/// * `owner` - The storage owner's z-base-32 pubkey
/// * `path` - The full storage path
/// * `id` - The object identifier
pub fn build_aad(purpose: &str, owner: &str, path: &str, id: &str) -> String {
    format!("{}:{}:{}:{}:{}", AAD_PREFIX, purpose, owner, path, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER_PUBKEY: &str = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
    const RECIPIENT_PUBKEY: &str = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

    #[test]
    fn payment_request_aad_format() {
        let aad =
            payment_request_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123")
                .unwrap();
        assert!(aad.starts_with("paykit:v0:request:"));
        assert!(aad.contains(SENDER_PUBKEY));
        assert!(aad.contains("/pub/paykit.app/v0/requests/"));
        assert!(aad.ends_with(":req-123"));
    }

    #[test]
    fn subscription_proposal_aad_format() {
        let aad = subscription_proposal_aad(
            SENDER_PUBKEY,
            SENDER_PUBKEY,
            RECIPIENT_PUBKEY,
            "prop-456",
        )
        .unwrap();
        assert!(aad.starts_with("paykit:v0:subscription_proposal:"));
        assert!(aad.contains(SENDER_PUBKEY));
        assert!(aad.contains("/pub/paykit.app/v0/subscriptions/proposals/"));
        assert!(aad.ends_with(":prop-456"));
    }

    #[test]
    fn secure_handoff_aad_format() {
        let aad = secure_handoff_aad(SENDER_PUBKEY, "handoff-789");
        assert!(aad.starts_with("paykit:v0:handoff:"));
        assert!(aad.contains(SENDER_PUBKEY));
        assert!(aad.contains("/pub/paykit.app/v0/handoff/handoff-789"));
        assert!(aad.ends_with(":handoff-789"));
    }

    #[test]
    fn ack_aad_format() {
        let aad =
            ack_aad("request", RECIPIENT_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req_001")
                .unwrap();
        assert!(aad.starts_with("paykit:v0:ack_request:"));
        assert!(aad.contains(RECIPIENT_PUBKEY));
        assert!(aad.contains("/pub/paykit.app/v0/acks/request/"));
        assert!(aad.ends_with(":req_001"));
    }

    #[test]
    fn build_aad_produces_correct_format() {
        let aad = build_aad("custom", "owner123", "/some/path", "id-123");
        assert_eq!(aad, "paykit:v0:custom:owner123:/some/path:id-123");
    }

    #[test]
    fn aad_is_deterministic() {
        let aad1 =
            payment_request_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123")
                .unwrap();
        let aad2 =
            payment_request_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123")
                .unwrap();
        assert_eq!(aad1, aad2);
    }

    #[test]
    fn aad_differs_for_different_ids() {
        let aad1 =
            payment_request_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123")
                .unwrap();
        let aad2 =
            payment_request_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-456")
                .unwrap();
        assert_ne!(aad1, aad2);
    }

    #[test]
    fn aad_differs_for_different_owners() {
        let aad1 =
            payment_request_aad(SENDER_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123")
                .unwrap();
        let aad2 =
            payment_request_aad(RECIPIENT_PUBKEY, SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123")
                .unwrap();
        assert_ne!(aad1, aad2);
    }
}
