//! Error types for the domain layer.

/// Errors produced while constructing or interpreting price data.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PriceError {
    /// The feed symbol was empty.
    #[error("feed id must not be empty")]
    EmptyFeed,

    /// The feed symbol exceeded [`crate::MAX_FEED_ID_LEN`].
    #[error("feed id exceeds the maximum length of {max}")]
    FeedIdTooLong {
        /// The configured maximum.
        max: usize,
    },

    /// The feed symbol contained a disallowed character.
    #[error("feed id contains an invalid character: {ch:?}")]
    InvalidFeedChar {
        /// The offending character.
        ch: char,
    },

    /// Confidence must be strictly positive.
    #[error("confidence must be greater than zero")]
    ZeroConfidence,

    /// The publish time was not a positive unix timestamp.
    #[error("publish time must be positive, got {0}")]
    NonPositiveTimestamp(i64),

    /// A read was attempted on a feed that is not currently trading.
    #[error("feed is not trading (status: {status})")]
    NotTrading {
        /// The status that blocked the read.
        status: &'static str,
    },

    /// The price was older than the caller's freshness bound.
    #[error("price is stale: age {age}s exceeds the maximum of {max_age}s")]
    Stale {
        /// The observed age in seconds.
        age: i64,
        /// The caller-supplied maximum age in seconds.
        max_age: i64,
    },
}

impl PriceError {
    /// A stable, machine-readable code for cross-language parity.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyFeed => "empty_feed",
            Self::FeedIdTooLong { .. } => "feed_id_too_long",
            Self::InvalidFeedChar { .. } => "invalid_feed_char",
            Self::ZeroConfidence => "zero_confidence",
            Self::NonPositiveTimestamp(_) => "non_positive_timestamp",
            Self::NotTrading { .. } => "not_trading",
            Self::Stale { .. } => "stale",
        }
    }
}
