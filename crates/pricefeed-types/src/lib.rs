//! Core typed domain for the PriceFeed SDK.
//!
//! This crate has no I/O and no async; it is the shared vocabulary every
//! language binding mirrors. The invariants here (validated feed ids,
//! always-positive confidence, fixed-point scaling) make whole classes of
//! consumer bugs unrepresentable.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod error;
mod ids;
mod price;
mod update;

pub use error::PriceError;
pub use ids::{FeedId, MAX_FEED_ID_LEN};
pub use price::{Price, PriceStatus};
pub use update::PriceUpdate;
