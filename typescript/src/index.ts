/**
 * PriceFeed TypeScript SDK — generic, transport-agnostic price client.
 *
 * Mirrors the Rust core's domain types and behavior so consumers get identical
 * validation, staleness, and confidence semantics in either language.
 */
export {
  FeedId,
  MAX_FEED_ID_LEN,
  Price,
  PriceStatus,
  PriceUpdate,
  isTradeable,
  statusFromString,
} from "./types.js";
export { PriceError, type PriceErrorCode } from "./errors.js";
export {
  ClientError,
  DEFAULT_RETRY_POLICY,
  runWithPolicy,
  type RetryPolicy,
} from "./retry.js";
export { type PriceTransport, fetchManyDefault } from "./transport.js";
export { CachingTransport } from "./cache.js";
export { PriceClient } from "./client.js";
export { HttpTransport, parsePriceResponse } from "./http.js";
