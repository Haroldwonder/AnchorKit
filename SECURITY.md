# AnchorKit Authorization Model

Every public contract function falls into one of three categories.

## Admin-only

Caller must be the address stored as admin (set during `initialize`).
`require_admin` fetches the admin from instance storage and calls `admin.require_auth()`.

| Function | Notes |
|---|---|
| `initialize` | Sets the admin; admin must sign |
| `set_sep10_jwt_verifying_key` | Registers anchor signing keys |
| `register_attestor` | Adds an attestor; also verifies SEP-10 JWT |
| `revoke_attestor` | Removes an attestor |
| `register_attestor_with_session` | Session-aware variant |
| `revoke_attestor_with_session` | Session-aware variant |
| `cache_metadata` | Writes anchor metadata cache |
| `refresh_metadata_cache` | Clears anchor metadata cache |
| `cache_capabilities` | Writes capabilities cache |
| `refresh_capabilities_cache` | Clears capabilities cache |
| `set_health_failure_threshold` | Sets auto-deactivation threshold |
| `update_health_status` | Writes health metrics; may auto-deactivate anchors |
| `set_anchor_metadata` | Registers anchor in routing table |

## Self-only (registered attestor/anchor must sign)

Caller must be the relevant attestor/anchor address **and** must already be registered.

| Function | Auth check | Registration check |
|---|---|---|
| `set_endpoint` | `attestor.require_auth()` | `check_attestor` |
| `configure_services` | `anchor.require_auth()` | `is_attestor` storage check |
| `submit_attestation` | `issuer.require_auth()` | `check_attestor` |
| `submit_with_request_id` | `issuer.require_auth()` | `check_attestor` |
| `submit_attestation_with_session` | `issuer.require_auth()` | `check_attestor` |
| `submit_quote` | `anchor.require_auth()` | `check_attestor` |
| `fetch_anchor_info` | `anchor.require_auth()` | — (TOML cache, not state-critical) |
| `refresh_anchor_info` | `anchor.require_auth()` | — (clears own cache entry) |
| `create_session` | `initiator.require_auth()` | — |
| `receive_quote` | `receiver.require_auth()` | — |

## Public (read-only, no auth required)

These functions only read state and never panic on missing auth.

`is_initialized`, `get_admin`, `get_attestation`, `list_attestations`,
`is_attestor`, `get_endpoint`, `get_supported_services`, `supports_service`,
`get_tracing_span`, `compute_payload_hash`, `verify_payload_hash`,
`get_session`, `get_audit_log`, `get_session_operation_count`,
`get_cached_metadata`, `get_cached_capabilities`, `get_health_status`,
`get_quote`, `get_routing_anchors`, `route_transaction`,
`get_anchor_toml`, `get_anchor_assets`, `get_anchor_asset_info`,
`get_anchor_deposit_limits`, `get_anchor_withdrawal_limits`,
`get_anchor_deposit_fees`, `get_anchor_withdrawal_fees`,
`anchor_supports_deposits`, `anchor_supports_withdrawals`,
`generate_request_id`, `verify_sep10_token`

## Replay Protection

- Attestation `payload_hash` values are stored in persistent storage after first use; re-submission panics with `ReplayAttack`.
- Sessions use a nonce (`session.nonce`) to prevent operation replay within a session.

## Error Codes

Authorization failures surface as stable error codes (see `src/errors.rs`):
- `NotInitialized` (101) — contract not yet initialized
- `Unauthorized` (102) — caller is not the admin
- `AttestorNotRegistered` (104) — attestor address not in registry
