# Anchor Health Score Implementation Summary

## Overview

Implemented `get_anchor_health_score` function that computes a 0-100 health score from cached anchor metadata, eliminating the need for callers to manually combine uptime, reputation, and settlement time metrics.

## Implementation Details

### Core Function

**Location:** `src/contract.rs` (after `refresh_metadata_cache`)

**Signature:**
```rust
pub fn get_anchor_health_score(env: Env, anchor: Address) -> u32
```

**Algorithm:**
1. Retrieves cached metadata using `get_cached_metadata` (inherits cache validation)
2. Computes three component scores:
   - Uptime: Direct scaling from 0-10000 to 0-100
   - Reputation: Direct scaling from 0-10000 to 0-100
   - Speed: Tiered scoring based on settlement time thresholds
3. Applies weighted formula: `(40×uptime + 35×reputation + 25×speed) / 100`
4. Caps result at 100

### Formula Weights

| Metric | Weight | Justification |
|--------|--------|---------------|
| Uptime | 40% | Availability is most critical |
| Reputation | 35% | Trust and track record essential |
| Speed | 25% | Important but secondary to availability |

### Settlement Speed Tiers

| Time Range | Score | Rationale |
|------------|-------|-----------|
| 0-300s | 100 | Excellent (≤5 min) |
| 301-600s | 80 | Good (5-10 min) |
| 601-1800s | 60 | Acceptable (10-30 min) |
| 1801-3600s | 40 | Slow (30-60 min) |
| >3600s | 20 | Very slow (>1 hour) |

Tiered approach chosen over continuous function to:
- Provide clear performance categories
- Avoid over-precision in scoring
- Simplify reasoning about anchor quality
- Reduce sensitivity to minor time variations

## Error Handling

Delegates to `get_cached_metadata`, which panics with:
- `CacheNotFound` (49) - No metadata entry exists
- `CacheExpired` (48) - TTL has elapsed

No additional error codes needed.

## Test Coverage

**File:** `src/anchor_health_score_tests.rs`

**Test Cases:**
1. `test_perfect_health_score` - All metrics at maximum (100)
2. `test_good_health_score` - High-quality anchor (87)
3. `test_acceptable_health_score` - Moderate quality (71)
4. `test_poor_health_score` - Low quality (44)
5. `test_very_poor_health_score` - Critical issues (16)
6. `test_settlement_time_boundaries` - Tier boundary conditions
7. `test_cache_not_found_error` - Missing cache entry
8. `test_cache_expired_error` - Expired cache entry
9. `test_zero_values` - Edge case with zero metrics
10. `test_multiple_anchors` - Independent scoring
11. `test_edge_case_max_values` - Score capping at 100
12. `test_realistic_scenarios` - Real-world use cases

All tests use helper functions for setup and metadata creation to ensure consistency.

## Documentation

### Feature Documentation
**File:** `docs/features/ANCHOR_HEALTH_SCORE.md`

Includes:
- API reference with parameters and errors
- Detailed formula explanation
- Component score breakdowns
- Usage examples (basic, comparison, error handling)
- Score interpretation guidelines
- Real-world scenarios with calculations
- Integration with routing strategies

### Quick Reference
**File:** `docs/guides/HEALTH_SCORE_QUICK_REF.md`

Provides:
- TL;DR usage
- Score range interpretation
- Common code patterns
- Formula summary
- Example calculations

## Integration Points

### Existing Features
- **Metadata Cache:** Depends on `get_cached_metadata` for data retrieval
- **Routing Strategy:** Can be used to pre-filter anchors before routing
- **Status Monitor:** Complements real-time health monitoring

### Future Enhancements
- Could be integrated into routing options as a filter criterion
- May inform automatic anchor deactivation thresholds
- Could drive alerting/notification systems

## Design Decisions

### Why Weighted Formula?
Different metrics have different importance:
- Uptime affects all transactions (highest weight)
- Reputation reflects long-term reliability (high weight)
- Speed matters but not if anchor is unavailable (moderate weight)

### Why Tiered Speed Scoring?
- Continuous functions (e.g., inverse) are sensitive to outliers
- Tiers provide clear performance categories
- Easier to reason about and communicate
- Aligns with user expectations (5min vs 6min not meaningfully different)

### Why Not Store Score?
- Score is cheap to compute (simple arithmetic)
- Storing adds complexity (cache invalidation, consistency)
- On-demand computation ensures freshness
- Reduces storage costs

### Why These Tier Boundaries?
Based on typical transaction expectations:
- 5 min: Near-instant for financial operations
- 10 min: Acceptable for most use cases
- 30 min: Tolerable with user notification
- 60 min: Slow but sometimes necessary
- >60 min: Problematic for user experience

## Acceptance Criteria Status

✅ **Score formula documented**
- Formula explained in code comments
- Detailed breakdown in feature documentation
- Quick reference guide created

✅ **Returns CacheNotFound when no metadata cached**
- Delegates to `get_cached_metadata` which handles this
- Test case `test_cache_not_found_error` verifies behavior

✅ **Test with various metric combinations**
- 12 comprehensive test cases covering:
  - Perfect, good, acceptable, poor, and critical scores
  - Boundary conditions for all tiers
  - Zero values and maximum values
  - Multiple anchors
  - Realistic scenarios
  - Error conditions

## Files Modified

1. `src/contract.rs` - Added `get_anchor_health_score` function
2. `src/lib.rs` - Added test module declaration
3. `src/anchor_health_score_tests.rs` - New test file (12 tests)
4. `docs/features/ANCHOR_HEALTH_SCORE.md` - New feature documentation
5. `docs/guides/HEALTH_SCORE_QUICK_REF.md` - New quick reference
6. `docs/internal/HEALTH_SCORE_IMPLEMENTATION.md` - This file

## Verification Steps

To verify the implementation:

```bash
# Run all health score tests
cargo test anchor_health_score --lib

# Run specific test
cargo test test_perfect_health_score --lib

# Check documentation builds
cargo doc --no-deps --open
```

## Example Usage

```rust
use soroban_sdk::{Address, Env};

// Cache metadata first
let metadata = AnchorMetadata {
    anchor: anchor_addr.clone(),
    reputation_score: 9000,
    liquidity_score: 8000,
    uptime_percentage: 9500,
    total_volume: 1_000_000,
    average_settlement_time: 400, // 6.67 minutes
    is_active: true,
};
contract.cache_metadata(&anchor_addr, &metadata, &3600);

// Get health score
let score = contract.get_anchor_health_score(&env, &anchor_addr);
// Returns: (40 × 95 + 35 × 90 + 25 × 80) / 100 = 89

// Use score for decision making
if score >= 80 {
    // Proceed with transaction
}
```

## Performance Characteristics

- **Time Complexity:** O(1) - simple arithmetic operations
- **Storage Reads:** 1 (metadata cache lookup)
- **Storage Writes:** 0 (read-only operation)
- **Gas Cost:** Minimal - dominated by cache read

## Backward Compatibility

- No breaking changes to existing APIs
- New function is additive only
- Existing metadata cache behavior unchanged
- No migration required

## Future Considerations

1. **Configurable Weights:** Allow admin to adjust metric weights
2. **Historical Scoring:** Track score changes over time
3. **Composite Routing:** Integrate directly into routing strategy
4. **Alert Thresholds:** Trigger events when score drops below threshold
5. **Score Caching:** Cache computed scores if performance becomes an issue
6. **Additional Metrics:** Incorporate liquidity_score or other factors

## Related Issues

- Addresses feature request for simplified health assessment
- Complements existing routing and monitoring features
- Enables more sophisticated anchor selection logic
