"""Client-layer errors, retry policy, and transport protocol."""

from __future__ import annotations

import asyncio
from dataclasses import dataclass
from typing import Awaitable, Callable, List, Protocol, TypeVar

from .types import FeedId, PriceUpdate

T = TypeVar("T")


class ClientError(Exception):
    """An error surfaced by a transport or client."""

    def __init__(self, code: str, message: str, retryable: bool) -> None:
        super().__init__(message)
        self.code = code
        self.retryable = retryable

    @classmethod
    def timeout(cls, elapsed_ms: float) -> "ClientError":
        return cls("timeout", f"request timed out after {elapsed_ms}ms", True)

    @classmethod
    def transport(cls, message: str) -> "ClientError":
        return cls("transport", message, True)

    @classmethod
    def permanent(cls, message: str) -> "ClientError":
        return cls("transport", message, False)

    @classmethod
    def not_found(cls, feed: FeedId) -> "ClientError":
        return cls("not_found", f"feed not found: {feed.as_str()}", False)


@dataclass(frozen=True)
class RetryPolicy:
    """Configuration for retrying a fallible async operation."""

    max_retries: int = 3
    base_delay_s: float = 0.05
    max_delay_s: float = 2.0
    multiplier: int = 2
    per_attempt_timeout_s: float = 5.0

    @classmethod
    def none(cls) -> "RetryPolicy":
        return cls(max_retries=0, base_delay_s=0.0, max_delay_s=0.0, multiplier=1, per_attempt_timeout_s=0.0)

    def delay_for(self, attempt: int) -> float:
        return min(self.base_delay_s * (self.multiplier ** attempt), self.max_delay_s)


class PriceTransport(Protocol):
    """A source of price updates — the SDK's single extension point."""

    async def fetch(self, feed: FeedId) -> PriceUpdate:
        ...


async def fetch_many_default(
    transport: PriceTransport, feeds: List[FeedId]
) -> List[PriceUpdate]:
    """Default sequential implementation of fetching many feeds."""
    out: List[PriceUpdate] = []
    for feed in feeds:
        out.append(await transport.fetch(feed))
    return out


async def run_with_policy(policy: RetryPolicy, op: Callable[[], Awaitable[T]]) -> T:
    """Run ``op`` under the :class:`RetryPolicy`, retrying only retryable errors."""
    attempt = 0
    while True:
        try:
            if policy.per_attempt_timeout_s > 0:
                return await asyncio.wait_for(op(), timeout=policy.per_attempt_timeout_s)
            return await op()
        except asyncio.TimeoutError:
            err: Exception = ClientError.timeout(policy.per_attempt_timeout_s * 1000)
        except ClientError as e:
            err = e
            if not e.retryable:
                raise
        if attempt >= policy.max_retries:
            raise err
        delay = policy.delay_for(attempt)
        if delay > 0:
            await asyncio.sleep(delay)
        attempt += 1
