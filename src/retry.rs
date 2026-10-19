extern crate alloc;
use alloc::vec::Vec;

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
    /// List of non-retryable error codes that should fail immediately.
    pub non_retryable: Vec<u32>,
    /// Total cumulative wait budget in milliseconds. Once the sum of all
    /// delays reaches or exceeds this value, no further retries are attempted.
    /// Set to `u64::MAX` (the default) to disable the budget cap.
    pub budget_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        RetryConfig {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5_000,
            backoff_multiplier: 2,
            non_retryable: Vec::new(),
            budget_ms: u64::MAX,
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
            non_retryable: Vec::new(),
            budget_ms: u64::MAX,
        }
    }

    pub fn with_non_retryable(mut self, codes: Vec<u32>) -> Self {
        self.non_retryable = codes;
        self
    }

    /// Check if an error code is in the non-retryable list.
    pub fn is_non_retryable(&self, error_code: u32) -> bool {
        self.non_retryable.contains(&error_code)
    }

    /// Compute the delay (ms) for a given attempt index (0-based), with full jitter.
    ///
    /// `delay = rand(0..=min(base * multiplier^attempt, max))`
    ///
    /// # Full jitter
    ///
    /// Full jitter spreads retries uniformly across the entire backoff window
    /// `[0, capped_delay]` rather than adding a small random offset on top of
    /// the deterministic delay. This gives better thundering-herd protection
    /// when many clients retry simultaneously (see the AWS "Exponential Backoff
    /// And Jitter" post).
    ///
    /// The jitter is derived deterministically from `jitter_seed` via
    /// `seed % (capped + 1)`. This is intentionally **approximate** and
    /// **not cryptographically uniform**:
    ///
    /// - Modulo introduces a small bias toward lower values when `capped + 1`
    ///   is not a power of two. The bias is negligible for typical retry ranges.
    /// - The jitter exists to desynchronize retry storms across clients, not
    ///   to provide secret-quality randomness. Do not use this output for any
    ///   security-sensitive purpose.
    pub fn delay_for_attempt(&self, attempt: u32, jitter_seed: u64) -> u64 {
        let exp = (self.backoff_multiplier as u64).saturating_pow(attempt);
        let raw = self.base_delay_ms.saturating_mul(exp);
        let capped = raw.min(self.max_delay_ms);
        // Full jitter: uniform in [0, capped].
        // Use checked_add to avoid overflow when capped == u64::MAX.
        match capped.checked_add(1) {
            Some(m) => jitter_seed % m,
            None => jitter_seed,
        }
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
///
/// Additional non-retryable errors: UnauthorizedAttestor, ValidationError,
/// InvalidQuote, InvalidServiceType, InvalidTransactionIntent, ComplianceNotMet.
pub fn is_retryable(code: u32) -> bool {
    use crate::errors::ErrorCode;
    code == ErrorCode::ServicesNotConfigured as u32
        || code == ErrorCode::AttestationNotFound as u32
        || code == ErrorCode::StaleQuote as u32
        || code == ErrorCode::NoQuotesAvailable as u32
        || code == ErrorCode::CacheExpired as u32
        || code == ErrorCode::CacheNotFound as u32
}

/// Extract a numeric error code for [`RetryConfig::non_retryable`] checks.
///
/// Return `None` when the error type has no numeric code; in that case only
/// the caller-supplied `retryable` callback decides whether to retry.
pub trait RetryErrorCode {
    fn error_code(&self) -> Option<u32>;
}

impl RetryErrorCode for crate::errors::AnchorKitError {
    fn error_code(&self) -> Option<u32> {
        Some(self.code as u32)
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
///
/// `jitter_seed` must vary per caller/process (e.g. `env.prng()` output, a
/// wall-clock nonce, or a value sampled once at startup). It is mixed with the
/// attempt index so concurrent clients do not share identical retry delays.
///
/// Errors whose [`RetryErrorCode::error_code`] is listed in
/// [`RetryConfig::non_retryable`] fail immediately without retrying,
/// regardless of the `retryable` callback.
pub fn retry_with_backoff<T, E, F, S>(
    config: &RetryConfig,
    jitter_seed: u64,
    mut f: F,
    retryable: impl Fn(&E) -> bool,
    mut sleep_fn: S,
) -> Result<T, E>
where
    E: RetryErrorCode,
    F: FnMut(u32) -> Result<T, E>,
    S: FnMut(u64), // delay_ms: millisecond delay value
{
    let mut last_err: Option<E> = None;
    let mut cumulative_delay_ms: u64 = 0;

    for attempt in 0..config.max_attempts {
        match f(attempt) {
            Ok(val) => return Ok(val),
            Err(e) => {
                if e.error_code()
                    .map(|code| config.is_non_retryable(code))
                    .unwrap_or(false)
                {
                    return Err(e);
                }
                if !retryable(&e) || attempt + 1 >= config.max_attempts {
                    return Err(e);
                }
                // Mix caller-supplied entropy with the attempt index so the
                // same attempt number does not yield identical delays across clients.
                let seed = jitter_seed
                    .wrapping_mul(31)
                    .wrapping_add(attempt as u64 * 17 + 3);
                let delay = config.delay_for_attempt(attempt, seed);
                cumulative_delay_ms = cumulative_delay_ms.saturating_add(delay);
                if cumulative_delay_ms >= config.budget_ms {
                    return Err(e);
                }
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
    use alloc::vec::Vec;

    #[derive(Debug, PartialEq)]
    enum TestError {
        Transient,
        Permanent,
    }

    impl RetryErrorCode for TestError {
        fn error_code(&self) -> Option<u32> {
            None
        }
    }

    #[derive(Debug, PartialEq)]
    struct CodedError {
        code: u32,
    }

    impl RetryErrorCode for CodedError {
        fn error_code(&self) -> Option<u32> {
            Some(self.code)
        }
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
            0,
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
            0,
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
            0,
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
            0,
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
        // Full jitter: delay is in [0, capped]. Use non-zero seed to get non-zero delays.
        // attempt 0 capped = 100, attempt 1 = 200, attempt 2 = 400
        assert!(config.delay_for_attempt(0, 50) <= 100);
        assert!(config.delay_for_attempt(1, 100) <= 200);
        assert!(config.delay_for_attempt(2, 300) <= 400);
        // With seed=capped the result equals capped (modulo wraps to capped)
        assert_eq!(config.delay_for_attempt(0, 100), 100 % 101);
    }

    #[test]
    fn test_delay_capped_at_max() {
        let config = RetryConfig::new(10, 1000, 3_000, 2);
        // attempt 5: 1000 * 2^5 = 32000, capped at 3000; full jitter in [0, 3000]
        assert!(config.delay_for_attempt(5, 99999) <= 3_000);
    }

    #[test]
    fn test_sleep_called_between_retries() {
        let config = RetryConfig::new(3, 50, 5000, 2);
        let mut sleep_calls = 0u32;
        let _ = retry_with_backoff(
            &config,
            0,
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
            0,
            |_| Err::<i32, _>(TestError::Transient),
            is_retryable_test,
            |delay_ms| delays.push(delay_ms),
        );
        // 2 retries from 3 attempts
        assert_eq!(delays.len(), 2);
        // Full jitter: delays are in [0, capped_delay]
        assert!(delays[0] <= 100);
        assert!(delays[1] <= 200);
    }

    #[test]
    #[should_panic(expected = "max_attempts must be at least 1")]
    fn test_max_attempts_zero_panics() {
        let _ = RetryConfig::new(0, 100, 5000, 2);
    }

    #[test]
    fn test_budget_ms_stops_retries_early() {
        // With jitter_seed=0: attempt 0 seed = 0*31 + 3 = 3, delay = 3 % 101 = 3
        // budget_ms=2 means cumulative (3) >= budget (2) on first retry → stops after 2 calls
        let config = RetryConfig {
            budget_ms: 2,
            ..RetryConfig::new(5, 100, 5000, 2)
        };
        let mut calls = 0u32;
        let result = retry_with_backoff(
            &config,
            0,
            |_| {
                calls += 1;
                Err::<i32, _>(TestError::Transient)
            },
            is_retryable_test,
            |_| {},
        );
        assert_eq!(result, Err(TestError::Transient));
        assert_eq!(calls, 2); // stopped before sleeping/retrying further
    }

    #[test]
    fn test_budget_ms_max_does_not_limit() {
        let config = RetryConfig::new(3, 10, 1000, 2); // budget_ms = u64::MAX by default
        let mut calls = 0u32;
        let _ = retry_with_backoff(
            &config,
            0,
            |_| {
                calls += 1;
                Err::<i32, _>(TestError::Transient)
            },
            is_retryable_test,
            |_| {},
        );
        assert_eq!(calls, 3); // all attempts used
    }

    #[test]
    fn test_jitter_seed_desynchronizes_clients() {
        let config = RetryConfig::new(3, 100, 5000, 2);
        let mut delays_a = Vec::new();
        let mut delays_b = Vec::new();
        let _ = retry_with_backoff(
            &config,
            11,
            |_| Err::<i32, _>(TestError::Transient),
            is_retryable_test,
            |d| delays_a.push(d),
        );
        let _ = retry_with_backoff(
            &config,
            99,
            |_| Err::<i32, _>(TestError::Transient),
            is_retryable_test,
            |d| delays_b.push(d),
        );
        assert_eq!(delays_a.len(), 2);
        assert_eq!(delays_b.len(), 2);
        // Distinct per-call seeds must not produce identical delay sequences.
        assert_ne!(delays_a, delays_b);
    }

    #[test]
    fn test_config_non_retryable_stops_even_if_callback_allows() {
        // Config lists code 42 as non-retryable; callback would otherwise retry.
        let config = RetryConfig::new(5, 10, 1000, 2).with_non_retryable(alloc::vec![42]);
        let mut calls = 0u32;
        let result = retry_with_backoff(
            &config,
            0,
            |_| {
                calls += 1;
                Err::<i32, _>(CodedError { code: 42 })
            },
            |_| true,
            |_| {},
        );
        assert_eq!(result, Err(CodedError { code: 42 }));
        assert_eq!(calls, 1);
    }

    #[test]
    fn test_config_non_retryable_does_not_block_other_codes() {
        let config = RetryConfig::new(3, 10, 1000, 2).with_non_retryable(alloc::vec![42]);
        let mut calls = 0u32;
        let result = retry_with_backoff(
            &config,
            0,
            |_| {
                calls += 1;
                Err::<i32, _>(CodedError { code: 7 })
            },
            |_| true,
            |_| {},
        );
        assert_eq!(result, Err(CodedError { code: 7 }));
        assert_eq!(calls, 3);
    }
}
