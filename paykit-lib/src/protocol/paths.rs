//! Canonical storage path builders for Paykit v0.
//!
//! These functions produce the exact paths used for storing and retrieving
//! Paykit objects on Pubky homeservers. All clients must use identical paths.
//!
//! ## Storage Layout (Paykit v0)
//!
//! | Object Type | Path |
//! |-------------|------|
//! | Payment Request | `/pub/paykit.app/v0/requests/{context_id}/{request_id}` |
//! | Subscription Proposal | `/pub/paykit.app/v0/subscriptions/proposals/{context_id}/{proposal_id}` |
//! | ACK | `/pub/paykit.app/v0/acks/{object_type}/{context_id}/{msg_id}` |
//! | Noise Endpoint | `/pub/paykit.app/v0/noise` |
//! | Secure Handoff | `/pub/paykit.app/v0/handoff/{request_id}` |
//!
//! ## ContextId Usage
//!
//! Per PUBKY_CRYPTO_SPEC v2.5, ContextId should be 32 random bytes chosen by the
//! thread initiator. The legacy pair-derived context_id is deprecated.
//!
//! - **New threads**: Use `generate_context_id()` and pass to `*_with_context_id()` functions
//! - **Legacy compatibility**: Use `payment_request_path()` which derives from peer pair

#[allow(deprecated)]
use super::scope::context_id;
use crate::Result;

/// Base path prefix for all Paykit v0 data.
pub const PAYKIT_V0_PREFIX: &str = "/pub/paykit.app/v0";

/// Path suffix for payment requests directory.
pub const REQUESTS_SUBPATH: &str = "requests";

/// Path suffix for subscription proposals directory.
pub const SUBSCRIPTION_PROPOSALS_SUBPATH: &str = "subscriptions/proposals";

/// Path for Noise endpoint.
pub const NOISE_ENDPOINT_SUBPATH: &str = "noise";

/// Path suffix for secure handoff directory.
pub const HANDOFF_SUBPATH: &str = "handoff";

/// Path suffix for ACKs directory.
pub const ACKS_SUBPATH: &str = "acks";

/// Build the storage path for a payment request.
///
/// Path format: `/pub/paykit.app/v0/requests/{context_id}/{request_id}`
///
/// This path is used on the **sender's** storage to store an encrypted
/// payment request addressed to the recipient.
///
/// # Arguments
///
/// * `sender_pubkey_z32` - The sender's z-base-32 encoded pubkey
/// * `recipient_pubkey_z32` - The recipient's z-base-32 encoded pubkey
/// * `request_id` - Unique identifier for this request
///
/// # Returns
///
/// The full storage path (without the `pubky://owner` prefix).
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::payment_request_path;
///
/// let path = payment_request_path(
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "abc123"
/// ).unwrap();
/// assert!(path.starts_with("/pub/paykit.app/v0/requests/"));
/// assert!(path.ends_with("/abc123"));
/// ```
#[allow(deprecated)]
pub fn payment_request_path(
    sender_pubkey_z32: &str,
    recipient_pubkey_z32: &str,
    request_id: &str,
) -> Result<String> {
    let ctx_id = context_id(sender_pubkey_z32, recipient_pubkey_z32)?;
    Ok(format!(
        "{}/{}/{}/{}",
        PAYKIT_V0_PREFIX, REQUESTS_SUBPATH, ctx_id, request_id
    ))
}

/// Build the directory path for listing payment requests between two peers.
///
/// Path format: `/pub/paykit.app/v0/requests/{context_id}/`
///
/// Used when polling a contact's storage to discover pending requests.
///
/// # Arguments
///
/// * `sender_pubkey_z32` - The sender's z-base-32 encoded pubkey
/// * `recipient_pubkey_z32` - The recipient's z-base-32 encoded pubkey
///
/// # Returns
///
/// The directory path (with trailing slash for listing).
#[allow(deprecated)]
pub fn payment_requests_dir(
    sender_pubkey_z32: &str,
    recipient_pubkey_z32: &str,
) -> Result<String> {
    let ctx_id = context_id(sender_pubkey_z32, recipient_pubkey_z32)?;
    Ok(format!(
        "{}/{}/{}/",
        PAYKIT_V0_PREFIX, REQUESTS_SUBPATH, ctx_id
    ))
}

