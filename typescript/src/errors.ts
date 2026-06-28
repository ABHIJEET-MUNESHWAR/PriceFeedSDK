/** Stable, machine-readable error codes shared across all language bindings. */
export type PriceErrorCode =
  | "empty_feed"
  | "feed_id_too_long"
  | "invalid_feed_char"
  | "zero_confidence"
  | "non_positive_timestamp"
  | "not_trading"
  | "stale";

/** A domain error raised while constructing or interpreting price data. */
export class PriceError extends Error {
  readonly code: PriceErrorCode;

  constructor(code: PriceErrorCode, message: string) {
    super(message);
    this.name = "PriceError";
    this.code = code;
  }

  static emptyFeed(): PriceError {
    return new PriceError("empty_feed", "feed id must not be empty");
  }

  static feedIdTooLong(max: number): PriceError {
    return new PriceError(
      "feed_id_too_long",
      `feed id exceeds the maximum length of ${max}`,
    );
  }

  static invalidFeedChar(ch: string): PriceError {
    return new PriceError(
      "invalid_feed_char",
      `feed id contains an invalid character: '${ch}'`,
    );
  }

  static zeroConfidence(): PriceError {
    return new PriceError("zero_confidence", "confidence must be greater than zero");
  }

  static nonPositiveTimestamp(ts: number): PriceError {
    return new PriceError(
      "non_positive_timestamp",
      `publish time must be positive, got ${ts}`,
    );
  }

  static notTrading(status: string): PriceError {
    return new PriceError("not_trading", `feed is not trading (status: ${status})`);
  }

  static stale(age: number, maxAge: number): PriceError {
    return new PriceError(
      "stale",
      `price is stale: age ${age}s exceeds the maximum of ${maxAge}s`,
    );
  }
}
