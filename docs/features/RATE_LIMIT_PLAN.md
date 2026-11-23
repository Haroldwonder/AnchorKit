# Rate Limit Detection & Backoff Implementation Plan

> **Status: PLANNING DRAFT — not implemented.**
> This document is a proposal, not shipped feature documentation. Every task
> below is unchecked on purpose. The `Current State Analysis` section describes
> the code as of the commit that drafted this plan; re-verify it against
> `src/` before using this as a starting point, because the error codes in
> particular have changed since the plan was written.

## 1. Information Gathered

### Current State Analysis:

- **retry.rs**: Has basic exponential backoff but NO jitter and NO Retry-After header parsing
- **rate_limiter.rs**: Internal client-side rate limiting (FixedWindow/TokenBucket)
- **errors.rs**: Has `RateLimitExceeded` (error code 16). There is no `ProtocolRateLimitExceeded` variant — HTTP 429 handling maps onto the existing `RateLimitExceeded` code.
- **error_mapping.rs**: Maps HTTP 429 onto `RateLimitExceeded`
- **transport.rs**: Transport layer with MockTransport for testing (no HTTP response parsing)
- **events.rs**: No rate limit events for monitoring

### Gaps Identified:

1. No Retry-After header parsing from HTTP responses
2. No X-RateLimit-\* header parsing (Remaining, Reset)
3. No jitter in exponential backoff
4. No HTTP status code detection in retry logic
5. No rate limit event logging for monitoring
6. No integration between HTTP responses and retry logic

---

## 2. Implementation Plan

### Phase 1: Enhanced Rate Limit Types & Configuration

- [ ] **src/rate_limit_response.rs** (NEW): Create new module for rate limit response parsing
  - `RateLimitInfo` struct: stores retry_after_ms, rate_limit_headers
  - Parse Retry-After header (seconds or HTTP-date)
  - Parse X-RateLimit-Remaining, X-RateLimit-Reset headers
  - Helper to detect if response indicates rate limiting

### Phase 2: Enhanced Retry Configuration

- [ ] **src/retry.rs** (UPDATE): Add jitter and rate-limit-aware configuration
  - Add `jitter_factor` field to RetryConfig (0.0-1.0)
  - Add `use_retry_after` field to use Retry-After header when present
  - Add `rate_limit_initial_delay_ms` for rate limit specific delays
  - Update `calculate_delay()` to include jitter

### Phase 3: Rate Limit Events

- [ ] **src/events.rs** (UPDATE): Add rate limit events
  - `RateLimitEncountered`: When 429 is detected
  - `RateLimitBackoff`: When backoff is applied
  - `RateLimitRecovered`: When retry succeeds after rate limit

### Phase 4: HTTP Status Detection

- [ ] **src/error_mapping.rs** (UPDATE): Enhance error mapping
  - Add function to detect 429 status
  - Add function to extract rate limit headers info
  - Improve rate limit error classification

### Phase 5: Integration & Testing

- [ ] **src/retry_tests.rs** (UPDATE): Add tests for new features
  - Test jitter calculation
  - Test Retry-After header parsing
  - Test rate limit specific backoff

---

## 3. Dependent Files to be Edited

| File                         | Changes                                  |
| ---------------------------- | ---------------------------------------- |
| `src/rate_limit_response.rs` | NEW - Rate limit response parsing        |
| `src/retry.rs`               | UPDATE - Add jitter, Retry-After support |
| `src/events.rs`              | UPDATE - Add rate limit events           |
| `src/error_mapping.rs`       | UPDATE - Enhance 429 detection           |
| `src/lib.rs`                 | UPDATE - Export new modules              |
| `src/retry_tests.rs`         | UPDATE - Add comprehensive tests         |

---

## 4. Followup Steps

1. Implement the new modules and updates
2. Add comprehensive unit tests for each new feature
3. Update documentation in RETRY_BACKOFF.md
4. Test with cargo test
5. Verify no compilation errors

---

## 5. Key Design Decisions

### Jitter Implementation

- Use "Decorrelated Jitter" algorithm for better spread
- Jitter factor: 0.0 = no jitter, 1.0 = full jitter
- Default: 0.5 (50% variance)

### Retry-After Handling

- If Retry-After header present: use it as delay (with min bound)
- Otherwise: use exponential backoff
- Cap at max_delay_ms even with Retry-After

### Rate Limit Specific Configuration

- Separate initial delay for rate limits (default: 1000ms vs regular 100ms)
- Higher backoff multiplier for rate limits (default: 3 vs 2)
- Longer max delay for rate limits (default: 60000ms vs 5000ms)
