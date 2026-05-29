# Retry Backoff Docs Fix

Align `docs/features/RETRY_BACKOFF.md` with the actual `RetryConfig` struct in `src/retry.rs`.

- Fixed field names from `initial_delay_ms` to `base_delay_ms`.
- Updated retry classification text to match `is_retryable` implementation.

closes #535
