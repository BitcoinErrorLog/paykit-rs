//! Private endpoint types.

use crate::{EndpointData, MethodId, PublicKey};
use serde::{Deserialize, Serialize};

/// A private payment endpoint exchanged via encrypted channel.
///
/// Private endpoints are not published to the public directory. Instead,
/// they are shared directly between peers over encrypted channels (Noise Protocol).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PrivateEndpoint {
    /// The peer who provided this endpoint.
    pub peer: PublicKey,
    /// The payment method this endpoint is for.
    pub method_id: MethodId,
    /// The endpoint data (e.g., Bitcoin address, Lightning invoice).
    pub endpoint: EndpointData,
    /// When this endpoint was created (unix timestamp).
    pub created_at: i64,
    /// When this endpoint expires (unix timestamp), if applicable.
    pub expires_at: Option<i64>,
    /// Number of times this endpoint has been used.
    pub use_count: u32,
    /// Last time this endpoint was used (unix timestamp).
    pub last_used_at: Option<i64>,
}

impl PrivateEndpoint {
    /// Create a new private endpoint.
    pub fn new(
        peer: PublicKey,
        method_id: MethodId,
        endpoint: EndpointData,
        expires_at: Option<i64>,
    ) -> Self {
        Self {
            peer,
            method_id,
            endpoint,
            created_at: chrono::Utc::now().timestamp(),
            expires_at,
            use_count: 0,
            last_used_at: None,
        }
    }

    /// Create a new private endpoint with a specific creation time.
    pub fn with_created_at(
        peer: PublicKey,
        method_id: MethodId,
        endpoint: EndpointData,
        created_at: i64,
        expires_at: Option<i64>,
    ) -> Self {
        Self {
            peer,
            method_id,
            endpoint,
            created_at,
            expires_at,
            use_count: 0,
            last_used_at: None,
        }
    }

    /// Check if this endpoint has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now().timestamp() > expires_at
        } else {
            false
        }
    }

    /// Check if this endpoint is still valid.
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }

    /// Get the remaining time until expiration in seconds.
    ///
    /// Returns `None` if there is no expiration, or negative if already expired.
    pub fn time_until_expiry(&self) -> Option<i64> {
        self.expires_at
            .map(|exp| exp - chrono::Utc::now().timestamp())
    }

    /// Record that this endpoint was used.
    pub fn record_use(&mut self) {
        self.use_count += 1;
        self.last_used_at = Some(chrono::Utc::now().timestamp());
    }

    /// Get the age of this endpoint in seconds.
    pub fn age_seconds(&self) -> i64 {
        chrono::Utc::now().timestamp() - self.created_at
    }

    /// Create a unique key for this endpoint (for storage lookups).
    pub fn key(&self) -> String {
        format!("{}:{}", peer_to_string(&self.peer), self.method_id.0)
    }
}

/// Policy for handling private endpoints.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EndpointPolicy {
    /// Default expiration time for endpoints without explicit expiration.
    pub default_expiration: ExpirationPolicy,
    /// Maximum number of endpoints to store per peer.
    pub max_endpoints_per_peer: usize,
    /// Whether to automatically clean up expired endpoints.
    pub auto_cleanup: bool,
    /// Whether to prefer private endpoints over public ones.
    pub prefer_private: bool,
}

impl Default for EndpointPolicy {
    fn default() -> Self {
        Self {
            default_expiration: ExpirationPolicy::Days(30),
            max_endpoints_per_peer: 10,
            auto_cleanup: true,
            prefer_private: true,
        }
    }
}

impl EndpointPolicy {
    /// Create a policy with no expiration.
    pub fn no_expiration() -> Self {
        Self {
            default_expiration: ExpirationPolicy::Never,
            ..Default::default()
        }
    }

    /// Create a policy with a specific expiration in days.
    pub fn with_expiration_days(days: u32) -> Self {
        Self {
            default_expiration: ExpirationPolicy::Days(days),
            ..Default::default()
        }
    }

    /// Create a policy for single-use endpoints.
    pub fn single_use() -> Self {
        Self {
            default_expiration: ExpirationPolicy::AfterUse(1),
            ..Default::default()
        }
    }

    /// Get the expiration timestamp based on the policy.
    pub fn calculate_expiration(&self) -> Option<i64> {
        self.default_expiration.calculate_timestamp()
    }
}

/// Expiration policy for private endpoints.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ExpirationPolicy {
    /// Never expire.
    Never,
    /// Expire after a number of days.
    Days(u32),
    /// Expire after a number of hours.
    Hours(u32),
    /// Expire after a specific unix timestamp.
    At(i64),
    /// Expire after a number of uses.
    AfterUse(u32),
}

