# Routing Strategy Guide

`route_transaction` selects the best anchor from all registered, active anchors
that can fulfil a transaction. The selection algorithm is controlled by the
`strategy` field of `RoutingOptions`.

## Valid Strategy Symbols

Pass exactly one of the following symbols as the first (and only) element of
the `strategy` vec:

| Symbol | Selection criterion |
|---|---|
| `"LowestFee"` | Anchor with the lowest `fee_percentage` on its current quote |
| `"FastestSettlement"` | Anchor with the lowest `average_settlement_time` (seconds) |
| `"HighestReputation"` | Anchor with the highest `reputation_score` (0–100) |
| `"Balanced"` | Highest composite score weighting fee (40%), speed (30%), and reputation (30%) |

## Balanced Strategy

`"Balanced"` scores each candidate anchor using integer fixed-point arithmetic:

```
score = (40_000 / fee_percentage)
      + (30_000 / average_settlement_time)
      + (reputation_score * 30 / 10_000)
```

All three terms are dimensionless and comparable. A zero `fee_percentage` or
`average_settlement_time` contributes `0` to that term (no division by zero).
The anchor with the highest total score is selected.

Example with three anchors:

| Anchor | fee | time (s) | reputation | fee term | time term | rep term | score |
|--------|-----|----------|------------|----------|-----------|----------|-------|
| A | 10 | 1000 | 2000 | 4000 | 30 | 6 | **4036** |
| C | 20 | 200 | 6000 | 2000 | 150 | 18 | 2168 |
| B | 50 | 100 | 9000 | 800 | 300 | 27 | 1127 |

Anchor A wins despite slow speed and low reputation because its very low fee
dominates the score.

## Default Strategy

`strategy` is **required**. Passing an empty vec causes the call to panic with
`NoQuotesAvailable`.

An unrecognised symbol string does not error — it falls through all strategy
branches and returns the **first candidate in storage iteration order**, which
is non-deterministic. Always use one of the documented symbols above.

## Candidate Filtering

Before strategy selection, anchors are excluded if any of the following apply:

- `is_active` is `false`
- `reputation_score` < `options.min_reputation` — set `min_reputation = 0` (the
  default) to include all active anchors regardless of reputation score
- Latest quote has expired (`valid_until <= now`)
- `request.amount` is outside the quote's `[minimum_amount, maximum_amount]` range

If no candidates remain after filtering, the call panics with `NoQuotesAvailable`.

## Usage Example

```rust
let mut strategy = Vec::new(&env);
strategy.push_back(Symbol::new(&env, "LowestFee"));

let options = RoutingOptions {
    request: RoutingRequest {
        base_asset: String::from_str(&env, "USDC"),
        quote_asset: String::from_str(&env, "BRL"),
        amount: 1_000_000,
        operation_type: 1,
    },
    strategy,
    min_reputation: 80,
    max_anchors: 0,   // reserved, not yet enforced
    require_kyc: false, // reserved, not yet enforced
};

let best_quote = contract.route_transaction(&options);
```

## Notes

- `max_anchors` and `require_kyc` fields are reserved for future use and are
  not enforced by the current implementation.
- Reputation scores are set via `register_routing_anchor` /
  `update_routing_anchor_meta` and reflect operator-assigned trust levels.
