//! Microbenchmarks for the client hot paths.
//!
//! Run with `cargo bench -p pricefeed-client`.

use std::time::Duration;

use async_trait::async_trait;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pricefeed_client::PriceClient;
use pricefeed_client::{CachingTransport, ClientError, PriceTransport, RetryPolicy};
use pricefeed_types::{FeedId, Price, PriceStatus, PriceUpdate};

/// A zero-cost transport that returns a constant update.
struct StaticTransport(PriceUpdate);

#[async_trait]
impl PriceTransport for StaticTransport {
    async fn fetch(&self, _feed: &FeedId) -> Result<PriceUpdate, ClientError> {
        Ok(self.0.clone())
    }
}

fn sample(feed: &FeedId) -> PriceUpdate {
    PriceUpdate::new(
        feed.clone(),
        Price::new(6_500_000, 500, -2),
        PriceStatus::Trading,
        1_000,
    )
    .unwrap()
}

fn bench_price_math(c: &mut Criterion) {
    let price = Price::new(6_500_000, 500, -2);
    c.bench_function("price/confidence_interval", |b| {
        b.iter(|| black_box(black_box(&price).confidence_interval()))
    });
    c.bench_function("price/confidence_ratio", |b| {
        b.iter(|| black_box(black_box(&price).confidence_ratio()))
    });
}

fn bench_cache_hit(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let feed = FeedId::new("BTC/USD").unwrap();
    let transport = CachingTransport::new(StaticTransport(sample(&feed)), Duration::from_secs(60));

    // Warm the cache.
    rt.block_on(async { transport.fetch(&feed).await.unwrap() });

    c.bench_function("cache/hit", |b| {
        b.iter(|| {
            rt.block_on(async { black_box(transport.fetch(black_box(&feed)).await.unwrap()) })
        })
    });
}

fn bench_client_get(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let feed = FeedId::new("BTC/USD").unwrap();
    let client = PriceClient::with_policy(
        StaticTransport(sample(&feed)),
        RetryPolicy {
            per_attempt_timeout: None,
            ..RetryPolicy::none()
        },
    );

    c.bench_function("client/get", |b| {
        b.iter(|| rt.block_on(async { black_box(client.get(black_box(&feed)).await.unwrap()) }))
    });
}

criterion_group!(benches, bench_price_math, bench_cache_hit, bench_client_get);
criterion_main!(benches);
