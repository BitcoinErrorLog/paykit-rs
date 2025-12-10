use std::collections::HashMap;

use async_trait::async_trait;
use pubky::{
    errors::RequestError, Error as PubkyError, PubkyResource,
    PublicStorage as SdkUnauthenticatedTransport, StatusCode,
};

use super::{PAYKIT_PATH_PREFIX, PUBKY_FOLLOWS_PATH};
use crate::transport::traits::UnauthenticatedTransportRead;
use crate::{EndpointData, MethodId, PaykitError, PublicKey, Result, SupportedPayments};

/// Adapter around `pubky::PublicStorage` implementing `UnauthenticatedTransportRead`.
#[derive(Clone)]
pub struct PubkyUnauthenticatedTransport {
    inner: SdkUnauthenticatedTransport,
}

impl PubkyUnauthenticatedTransport {
    /// Build an adapter from an existing SDK handle.
    pub fn new(inner: SdkUnauthenticatedTransport) -> Self {
        Self { inner }
    }

    /// Attempt to construct the underlying SDK transport via `pubky::PublicStorage::new()`.
    pub fn try_new() -> Result<Self> {
        let inner = SdkUnauthenticatedTransport::new().map_err(|err| {
            PaykitError::Transport(format!("failed to create Pubky public transport: {err}"))
        })?;
        Ok(Self { inner })
    }

    /// Access the wrapped SDK transport handle.
    pub fn inner(&self) -> &SdkUnauthenticatedTransport {
        &self.inner
    }

    async fn fetch_text(&self, addr: String, label: &str) -> Result<Option<String>> {
        match self.inner.get(&addr).await {
            Ok(resp) => {
                let bytes = resp
                    .bytes()
                    .await
                    .map_err(|err| PaykitError::Transport(format!("{label}: {err}")))?;
                if bytes.is_empty() {
                    return Ok(None);
                }
                let data = String::from_utf8(bytes.to_vec())
                    .map_err(|err| PaykitError::Transport(format!("{label}: {err}")))?;
                Ok(Some(data))
            }
            Err(err) if is_not_found(&err) => Ok(None),
            Err(err) => Err(PaykitError::Transport(format!("{label}: {err}"))),
        }
    }

    async fn list_entries(&self, addr: String, label: &str) -> Result<Vec<PubkyResource>> {
        let builder = match self.inner.list(&addr) {
            Ok(builder) => builder,
            Err(err) if is_not_found(&err) => return Ok(Vec::new()),
            Err(err) => return Err(PaykitError::Transport(format!("{label}: {err}"))),
        };

        match builder.shallow(true).send().await {
            Ok(entries) => Ok(entries),
            Err(err) if is_not_found(&err) => Ok(Vec::new()),
            Err(err) => Err(PaykitError::Transport(format!(
                "{label} send failed: {err}"
            ))),
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl UnauthenticatedTransportRead for PubkyUnauthenticatedTransport {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    async fn fetch_supported_payments(&self, payee: &PublicKey) -> Result<SupportedPayments> {
        let addr = format!("pubky{payee}{PAYKIT_PATH_PREFIX}");
        let entries = self.list_entries(addr, "list supported payments").await?;

        let mut map = HashMap::new();
        for resource in entries {
            if resource.path.as_str().ends_with('/') {
                continue;
            }

            let method = resource
                .path
                .as_str()
                .rsplit('/')
                .next()
                .filter(|segment| !segment.is_empty())
                .ok_or_else(|| {
                    PaykitError::Transport(
                        "invalid resource returned for supported payment entry".into(),
                    )
                })?
                .to_string();

            let label = format!("fetch endpoint {}", method);
            if let Some(payload) = self.fetch_text(resource.to_string(), &label).await? {
                map.insert(MethodId(method), EndpointData(payload));
            }
        }

        Ok(SupportedPayments { entries: map })
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), fields(method = %method.0)))]
    async fn fetch_payment_endpoint(
        &self,
        payee: &PublicKey,
        method: &MethodId,
    ) -> Result<Option<EndpointData>> {
        let addr = format!("pubky{payee}{PAYKIT_PATH_PREFIX}{}", method.0);
        match self.fetch_text(addr, "fetch endpoint").await? {
            Some(payload) => Ok(Some(EndpointData(payload))),
            None => Ok(None),
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    async fn fetch_known_contacts(&self, owner: &PublicKey) -> Result<Vec<PublicKey>> {
        let addr = format!("pubky{owner}{PUBKY_FOLLOWS_PATH}");
        let entries = self.list_entries(addr, "list known contacts").await?;

        let mut contacts = Vec::new();
        for resource in entries {
            if resource.path.as_str().ends_with('/') {
                continue;
            }
            let name = resource
                .path
                .as_str()
                .rsplit('/')
                .next()
                .filter(|segment| !segment.is_empty());
            if let Some(pk_str) = name {
                match pk_str.parse::<PublicKey>() {
                    Ok(pk) => contacts.push(pk),
                    Err(err) => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!("invalid contact entry '{pk_str}': {err}");
                        #[cfg(not(feature = "tracing"))]
                        eprintln!("invalid contact entry '{pk_str}': {err}");
                        continue;
                    }
                }
            }
        }

        Ok(contacts)
    }

    async fn get(&self, owner: &PublicKey, path: &str) -> Result<Option<String>> {
        let addr = format!("pubky{owner}{path}");
        self.fetch_text(addr, "get").await
    }

    async fn list_directory(&self, owner: &PublicKey, path: &str) -> Result<Vec<String>> {
        let addr = format!("pubky{owner}{path}");
        let entries = self.list_entries(addr, "list directory").await?;

        let names: Vec<String> = entries
            .into_iter()
            .filter_map(|resource| {
                resource
                    .path
                    .as_str()
                    .rsplit('/')
                    .next()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
            })
            .collect();

        Ok(names)
    }
}

fn is_not_found(err: &PubkyError) -> bool {
    matches!(
        err,
        PubkyError::Request(RequestError::Server { status, .. })
            if *status == StatusCode::NOT_FOUND || *status == StatusCode::GONE
    )
}
