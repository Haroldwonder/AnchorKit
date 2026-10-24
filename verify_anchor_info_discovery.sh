#!/usr/bin/env bash
# Verification script: Anchor Info Discovery
#
# This script performs real checks against the working tree. It does not
# print a hardcoded summary. Every assertion below either inspects the
# filesystem, greps the source, or runs `cargo test` and requires a
# non-zero test count.
#
# Layout note: there is no `src/anchor_info_discovery.rs`. The discovery
# implementation lives in `src/contract.rs` alongside the rest of the
# contract API; the tests live in `src/anchor_info_discovery_tests.rs`
# and are registered in `src/lib.rs`.

set -uo pipefail

cd "$(dirname "$0")" || exit 1

CHECKS=0
FAILURES=0

pass() {
    CHECKS=$((CHECKS + 1))
    printf '  [ok]   %s\n' "$1"
}

fail() {
    CHECKS=$((CHECKS + 1))
    FAILURES=$((FAILURES + 1))
    printf '  [FAIL] %s\n' "$1"
}

require_file() {
    if [ -f "$1" ]; then
        pass "$1 exists"
    else
        fail "$1 is missing"
    fi
}

# require_symbol <file> <regex> <label>
require_symbol() {
    local file="$1"
    local regex="$2"
    local label="$3"

    if [ ! -f "$file" ]; then
        fail "$label ($file missing, cannot check)"
        return
    fi
    if grep -qE "$regex" "$file"; then
        pass "$label"
    else
        fail "$label"
    fi
}

# require_absent_symbol <file> <regex> <label>
require_absent_symbol() {
    local file="$1"
    local regex="$2"
    local label="$3"

    if [ ! -f "$file" ]; then
        fail "$label ($file missing, cannot check)"
        return
    fi
    if grep -qE "$regex" "$file"; then
        fail "$label"
    else
        pass "$label"
    fi
}

# require_test <filter> <expected_min> <label>
#
# Runs `cargo test <filter> --lib` and fails when cargo errors or when the
# filter matched zero tests. `cargo test <filter>` exits 0 on a filter that
# matches nothing, so the passed count has to be parsed explicitly.
require_test() {
    local filter="$1"
    local min_tests="$2"
    local label="$3"

    local output passed
    if ! output=$(cargo test "$filter" --lib 2>&1); then
        fail "$label — cargo test exited non-zero"
        printf '%s\n' "$output" | sed 's/^/         /' | tail -n 25
        return
    fi

    passed=$(printf '%s\n' "$output" \
        | grep -oE 'test result: ok\. [0-9]+ passed' \
        | grep -oE '[0-9]+' \
        | awk '{ total += $1 } END { print total + 0 }')

    if [ "$passed" -lt "$min_tests" ]; then
        fail "$label — expected >= $min_tests test(s), $passed ran"
        return
    fi
    pass "$label ($passed test(s))"
}

printf '=== Anchor Info Discovery - Verification ===\n'
printf '\n'

# ---------------------------------------------------------------------------
printf 'Files\n'
printf -- '---------------------------------------------------------------------------\n'
require_file src/contract.rs
require_file src/anchor_info_discovery_tests.rs
require_file src/lib.rs
require_file src/errors.rs
require_file docs/features/ANCHOR_INFO_DISCOVERY.md

if [ -f examples/anchor_info_discovery.sh ]; then
    pass "examples/anchor_info_discovery.sh exists"
else
    fail "examples/anchor_info_discovery.sh is missing"
fi

# ---------------------------------------------------------------------------
printf '\nModule registration\n'
printf -- '---------------------------------------------------------------------------\n'
require_symbol src/lib.rs '^mod anchor_info_discovery_tests;' 'anchor_info_discovery_tests is declared in src/lib.rs'
require_symbol src/lib.rs '^#\[cfg\(test\)\]' 'test modules are cfg(test)-gated'

# ---------------------------------------------------------------------------
printf '\nPublic API surface (src/contract.rs)\n'
printf -- '---------------------------------------------------------------------------\n'
require_symbol src/contract.rs 'pub fn fetch_anchor_info\('      'fetch_anchor_info'
require_symbol src/contract.rs 'pub fn get_anchor_toml\('        'get_anchor_toml'
require_symbol src/contract.rs 'pub fn refresh_anchor_info\('    'refresh_anchor_info'
require_symbol src/contract.rs 'pub fn get_anchor_assets\('      'get_anchor_assets'
require_symbol src/contract.rs 'pub fn get_anchor_currencies\('  'get_anchor_currencies'
require_symbol src/contract.rs 'pub fn get_anchor_asset_info\('  'get_anchor_asset_info'
require_symbol src/contract.rs 'pub fn get_anchor_deposit_limits\('    'get_anchor_deposit_limits'
require_symbol src/contract.rs 'pub fn get_anchor_withdrawal_limits\(' 'get_anchor_withdrawal_limits'
require_symbol src/contract.rs 'pub fn get_anchor_deposit_fees\('      'get_anchor_deposit_fees'
require_symbol src/contract.rs 'pub fn get_anchor_withdrawal_fees\('   'get_anchor_withdrawal_fees'
require_symbol src/contract.rs 'pub fn anchor_supports_deposits\('     'anchor_supports_deposits'
require_symbol src/contract.rs 'pub fn anchor_supports_withdrawals\('  'anchor_supports_withdrawals'

