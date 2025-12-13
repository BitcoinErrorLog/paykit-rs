//! LND REST API executor implementation.
//!
//! Connects to LND nodes via their REST API for Lightning payments.
//!
//! # Feature Flags
//!
//! This module requires the `http-executor` feature flag to be enabled for actual
//! HTTP requests. Without it, all requests return an `Unimplemented` error.
//!
//! ```toml
//! [dependencies]
//! paykit-lib = { version = "1.0", features = ["http-executor"] }
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use paykit_lib::executors::{LndConfig, LndExecutor};
//! use paykit_lib::methods::LightningExecutor;
//!
//! let config = LndConfig::new("https://localhost:8080", "your_macaroon_hex");
//! let executor = LndExecutor::new(config)?;
//!
//! // Decode an invoice
//! let decoded = executor.decode_invoice("lnbc...").await?;
//! println!("Amount: {:?} msat", decoded.amount_msat);
//!
//! // Pay an invoice
//! let result = executor.pay_invoice("lnbc...", None, None).await?;
//! println!("Payment preimage: {}", result.preimage);
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[cfg(feature = "http-executor")]
use std::time::Duration;

use super::config::LndConfig;
use crate::methods::{
    DecodedInvoice, LightningExecutor, LightningPaymentResult, LightningPaymentStatus,
};
use crate::{PaykitError, Result};

/// LND REST API executor for Lightning payments.
///
/// This executor connects to an LND node via its REST API to execute
/// Lightning Network payments, decode invoices, and estimate fees.
///
/// # Security
///
/// The macaroon is sent with each request for authentication. Ensure you use
/// HTTPS in production and consider using a restricted macaroon with minimal
/// permissions needed for your use case.
#[derive(Debug)]
pub struct LndExecutor {
    config: LndConfig,
    #[cfg(feature = "http-executor")]
    client: reqwest::Client,
}

impl LndExecutor {
    /// Create a new LND executor with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The REST URL is empty
    /// - The macaroon is empty
    /// - (With `http-executor` feature) The HTTP client cannot be built
    pub fn new(config: LndConfig) -> Result<Self> {
        // Validate configuration
        if config.rest_url.is_empty() {
            return Err(PaykitError::InvalidData {
                field: "rest_url".to_string(),
                reason: "REST URL cannot be empty".to_string(),
            });
        }
        if config.macaroon_hex.is_empty() {
            return Err(PaykitError::InvalidData {
                field: "macaroon_hex".to_string(),
                reason: "Macaroon cannot be empty".to_string(),
            });
        }

        #[cfg(feature = "http-executor")]
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .danger_accept_invalid_certs(config.tls_cert_pem.is_some()) // Accept self-signed if cert provided
            .build()
            .map_err(|e| PaykitError::Internal(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            config,
            #[cfg(feature = "http-executor")]
            client,
        })
    }

    /// Get the configuration.
    pub fn config(&self) -> &LndConfig {
        &self.config
    }

    /// Build the full URL for an API endpoint.
    #[cfg(feature = "http-executor")]
    fn url(&self, path: &str) -> String {
        format!("{}/v1/{}", self.config.rest_url.trim_end_matches('/'), path)
    }

    /// Build the full URL for an API endpoint (for tests).
    #[cfg(all(not(feature = "http-executor"), test))]
    fn url(&self, path: &str) -> String {
        format!("{}/v1/{}", self.config.rest_url.trim_end_matches('/'), path)
    }

    /// Make an authenticated GET request.
    #[cfg(feature = "http-executor")]
    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = self.url(path);

        let response = self
            .client
            .get(&url)
            .header("Grpc-Metadata-macaroon", &self.config.macaroon_hex)
            .send()
            .await
            .map_err(|e| self.map_reqwest_error(e))?;

