# Request ID Propagation

AnchorKit supports UUID-per-flow request ID tracking for end-to-end tracing of contract operations.

## Overview

Every session-aware contract operation accepts a `RequestId` and stores a `TracingSpan` in temporary storage. This enables correlating all operations belonging to a single logical flow.

## RequestId Structure

```rust
pub struct RequestId {
    pub id: Bytes,       // 16-byte deterministic ID
    pub created_at: u64, // ledger timestamp at creation
}
```

The `id` is derived as `sha256(timestamp_u64_be || sequence_number_u32_be)[:16]`, making it deterministic and unique per ledger sequence.

## Usage

### 1. Generate a Request ID

```rust
let req_id = contract.generate_request_id(&env);
```

### 2. Submit an Attestation with Request ID

```rust
let attestation_id = contract.submit_with_request_id(
    &req_id,
    &attestor,
    &subject,
    &payload_hash,
    &timestamp,
    &nonce,
);
```

### 3. Submit a Quote with Request ID

```rust
contract.quote_with_request_id(
    &req_id,
    &anchor,
    &base_asset,
    &quote_asset,
    &rate,
    &fee,
    &min_amount,
    &max_amount,
    &expires_at,
);
```

### 4. Retrieve a Tracing Span

```rust
let span = contract.get_tracing_span(&req_id.id);
```

## TracingSpan Structure

```rust
pub struct TracingSpan {
    pub request_id: RequestId,  // the originating request ID
    pub operation: String,      // e.g. "submit_attestation", "submit_quote"
    pub actor: Address,         // who performed the operation
    pub started_at: u64,        // ledger timestamp when operation started
    pub completed_at: u64,      // ledger timestamp when operation completed
    pub status: String,         // "success" or "failure"
}
```

## Storage

Tracing spans are stored in **temporary storage** under the key `["SPAN", request_id_bytes]` with a TTL of 17,280 ledgers (~1 day at 5s/ledger).

On transaction failure, the span is NOT stored (Soroban rolls back all storage writes).

## Supported Operations

| Function | Operation Name | Notes |
|---|---|---|
| `submit_with_request_id` | `submit_attestation` | Requires registered attestor |
| `quote_with_request_id` | `submit_quote` | Requires Quotes service configured |

## Error Codes

| Code | Name | Description |
|---|---|---|
| 1 | `AlreadyInitialized` | Contract already initialized |
| 3 | `AttestorNotRegistered` | Attestor not registered |
| 6 | `ReplayAttack` | Duplicate payload hash |
| 14 | `ServicesNotConfigured` | Anchor has no services configured |
