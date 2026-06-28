import { FeedId, Price, PriceUpdate, statusFromString } from "./types.js";
import { ClientError } from "./retry.js";
import type { PriceTransport } from "./transport.js";

const PRICE_QUERY = `query($feed: String!) {
  price(feedId: $feed) {
    feedId
    mantissa
    conf
    expo
    status
    publishTime
  }
}`;

interface WirePrice {
  feedId: string;
  mantissa: number | string;
  conf: number | string;
  expo: number;
  status: string;
  publishTime: number;
}

interface WireResponse {
  data?: { price?: WirePrice | null };
  errors?: Array<{ message: string }>;
}

/** Parses a raw GraphQL JSON body into a {@link PriceUpdate}. */
export function parsePriceResponse(body: string, feed: FeedId): PriceUpdate {
  let parsed: WireResponse;
  try {
    parsed = JSON.parse(body) as WireResponse;
  } catch (e) {
    throw ClientError.permanent(`decode error: ${(e as Error).message}`);
  }

  if (parsed.errors && parsed.errors.length > 0) {
    throw ClientError.permanent(`graphql error: ${parsed.errors[0]!.message}`);
  }

  const price = parsed.data?.price;
  if (!price) {
    throw new ClientError("not_found", `feed not found: ${feed.asString()}`, false);
  }

  return PriceUpdate.create(
    FeedId.parse(price.feedId),
    new Price(BigInt(price.mantissa), BigInt(price.conf), price.expo),
    statusFromString(price.status),
    price.publishTime,
  );
}

/**
 * A {@link PriceTransport} backed by a GraphQL price endpoint.
 *
 * Compatible with the OracleForge / OracleBridge GraphQL surfaces.
 */
export class HttpTransport implements PriceTransport {
  constructor(
    private readonly endpoint: string,
    private readonly fetchImpl: typeof fetch = fetch,
  ) {}

  async fetch(feed: FeedId): Promise<PriceUpdate> {
    let resp: Response;
    try {
      resp = await this.fetchImpl(this.endpoint, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          query: PRICE_QUERY,
          variables: { feed: feed.asString() },
        }),
      });
    } catch (e) {
      throw ClientError.transport(`network error: ${(e as Error).message}`);
    }

    if (!resp.ok) {
      throw ClientError.transport(`network error: HTTP ${resp.status}`);
    }

    const text = await resp.text();
    return parsePriceResponse(text, feed);
  }
}
