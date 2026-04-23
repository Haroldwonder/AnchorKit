# SEP-10 Authentication Module

> **Location:** This file is the canonical SEP-10 doc (`docs/features/SEP10_AUTH.md`). A short pointer also exists at the repo root as [`SEP10_AUTH.md`](../../SEP10_AUTH.md) for older links.

## Overview

Minimal SEP-10 (Stellar Web Authentication) support in AnchorKit, aligned with the [SEP-10 specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md).

Off-chain flows (challenge, wallet signing, obtaining a JWT from the anchor) are documented for integrators and reflected in the UI (`Sep10AuthFlow.tsx`). **On-chain**, the Soroban contract enforces SEP-10 **JWT** verification before sensitive operations such as attestor registration.

## On-chain implementation (`src/sep10_jwt.rs` + `AnchorKitContract`)

The contract verifies **JWS compact** tokens whose header uses **EdDSA** (Ed25519), matching common SEP-10 JWT issuance:

1. **Anchor public key** — The admin stores the anchor’s JWT verification key (32-byte Ed25519 public key) per logical issuer:
   - `set_sep10_jwt_verifying_key(env, issuer, public_key)`  
   - Persistent key: `("SEP10KEY", issuer) -> Bytes` (32 bytes).

2. **JWT verification** — `verify_sep10_token(env, token, issuer)`:
   - Loads the stored public key for `issuer`.
   - Parses the JWT (`header.payload.signature`), base64url-decodes segments.
   - Requires header JSON to contain `EdDSA`.
   - Verifies Ed25519 over the ASCII signing input `header_b64 + "." + payload_b64` via `env.crypto().ed25519_verify`.
   - Decodes the payload JSON and requires:
     - `exp` (Unix seconds) **strictly greater** than `env.ledger().timestamp() - clock_skew_seconds` (see [Clock skew](#clock-skew) below).
     - A parseable string `sub` claim (Stellar strkey of the authenticated account).

3. **Attestor registration** — `register_attestor(env, attestor, sep10_token, sep10_issuer)`:
   - Requires admin auth (unchanged).
   - Runs the same checks as `verify_sep10_token`, and additionally requires `sub` to equal **`attestor.to_string()`** (the strkey of the address being registered).

### Clock skew

The Soroban ledger timestamp (`env.ledger().timestamp()`) reflects the time the last ledger was closed, which can lag wall-clock time by several seconds to a minute under normal network conditions. If a JWT is issued with a short TTL (e.g. 60 s) and the ledger lags by 30 s, the token may appear expired on-chain even though it is still valid by wall-clock time.

`verify_sep10_jwt` accepts a `clock_skew_seconds: u64` parameter to accommodate this:

```
token accepted  iff  exp + clock_skew_seconds > ledger_timestamp
```

The contract entry points (`verify_sep10_token`, `register_attestor`) currently pass `clock_skew_seconds = 0` for strict enforcement. Operators who observe spurious expiry rejections should increase this value — **60 seconds** is a reasonable default for Stellar Testnet/Mainnet. Values above 300 s are not recommended as they meaningfully extend the window in which a revoked token remains accepted.

Token length is capped (`MAX_JWT_LEN`, 2048 characters) to bound host work.

### Error handling

Failures (missing key, bad format, wrong signature, expired `exp`, `sub` mismatch on registration) panic with **`ErrorCode::InvalidSep10Token`** (see `src/errors.rs`).

### Contract API (summary)

```rust
// Admin: store the anchor JWT signing public key (Ed25519, 32 bytes) for `issuer`.
pub fn set_sep10_jwt_verifying_key(env: Env, issuer: Address, public_key: Bytes);

// Verify signature + exp + parseable sub (sub not compared to an address).
pub fn verify_sep10_token(env: Env, token: String, issuer: Address);

// Register attestor; requires valid SEP-10 JWT whose sub matches `attestor`.
pub fn register_attestor(env: Env, attestor: Address, sep10_token: String, sep10_issuer: Address);
```

## Off-chain / SDK-style types (reference)

Higher-level types such as `Sep10Challenge` / `Sep10Session` may appear in docs or SDK layers; the **on-chain** enforcement for AnchorKit is the JWT path above.

## Testing

```bash
cargo test --lib sep10
```

Coverage includes:

- Valid token (Ed25519 JWS, future `exp`).
- Expired token (`exp` ≤ ledger timestamp, `clock_skew_seconds = 0`).
- Token within clock skew window (expired by wall clock but accepted with `clock_skew_seconds = 60`).
- Invalid signature (wrong verifying key).
- Unauthorized `set_sep10_jwt_verifying_key` call (non-admin) panics.
- Contract integration: `verify_sep10_token`, `register_attestor` with `set_sep10_jwt_verifying_key`.

## Compliance notes

- Challenge-response and JWT issuance follow SEP-10; **on-chain** validation uses the JWT’s **EdDSA** JWS profile and `exp` / `sub` claims.
- Anchors must publish signing material consistent with their SEP-10 endpoint; operators store the corresponding **verification** public key on-chain via `set_sep10_jwt_verifying_key`.

## Security

- Ed25519 verification uses the Soroban host’s `ed25519_verify`.
- Registration binds the JWT’s `sub` to the `attestor` address being added.
- Admin-only setup of verification keys; registration remains admin-gated.
- `set_sep10_jwt_verifying_key` is admin-gated: it calls `admin.require_auth()` and panics if the caller is not the stored admin. An attacker who could overwrite the verifying key would be able to forge valid SEP-10 tokens for any issuer, so this guard is critical.
