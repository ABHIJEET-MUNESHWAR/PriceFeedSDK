//! Timeout and exponential-backoff retry helpers.

use std::future::Future;
use std::time::Duration;

use crate::error::ClientError;

/// Configuration for retrying a fallible async operation.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// Maximum number of *additional* attempts after the first.
    pub max_retries: u32,
    /// Delay before the first retry.
    pub base_delay: Duration,
    /// Upper bound on any single backoff delay.
    pub max_delay: Duration,
    /// Multiplier applied to the delay after each attempt.
    pub multiplier: u32,
    /// Per-attempt timeout. `None` disables the timeout.
    pub per_attempt_timeout: Option<Duration>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(2),
            multiplier: 2,
            per_attempt_timeout: Some(Duration::from_secs(5)),
        }
    }
}

impl RetryPolicy {
    /// A policy that performs exactly one attempt (no retries, no timeout).
    #[must_use]
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            base_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            multiplier: 1,
            per_attempt_timeout: None,
        }
    }

    fn delay_for(&self, attempt: u32) -> Duration {
        let factor = self.multiplier.saturating_pow(attempt);
        let scaled = self.base_delay.saturating_mul(factor.max(1));
        scaled.min(self.max_delay)
    }
}

/// Runs `op` under the [`RetryPolicy`], retrying only on retryable errors.
///
/// `op` is a closure producing a fresh future per attempt so the request can be
/// re-issued cleanly. Non-retryable errors short-circuit immediately.
pub async fn run_with_policy<F, Fut, T>(policy: RetryPolicy, mut op: F) -> Result<T, ClientError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, ClientError>>,
{
    let mut attempt = 0u32;
    loop {
        let result = match policy.per_attempt_timeout {
            Some(limit) => match tokio::time::timeout(limit, op()).await {
                Ok(r) => r,
                Err(_) => Err(ClientError::Timeout {
                    elapsed_ms: limit.as_millis(),
                }),
            },
            None => op().await,
        };

        match result {
            Ok(value) => return Ok(value),
            Err(err) if err.is_retryable() && attempt < policy.max_retries => {
                let delay = policy.delay_for(attempt);
                tracing::debug!(attempt, ?delay, error = %err, "retrying price fetch");
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                attempt += 1;
            }
            Err(err) => return Err(err),
        }
    }
}
