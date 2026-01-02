//! AAD (Additional Authenticated Data) builders for Sealed Blob v1.
//!
//! AAD binds the ciphertext to its storage context, preventing relocation attacks.
//! All Paykit clients must use identical AAD formats.
//!
//! # AAD Format
//!
//! `paykit:v0:{type}:{path}:{id}`
//!
//! Where:
//! - `type` is the object type (e.g., "request", "subscription_proposal", "handoff")
//! - `path` is the full storage path
//! - `id` is the object identifier

use super::paths::{payment_request_path, secure_handoff_path, subscription_proposal_path};
use crate::Result;

/// AAD prefix for all Paykit v0 sealed blobs.
pub const AAD_PREFIX: &str = "paykit:v0";

/// Purpose label for payment requests.
pub const PURPOSE_REQUEST: &str = "request";

/// Purpose label for subscription proposals.
pub const PURPOSE_SUBSCRIPTION_PROPOSAL: &str = "subscription_proposal";

/// Purpose label for secure handoff payloads.
pub const PURPOSE_HANDOFF: &str = "handoff";

/// Build AAD for a payment request.
///
/// Format: `paykit:v0:request:{path}:{request_id}`
///
/// # Arguments
///
/// * `recipient_pubkey_z32` - The recipient's z-base-32 encoded pubkey
/// * `request_id` - Unique identifier for this request
///
/// # Returns
///
/// The AAD string to use with Sealed Blob v1 encryption.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::payment_request_aad;
///
/// let aad = payment_request_aad(
///     "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///     "req-123"
/// ).unwrap();
/// assert!(aad.starts_with("paykit:v0:request:"));
/// ```
pub fn payment_request_aad(recipient_pubkey_z32: &str, request_id: &str) -> Result<String> {
    let path = payment_request_path(recipient_pubkey_z32, request_id)?;
    Ok(format!(
        "{}:{}:{}:{}",
        AAD_PREFIX, PURPOSE_REQUEST, path, request_id
    ))
}

/// Build AAD for a subscription proposal.
///
/// Format: `paykit:v0:subscription_proposal:{path}:{proposal_id}`
///
/// # Arguments
///
/// * `subscriber_pubkey_z32` - The subscriber's z-base-32 encoded pubkey
/// * `proposal_id` - Unique identifier for this proposal
///
/// # Returns
///
/// The AAD string to use with Sealed Blob v1 encryption.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::subscription_proposal_aad;
///
/// let aad = subscription_proposal_aad(
///     "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
///     "prop-456"
/// ).unwrap();
/// assert!(aad.starts_with("paykit:v0:subscription_proposal:"));
/// ```
pub fn subscription_proposal_aad(subscriber_pubkey_z32: &str, proposal_id: &str) -> Result<String> {
    let path = subscription_proposal_path(subscriber_pubkey_z32, proposal_id)?;
    Ok(format!(
        "{}:{}:{}:{}",
        AAD_PREFIX, PURPOSE_SUBSCRIPTION_PROPOSAL, path, proposal_id
    ))
}

/// Build AAD for a secure handoff payload.
///
/// Format: `paykit:v0:handoff:{path}:{request_id}`
///
/// Note: For handoff, the path includes the owner pubkey in practice,
/// but for simplicity we use the fixed handoff path structure.
///
/// # Arguments
///
/// * `owner_pubkey_z32` - The Ring user's z-base-32 encoded pubkey (for path context)
/// * `request_id` - Unique identifier for this handoff
///
/// # Returns
///
/// The AAD string to use with Sealed Blob v1 encryption.
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
    // Include owner in AAD for additional binding
    format!(
        "{}:{}:{}:{}:{}",
        AAD_PREFIX, PURPOSE_HANDOFF, owner_pubkey_z32, path, request_id
    )
}

/// Build AAD from explicit path and ID.
///
/// This is the low-level builder for cases where you already have the path.
///
/// Format: `paykit:v0:{purpose}:{path}:{id}`
///
/// # Arguments
///
/// * `purpose` - The object type (use constants like `PURPOSE_REQUEST`)
/// * `path` - The full storage path
/// * `id` - The object identifier
pub fn build_aad(purpose: &str, path: &str, id: &str) -> String {
    format!("{}:{}:{}:{}", AAD_PREFIX, purpose, path, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PUBKEY: &str = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";

    #[test]
    fn payment_request_aad_format() {
        let aad = payment_request_aad(TEST_PUBKEY, "req-123").unwrap();
        assert!(aad.starts_with("paykit:v0:request:/pub/paykit.app/v0/requests/"));
        assert!(aad.ends_with(":req-123"));
    }

    #[test]
    fn subscription_proposal_aad_format() {
        let aad = subscription_proposal_aad(TEST_PUBKEY, "prop-456").unwrap();
        assert!(aad.starts_with(
            "paykit:v0:subscription_proposal:/pub/paykit.app/v0/subscriptions/proposals/"
        ));
        assert!(aad.ends_with(":prop-456"));
    }

    #[test]
    fn secure_handoff_aad_format() {
        let aad = secure_handoff_aad(TEST_PUBKEY, "handoff-789");
        assert!(aad.starts_with("paykit:v0:handoff:"));
        assert!(aad.contains(TEST_PUBKEY));
        assert!(aad.contains("/pub/paykit.app/v0/handoff/handoff-789"));
        assert!(aad.ends_with(":handoff-789"));
    }

    #[test]
    fn build_aad_produces_correct_format() {
        let aad = build_aad("custom", "/some/path", "id-123");
        assert_eq!(aad, "paykit:v0:custom:/some/path:id-123");
    }

    #[test]
    fn aad_is_deterministic() {
        let aad1 = payment_request_aad(TEST_PUBKEY, "req-123").unwrap();
        let aad2 = payment_request_aad(TEST_PUBKEY, "req-123").unwrap();
        assert_eq!(aad1, aad2);
    }

    #[test]
    fn aad_differs_for_different_ids() {
        let aad1 = payment_request_aad(TEST_PUBKEY, "req-123").unwrap();
        let aad2 = payment_request_aad(TEST_PUBKEY, "req-456").unwrap();
        assert_ne!(aad1, aad2);
    }

    #[test]
    fn aad_differs_for_different_recipients() {
        let pubkey2 = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
        let aad1 = payment_request_aad(TEST_PUBKEY, "req-123").unwrap();
        let aad2 = payment_request_aad(pubkey2, "req-123").unwrap();
        assert_ne!(aad1, aad2);
    }
}
