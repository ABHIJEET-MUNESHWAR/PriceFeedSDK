//! Unit and property tests for the domain types.

use pricefeed_types::{FeedId, Price, PriceError, PriceStatus, PriceUpdate, MAX_FEED_ID_LEN};
use proptest::prelude::*;

#[test]
fn feed_id_normalizes_and_validates() {
    assert_eq!(FeedId::new("btc/usd").unwrap().as_str(), "BTC/USD");
    assert_eq!(
        FeedId::new("ETH-USD.PERP").unwrap().as_str(),
        "ETH-USD.PERP"
    );
    assert_eq!(FeedId::new("").unwrap_err(), PriceError::EmptyFeed);
    assert_eq!(
        FeedId::new("a".repeat(MAX_FEED_ID_LEN + 1)).unwrap_err(),
        PriceError::FeedIdTooLong {
            max: MAX_FEED_ID_LEN
        }
    );
    assert_eq!(
        FeedId::new("BTC USD").unwrap_err(),
        PriceError::InvalidFeedChar { ch: ' ' }
    );
}

#[test]
fn feed_id_serde_round_trips_as_string() {
    let id = FeedId::new("SOL/USD").unwrap();
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"SOL/USD\"");
    let back: FeedId = serde_json::from_str(&json).unwrap();
    assert_eq!(back, id);
}

#[test]
fn feed_id_serde_rejects_invalid() {
    let err = serde_json::from_str::<FeedId>("\"bad id!\"");
    assert!(err.is_err());
}

#[test]
fn price_value_and_interval() {
    let p = Price::new(6_500_000, 500, -2); // 65000.00 ± 5.00
    assert!((p.value() - 65_000.0).abs() < 1e-6);
    assert!((p.confidence() - 5.0).abs() < 1e-6);
    let (lo, hi) = p.confidence_interval();
    assert!((lo - 64_995.0).abs() < 1e-6);
    assert!((hi - 65_005.0).abs() < 1e-6);
}

#[test]
fn price_confidence_ratio_and_bound() {
    let p = Price::new(100_000, 1_000, -2); // ratio = 0.01
    assert!((p.confidence_ratio() - 0.01).abs() < 1e-9);
    assert!(p.is_confident_within(0.01));
    assert!(!p.is_confident_within(0.005));
}

#[test]
fn zero_mantissa_is_maximally_uncertain() {
    let p = Price::new(0, 5, -2);
    assert!(p.confidence_ratio().is_infinite());
    assert!(!p.is_confident_within(1.0));
}

#[test]
fn price_display_uses_expo_decimals() {
    let p = Price::new(6_500_000, 500, -2);
    assert_eq!(p.to_string(), "65000.00 ± 5.00");
}

#[test]
fn status_codes_and_tradeability() {
    assert_eq!(PriceStatus::Trading.code(), "trading");
    assert!(PriceStatus::Trading.is_tradeable());
    assert!(!PriceStatus::Halted.is_tradeable());
    assert!(!PriceStatus::Unknown.is_tradeable());
    assert_eq!(PriceStatus::default(), PriceStatus::Unknown);
}

#[test]
fn update_rejects_bad_invariants() {
    let feed = FeedId::new("BTC/USD").unwrap();
    assert_eq!(
        PriceUpdate::new(feed.clone(), Price::new(1, 0, -2), PriceStatus::Trading, 1).unwrap_err(),
        PriceError::ZeroConfidence
    );
    assert_eq!(
        PriceUpdate::new(feed, Price::new(1, 1, -2), PriceStatus::Trading, 0).unwrap_err(),
        PriceError::NonPositiveTimestamp(0)
    );
}

#[test]
fn price_no_older_than_enforces_freshness_and_status() {
    let feed = FeedId::new("BTC/USD").unwrap();
    let u = PriceUpdate::new(feed, Price::new(100, 1, -2), PriceStatus::Trading, 1_000).unwrap();

    // Fresh + trading -> Ok.
    assert!(u.price_no_older_than(1_010, 30).is_ok());
    assert!(u.is_fresh(1_010, 30));

    // Stale -> error.
    assert_eq!(
        u.price_no_older_than(2_000, 30).unwrap_err(),
        PriceError::Stale {
            age: 1_000,
            max_age: 30
        }
    );

    // Not trading -> error.
    let halted = PriceUpdate::new(u.feed_id.clone(), u.price, PriceStatus::Halted, 1_000).unwrap();
    assert_eq!(
        halted.price_no_older_than(1_005, 30).unwrap_err(),
        PriceError::NotTrading { status: "halted" }
    );
}

#[test]
fn age_is_never_negative() {
    let feed = FeedId::new("BTC/USD").unwrap();
    let u = PriceUpdate::new(feed, Price::new(1, 1, -2), PriceStatus::Trading, 1_000).unwrap();
    // now before publish_time -> clamped to 0.
    assert_eq!(u.age(900), 0);
}

#[test]
fn error_codes_are_stable() {
    assert_eq!(PriceError::EmptyFeed.code(), "empty_feed");
    assert_eq!(PriceError::ZeroConfidence.code(), "zero_confidence");
    assert_eq!(PriceError::Stale { age: 1, max_age: 0 }.code(), "stale");
}

proptest! {
    /// The confidence interval is always centered on the value and symmetric.
    #[test]
    fn interval_is_symmetric(mantissa in -1_000_000_000i64..1_000_000_000, conf in 0u64..1_000_000, expo in -8i32..2) {
        let p = Price::new(mantissa, conf, expo);
        let (lo, hi) = p.confidence_interval();
        let v = p.value();
        prop_assert!((v - lo - (hi - v)).abs() < 1e-3 * (1.0 + v.abs()));
        prop_assert!(lo <= hi);
    }

    /// `price_no_older_than` accepts exactly when fresh and trading.
    #[test]
    fn fresh_iff_within_bound(publish in 1i64..1_000_000, delta in 0i64..10_000, max_age in 0i64..10_000) {
        let feed = FeedId::new("BTC/USD").unwrap();
        let u = PriceUpdate::new(feed, Price::new(1, 1, -2), PriceStatus::Trading, publish).unwrap();
        let now = publish + delta;
        let ok = u.price_no_older_than(now, max_age).is_ok();
        prop_assert_eq!(ok, delta <= max_age);
    }
}
