"""A TTL cache decorator wrapping any transport."""

from __future__ import annotations

import time
from typing import Dict, Tuple

from .types import FeedId, PriceUpdate


class CachingTransport:
    """Wraps an inner transport with a time-to-live in-memory cache.

    ``CachingTransport`` *is a* transport, so it composes with the generic
    client exactly like any other backend.
    """

    def __init__(self, inner: "object", ttl_s: float) -> None:
        self._inner = inner
        self._ttl_s = ttl_s
        self._cache: Dict[str, Tuple[PriceUpdate, float]] = {}

    async def fetch(self, feed: FeedId) -> PriceUpdate:
        key = feed.as_str()
        hit = self._cache.get(key)
        if hit is not None and (time.monotonic() - hit[1]) < self._ttl_s:
            return hit[0]
        fresh = await self._inner.fetch(feed)  # type: ignore[attr-defined]
        self._cache[key] = (fresh, time.monotonic())
        return fresh

    def size(self) -> int:
        return len(self._cache)

    def clear(self) -> None:
        self._cache.clear()
