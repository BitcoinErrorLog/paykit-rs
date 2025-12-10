//! Payment Request Discovery
//!
//! This module provides functionality to publish and discover payment requests
//! via Pubky homeservers, enabling async payment request delivery.

use crate::{PaymentRequest, RequestNotification};
use paykit_lib::{AuthenticatedTransport, PublicKey, UnauthenticatedTransportRead};
use serde::{Deserialize, Serialize};

/// Path prefix for payment requests in Pubky storage.
pub const PAYKIT_REQUESTS_PATH: &str = "/pub/paykit.app/v0/requests/";

/// Path prefix for incoming request notifications.
pub const PAYKIT_NOTIFICATIONS_PATH: &str = "/pub/paykit.app/v0/notifications/requests/";

/// A discoverable payment request stored in Pubky.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedRequest {
    /// The full payment request.
    pub request: PaymentRequest,
    /// Timestamp when published.
    pub published_at: i64,
    /// Whether this request is still active.
    pub active: bool,
}

impl PublishedRequest {
    /// Create a new published request.
    pub fn new(request: PaymentRequest) -> Self {
        Self {
            request,
            published_at: chrono::Utc::now().timestamp(),
            active: true,
        }
    }

    /// Mark this request as inactive.
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

/// Publish a payment request to the sender's Pubky storage.
///
/// The request is stored at `/pub/paykit.app/v0/requests/{request_id}`.
/// A notification is also published to the recipient's notifications path.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the sender
/// * `request` - The payment request to publish
///
/// # Example
///
/// ```ignore
/// use paykit_subscriptions::discovery::publish_payment_request;
///
/// let request = PaymentRequest::new(from, to, amount, currency, method);
/// publish_payment_request(&transport, &request).await?;
/// ```
pub async fn publish_payment_request<T: AuthenticatedTransport>(
    transport: &T,
    request: &PaymentRequest,
) -> crate::Result<()> {
    let published = PublishedRequest::new(request.clone());
    let json = serde_json::to_string(&published)?;
    
    // Store in sender's requests directory
    let path = format!("{}{}", PAYKIT_REQUESTS_PATH, request.request_id);
    transport.put(&path, &json).await
        .map_err(|e| anyhow::anyhow!("Failed to publish request: {}", e))?;
    
    Ok(())
}

/// Publish a notification for a payment request to the recipient.
///
/// This creates a lightweight notification in the recipient's Pubky storage
/// that they can discover via polling.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the sender
/// * `recipient` - The recipient's public key
/// * `request` - The payment request
pub async fn publish_request_notification<T: AuthenticatedTransport>(
    transport: &T,
    recipient: &PublicKey,
    request: &PaymentRequest,
) -> crate::Result<()> {
    let notification = RequestNotification {
        request_id: request.request_id.clone(),
        from: request.from.clone(),
        amount: request.amount,
        currency: request.currency.clone(),
        created_at: request.created_at,
    };
    
    let json = serde_json::to_string(&notification)?;
    
    // Store notification for recipient
    // Note: This requires write access to recipient's storage,
    // which may not be possible in all configurations.
    // The actual implementation may use a different mechanism.
    let path = format!("pubky://{}{}{}", recipient, PAYKIT_NOTIFICATIONS_PATH, request.request_id);
    transport.put(&path, &json).await
        .map_err(|e| anyhow::anyhow!("Failed to publish notification: {}", e))?;
    
    Ok(())
}

/// Discover payment requests from a sender.
///
/// # Arguments
///
/// * `reader` - Unauthenticated reader for Pubky storage
/// * `sender` - The sender's public key
///
/// # Returns
///
/// A list of published payment requests from the sender.
pub async fn discover_requests<R: UnauthenticatedTransportRead>(
    reader: &R,
    sender: &PublicKey,
) -> crate::Result<Vec<PublishedRequest>> {
    let entries = reader.list_directory(sender, PAYKIT_REQUESTS_PATH).await
        .map_err(|e| anyhow::anyhow!("Failed to list requests: {}", e))?;
    
    let mut requests = Vec::new();
    
    for entry in entries {
        let path = format!("{}{}", PAYKIT_REQUESTS_PATH, entry);
        if let Ok(Some(content)) = reader.get(sender, &path).await {
            if let Ok(published) = serde_json::from_str::<PublishedRequest>(&content) {
                if published.active {
                    requests.push(published);
                }
            }
        }
    }
    
    Ok(requests)
}

/// Discover a specific payment request by ID.
///
/// # Arguments
///
/// * `reader` - Unauthenticated reader for Pubky storage
/// * `sender` - The sender's public key
/// * `request_id` - The request ID
///
/// # Returns
///
/// The published request if found.
pub async fn discover_request<R: UnauthenticatedTransportRead>(
    reader: &R,
    sender: &PublicKey,
    request_id: &str,
) -> crate::Result<Option<PublishedRequest>> {
    let path = format!("{}{}", PAYKIT_REQUESTS_PATH, request_id);
    
    match reader.get(sender, &path).await {
        Ok(Some(content)) => {
            let published: PublishedRequest = serde_json::from_str(&content)?;
            Ok(Some(published))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to fetch request: {}", e)),
    }
}

/// Cancel a published payment request.
///
/// This deactivates the request in storage.
///
/// # Arguments
///
/// * `transport` - Authenticated transport for the sender
/// * `request_id` - The request ID to cancel
pub async fn cancel_payment_request<T: AuthenticatedTransport>(
    transport: &T,
    request_id: &str,
) -> crate::Result<()> {
    let path = format!("{}{}", PAYKIT_REQUESTS_PATH, request_id);
    
    // Get current request
    if let Ok(Some(content)) = transport.get(&path).await {
        if let Ok(mut published) = serde_json::from_str::<PublishedRequest>(&content) {
            published.deactivate();
            let json = serde_json::to_string(&published)?;
            transport.put(&path, &json).await
                .map_err(|e| anyhow::anyhow!("Failed to cancel request: {}", e))?;
        }
    }
    
    Ok(())
}

/// Discovery poller for incoming payment requests.
///
/// This struct provides polling-based discovery of payment requests
/// from known contacts or specific peers.
pub struct RequestDiscoveryPoller<R: UnauthenticatedTransportRead> {
    reader: R,
    known_peers: Vec<PublicKey>,
    last_poll: i64,
    poll_interval_secs: u64,
}

impl<R: UnauthenticatedTransportRead> RequestDiscoveryPoller<R> {
    /// Create a new poller.
    pub fn new(reader: R, poll_interval_secs: u64) -> Self {
        Self {
            reader,
            known_peers: Vec::new(),
            last_poll: 0,
            poll_interval_secs,
        }
    }

    /// Add a peer to monitor for payment requests.
    pub fn add_peer(&mut self, peer: PublicKey) {
        if !self.known_peers.contains(&peer) {
            self.known_peers.push(peer);
        }
    }

    /// Remove a peer from monitoring.
    pub fn remove_peer(&mut self, peer: &PublicKey) {
        self.known_peers.retain(|p| p != peer);
    }

    /// Check if a poll is due.
    pub fn should_poll(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        (now - self.last_poll) as u64 >= self.poll_interval_secs
    }

    /// Poll for new payment requests from all monitored peers.
    ///
    /// Returns a list of (sender, requests) tuples for peers with active requests.
    pub async fn poll(&mut self) -> crate::Result<Vec<(PublicKey, Vec<PublishedRequest>)>> {
        let mut results = Vec::new();
        
        for peer in &self.known_peers {
            if let Ok(requests) = discover_requests(&self.reader, peer).await {
                if !requests.is_empty() {
                    results.push((peer.clone(), requests));
                }
            }
        }
        
        self.last_poll = chrono::Utc::now().timestamp();
        Ok(results)
    }

    /// Poll for new payment requests and filter by creation time.
    ///
    /// Only returns requests created after `after_timestamp`.
    pub async fn poll_new(&mut self, after_timestamp: i64) -> crate::Result<Vec<(PublicKey, Vec<PublishedRequest>)>> {
        let all = self.poll().await?;
        
        let filtered: Vec<(PublicKey, Vec<PublishedRequest>)> = all
            .into_iter()
            .map(|(peer, requests)| {
                let new_requests: Vec<PublishedRequest> = requests
                    .into_iter()
                    .filter(|r| r.request.created_at > after_timestamp)
                    .collect();
                (peer, new_requests)
            })
            .filter(|(_, requests)| !requests.is_empty())
            .collect();
        
        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Amount;
    use paykit_lib::MethodId;
    use std::str::FromStr;

    fn test_pubkey() -> PublicKey {
        let keypair = pkarr::Keypair::random();
        PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
    }

    #[test]
    fn test_published_request_creation() {
        let from = test_pubkey();
        let to = test_pubkey();
        let request = PaymentRequest::new(
            from,
            to,
            Amount::from_sats(1000),
            "SAT".to_string(),
            MethodId("lightning".to_string()),
        );

        let published = PublishedRequest::new(request);
        assert!(published.active);
        assert!(published.published_at > 0);
    }

    #[test]
    fn test_published_request_deactivate() {
        let from = test_pubkey();
        let to = test_pubkey();
        let request = PaymentRequest::new(
            from,
            to,
            Amount::from_sats(1000),
            "SAT".to_string(),
            MethodId("lightning".to_string()),
        );

        let mut published = PublishedRequest::new(request);
        assert!(published.active);
        
        published.deactivate();
        assert!(!published.active);
    }

    #[test]
    fn test_path_constants() {
        assert!(PAYKIT_REQUESTS_PATH.starts_with("/pub/paykit.app/"));
        assert!(PAYKIT_NOTIFICATIONS_PATH.starts_with("/pub/paykit.app/"));
    }
}
