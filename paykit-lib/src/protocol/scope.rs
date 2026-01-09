//! Pubkey normalization and scope hashing.
//!
//! The `scope` is a per-recipient directory hash used in storage paths
//! to avoid leaking the recipient's pubkey while remaining deterministic.

use crate::{PaykitError, Result};
use sha2::{Digest, Sha256};

/// Valid characters in z-base-32 encoding (lowercase only).
const Z32_ALPHABET: &str = "ybndrfg8ejkmcpqxot1uwisza345h769";

/// Expected length of a z-base-32 encoded Ed25519 public key (256 bits / 5 bits per char).
const Z32_PUBKEY_LENGTH: usize = 52;

/// Normalize a z-base-32 pubkey string.
///
/// Performs:
/// 1. Trim whitespace
/// 2. Strip `pubky://` prefix if present
/// 3. Strip `pk:` prefix if present
/// 4. Lowercase
/// 5. Validate length (52 chars) and alphabet
///
/// # Errors
///
/// Returns `PaykitError::InvalidData` if the pubkey is malformed.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::normalize_pubkey_z32;
///
/// let normalized = normalize_pubkey_z32("pubky://YBNDRFG8EJKMCPQXOT1UWISZA345H769YBNDRFG8EJKMCPQXOT1U").unwrap();
/// assert_eq!(normalized.len(), 52);
/// assert!(normalized.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
/// ```
pub fn normalize_pubkey_z32(pubkey: &str) -> Result<String> {
    let trimmed = pubkey.trim();

    // Strip pubky:// prefix if present
    let without_pubky = trimmed.strip_prefix("pubky://").unwrap_or(trimmed);

    // Strip pk: prefix if present
    let without_prefix = without_pubky.strip_prefix("pk:").unwrap_or(without_pubky);

    // Lowercase
    let lowercased = without_prefix.to_ascii_lowercase();

    // Validate length
    if lowercased.len() != Z32_PUBKEY_LENGTH {
        return Err(PaykitError::InvalidData {
            field: "pubkey".into(),
            reason: format!(
                "z32 pubkey must be {} chars, got {}",
                Z32_PUBKEY_LENGTH,
                lowercased.len()
            ),
        });
    }

    // Validate alphabet
    for c in lowercased.chars() {
        if !Z32_ALPHABET.contains(c) {
            return Err(PaykitError::InvalidData {
                field: "pubkey".into(),
                reason: format!("invalid z32 character: '{}'", c),
            });
        }
    }

    Ok(lowercased)
}

/// Compute ContextId for a peer pair (symmetric).
///
/// Formula: `hex(sha256("paykit:v0:context:" + first_z32 + ":" + second_z32))`
///
/// ContextId is symmetric: `context_id(A, B) == context_id(B, A)`
///
/// # Arguments
///
/// * `pubkey_a` - First peer's z-base-32 pubkey
/// * `pubkey_b` - Second peer's z-base-32 pubkey
///
/// # Returns
///
/// Lowercase hex string (64 chars) representing the SHA-256 hash.
///
/// # Errors
///
/// Returns `PaykitError::InvalidData` if either pubkey is malformed.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::context_id;
///
/// let ctx = context_id("pk:ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
///                      "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo").unwrap();
/// assert_eq!(ctx.len(), 64);
/// ```
pub fn context_id(pubkey_a: &str, pubkey_b: &str) -> Result<String> {
    let norm_a = normalize_pubkey_z32(pubkey_a)?;
    let norm_b = normalize_pubkey_z32(pubkey_b)?;
    let (first, second) = if norm_a <= norm_b {
        (norm_a, norm_b)
    } else {
        (norm_b, norm_a)
    };

    let mut hasher = Sha256::new();
    hasher.update(format!("paykit:v0:context:{}:{}", first, second).as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

/// Compute the scope hash for a pubkey.
///
/// `scope = hex(sha256(utf8(normalized_pubkey_z32)))`
///
/// The scope is used as a per-recipient directory name in storage paths.
///
/// # Arguments
///
/// * `pubkey_z32` - A z-base-32 encoded pubkey (will be normalized)
///
/// # Returns
///
/// Lowercase hex string (64 chars) representing the SHA-256 hash.
///
/// # Errors
///
/// Returns `PaykitError::InvalidData` if the pubkey is malformed.
///
/// # Example
///
/// ```
/// use paykit_lib::protocol::recipient_scope;
///
/// let scope = recipient_scope("pk:ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u").unwrap();
/// assert_eq!(scope.len(), 64); // SHA-256 hex is 64 chars
/// ```
#[deprecated(
    since = "0.3.0",
    note = "Use context_id() instead. RecipientScope is Paykit v0 legacy."
)]
pub fn recipient_scope(pubkey_z32: &str) -> Result<String> {
    let normalized = normalize_pubkey_z32(pubkey_z32)?;
    Ok(compute_scope_hash(&normalized))
}

/// Alias for `recipient_scope` - used for subscription proposals.
///
/// Semantically identical, but named for clarity when dealing with subscriptions.
#[deprecated(
    since = "0.3.0",
    note = "Use context_id() instead. SubscriberScope is Paykit v0 legacy."
)]
pub fn subscriber_scope(pubkey_z32: &str) -> Result<String> {
    #[allow(deprecated)]
    recipient_scope(pubkey_z32)
}

