"""PriceFeed Python SDK — generic, transport-agnostic price client.

Mirrors the Rust core's domain types and behavior so consumers get identical
validation, staleness, and confidence semantics across languages.
"""

from __future__ import annotations

from .cache import CachingTransport
from .client import PriceClient
from .errors import PriceError
from .http import HttpTransport, parse_price_response
from .transport import (
    ClientError,
    PriceTransport,
    RetryPolicy,
    fetch_many_default,
    run_with_policy,
)
from .types import (
    MAX_FEED_ID_LEN,
    FeedId,
    Price,
    PriceStatus,
    PriceUpdate,
)

__all__ = [
    "MAX_FEED_ID_LEN",
    "CachingTransport",
    "ClientError",
    "FeedId",
    "HttpTransport",
    "Price",
    "PriceClient",
    "PriceError",
    "PriceStatus",
    "PriceTransport",
    "PriceUpdate",
    "RetryPolicy",
    "fetch_many_default",
    "parse_price_response",
    "run_with_policy",
]

__version__ = "0.1.0"
