use crate::Amount;
use paykit_lib::{MethodId, PublicKey};
use serde::{Deserialize, Serialize};

/// A payment request that can be sent async to a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub request_id: String,
    pub from: PublicKey,
    pub to: PublicKey,
    pub amount: Amount,
    pub currency: String,
    pub method: MethodId,
    pub description: Option<String>,
    pub due_date: Option<i64>,
    pub metadata: serde_json::Value,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

impl PaymentRequest {
    pub fn new(
        from: PublicKey,
        to: PublicKey,
        amount: Amount,
        currency: String,
        method: MethodId,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            request_id: format!("req_{}", now),
            from,
            to,
            amount,
            currency,
            method,
            description: None,
            due_date: None,
            metadata: serde_json::json!({}),
            created_at: now,
            expires_at: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_due_date(mut self, due_date: i64) -> Self {
        self.due_date = Some(due_date);
        self
    }

    pub fn with_expiration(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now().timestamp() > expires_at
        } else {
            false
        }
    }
}

/// Response to a payment request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PaymentRequestResponse {
    Accepted {
        request_id: String,
        receipt: paykit_interactive::PaykitReceipt,
    },
    Declined {
        request_id: String,
        reason: Option<String>,
    },
    Pending {
        request_id: String,
        estimated_payment_time: Option<i64>,
    },
}

/// Status of a payment request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    Pending,   // Created locally, not yet sent
    Sent,      // Sent to peer, awaiting response
    Accepted,  // Accepted by recipient
    Declined,  // Declined by recipient
    Expired,   // Request expired
    Paid,      // Payment completed
    Fulfilled, // Auto-payment completed (Phase 3)
}

/// Notification stored in Pubky for async discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestNotification {
    pub request_id: String,
    pub from: PublicKey,
    pub amount: Amount,
    pub currency: String,
    pub created_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn test_pubkey() -> PublicKey {
        let keypair = pkarr::Keypair::random();
        PublicKey::from_str(&keypair.public_key().to_z32()).unwrap()
    }

    #[test]
    fn test_payment_request_creation() {
        let from = test_pubkey();
        let to = test_pubkey();
        let request = PaymentRequest::new(
            from.clone(),
            to.clone(),
            Amount::from_sats(1000),
            "SAT".to_string(),
            MethodId("lightning".to_string()),
        );

        assert_eq!(request.from, from);
        assert_eq!(request.to, to);
        assert_eq!(request.amount, Amount::from_sats(1000));
        assert_eq!(request.currency, "SAT");
        assert!(!request.is_expired());
    }

    #[test]
    fn test_payment_request_expiration() {
        let from = test_pubkey();
        let to = test_pubkey();
        let past_time = chrono::Utc::now().timestamp() - 3600;

        let request = PaymentRequest::new(
            from,
            to,
            Amount::from_sats(1000),
            "SAT".to_string(),
            MethodId("lightning".to_string()),
        )
        .with_expiration(past_time);

        assert!(request.is_expired());
    }

    #[test]
    fn test_payment_request_serialization() {
        let from = test_pubkey();
        let to = test_pubkey();
        let request = PaymentRequest::new(
            from,
            to,
            Amount::from_sats(1000),
            "SAT".to_string(),
            MethodId("lightning".to_string()),
        )
        .with_description("Test payment".to_string());

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: PaymentRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.request_id, deserialized.request_id);
        assert_eq!(request.amount, deserialized.amount);
        assert_eq!(request.description, deserialized.description);
    }
}
