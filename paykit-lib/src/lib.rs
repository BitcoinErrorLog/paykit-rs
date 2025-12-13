//! Paykit library.
//!
//! This crate intentionally stays stateless and delegates authenticated access
//! to callers through trait-based dependency injection.
//!
//! # Features
//!
//! - **Directory Protocol**: Publish and discover payment endpoints via Pubky homeservers
//! - **Payment Method Plugins**: Extensible system for adding new payment methods
//! - **Transport Abstraction**: Trait-based design for custom transport implementations
//!
//! # Example
//!
//! ```ignore
//! use paykit_lib::{MethodId, EndpointData, set_payment_endpoint, get_payment_list};
//! use paykit_lib::methods::{PaymentMethodRegistry, OnchainPlugin, LightningPlugin};
//!
//! // Create a registry with plugins
//! let registry = PaymentMethodRegistry::new();
//! registry.register(Box::new(OnchainPlugin::new()));
//! registry.register(Box::new(LightningPlugin::new()));
//!
//! // Validate an endpoint before publishing
//! let plugin = registry.get(&MethodId("onchain".into())).unwrap();
//! let result = plugin.validate_endpoint(&EndpointData("bc1q...".into()));
//! assert!(result.valid);
//! ```

use std::collections::HashMap;

#[cfg(feature = "pubky")]
pub use pubky::PublicKey;

#[cfg(not(feature = "pubky"))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PublicKey(pub String);

pub mod errors;
pub mod executors;
pub mod health;
pub mod methods;
pub mod prelude;
pub mod private_endpoints;
pub mod rotation;
pub mod routing;
pub mod secure_storage;
pub mod selection;
mod transport;
pub mod uri;

/// Test utilities for payment testing.
///
/// This module is only available with the `test-utils` feature or in test builds.
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use errors::{PaykitError, PaykitErrorCode};
pub use transport::{AuthenticatedTransport, UnauthenticatedTransportRead};
pub use uri::{parse_uri, PaykitUri};

/// Pubky adapters are only exposed when the default `pubky` feature is enabled.
#[cfg(feature = "pubky")]
pub use transport::{PubkyAuthenticatedTransport, PubkyUnauthenticatedTransport};

/// Common result alias for Paykit operations.
pub type Result<T> = std::result::Result<T, PaykitError>;

/// Identifier for a payment method specification.
///
/// Typically based filename component stored under `/pub/paykit.app/v0/…`.
///
/// # Example
///
/// ```
/// use paykit_lib::MethodId;
///
/// // Create from &str
/// let method: MethodId = "lightning".into();
///
/// // Or explicitly
/// let method = MethodId::new("onchain");
///
/// // Access the inner value
/// assert!(method.as_str().starts_with("on"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MethodId(pub String);

impl MethodId {
    /// Create a new MethodId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the method ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Well-known method ID for on-chain Bitcoin payments.
    pub const ONCHAIN: &'static str = "onchain";

    /// Well-known method ID for Lightning payments.
    pub const LIGHTNING: &'static str = "lightning";

    /// Create the on-chain method ID.
    pub fn onchain() -> Self {
        Self::new(Self::ONCHAIN)
    }

    /// Create the lightning method ID.
    pub fn lightning() -> Self {
        Self::new(Self::LIGHTNING)
    }
}

impl From<&str> for MethodId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for MethodId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for MethodId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MethodId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Serialized payload served by a payment endpoint (UTF-8 text such as JSON, lnurl, etc.).
///
/// If you need to transmit binary payloads, encode them (e.g., base64) before wrapping
/// in `EndpointData`.
///
/// # Example
///
/// ```
/// use paykit_lib::EndpointData;
///
/// // Create from &str
/// let data: EndpointData = "bc1qtest...".into();
///
/// // Or explicitly
/// let data = EndpointData::new("lnurl1...");
///
/// // Access the inner value
/// assert!(data.as_str().starts_with("lnurl"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EndpointData(pub String);

impl EndpointData {
    /// Create new endpoint data from a string.
    pub fn new(data: impl Into<String>) -> Self {
        Self(data.into())
    }

    /// Get the endpoint data as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if the endpoint data is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the length of the endpoint data.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<&str> for EndpointData {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for EndpointData {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for EndpointData {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EndpointData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Collection of supported payment entries keyed by method identifiers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SupportedPayments {
    /// Map of `MethodId` to endpoint data.
    pub entries: HashMap<MethodId, EndpointData>,
}