# ---------------------------------------------------------------------------
printf '\nBehaviour\n'
printf -- '---------------------------------------------------------------------------\n'
require_symbol src/contract.rs 'pub fn fetch_anchor_info\(.*ttl_override' 'fetch_anchor_info accepts a TTL override'
require_symbol src/contract.rs 'ttl_override\.unwrap_or\(3600\)'           'default TTL is 3600s'
require_symbol src/contract.rs 'cached\.cached_at \+ cached\.ttl_seconds <= now' 'expiry is evaluated on read'
require_symbol src/contract.rs 'validate_anchor_domain\(transfer_server_str\)'   'transfer_server is domain-validated'
require_symbol src/contract.rs 'asset\.decimals > 18'                          'currency decimals are range-checked'
require_symbol src/contract.rs 'fn add_to_cached_anchors\('                    'anchors are tracked for multi-anchor queries'

# Refresh must be able to force-evict a live entry, and must evict an
# expired one even when force is false.
require_symbol src/contract.rs 'if force \{' 'refresh_anchor_info supports forced eviction'

# ---------------------------------------------------------------------------
printf '\nError variants\n'
printf -- '---------------------------------------------------------------------------\n'
require_symbol src/errors.rs '^\s*CacheNotFound = ' 'ErrorCode::CacheNotFound exists'
require_symbol src/errors.rs '^\s*CacheExpired = '  'ErrorCode::CacheExpired exists'
require_symbol src/errors.rs '^\s*NotInitialized = ' 'ErrorCode::NotInitialized exists'
require_symbol src/errors.rs '^\s*InvalidEndpointFormat = ' 'ErrorCode::InvalidEndpointFormat exists'
require_symbol src/errors.rs '^\s*ValidationError = ' 'ErrorCode::ValidationError exists'
require_symbol src/errors.rs '^\s*UnauthorizedAttestor = ' 'ErrorCode::UnauthorizedAttestor exists'

# UnsupportedAsset was previously claimed here but never existed. It must
# stay absent — the API returns ValidationError for an unknown asset code.
require_absent_symbol src/errors.rs '^\s*UnsupportedAsset = ' 'ErrorCode::UnsupportedAsset is absent (unknown asset codes return ValidationError)'

require_symbol src/contract.rs 'Err\(ErrorCode::ValidationError\)' 'unknown asset code returns ValidationError'

# ---------------------------------------------------------------------------
printf '\nData types\n'
printf -- '---------------------------------------------------------------------------\n'
require_symbol src/types.rs 'pub struct StellarToml' 'StellarToml'
require_symbol src/types.rs 'pub struct AssetInfo'  'AssetInfo'
require_symbol src/types.rs 'pub struct CachedToml'  'CachedToml'
require_symbol src/types.rs 'pub struct FiatCurrency' 'FiatCurrency'

# ---------------------------------------------------------------------------
printf '\nTests\n'
printf -- '---------------------------------------------------------------------------\n'
if ! command -v cargo >/dev/null 2>&1; then
    printf '  [skip] cargo not found on PATH — cannot run the test suite\n'
    printf '         Install the Rust toolchain and re-run to verify behaviour.\n'
else
    require_test 'anchor_info_discovery_tests' 31 'anchor_info_discovery_tests module'
    require_test 'test_fetch_and_cache_toml'  1  'fetch caches TOML'
    require_test 'test_get_cached_toml'        1  'get_anchor_toml reads the cache'
    require_test 'test_cache_not_found'        1  'cache miss returns CacheNotFound'
    require_test 'test_cache_expiration'       1  'expired cache returns CacheExpired'
    require_test 'test_cache_ttl_custom'       1  'custom TTL is honoured'
    require_test 'test_refresh_cache'          1  'forced refresh evicts the cache'
    require_test 'test_refresh_cache_force_false' 1 'non-forced refresh evicts only when expired'
    require_test 'test_get_supported_assets'   1  'asset listing'
    require_test 'test_get_asset_info_not_found' 1 'unknown asset code'
    require_test 'test_get_deposit_limits'     1  'deposit limits'
    require_test 'test_get_withdrawal_limits'  1  'withdrawal limits'
    require_test 'test_get_deposit_fees'       1  'deposit fees'
    require_test 'test_get_withdrawal_fees'    1  'withdrawal fees'
    require_test 'test_supports_deposits'      1  'deposit support flag'
    require_test 'test_supports_withdrawals'   1  'withdrawal support flag'
    require_test 'test_get_anchor_currencies_with_fiat_entries' 1 'fiat currency passthrough'
    require_test 'test_multiple_anchors'       1  'per-anchor cache isolation'
    require_test 'test_asset_limits_validation' 1 'limit validation'
    require_test 'test_asset_decimals'         1  'decimals handling'
fi

# ---------------------------------------------------------------------------
printf '\n==========================================\n'
printf 'Checks:   %s\n' "$CHECKS"
printf 'Failures: %s\n' "$FAILURES"
printf '==========================================\n'

if [ "$FAILURES" -ne 0 ]; then
    printf 'VERIFICATION FAILED\n'
    exit 1
fi

printf 'VERIFICATION PASSED\n'
exit 0