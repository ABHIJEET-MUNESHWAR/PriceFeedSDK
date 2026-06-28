//! The validated feed identifier newtype.

use serde::{Deserialize, Serialize};

use crate::error::PriceError;

/// Maximum length of a feed symbol (e.g. `BTC/USD`).
pub const MAX_FEED_ID_LEN: usize = 32;

/// A validated, normalized feed symbol.
///
/// Construction enforces the invariant once, so every downstream consumer can
/// rely on a non-empty, bounded, uppercase symbol drawn from `[A-Z0-9./-]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FeedId(String);

impl FeedId {
    /// Validates and normalizes `symbol` into a [`FeedId`].
    ///
    /// # Errors
    /// Returns [`PriceError`] if the symbol is empty, too long, or contains a
    /// character outside `[A-Za-z0-9./-]`.
    pub fn new(symbol: impl Into<String>) -> Result<Self, PriceError> {
        let raw = symbol.into();
        if raw.is_empty() {
            return Err(PriceError::EmptyFeed);
        }
        if raw.len() > MAX_FEED_ID_LEN {
            return Err(PriceError::FeedIdTooLong {
                max: MAX_FEED_ID_LEN,
            });
        }
        for ch in raw.chars() {
            let ok = ch.is_ascii_alphanumeric() || matches!(ch, '.' | '/' | '-');
            if !ok {
                return Err(PriceError::InvalidFeedChar { ch });
            }
        }
        Ok(Self(raw.to_ascii_uppercase()))
    }

    /// The normalized symbol as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FeedId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for FeedId {
    type Error = PriceError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<FeedId> for String {
    fn from(id: FeedId) -> Self {
        id.0
    }
}

impl std::str::FromStr for FeedId {
    type Err = PriceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}
