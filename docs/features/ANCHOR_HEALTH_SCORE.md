# Anchor Health Score

The anchor health score feature provides a simplified, single-metric assessment of anchor reliability by computing a weighted score from cached metadata.

## Overview

Instead of manually evaluating `uptime_percentage`, `reputation_score`, and `average_settlement_time` separately, callers can use `get_anchor_health_score` to obtain a 0-100 health score that combines all three metrics using a documented formula.

## API

### get_anchor_health_score

```rust
pub fn get_anchor_health_score(env: Env, anchor: Address) -> u32
```

Returns a health score (0-100) for the specified anchor based on its cached metadata.

**Parameters:**
- `env` - Soroban environment
- `anchor` - Address of the anchor to evaluate

**Returns:**
- `u32` - Health score from 0 (worst) to 100 (best)

**Errors:**
- `CacheNotFound` (49) - No metadata cached for this anchor
- `CacheExpired` (48) - Cached metadata has expired

## Score Formula

The health score is computed as a weighted combination of three metrics:

```
health_score = (uptime_weight × uptime_score) 
             + (reputation_weight × reputation_score) 
             + (speed_weight × speed_score)
```

### Weights

| Metric | Weight | Rationale |
|--------|--------|-----------|
| Uptime | 40% | Most critical factor - anchor must be available |
| Reputation | 35% | Trust and track record are essential |
| Settlement Speed | 25% | Important but less critical than availability |

### Component Scores

#### 1. Uptime Score
Directly scaled from the cached `uptime_percentage` (0-10000 scale):
```
uptime_score = uptime_percentage / 100
```
- 10000 (100%) → 100 points
- 9500 (95%) → 95 points
- 5000 (50%) → 50 points

#### 2. Reputation Score
Directly scaled from the cached `reputation_score` (0-10000 scale):
```
reputation_score = reputation_score / 100
```
- 10000 (100%) → 100 points
- 8000 (80%) → 80 points
- 3000 (30%) → 30 points

#### 3. Settlement Speed Score
Tiered scoring based on `average_settlement_time` (seconds):

| Settlement Time | Score | Grade |
|----------------|-------|-------|
| 0-300s (≤5 min) | 100 | Excellent |
| 301-600s (5-10 min) | 80 | Good |
| 601-1800s (10-30 min) | 60 | Acceptable |
| 1801-3600s (30-60 min) | 40 | Slow |
| >3600s (>1 hour) | 20 | Very Slow |

## Usage Examples

### Basic Usage

```rust
use soroban_sdk::{Address, Env};

// Get health score for an anchor
let score = client.get_anchor_health_score(&anchor_address);

if score >= 80 {
    // High-quality anchor
} else if score >= 60 {
    // Acceptable anchor
} else {
    // Consider alternative anchors
}
```

### Comparing Multiple Anchors

```rust
let anchor1_score = client.get_anchor_health_score(&anchor1);
let anchor2_score = client.get_anchor_health_score(&anchor2);
let anchor3_score = client.get_anchor_health_score(&anchor3);

// Select the healthiest anchor
let best_anchor = if anchor1_score >= anchor2_score && anchor1_score >= anchor3_score {
    anchor1
} else if anchor2_score >= anchor3_score {
    anchor2
} else {
    anchor3
};
```

### Error Handling

```rust
use crate::errors::ErrorCode;

match client.try_get_anchor_health_score(&anchor) {
    Ok(score) => {
        // Use the score
    },
    Err(ErrorCode::CacheNotFound) => {
        // Metadata not cached - fetch and cache it first
        client.cache_metadata(&anchor, &metadata, &ttl);
    },
    Err(ErrorCode::CacheExpired) => {
        // Cache expired - refresh metadata
        client.refresh_metadata_cache(&anchor);
        client.cache_metadata(&anchor, &updated_metadata, &ttl);
    },
    Err(e) => {
        // Handle other errors
    }
}
```

## Score Interpretation

| Score Range | Health Level | Recommendation |
|-------------|--------------|----------------|
| 90-100 | Excellent | Preferred choice for transactions |
| 75-89 | Good | Reliable for most use cases |
| 60-74 | Acceptable | Suitable with monitoring |
| 40-59 | Poor | Use with caution |
| 0-39 | Critical | Avoid or investigate issues |

## Example Scenarios

### Scenario 1: Premium Anchor
```
Uptime: 98% (9800)
Reputation: 95% (9500)
Settlement: 3 minutes (180s)

Calculation:
- Uptime score: 98
- Reputation score: 95
- Speed score: 100 (≤300s)
- Health score: (40 × 98 + 35 × 95 + 25 × 100) / 100 = 97.45 → 97
```

### Scenario 2: New Anchor
```
Uptime: 95% (9500)
Reputation: 30% (3000) - new, unproven
Settlement: 4 minutes (240s)

Calculation:
- Uptime score: 95
- Reputation score: 30
- Speed score: 100 (≤300s)
- Health score: (40 × 95 + 35 × 30 + 25 × 100) / 100 = 71.5 → 71
```

### Scenario 3: Struggling Anchor
```
Uptime: 60% (6000)
Reputation: 50% (5000)
Settlement: 50 minutes (3000s)

Calculation:
- Uptime score: 60
- Reputation score: 50
- Speed score: 40 (1801-3600s)
- Health score: (40 × 60 + 35 × 50 + 25 × 40) / 100 = 51.5 → 51
```

## Integration with Routing

The health score can be used alongside the existing routing strategies:

```rust
// Filter anchors by minimum health score before routing
let health_score = client.get_anchor_health_score(&anchor);
if health_score >= 70 {
    // Include in routing candidates
    let quote = client.route_transaction(&options);
}
```

## Prerequisites

Before calling `get_anchor_health_score`, ensure:

1. Metadata has been cached for the anchor using `cache_metadata`
2. The cache has not expired (check TTL)
3. The anchor address is valid

## Related Features

- [Metadata Cache](METADATA_CACHE.md) - Underlying cache system
- [Routing Strategy](ROUTING_STRATEGY.md) - Anchor selection algorithms
- [Status Monitor](STATUS_MONITOR.md) - Real-time health monitoring

## Notes

- The health score is computed on-demand from cached data; it is not stored separately
- Score calculation is deterministic - same inputs always produce the same score
- The formula weights can be adjusted in future versions based on operational data
- Zero TTL in metadata cache means "never expire" and will not trigger `CacheExpired`
