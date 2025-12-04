//! Session management for Pubky authentication
//!
//! Provides helpers to create authenticated sessions for publishing and managing
//! payment methods on Pubky homeservers.

use anyhow::{Context, Result};
use paykit_lib::PubkyAuthenticatedTransport;
use pubky::{Keypair, Pubky};

use crate::Identity;

/// Session manager for creating authenticated Pubky transports
pub struct SessionManager;

impl SessionManager {
    /// Create an authenticated transport using a Pubky SDK instance
    ///
    /// This method:
    /// 1. Creates a signer with the identity's keypair
    /// 2. Signs up with the homeserver
    /// 3. Returns a wrapped authenticated transport
    ///
    /// # Arguments
    /// * `sdk` - A Pubky SDK instance (configured with homeserver connection)
    /// * `identity` - The user's identity containing their keypair
    /// * `homeserver_pubkey` - The public key of the homeserver to authenticate with
    ///
    /// # Example
    /// ```no_run
    /// # use paykit_demo_core::{Identity, SessionManager};
    /// # use pubky::Pubky;
    /// # async fn example() -> anyhow::Result<()> {
    /// let sdk = Pubky::new()?;
    /// let identity = Identity::generate();
    /// let homeserver_pk = "8pinxxgqs41n4aididenw5apqp1urfmzdztr8jt4abrkdn435ewo".parse()?;
    ///
    /// let auth_transport = SessionManager::create_with_sdk(
    ///     &sdk,
    ///     &identity,
    ///     &homeserver_pk,
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_with_sdk(
        sdk: &Pubky,
        identity: &Identity,
        homeserver_pubkey: &pubky::PublicKey,
    ) -> Result<PubkyAuthenticatedTransport> {
        // Create signer with the identity's keypair
        let signer = sdk.signer(identity.keypair.clone());

        // Sign up and create session
        let session = signer
            .signup(homeserver_pubkey, None)
            .await
            .context("Failed to signup with homeserver")?;

        // Wrap in authenticated transport
        Ok(PubkyAuthenticatedTransport::new(session))
    }

    /// Create authenticated transport from a keypair
    ///
    /// Convenience method that doesn't require a full Identity object
    pub async fn create_with_keypair(
        sdk: &Pubky,
        keypair: &Keypair,
        homeserver_pubkey: &pubky::PublicKey,
    ) -> Result<PubkyAuthenticatedTransport> {
        let identity = Identity::from_keypair(keypair.clone());
        Self::create_with_sdk(sdk, &identity, homeserver_pubkey).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pubky_testnet::EphemeralTestnet;

    #[tokio::test]
    #[ignore] // Requires network access to testnet/mainline
    async fn test_session_manager_creates_authenticated_transport() {
        // Start testnet
        let testnet = EphemeralTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver = testnet.homeserver();
        let sdk = testnet.sdk().expect("Failed to get SDK");

        // Create identity
        let identity = Identity::generate();

        // Create authenticated transport
        let result =
            SessionManager::create_with_sdk(&sdk, &identity, &homeserver.public_key()).await;

        match result {
            Ok(_) => {} // Success
            Err(e) => panic!("Failed to create authenticated transport: {:#}", e),
        }
    }

    #[tokio::test]
    #[ignore] // Requires network access to testnet/mainline
    async fn test_session_manager_from_keypair() {
        // Start testnet
        let testnet = EphemeralTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver = testnet.homeserver();
        let sdk = testnet.sdk().expect("Failed to get SDK");

        // Create keypair
        let keypair = Keypair::random();

        // Create authenticated transport
        let result =
            SessionManager::create_with_keypair(&sdk, &keypair, &homeserver.public_key()).await;

        match result {
            Ok(_) => {} // Success
            Err(e) => panic!(
                "Failed to create authenticated transport from keypair: {:#}",
                e
            ),
        }
    }

    #[tokio::test]
    #[ignore] // Requires network access to testnet/mainline
    async fn test_session_manager_can_publish() {
        // Integration test: create session and publish a method
        use paykit_lib::{AuthenticatedTransport, EndpointData, MethodId};

        let testnet = EphemeralTestnet::start()
            .await
            .expect("Failed to start testnet");
        let homeserver = testnet.homeserver();
        let sdk = testnet.sdk().expect("Failed to get SDK");

        let identity = Identity::generate();

        // Create authenticated transport
        let auth_transport =
            SessionManager::create_with_sdk(&sdk, &identity, &homeserver.public_key())
                .await
                .expect("Failed to create transport");

        // Try to publish a method
        let method = MethodId("lightning".to_string());
        let endpoint = EndpointData("lnbc1...test".to_string());

        let result = auth_transport
            .upsert_payment_endpoint(&method, &endpoint)
            .await;

        assert!(
            result.is_ok(),
            "Should be able to publish method: {:?}",
            result.err()
        );
    }
}
