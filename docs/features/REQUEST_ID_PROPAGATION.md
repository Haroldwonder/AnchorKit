# Request ID Propagation

Request IDs allow callers to correlate on-chain operations with off-chain
systems (logs, tracing, support tickets). Every state-changing entry point that
supports request IDs accepts a `RequestId` as its first argument and emits it
in the corresponding event.

## `submit_with_request_id`

The real signature is:

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

Usage:

```rust
let attestation_id = contract.submit_with_request_id(
    &req_id, &attestor, &subject, &payload_hash, &timestamp, &nonce,
);
```

Note the parameter order: `payload_hash` comes **before** `timestamp`, and the
final argument is a `nonce: u64` (not a signature).

## `quote_with_request_id`

The real signature is:

```rust
fn quote_with_request_id(
    env: Env,
    request_id: RequestId,
    anchor: Address,
    base_asset: Address,
    quote_asset: Address,
    rate: u64,
    fee: u64,
    min_amount: u64,
    max_amount: u64,
    expires_at: u64,
) -> u64
```

Usage:

```rust
let quote_id = contract.quote_with_request_id(
    &req_id, &anchor, &base_asset, &quote_asset, &rate, &fee,
    &min_amount, &max_amount, &expires_at,
);
```

All nine arguments are required: `request_id`, `anchor`, `base_asset`,
`quote_asset`, `rate`, `fee`, `min_amount`, `max_amount`, and `expires_at`.
