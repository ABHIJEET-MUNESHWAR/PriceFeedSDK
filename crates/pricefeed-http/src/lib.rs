//! HTTP/GraphQL transport for the PriceFeed SDK.
//!
//! Plugs an [`HttpTransport`] into the generic [`pricefeed_client::PriceClient`]
//! so a consumer can fetch prices from any compatible GraphQL endpoint with
//! built-in caching, retries, and timeouts.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod error;
mod transport;
mod wire;

pub use error::HttpError;
pub use transport::HttpTransport;
pub use wire::parse_price_response;