/// Build the storage path for a subscription proposal.
///
/// Path format: `/pub/paykit.app/v0/subscriptions/proposals/{context_id}/{proposal_id}`
///
/// This path is used on the **provider's** storage to store an encrypted
/// subscription proposal addressed to the subscriber.
///
/// # Arguments
///
/// * `provider_pubkey_z32` - The provider's z-base-32 encoded pubkey
/// * `subscriber_pubkey_z32` - The subscriber's z-base-32 encoded pubkey
/// * `proposal_id` - Unique identifier for this proposal
///
/// # Returns
///
/// The full storage path (without the `pubky://owner` prefix).
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::subscription_proposal_path;
///
/// let path = subscription_proposal_path(
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "prop-456"
/// ).unwrap();
/// assert!(path.starts_with("/pub/paykit.app/v0/subscriptions/proposals/"));
/// assert!(path.ends_with("/prop-456"));
/// ```
#[allow(deprecated)]
pub fn subscription_proposal_path(
    provider_pubkey_z32: &str,
    subscriber_pubkey_z32: &str,
    proposal_id: &str,
) -> Result<String> {
    let ctx_id = context_id(provider_pubkey_z32, subscriber_pubkey_z32)?;
    Ok(format!(
        "{}/{}/{}/{}",
        PAYKIT_V0_PREFIX, SUBSCRIPTION_PROPOSALS_SUBPATH, ctx_id, proposal_id
    ))
}

/// Build the directory path for listing subscription proposals between two peers.
///
/// Path format: `/pub/paykit.app/v0/subscriptions/proposals/{context_id}/`
///
/// Used when polling a provider's storage to discover pending proposals.
///
/// # Arguments
///
/// * `provider_pubkey_z32` - The provider's z-base-32 encoded pubkey
/// * `subscriber_pubkey_z32` - The subscriber's z-base-32 encoded pubkey
///
/// # Returns
///
/// The directory path (with trailing slash for listing).
#[allow(deprecated)]
pub fn subscription_proposals_dir(
    provider_pubkey_z32: &str,
    subscriber_pubkey_z32: &str,
) -> Result<String> {
    let ctx_id = context_id(provider_pubkey_z32, subscriber_pubkey_z32)?;
    Ok(format!(
        "{}/{}/{}/",
        PAYKIT_V0_PREFIX, SUBSCRIPTION_PROPOSALS_SUBPATH, ctx_id
    ))
}

/// Build the storage path for a Noise endpoint.
///
/// Path format: `/pub/paykit.app/v0/noise`
///
/// This is a fixed path on the user's own storage.
pub fn noise_endpoint_path() -> &'static str {
    concat!("/pub/paykit.app/v0/", "noise")
}

/// Build the storage path for a secure handoff payload.
///
/// Path format: `/pub/paykit.app/v0/handoff/{request_id}`
///
/// This path is used on the Ring user's storage to temporarily store
/// an encrypted handoff payload for Bitkit to retrieve.
///
/// # Arguments
///
/// * `request_id` - Unique identifier for this handoff request
///
/// # Returns
///
/// The full storage path.
pub fn secure_handoff_path(request_id: &str) -> String {
    format!("{}/{}/{}", PAYKIT_V0_PREFIX, HANDOFF_SUBPATH, request_id)
}

/// Build the storage path for an ACK.
///
/// Path format: `/pub/paykit.app/v0/acks/{object_type}/{context_id}/{msg_id}`
///
/// This path is used on the **receiver's** storage to store an encrypted
/// ACK for the original sender to poll.
///
/// # Arguments
///
/// * `object_type` - Type of object being ACKed (e.g., "request", "subscription_proposal")
/// * `sender_pubkey_z32` - The original sender's z-base-32 encoded pubkey
/// * `recipient_pubkey_z32` - The recipient's z-base-32 encoded pubkey
/// * `msg_id` - The original message's identifier
///
/// # Returns
///
/// The full storage path (without the `pubky://owner` prefix).
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::ack_path;
///
/// let path = ack_path(
///     "request",
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "req_001"
/// ).unwrap();
/// assert!(path.starts_with("/pub/paykit.app/v0/acks/request/"));
/// assert!(path.ends_with("/req_001"));
/// ```
#[allow(deprecated)]
pub fn ack_path(
    object_type: &str,
    sender_pubkey_z32: &str,
    recipient_pubkey_z32: &str,
    msg_id: &str,
) -> Result<String> {
    let ctx_id = context_id(sender_pubkey_z32, recipient_pubkey_z32)?;
    Ok(format!(
        "{}/{}/{}/{}/{}",
        PAYKIT_V0_PREFIX, ACKS_SUBPATH, object_type, ctx_id, msg_id
    ))
}

