# AnchorKit API Reference

Every public method on the `AnchorKitContract` Soroban contract. Methods are grouped by feature area. All methods are invoked via the generated `AnchorKitContractClient`.

---

## Table of Contents

1. [Contract Lifecycle](#contract-lifecycle)
2. [Admin Management](#admin-management)
3. [SEP-10 Authentication](#sep-10-authentication)
4. [Attestor Registration](#attestor-registration)
5. [Attestation Submission & Retrieval](#attestation-submission--retrieval)
6. [Service Configuration](#service-configuration)
7. [Session Management](#session-management)
8. [Quotes & Routing](#quotes--routing)
9. [Metadata & Capabilities Cache](#metadata--capabilities-cache)
10. [Anchor Info Discovery](#anchor-info-discovery)
11. [Health Monitoring](#health-monitoring)
12. [Request ID & Tracing](#request-id--tracing)
13. [Payload Hash Utilities](#payload-hash-utilities)
14. [Audit Log](#audit-log)

---

## Contract Lifecycle

### `initialize`

```rust
fn initialize(env: Env, admin: Address, max_audit_log_size: u64, replay_window_seconds: Option<u64>, clock_skew_seconds: Option<u64>)
```

Initialises the contract. Must be called once before any other method.

| Parameter | Type | Description |
|-----------|------|-------------|
| `admin` | `Address` | Initial admin address |
| `max_audit_log_size` | `u64` | Maximum number of audit log entries to retain |
| `replay_window_seconds` | `Option<u64>` | Replay-attack window in seconds; `None` defaults to 300 |
| `clock_skew_seconds` | `Option<u64>` | SEP-10 clock-skew tolerance in seconds; `None` defaults to 60 |

**Errors:** `AlreadyInitialized (1)`

**Example**
```rust
client.initialize(&admin, &1000u64, &None, &None);
```

---

### `is_initialized`

```rust
fn is_initialized(env: Env) -> bool
```

Returns `true` if the contract has been initialised. Safe to call before `initialize`.

**Example**
```rust
let ready: bool = client.is_initialized();
```

---

## Admin Management

### `get_admin`

```rust
fn get_admin(env: Env) -> Address
```

Returns the current admin address.

**Errors:** `NotInitialized (26)`

---

### `propose_admin`

```rust
fn propose_admin(env: Env, new_admin: Address)
```

Step 1 of the two-step admin transfer. Called by the current admin.

| Parameter | Type | Description |
|-----------|------|-------------|
| `new_admin` | `Address` | Address being proposed as the new admin |

**Errors:** `UnauthorizedAttestor (4)`

---

### `accept_admin`

```rust
fn accept_admin(env: Env)
```

Step 2 of the admin transfer. Must be called by the proposed admin.

**Errors:** `NoPendingAdmin (53)`, `NotPendingAdmin (54)`

**Example**
```rust
// current admin proposes
client.propose_admin(&new_admin);
// new admin accepts (invoke as new_admin)
client.accept_admin();
```

---

### `get_max_page_size` / `set_max_page_size`

```rust
fn get_max_page_size(env: Env) -> u32
fn set_max_page_size(env: Env, max_page_size: u32)
```

Get or set the global maximum page size for paginated queries (default: 50).

---

## SEP-10 Authentication

### `upsert_sep10_verifying_key`

```rust
fn upsert_sep10_verifying_key(env: Env, issuer: Address, public_key: Bytes)
```

Insert or replace the Ed25519 public key used to verify JWTs issued by `issuer`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `issuer` | `Address` | Issuer identity (Stellar account address) |
| `public_key` | `Bytes` | 32-byte Ed25519 public key |

**Errors:** `UnauthorizedAttestor (4)` if caller is not admin.

---

### `set_sep10_jwt_verifying_key` / `add_sep10_verifying_key`

Aliases for `upsert_sep10_verifying_key`. Kept for backwards compatibility.

---

### `remove_sep10_verifying_key`

```rust
fn remove_sep10_verifying_key(env: Env, issuer: Address, public_key: Bytes)
```

Removes a verifying key for `issuer`. Used during key rotation.

---

### `verify_sep10_token`

```rust
fn verify_sep10_token(env: Env, token: String, issuer: Address)
```

Verifies a SEP-10 JWT against the registered key for `issuer`. Panics on failure.

**Errors:** `InvalidSep10Token (18)`

---

### `verify_sep10_token_for_service`

```rust
fn verify_sep10_token_for_service(env: Env, token: String, issuer: Address, service: u32)
```

Same as `verify_sep10_token` with an additional service-capability check.

| Parameter | Type | Description |
|-----------|------|-------------|
| `service` | `u32` | One of `SERVICE_DEPOSITS`, `SERVICE_WITHDRAWALS`, `SERVICE_QUOTES`, `SERVICE_KYC` |

**Errors:** `InvalidSep10Token (18)`, `ServicesNotConfigured (14)`

---

## Attestor Registration

### `register_attestor`

```rust
fn register_attestor(env: Env, attestor: Address, sep10_token: String, sep10_issuer: Address)
```

Registers a new attestor. Requires a valid SEP-10 JWT.

| Parameter | Type | Description |
|-----------|------|-------------|
| `attestor` | `Address` | The attestor to register |
| `sep10_token` | `String` | Valid SEP-10 JWT for the attestor |
| `sep10_issuer` | `Address` | Issuer whose verifying key validates the token |

**Errors:** `AttestorAlreadyRegistered (2)`, `InvalidSep10Token (18)`, `UnauthorizedAttestor (4)`

**Example**
```rust
client.register_attestor(&anchor, &jwt_token, &issuer);
```

---

### `register_attestor_with_session`

```rust
fn register_attestor_with_session(env: Env, session_id: u64, attestor: Address, sep10_token: String, sep10_issuer: Address)
```

Same as `register_attestor`, and writes an `AuditLog` entry linked to `session_id`.

**Errors:** same as `register_attestor`, plus `SessionNotFound (55)`, `SessionExpired (56)`

---

### `revoke_attestor`

```rust
fn revoke_attestor(env: Env, attestor: Address)
```

Revokes a registered attestor. Admin only.

**Errors:** `AttestorNotRegistered (3)`, `UnauthorizedAttestor (4)`

---

### `revoke_attestor_with_session`

```rust
fn revoke_attestor_with_session(env: Env, session_id: u64, attestor: Address)
```

Same as `revoke_attestor` with audit logging.

---

### `is_attestor`

```rust
fn is_attestor(env: Env, attestor: Address) -> bool
```

Returns `true` if `attestor` is currently registered.

---

### `set_endpoint` / `get_endpoint`

```rust
fn set_endpoint(env: Env, attestor: Address, endpoint: String)
fn get_endpoint(env: Env, attestor: Address) -> String
```

Set or get the HTTPS endpoint URL for an attestor.

**Errors (`set_endpoint`):** `UnauthorizedAttestor (4)`, `InvalidEndpointFormat (12)` if the URL is not HTTPS.

---

## Attestation Submission & Retrieval

### `submit_attestation`

```rust
fn submit_attestation(
    env: Env,
    attestor: Address,
    subject: Address,
    payload_hash: BytesN<32>,
    timestamp: u64,
    nonce: u64,
) -> u64
```

Submits an attestation. Returns the new `attestation_id`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `attestor` | `Address` | Registered attestor submitting the attestation |
| `subject` | `Address` | Entity the attestation is about |
| `payload_hash` | `BytesN<32>` | 32-byte SHA-256 hash of the off-chain payload |
| `timestamp` | `u64` | Unix timestamp of the event (must be within the replay window) |
| `nonce` | `u64` | Unique nonce to prevent replay |

**Errors:** `AttestorNotRegistered (3)`, `InvalidTimestamp (5)`, `ReplayAttack (6)`, `RateLimitExceeded (16)`

**Example**
```rust
let id = client.submit_attestation(&attestor, &subject, &hash, &timestamp, &nonce);
```

---

### `submit_attestation_with_session`

```rust
fn submit_attestation_with_session(
    env: Env,
    session_id: u64,
    attestor: Address,
    subject: Address,
    payload_hash: BytesN<32>,
    timestamp: u64,
    nonce: u64,
) -> u64
```

Same as `submit_attestation` with session-linked audit logging.

---

### `submit_with_request_id`

```rust
fn submit_with_request_id(
    env: Env,
    request_id: RequestId,
    attestor: Address,
    subject: Address,
    payload_hash: BytesN<32>,
    timestamp: u64,
    nonce: u64,
) -> u64
```

Submits an attestation and records a `TracingSpan` keyed by `request_id`.

---

### `get_attestation`

```rust
fn get_attestation(env: Env, id: u64) -> Option<Attestation>
```

Returns the `Attestation` for the given ID, or `None` if not found, has been revoked, or has expired.

---

### `get_attestation_or_panic`

```rust
fn get_attestation_or_panic(env: Env, id: u64) -> Attestation
```

Returns the `Attestation` for the given ID, panicking if it does not exist, has been revoked, or has expired.

---

### `list_attestations`

```rust
fn list_attestations(env: Env, subject: Address, offset: u64, limit: u32) -> Vec<Attestation>
```

Returns paginated attestations for `subject`. `limit` is capped at `max_page_size` (default 50).

---

### `get_attestation_count`

```rust
fn get_attestation_count(env: Env) -> u64
```

Returns the total number of attestations ever submitted.

---

## Service Configuration

### `configure_services`

```rust
fn configure_services(env: Env, anchor: Address, services: Vec<u32>)
```

Sets the services an anchor supports. Replaces any existing configuration.

| Parameter | Type | Description |
|-----------|------|-------------|
| `services` | `Vec<u32>` | List of service constants: `SERVICE_DEPOSITS=1`, `SERVICE_WITHDRAWALS=2`, `SERVICE_QUOTES=3`, `SERVICE_KYC=4` |

**Errors:** `InvalidServiceType (8)`, `UnauthorizedAttestor (4)`

**Example**
```rust
use anchorkit::contract::{SERVICE_DEPOSITS, SERVICE_WITHDRAWALS, SERVICE_KYC};
let mut svcs: Vec<u32> = Vec::new(&env);
svcs.push_back(SERVICE_DEPOSITS);
svcs.push_back(SERVICE_KYC);
client.configure_services(&anchor, &svcs);
```

---

### `get_supported_services`

```rust
fn get_supported_services(env: Env, anchor: Address) -> AnchorServices
```

Returns an `AnchorServices` struct containing the anchor address and its service list.

---

### `supports_service`

```rust
fn supports_service(env: Env, anchor: Address, service: u32) -> bool
```

Returns `true` if `anchor` supports `service`.

---

## Session Management

### `create_session`

```rust
fn create_session(env: Env, initiator: Address) -> u64
```

Opens a new session. Returns the `session_id`.

| Parameter | Type | Description |
|-----------|------|-------------|
| `initiator` | `Address` | Account opening the session |

**Example**
```rust
let session_id: u64 = client.create_session(&user);
```

---

### `get_session`

```rust
fn get_session(env: Env, session_id: u64) -> Session
```

Returns the `Session` struct for the given ID.

**Errors:** `SessionNotFound (55)`

---

### `get_session_operation_count`

```rust
fn get_session_operation_count(env: Env, session_id: u64) -> Option<u64>
```

Returns the number of operations logged in the session, or `None` if the session does not exist.

### Session CLI commands

The `anchorkit session create`, `get`, and `list` commands use Soroban RPC. Set
`SOROBAN_CONTRACT_ID` (or `ANCHORKIT_CONTRACT_ID`) to the deployed contract ID.
`SOROBAN_RPC_URL` (or `ANCHORKIT_RPC_URL`) selects the RPC endpoint and defaults
to Stellar testnet. Creating a session also requires a signing secret in
`STELLAR_SECRET_KEY`, `SOROBAN_SECRET_KEY`, or `ANCHORKIT_SECRET_KEY`; the
initiator must be the account derived from that key. `get` and `list` read
contract storage and do not require a signing key. `list` returns active
sessions, newest first, and accepts limits up to 1,000.

---

## Quotes & Routing

### `submit_quote`

```rust
fn submit_quote(
    env: Env,
    anchor: Address,
    base_asset: String,
    quote_asset: String,
    rate: u64,
    fee: u64,
    min_amount: u64,
    max_amount: u64,
    expires_at: u64,
) -> u64
```

Anchor submits an exchange-rate quote. Returns the `quote_id`.

**Errors:** `AttestorNotRegistered (3)`, `InvalidQuote (7)`

---

### `receive_quote`

```rust
fn receive_quote(env: Env, receiver: Address, anchor: Address, quote_id: u64) -> Quote
```

Receiver retrieves and acknowledges a quote.

**Errors:** `InvalidQuote (7)`, `StaleQuote (10)`

---

### `get_quote`

```rust
fn get_quote(env: Env, anchor: Address, quote_id: u64) -> Option<Quote>
```

Fetches a quote by ID without acknowledging it.

---

### `quote_with_request_id`

```rust
fn quote_with_request_id(
    env: Env,
    request_id: RequestId,
    anchor: Address,
    base_asset: String,
    quote_asset: String,
    rate: u64,
    fee: u64,
    min_amount: u64,
    max_amount: u64,
    expires_at: u64,
) -> u64
```

Same as `submit_quote` with tracing via `request_id`.

---

### `route_transaction`

```rust
fn route_transaction(env: Env, options: RoutingOptions) -> Quote
```

Selects the best anchor according to the routing strategy in `options` and returns its best quote.

| Field in `RoutingOptions` | Description |
|--------------------------|-------------|
| `request` | `RoutingRequest` with base/quote assets, amount, operation type |
| `strategy` | `LowestFee`, `FastestSettlement`, `HighestReputation`, `Balanced`, or `Weighted` |
| `filters` | Optional min-reputation, max-fee, required services |

**Errors:** `NoQuotesAvailable (13)`, `ServicesNotConfigured (14)`

---

### `set_anchor_metadata` / `get_routing_anchors`

```rust
fn set_anchor_metadata(env: Env, anchor: Address, reputation: u32, settlement_time: u32, liquidity: u64, uptime: u32, volume: u64)
fn get_routing_anchors(env: Env) -> Vec<Address>
```

Register routing metadata for an anchor, or list all routing-eligible anchors.

---

## Metadata & Capabilities Cache

### `cache_metadata`

```rust
fn cache_metadata(env: Env, anchor: Address, metadata: AnchorMetadata, ttl_seconds: u64)
```

Stores `AnchorMetadata` for `anchor` with a TTL.

**Errors:** `UnauthorizedAttestor (4)`

---

### `get_cached_metadata`

```rust
fn get_cached_metadata(env: Env, anchor: Address) -> AnchorMetadata
```

Returns cached metadata.

**Errors:** `CacheNotFound (49)`, `CacheExpired (48)`

---

### `get_cache_age_seconds`

```rust
fn get_cache_age_seconds(env: Env, anchor: Address) -> Result<u64, ErrorCode>
```

Returns seconds since the metadata was last cached.

---

### `refresh_metadata_cache`

```rust
fn refresh_metadata_cache(env: Env, anchor: Address) -> AnchorMetadata
```

Invalidates the existing cache entry and returns the last-known metadata.

---

### `list_cached_anchors`

```rust
fn list_cached_anchors(env: Env) -> Vec<Address>
```

Lists all anchors with active (non-expired) metadata cache entries.

---

### `cache_capabilities` / `get_cached_capabilities` / `refresh_capabilities_cache`

```rust
fn cache_capabilities(env: Env, anchor: Address, toml_url: String, capabilities: Vec<u32>, ttl_seconds: u64)
fn get_cached_capabilities(env: Env, anchor: Address) -> CapabilitiesCache
fn refresh_capabilities_cache(env: Env, anchor: Address)
```

Cache and retrieve the capabilities (service flags) parsed from an anchor's stellar.toml.

---

### `invalidate_all_caches`

```rust
fn invalidate_all_caches(env: Env)
```

Admin-only. Clears all metadata and capabilities cache entries.

---

### `get_anchor_health_score`

```rust
fn get_anchor_health_score(env: Env, anchor: Address) -> u32
```

Returns a composite health score in the range `[0, 100]` based on uptime, reputation, and settlement speed from cached metadata.

**Example**
```rust
let score: u32 = client.get_anchor_health_score(&anchor);
if score >= 80 { /* high-quality anchor */ }
```

---

## Anchor Info Discovery

### `fetch_anchor_info`

```rust
fn fetch_anchor_info(env: Env, anchor: Address, toml_data: StellarToml, ttl_override: Option<u64>)
```

Stores a parsed `StellarToml` for `anchor`. Use `ttl_override` to set a custom TTL (defaults to 3600 seconds).

---

### `get_anchor_toml`

```rust
fn get_anchor_toml(env: Env, anchor: Address) -> StellarToml
```

**Errors:** `CacheNotFound (49)`, `CacheExpired (48)`

---

### `refresh_anchor_info`

```rust
fn refresh_anchor_info(env: Env, anchor: Address, force: bool)
```

Invalidates cached TOML. If `force` is `false`, only invalidates if the TTL has elapsed.

---

### `get_anchor_assets`

```rust
fn get_anchor_assets(env: Env, anchor: Address) -> Vec<String>
```

Returns asset codes from the cached stellar.toml.

---

### `get_anchor_currencies`

```rust
fn get_anchor_currencies(env: Env, anchor: Address) -> Vec<FiatCurrency>
```

Returns fiat currencies from the cached stellar.toml.

---

### `get_anchor_asset_info`

```rust
fn get_anchor_asset_info(env: Env, anchor: Address, asset_code: String) -> AssetInfo
```

Returns full `AssetInfo` for one asset including fees, limits, and enabled flags.

---

### `get_anchor_deposit_limits` / `get_anchor_withdrawal_limits`

```rust
fn get_anchor_deposit_limits(env: Env, anchor: Address, asset_code: String) -> (u64, u64)
fn get_anchor_withdrawal_limits(env: Env, anchor: Address, asset_code: String) -> (u64, u64)
```

Returns `(min_amount, max_amount)` for deposit or withdrawal.

---

### `get_anchor_deposit_fees` / `get_anchor_withdrawal_fees`

```rust
fn get_anchor_deposit_fees(env: Env, anchor: Address, asset_code: String) -> (u64, u64)
fn get_anchor_withdrawal_fees(env: Env, anchor: Address, asset_code: String) -> (u64, u64)
```

Returns `(fixed_fee, percent_fee)` for deposit or withdrawal.

---

### `anchor_supports_deposits` / `anchor_supports_withdrawals`

```rust
fn anchor_supports_deposits(env: Env, anchor: Address, asset_code: String) -> bool
fn anchor_supports_withdrawals(env: Env, anchor: Address, asset_code: String) -> bool
```

Returns `true` if the asset is enabled for deposits or withdrawals.

---

## Health Monitoring

### `update_health_status`

```rust
fn update_health_status(
    env: Env,
    anchor: Address,
    latency_ms: u64,
    failure_count: u32,
    availability_percent: u32,
)
```

Records the latest health metrics for `anchor`. Auto-deactivates the anchor if `failure_count` exceeds the threshold.

---

### `get_health_status`

```rust
fn get_health_status(env: Env, anchor: Address) -> Option<HealthStatus>
```

Returns the most recent `HealthStatus`, or `None` if not yet recorded.

---

### `set_health_failure_threshold`

```rust
fn set_health_failure_threshold(env: Env, threshold: u32)
```

Admin-only. Anchors whose `failure_count` reaches or exceeds `threshold` are automatically deactivated.

---

## Request ID & Tracing

### `generate_request_id`

```rust
fn generate_request_id(env: Env) -> RequestId
```

Returns a deterministic 16-byte UUID derived from current ledger state.

---

### `get_tracing_span`

```rust
fn get_tracing_span(env: Env, request_id_bytes: Bytes) -> Option<TracingSpan>
```

Retrieves the `TracingSpan` recorded for a request, or `None` if not found.

---

## Payload Hash Utilities

### `compute_payload_hash` / `compute_payload_hash_public`

```rust
fn compute_payload_hash(env: Env, subject: Address, timestamp: u64, data: Bytes) -> BytesN<32>
fn compute_payload_hash_public(env: Env, subject: Address, timestamp: u64, data: Bytes) -> BytesN<32>
```

Computes a deterministic SHA-256 hash of `(subject || timestamp || data)`. `compute_payload_hash_public` is identical but callable from off-chain clients for pre-image verification.

**Example**
```rust
let hash = client.compute_payload_hash_public(&subject, &timestamp, &payload_bytes);
// hash matches the on-chain compute_payload_hash result for the same inputs
```

---

### `verify_payload_hash`

```rust
fn verify_payload_hash(env: Env, attestation_id: u64, expected_hash: BytesN<32>) -> bool
```

Returns `true` if the stored attestation's payload hash matches `expected_hash`.

---

## Audit Log

### `get_audit_log`

```rust
fn get_audit_log(env: Env, log_id: u64) -> AuditLog
```

Returns a single immutable audit log entry.

**Errors:** `CacheNotFound (49)` if the entry does not exist.

---

### `get_audit_log_range`

```rust
fn get_audit_log_range(env: Env, from_id: u64, to_id: u64) -> Vec<AuditLog>
```

Returns up to 100 `AuditLog` entries in the range `[from_id, to_id)`.

---

### `get_audit_log_offset`

```rust
fn get_audit_log_offset(env: Env) -> u64
```

Returns the ID of the oldest retained audit log entry. Entries below this offset have been pruned.

---

## Error Code Reference

| Code | Name | Meaning |
|------|------|---------|
| 1 | `AlreadyInitialized` | Contract already initialised |
| 2 | `AttestorAlreadyRegistered` | Attestor is already registered |
| 3 | `AttestorNotRegistered` | Attestor is not registered |
| 4 | `UnauthorizedAttestor` | Caller is not authorised |
| 5 | `InvalidTimestamp` | Timestamp outside replay window |
| 6 | `ReplayAttack` | Duplicate nonce detected |
| 7 | `InvalidQuote` | Quote data is invalid |
| 8 | `InvalidServiceType` | Unknown service constant |
| 9 | `InvalidTransactionIntent` | Transaction intent is invalid |
| 10 | `StaleQuote` | Quote has expired |
| 11 | `ComplianceNotMet` | KYC/compliance check failed |
| 12 | `InvalidEndpointFormat` | Endpoint is not a valid HTTPS URL |
| 13 | `NoQuotesAvailable` | No matching quotes for routing |
| 14 | `ServicesNotConfigured` | Anchor has no services configured |
| 15 | `ValidationError` | Schema/input validation failed |
| 16 | `RateLimitExceeded` | Per-attestor rate limit exceeded |
| 17 | `AttestationNotFound` | Attestation ID not found |
| 18 | `InvalidSep10Token` | SEP-10 JWT missing, expired, or invalid |
| 19 | `StorageCorrupted` | Storage entry unreadable |
| 26 | `NotInitialized` | Contract not yet initialised |
| 48 | `CacheExpired` | Cache TTL elapsed |
| 49 | `CacheNotFound` | No cache entry for key |
| 51 | `AuditLogMaxSizeInvalid` | `max_audit_log_size` set to zero |
| 52 | `PendingAdminAlreadyExists` | Admin transfer already in progress |
| 53 | `NoPendingAdmin` | No pending admin transfer |
| 54 | `NotPendingAdmin` | Caller is not the pending admin |
| 55 | `SessionNotFound` | Session ID not found |
| 56 | `SessionExpired` | Session has expired |
| 57 | `MissingSigningKey` | Anchor TOML has no signing key |

For the full `AnchorKitError` type and constructor helpers, see [`src/errors.rs`](../src/errors.rs) and [`docs/features/ERROR_CODES_REFERENCE.md`](features/ERROR_CODES_REFERENCE.md).