/// Internal: compute SHA-256 hash and return as lowercase hex.
fn compute_scope_hash(normalized_pubkey: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalized_pubkey.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_prefix_and_lowercases() {
        let input = "pk:YBNDRFG8EJKMCPQXOT1UWISZA345H769YBNDRFG8EJKMCPQXOT1U";
        let result = normalize_pubkey_z32(input).unwrap();
        assert_eq!(
            result,
            "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u"
        );
    }

    #[test]
    fn normalize_strips_pubky_prefix() {
        let input = "pubky://YBNDRFG8EJKMCPQXOT1UWISZA345H769YBNDRFG8EJKMCPQXOT1U";
        let result = normalize_pubkey_z32(input).unwrap();
        assert_eq!(
            result,
            "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u"
        );
    }

    #[test]
    fn normalize_handles_already_normalized() {
        let input = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
        let result = normalize_pubkey_z32(input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn normalize_trims_whitespace() {
        let input = "  pk:ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u  ";
        let result = normalize_pubkey_z32(input).unwrap();
        assert_eq!(
            result,
            "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u"
        );
    }

    #[test]
    fn normalize_rejects_wrong_length() {
        let result = normalize_pubkey_z32("tooshort");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("must be 52 chars"));
    }

    #[test]
    fn normalize_rejects_invalid_chars() {
        // 'l' and 'v' are not in z32 alphabet
        let input = "lbndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
        let result = normalize_pubkey_z32(input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid z32 character"));
    }

    // ContextId tests
    #[test]
    fn context_id_is_symmetric() {
        let pubkey_a = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
        let pubkey_b = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

        let ctx_ab = context_id(pubkey_a, pubkey_b).unwrap();
        let ctx_ba = context_id(pubkey_b, pubkey_a).unwrap();

        assert_eq!(ctx_ab, ctx_ba);
        assert_eq!(ctx_ab.len(), 64);
    }

    #[test]
    fn context_id_handles_pubky_prefix() {
        let pubkey_a = "pubky://ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
        let pubkey_b = "pk:8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";

        let ctx = context_id(pubkey_a, pubkey_b).unwrap();
        assert_eq!(ctx.len(), 64);

        // Should match without prefixes
        let ctx_no_prefix = context_id(
            "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u",
            "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo",
        )
        .unwrap();
        assert_eq!(ctx, ctx_no_prefix);
    }

    #[test]
    fn context_id_same_pubkey() {
        let pubkey = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
        let ctx = context_id(pubkey, pubkey).unwrap();
        assert_eq!(ctx.len(), 64);
    }

    // Legacy tests (with #[allow(deprecated)])
    #[test]
    #[allow(deprecated)]
    fn scope_hash_is_deterministic() {
        let pubkey = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
        let scope1 = recipient_scope(pubkey).unwrap();
        let scope2 = recipient_scope(pubkey).unwrap();
        assert_eq!(scope1, scope2);
        assert_eq!(scope1.len(), 64);
    }

    #[test]
    #[allow(deprecated)]
    fn scope_hash_differs_for_different_pubkeys() {
        let pubkey1 = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
        let pubkey2 = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
        let scope1 = recipient_scope(pubkey1).unwrap();
        let scope2 = recipient_scope(pubkey2).unwrap();
        assert_ne!(scope1, scope2);
    }

    #[test]
    #[allow(deprecated)]
    fn subscriber_scope_is_alias_for_recipient_scope() {
        let pubkey = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
        let r_scope = recipient_scope(pubkey).unwrap();
        let s_scope = subscriber_scope(pubkey).unwrap();
        assert_eq!(r_scope, s_scope);
    }

    // Cross-platform test vectors - these MUST match Kotlin/Swift implementations
    #[test]
    #[allow(deprecated)]
    fn cross_platform_scope_vectors() {
        // Vector 1: test pubkey (all z32 chars)
        let pubkey1 = "ybndrfg8ejkmcpqxot1uwisza345h769ybndrfg8ejkmcpqxot1u";
        let scope1 = recipient_scope(pubkey1).unwrap();
        assert_eq!(
            scope1,
            "55340b54f918470e1f025a80bb3347934fad3f57189eef303d620e65468cde80"
        );

        // Vector 2: default homeserver pubkey
        let pubkey2 = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
        let scope2 = recipient_scope(pubkey2).unwrap();
        assert_eq!(
            scope2,
            "04dc3323da61313c6f5404cf7921af2432ef867afe6cc4c32553858b8ac07f12"
        );

        // Vector 3: with pk: prefix (should normalize to same as without)
        let pubkey3_prefixed = "pk:8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo";
        let scope3 = recipient_scope(pubkey3_prefixed).unwrap();
        assert_eq!(scope2, scope3);

        // Vector 4: uppercase (should normalize to same as lowercase)
        let pubkey4_upper = "YBNDRFG8EJKMCPQXOT1UWISZA345H769YBNDRFG8EJKMCPQXOT1U";
        let scope4 = recipient_scope(pubkey4_upper).unwrap();
        assert_eq!(scope1, scope4);
    }
}
