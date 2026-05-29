/// Retry configuration for off-chain anchor requests.
#[derive(Clone, Debug)]
pub struct RetryConfig {
    /// Maximum number of attempts (including the first try).
    pub max_attempts: u32,
    /// Initial delay in milliseconds before the first retry.
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds (caps exponential growth).
    pub max_delay_ms: u64,
    /// Multiplier applied to the delay after each failed attempt.
    pub backoff_multiplier: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        RetryConfig {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5_000,
            backoff_multiplier: 2,
        }
    }
}

impl RetryConfig {
    pub fn new(
        max_attempts: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
        backoff_multiplier: u32,
    ) -> Self {
        assert!(max_attempts >= 1, "max_attempts must be at least 1");
        RetryConfig {
            max_attempts,
            base_delay_ms,
            max_delay_ms,
            backoff_multiplier,
        }
    }

    /// Compute the delay (ms) for a given attempt index (0-based), with jitter.
    ///
    /// `delay = min(base * multiplier^attempt, max) + jitter(0..=base/2)`
    ///
    /// # Jitter properties
    ///
    /// The jitter is derived deterministically from `jitter_seed` via
    /// `seed % (base_delay_ms / 2 + 1)`. This is intentionally **approximate**
    /// and **not cryptographically uniform**:
    ///
    /// - When `base_delay_ms / 2 + 1` is not a power of two, modulo introduces
    ///   a small bias toward lower values. The bias is bounded by
    ///   `range / u64::MAX` and is negligible for typical retry ranges
    ///   (sub-millisecond effect for any realistic `base_delay_ms`).
    /// - The jitter exists to desynchronize retry storms across clients, not
    ///   to provide secret-quality randomness. Do not use this output for any
    ///   security-sensitive purpose.
    ///
    /// If unbiased jitter is ever required, swap the modulo for rejection
    /// sampling or a power-of-two mask.
    pub fn delay_for_attempt(&self, attempt: u32, jitter_seed: u64) -> u64 {
        let exp = (self.backoff_multiplier as u64).saturating_pow(attempt);
        let raw = self.base_delay_ms.saturating_mul(exp);
        let capped = raw.min(self.max_delay_ms);
        // Approximate jitter — see doc comment for bias caveat.
        let jitter = jitter_seed % (self.base_delay_ms / 2 + 1);
        capped.saturating_add(jitter)
    }
}

/// Classify whether an error code is retryable.
///
/// Retryable: transient network/server errors.
/// Non-retryable: auth failures, bad input, protocol violations, and
/// rate-limit rejections. `RateLimitExceeded` is intentionally excluded:
/// the rate window only clears after `window_length` ledgers, so retrying
/// in a tight backoff loop would burn through every attempt and still fail.
/// Callers that need to recover from a rate limit should wait for the
/// window to reset (or call the admin reset path) before issuing the
/// next request.
pub fn is_retryable(code: u32) -> bool {
    use crate::errors::ErrorCode;
    match code {
        ErrorCode::ServicesNotConfigured as u32
            | ErrorCode::AttestationNotFound as u32
            | ErrorCode::StaleQuote as u32
            | ErrorCode::NoQuotesAvailable as u32
            | ErrorCode::CacheExpired as u32
            | ErrorCode::CacheNotFound as u32 => true,
        _ => false,
    }
}

/// Execute `f` with exponential backoff retry.
///
/// `f` receives the current attempt number (0-based) and returns `Ok(T)` on
/// success or `Err(E)` on failure.  `retryable` classifies whether an error
/// warrants another attempt.
///
/// A `sleep_fn` callback is provided so callers can inject real or mock sleep
/// (avoids pulling in `std::thread::sleep` or async runtimes).
/// The delay value passed to `sleep_fn` is in **milliseconds**.
pub fn retry_with_backoff<T, E, F, S>(
    config: &RetryConfig,
    mut f: F,
    retryable: impl Fn(&E) -> bool,
    mut sleep_fn: S,
) -> Result<T, E>
where
    F: FnMut(u32) -> Result<T, E>,
    S: FnMut(u64), // delay_ms: millisecond delay value
{
    let mut last_err: Option<E> = None;

    for attempt in 0..config.max_attempts {
        match f(attempt) {
            Ok(val) => return Ok(val),
            Err(e) => {
                if !retryable(&e) || attempt + 1 >= config.max_attempts {
                    return Err(e);
                }
                let delay = config.delay_for_attempt(attempt, attempt as u64 * 17 + 3);
                sleep_fn(delay);
                last_err = Some(e);
            }
        }
    }

    Err(last_err.expect("max_attempts must be >= 1"))
}

