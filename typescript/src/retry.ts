/** Whether a transport error should be retried. */
export class ClientError extends Error {
  constructor(
    readonly code: "not_found" | "timeout" | "transport" | "domain",
    message: string,
    readonly retryable: boolean,
  ) {
    super(message);
    this.name = "ClientError";
  }

  static timeout(elapsedMs: number): ClientError {
    return new ClientError("timeout", `request timed out after ${elapsedMs}ms`, true);
  }

  static transport(message: string): ClientError {
    return new ClientError("transport", message, true);
  }

  static permanent(message: string): ClientError {
    return new ClientError("transport", message, false);
  }
}

/** Configuration for retrying a fallible async operation. */
export interface RetryPolicy {
  /** Maximum number of additional attempts after the first. */
  maxRetries: number;
  /** Delay before the first retry, in milliseconds. */
  baseDelayMs: number;
  /** Upper bound on any single backoff delay, in milliseconds. */
  maxDelayMs: number;
  /** Multiplier applied to the delay after each attempt. */
  multiplier: number;
  /** Per-attempt timeout in milliseconds (0 disables it). */
  perAttemptTimeoutMs: number;
}

/** The default retry policy. */
export const DEFAULT_RETRY_POLICY: RetryPolicy = {
  maxRetries: 3,
  baseDelayMs: 50,
  maxDelayMs: 2000,
  multiplier: 2,
  perAttemptTimeoutMs: 5000,
};

const sleep = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

function isRetryable(err: unknown): boolean {
  return err instanceof ClientError && err.retryable;
}

async function withTimeout<T>(p: Promise<T>, ms: number): Promise<T> {
  if (ms <= 0) {
    return p;
  }
  return await Promise.race([
    p,
    new Promise<T>((_, reject) =>
      setTimeout(() => reject(ClientError.timeout(ms)), ms),
    ),
  ]);
}

/** Runs `op` under the {@link RetryPolicy}, retrying only retryable errors. */
export async function runWithPolicy<T>(
  policy: RetryPolicy,
  op: () => Promise<T>,
): Promise<T> {
  let attempt = 0;
  for (;;) {
    try {
      return await withTimeout(op(), policy.perAttemptTimeoutMs);
    } catch (err) {
      if (isRetryable(err) && attempt < policy.maxRetries) {
        const delay = Math.min(
          policy.baseDelayMs * Math.pow(policy.multiplier, attempt),
          policy.maxDelayMs,
        );
        if (delay > 0) {
          await sleep(delay);
        }
        attempt += 1;
        continue;
      }
      throw err;
    }
  }
}
