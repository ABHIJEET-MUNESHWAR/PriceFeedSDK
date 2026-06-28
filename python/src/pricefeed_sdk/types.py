"""Core domain types, mirroring the Rust core."""

from __future__ import annotations

import re
from dataclasses import dataclass
from enum import Enum
from typing import Final, Tuple

from .errors import PriceError

MAX_FEED_ID_LEN: Final = 32
_FEED_ID_PATTERN: Final = re.compile(r"^[A-Z0-9./-]+$")


@dataclass(frozen=True)
class FeedId:
    """A validated, normalized feed symbol.

    Use :meth:`parse` to construct; the constructor assumes a valid value.
    """

    value: str

    @classmethod
    def parse(cls, symbol: str) -> "FeedId":
        """Validate and normalize ``symbol``, raising :class:`PriceError`."""
        if not symbol:
            raise PriceError.empty_feed()
        if len(symbol) > MAX_FEED_ID_LEN:
            raise PriceError.feed_id_too_long(MAX_FEED_ID_LEN)
        upper = symbol.upper()
        if not _FEED_ID_PATTERN.match(upper):
            bad = next((c for c in upper if not re.match(r"[A-Z0-9./-]", c)), "")
            raise PriceError.invalid_feed_char(bad)
        return cls(upper)

    def as_str(self) -> str:
        return self.value

    def __str__(self) -> str:
        return self.value


class PriceStatus(str, Enum):
    """Feed status, mirroring the Rust ``PriceStatus``."""

    UNKNOWN = "unknown"
    TRADING = "trading"
    HALTED = "halted"

    @classmethod
    def from_str(cls, raw: str) -> "PriceStatus":
        try:
            return cls(raw.lower())
        except ValueError:
            return cls.UNKNOWN

    def is_tradeable(self) -> bool:
        return self is PriceStatus.TRADING


@dataclass(frozen=True)
class Price:
    """A fixed-point price with an explicit confidence interval.

    The real value is ``mantissa * 10**expo``; the confidence half-width is
    ``conf * 10**expo``.
    """

    mantissa: int
    conf: int
    expo: int

    def scale(self) -> float:
        return 10.0 ** self.expo

    def value(self) -> float:
        return self.mantissa * self.scale()

    def confidence(self) -> float:
        return self.conf * self.scale()

    def confidence_interval(self) -> Tuple[float, float]:
        v = self.value()
        c = self.confidence()
        return (v - c, v + c)

    def confidence_ratio(self) -> float:
        if self.mantissa == 0:
            return float("inf")
        return self.conf / abs(self.mantissa)

    def is_confident_within(self, max_ratio: float) -> bool:
        return self.confidence_ratio() <= max_ratio

    def __str__(self) -> str:
        decimals = -self.expo if self.expo < 0 else 0
        return f"{self.value():.{decimals}f} ± {self.confidence():.{decimals}f}"


@dataclass(frozen=True)
class PriceUpdate:
    """A timestamped price observation for a feed."""

    feed_id: FeedId
    price: Price
    status: PriceStatus
    publish_time: int

    @classmethod
    def create(
        cls,
        feed_id: FeedId,
        price: Price,
        status: PriceStatus,
        publish_time: int,
    ) -> "PriceUpdate":
        """Build a validated update, raising :class:`PriceError` on bad invariants."""
        if price.conf <= 0:
            raise PriceError.zero_confidence()
        if publish_time <= 0:
            raise PriceError.non_positive_timestamp(publish_time)
        return cls(feed_id, price, status, publish_time)

    def age(self, now: int) -> int:
        return max(0, now - self.publish_time)

    def is_fresh(self, now: int, max_age: int) -> bool:
        return self.age(now) <= max_age

    def price_no_older_than(self, now: int, max_age: int) -> Price:
        """Return the price only if trading and no older than ``max_age``."""
        if not self.status.is_tradeable():
            raise PriceError.not_trading(self.status.value)
        age = self.age(now)
        if age > max_age:
            raise PriceError.stale(age, max_age)
        return self.price
