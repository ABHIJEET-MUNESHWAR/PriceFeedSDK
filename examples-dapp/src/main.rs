//! Example dApp: a small CLI that watches one or more price feeds.
//!
//! Two modes:
//! * `--demo` uses a deterministic in-memory transport, so the example runs
//!   end-to-end with no server (handy for CI and first-run UX).
//! * otherwise it talks to a live GraphQL endpoint via [`HttpTransport`].

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use pricefeed_client::{CachingTransport, ClientError, PriceClient, PriceTransport, RetryPolicy};
use pricefeed_http::HttpTransport;
use pricefeed_types::{FeedId, Price, PriceStatus, PriceUpdate};

/// CLI arguments.
#[derive(Debug, Parser)]
#[command(
    name = "pricefeed-watch",
    about = "Watch price feeds via the PriceFeed SDK"
)]
struct Args {
    /// Feed symbols to watch (e.g. BTC/USD ETH/USD).
    #[arg(default_values = ["BTC/USD", "ETH/USD", "SOL/USD"])]
    feeds: Vec<String>,

    /// GraphQL endpoint to query (ignored in --demo mode).
    #[arg(
        long,
        env = "PRICEFEED_ENDPOINT",
        default_value = "http://localhost:8080/graphql"
    )]
    endpoint: String,

    /// Run against a deterministic in-memory transport (no server required).
    #[arg(long)]
    demo: bool,

    /// Number of polling rounds to perform.
    #[arg(long, default_value_t = 3)]
    rounds: u32,

    /// Maximum acceptable staleness, in seconds.
    #[arg(long, default_value_t = 30)]
    max_age: i64,

    /// Cache time-to-live, in milliseconds.
    #[arg(long, default_value_t = 500)]
    cache_ttl_ms: u64,
}

/// A deterministic in-memory transport used by `--demo`.
struct DemoTransport;

#[async_trait]
impl PriceTransport for DemoTransport {
    async fn fetch(&self, feed: &FeedId) -> Result<PriceUpdate, ClientError> {
        let (mantissa, expo) = match feed.as_str() {
            "BTC/USD" => (6_500_000, -2),
            "ETH/USD" => (350_000, -2),
            "SOL/USD" => (15_000, -2),
            _ => (100, -2),
        };
        let now = unix_now();
        let update = PriceUpdate::new(
            feed.clone(),
            Price::new(mantissa, (mantissa / 1000).max(1) as u64, expo),
            PriceStatus::Trading,
            now,
        )
        .map_err(ClientError::Domain)?;
        Ok(update)
    }
}

fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

async fn run<T: PriceTransport>(client: &PriceClient<T>, args: &Args) -> Result<()> {
    let feeds = args
        .feeds
        .iter()
        .map(FeedId::new)
        .collect::<Result<Vec<_>, _>>()?;

    for round in 1..=args.rounds {
        println!("--- round {round} ---");
        for feed in &feeds {
            match client.get(feed).await {
                Ok(update) => {
                    let now = unix_now();
                    let fresh = update.is_fresh(now, args.max_age);
                    let (lo, hi) = update.price.confidence_interval();
                    println!(
                        "{:<10} {}  [{:.4}, {:.4}]  {}  age={}s{}",
                        feed.as_str(),
                        update.price,
                        lo,
                        hi,
                        update.status.code(),
                        update.age(now),
                        if fresh { "" } else { "  (STALE)" },
                    );
                }
                Err(err) => {
                    eprintln!("{:<10} error[{}]: {err}", feed.as_str(), err.code());
                }
            }
        }
        if round < args.rounds {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();
    let policy = RetryPolicy::default();
    let ttl = Duration::from_millis(args.cache_ttl_ms);

    if args.demo {
        let transport = CachingTransport::new(DemoTransport, ttl);
        let client = PriceClient::with_policy(transport, policy);
        run(&client, &args).await
    } else {
        let transport = HttpTransport::new(args.endpoint.clone())?;
        let cached = CachingTransport::new(transport, ttl);
        let client = PriceClient::with_policy(cached, policy);
        run(&client, &args).await
    }
}
