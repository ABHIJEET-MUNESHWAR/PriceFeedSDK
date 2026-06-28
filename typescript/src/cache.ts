import type { FeedId, PriceUpdate } from "./types.js";
import type { PriceTransport } from "./transport.js";

interface Entry {
  update: PriceUpdate;
  fetchedAtMs: number;
}

/**
 * A TTL cache decorator that wraps any {@link PriceTransport}.
 *
 * `CachingTransport` *is a* `PriceTransport`, so it composes with the generic
 * client exactly like any other backend. Entries older than `ttlMs` are
 * refreshed on access.
 */
export class CachingTransport implements PriceTransport {
  private readonly cache = new Map<string, Entry>();

  constructor(
    private readonly inner: PriceTransport,
    private readonly ttlMs: number,
  ) {}

  async fetch(feed: FeedId): Promise<PriceUpdate> {
    const key = feed.asString();
    const hit = this.cache.get(key);
    if (hit && Date.now() - hit.fetchedAtMs < this.ttlMs) {
      return hit.update;
    }
    const fresh = await this.inner.fetch(feed);
    this.cache.set(key, { update: fresh, fetchedAtMs: Date.now() });
    return fresh;
  }

  /** Number of cached entries. */
  size(): number {
    return this.cache.size;
  }

  /** Drops every cached entry. */
  clear(): void {
    this.cache.clear();
  }
}
