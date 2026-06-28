import type { FeedId, Price, PriceUpdate } from "./types.js";
import { fetchManyDefault, type PriceTransport } from "./transport.js";
import {
  DEFAULT_RETRY_POLICY,
  runWithPolicy,
  type RetryPolicy,
} from "./retry.js";

/**
 * A high-level client over any {@link PriceTransport}.
 *
 * Adds resilience (timeouts + backoff retries) and consumer-grade conveniences
 * (`getPriceNoOlderThan`) on top of a bare transport.
 */
export class PriceClient {
  private readonly policy: RetryPolicy;

  constructor(
    private readonly transport: PriceTransport,
    policy: RetryPolicy = DEFAULT_RETRY_POLICY,
  ) {
    this.policy = policy;
  }

  /** Fetches the latest update for `feed`, applying timeouts and retries. */
  async get(feed: FeedId): Promise<PriceUpdate> {
    return runWithPolicy(this.policy, () => this.transport.fetch(feed));
  }

  /** Fetches updates for many feeds, applying timeouts and retries. */
  async getMany(feeds: FeedId[]): Promise<PriceUpdate[]> {
    return runWithPolicy(this.policy, () =>
      this.transport.fetchMany
        ? this.transport.fetchMany(feeds)
        : fetchManyDefault(this.transport, feeds),
    );
  }

  /** Fetches `feed` and returns the price only if trading and fresh. */
  async getPriceNoOlderThan(
    feed: FeedId,
    now: number,
    maxAgeSecs: number,
  ): Promise<Price> {
    const update = await this.get(feed);
    return update.priceNoOlderThan(now, maxAgeSecs);
  }
}
