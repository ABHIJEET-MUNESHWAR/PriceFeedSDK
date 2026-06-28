"""Domain error type, mirroring the Rust core's ``PriceError``."""

from __future__ import annotations

from typing import Final

_CODES: Final = {
    "empty_feed",
    "feed_id_too_long",
    "invalid_feed_char",
    "zero_confidence",
    "non_positive_timestamp",
    "not_trading",
    "stale",
}


class PriceError(Exception):
    """A domain error raised while constructing or interpreting price data."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code

    @classmethod
    def empty_feed(cls) -> "PriceError":
        return cls("empty_feed", "feed id must not be empty")

    @classmethod
    def feed_id_too_long(cls, max_len: int) -> "PriceError":
        return cls("feed_id_too_long", f"feed id exceeds the maximum length of {max_len}")

    @classmethod
    def invalid_feed_char(cls, ch: str) -> "PriceError":
        return cls("invalid_feed_char", f"feed id contains an invalid character: {ch!r}")

    @classmethod
    def zero_confidence(cls) -> "PriceError":
        return cls("zero_confidence", "confidence must be greater than zero")

    @classmethod
    def non_positive_timestamp(cls, ts: int) -> "PriceError":
        return cls("non_positive_timestamp", f"publish time must be positive, got {ts}")

    @classmethod
    def not_trading(cls, status: str) -> "PriceError":
        return cls("not_trading", f"feed is not trading (status: {status})")

    @classmethod
    def stale(cls, age: int, max_age: int) -> "PriceError":
        return cls("stale", f"price is stale: age {age}s exceeds the maximum of {max_age}s")
