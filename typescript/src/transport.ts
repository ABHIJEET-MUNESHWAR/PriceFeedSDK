import type { FeedId, PriceUpdate } from "./types.js";

/**
 * A source of price updates — the SDK's single extension point.
 *
 * HTTP, websocket, or in-memory backends all implement this one interface, and
 * the generic {@link PriceClient} composes behavior on top of it.
 */
export interface PriceTransport {
  /** Fetches the latest update for a single feed. */
  fetch(feed: FeedId): Promise<PriceUpdate>;

  /**
   * Fetches the latest updates for many feeds. Implementations that support
   * batching should override this; otherwise {@link fetchManyDefault} is used.
   */
  fetchMany?(feeds: FeedId[]): Promise<PriceUpdate[]>;
}

/** Default sequential implementation of `fetchMany`. */
export async function fetchManyDefault(
  transport: PriceTransport,
  feeds: FeedId[],
): Promise<PriceUpdate[]> {
  const out: PriceUpdate[] = [];
  for (const feed of feeds) {
    out.push(await transport.fetch(feed));
  }
  return out;
}
