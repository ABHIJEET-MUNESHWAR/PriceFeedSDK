"""HTTP/GraphQL transport, with an offline-testable response parser."""

from __future__ import annotations

import asyncio
import json
import urllib.error
import urllib.request
from typing import Awaitable, Callable, Optional

from .transport import ClientError
from .types import FeedId, Price, PriceStatus, PriceUpdate

_PRICE_QUERY = """query($feed: String!) {
  price(feedId: $feed) {
    feedId
    mantissa
    conf
    expo
    status
    publishTime
  }
}"""

# A synchronous fetcher: (url, json_body) -> response_text.
Fetcher = Callable[[str, str], str]


def parse_price_response(body: str, feed: FeedId) -> PriceUpdate:
    """Parse a raw GraphQL JSON body into a :class:`PriceUpdate`."""
    try:
        parsed = json.loads(body)
    except json.JSONDecodeError as exc:
        raise ClientError.permanent(f"decode error: {exc}") from exc

    errors = parsed.get("errors")
    if errors:
        raise ClientError.permanent(f"graphql error: {errors[0].get('message', '')}")

    price = (parsed.get("data") or {}).get("price")
    if not price:
        raise ClientError.not_found(feed)

    return PriceUpdate.create(
        FeedId.parse(price["feedId"]),
        Price(int(price["mantissa"]), int(price["conf"]), int(price["expo"])),
        PriceStatus.from_str(price["status"]),
        int(price["publishTime"]),
    )


def _default_fetcher(url: str, body: str) -> str:
    request = urllib.request.Request(
        url,
        data=body.encode("utf-8"),
        headers={"content-type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=10) as resp:  # noqa: S310
            return resp.read().decode("utf-8")
    except urllib.error.HTTPError as exc:
        raise ClientError.transport(f"network error: HTTP {exc.code}") from exc
    except urllib.error.URLError as exc:
        raise ClientError.transport(f"network error: {exc.reason}") from exc


class HttpTransport:
    """A transport backed by a GraphQL price endpoint.

    Compatible with the OracleForge / OracleBridge GraphQL surfaces. A custom
    ``fetcher`` may be injected for testing.
    """

    def __init__(self, endpoint: str, fetcher: Optional[Fetcher] = None) -> None:
        self._endpoint = endpoint
        self._fetcher = fetcher if fetcher is not None else _default_fetcher

    async def fetch(self, feed: FeedId) -> PriceUpdate:
        body = json.dumps(
            {"query": _PRICE_QUERY, "variables": {"feed": feed.as_str()}}
        )
        text = await asyncio.to_thread(self._fetcher, self._endpoint, body)
        return parse_price_response(text, feed)
