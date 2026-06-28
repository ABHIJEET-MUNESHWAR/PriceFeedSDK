"""Unit tests for the Python SDK (stdlib unittest, async via IsolatedAsyncioTestCase)."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from pricefeed_sdk import (  # noqa: E402
    MAX_FEED_ID_LEN,
    CachingTransport,
    ClientError,
    FeedId,
    HttpTransport,
    Price,
    PriceClient,
    PriceError,
    PriceStatus,
    PriceUpdate,
    RetryPolicy,
    parse_price_response,
)


class TypesTests(unittest.TestCase):
    def test_feed_id_normalizes_and_validates(self) -> None:
        self.assertEqual(FeedId.parse("btc/usd").as_str(), "BTC/USD")
        with self.assertRaises(PriceError) as ctx:
            FeedId.parse("")
        self.assertEqual(ctx.exception.code, "empty_feed")
        with self.assertRaises(PriceError) as ctx:
            FeedId.parse("a" * (MAX_FEED_ID_LEN + 1))
        self.assertEqual(ctx.exception.code, "feed_id_too_long")
        with self.assertRaises(PriceError) as ctx:
            FeedId.parse("BTC USD")
        self.assertEqual(ctx.exception.code, "invalid_feed_char")

    def test_price_value_and_interval(self) -> None:
        p = Price(6_500_000, 500, -2)
        self.assertAlmostEqual(p.value(), 65_000.0)
        lo, hi = p.confidence_interval()
        self.assertAlmostEqual(lo, 64_995.0)
        self.assertAlmostEqual(hi, 65_005.0)
        self.assertEqual(str(p), "65000.00 ± 5.00")

    def test_price_confidence_ratio(self) -> None:
        p = Price(100_000, 1_000, -2)
        self.assertAlmostEqual(p.confidence_ratio(), 0.01)
        self.assertTrue(p.is_confident_within(0.01))
        self.assertFalse(p.is_confident_within(0.005))
        self.assertEqual(Price(0, 5, -2).confidence_ratio(), float("inf"))

    def test_update_invariants(self) -> None:
        feed = FeedId.parse("BTC/USD")
        with self.assertRaises(PriceError) as ctx:
            PriceUpdate.create(feed, Price(1, 0, -2), PriceStatus.TRADING, 1)
        self.assertEqual(ctx.exception.code, "zero_confidence")
        with self.assertRaises(PriceError) as ctx:
            PriceUpdate.create(feed, Price(1, 1, -2), PriceStatus.TRADING, 0)
        self.assertEqual(ctx.exception.code, "non_positive_timestamp")

    def test_price_no_older_than(self) -> None:
        feed = FeedId.parse("BTC/USD")
        u = PriceUpdate.create(feed, Price(100, 1, -2), PriceStatus.TRADING, 1_000)
        self.assertTrue(u.price_no_older_than(1_010, 30))
        with self.assertRaises(PriceError) as ctx:
            u.price_no_older_than(2_000, 30)
        self.assertEqual(ctx.exception.code, "stale")
        halted = PriceUpdate.create(feed, Price(100, 1, -2), PriceStatus.HALTED, 1_000)
        with self.assertRaises(PriceError) as ctx:
            halted.price_no_older_than(1_005, 30)
        self.assertEqual(ctx.exception.code, "not_trading")


class _MockTransport:
    def __init__(self, behavior) -> None:
        self.behavior = behavior
        self.calls = 0

    async def fetch(self, feed: FeedId) -> PriceUpdate:
        self.calls += 1
        return self.behavior(self.calls, feed)


def _ok(_n, feed):
    return PriceUpdate.create(feed, Price(100, 1, -2), PriceStatus.TRADING, 1_000)


class ClientTests(unittest.IsolatedAsyncioTestCase):
    async def test_get_succeeds(self) -> None:
        client = PriceClient(_MockTransport(_ok))
        u = await client.get(FeedId.parse("BTC/USD"))
        self.assertEqual(u.feed_id.as_str(), "BTC/USD")

    async def test_retries_then_succeeds(self) -> None:
        def behavior(n, feed):
            if n < 3:
                raise ClientError.transport("temporary")
            return _ok(n, feed)

        transport = _MockTransport(behavior)
        policy = RetryPolicy(max_retries=3, base_delay_s=0.001, per_attempt_timeout_s=0.0)
        client = PriceClient(transport, policy)
        await client.get(FeedId.parse("BTC/USD"))
        self.assertEqual(transport.calls, 3)

    async def test_does_not_retry_permanent(self) -> None:
        def behavior(_n, _feed):
            raise ClientError.permanent("nope")

        transport = _MockTransport(behavior)
        client = PriceClient(transport, RetryPolicy(max_retries=3, base_delay_s=0.001))
        with self.assertRaises(ClientError):
            await client.get(FeedId.parse("BTC/USD"))
        self.assertEqual(transport.calls, 1)

    async def test_caching_within_ttl(self) -> None:
        transport = _MockTransport(_ok)
        cached = CachingTransport(transport, ttl_s=60.0)
        feed = FeedId.parse("BTC/USD")
        await cached.fetch(feed)
        await cached.fetch(feed)
        self.assertEqual(transport.calls, 1)
        self.assertEqual(cached.size(), 1)
        cached.clear()
        self.assertEqual(cached.size(), 0)

    async def test_get_price_no_older_than_rejects_stale(self) -> None:
        client = PriceClient(_MockTransport(_ok))
        with self.assertRaises(PriceError) as ctx:
            await client.get_price_no_older_than(FeedId.parse("BTC/USD"), 5_000, 30)
        self.assertEqual(ctx.exception.code, "stale")


class HttpTests(unittest.IsolatedAsyncioTestCase):
    def test_parse_success(self) -> None:
        feed = FeedId.parse("BTC/USD")
        body = (
            '{"data":{"price":{"feedId":"BTC/USD","mantissa":6500000,'
            '"conf":500,"expo":-2,"status":"trading","publishTime":1700000000}}}'
        )
        u = parse_price_response(body, feed)
        self.assertEqual(u.status, PriceStatus.TRADING)

    def test_parse_not_found(self) -> None:
        feed = FeedId.parse("BTC/USD")
        with self.assertRaises(ClientError) as ctx:
            parse_price_response('{"data":{"price":null}}', feed)
        self.assertEqual(ctx.exception.code, "not_found")

    def test_parse_graphql_error(self) -> None:
        feed = FeedId.parse("BTC/USD")
        with self.assertRaises(ClientError):
            parse_price_response('{"errors":[{"message":"boom"}]}', feed)

    def test_parse_decode_error(self) -> None:
        feed = FeedId.parse("BTC/USD")
        with self.assertRaises(ClientError):
            parse_price_response("not json", feed)

    async def test_transport_with_injected_fetcher(self) -> None:
        feed = FeedId.parse("BTC/USD")

        def fetcher(_url, _body):
            return (
                '{"data":{"price":{"feedId":"BTC/USD","mantissa":100,'
                '"conf":1,"expo":-2,"status":"trading","publishTime":1700000000}}}'
            )

        transport = HttpTransport("http://example/graphql", fetcher)
        u = await transport.fetch(feed)
        self.assertEqual(u.feed_id.as_str(), "BTC/USD")


if __name__ == "__main__":
    unittest.main()
