import { PriceError } from "./errors.js";

/** Maximum length of a feed symbol (mirrors the Rust core). */
export const MAX_FEED_ID_LEN = 32;

const FEED_ID_PATTERN = /^[A-Z0-9./-]+$/;

/**
 * A validated, normalized feed symbol.
 *
 * Construction enforces the invariant once, so every consumer can rely on a
 * non-empty, bounded, uppercase symbol drawn from `[A-Z0-9./-]`.
 */
export class FeedId {
  private constructor(private readonly value: string) {}

  /** Validates and normalizes `symbol`, throwing {@link PriceError} on failure. */
  static parse(symbol: string): FeedId {
    if (symbol.length === 0) {
      throw PriceError.emptyFeed();
    }
    if (symbol.length > MAX_FEED_ID_LEN) {
      throw PriceError.feedIdTooLong(MAX_FEED_ID_LEN);
    }
    const upper = symbol.toUpperCase();
    if (!FEED_ID_PATTERN.test(upper)) {
      const bad = [...upper].find((ch) => !/[A-Z0-9./-]/.test(ch)) ?? "";
      throw PriceError.invalidFeedChar(bad);
    }
    return new FeedId(upper);
  }

  /** The normalized symbol string. */
  asString(): string {
    return this.value;
  }

  toString(): string {
    return this.value;
  }

  equals(other: FeedId): boolean {
    return this.value === other.value;
  }
}

/** Feed status, mirroring the Rust `PriceStatus`. */
export enum PriceStatus {
  Unknown = "unknown",
  Trading = "trading",
  Halted = "halted",
}

/** Parses a wire status string into a {@link PriceStatus}. */
export function statusFromString(raw: string): PriceStatus {
  switch (raw.toLowerCase()) {
    case "trading":
      return PriceStatus.Trading;
    case "halted":
      return PriceStatus.Halted;
    default:
      return PriceStatus.Unknown;
  }
}

/** Whether reads should be allowed in the given status. */
export function isTradeable(status: PriceStatus): boolean {
  return status === PriceStatus.Trading;
}

/**
 * A fixed-point price with an explicit confidence interval.
 *
 * The real value is `mantissa × 10^expo`; the confidence half-width is
 * `conf × 10^expo`. Bigints preserve exact on-chain semantics.
 */
export class Price {
  constructor(
    readonly mantissa: bigint,
    readonly conf: bigint,
    readonly expo: number,
  ) {}

  /** The scale factor `10^expo`. */
  scale(): number {
    return Math.pow(10, this.expo);
  }

  /** The real-number value `mantissa × 10^expo`. */
  value(): number {
    return Number(this.mantissa) * this.scale();
  }

  /** The confidence half-width as a real number. */
  confidence(): number {
    return Number(this.conf) * this.scale();
  }

  /** The inclusive confidence interval `[value - conf, value + conf]`. */
  confidenceInterval(): [number, number] {
    const v = this.value();
    const c = this.confidence();
    return [v - c, v + c];
  }

  /** The relative confidence `conf / |mantissa|` (Infinity when mantissa is 0). */
  confidenceRatio(): number {
    if (this.mantissa === 0n) {
      return Number.POSITIVE_INFINITY;
    }
    const abs = this.mantissa < 0n ? -this.mantissa : this.mantissa;
    return Number(this.conf) / Number(abs);
  }

  /** Whether the relative confidence is within `maxRatio`. */
  isConfidentWithin(maxRatio: number): boolean {
    return this.confidenceRatio() <= maxRatio;
  }

  toString(): string {
    const decimals = this.expo < 0 ? -this.expo : 0;
    return `${this.value().toFixed(decimals)} ± ${this.confidence().toFixed(decimals)}`;
  }
}

/** A timestamped price observation for a feed. */
export class PriceUpdate {
  private constructor(
    readonly feedId: FeedId,
    readonly price: Price,
    readonly status: PriceStatus,
    readonly publishTime: number,
  ) {}

  /** Builds a validated update, throwing {@link PriceError} on bad invariants. */
  static create(
    feedId: FeedId,
    price: Price,
    status: PriceStatus,
    publishTime: number,
  ): PriceUpdate {
    if (price.conf <= 0n) {
      throw PriceError.zeroConfidence();
    }
    if (publishTime <= 0) {
      throw PriceError.nonPositiveTimestamp(publishTime);
    }
    return new PriceUpdate(feedId, price, status, publishTime);
  }

  /** Age relative to `now`, in seconds (never negative). */
  age(now: number): number {
    return Math.max(0, now - this.publishTime);
  }

  /** Whether the update is no older than `maxAge` seconds at `now`. */
  isFresh(now: number, maxAge: number): boolean {
    return this.age(now) <= maxAge;
  }

  /**
   * Returns the price only if trading and no older than `maxAge` — the SDK
   * analogue of Pyth's `get_price_no_older_than`.
   */
  priceNoOlderThan(now: number, maxAge: number): Price {
    if (!isTradeable(this.status)) {
      throw PriceError.notTrading(this.status);
    }
    const age = this.age(now);
    if (age > maxAge) {
      throw PriceError.stale(age, maxAge);
    }
    return this.price;
  }
}
