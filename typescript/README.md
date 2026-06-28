# @pricefeed/sdk (TypeScript)

TypeScript consumer SDK for PriceFeed — a generic, transport-agnostic price client
that mirrors the Rust core's domain types and semantics. Zero runtime dependencies;
uses the platform `fetch`.

```ts
import { PriceClient, CachingTransport, HttpTransport, FeedId } from "@pricefeed/sdk";

const transport = new CachingTransport(
  new HttpTransport("http://localhost:8080/graphql"),
  500,
);
const client = new PriceClient(transport);

const price = await client.getPriceNoOlderThan(
  FeedId.parse("BTC/USD"),
  Math.floor(Date.now() / 1000),
  30,
);
console.log(`BTC/USD = ${price.toString()}`);
```

## Scripts

```bash
npm run build   # tsc -> dist/
npm test        # builds, then runs node:test suites
npm run lint    # tsc --noEmit type-check
```

See the top-level [README](../README.md) for the full design overview.