#[cfg(test)]
mod retry_tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum TestError {
        Transient,
        Permanent,
    }

    fn is_retryable_test(e: &TestError) -> bool {
        matches!(e, TestError::Transient)
    }

    #[test]
    fn test_success_on_first_try() {
        let config = RetryConfig::default();
        let mut calls = 0u32;
        let result = retry_with_backoff(
            &config,
            |_| {
                calls += 1;
                Ok::<_, TestError>(42)
            },
            is_retryable_test,
            |_| {},
        );
        assert_eq!(result, Ok(42));
        assert_eq!(calls, 1);
    }

    #[test]
    fn test_success_after_retry() {
        let config = RetryConfig::default();
        let mut calls = 0u32;
        let result = retry_with_backoff(
            &config,
            |attempt| {
                calls += 1;
                if attempt < 2 {
                    Err(TestError::Transient)
                } else {
                    Ok(99)
                }
            },
            is_retryable_test,
            |_| {},
        );
        assert_eq!(result, Ok(99));
        assert_eq!(calls, 3);
    }

    #[test]
    fn test_exhausted_retries() {
        let config = RetryConfig::new(3, 10, 1000, 2);
        let mut calls = 0u32;
        let result = retry_with_backoff(
            &config,
            |_| {
                calls += 1;
                Err::<i32, _>(TestError::Transient)
            },
            is_retryable_test,
            |_| {},
        );
        assert_eq!(result, Err(TestError::Transient));
        assert_eq!(calls, 3);
    }

    #[test]
    fn test_non_retryable_error_stops_immediately() {
        let config = RetryConfig::new(5, 10, 1000, 2);
        let mut calls = 0u32;
        let result = retry_with_backoff(
            &config,
            |_| {
                calls += 1;
                Err::<i32, _>(TestError::Permanent)
            },
            is_retryable_test,
            |_| {},
        );
        assert_eq!(result, Err(TestError::Permanent));
        assert_eq!(calls, 1);
    }

    #[test]
    fn test_delay_increases_exponentially() {
        let config = RetryConfig::new(4, 100, 10_000, 2);
        // attempt 0: 100 * 2^0 = 100, attempt 1: 200, attempt 2: 400
        assert!(config.delay_for_attempt(0, 0) >= 100);
        assert!(config.delay_for_attempt(1, 0) >= 200);
        assert!(config.delay_for_attempt(2, 0) >= 400);
    }

    #[test]
    fn test_delay_capped_at_max() {
        let config = RetryConfig::new(10, 1000, 3_000, 2);
        // attempt 5: 1000 * 2^5 = 32000, capped at 3000
        assert!(config.delay_for_attempt(5, 0) <= 3_000 + config.base_delay_ms / 2 + 1);
    }

    #[test]
    fn test_sleep_called_between_retries() {
        let config = RetryConfig::new(3, 50, 5000, 2);
        let mut sleep_calls = 0u32;
        let _ = retry_with_backoff(
            &config,
            |_| Err::<i32, _>(TestError::Transient),
            is_retryable_test,
            |_| sleep_calls += 1,
        );
        // 3 attempts → 2 sleeps (no sleep after last attempt)
        assert_eq!(sleep_calls, 2);
    }

    #[test]
    fn test_sleep_fn_receives_millisecond_delay() {
        let config = RetryConfig::new(3, 100, 5000, 2);
        let mut delays = Vec::new();
        let _ = retry_with_backoff(
            &config,
            |_| Err::<i32, _>(TestError::Transient),
            is_retryable_test,
            |delay_ms| delays.push(delay_ms),
        );
        // 2 retries from 3 attempts
        assert_eq!(delays.len(), 2);
        // First delay is for attempt 0: 100 * 2^0 + jitter = 100 + jitter
        assert!(delays[0] >= 100);
        // Second delay is for attempt 1: 100 * 2^1 + jitter = 200 + jitter
        assert!(delays[1] >= 200);
    }

    #[test]
    #[should_panic(expected = "max_attempts must be at least 1")]
    fn test_max_attempts_zero_panics() {
        let _ = RetryConfig::new(0, 100, 5000, 2);
    }
}
