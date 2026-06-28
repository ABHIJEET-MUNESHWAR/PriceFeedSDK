//! The transport abstraction: the single port every backend implements.

use async_trait::async_trait;
use pricefeed_types::{FeedId, PriceUpdate};

use crate::error::ClientError;

/// A source of price updates.
///
/// This is the SDK's single extension point (a hexagonal *port*). HTTP,
/// gRPC, websocket, or in-memory backends all implement this one trait, and the
/// generic [`crate::PriceClient`] composes behavior on top of it.
#[mockall::automock]
#[async_trait]
pub trait PriceTransport: Send + Sync {
    /// Fetches the latest update for a single feed.
    async fn fetch(&self, feed: &FeedId) -> Result<PriceUpdate, ClientError>;

    /// Fetches the latest updates for many feeds.
    ///
    /// The default implementation fetches sequentially; transports that support
    /// batching should override this for efficiency.
    async fn fetch_many(&self, feeds: &[FeedId]) -> Result<Vec<PriceUpdate>, ClientError> {
        let mut out = Vec::with_capacity(feeds.len());
        for feed in feeds {
            out.push(self.fetch(feed).await?);
        }
        Ok(out)
    }
}

#[async_trait]
impl<T: PriceTransport + ?Sized> PriceTransport for std::sync::Arc<T> {
    async fn fetch(&self, feed: &FeedId) -> Result<PriceUpdate, ClientError> {
        (**self).fetch(feed).await
    }

    async fn fetch_many(&self, feeds: &[FeedId]) -> Result<Vec<PriceUpdate>, ClientError> {
        (**self).fetch_many(feeds).await
    }
}
