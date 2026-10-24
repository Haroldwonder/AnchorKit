# Migration Guide

---

> **Note:** Version 1.0 has not yet been released. This guide currently covers only the stable 0.x releases. As of the current version (0.1.0), all features described in future 1.0 planning below have already been implemented. Please refer to the [0.0.1 → 0.1.0](#migration-guide-001--010) section for breaking changes in the current release.

---

# Migration Guide: 0.0.1 → 0.1.0

This guide covers every breaking change introduced in 0.1.0 and shows exactly what you need to update in your integration.

---

## Table of Contents

1. [Breaking Changes](#breaking-changes)
   - [initialize()](#1-initialize)
   - [register_attestor()](#2-register_attestor)
   - [configure_services()](#3-configure_services)
   - [supports_service()](#4-supports_service)
   - [StellarToml struct](#5-stellartoml-struct)
   - [Admin transfer flow](#6-admin-transfer-flow)
   - [Error type renamed to AnchorKitError](#7-error-type-renamed-to-anchorkiterror)
2. [New Required Configuration](#new-required-configuration)
3. [New Methods Reference](#new-methods-reference)
4. [New Error Codes](#new-error-codes)
5. [New Types](#new-types)

---

## Breaking Changes

### 1. `initialize()`

The function now accepts three additional parameters: `max_audit_log_size`, `replay_window_seconds`, and `clock_skew_seconds`.

**Before (0.0.1)**
```rust
client.initialize(&admin);
```

**After (0.1.0)**
```rust
// Pass a max_audit_log_size and None for default replay window (300 seconds)
client.initialize(&admin, &1000u64, &None, &None);

// Or set the replay window explicitly and use the default clock skew (60 seconds):
client.initialize(&admin, &1000u64, &Some(600u64), &None); // 10-minute window
```

The `max_audit_log_size` parameter caps the number of audit entries retained on-chain. The `replay_window_seconds` parameter controls how far in the past or future an attestation timestamp may be relative to the current ledger time. Attestations outside `[now - window, now + window]` are rejected with `InvalidTimestamp`. Passing `None` for the replay window defaults to **300 seconds**. The `clock_skew_seconds` parameter controls SEP-10 token expiry tolerance; passing `None` defaults to **60 seconds**.

---

### 2. `register_attestor()`

SEP-10 JWT verification is now mandatory when registering an attestor. You must obtain a valid SEP-10 JWT for the attestor and supply the issuer address that holds the corresponding verifying key.

**Before (0.0.1)**
```rust
client.register_attestor(&attestor);
```

**After (0.1.0)**
```rust
// 1. Admin must first register the SEP-10 verifying key for the issuer.
client.set_sep10_jwt_verifying_key(&issuer, &verifying_key_bytes);

// 2. Obtain a SEP-10 JWT for the attestor (off-chain, via your SEP-10 server).
let sep10_token = String::from_str(&env, "<jwt-token>");

// 3. Register the attestor with the token and issuer.
client.register_attestor(&attestor, &sep10_token, &issuer);
```

If the token is missing, expired, or does not match the attestor address, the call panics with `InvalidSep10Token` (code 18).

See [docs/features/SEP10_AUTH.md](./features/SEP10_AUTH.md) for full SEP-10 setup instructions.

---

### 3. `configure_services()`

The `services` parameter changed from `Vec<ServiceType>` to `Vec<u32>`. Use the exported service constants instead of the enum variants.

**Before (0.0.1)**
```rust
let mut services = Vec::new(&env);
services.push_back(ServiceType::Deposits);
services.push_back(ServiceType::Withdrawals);
services.push_back(ServiceType::KYC);
client.configure_services(&anchor, &services);
```

**After (0.1.0)**
```rust
use anchorkit::contract::{SERVICE_DEPOSITS, SERVICE_WITHDRAWALS, SERVICE_KYC};

let mut services: Vec<u32> = Vec::new(&env);
services.push_back(SERVICE_DEPOSITS);   // 1
services.push_back(SERVICE_WITHDRAWALS); // 2
services.push_back(SERVICE_KYC);        // 4
client.configure_services(&anchor, &services);
```

Available constants and their values:

| Constant             | Value |
|----------------------|-------|
| `SERVICE_DEPOSITS`   | `1`   |
| `SERVICE_WITHDRAWALS`| `2`   |
| `SERVICE_QUOTES`     | `3`   |
| `SERVICE_KYC`        | `4`   |

The `ServiceType` enum still exists as a helper with an `as_u32()` method if you prefer the typed form:

```rust
services.push_back(ServiceType::Deposits.as_u32());
```

---

### 4. `supports_service()`

Same change as `configure_services()` — the `service` parameter is now `u32`.

**Before (0.0.1)**
```rust
if client.supports_service(&anchor, &ServiceType::Deposits) { ... }
```

**After (0.1.0)**
```rust
use anchorkit::contract::SERVICE_DEPOSITS;

if client.supports_service(&anchor, &SERVICE_DEPOSITS) { ... }
// or:
if client.supports_service(&anchor, &ServiceType::Deposits.as_u32()) { ... }
```

---

### 5. `StellarToml` struct

Three fields changed:

| Field                        | 0.0.1          | 0.1.0                    |
|------------------------------|----------------|--------------------------|
| `signing_key`                | `String`       | `Option<String>`         |
| `fiat_currencies`            | *(not present)*| `Vec<FiatCurrency>` (new)|
| `claimable_balance_supported`| *(not present)*| `bool` (new)             |

**Before (0.0.1)**
```rust
let toml = StellarToml {
    version: String::from_str(&env, "2.0.0"),
    network_passphrase: String::from_str(&env, "Test SDF Network ; September 2015"),
    accounts: Vec::new(&env),
    signing_key: String::from_str(&env, "GABCDE..."),
    currencies: assets,
    transfer_server: String::from_str(&env, "https://anchor.example.com"),
    transfer_server_sep0024: String::from_str(&env, "https://anchor.example.com/sep24"),
    kyc_server: String::from_str(&env, "https://anchor.example.com/kyc"),
    web_auth_endpoint: String::from_str(&env, "https://anchor.example.com/auth"),
};
```

**After (0.1.0)**
```rust
let toml = StellarToml {
    version: String::from_str(&env, "2.0.0"),
    network_passphrase: String::from_str(&env, "Test SDF Network ; September 2015"),
    accounts: Vec::new(&env),
    signing_key: Some(String::from_str(&env, "GABCDE...")), // now Option<String>
    currencies: assets,
    fiat_currencies: Vec::new(&env),                        // new required field
    transfer_server: String::from_str(&env, "https://anchor.example.com"),
    transfer_server_sep0024: String::from_str(&env, "https://anchor.example.com/sep24"),
    kyc_server: String::from_str(&env, "https://anchor.example.com/kyc"),
    web_auth_endpoint: String::from_str(&env, "https://anchor.example.com/auth"),
    claimable_balance_supported: false,                     // new required field
};
```

Use `None` for `signing_key` when the anchor does not publish a signing key. Use an empty `Vec` for `fiat_currencies` if the anchor does not support fiat currencies. Set `claimable_balance_supported` to `true` if the anchor accepts claimable balances as a deposit destination (sourced from `CLAIMABLE_BALANCE_SUPPORTED` in stellar.toml).

---

### 6. Admin transfer flow

Direct admin replacement is gone. Admin transfers now use a two-step propose/accept flow to prevent accidental lockout.

**Before (0.0.1)**
```rust
// Single-step (no longer available)
client.transfer_admin(&new_admin);
```

**After (0.1.0)**
```rust
// Step 1: current admin proposes the new admin.
client.propose_admin(&new_admin);

// Step 2: new admin accepts (must be called by new_admin).
client.accept_admin(); // invoked by new_admin
```

Until `accept_admin()` is called, the current admin remains in control. The pending admin address is stored in instance storage and can be overwritten by calling `propose_admin()` again.

---

### 7. Error type renamed to `AnchorKitError`

The top-level error type was renamed from `Error` to `AnchorKitError`. A backward-compatible type alias keeps existing code compiling without changes, but new code should use `AnchorKitError` directly.

**What changed in `src/errors.rs`**

| Before (0.0.1) | After (0.1.0) |
|----------------|---------------|
| `pub enum Error { ... }` | `pub struct AnchorKitError { code: ErrorCode, message: String, context: Option<String> }` |
| Variants were the error kind (e.g. `Error::AlreadyInitialized`) | Error kind is now a separate `ErrorCode` enum; use constructor helpers instead |
| No stable numeric codes | Every kind has a stable `u32` code via `ErrorCode` |

**Backward-compatible alias (no action required)**

```rust
// src/errors.rs — still present in 0.1.0
pub type Error = AnchorKitError;
```

Code that imports or names `Error` continues to compile unchanged. The alias is intentionally kept for the foreseeable future.

**Recommended migration for new code**

Replace bare `Error` with `AnchorKitError` and switch from enum-variant matching to `ErrorCode` matching:

```rust
// Before (0.0.1)
use anchorkit::errors::Error;

fn handle(err: Error) {
    match err {
        Error::AlreadyInitialized => { /* ... */ }
        Error::AttestorNotRegistered => { /* ... */ }
        _ => { /* ... */ }
    }
}
```

```rust
// After (0.1.0)
use anchorkit::errors::{AnchorKitError, ErrorCode};

fn handle(err: AnchorKitError) {
    match err.code {
        ErrorCode::AlreadyInitialized => { /* ... */ }
        ErrorCode::AttestorNotRegistered => { /* ... */ }
        _ => { /* ... */ }
    }
}
```

**Creating errors**

Use the named constructor helpers instead of constructing the struct directly:

```rust
// Preferred — named constructors
let err = AnchorKitError::already_initialized();
let err = AnchorKitError::attestor_not_registered();
let err = AnchorKitError::validation_error("missing field: status");

// Also available — build from a code
let err = AnchorKitError::from_code(ErrorCode::RateLimitExceeded);

// With custom message and context
let err = AnchorKitError::with_context(
    ErrorCode::ValidationError,
    "Schema mismatch",
    "field: transaction_id",
);
```

**no-std / WASM builds**

In WASM (no `std` feature) `AnchorKitError` stores a `&'static str` instead of a heap-allocated `String`. The public API is identical; only the field types differ. The type alias `Error = AnchorKitError` is present in both build modes.

---

## New Required Configuration

If you use **SEP-10 authentication** (required for `register_attestor`), you must configure at least one verifying key before registering any attestors:

```rust
// Register the Ed25519 public key (32 bytes) used to verify SEP-10 JWTs.
client.set_sep10_jwt_verifying_key(&issuer_address, &public_key_bytes);

// Optionally add additional keys (key rotation support):
client.add_sep10_verifying_key(&issuer_address, &new_public_key_bytes);

// Remove a rotated-out key:
client.remove_sep10_verifying_key(&issuer_address, &old_public_key_bytes);
```

If you use **health-based auto-deactivation**, set the failure threshold after initialization:

```rust
// Anchors with failure_count >= threshold are automatically deactivated.
client.set_health_failure_threshold(&5u32);
```

---

## New Methods Reference

All new methods are additive and do not affect existing integrations unless you opt in.

### Session management
| Method | Description |
|--------|-------------|
| `create_session(initiator)` | Opens a new session; returns `session_id: u64` |
| `get_session(session_id)` | Returns the `Session` struct |
| `get_session_operation_count(session_id)` | Number of operations logged in the session |
| `get_audit_log(log_id)` | Returns a single `AuditLog` entry |
| `get_audit_log_range(from_id, to_id)` | Returns up to 100 `AuditLog` entries |

### Session-aware operations
| Method | Description |
|--------|-------------|
| `submit_attestation_with_session(session_id, ...)` | Submit attestation and write audit log |
| `register_attestor_with_session(session_id, attestor)` | Register attestor and write audit log |
| `revoke_attestor_with_session(session_id, attestor)` | Revoke attestor and write audit log |

### Request ID and tracing
| Method | Description |
|--------|-------------|
| `generate_request_id()` | Returns a `RequestId` (16-byte UUID derived from ledger state) |
| `submit_with_request_id(request_id, ...)` | Submit attestation with tracing span |
| `get_tracing_span(request_id_bytes)` | Retrieve the `TracingSpan` for a request |

### Payload hash utilities
| Method | Description |
|--------|-------------|
| `compute_payload_hash(subject, timestamp, data)` | Compute deterministic hash (on-chain) |
| `compute_payload_hash_public(subject, timestamp, data)` | Same as above, exposed for off-chain matching |

### Quotes and routing
| Method | Description |
|--------|-------------|
| `submit_quote(anchor, ...)` | Anchor submits an exchange rate quote |
| `receive_quote(receiver, anchor, quote_id)` | Retrieve and acknowledge a quote |
| `get_quote(anchor, quote_id)` | Fetch a quote by ID |
| `route_transaction(options)` | Select best anchor using a routing strategy |
| `set_anchor_metadata(anchor, ...)` | Set routing metadata for an anchor |
| `get_routing_anchors()` | List all anchors registered for routing |

### Metadata and capabilities cache
| Method | Description |
|--------|-------------|
| `cache_metadata(anchor, metadata, ttl_seconds)` | Cache anchor metadata with TTL |
| `get_cached_metadata(anchor)` | Retrieve cached metadata (errors if expired) |
| `refresh_metadata_cache(anchor)` | Invalidate cached metadata |
| `list_cached_anchors()` | List anchors with active metadata cache entries |
| `cache_capabilities(anchor, toml_url, capabilities, ttl_seconds)` | Cache capabilities string |
| `get_cached_capabilities(anchor)` | Retrieve cached capabilities |
| `refresh_capabilities_cache(anchor)` | Invalidate cached capabilities |

### Anchor info discovery
| Method | Description |
|--------|-------------|
| `fetch_anchor_info(anchor, toml_data, ttl_override)` | Store parsed stellar.toml for an anchor |
| `get_anchor_toml(anchor)` | Retrieve cached `StellarToml` |
| `refresh_anchor_info(anchor, force)` | Invalidate cached TOML |
| `get_anchor_assets(anchor)` | List asset codes from cached TOML |
| `get_anchor_currencies(anchor)` | List fiat currencies from cached TOML |
| `get_anchor_asset_info(anchor, asset_code)` | Full `AssetInfo` for one asset |
| `get_anchor_deposit_limits(anchor, asset_code)` | `(min, max)` deposit amounts |
| `get_anchor_withdrawal_limits(anchor, asset_code)` | `(min, max)` withdrawal amounts |
| `get_anchor_deposit_fees(anchor, asset_code)` | `(fixed_fee, percent_fee)` for deposits |
| `get_anchor_withdrawal_fees(anchor, asset_code)` | `(fixed_fee, percent_fee)` for withdrawals |
| `anchor_supports_deposits(anchor, asset_code)` | `bool` — deposit enabled for asset |
| `anchor_supports_withdrawals(anchor, asset_code)` | `bool` — withdrawal enabled for asset |

### Health monitoring
| Method | Description |
|--------|-------------|
| `update_health_status(anchor, latency_ms, failure_count, availability_percent)` | Record health metrics |
| `get_health_status(anchor)` | Returns `Option<HealthStatus>` |
| `set_health_failure_threshold(threshold)` | Auto-deactivate anchors above this failure count |

### Attestation queries
| Method | Description |
|--------|-------------|
| `list_attestations(subject, offset, limit)` | Paginated attestations for a subject (max 50 per call) |
| `get_attestation_count()` | Total attestations ever submitted |
| `is_initialized()` | Returns `bool`; safe to call before initialization |

---

## New Error Codes

| Code | Value | Meaning |
|------|-------|---------|
| `InvalidSep10Token` | 18 | SEP-10 JWT is missing, expired, or does not match the attestor |
| `NoQuotesAvailable` | 13 | No valid quotes found for the routing request |
| `ServicesNotConfigured` | 14 | Anchor has not configured any services |
| `ValidationError` | 15 | Generic schema or input validation failure |
| `RateLimitExceeded` | 16 | Request rate limit exceeded |
| `CacheExpired` | 48 | Cache entry exists but its TTL has elapsed |
| `CacheNotFound` | 49 | No cache entry found for the given key |
| `StorageCorrupted` | 19 | On-chain storage entry is unreadable |
| `AuditLogMaxSizeInvalid` | 51 | `max_audit_log_size` was set to zero |

---

## New Types

| Type | Description |
|------|-------------|
| `Session` | Session metadata: `session_id`, `initiator`, `created_at`, `nonce`, `operation_count` |
| `OperationContext` | Per-operation record: type, timestamp, status, result summary |
| `AuditLog` | Immutable audit entry linking a session, actor, and `OperationContext` |
| `RequestId` | 16-byte UUID with `created_at` timestamp |
| `TracingSpan` | Tracing record for a single request: operation, actor, start/end time, status |
| `Quote` | Exchange rate quote: assets, rate, fee, min/max amounts, expiry |
| `AnchorServices` | Anchor address + `Vec<u32>` of enabled service constants |
| `RoutingAnchorMeta` | Routing metadata: reputation, settlement time, liquidity, uptime, volume |
| `RoutingRequest` | Routing input: base/quote asset, amount, operation type |
| `RoutingOptions` | Full routing call: `RoutingRequest` + strategy + filters |
| `AnchorMetadata` | Cached anchor performance metrics |
| `MetadataCache` | `AnchorMetadata` + `cached_at` + `ttl_seconds` |
| `CapabilitiesCache` | Cached capabilities string + TOML URL + TTL |
| `AssetInfo` | Full asset record: fees, limits, deposit/withdrawal enabled flags |
| `FiatCurrency` | Fiat currency entry: ISO code, name, deposit/withdrawal enabled |
| `StellarToml` | Parsed stellar.toml representation (see [breaking change #5](#5-stellartoml-struct)) |
| `CachedToml` | `StellarToml` + `cached_at` + `ttl_seconds` |
| `HealthStatus` | Anchor health: latency, failure count, availability percentage |
