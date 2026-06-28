"""The generic, ergonomic price client."""

from __future__ import annotations

from typing import List

from .transport import (
    PriceTransport,
    RetryPolicy,
    fetch_many_default,
    run_with_policy,
)
from .types import FeedId, Price, PriceUpdate


class PriceClient:
    """A high-level client over any :class:`PriceTransport`.

    Adds resilience (timeouts + backoff retries) and consumer-grade
    conveniences (``get_price_no_older_than``) on top of a bare transport.
    """

    def __init__(self, transport: PriceTransport, policy: RetryPolicy | None = None) -> None:
        self._transport = transport
        self._policy = policy if policy is not None else RetryPolicy()

    async def get(self, feed: FeedId) -> PriceUpdate:
        return await run_with_policy(self._policy, lambda: self._transport.fetch(feed))

    async def get_many(self, feeds: List[FeedId]) -> List[PriceUpdate]:
        return await run_with_policy(
            self._policy, lambda: fetch_many_default(self._transport, feeds)
        )

    async def get_price_no_older_than(
        self, feed: FeedId, now: int, max_age: int
    ) -> Price:
        update = await self.get(feed)
        return update.price_no_older_than(now, max_age)
