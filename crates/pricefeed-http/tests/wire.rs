//! Offline parsing tests for the HTTP transport.

use pricefeed_http::{parse_price_response, HttpError};
use pricefeed_types::{FeedId, PriceStatus};

fn feed() -> FeedId {
    FeedId::new("BTC/USD").unwrap()
}

#[test]
fn parses_a_valid_response() {
    let body = r#"{
        "data": {
            "price": {
                "feedId": "BTC/USD",
                "mantissa": 6500000,
                "conf": 500,
                "expo": -2,
                "status": "trading",
                "publishTime": 1700000000
            }
        }
    }"#;
    let update = parse_price_response(body, &feed()).unwrap();
    assert_eq!(update.feed_id, feed());
    assert_eq!(update.status, PriceStatus::Trading);
    assert!((update.price.value() - 65_000.0).abs() < 1e-6);
}

#[test]
fn null_price_is_not_found() {
    let body = r#"{ "data": { "price": null } }"#;
    let err = parse_price_response(body, &feed()).unwrap_err();
    assert!(matches!(err, HttpError::NotFound(_)));
}

#[test]
fn graphql_errors_are_surfaced() {
    let body = r#"{ "errors": [ { "message": "boom" } ] }"#;
    let err = parse_price_response(body, &feed()).unwrap_err();
    assert!(matches!(err, HttpError::GraphQl(m) if m == "boom"));
}

#[test]
fn malformed_json_is_a_decode_error() {
    let err = parse_price_response("not json", &feed()).unwrap_err();
    assert!(matches!(err, HttpError::Decode(_)));
}

#[test]
fn zero_confidence_is_a_domain_error() {
    let body = r#"{
        "data": {
            "price": {
                "feedId": "BTC/USD",
                "mantissa": 100,
                "conf": 0,
                "expo": -2,
                "status": "trading",
                "publishTime": 1700000000
            }
        }
    }"#;
    let err = parse_price_response(body, &feed()).unwrap_err();
    assert!(matches!(err, HttpError::Domain(_)));
}

#[test]
fn unknown_status_maps_to_unknown() {
    let body = r#"{
        "data": {
            "price": {
                "feedId": "BTC/USD",
                "mantissa": 100,
                "conf": 1,
                "expo": -2,
                "status": "weird",
                "publishTime": 1700000000
            }
        }
    }"#;
    let update = parse_price_response(body, &feed()).unwrap();
    assert_eq!(update.status, PriceStatus::Unknown);
}