impl SupportedPayments {
    /// Convert to a list of (method_id, endpoint) tuples for FFI.
    pub fn to_list(&self) -> Vec<(String, String)> {
        self.entries
            .iter()
            .map(|(k, v)| (k.0.clone(), v.0.clone()))
            .collect()
    }
}

/// Stores or updates a payment endpoint via the injected authenticated client.
///
/// # Examples
/// ```
/// # use paykit_lib::{set_payment_endpoint, MethodId, EndpointData, PublicKey};
/// # use paykit_lib::AuthenticatedTransport;
/// # async fn demo(client: &impl AuthenticatedTransport) -> paykit_lib::Result<()> {
/// let method = MethodId("lightning".into());
/// let data = EndpointData("{\"bolt11\":\"ln...\"}".into());
/// set_payment_endpoint(client, method, data).await?;
/// # Ok(())
/// # }
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip(client, data), fields(method = %method.0, data_len = data.0.len())))]
pub async fn set_payment_endpoint<S>(client: &S, method: MethodId, data: EndpointData) -> Result<()>
where
    S: AuthenticatedTransport,
{
    client
        .upsert_payment_endpoint(&method, &data)
        .await
        .map_err(|err| map_transport_error("set_payment_endpoint", err))
}

/// Removes a payment endpoint via the injected authenticated client.
#[cfg_attr(feature = "tracing", tracing::instrument(skip(client), fields(method = %method.0)))]
pub async fn remove_payment_endpoint<S>(client: &S, method: MethodId) -> Result<()>
where
    S: AuthenticatedTransport,
{
    client
        .remove_payment_endpoint(&method)
        .await
        .map_err(|err| map_transport_error("remove_payment_endpoint", err))
}

/// Retrieves all supported payment methods for the given payee.
///
/// # Semantics
/// - Returns an empty map when the payee has not published any endpoints or their
///   storage directory is missing.
/// - Propagates transport failures (e.g., network errors) as `PaykitError::Transport`.
///
/// # Examples
/// ```
/// # use paykit_lib::{get_payment_list, MethodId, EndpointData, SupportedPayments};
/// # use paykit_lib::{AuthenticatedTransport, UnauthenticatedTransportRead};
/// # async fn demo(reader: &impl UnauthenticatedTransportRead, pk: &paykit_lib::PublicKey) -> paykit_lib::Result<()> {
/// let payments = get_payment_list(reader, pk).await?;
/// if payments.entries.is_empty() {
///     println!("payee published no endpoints yet");
/// } else {
///     for (method, data) in &payments.entries {
///         println!("method={} payload={}", method.0, data.0);
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip(reader)))]
pub async fn get_payment_list<R>(reader: &R, payee: &PublicKey) -> Result<SupportedPayments>
where
    R: UnauthenticatedTransportRead,
{
    reader
        .fetch_supported_payments(payee)
        .await
        .map_err(|err| map_transport_error("get_payment_list", err))
}

/// Retrieves a specific payment endpoint for `payee` and `method`.
///
/// # Semantics
/// - Returns `Ok(None)` when the endpoint file is missing or empty.
/// - Returns `Err` only when the underlying transport fails (permissions, network, etc.).
///
/// # Examples
/// ```
/// # use paykit_lib::{get_payment_endpoint, MethodId, PublicKey};
/// # use paykit_lib::UnauthenticatedTransportRead;
/// # async fn inspect(reader: &impl UnauthenticatedTransportRead, pk: &PublicKey) -> paykit_lib::Result<()> {
/// let lightning = MethodId("lightning".into());
/// if let Some(endpoint) = get_payment_endpoint(reader, pk, &lightning).await? {
///     println!("lightning endpoint: {}", endpoint.0);
/// } else {
///     println!("no lightning endpoint published");
/// }
/// # Ok(())
/// # }
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip(reader), fields(method = %method.0)))]
pub async fn get_payment_endpoint<R>(
    reader: &R,
    payee: &PublicKey,
    method: &MethodId,
) -> Result<Option<EndpointData>>
where
    R: UnauthenticatedTransportRead,
{
    reader
        .fetch_payment_endpoint(payee, method)
        .await
        .map_err(|err| map_transport_error("get_payment_endpoint", err))
}