        self.handle_response(response).await
    }

    /// Make an authenticated GET request (stub when feature disabled).
    #[cfg(not(feature = "http-executor"))]
    async fn get<T: for<'de> Deserialize<'de>>(&self, _path: &str) -> Result<T> {
        Err(PaykitError::Unimplemented(
            "LND HTTP client not compiled - enable the 'http-executor' feature",
        ))
    }

    /// Make an authenticated POST request.
    #[cfg(feature = "http-executor")]
    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.url(path);

        let response = self
            .client
            .post(&url)
            .header("Grpc-Metadata-macaroon", &self.config.macaroon_hex)
            .json(body)
            .send()
            .await
            .map_err(|e| self.map_reqwest_error(e))?;

        self.handle_response(response).await
    }

    /// Make an authenticated POST request (stub when feature disabled).
    #[cfg(not(feature = "http-executor"))]
    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        _path: &str,
        _body: &B,
    ) -> Result<T> {
        Err(PaykitError::Unimplemented(
            "LND HTTP client not compiled - enable the 'http-executor' feature",
        ))
    }

    /// Handle an HTTP response, parsing JSON or returning an error.
    #[cfg(feature = "http-executor")]
    async fn handle_response<T: for<'de> Deserialize<'de>>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            return match status.as_u16() {
                401 | 403 => Err(PaykitError::Auth(format!(
                    "LND authentication failed: {}",
                    error_text
                ))),
                404 => Err(PaykitError::NotFound {
                    resource_type: "LND resource".to_string(),
                    identifier: error_text,
                }),
                429 => Err(PaykitError::RateLimited {
                    retry_after_ms: 5000,
                }),
                500..=599 => Err(PaykitError::Internal(format!(
                    "LND server error ({}): {}",
                    status, error_text
                ))),
                _ => Err(PaykitError::Transport(format!(
                    "LND request failed ({}): {}",
                    status, error_text
                ))),
            };
        }

        response
            .json::<T>()
            .await
            .map_err(|e| PaykitError::Serialization(format!("Failed to parse LND response: {}", e)))
    }

    /// Map reqwest errors to PaykitError.
    #[cfg(feature = "http-executor")]
    fn map_reqwest_error(&self, e: reqwest::Error) -> PaykitError {
        if e.is_timeout() {
            PaykitError::ConnectionTimeout {
                operation: "LND request".to_string(),
                timeout_ms: self.config.timeout_secs * 1000,
            }
        } else if e.is_connect() {
            PaykitError::ConnectionFailed {
                target: self.config.rest_url.clone(),
                reason: e.to_string(),
            }
        } else {
            PaykitError::Transport(format!("LND request failed: {}", e))
        }
    }
}

#[async_trait]
impl LightningExecutor for LndExecutor {
    async fn pay_invoice(
        &self,
        invoice: &str,
        amount_msat: Option<u64>,
        max_fee_msat: Option<u64>,
    ) -> Result<LightningPaymentResult> {
        // Build pay request
        let pay_req = LndPayReq {
            payment_request: invoice.to_string(),
            amt_msat: amount_msat.map(|a| a.to_string()),
            fee_limit_msat: max_fee_msat
                .or_else(|| {
                    // Calculate max fee from config percentage
                    amount_msat.map(|a| ((a as f64) * (self.config.max_fee_percent / 100.0)) as u64)
                })
                .map(|f| f.to_string()),
            timeout_seconds: Some(self.config.timeout_secs as i32),
            no_inflight_updates: Some(true),
        };

        // Send payment
        let response: LndPayResponse = self.post("channels/transactions", &pay_req).await?;

        // Check for error
        if !response.payment_error.is_empty() {
            return Err(PaykitError::Payment {
                payment_id: Some(response.payment_hash.clone()),
                reason: response.payment_error,
            });
        }

        let status = if response.payment_preimage.is_empty() {
            LightningPaymentStatus::Failed
        } else {
            LightningPaymentStatus::Succeeded
        };

        Ok(LightningPaymentResult {
            preimage: response.payment_preimage,
            payment_hash: response.payment_hash,
            amount_msat: amount_msat.unwrap_or(0),
            fee_msat: response
                .payment_route
                .as_ref()
                .map(|r| r.total_fees_msat.parse().unwrap_or(0))
                .unwrap_or(0),
            hops: response
                .payment_route
                .as_ref()
                .map(|r| r.hops.len() as u32)
                .unwrap_or(0),
            status,
        })
    }

    async fn decode_invoice(&self, invoice: &str) -> Result<DecodedInvoice> {
        let response: LndDecodeResponse = self.get(&format!("payreq/{}", invoice)).await?;

        let timestamp = response.timestamp.parse::<u64>().unwrap_or(0);
        let expiry = response.expiry.parse::<u64>().unwrap_or(3600);
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(DecodedInvoice {
            payment_hash: response.payment_hash,
            amount_msat: response
                .num_msat
                .as_ref()
                .and_then(|s| s.parse::<u64>().ok()),
            description: if response.description.is_empty() {
                None
            } else {
                Some(response.description)
            },
            description_hash: if response.description_hash.is_empty() {
                None
            } else {
                Some(response.description_hash)
            },
            payee: response.destination,
            expiry,
            timestamp,
            expired: current_time > timestamp + expiry,
        })
    }

