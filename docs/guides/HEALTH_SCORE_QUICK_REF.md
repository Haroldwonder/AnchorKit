# Anchor Health Score - Quick Reference

## TL;DR

```rust
// Get a 0-100 health score for an anchor
let score = contract.get_anchor_health_score(&env, &anchor_address);
```

## What It Does

Combines three metrics into a single 0-100 score:
- **Uptime** (40% weight)
- **Reputation** (35% weight)  
- **Settlement Speed** (25% weight)

## Score Ranges

| Score | Meaning |
|-------|---------|
| 90-100 | Excellent - use with confidence |
| 75-89 | Good - reliable choice |
| 60-74 | Acceptable - monitor performance |
| 40-59 | Poor - use with caution |
| 0-39 | Critical - avoid |

## Common Patterns

### Select Best Anchor

```rust
let mut best_score = 0;
let mut best_anchor = None;

for anchor in anchors.iter() {
    if let Ok(score) = contract.try_get_anchor_health_score(&env, &anchor) {
        if score > best_score {
            best_score = score;
            best_anchor = Some(anchor);
        }
    }
}
```

### Filter by Minimum Score

```rust
let min_score = 70;
let qualified_anchors: Vec<Address> = anchors
    .iter()
    .filter(|anchor| {
        contract.try_get_anchor_health_score(&env, anchor)
            .map(|score| score >= min_score)
            .unwrap_or(false)
    })
    .collect();
```

### Handle Cache Errors

```rust
match contract.try_get_anchor_health_score(&env, &anchor) {
    Ok(score) => {
        // Use score
    },
    Err(ErrorCode::CacheNotFound) => {
        // Fetch and cache metadata first
    },
    Err(ErrorCode::CacheExpired) => {
        // Refresh cache
    },
    Err(e) => {
        // Handle other errors
    }
}
```

## Formula Details

```
score = (40 × uptime/100) + (35 × reputation/100) + (25 × speed_score)
```

### Speed Score Tiers

| Settlement Time | Points |
|----------------|--------|
| ≤5 min | 100 |
| 5-10 min | 80 |
| 10-30 min | 60 |
| 30-60 min | 40 |
| >60 min | 20 |

## Prerequisites

1. Metadata must be cached:
   ```rust
   contract.cache_metadata(&anchor, &metadata, &ttl_seconds);
   ```

2. Cache must not be expired (check TTL)

## Errors

- `CacheNotFound` (49) - No metadata cached
- `CacheExpired` (48) - Cache TTL elapsed

## Examples

### Perfect Score (100)
```
Uptime: 100% (10000)
Reputation: 100% (10000)
Settlement: 5 min (300s)
→ Score: 100
```

### Good Score (87)
```
Uptime: 95% (9500)
Reputation: 85% (8500)
Settlement: 8 min (480s)
→ Score: 87
```

### Poor Score (44)
```
Uptime: 50% (5000)
Reputation: 40% (4000)
Settlement: 45 min (2700s)
→ Score: 44
```

## See Also

- [Full Documentation](../features/ANCHOR_HEALTH_SCORE.md)
- [Metadata Cache](../features/METADATA_CACHE.md)
- [Routing Strategy](../features/ROUTING_STRATEGY.md)
