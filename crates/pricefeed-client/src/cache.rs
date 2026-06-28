//! A TTL cache decorator that wraps any [`PriceTransport`].

use std::collections::HashMap;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use parking_lot::Mutex;
use pricefeed_types::{FeedId, PriceUpdate};

use crate::error::ClientError;
use crate::transport::PriceTransport;

struct Entry {
    update: PriceUpdate,
    fetched_at: Instant,
}

/// Wraps an inner transport with a time-to-live in-memory cache.
///
/// This is the decorator pattern: `CachingTransport<T>` *is a*
/// [`PriceTransport`], so it can be composed with the generic client exactly
/// like any other backend. Entries older than `ttl` are refreshed on access.
pub struct CachingTransport<T> {
    inner: T,
    ttl: Duration,
    cache: Mutex<HashMap<FeedId, Entry>>,
}

impl<T> CachingTransport<T> {
    /// Wraps `inner`, caching responses for `ttl`.
    #[must_use]
    pub fn new(inner: T, ttl: Duration) -> Self {
        Self {
            inner,
            ttl,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// The number of currently cached entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cache.lock().len()
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cache.lock().is_empty()
    }

    /// Drops every cached entry.
    pub fn clear(&self) {
        self.cache.lock().clear();
    }

    fn cached_fresh(&self, feed: &FeedId) -> Option<PriceUpdate> {
        let cache = self.cache.lock();
        cache.get(feed).and_then(|e| {
            if e.fetched_at.elapsed() < self.ttl {
                Some(e.update.clone())
            } else {
                None
            }
        })
    }

    fn store(&self, feed: FeedId, update: PriceUpdate) {
        self.cache.lock().insert(
            feed,
            Entry {
                update,
                fetched_at: Instant::now(),
            },
        );
    }
}

#[async_trait]
impl<T: PriceTransport> PriceTransport for CachingTransport<T> {
    async fn fetch(&self, feed: &FeedId) -> Result<PriceUpdate, ClientError> {
        if let Some(hit) = self.cached_fresh(feed) {
            return Ok(hit);
        }
        let fresh = self.inner.fetch(feed).await?;
        self.store(feed.clone(), fresh.clone());
        Ok(fresh)
    }
}
