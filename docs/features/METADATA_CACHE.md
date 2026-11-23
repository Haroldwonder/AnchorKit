# Metadata Caching

AnchorKit provides TTL-based on-chain caching for anchor metadata and capabilities, reducing redundant off-chain lookups.

## Cache Types

### AnchorMetadata
Stores anchor performance and status data:
- `anchor` – anchor address
- `reputation_score` – u32 (0–10000)
- `liquidity_score` – u32 (0–10000)
- `uptime_percentage` – u32 (0–10000)
- `total_volume` – u64
- `average_settlement_time` – u64 (seconds)
- `is_active` – bool

### CapabilitiesCache
Stores parsed stellar.toml capability data:
- `toml_url` – URL of the stellar.toml
- `capabilities` – JSON string of capabilities
- `cached_at` – ledger timestamp when cached
- `ttl_seconds` – TTL in seconds

## API

### Metadata Cache

```rust
// Store metadata with TTL (admin only)
contract.cache_metadata(&anchor, &metadata, &ttl_seconds);

// Retrieve metadata (errors if expired or not found)
let meta = contract.get_cached_metadata(&anchor);

// Invalidate cache entry (admin only)
contract.refresh_metadata_cache(&anchor);
```

### Capabilities Cache

```rust
// Store capabilities with TTL (admin only)
contract.cache_capabilities(&anchor, &toml_url, &capabilities_json, &ttl_seconds);

// Retrieve capabilities (errors if expired or not found)
let caps = contract.get_cached_capabilities(&anchor);

// Invalidate cache entry (admin only)
contract.refresh_capabilities_cache(&anchor);
```

## TTL Behaviour

- Cache entries are stored in **temporary storage** keyed by `["METACACHE", anchor]` or `["CAPCACHE", anchor]`.
- `get_cached_metadata` / `get_cached_capabilities` check `cached_at + ttl_seconds <= now` and panic with `CacheExpired` (48) if true.
- If no entry exists, panics with `CacheNotFound` (49).
- `refresh_*_cache` removes the entry immediately regardless of TTL.

## Error Codes

| Code | Name | Meaning |
|---|---|---|
| 48 | `CacheExpired` | Entry exists but TTL has elapsed |
| 49 | `CacheNotFound` | No entry for this anchor |