impl ExpirationPolicy {
    /// Calculate the expiration timestamp based on this policy.
    ///
    /// Returns `None` for `Never` and `AfterUse` policies.
    pub fn calculate_timestamp(&self) -> Option<i64> {
        let now = chrono::Utc::now().timestamp();
        match self {
            ExpirationPolicy::Never => None,
            ExpirationPolicy::Days(days) => Some(now + (*days as i64) * 24 * 3600),
            ExpirationPolicy::Hours(hours) => Some(now + (*hours as i64) * 3600),
            ExpirationPolicy::At(timestamp) => Some(*timestamp),
            ExpirationPolicy::AfterUse(_) => None, // Handled separately
        }
    }
}

/// Helper function to convert PublicKey to string for storage keys.
#[cfg(feature = "pubky")]
fn peer_to_string(peer: &PublicKey) -> String {
    peer.to_string()
}

#[cfg(not(feature = "pubky"))]
fn peer_to_string(peer: &PublicKey) -> String {
    peer.0.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pubkey() -> PublicKey {
        #[cfg(feature = "pubky")]
        {
            use pubky::Keypair;
            let keypair = Keypair::random();
            keypair.public_key()
        }
        #[cfg(not(feature = "pubky"))]
        {
            PublicKey("test_key".to_string())
        }
    }

    #[test]
    fn test_private_endpoint_creation() {
        let peer = test_pubkey();
        let method = MethodId("lightning".to_string());
        let endpoint = EndpointData("lnbc...".to_string());

        let private = PrivateEndpoint::new(peer.clone(), method.clone(), endpoint.clone(), None);

        assert_eq!(private.peer, peer);
        assert_eq!(private.method_id, method);
        assert_eq!(private.endpoint, endpoint);
        assert!(private.expires_at.is_none());
        assert!(!private.is_expired());
        assert!(private.is_valid());
    }

    #[test]
    fn test_expiration() {
        let peer = test_pubkey();
        let method = MethodId("lightning".to_string());
        let endpoint = EndpointData("lnbc...".to_string());

        // Expired endpoint
        let expired_at = chrono::Utc::now().timestamp() - 3600;
        let expired = PrivateEndpoint::new(
            peer.clone(),
            method.clone(),
            endpoint.clone(),
            Some(expired_at),
        );
        assert!(expired.is_expired());
        assert!(!expired.is_valid());

        // Valid endpoint
        let valid_until = chrono::Utc::now().timestamp() + 3600;
        let valid = PrivateEndpoint::new(peer, method, endpoint, Some(valid_until));
        assert!(!valid.is_expired());
        assert!(valid.is_valid());
    }

    #[test]
    fn test_record_use() {
        let peer = test_pubkey();
        let method = MethodId("lightning".to_string());
        let endpoint = EndpointData("lnbc...".to_string());

        let mut private = PrivateEndpoint::new(peer, method, endpoint, None);
        assert_eq!(private.use_count, 0);
        assert!(private.last_used_at.is_none());

        private.record_use();
        assert_eq!(private.use_count, 1);
        assert!(private.last_used_at.is_some());

        private.record_use();
        assert_eq!(private.use_count, 2);
    }

    #[test]
    fn test_expiration_policy() {
        // Never policy
        let never = ExpirationPolicy::Never;
        assert!(never.calculate_timestamp().is_none());

        // Days policy
        let days = ExpirationPolicy::Days(7);
        let timestamp = days.calculate_timestamp().unwrap();
        let expected = chrono::Utc::now().timestamp() + 7 * 24 * 3600;
        assert!((timestamp - expected).abs() < 2); // Within 2 seconds

        // Hours policy
        let hours = ExpirationPolicy::Hours(24);
        let timestamp = hours.calculate_timestamp().unwrap();
        let expected = chrono::Utc::now().timestamp() + 24 * 3600;
        assert!((timestamp - expected).abs() < 2);

        // AfterUse policy
        let after_use = ExpirationPolicy::AfterUse(3);
        assert!(after_use.calculate_timestamp().is_none());
    }

    #[test]
    fn test_endpoint_policy_defaults() {
        let policy = EndpointPolicy::default();
        assert_eq!(policy.max_endpoints_per_peer, 10);
        assert!(policy.auto_cleanup);
        assert!(policy.prefer_private);
    }

    #[test]
    fn test_time_until_expiry() {
        let peer = test_pubkey();
        let method = MethodId("lightning".to_string());
        let endpoint = EndpointData("lnbc...".to_string());

        // No expiration
        let no_exp = PrivateEndpoint::new(peer.clone(), method.clone(), endpoint.clone(), None);
        assert!(no_exp.time_until_expiry().is_none());

        // Future expiration
        let future = chrono::Utc::now().timestamp() + 3600;
        let valid = PrivateEndpoint::new(peer.clone(), method.clone(), endpoint.clone(), Some(future));
        let remaining = valid.time_until_expiry().unwrap();
        assert!(remaining > 3590 && remaining <= 3600);

        // Past expiration
        let past = chrono::Utc::now().timestamp() - 3600;
        let expired = PrivateEndpoint::new(peer, method, endpoint, Some(past));
        let remaining = expired.time_until_expiry().unwrap();
        assert!(remaining < 0);
    }
}
