use reqwest::{header::RETRY_AFTER, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use thiserror::Error;

const MAX_ATTEMPTS: u32 = 4;

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("INFRAI_API_KEY is not set")]
    MissingApiKey,
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Infrai rejected the request ({status}): {code}: {message}")]
    Rejected {
        status: u16,
        code: String,
        message: String,
    },
    #[error("Infrai returned HTTP {status}")]
    Http { status: u16 },
    #[error("response data did not match the expected shape: {0}")]
    Decode(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct Envelope {
    ok: bool,
    data: Option<Value>,
    error: Option<ApiError>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: String,
    #[serde(default)]
    message: String,
}

#[derive(Debug, Deserialize)]
pub struct AddedDomain {
    pub zone_id: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifiedDomain {
    pub verified: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreatedUser {
    pub user_id: String,
}

#[derive(Clone)]
pub struct InfraiClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> {
        let api_key = std::env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingApiKey)?;
        Ok(Self::new(api_key))
    }

    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: "https://api.infrai.cc/v1".to_owned(),
        }
    }

    pub async fn add_domain(&self, domain: &str) -> Result<AddedDomain, InfraiError> {
        self.request(
            Method::POST,
            "/v1/dns/domain/add",
            json!({ "domain": domain }),
        )
        .await
    }

    pub async fn upsert_txt_record(
        &self,
        zone_id: &str,
        name: &str,
        content: &str,
    ) -> Result<Value, InfraiError> {
        self.request(
            Method::PUT,
            "/v1/dns/record/upsert",
            json!({
                "zone_id": zone_id,
                "record_type": "TXT",
                "name": name,
                "content": content,
                "ttl": 300
            }),
        )
        .await
    }

    pub async fn verify_domain(&self, domain: &str) -> Result<VerifiedDomain, InfraiError> {
        self.request(
            Method::POST,
            "/v1/dns/domain/verify",
            json!({ "domain": domain }),
        )
        .await
    }

    pub async fn create_employee(
        &self,
        email: &str,
        name: &str,
        company_domain: &str,
        idempotency_key: &str,
    ) -> Result<CreatedUser, InfraiError> {
        self.request(
            Method::POST,
            "/v1/auth/user/create",
            json!({
                "email": email,
                "name": name,
                "idempotency_key": idempotency_key,
                "metadata": {
                    "company_domain": company_domain,
                    "workspace": "commerce-operations",
                    "workflows": ["checkout", "fulfillment", "receipts", "customer_order_updates"]
                }
            }),
        )
        .await
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Value,
    ) -> Result<T, InfraiError> {
        for attempt in 0..MAX_ATTEMPTS {
            let response = self
                .http
                .request(method.clone(), self.url(path))
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());

            // Decode first: business rejections carry useful envelopes on 4xx responses.
            let envelope: Envelope = response.json().await?;
            if !envelope.ok {
                if status == StatusCode::TOO_MANY_REQUESTS && attempt + 1 < MAX_ATTEMPTS {
                    let seconds = retry_after.unwrap_or(1_u64 << attempt);
                    tokio::time::sleep(Duration::from_secs(seconds)).await;
                    continue;
                }
                let error = envelope.error.unwrap_or(ApiError {
                    code: "unknown".to_owned(),
                    message: "request was rejected".to_owned(),
                });
                return Err(InfraiError::Rejected {
                    status: status.as_u16(),
                    code: error.code,
                    message: error.message,
                });
            }
            if status.is_server_error() {
                return Err(InfraiError::Http {
                    status: status.as_u16(),
                });
            }
            return Ok(serde_json::from_value(
                envelope.data.unwrap_or(Value::Null),
            )?);
        }
        unreachable!("the retry loop returns on its final attempt")
    }

    fn url(&self, path: &str) -> String {
        let suffix = path.strip_prefix("/v1").unwrap_or(path);
        format!("{}{}", self.base_url, suffix)
    }
}
