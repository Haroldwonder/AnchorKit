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
/// # Classification table
///
/// Retryable (transient — state may change between attempts):
/// - `TransportTimeout`: network/timeout failures are transient by nature.
/// - `EndpointFailure`: the endpoint may recover; retry with backoff.
/// - `ServerError` (5xx-style): the server may recover; retry with backoff.
///
/// Non-retryable (deterministic — a millisecond-scale backoff will not change
/// the outcome unless some other actor mutates state):
/// - `AttestationNotFound`: a missing attestation will not appear on its own.
/// - `ServicesNotConfigured`: configuration is not set up; retrying is futile.
/// - `CacheNotFound`: the cache entry does not exist; retrying won't create it.
/// - `CacheExpired`: the entry is stale; a retry returns the same result.
/// - `NoQuotesAvailable`: no quotes exist; retrying won't conjure them.
/// - `StaleQuote`: the quote is stale; a fresh request is required.
/// - `RateLimitExceeded`: the rate window only clears after `window_length`
///   ledgers, so retrying in a tight backoff loop would burn through every
///   attempt and still fail. Callers should wait for the window to reset (or
///   call the admin reset path) before issuing the next request.
/// - `UnauthorizedAttestor`, `ValidationError`, `InvalidQuote`,
///   `InvalidServiceType`, `InvalidTransactionIntent`, `ComplianceNotMet`:
///   auth failures, bad input, and protocol violations are permanent.
///
/// Callers that genuinely need to recover from a deterministic error should
/// wait for the relevant state change (or use an explicit long backoff)
/// rather than relying on this classifier.
pub fn is_retryable(code: u32) -> bool {
    use crate::errors::ErrorCode;
    code == ErrorCode::TransportTimeout as u32
        || code == ErrorCode::EndpointFailure as u32
        || code == ErrorCode::ServerError as u32
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

    #[test]
    fn is_retryable_classification_table() {
        use crate::errors::ErrorCode;

        // Transient transport/server errors are retryable.
        assert!(is_retryable(ErrorCode::TransportTimeout as u32));
        assert!(is_retryable(ErrorCode::EndpointFailure as u32));
        assert!(is_retryable(ErrorCode::ServerError as u32));

        // Deterministic errors are non-retryable.
        assert!(!is_retryable(ErrorCode::AttestationNotFound as u32));
        assert!(!is_retryable(ErrorCode::ServicesNotConfigured as u32));
        assert!(!is_retryable(ErrorCode::CacheNotFound as u32));
        assert!(!is_retryable(ErrorCode::CacheExpired as u32));
        assert!(!is_retryable(ErrorCode::NoQuotesAvailable as u32));
        assert!(!is_retryable(ErrorCode::StaleQuote as u32));
        assert!(!is_retryable(ErrorCode::RateLimitExceeded as u32));
    }

    #[test]
    fn attestation_not_found_is_non_retryable() {
        use crate::errors::ErrorCode;
        assert!(!is_retryable(ErrorCode::AttestationNotFound as u32));
    }
}
