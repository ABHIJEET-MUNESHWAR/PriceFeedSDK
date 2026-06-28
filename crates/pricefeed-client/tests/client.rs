//! Integration tests for the client layer.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use pricefeed_client::{
    CachingTransport, ClientError, MockPriceTransport, PriceClient, PriceTransport, RetryPolicy,
};
use pricefeed_types::{FeedId, Price, PriceStatus, PriceUpdate};

fn sample(feed: &FeedId, publish_time: i64, status: PriceStatus) -> PriceUpdate {
    PriceUpdate::new(feed.clone(), Price::new(100, 1, -2), status, publish_time).unwrap()
}

#[tokio::test]
async fn get_returns_update_on_success() {
    let feed = FeedId::new("BTC/USD").unwrap();
    let mut transport = MockPriceTransport::new();
    let f = feed.clone();
    transport
        .expect_fetch()
        .times(1)
        .returning(move |_| Ok(sample(&f, 1_000, PriceStatus::Trading)));

    let client = PriceClient::new(transport);
    let update = client.get(&feed).await.unwrap();
    assert_eq!(update.feed_id, feed);
}

#[tokio::test]
async fn retries_retryable_errors_then_succeeds() {
    let feed = FeedId::new("BTC/USD").unwrap();
    let calls = Arc::new(AtomicU32::new(0));
    let calls_in = calls.clone();
    let f = feed.clone();

    let mut transport = MockPriceTransport::new();
    transport.expect_fetch().returning(move |_| {
        let n = calls_in.fetch_add(1, Ordering::SeqCst);
        if n < 2 {
            Err(ClientError::transport("temporary"))
        } else {
            Ok(sample(&f, 1_000, PriceStatus::Trading))
        }
    });

    let policy = RetryPolicy {
        base_delay: Duration::from_millis(1),
        per_attempt_timeout: None,
        ..RetryPolicy::default()
    };
    let client = PriceClient::with_policy(transport, policy);
    assert!(client.get(&feed).await.is_ok());
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn does_not_retry_permanent_errors() {
    let feed = FeedId::new("BTC/USD").unwrap();
    let mut transport = MockPriceTransport::new();
    transport
        .expect_fetch()
        .times(1)
        .returning(|_| Err(ClientError::permanent("nope")));

    let client = PriceClient::new(transport);
    let err = client.get(&feed).await.unwrap_err();
    assert!(!err.is_retryable());
    assert_eq!(err.code(), "transport");
}

#[tokio::test]
async fn give_up_after_max_retries() {
    let feed = FeedId::new("BTC/USD").unwrap();
    let calls = Arc::new(AtomicU32::new(0));
    let calls_in = calls.clone();
    let mut transport = MockPriceTransport::new();
    transport.expect_fetch().returning(move |_| {
        calls_in.fetch_add(1, Ordering::SeqCst);
        Err(ClientError::transport("always fails"))
    });

    let policy = RetryPolicy {
        max_retries: 2,
        base_delay: Duration::from_millis(1),
        per_attempt_timeout: None,
        ..RetryPolicy::default()
    };
    let client = PriceClient::with_policy(transport, policy);
    assert!(client.get(&feed).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 3); // 1 + 2 retries
}

#[tokio::test]
async fn get_price_no_older_than_rejects_stale() {
    let feed = FeedId::new("BTC/USD").unwrap();
    let f = feed.clone();
    let mut transport = MockPriceTransport::new();
    transport
        .expect_fetch()
        .returning(move |_| Ok(sample(&f, 1_000, PriceStatus::Trading)));

    let client = PriceClient::new(transport);
    let err = client
        .get_price_no_older_than(&feed, 5_000, Duration::from_secs(30))
        .await
        .unwrap_err();
    assert_eq!(err.code(), "domain");
}

#[tokio::test]
async fn caching_transport_serves_within_ttl() {
    let feed = FeedId::new("BTC/USD").unwrap();
    let f = feed.clone();
    let mut transport = MockPriceTransport::new();
    transport
        .expect_fetch()
        .times(1) // only one underlying call despite two reads
        .returning(move |_| Ok(sample(&f, 1_000, PriceStatus::Trading)));

    let cached = CachingTransport::new(transport, Duration::from_secs(60));
    assert!(cached.is_empty());
    let _ = cached.fetch(&feed).await.unwrap();
    let _ = cached.fetch(&feed).await.unwrap();
    assert_eq!(cached.len(), 1);
    cached.clear();
    assert!(cached.is_empty());
}

#[tokio::test]
async fn fetch_many_default_iterates() {
    // A concrete transport that only implements `fetch`, exercising the default
    // `fetch_many` provided by the trait.
    struct Counting {
        calls: Arc<AtomicU32>,
    }

    #[async_trait::async_trait]
    impl PriceTransport for Counting {
        async fn fetch(&self, feed: &FeedId) -> Result<PriceUpdate, ClientError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(sample(feed, 1_000, PriceStatus::Trading))
        }
    }

    let feeds = [
        FeedId::new("BTC/USD").unwrap(),
        FeedId::new("ETH/USD").unwrap(),
    ];
    let calls = Arc::new(AtomicU32::new(0));
    let client = PriceClient::new(Counting {
        calls: calls.clone(),
    });
    let updates = client.get_many(&feeds).await.unwrap();
    assert_eq!(updates.len(), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn arc_transport_is_a_transport() {
    let feed = FeedId::new("BTC/USD").unwrap();
    let f = feed.clone();
    let mut transport = MockPriceTransport::new();
    transport
        .expect_fetch()
        .returning(move |_| Ok(sample(&f, 1_000, PriceStatus::Trading)));

    let shared: Arc<dyn PriceTransport> = Arc::new(transport);
    let client = PriceClient::new(shared);
    assert!(client.get(&feed).await.is_ok());
}
