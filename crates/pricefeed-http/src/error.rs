//! Error types for the HTTP transport.

use pricefeed_types::{FeedId, PriceError};

/// Errors raised by the HTTP/GraphQL transport.
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    /// The HTTP request itself failed (DNS, connect, timeout, status).
    #[error("network error: {0}")]
    Network(String),

    /// The response body could not be decoded as JSON.
    #[error("decode error: {0}")]
    Decode(String),

    /// The GraphQL server returned an `errors` array.
    #[error("graphql error: {0}")]
    GraphQl(String),

    /// The server returned a null price for the requested feed.
    #[error("feed not found: {0}")]
    NotFound(FeedId),

    /// The response violated a domain invariant.
    #[error(transparent)]
    Domain(#[from] PriceError),
}

impl HttpError {
    /// Whether retrying might succeed (network failures only).
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Network(_))
    }
}

impl From<HttpError> for pricefeed_client::ClientError {
    fn from(err: HttpError) -> Self {
        use pricefeed_client::ClientError;
        match err {
            HttpError::NotFound(feed) => ClientError::NotFound(feed),
            HttpError::Domain(e) => ClientError::Domain(e),
            HttpError::Network(message) => ClientError::Transport {
                message,
                retryable: true,
            },
            HttpError::Decode(message) | HttpError::GraphQl(message) => ClientError::Transport {
                message,
                retryable: false,
            },
        }
    }
}