    async fn estimate_fee(&self, invoice: &str) -> Result<u64> {
        // Query route to estimate fees
        let decoded = self.decode_invoice(invoice).await?;
        let amount_msat = decoded.amount_msat.unwrap_or(0);

        // Use query routes endpoint
        let query = LndQueryRoutesRequest {
            pub_key: decoded.payee.clone(),
            amt_msat: amount_msat.to_string(),
        };

        let response: LndQueryRoutesResponse = self.post("graph/routes", &query).await?;

        // Get fee from first route
        let fee = response
            .routes
            .first()
            .map(|r| r.total_fees_msat.parse::<u64>().unwrap_or(0))
            .unwrap_or(0);

        Ok(fee)
    }

    async fn get_payment(&self, payment_hash: &str) -> Result<Option<LightningPaymentResult>> {
        // List payments and find matching one
        let response: LndListPaymentsResponse = self.get("payments").await?;

        let payment = response
            .payments
            .iter()
            .find(|p| p.payment_hash == payment_hash);

        Ok(payment.map(|p| LightningPaymentResult {
            preimage: p.payment_preimage.clone(),
            payment_hash: p.payment_hash.clone(),
            amount_msat: p.value_msat.parse().unwrap_or(0),
            fee_msat: p.fee_msat.parse().unwrap_or(0),
            hops: 0, // Not available in list response
            status: match p.status.as_str() {
                "SUCCEEDED" => LightningPaymentStatus::Succeeded,
                "IN_FLIGHT" => LightningPaymentStatus::Pending,
                _ => LightningPaymentStatus::Failed,
            },
        }))
    }
}

// ============================================================================
// LND REST API Types
// ============================================================================

#[derive(Serialize)]
struct LndPayReq {
    payment_request: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    amt_msat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fee_limit_msat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_seconds: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_inflight_updates: Option<bool>,
}

#[derive(Deserialize)]
struct LndPayResponse {
    #[serde(default)]
    payment_preimage: String,
    #[serde(default)]
    payment_hash: String,
    #[serde(default)]
    payment_error: String,
    payment_route: Option<LndRoute>,
}

#[derive(Deserialize)]
struct LndRoute {
    #[serde(default)]
    total_fees_msat: String,
    #[serde(default)]
    hops: Vec<LndHop>,
}

/// LND hop information from route.
#[derive(Deserialize)]
#[allow(dead_code)] // Fields required for serde deserialization from LND API
struct LndHop {
    #[serde(default)]
    chan_id: String,
}

#[derive(Deserialize)]
struct LndDecodeResponse {
    #[serde(default)]
    destination: String,
    #[serde(default)]
    payment_hash: String,
    #[serde(default)]
    num_msat: Option<String>,
    #[serde(default)]
    timestamp: String,
    #[serde(default)]
    expiry: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    description_hash: String,
}

#[derive(Serialize)]
struct LndQueryRoutesRequest {
    pub_key: String,
    amt_msat: String,
}

#[derive(Deserialize)]
struct LndQueryRoutesResponse {
    #[serde(default)]
    routes: Vec<LndRoute>,
}

#[derive(Deserialize)]
struct LndListPaymentsResponse {
    #[serde(default)]
    payments: Vec<LndPayment>,
}

#[derive(Deserialize)]
struct LndPayment {
    #[serde(default)]
    payment_hash: String,
    #[serde(default)]
    payment_preimage: String,
    #[serde(default)]
    value_msat: String,
    #[serde(default)]
    fee_msat: String,
    #[serde(default)]
    status: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lnd_executor_creation() {
        let config = LndConfig::new("https://localhost:8080", "macaroon123");
        let executor = LndExecutor::new(config).unwrap();

        assert_eq!(executor.config().rest_url, "https://localhost:8080");
    }

    #[test]
    fn test_lnd_executor_validation_empty_url() {
        let config = LndConfig::new("", "macaroon123");
        let result = LndExecutor::new(config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("REST URL"));
    }

    #[test]
    fn test_lnd_executor_validation_empty_macaroon() {
        let config = LndConfig::new("https://localhost:8080", "");
        let result = LndExecutor::new(config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Macaroon"));
    }

    #[test]
    fn test_url_building() {
        let config = LndConfig::new("https://localhost:8080/", "macaroon123");
        let executor = LndExecutor::new(config).unwrap();

        assert_eq!(
            executor.url("channels/transactions"),
            "https://localhost:8080/v1/channels/transactions"
        );
    }

    #[test]
    fn test_url_building_no_trailing_slash() {
        let config = LndConfig::new("https://localhost:8080", "macaroon123");
        let executor = LndExecutor::new(config).unwrap();

        assert_eq!(
            executor.url("payreq/lnbc123"),
            "https://localhost:8080/v1/payreq/lnbc123"
        );
    }
}
