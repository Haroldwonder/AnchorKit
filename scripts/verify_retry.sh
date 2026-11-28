#!/usr/bin/env bash
# Verification script for Issue #112: Retry & Exponential Backoff
#
# Every check below runs a real `cargo test` filter and asserts that:
#   1. cargo exited successfully (compiles + no failures), and
#   2. at least one test actually ran.
#
# `cargo test <filter>` exits 0 when the filter matches nothing, printing
# "running 0 tests". A script that only checks the exit code therefore
# passes vacuously. `run_filter` below parses the test harness summary and
# fails when the passed count is zero.

set -uo pipefail

cd "$(dirname "$0")" || exit 1

FILTERS_RUN=0
FAILURES=0

# run_filter <label> <expected_min_tests> <cargo filter>
#
# Runs `cargo test <filter> --lib`, echoes the harness output, and fails if
# the run errored or ran fewer than <expected_min_tests> tests.
run_filter() {
    local label="$1"
    local min_tests="$2"
    local filter="$3"

    printf '\n--- %s\n' "$label"
    printf '    filter: %s\n' "$filter"

    local output
    if ! output=$(cargo test "$filter" --lib 2>&1); then
        printf '    FAIL: cargo test exited non-zero\n'
        printf '%s\n' "$output" | sed 's/^/    /'
        FAILURES=$((FAILURES + 1))
        return 1
    fi

    # The harness prints one "test result: ..." line per test binary. The lib
    # target is the only one selected by `--lib`, but sum all of them to be
    # safe and to avoid misreading a filtered-out target.
    local passed
    passed=$(printf '%s\n' "$output" \
        | grep -oE 'test result: ok\. [0-9]+ passed' \
        | grep -oE '[0-9]+' \
        | awk '{ total += $1 } END { print total + 0 }')

    if [ "$passed" -lt "$min_tests" ]; then
        printf '    FAIL: expected at least %s test(s) to run, but %s did\n' "$min_tests" "$passed"
        printf '    The filter matched nothing — this check would pass vacuously.\n'
        printf '%s\n' "$output" | sed 's/^/    /'
        FAILURES=$((FAILURES + 1))
        return 1
    fi

    FILTERS_RUN=$((FILTERS_RUN + 1))
    printf '    OK: %s test(s) passed\n' "$passed"
    return 0
}

printf '==========================================\n'
printf 'Issue #112: Retry & Exponential Backoff\n'
printf 'Verification Script\n'
printf '==========================================\n'

if ! command -v cargo >/dev/null 2>&1; then
    printf '\ncargo not found on PATH. Install the Rust toolchain to run this script.\n'
    exit 1
fi

# ---------------------------------------------------------------------------
# Retry engine: src/retry.rs `retry_tests` module
# ---------------------------------------------------------------------------
run_filter "retry engine (full module)" 14 'retry_tests::'

run_filter "success on first try" 1 'test_success_on_first_try'
run_filter "success after retry" 1 'test_success_after_retry'
run_filter "attempts exhausted" 1 'test_exhausted_retries'
run_filter "non-retryable error short-circuits" 1 'test_non_retryable_error_stops_immediately'

# ---------------------------------------------------------------------------
# Delay computation
# ---------------------------------------------------------------------------
run_filter "delay grows exponentially" 1 'test_delay_increases_exponentially'
run_filter "delay capped at max" 1 'test_delay_capped_at_max'
run_filter "sleep invoked between retries only" 1 'test_sleep_called_between_retries'
run_filter "sleep receives millisecond delays" 1 'test_sleep_fn_receives_millisecond_delay'
run_filter "jitter desynchronizes clients" 1 'test_jitter_seed_desynchronizes_clients'

# ---------------------------------------------------------------------------
# Budget cap
# ---------------------------------------------------------------------------
run_filter "budget_ms stops retries early" 1 'test_budget_ms_stops_retries_early'
run_filter "budget_ms unset does not limit" 1 'test_budget_ms_max_does_not_limit'

# ---------------------------------------------------------------------------
# Configurable strategy (RetryConfig::new / with_non_retryable)
# ---------------------------------------------------------------------------
run_filter "non_retryable list overrides callback" 1 'test_config_non_retryable_stops_even_if_callback_allows'
run_filter "non_retryable list is code-scoped" 1 'test_config_non_retryable_does_not_block_other_codes'
run_filter "max_attempts below 1 is rejected" 1 'test_max_attempts_zero_panics'

# ---------------------------------------------------------------------------
# Error classification and integration in src/sep6.rs
# ---------------------------------------------------------------------------
run_filter "HTTP status retry classification" 3 'test_is_http_error_retryable'
run_filter "5xx retryable" 1 'test_is_http_error_retryable_5xx_errors'
run_filter "4xx not retryable" 1 'test_is_http_error_retryable_4xx_errors_not_retryable'
run_filter "2xx/3xx/1xx not retryable" 1 'test_is_http_error_retryable_2xx_3xx_1xx_not_retryable'

# The rate limit rejection path is non-retryable by design: the window only
# clears after window_length ledgers, so a tight backoff loop would exhaust
# every attempt and still fail. See is_retryable() in src/retry.rs.
run_filter "rate limit maps to RateLimitExceeded" 1 'test_get_transaction_status_429_returns_rate_limit_exceeded'

run_filter "retry flow: basic" 1 'test_fetch_transaction_status_with_retry_basic_flow'
run_filter "retry flow: honours config" 1 'test_fetch_transaction_status_with_retry_respects_config'
run_filter "retry flow: 503 then success" 1 'test_fetch_transaction_status_with_retry_simulates_503_then_success'
run_filter "retry flow: error handling" 1 'test_fetch_transaction_status_with_retry_error_handling'

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
printf '\n==========================================\n'
printf 'Checks run:   %s\n' "$FILTERS_RUN"
printf 'Failures:     %s\n' "$FAILURES"
printf '==========================================\n'

if [ "$FAILURES" -ne 0 ]; then
    printf 'VERIFICATION FAILED\n'
    exit 1
fi

if [ "$FILTERS_RUN" -eq 0 ]; then
    printf 'VERIFICATION FAILED: no checks executed\n'
    exit 1
fi

printf 'ALL VERIFICATIONS PASSED\n'
printf '\n'
printf 'Verified behaviour:\n'
printf '  - Retry succeeds immediately when the operation succeeds on attempt 1\n'
printf '  - Retry succeeds after transient failures within max_attempts\n'
printf '  - Attempts are bounded by max_attempts and surfaced when exhausted\n'
printf '  - Non-retryable errors fail on the first attempt (no sleep, no retry)\n'
printf '  - Delay grows exponentially and is capped at max_delay_ms (full jitter)\n'
printf '  - Jitter seeds differing across callers produce differing delay sequences\n'
printf '  - budget_ms caps cumulative wait and stops retrying early\n'
printf '  - RetryConfig::non_retryable overrides a permissive retryable callback\n'
printf '  - 5xx statuses are retryable; 1xx/2xx/3xx/4xx are not\n'
printf '  - Rate limit (429) is deliberately non-retryable\n'
printf '  - max_attempts below 1 is rejected rather than silently retried\n'
printf '\n'
printf 'Documentation:\n'
printf '  - docs/features/RETRY_BACKOFF.md\n'
printf '\n'
printf 'Note: RetryConfig is configurable, not strategy-pluggable. There is no\n'
printf 'strategy trait; callers supply their own retryable predicate and sleep\n'
printf 'function to retry_with_backoff.\n'
exit 0