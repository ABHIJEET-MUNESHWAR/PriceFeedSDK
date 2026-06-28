import test from "node:test";
import assert from "node:assert/strict";

import {
  FeedId,
  Price,
  PriceStatus,
  PriceUpdate,
  PriceError,
  ClientError,
  PriceClient,
  CachingTransport,
  HttpTransport,
  parsePriceResponse,
  MAX_FEED_ID_LEN,
} from "../dist/index.js";

test("FeedId normalizes and validates", () => {
  assert.equal(FeedId.parse("btc/usd").asString(), "BTC/USD");
  assert.throws(() => FeedId.parse(""), (e) => e instanceof PriceError && e.code === "empty_feed");
  assert.throws(
    () => FeedId.parse("a".repeat(MAX_FEED_ID_LEN + 1)),
    (e) => e.code === "feed_id_too_long",
  );
  assert.throws(() => FeedId.parse("BTC USD"), (e) => e.code === "invalid_feed_char");
});

test("Price value and confidence interval", () => {
  const p = new Price(6500000n, 500n, -2);
  assert.ok(Math.abs(p.value() - 65000) < 1e-6);
  const [lo, hi] = p.confidenceInterval();
  assert.ok(Math.abs(lo - 64995) < 1e-6);
  assert.ok(Math.abs(hi - 65005) < 1e-6);
  assert.equal(p.toString(), "65000.00 ± 5.00");
});

test("Price confidence ratio", () => {
  const p = new Price(100000n, 1000n, -2);
  assert.ok(Math.abs(p.confidenceRatio() - 0.01) < 1e-9);
  assert.ok(p.isConfidentWithin(0.01));
  assert.ok(!p.isConfidentWithin(0.005));
  assert.equal(new Price(0n, 5n, -2).confidenceRatio(), Infinity);
});

test("PriceUpdate invariants", () => {
  const feed = FeedId.parse("BTC/USD");
  assert.throws(
    () => PriceUpdate.create(feed, new Price(1n, 0n, -2), PriceStatus.Trading, 1),
    (e) => e.code === "zero_confidence",
  );
  assert.throws(
    () => PriceUpdate.create(feed, new Price(1n, 1n, -2), PriceStatus.Trading, 0),
    (e) => e.code === "non_positive_timestamp",
  );
});

test("priceNoOlderThan enforces freshness and status", () => {
  const feed = FeedId.parse("BTC/USD");
  const u = PriceUpdate.create(feed, new Price(100n, 1n, -2), PriceStatus.Trading, 1000);
  assert.ok(u.priceNoOlderThan(1010, 30));
  assert.throws(() => u.priceNoOlderThan(2000, 30), (e) => e.code === "stale");

  const halted = PriceUpdate.create(feed, new Price(100n, 1n, -2), PriceStatus.Halted, 1000);
  assert.throws(() => halted.priceNoOlderThan(1005, 30), (e) => e.code === "not_trading");
});

test("PriceClient retries retryable errors", async () => {
  let calls = 0;
  const transport = {
    async fetch(feed) {
      calls += 1;
      if (calls < 3) throw ClientError.transport("temporary");
      return PriceUpdate.create(feed, new Price(100n, 1n, -2), PriceStatus.Trading, 1000);
    },
  };
  const client = new PriceClient(transport, {
    maxRetries: 3,
    baseDelayMs: 1,
    maxDelayMs: 10,
    multiplier: 2,
    perAttemptTimeoutMs: 0,
  });
  const u = await client.get(FeedId.parse("BTC/USD"));
  assert.equal(u.feedId.asString(), "BTC/USD");
  assert.equal(calls, 3);
});

test("PriceClient does not retry permanent errors", async () => {
  let calls = 0;
  const transport = {
    async fetch() {
      calls += 1;
      throw ClientError.permanent("nope");
    },
  };
  const client = new PriceClient(transport, {
    maxRetries: 3,
    baseDelayMs: 1,
    maxDelayMs: 10,
    multiplier: 2,
    perAttemptTimeoutMs: 0,
  });
  await assert.rejects(() => client.get(FeedId.parse("BTC/USD")));
  assert.equal(calls, 1);
});

test("CachingTransport serves within TTL", async () => {
  let calls = 0;
  const inner = {
    async fetch(feed) {
      calls += 1;
      return PriceUpdate.create(feed, new Price(100n, 1n, -2), PriceStatus.Trading, 1000);
    },
  };
  const cached = new CachingTransport(inner, 60000);
  const feed = FeedId.parse("BTC/USD");
  await cached.fetch(feed);
  await cached.fetch(feed);
  assert.equal(calls, 1);
  assert.equal(cached.size(), 1);
  cached.clear();
  assert.equal(cached.size(), 0);
});

test("parsePriceResponse handles success and errors", () => {
  const feed = FeedId.parse("BTC/USD");
  const ok = parsePriceResponse(
    JSON.stringify({
      data: {
        price: {
          feedId: "BTC/USD",
          mantissa: 6500000,
          conf: 500,
          expo: -2,
          status: "trading",
          publishTime: 1700000000,
        },
      },
    }),
    feed,
  );
  assert.equal(ok.status, PriceStatus.Trading);

  assert.throws(
    () => parsePriceResponse(JSON.stringify({ data: { price: null } }), feed),
    (e) => e.code === "not_found",
  );
  assert.throws(
    () => parsePriceResponse(JSON.stringify({ errors: [{ message: "boom" }] }), feed),
    (e) => e instanceof ClientError,
  );
  assert.throws(() => parsePriceResponse("not json", feed), (e) => e instanceof ClientError);
});

test("HttpTransport uses injected fetch", async () => {
  const feed = FeedId.parse("BTC/USD");
  const fakeFetch = async () =>
    new Response(
      JSON.stringify({
        data: {
          price: {
            feedId: "BTC/USD",
            mantissa: 100,
            conf: 1,
            expo: -2,
            status: "trading",
            publishTime: 1700000000,
          },
        },
      }),
      { status: 200 },
    );
  const transport = new HttpTransport("http://example/graphql", fakeFetch);
  const u = await transport.fetch(feed);
  assert.equal(u.feedId.asString(), "BTC/USD");
});
