//! HTTP/GraphQL transport implementation.

use async_trait::async_trait;
use pricefeed_client::{ClientError, PriceTransport};
use pricefeed_types::{FeedId, PriceUpdate};
use serde_json::json;

use crate::error::HttpError;
use crate::wire::parse_price_response;

const PRICE_QUERY: &str = r"query($feed: String!) {
  price(feedId: $feed) {
    feedId
    mantissa
    conf
    expo
    status
    publishTime
  }
}";

/// A [`PriceTransport`] backed by a GraphQL price endpoint.
///
/// Compatible with the OracleForge / OracleBridge GraphQL surfaces: it issues a
/// `price(feedId:)` query and maps the response into the shared domain types.
#[derive(Debug, Clone)]
pub struct HttpTransport {
    client: reqwest::Client,
    endpoint: String,
}

impl HttpTransport {
    /// Builds a transport targeting `endpoint` (e.g. `http://localhost:8080/graphql`).
    ///
    /// # Errors
    /// Returns [`HttpError::Network`] if the underlying client cannot be built.
    pub fn new(endpoint: impl Into<String>) -> Result<Self, HttpError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| HttpError::Network(e.to_string()))?;
        Ok(Self {
            client,
            endpoint: endpoint.into(),
        })
    }

    /// Builds a transport from a pre-configured [`reqwest::Client`].
    #[must_use]
    pub fn with_client(client: reqwest::Client, endpoint: impl Into<String>) -> Self {
        Self {
            client,
            endpoint: endpoint.into(),
        }
    }

    async fn query(&self, feed: &FeedId) -> Result<PriceUpdate, HttpError> {
        let body = json!({
            "query": PRICE_QUERY,
            "variables": { "feed": feed.as_str() },
        });

        let resp = self
            .client
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| HttpError::Network(e.to_string()))?;

        let resp = resp
            .error_for_status()
            .map_err(|e| HttpError::Network(e.to_string()))?;

        let text = resp
            .text()
            .await
            .map_err(|e| HttpError::Network(e.to_string()))?;

        parse_price_response(&text, feed)
    }
}

#[async_trait]
impl PriceTransport for HttpTransport {
    async fn fetch(&self, feed: &FeedId) -> Result<PriceUpdate, ClientError> {
        self.query(feed).await.map_err(Into::into)
    }
}