// ============================================================================
// Random ContextId Path Builders (PUBKY_CRYPTO_SPEC v2.5)
// ============================================================================

/// Build the storage path for a payment request using a random ContextId.
///
/// Path format: `/pub/paykit.app/v0/requests/{context_id_hex}/{request_id}`
///
/// Per PUBKY_CRYPTO_SPEC v2.5, the ContextId should be 32 random bytes chosen
/// by the thread initiator. Use `generate_context_id_hex()` to create one.
///
/// # Arguments
///
/// * `context_id_hex` - Hex-encoded 32-byte random ContextId (64 chars)
/// * `request_id` - Unique identifier for this request
///
/// # Returns
///
/// The full storage path (without the `pubky://owner` prefix).
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::{generate_context_id_hex, payment_request_path_with_context_id};
///
/// let ctx_id = generate_context_id_hex();
/// let path = payment_request_path_with_context_id(&ctx_id, "req-123");
/// assert!(path.starts_with("/pub/paykit.app/v0/requests/"));
/// assert!(path.ends_with("/req-123"));
/// ```
pub fn payment_request_path_with_context_id(context_id_hex: &str, request_id: &str) -> String {
    format!(
        "{}/{}/{}/{}",
        PAYKIT_V0_PREFIX, REQUESTS_SUBPATH, context_id_hex, request_id
    )
}

/// Build the directory path for listing payment requests with a known ContextId.
///
/// Path format: `/pub/paykit.app/v0/requests/{context_id_hex}/`
///
/// Used when polling a contact's storage to discover pending requests.
pub fn payment_requests_dir_with_context_id(context_id_hex: &str) -> String {
    format!("{}/{}/{}/", PAYKIT_V0_PREFIX, REQUESTS_SUBPATH, context_id_hex)
}

/// Build the storage path for a subscription proposal using a random ContextId.
///
/// Path format: `/pub/paykit.app/v0/subscriptions/proposals/{context_id_hex}/{proposal_id}`
pub fn subscription_proposal_path_with_context_id(
    context_id_hex: &str,
    proposal_id: &str,
) -> String {
    format!(
        "{}/{}/{}/{}",
        PAYKIT_V0_PREFIX, SUBSCRIPTION_PROPOSALS_SUBPATH, context_id_hex, proposal_id
    )
}

/// Build the directory path for listing subscription proposals with a known ContextId.
///
/// Path format: `/pub/paykit.app/v0/subscriptions/proposals/{context_id_hex}/`
pub fn subscription_proposals_dir_with_context_id(context_id_hex: &str) -> String {
    format!(
        "{}/{}/{}/",
        PAYKIT_V0_PREFIX, SUBSCRIPTION_PROPOSALS_SUBPATH, context_id_hex
    )
}

/// Build the storage path for an ACK using a random ContextId.
///
/// Path format: `/pub/paykit.app/v0/acks/{object_type}/{context_id_hex}/{msg_id}`
pub fn ack_path_with_context_id(
    object_type: &str,
    context_id_hex: &str,
    msg_id: &str,
) -> String {
    format!(
        "{}/{}/{}/{}/{}",
        PAYKIT_V0_PREFIX, ACKS_SUBPATH, object_type, context_id_hex, msg_id
    )
}

