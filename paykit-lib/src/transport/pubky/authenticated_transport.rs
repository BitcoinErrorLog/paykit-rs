use async_trait::async_trait;
use pubky::PubkySession;

use super::PAYKIT_PATH_PREFIX;
use crate::transport::traits::AuthenticatedTransport;
use crate::{EndpointData, MethodId, PaykitError, Result};

/// Adapter around `pubky::PubkySession` implementing `AuthenticatedTransport`.
#[derive(Clone)]
pub struct PubkyAuthenticatedTransport {
    session: PubkySession,
}

impl PubkyAuthenticatedTransport {
    /// Create a new adapter from an existing session.
    pub fn new(session: PubkySession) -> Self {
        Self { session }
    }

    /// Access the wrapped session for advanced payers/payees.
    pub fn session(&self) -> &PubkySession {
        &self.session
    }
}

impl From<PubkySession> for PubkyAuthenticatedTransport {
    fn from(session: PubkySession) -> Self {
        Self { session }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AuthenticatedTransport for PubkyAuthenticatedTransport {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, data), fields(method = %method.0, data_len = data.0.len())))]
    async fn upsert_payment_endpoint(&self, method: &MethodId, data: &EndpointData) -> Result<()> {
        let path = format!("{PAYKIT_PATH_PREFIX}{}", method.0);
        self.session
            .storage()
            .put(path, data.0.clone())
            .await
            .map_err(|err| PaykitError::Transport(format!("put endpoint: {err}")))?;
        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self), fields(method = %method.0)))]
    async fn remove_payment_endpoint(&self, method: &MethodId) -> Result<()> {
        let path = format!("{PAYKIT_PATH_PREFIX}{}", method.0);
        self.session
            .storage()
            .delete(path)
            .await
            .map_err(|err| PaykitError::Transport(format!("delete endpoint: {err}")))?;
        Ok(())
    }

    async fn put(&self, path: &str, content: &str) -> Result<()> {
        self.session
            .storage()
            .put(path.to_string(), content.to_string())
            .await
            .map_err(|err| PaykitError::Transport(format!("put: {err}")))?;
        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Option<String>> {
        match self.session.storage().get(path).await {
            Ok(response) => {
                let bytes = response.bytes().await
                    .map_err(|e| PaykitError::Transport(format!("get bytes: {e}")))?;
                if bytes.is_empty() {
                    return Ok(None);
                }
                let content = String::from_utf8_lossy(&bytes).to_string();
                Ok(Some(content))
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("404") || err_str.contains("not found") {
                    Ok(None)
                } else {
                    Err(PaykitError::Transport(format!("get: {e}")))
                }
            }
        }
    }

    async fn delete(&self, path: &str) -> Result<()> {
        self.session
            .storage()
            .delete(path.to_string())
            .await
            .map_err(|err| PaykitError::Transport(format!("delete: {err}")))?;
        Ok(())
    }
}
