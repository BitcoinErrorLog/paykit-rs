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
    /// This creates a session using the Pubky SDK with the provided keypair.
    /// The homeserver URL from this DirectoryClient is used for the session.
    ///
    /// # Note
    ///
    /// This is a placeholder implementation. In production, you should:
    /// 1. Use `pubky::Pubky::new()` to create a Pubky instance
    /// 2. Get a signer from the SDK: `sdk.signer(keypair)`
    /// 3. Call `signer.signup(&homeserver_public_key, None).await`
    ///
    /// For testing, use `pubky_testnet::EphemeralTestnet` to create sessions.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use paykit_demo_core::directory::DirectoryClient;
    /// use pubky::Keypair;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = DirectoryClient::new("https://homeserver.example.com");
    /// let keypair = Keypair::random();
    /// // In production, create session using Pubky SDK directly
    /// // For now, this method is a placeholder
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_session(&self, _keypair: &pubky::Keypair) -> Result<PubkySession> {
        // TODO: Implement proper session creation using Pubky SDK 0.6.0-rc.6+ API
        // This requires:
        // 1. Creating a Pubky instance with homeserver URL
        // 2. Getting a signer: sdk.signer(keypair)
        // 3. Calling signup: signer.signup(&homeserver_pk, None).await
        //
        // For now, return an error with guidance
        anyhow::bail!(
            "Session creation not yet implemented. \
             To create a session, use Pubky SDK directly:\n\
             1. Create Pubky instance with homeserver: {}\n\
             2. Get signer: sdk.signer(keypair)\n\
             3. Signup: signer.signup(&homeserver_pk, None).await\n\
             See paykit-lib/src/lib.rs tests for example implementation.",
            self.homeserver
        )
    }
}
