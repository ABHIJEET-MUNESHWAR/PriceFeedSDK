//! The generic, ergonomic price client.

use std::time::Duration;

use pricefeed_types::{FeedId, Price, PriceUpdate};

use crate::error::ClientError;
use crate::retry::{run_with_policy, RetryPolicy};
use crate::transport::PriceTransport;

/// A high-level client generic over any [`PriceTransport`].
///
/// The client adds resilience (timeouts + backoff retries) and consumer-grade
/// conveniences (`price_no_older_than`) on top of a bare transport, without the
/// transport needing to know about any of it.
///
/// # Examples
/// ```
/// # use pricefeed_client::{PriceClient, MockPriceTransport};
/// # use pricefeed_types::{FeedId, Price, PriceStatus, PriceUpdate};
/// # tokio_test::block_on(async {
/// let mut transport = MockPriceTransport::new();
/// transport.expect_fetch().returning(|feed| {
///     Ok(PriceUpdate::new(
///         feed.clone(),
///         Price::new(100, 1, -2),
///         PriceStatus::Trading,
///         1_000,
///     )
///     .unwrap())
/// });
/// let client = PriceClient::new(transport);
/// let feed = FeedId::new("BTC/USD").unwrap();
/// let update = client.get(&feed).await.unwrap();
/// assert_eq!(update.feed_id, feed);
/// # });
/// ```
#[derive(Debug, Clone)]
pub struct PriceClient<T> {
    transport: T,
    policy: RetryPolicy,
}

impl<T: PriceTransport> PriceClient<T> {
    /// Creates a client with the default [`RetryPolicy`].
    #[must_use]
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            policy: RetryPolicy::default(),
        }
    }

    /// Creates a client with a custom [`RetryPolicy`].
    #[must_use]
    pub fn with_policy(transport: T, policy: RetryPolicy) -> Self {
        Self { transport, policy }
    }

    /// Overrides the retry policy, returning the updated client (builder-style).
    #[must_use]
    pub fn set_policy(mut self, policy: RetryPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Borrows the underlying transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Fetches the latest update for `feed`, applying timeouts and retries.
    pub async fn get(&self, feed: &FeedId) -> Result<PriceUpdate, ClientError> {
        run_with_policy(self.policy, || self.transport.fetch(feed)).await
    }

    /// Fetches updates for many feeds, applying timeouts and retries to the
    /// batch as a whole.
    pub async fn get_many(&self, feeds: &[FeedId]) -> Result<Vec<PriceUpdate>, ClientError> {
        run_with_policy(self.policy, || self.transport.fetch_many(feeds)).await
    }

    /// Fetches `feed` and returns the price only if it is trading and no older
    /// than `max_age`.
    ///
    /// # Errors
    /// Propagates transport errors, or a [`pricefeed_types::PriceError`] wrapped
    /// in [`ClientError::Domain`] when the price is stale or not trading.
    pub async fn get_price_no_older_than(
        &self,
        feed: &FeedId,
        now: i64,
        max_age: Duration,
    ) -> Result<Price, ClientError> {
        let update = self.get(feed).await?;
        let secs = i64::try_from(max_age.as_secs()).unwrap_or(i64::MAX);
        update.price_no_older_than(now, secs).map_err(Into::into)
    }
}
