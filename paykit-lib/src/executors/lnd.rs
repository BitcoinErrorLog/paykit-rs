//! LND REST API executor implementation.
//!
//! Connects to LND nodes via their REST API for Lightning payments.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::config::LndConfig;
use crate::methods::{
    DecodedInvoice, LightningExecutor, LightningPaymentResult, LightningPaymentStatus,
};
use crate::{PaykitError, Result};

/// LND REST API executor for Lightning payments.
pub struct LndExecutor {
    config: LndConfig,
}

impl LndExecutor {
    /// Create a new LND executor with the given configuration.
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

        Ok(Self { config })
    }

    /// Get the configuration.
    pub fn config(&self) -> &LndConfig {
        &self.config
    }

    /// Build the full URL for an API endpoint.
    ///
    /// Note: Currently unused as full HTTP client implementation is pending.
    /// Will be used when REST API integration is complete.
    #[allow(dead_code)]
    fn url(&self, path: &str) -> String {
        format!("{}/v1/{}", self.config.rest_url.trim_end_matches('/'), path)
    }

    /// Make an authenticated GET request.
    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        self.request("GET", path, Option::<&()>::None).await
    }

    /// Make an authenticated POST request.
    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request("POST", path, Some(body)).await
    }

    /// Make an authenticated HTTP request.
    async fn request<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        _method: &str,
        path: &str,
        _body: Option<&B>,
    ) -> Result<T> {
        // This is a stub implementation.
        // In a full implementation, you would use reqwest or another HTTP client:
        //
        // ```
        // let client = reqwest::Client::builder()
        //     .timeout(Duration::from_secs(self.config.timeout_secs))
        //     .build()?;
        //
        // let mut request = match method {
        //     "GET" => client.get(&self.url(path)),
        //     "POST" => client.post(&self.url(path)),
        //     _ => return Err(PaykitError::Internal("Invalid method".to_string())),
        // };
        //
        // request = request.header("Grpc-Metadata-macaroon", &self.config.macaroon_hex);
        //
        // if let Some(body) = body {
        //     request = request.json(body);
        // }
        //
        // let response = request.send().await?;
        // let result: T = response.json().await?;
        // Ok(result)
        // ```

        let _ = path;
        Err(PaykitError::Unimplemented(
            "LND HTTP client not compiled - add reqwest dependency",
        ))
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

// LND REST API types

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
/// Fields match the LND REST API response schema.
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
    fn test_lnd_executor_validation() {
        let config = LndConfig::new("", "macaroon123");
        let result = LndExecutor::new(config);
        assert!(result.is_err());

        let config = LndConfig::new("https://localhost:8080", "");
        let result = LndExecutor::new(config);
        assert!(result.is_err());
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
}
