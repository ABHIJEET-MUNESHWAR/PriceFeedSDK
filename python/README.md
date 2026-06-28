# pricefeed-sdk (Python)

Python consumer SDK for PriceFeed — a generic, transport-agnostic price client that
mirrors the Rust core's domain types and semantics. Pure standard library; no
third-party runtime dependencies.

```python
import asyncio, time
from pricefeed_sdk import PriceClient, CachingTransport, HttpTransport, FeedId

async def main() -> None:
    transport = CachingTransport(HttpTransport("http://localhost:8080/graphql"), ttl_s=0.5)
    client = PriceClient(transport)
    price = await client.get_price_no_older_than(FeedId.parse("BTC/USD"), int(time.time()), 30)
    print(f"BTC/USD = {price}")

asyncio.run(main())
```

## Testing

```bash
python3 -m unittest discover -s tests -v
```

See the top-level [README](../README.md) for the full design overview.
