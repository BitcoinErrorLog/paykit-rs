//! Directory operations using paykit-lib

use crate::models::PaymentMethod;
use anyhow::{Context, Result};
use paykit_lib::{
    AuthenticatedTransport, EndpointData, MethodId, PubkyAuthenticatedTransport,
    PubkyUnauthenticatedTransport, PublicKey, UnauthenticatedTransportRead,
};
use pubky::{PubkySession, PublicStorage};

/// Client for interacting with the Pubky directory
pub struct DirectoryClient {
    #[allow(dead_code)]
    homeserver: String,
}

impl DirectoryClient {
    /// Create a new directory client
    pub fn new(homeserver: impl Into<String>) -> Self {
        Self {
            homeserver: homeserver.into(),
        }
    }

    /// Publish payment methods to the public directory
    pub async fn publish_methods(
        &self,
        session: &PubkySession,
        methods: &[PaymentMethod],
    ) -> Result<()> {
        let transport = PubkyAuthenticatedTransport::new(session.clone());

        for method in methods {
            let method_id = MethodId(method.method_id.clone());
            let endpoint_data = EndpointData(method.endpoint.clone());

            transport
                .upsert_payment_endpoint(&method_id, &endpoint_data)
                .await
                .with_context(|| format!("Failed to publish method: {}", method.method_id))?;
        }

        Ok(())
    }

    /// Query payment methods from a public key
    pub async fn query_methods(&self, public_key: &PublicKey) -> Result<Vec<PaymentMethod>> {
        let storage = PublicStorage::new().context("Failed to create PublicStorage")?;
        let transport = PubkyUnauthenticatedTransport::new(storage);

        let supported = transport
            .fetch_supported_payments(public_key)
            .await
            .context("Failed to fetch supported payments")?;

        let mut methods = Vec::new();
        for (method_id, endpoint_data) in supported.entries {
            methods.push(PaymentMethod::new(
                method_id.0,
                endpoint_data.0,
                true, // Public by definition
            ));
        }

        Ok(methods)
    }

    /// Delete a payment method from the directory
    pub async fn delete_method(&self, session: &PubkySession, method_id: &str) -> Result<()> {
        let transport = PubkyAuthenticatedTransport::new(session.clone());
        let method_id = MethodId(method_id.to_string());

        transport
            .remove_payment_endpoint(&method_id)
            .await
            .context("Failed to remove payment endpoint")?;

        Ok(())
    }

    /// Create a Pubky session for authenticated operations
    ///
    /// Note: This is a simplified implementation for demo purposes.
    /// In production, you would use Pubky::new() and configure it properly.
    pub async fn create_session(&self, _keypair: &pubky::Keypair) -> Result<PubkySession> {
        // TODO: Implement proper session creation using Pubky SDK
        anyhow::bail!("Session creation not yet implemented - use existing session")
    }
}
