//! Canonical storage path builders for Paykit v0.
//!
//! These functions produce the exact paths used for storing and retrieving
//! Paykit objects on Pubky homeservers. All clients must use identical paths.

use super::scope::{recipient_scope, subscriber_scope};
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

/// Build the storage path for a payment request.
///
/// Path format: `/pub/paykit.app/v0/requests/{recipient_scope}/{request_id}`
///
/// This path is used on the **sender's** storage to store an encrypted
/// payment request addressed to the recipient.
///
/// # Arguments
///
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
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "abc123"
/// ).unwrap();
/// assert!(path.starts_with("/pub/paykit.app/v0/requests/"));
/// assert!(path.ends_with("/abc123"));
/// ```
pub fn payment_request_path(recipient_pubkey_z32: &str, request_id: &str) -> Result<String> {
    let scope = recipient_scope(recipient_pubkey_z32)?;
    Ok(format!(
        "{}/{}/{}/{}",
        PAYKIT_V0_PREFIX, REQUESTS_SUBPATH, scope, request_id
    ))
}

/// Build the directory path for listing payment requests for a recipient.
///
/// Path format: `/pub/paykit.app/v0/requests/{recipient_scope}/`
///
/// Used when polling a contact's storage to discover pending requests.
///
/// # Arguments
///
/// * `recipient_pubkey_z32` - The recipient's z-base-32 encoded pubkey
///
/// # Returns
///
/// The directory path (with trailing slash for listing).
pub fn payment_requests_dir(recipient_pubkey_z32: &str) -> Result<String> {
    let scope = recipient_scope(recipient_pubkey_z32)?;
    Ok(format!(
        "{}/{}/{}/",
        PAYKIT_V0_PREFIX, REQUESTS_SUBPATH, scope
    ))
}

/// Build the storage path for a subscription proposal.
///
/// Path format: `/pub/paykit.app/v0/subscriptions/proposals/{subscriber_scope}/{proposal_id}`
///
/// This path is used on the **provider's** storage to store an encrypted
/// subscription proposal addressed to the subscriber.
///
/// # Arguments
///
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
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "prop-456"
/// ).unwrap();
/// assert!(path.starts_with("/pub/paykit.app/v0/subscriptions/proposals/"));
/// assert!(path.ends_with("/prop-456"));
/// ```
pub fn subscription_proposal_path(
    subscriber_pubkey_z32: &str,
    proposal_id: &str,
) -> Result<String> {
    let scope = subscriber_scope(subscriber_pubkey_z32)?;
    Ok(format!(
        "{}/{}/{}/{}",
        PAYKIT_V0_PREFIX, SUBSCRIPTION_PROPOSALS_SUBPATH, scope, proposal_id
    ))
}

/// Build the directory path for listing subscription proposals for a subscriber.
///
/// Path format: `/pub/paykit.app/v0/subscriptions/proposals/{subscriber_scope}/`
///
/// Used when polling a provider's storage to discover pending proposals.
///
/// # Arguments
///
/// * `subscriber_pubkey_z32` - The subscriber's z-base-32 encoded pubkey
///
/// # Returns
///
/// The directory path (with trailing slash for listing).
pub fn subscription_proposals_dir(subscriber_pubkey_z32: &str) -> Result<String> {
    let scope = subscriber_scope(subscriber_pubkey_z32)?;
    Ok(format!(
        "{}/{}/{}/",
        PAYKIT_V0_PREFIX, SUBSCRIPTION_PROPOSALS_SUBPATH, scope
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

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PUBKEY: &str = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";

    #[test]
    fn payment_request_path_format() {
        let path = payment_request_path(TEST_PUBKEY, "req-123").unwrap();
        assert!(path.starts_with("/pub/paykit.app/v0/requests/"));
        assert!(path.ends_with("/req-123"));
        // Should contain a 64-char hex scope between requests/ and /req-123
        let parts: Vec<&str> = path.split('/').collect();
        assert_eq!(parts.len(), 7); // ["", "pub", "paykit.app", "v0", "requests", scope, "req-123"]
        assert_eq!(parts[5].len(), 64); // scope is 64 hex chars
    }

    #[test]
    fn payment_requests_dir_format() {
        let dir = payment_requests_dir(TEST_PUBKEY).unwrap();
        assert!(dir.starts_with("/pub/paykit.app/v0/requests/"));
        assert!(dir.ends_with('/'));
    }

    #[test]
    fn subscription_proposal_path_format() {
        let path = subscription_proposal_path(TEST_PUBKEY, "prop-456").unwrap();
        assert!(path.starts_with("/pub/paykit.app/v0/subscriptions/proposals/"));
        assert!(path.ends_with("/prop-456"));
        let parts: Vec<&str> = path.split('/').collect();
        assert_eq!(parts.len(), 8); // ["", "pub", "paykit.app", "v0", "subscriptions", "proposals", scope, "prop-456"]
        assert_eq!(parts[6].len(), 64); // scope is 64 hex chars
    }

    #[test]
    fn subscription_proposals_dir_format() {
        let dir = subscription_proposals_dir(TEST_PUBKEY).unwrap();
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
    fn paths_are_consistent_for_same_pubkey() {
        let path1 = payment_request_path(TEST_PUBKEY, "req-1").unwrap();
        let path2 = payment_request_path(TEST_PUBKEY, "req-2").unwrap();

        // Extract scope from both paths
        let scope1 = path1.split('/').nth(5).unwrap();
        let scope2 = path2.split('/').nth(5).unwrap();
        assert_eq!(scope1, scope2);
    }

    #[test]
    fn paths_differ_for_different_pubkeys() {
        let pubkey2 = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

        let path1 = payment_request_path(TEST_PUBKEY, "req-1").unwrap();
        let path2 = payment_request_path(pubkey2, "req-1").unwrap();

        let scope1 = path1.split('/').nth(5).unwrap();
        let scope2 = path2.split('/').nth(5).unwrap();
        assert_ne!(scope1, scope2);
    }
}
