//! Generic, transport-agnostic price-feed client.
//!
//! The design is hexagonal: [`PriceTransport`] is the single port, and
//! everything else (caching, retries, timeouts, conveniences) is composed
//! generically on top of it. Bring your own backend — HTTP, websocket, gRPC, or
//! an in-memory mock — and the same ergonomic [`PriceClient`] wraps it.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod cache;
mod client;
mod error;
mod retry;
mod transport;

pub use cache::CachingTransport;
pub use client::PriceClient;
pub use error::ClientError;
pub use retry::{run_with_policy, RetryPolicy};
pub use transport::{MockPriceTransport, PriceTransport};

/// Re-export of the shared domain types for binding convenience.
pub use pricefeed_types as types;
