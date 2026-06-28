//! Typed errors for the client layer.

use pricefeed_types::{FeedId, PriceError};

/// Errors surfaced by a [`crate::PriceClient`] or a [`crate::PriceTransport`].
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The requested feed was not served by the transport.
    #[error("feed not found: {0}")]
    NotFound(FeedId),

    /// A domain invariant was violated while interpreting the data.
    #[error("invalid price data: {0}")]
    Domain(#[from] PriceError),

    /// The transport timed out before producing a response.
    #[error("request timed out after {elapsed_ms}ms")]
    Timeout {
        /// How long the caller waited.
        elapsed_ms: u128,
    },

    /// The transport failed for an underlying (network/protocol) reason.
    #[error("transport error: {message}")]
    Transport {
        /// A human-readable description of the failure.
        message: String,
        /// Whether retrying the request might succeed.
        retryable: bool,
    },
}

impl ClientError {
    /// Convenience constructor for a retryable transport failure.
    #[must_use]
    pub fn transport(message: impl Into<String>) -> Self {
        Self::Transport {
            message: message.into(),
            retryable: true,
        }
    }

    /// Convenience constructor for a permanent transport failure.
    #[must_use]
    pub fn permanent(message: impl Into<String>) -> Self {
        Self::Transport {
            message: message.into(),
            retryable: false,
        }
    }

    /// Whether retrying this error could plausibly succeed.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout { .. } => true,
            Self::Transport { retryable, .. } => *retryable,
            Self::NotFound(_) | Self::Domain(_) => false,
        }
    }

    /// A stable, machine-readable code for cross-language parity.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "not_found",
            Self::Domain(_) => "domain",
            Self::Timeout { .. } => "timeout",
            Self::Transport { .. } => "transport",
        }
    }
}
