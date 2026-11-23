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
| `"HighestReputation"` | Anchor with the highest `reputation_score` (0–10,000) |
| `"Balanced"` | Highest composite score weighting fee (40%), speed (30%), and reputation (30%) |
| `"Weighted"` | Probabilistic selection proportional to each anchor's health score |

## Candidate Filtering

Before any strategy runs, the full anchor list is narrowed down. An anchor is
excluded if **any** of the following apply:

- `is_active` is `false` (includes anchors auto-deactivated after health failures)
- `reputation_score` < `options.min_reputation` (set `0` to skip this filter)
- Latest quote has expired (`valid_until <= now`)
- `request.amount` is outside `[minimum_amount, maximum_amount]` of the quote
- `options.jurisdiction` is `Some(code)` and the anchor's stored jurisdiction
  does not exactly match that code, or the anchor has no jurisdiction set

If `options.max_anchors > 0`, iteration stops once that many valid candidates
have been collected. The collection order mirrors the order anchors were
registered (insertion order of the persistent storage list).

If no candidates remain after filtering, the fallback chain is tried (see
[Fallback Chain](#fallback-chain)). If that is also empty, the call panics with
`NoQuotesAvailable`.

**Pseudocode:**

```
function collect_candidates(anchors, options, now):
    candidates = []
    for anchor in anchors:                          // insertion-order iteration
        meta = load_metadata(anchor)
        if meta is None: continue
        if not meta.is_active: continue
        if meta.reputation_score < options.min_reputation: continue

        if options.jurisdiction is Some(code):
            if anchor.jurisdiction != Some(code): continue  // None also excluded

        if options.require_kyc:
            if SERVICE_KYC not in anchor.services: continue

        quote = latest_quote(anchor)
        if quote is None: continue
        if quote.valid_until <= now:
            emit QuoteExpiredEvent(anchor, quote_id)
            continue
        if options.request.amount < quote.minimum_amount: continue
        if options.request.amount > quote.maximum_amount: continue

        candidates.append(quote)
        if options.max_anchors > 0 and len(candidates) >= options.max_anchors:
            break

    return candidates
```

---

## Strategy: LowestFee

Selects the candidate with the lowest `fee_percentage`. Fees are stored as
integer basis points (e.g. `50` = 0.50%).

**Algorithm:**

```
function select_lowest_fee(candidates):
    best = candidates[0]                    // first candidate is the initial winner
    for q in candidates:
        if q.fee_percentage < best.fee_percentage:
            best = q
    return best
```

**Tie-breaking:** When two candidates share the same `fee_percentage`, the one
that appears earlier in the storage list (lower insertion index) is kept,
because the strict `<` comparison never replaces the current best on a tie.

**Example:**

Three anchors registered in order A → B → C with fees 30, 20, 20:

| Anchor | fee_percentage | Iteration result |
|--------|---------------|-----------------|
| A | 30 | becomes initial `best` |
| B | 20 | `20 < 30` → B becomes `best` |
| C | 20 | `20 < 20` is false → B remains `best` |

Winner: **B** (first anchor to reach fee 20).

**Example configuration:**

```rust
let mut strategy = Vec::new(&env);
strategy.push_back(Symbol::new(&env, "LowestFee"));

let options = RoutingOptions {
    request: RoutingRequest {
        base_asset: String::from_str(&env, "USDC"),
        quote_asset: String::from_str(&env, "BRL"),
        amount: 500_000,
        operation_type: 1,
    },
    strategy,
    min_reputation: 5000,   // only anchors with score >= 5000
    max_anchors: 0,         // no limit on candidates
    require_kyc: false,
    jurisdiction: None,     // all regions included
    fallback_chain: Vec::new(&env),
};

let best_quote = contract.route_transaction(&options);
// best_quote.fee_percentage is the lowest fee among qualifying anchors
```

---

## Strategy: FastestSettlement

Selects the candidate whose anchor has the lowest `average_settlement_time`
(in seconds). Settlement time comes from `AnchorMetadata`, not from the quote
itself. Anchors without stored metadata are assigned `u64::MAX` and thus rank
last.

**Algorithm:**

```
function select_fastest_settlement(candidates, storage):
    best = candidates[0]
    best_time = metadata(best.anchor).average_settlement_time  // u64::MAX if missing
    for q in candidates:
        t = metadata(q.anchor).average_settlement_time         // u64::MAX if missing
        if t < best_time:
            best_time = t
            best = q
    return best
```

**Tie-breaking:** Same as LowestFee — strict `<` means the earlier-registered
anchor wins on equal settlement times.

**Example:**

Two anchors with the same fee but different speeds:

| Anchor | fee | avg_settlement_time (s) | Result |
|--------|-----|------------------------|--------|
| A | 25 | 600 | initial best |
| B | 25 | 200 | `200 < 600` → B wins |

Winner: **B**.

**Example configuration:**

```rust
let mut strategy = Vec::new(&env);
strategy.push_back(Symbol::new(&env, "FastestSettlement"));

let options = RoutingOptions {
    request: RoutingRequest { /* ... */ },
    strategy,
    min_reputation: 0,
    max_anchors: 5,         // evaluate up to 5 candidates
    require_kyc: false,
    jurisdiction: Some(String::from_str(&env, "USA")),
    fallback_chain: Vec::new(&env),
};
```

---

## Strategy: HighestReputation

Selects the candidate whose anchor has the highest `reputation_score`
(0–10,000). Reputation is set by operators via `register_routing_anchor` /
`update_routing_anchor_meta`. Anchors without stored metadata receive score `0`.

**Algorithm:**

```
function select_highest_reputation(candidates, storage):
    best = candidates[0]
    best_rep = metadata(best.anchor).reputation_score   // 0 if missing
    for q in candidates:
        rep = metadata(q.anchor).reputation_score       // 0 if missing
        if rep > best_rep:
            best_rep = rep
            best = q
    return best
```

**Tie-breaking:** Strict `>` — the earlier-registered anchor wins on equal
reputation scores.

**Example:**

Three anchors with reputations 5000, 9000, 9000 (registered A → B → C):

| Anchor | reputation_score | Result |
|--------|-----------------|--------|
| A | 5000 | initial best |
| B | 9000 | `9000 > 5000` → B wins |
| C | 9000 | `9000 > 9000` is false → B remains |

Winner: **B** (first to reach score 9000).

**Example configuration:**

```rust
let mut strategy = Vec::new(&env);
strategy.push_back(Symbol::new(&env, "HighestReputation"));

let options = RoutingOptions {
    request: RoutingRequest { /* ... */ },
    strategy,
    min_reputation: 7000,   // pre-filter: only anchors with score >= 7000
    max_anchors: 0,
    require_kyc: true,      // anchor must advertise KYC service
    jurisdiction: None,
    fallback_chain: Vec::new(&env),
};
```

---

## Strategy: Balanced

Scores each candidate anchor using a weighted composite of fee, settlement
time, and reputation. The anchor with the highest total score wins.

**Scoring formula:**

```
score = (40_000 / fee_percentage)
      + (30_000 / average_settlement_time)
      + (reputation_score × 3_000 / 10_000)
```

All arithmetic is integer (no floating point). A `fee_percentage` or
`average_settlement_time` of `0` contributes `0` to that term instead of
causing a division-by-zero error.

The approximate weight of each term in a typical scenario:
- **Fee (40%):** reward for low transaction cost
- **Speed (30%):** reward for fast settlement
- **Reputation (30%):** reward for operator-assigned trust level (scaled from
  0–10,000 into 0–3,000 to match the magnitude of the other terms)

**Algorithm:**

```
function balanced_score(q, storage):
    meta = metadata(q.anchor)
    fee_term  = if q.fee_percentage > 0 then 40_000 / q.fee_percentage else 0
    time_term = if meta.average_settlement_time > 0
                then 30_000 / meta.average_settlement_time else 0
    rep_term  = meta.reputation_score * 3_000 / 10_000
    return fee_term + time_term + rep_term

function select_balanced(candidates, storage):
    best = candidates[0]
    best_score = balanced_score(best, storage)
    for q in candidates:
        score = balanced_score(q, storage)
        if score > best_score:
            best_score = score
            best = q
    return best
```

**Tie-breaking:** Strict `>` — earlier-registered anchor wins on equal scores.

**Detailed example (three anchors):**

| Anchor | fee | time (s) | reputation | fee term | time term | rep term | score |
|--------|-----|----------|------------|----------|-----------|----------|-------|
| A | 10 | 1000 | 2000 | 4000 | 30 | 600 | **4630** |
| C | 20 | 200 | 6000 | 2000 | 150 | 1800 | 3950 |
| B | 50 | 100 | 9000 | 800 | 300 | 2700 | 3800 |

Anchor A wins despite being slow and low-reputation because its very low fee
produces a large `fee_term` that dominates the composite score.

**Interpretation:** Balanced is best when no single metric should unconditionally
win. It degrades gracefully — an anchor with a zero fee or zero settlement time
simply contributes 0 for that term rather than being disqualified.

**Example configuration:**

```rust
let mut strategy = Vec::new(&env);
strategy.push_back(Symbol::new(&env, "Balanced"));

let options = RoutingOptions {
    request: RoutingRequest {
        base_asset: String::from_str(&env, "USDC"),
        quote_asset: String::from_str(&env, "EUR"),
        amount: 1_000_000,
        operation_type: 1,
    },
    strategy,
    min_reputation: 0,
    max_anchors: 0,
    require_kyc: false,
    jurisdiction: None,
    fallback_chain: Vec::new(&env),
};
```

---

## Strategy: Weighted

Selects an anchor **probabilistically**, with each candidate's probability
proportional to its health score. This distributes traffic across multiple
healthy anchors rather than always routing to a single winner.

**Health score formula:**

```
health_score = max(0, availability_percent - (failure_count × 10))
```

- `availability_percent` comes from the anchor's `HealthStatus` record
  (0–100). Defaults to `100` if no health data is stored.
- `failure_count` is the number of consecutive health failures recorded via
  `update_health_status`. Each failure reduces the score by 10.
- The result is clamped to a minimum of `0`.

**Selection algorithm:**

```
function select_weighted(candidates, storage, prng):
    scores = [health_score(q) for q in candidates]
    total  = sum(scores)

    if total == 0:
        // All anchors are equally degraded — pick uniformly at random
        return candidates[prng.gen_range(0, len(candidates))]

    threshold = prng.gen_range(0, total)   // exclusive upper bound
    for q, score in zip(candidates, scores):
        threshold -= score
        if threshold <= 0:
            return q

    // Rounding safety net: return last candidate
    return candidates[last]
```

The random number comes from `env.prng()` (Soroban's deterministic PRNG seeded
from the ledger), so results are deterministic for a given ledger state but
non-predictable off-chain.

**Tie-breaking / equal health:** When all anchors share the same non-zero
health score, each has an equal probability of selection (uniform random).
When all scores are zero, uniform random selection is used directly.

**Example — traffic distribution:**

Three anchors with health scores 80, 60, and 0:

| Anchor | availability | failure_count | health_score | selection probability |
|--------|-------------|---------------|--------------|----------------------|
| A | 100% | 2 | 80 | 80/140 ≈ 57% |
| B | 80% | 2 | 60 | 60/140 ≈ 43% |
| C | 50% | 5 | 0 | 0/140 = 0% |

Anchor C is effectively excluded because its health score is 0. Anchors A and
B share traffic in roughly a 4:3 ratio.

**When to use Weighted:** Production environments where you want load balancing
across multiple healthy anchors, or gradual traffic shifting during anchor
maintenance.

**Example configuration:**

```rust
let mut strategy = Vec::new(&env);
strategy.push_back(Symbol::new(&env, "Weighted"));

let options = RoutingOptions {
    request: RoutingRequest {
        base_asset: String::from_str(&env, "USDC"),
        quote_asset: String::from_str(&env, "MXN"),
        amount: 2_500_000,
        operation_type: 1,
    },
    strategy,
    min_reputation: 6000,
    max_anchors: 0,
    require_kyc: false,
    jurisdiction: Some(String::from_str(&env, "MEX")),
    fallback_chain: Vec::new(&env),
};
```

---

## Fallback Chain

When the primary candidate set is empty (after all filters), the router tries
each address in `options.fallback_chain` in order. The first fallback anchor
that passes all filters (active, reputation, KYC, non-expired quote, amount
range) is used immediately; the strategy comparison is not applied to fallback
candidates — the first valid one wins.

Fallback anchors are still required to be registered in the main anchor list.
Addresses not in the list are silently skipped.

The jurisdiction filter is **not** applied during fallback evaluation.

**Pseudocode:**

```
function try_fallback(fallback_chain, anchors, options, now):
    for fallback_anchor in fallback_chain:
        if fallback_anchor not in anchors: continue
        meta = load_metadata(fallback_anchor)
        if meta is None or not meta.is_active: continue
        if meta.reputation_score < options.min_reputation: continue
        if options.require_kyc and SERVICE_KYC not in services(fallback_anchor): continue
        quote = latest_quote(fallback_anchor)
        if quote is None: continue
        if quote.valid_until <= now:
            emit QuoteExpiredEvent(...)
            continue
        if amount outside [quote.minimum_amount, quote.maximum_amount]: continue
        return quote      // first valid fallback wins
    return None
```

**Example — using a fallback for off-hours:**

```rust
let mut fallback_chain = Vec::new(&env);
fallback_chain.push_back(backup_anchor);

let options = RoutingOptions {
    request: RoutingRequest { /* ... */ },
    strategy,
    min_reputation: 8000,
    max_anchors: 0,
    require_kyc: false,
    jurisdiction: None,
    fallback_chain,
};
```

---

## Jurisdiction Filtering

Set `options.jurisdiction` to an ISO 3166-1 alpha-3 code (e.g. `"USA"`,
`"GBR"`, `"DEU"`) to restrict routing to anchors registered in that region.

Rules:
- Matching is **exact and case-sensitive** (`"USA" ≠ "usa"`).
- An anchor with **no jurisdiction set** is **excluded** when any jurisdiction
  filter is active. Only anchors that explicitly match the requested code pass.
- Setting `jurisdiction: None` disables geographic filtering; all anchors are
  eligible regardless of their jurisdiction setting.

```
filter condition: anchor.jurisdiction == Some(requested_code)
```

**Example — jurisdiction excludes unscoped anchor:**

| Anchor | stored jurisdiction | filter = `"USA"` | filter = `None` |
|--------|--------------------|--------------------|-----------------|
| A | `Some("USA")` | ✅ eligible | ✅ eligible |
| B | `Some("DEU")` | ❌ excluded | ✅ eligible |
| C | `None` | ❌ excluded | ✅ eligible |

---

## Tie-Breaking Summary

All deterministic strategies use a **first-registered-wins** rule on ties:
candidates are iterated in anchor insertion order, and the current best is only
replaced on a strict improvement (`<` for minimisation, `>` for maximisation).
An equal value never replaces the current best.

| Strategy | Comparison | Tie winner |
|---|---|---|
| `LowestFee` | `fee_percentage <` | Earlier-registered anchor |
| `FastestSettlement` | `settlement_time <` | Earlier-registered anchor |
| `HighestReputation` | `reputation_score >` | Earlier-registered anchor |
| `Balanced` | `composite_score >` | Earlier-registered anchor |
| `Weighted` | probabilistic | Random (PRNG-determined) |

---

## Dry Run

`route_transaction_dry_run` applies the same filtering and strategy selection
as `route_transaction` but returns only the selected `Address` and emits
**no events** (`QuoteExpiredEvent` and `RoutingDecisionEvent` are suppressed).

Use it to preview routing decisions before committing, or to validate
`RoutingOptions` configuration without side effects.

```rust
// Preview — no events emitted
let predicted_anchor = contract.route_transaction_dry_run(&options);

// Commit — emits RoutingDecisionEvent (and QuoteExpiredEvent for stale quotes)
let best_quote = contract.route_transaction(&options);

assert_eq!(predicted_anchor, best_quote.anchor);
```

---

## Events

| Event topic | Emitted by | When |
|---|---|---|
| `"routing"` → `RoutingDecisionEvent` | `route_transaction` | Successful routing; contains `anchor`, `strategy`, `quote_id`, `ledger_sequence` |
| `"quote"` → `QuoteExpiredEvent` | `route_transaction` | An anchor's latest quote was expired at evaluation time; contains `anchor`, `quote_id`, `valid_until` |

Neither event is emitted by `route_transaction_dry_run`.

---

## Default Strategy and Error Cases

- `strategy` is **required**. An empty vec panics with `NoQuotesAvailable`.
- An unrecognised symbol panics with `InvalidStrategy`.
- If all candidates (including the fallback chain) are exhausted, the call
  panics with `NoQuotesAvailable`.

Always pass exactly one of the five documented symbols.

---

## RoutingOptions Reference

| Field | Type | Default | Description |
|---|---|---|---|
| `request` | `RoutingRequest` | required | Asset pair, amount, and operation type |
| `strategy` | `Vec<Symbol>` | required | Single-element vec with a strategy symbol |
| `min_reputation` | `u32` | `0` | Minimum reputation score (0 = no filter) |
| `max_anchors` | `u32` | `0` | Max candidates to collect (0 = unlimited) |
| `require_kyc` | `bool` | `false` | Reserved: when `true`, restrict to KYC-capable anchors |
| `jurisdiction` | `Option<String>` | `None` | ISO 3166-1 alpha-3 code, or `None` for no filter |
| `fallback_chain` | `Vec<Address>` | empty | Ordered fallback anchors tried when primary set is empty |

`max_anchors` caps the **candidate collection** phase, not the final selection.
Setting it to `1` means only the first anchor to pass the filters is considered,
regardless of strategy.

---

## Full Usage Example

```rust
let mut strategy = Vec::new(&env);
strategy.push_back(Symbol::new(&env, "Balanced"));

let mut fallback = Vec::new(&env);
fallback.push_back(backup_anchor);

let options = RoutingOptions {
    request: RoutingRequest {
        base_asset: String::from_str(&env, "USDC"),
        quote_asset: String::from_str(&env, "BRL"),
        amount: 1_000_000,
        operation_type: 1,
    },
    strategy,
    min_reputation: 5000,
    max_anchors: 0,
    require_kyc: false,
    jurisdiction: Some(String::from_str(&env, "BRA")),
    fallback_chain: fallback,
};

let best_quote = contract.route_transaction(&options);
// best_quote.anchor  — the selected anchor address
// best_quote.fee_percentage — the agreed fee
// best_quote.quote_id — stable reference for this quote
```

## Notes

- `max_anchors` and `require_kyc` are present in `RoutingOptions` for future
  extensibility. `max_anchors` is already enforced during candidate collection.
  `require_kyc` is enforced when `true`. Both are safe to use today.
- Reputation scores are set via `register_routing_anchor` /
  `update_routing_anchor_meta` and reflect operator-assigned trust levels.
- Health data (used by `Weighted`) is recorded via `update_health_status` and
  auto-deactivates anchors that exceed the failure threshold set by
  `set_health_failure_threshold`.