/// Build the directory path for listing ACKs with a known ContextId.
///
/// Path format: `/pub/paykit.app/v0/acks/{object_type}/{context_id_hex}/`
pub fn acks_dir_with_context_id(object_type: &str, context_id_hex: &str) -> String {
    format!(
        "{}/{}/{}/{}/",
        PAYKIT_V0_PREFIX, ACKS_SUBPATH, object_type, context_id_hex
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER_PUBKEY: &str = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
    const RECIPIENT_PUBKEY: &str = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

    #[test]
    fn payment_request_path_format() {
        let path = payment_request_path(SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-123").unwrap();
        assert!(path.starts_with("/pub/paykit.app/v0/requests/"));
        assert!(path.ends_with("/req-123"));
        // Should contain a 64-char hex context_id between requests/ and /req-123
        let parts: Vec<&str> = path.split('/').collect();
        assert_eq!(parts.len(), 7); // ["", "pub", "paykit.app", "v0", "requests", context_id, "req-123"]
        assert_eq!(parts[5].len(), 64); // context_id is 64 hex chars
    }

    #[test]
    fn payment_requests_dir_format() {
        let dir = payment_requests_dir(SENDER_PUBKEY, RECIPIENT_PUBKEY).unwrap();
        assert!(dir.starts_with("/pub/paykit.app/v0/requests/"));
        assert!(dir.ends_with('/'));
    }

    #[test]
    fn subscription_proposal_path_format() {
        let path =
            subscription_proposal_path(SENDER_PUBKEY, RECIPIENT_PUBKEY, "prop-456").unwrap();
        assert!(path.starts_with("/pub/paykit.app/v0/subscriptions/proposals/"));
        assert!(path.ends_with("/prop-456"));
        let parts: Vec<&str> = path.split('/').collect();
        assert_eq!(parts.len(), 8); // ["", "pub", "paykit.app", "v0", "subscriptions", "proposals", context_id, "prop-456"]
        assert_eq!(parts[6].len(), 64); // context_id is 64 hex chars
    }

    #[test]
    fn subscription_proposals_dir_format() {
        let dir = subscription_proposals_dir(SENDER_PUBKEY, RECIPIENT_PUBKEY).unwrap();
        assert!(dir.starts_with("/pub/paykit.app/v0/subscriptions/proposals/"));
        assert!(dir.ends_with('/'));
    }

    #[test]
    fn noise_endpoint_path_is_fixed() {
        let path = noise_endpoint_path();
        assert_eq!(path, "/pub/paykit.app/v0/noise");
    }

    #[test]
    fn secure_handoff_path_format() {
        let path = secure_handoff_path("handoff-789");
        assert_eq!(path, "/pub/paykit.app/v0/handoff/handoff-789");
    }

    #[test]
    fn ack_path_format() {
        let path = ack_path("request", SENDER_PUBKEY, RECIPIENT_PUBKEY, "req_001").unwrap();
        assert!(path.starts_with("/pub/paykit.app/v0/acks/request/"));
        assert!(path.ends_with("/req_001"));
        let parts: Vec<&str> = path.split('/').collect();
        assert_eq!(parts.len(), 8); // ["", "pub", "paykit.app", "v0", "acks", "request", context_id, "req_001"]
        assert_eq!(parts[6].len(), 64); // context_id is 64 hex chars
    }

    #[test]
    fn context_id_is_symmetric_in_paths() {
        let path_ab = payment_request_path(SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-1").unwrap();
        let path_ba = payment_request_path(RECIPIENT_PUBKEY, SENDER_PUBKEY, "req-1").unwrap();

        // Extract context_id from both paths
        let ctx_ab = path_ab.split('/').nth(5).unwrap();
        let ctx_ba = path_ba.split('/').nth(5).unwrap();
        assert_eq!(ctx_ab, ctx_ba);
    }

    #[test]
    fn paths_differ_for_different_peer_pairs() {
        let third_pubkey = "o1gg96ewuojmopcjbz8895478wdtxtzzuxnfjjz8o8e77csa1ngo";

        let path1 = payment_request_path(SENDER_PUBKEY, RECIPIENT_PUBKEY, "req-1").unwrap();
        let path2 = payment_request_path(SENDER_PUBKEY, third_pubkey, "req-1").unwrap();

        let ctx1 = path1.split('/').nth(5).unwrap();
        let ctx2 = path2.split('/').nth(5).unwrap();
        assert_ne!(ctx1, ctx2);
    }
}
