//! Wire model and parsing for GraphQL price responses.
//!
//! Kept separate from the network code so it can be unit-tested offline.

use pricefeed_types::{FeedId, Price, PriceStatus, PriceUpdate};
use serde::Deserialize;

use crate::error::HttpError;

/// The `price` field of a GraphQL response payload.
#[derive(Debug, Deserialize)]
pub(crate) struct WirePrice {
    #[serde(rename = "feedId")]
    pub feed_id: String,
    pub mantissa: i64,
    pub conf: u64,
    pub expo: i32,
    pub status: String,
    #[serde(rename = "publishTime")]
    pub publish_time: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WireData {
    pub price: Option<WirePrice>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WireError {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WireResponse {
    pub data: Option<WireData>,
    pub errors: Option<Vec<WireError>>,
}

fn status_from_str(raw: &str) -> PriceStatus {
    match raw.to_ascii_lowercase().as_str() {
        "trading" => PriceStatus::Trading,
        "halted" => PriceStatus::Halted,
        _ => PriceStatus::Unknown,
    }
}

impl WirePrice {
    pub(crate) fn into_update(self) -> Result<PriceUpdate, HttpError> {
        let feed = FeedId::new(self.feed_id)?;
        let price = Price::new(self.mantissa, self.conf, self.expo);
        let update = PriceUpdate::new(
            feed,
            price,
            status_from_str(&self.status),
            self.publish_time,
        )?;
        Ok(update)
    }
}

/// Parses a raw GraphQL JSON body into a [`PriceUpdate`].
///
/// # Errors
/// Returns [`HttpError`] if the body is malformed, carries GraphQL errors, or
/// contains a null `price` (treated as "not found").
pub fn parse_price_response(body: &str, feed: &FeedId) -> Result<PriceUpdate, HttpError> {
    let parsed: WireResponse =
        serde_json::from_str(body).map_err(|e| HttpError::Decode(e.to_string()))?;

    if let Some(errors) = parsed.errors {
        if let Some(first) = errors.into_iter().next() {
            return Err(HttpError::GraphQl(first.message));
        }
    }

    let price = parsed
        .data
        .and_then(|d| d.price)
        .ok_or_else(|| HttpError::NotFound(feed.clone()))?;

    price.into_update()
}
