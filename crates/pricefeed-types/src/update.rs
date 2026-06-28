//! A timestamped price update for a specific feed, with staleness logic.

use serde::{Deserialize, Serialize};

use crate::error::PriceError;
use crate::ids::FeedId;
use crate::price::{Price, PriceStatus};

/// A single price observation for a feed at a point in time.
///
/// This is the unit a consumer receives from any transport. The freshness and
/// tradeability guards live here so every binding enforces them identically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceUpdate {
    /// The feed this update belongs to.
    pub feed_id: FeedId,
    /// The fixed-point price and confidence.
    pub price: Price,
    /// The feed status at publication time.
    pub status: PriceStatus,
    /// The source publish time (unix seconds).
    pub publish_time: i64,
}

impl PriceUpdate {
    /// Builds a validated update.
    ///
    /// # Errors
    /// Returns [`PriceError::ZeroConfidence`] if `price.conf == 0`, or
    /// [`PriceError::NonPositiveTimestamp`] if `publish_time <= 0`.
    pub fn new(
        feed_id: FeedId,
        price: Price,
        status: PriceStatus,
        publish_time: i64,
    ) -> Result<Self, PriceError> {
        if price.conf == 0 {
            return Err(PriceError::ZeroConfidence);
        }
        if publish_time <= 0 {
            return Err(PriceError::NonPositiveTimestamp(publish_time));
        }
        Ok(Self {
            feed_id,
            price,
            status,
            publish_time,
        })
    }

    /// The age of this update relative to `now`, in seconds (never negative).
    #[must_use]
    pub fn age(&self, now: i64) -> i64 {
        (now - self.publish_time).max(0)
    }

    /// Whether the update is no older than `max_age_secs` at `now`.
    #[must_use]
    pub fn is_fresh(&self, now: i64, max_age_secs: i64) -> bool {
        self.age(now) <= max_age_secs
    }

    /// Returns the price only if the feed is trading **and** the update is no
    /// older than `max_age_secs` — the SDK analogue of Pyth's
    /// `get_price_no_older_than`.
    ///
    /// # Errors
    /// - [`PriceError::NotTrading`] if the status is not `Trading`.
    /// - [`PriceError::Stale`] if the update is older than `max_age_secs`.
    pub fn price_no_older_than(&self, now: i64, max_age_secs: i64) -> Result<Price, PriceError> {
        if !self.status.is_tradeable() {
            return Err(PriceError::NotTrading {
                status: self.status.code(),
            });
        }
        let age = self.age(now);
        if age > max_age_secs {
            return Err(PriceError::Stale {
                age,
                max_age: max_age_secs,
            });
        }
        Ok(self.price)
    }
}