/// Returns known contacts of a given public key.
///
/// # Semantics
/// - Returns an empty vector when no contacts are stored under the follows path
///   or the directory does not exist yet.
/// - Returns `Err` only when listing fails due to a transport error.
///
/// # Examples
/// ```
/// # use paykit_lib::{get_known_contacts, PublicKey};
/// # use paykit_lib::UnauthenticatedTransportRead;
/// # async fn contacts(reader: &impl UnauthenticatedTransportRead, pk: &PublicKey) -> paykit_lib::Result<()> {
/// for contact in get_known_contacts(reader, pk).await? {
///     println!("known contact: {}", contact);
/// }
/// # Ok(())
/// # }
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip(reader)))]
pub async fn get_known_contacts<R>(reader: &R, key: &PublicKey) -> Result<Vec<PublicKey>>
where
    R: UnauthenticatedTransportRead,
{
    reader
        .fetch_known_contacts(key)
        .await
        .map_err(|err| map_transport_error("get_known_contacts", err))
}

fn map_transport_error(label: &'static str, err: PaykitError) -> PaykitError {
    match err {
        PaykitError::Transport(msg) => PaykitError::Transport(format!("{label}: {msg}")),
        _ => err,
    }
}

/// Integration tests that require network access.
///
/// These tests are gated behind the `integration-tests` feature and are ignored
/// by default. To run them:
///
/// ```bash
/// cargo test --features integration-tests -- --ignored
/// ```
///
/// Note: These tests require network access to connect to the Pubky testnet.
#[cfg(all(test, feature = "integration-tests"))]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::transport::pubky::PUBKY_FOLLOWS_PATH;
    use pubky::PubkySession;
    use pubky_testnet::{pubky::Keypair, EphemeralTestnet};

    struct TestSetup {
        _testnet: EphemeralTestnet,
        session_transport: PubkyAuthenticatedTransport,
        reader_transport: PubkyUnauthenticatedTransport,
        raw_session: PubkySession,
        public_key: PublicKey,
    }

    /// Error type for test setup failures.
    #[derive(Debug)]
    struct TestSetupError(String);

    impl std::fmt::Display for TestSetupError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestSetup error: {}", self.0)
        }
    }

    impl std::error::Error for TestSetupError {}

    impl TestSetup {
        async fn new() -> std::result::Result<Self, TestSetupError> {
            let testnet = EphemeralTestnet::start()
                .await
                .map_err(|e| TestSetupError(format!("Failed to start testnet: {}", e)))?;
            let homeserver = testnet.homeserver();
            let sdk = testnet
                .sdk()
                .map_err(|e| TestSetupError(format!("Failed to get SDK: {}", e)))?;

            let pair = Keypair::random();
            let signer = sdk.signer(pair.clone());
            let session = signer
                .signup(&homeserver.public_key(), None)
                .await
                .map_err(|e| TestSetupError(format!("Failed to signup: {}", e)))?;

            let session_transport = PubkyAuthenticatedTransport::new(session.clone());
            let reader_transport = PubkyUnauthenticatedTransport::new(sdk.public_storage());

            Ok(Self {
                _testnet: testnet,
                session_transport,
                reader_transport,
                raw_session: session,
                public_key: pair.public_key(),
            })
        }
    }

    #[tokio::test]
    #[ignore] // Requires network access - run with: cargo test --features integration-tests -- --ignored
    async fn endpoint_round_trip_and_update() {
        let setup = TestSetup::new()
            .await
            .expect("Failed to create test setup - network may be unavailable");

        let method = MethodId("onchain".into());
        let endpoint = EndpointData("{\"address\":\"bc1...\"}".into());

        set_payment_endpoint(&setup.session_transport, method.clone(), endpoint.clone())
            .await
            .unwrap();

        let fetched = get_payment_endpoint(&setup.reader_transport, &setup.public_key, &method)
            .await
            .unwrap();
        assert_eq!(fetched, Some(endpoint.clone()));

        let list = get_payment_list(&setup.reader_transport, &setup.public_key)
            .await
            .unwrap();
        assert_eq!(
            list,
            SupportedPayments {
                entries: vec![(method.clone(), endpoint.clone())]
                    .into_iter()
                    .collect()
            }
        );

        let new_endpoint = EndpointData("{\"address\":\"1c1...\"}".into());
        set_payment_endpoint(
            &setup.session_transport,
            method.clone(),
            new_endpoint.clone(),
        )
        .await
        .unwrap();

        let updated = get_payment_endpoint(&setup.reader_transport, &setup.public_key, &method)
            .await
            .unwrap();
        assert_eq!(updated, Some(new_endpoint.clone()));

        setup.raw_session.signout().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires network access - run with: cargo test --features integration-tests -- --ignored
    async fn missing_endpoint_returns_none() {
        let setup = TestSetup::new()
            .await
            .expect("Failed to create test setup - network may be unavailable");
        let method = MethodId("bolt11".into());

        let missing = get_payment_endpoint(&setup.reader_transport, &setup.public_key, &method)
            .await
            .unwrap();
        assert!(missing.is_none());

        setup.raw_session.signout().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires network access - run with: cargo test --features integration-tests -- --ignored
    async fn list_reflects_additions_and_removals() {
        let setup = TestSetup::new()
            .await
            .expect("Failed to create test setup - network may be unavailable");

        let onchain = MethodId("onchain".into());
        let lightning = MethodId("lightning".into());
        let onchain_data = EndpointData("{\"address\":\"bc1...\"}".into());
        let lightning_data = EndpointData("{\"bolt11\":\"ln...\"}".into());

        set_payment_endpoint(
            &setup.session_transport,
            onchain.clone(),
            onchain_data.clone(),
        )
        .await
        .unwrap();
        set_payment_endpoint(
            &setup.session_transport,
            lightning.clone(),
            lightning_data.clone(),
        )
        .await
        .unwrap();

        let list = get_payment_list(&setup.reader_transport, &setup.public_key)
            .await
            .unwrap();
        let mut expected = HashMap::new();
        expected.insert(onchain.clone(), onchain_data.clone());
        expected.insert(lightning.clone(), lightning_data.clone());
        assert_eq!(list.entries, expected);

        remove_payment_endpoint(&setup.session_transport, onchain.clone())
            .await
            .unwrap();
        let list = get_payment_list(&setup.reader_transport, &setup.public_key)
            .await
            .unwrap();
        assert_eq!(
            list.entries,
            vec![(lightning.clone(), lightning_data.clone())]
                .into_iter()
                .collect()
        );

        remove_payment_endpoint(&setup.session_transport, lightning.clone())
            .await
            .unwrap();
        let empty = get_payment_list(&setup.reader_transport, &setup.public_key)
            .await
            .unwrap();
        assert!(empty.entries.is_empty());

        setup.raw_session.signout().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires network access - run with: cargo test --features integration-tests -- --ignored
    async fn removing_missing_endpoint_is_error() {
        let setup = TestSetup::new()
            .await
            .expect("Failed to create test setup - network may be unavailable");
        let method = MethodId("unused".into());

        remove_payment_endpoint(&setup.session_transport, method)
            .await
            .expect_err("removing non-existent endpoint should fail");

        setup.raw_session.signout().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires network access - run with: cargo test --features integration-tests -- --ignored
    async fn lists_known_contacts() {
        let setup = TestSetup::new()
            .await
            .expect("Failed to create test setup - network may be unavailable");

        let contacts = get_known_contacts(&setup.reader_transport, &setup.public_key)
            .await
            .unwrap();
        assert!(contacts.is_empty());

        // Seed two contacts under the follows path using the authenticated session.
        let contact_a = Keypair::random().public_key();
        let contact_b = Keypair::random().public_key();
        setup
            .raw_session
            .storage()
            .put(format!("{PUBKY_FOLLOWS_PATH}{}", contact_a), "")
            .await
            .unwrap();
        setup
            .raw_session
            .storage()
            .put(format!("{PUBKY_FOLLOWS_PATH}{}", contact_b), "")
            .await
            .unwrap();

        let contacts = get_known_contacts(&setup.reader_transport, &setup.public_key)
            .await
            .unwrap();

        assert!(contacts.contains(&contact_a));
        assert!(contacts.contains(&contact_b));

        setup.raw_session.signout().await.unwrap();
    }
}
