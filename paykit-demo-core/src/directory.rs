//! Directory operations using paykit-lib

use crate::models::PaymentMethod;
use anyhow::{Context, Result};
use paykit_lib::{
    AuthenticatedTransport, EndpointData, MethodId, PubkyAuthenticatedTransport,
    PubkyUnauthenticatedTransport, PublicKey, UnauthenticatedTransportRead,
};
use pubky::{Pubky, PubkySession, PublicStorage};

/// Client for interacting with the Pubky directory
pub struct DirectoryClient {
    homeserver: String,
}

impl DirectoryClient {
    /// Create a new directory client
    pub fn new(homeserver: impl Into<String>) -> Self {
        Self {
            homeserver: homeserver.into(),
        }
    }

    /// Get the homeserver URL
    pub fn homeserver(&self) -> &str {
        &self.homeserver
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

    /// Create a Pubky session for authenticated operations.
    ///
    /// This creates a session by signing in to a homeserver using the Pubky SDK.
    /// It attempts to sign in first (for returning users), falling back to signup
    /// if the user doesn't have an account.
    ///
    /// # Arguments
    ///
    /// * `keypair` - The Ed25519 keypair for authentication
    /// * `use_testnet` - If true, use testnet configuration; otherwise use mainnet
    ///
    /// # Example
    ///
    /// ```no_run
    /// use paykit_demo_core::directory::DirectoryClient;
    /// use pubky::Keypair;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = DirectoryClient::new("8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo");
    /// let keypair = Keypair::random();
    /// let session = client.create_session(&keypair, true).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_session(
        &self,
        keypair: &pubky::Keypair,
        use_testnet: bool,
    ) -> Result<PubkySession> {
        // Create Pubky SDK instance
        let pubky = if use_testnet {
            Pubky::testnet().context("Failed to create Pubky testnet instance")?
        } else {
            Pubky::new().context("Failed to create Pubky instance")?
        };

        // Parse homeserver public key from the URL/ID
        let homeserver_pk = self
            .homeserver
            .parse::<PublicKey>()
            .context("Invalid homeserver public key")?;

        // Get signer from SDK
        let signer = pubky.signer(keypair.clone());

        // Try to sign in first (for returning users)
        match signer.signin().await {
            Ok(session) => Ok(session),
            Err(_) => {
                // Signup if signin fails (new user)
                signer
                    .signup(&homeserver_pk, None)
                    .await
                    .context("Failed to signup to homeserver")
            }
        }
    }

    /// Create a Pubky session with a signup token (for homeservers that require registration).
    pub async fn create_session_with_token(
        &self,
        keypair: &pubky::Keypair,
        signup_token: &str,
        use_testnet: bool,
    ) -> Result<PubkySession> {
        let pubky = if use_testnet {
            Pubky::testnet().context("Failed to create Pubky testnet instance")?
        } else {
            Pubky::new().context("Failed to create Pubky instance")?
        };

        let homeserver_pk = self
            .homeserver
            .parse::<PublicKey>()
            .context("Invalid homeserver public key")?;

        let signer = pubky.signer(keypair.clone());
        signer
            .signup(&homeserver_pk, Some(signup_token))
            .await
            .context("Failed to signup with token")
    }

    /// Get raw data from a public key's public storage
    ///
    /// Used for fetching profiles and other arbitrary data from the directory.
    pub async fn get_raw(&self, public_key: &PublicKey, path: &str) -> Result<Option<String>> {
        let storage = PublicStorage::new().context("Failed to create PublicStorage")?;

        // Construct the full URL for the resource
        let url = format!("pubky://{}{}", public_key, path);

        match storage.get(&url).await {
            Ok(Some(bytes)) => {
                let content =
                    String::from_utf8(bytes).context("Response is not valid UTF-8")?;
                Ok(Some(content))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to get {}: {}", path, e)),
        }
    }

    /// Put raw data to authenticated storage
    ///
    /// Used for publishing profiles and other arbitrary data to the directory.
    pub async fn put_raw(&self, session: &PubkySession, path: &str, content: &str) -> Result<()> {
        // Get the public key from the session
        let public_key = session.public_key();
        let url = format!("pubky://{}{}", public_key, path);

        session
            .put(&url, content.as_bytes())
            .await
            .with_context(|| format!("Failed to put data at {}", path))?;

        Ok(())
    }

    /// Delete raw data from authenticated storage
    pub async fn delete_raw(&self, session: &PubkySession, path: &str) -> Result<()> {
        let public_key = session.public_key();
        let url = format!("pubky://{}{}", public_key, path);

        session
            .delete(&url)
            .await
            .with_context(|| format!("Failed to delete {}", path))?;

        Ok(())
    }
}
