# AnchorKit Observability

AnchorKit's contract emits its operational record through two on-chain mechanisms — **tracing spans** for per-request correlation and an **audit log** for per-operation history — plus standard **Soroban events** for every state change.

There is no host-side logging framework inside the contract. The contract is `#![no_std]`, so it does not write to stdout, does not use the `log` or `tracing` crates, and has no `Logger` type. Everything observable is either written to contract storage and read back through a contract method, or published as an event for an indexer to consume.

> **Historical note:** earlier revisions of this document described a `Logger` / `LoggingConfig` / `configure_logging` API and a `RequestId::generate` associated function. None of those exist. The API below is the one that is actually implemented in `src/contract.rs`.

## What the contract actually provides

| Mechanism | Where it lives | How to read it |
|---|---|---|
| Tracing spans | `TracingSpan` in temporary storage | `get_tracing_span(request_id_bytes)` |
| Audit log | `AuditLog` in persistent storage | `get_audit_log`, `get_audit_log_range`, `get_audit_log_offset` |
| Events | `env.events().publish(...)` | Off-chain indexer / `soroban events` |

### Request correlation with `RequestId`

`RequestId` is a real public type, but it is produced by a **contract method**, not by an associated function:

```rust
#[contracttype]
#[derive(Clone)]
pub struct RequestId {
    pub id: Bytes,       // deterministic 16-byte ID
    pub created_at: u64, // ledger timestamp at creation
}
```

```rust
// Correct: generate via the contract method
let req_id = client.generate_request_id();
```

`RequestId::generate(&env)` does not exist — `RequestId` is a plain `contracttype` struct with no inherent methods.

Request IDs are accepted by two operations, which write a `TracingSpan` as a side effect:

| Method | Span `operation` | Precondition |
|---|---|---|
| `submit_with_request_id` | `submit_attestation` | issuer is a registered attestor |
| `quote_with_request_id` | `submit_quote` | anchor has the Quotes service configured |

```rust
let req_id = client.generate_request_id();

let attestation_id = client.submit_with_request_id(
    &req_id,
    &issuer,
    &subject,
    &env.ledger().timestamp(),
    &payload_hash,
    &signature,
);

let span = client.get_tracing_span(&req_id.id);
```

### TracingSpan

```rust
#[contracttype]
#[derive(Clone)]
pub struct TracingSpan {
    pub request_id: RequestId,
    pub operation: String,  // "submit_attestation" | "submit_quote"
    pub actor: Address,     // attestor or anchor that performed the operation
    pub started_at: u64,
    pub completed_at: u64,
    pub status: String,     // "success" | "failure"
}
```

`get_tracing_span` returns `Option<TracingSpan>` — `None` when the request ID was never used, or when the span's temporary entry has expired.

Spans are stored in **temporary storage** with a TTL of 17,280 ledgers (~24 hours at 5 s/ledger). A failed transaction rolls back the span write, so a span only ever exists for a successful operation.

See [REQUEST_ID_PROPAGATION.md](REQUEST_ID_PROPAGATION.md) for the full propagation guide.

### Audit log

Every audited operation appends an `AuditLog` entry to persistent storage and publishes an `audit/logged` event.

```rust
#[contracttype]
#[derive(Clone)]
pub struct AuditLog {
    pub log_id: u64,
    pub session_id: u64,
    pub actor: Address,
    pub operation: OperationContext,
}

#[contracttype]
#[derive(Clone)]
pub struct OperationContext {
    pub session_id: u64,
    pub operation_index: u64,
    pub operation_type: String,   // "attest" | "register" | "revoke"
    pub timestamp: u64,
    pub status: String,           // "success" | "failure"
    pub result_summary: String,   // e.g. "attestation_id=42"
    pub error_code: Option<u32>,  // populated on failure
    pub attempt_number: u32,      // retry attempts before success
}
```

Read side:

```rust
let entry = client.get_audit_log(&log_id);          // panics if the ID is unknown
let page  = client.get_audit_log_range(&from, &to); // capped at 100 entries per call
let floor = client.get_audit_log_offset();          // first live ID; lower IDs were pruned
```

`get_audit_log_range` returns an empty `Vec` when `from_id > to_id` and silently skips IDs with no stored entry, so a gap in the returned page is not by itself an error. Use `get_audit_log_offset()` to distinguish "pruned" from "never written".

Audit log entries live in **persistent storage** (`PERSISTENT_TTL = 1,555,200` ledgers), not temporary storage.

### Events

AnchorKit publishes events with `symbol_short!` topic pairs. These are the topic pairs present in `src/contract.rs`:

| Topics | Payload type | Emitted when |
|---|---|---|
| `admin/proposed`, `admin/transf` | — | Admin handoff |
| `attestor/reg`, `attestor/revoked` | — | Attestor registry changes |
| `attestor/added`, `attestor/removed` | — | Attestor list membership changes |
| `attest/recorded` | `AttestEvent` | An attestation is stored |
| `attest/revoked` | — | An attestation is revoked |
| `audit/logged` | `AuditLogEvent` | An audit log entry is appended |
| `audit/pruned` | `AuditLogPruned` | Old audit entries are pruned |
| `session/created`, `session/expired` | `SessionCreatedEvent` | Session lifecycle |
| `quote/submit`, `quote/received`, `quote` | `QuoteSubmitEvent`, `QuoteReceivedEvent` | Quote lifecycle |
| `services/config` | `AnchorServices` | Anchor service capability changes |
| `endpoint/updated` | `EndpointUpdated` | Attestor endpoint changes |
| `cache/invall` | — | Metadata cache invalidated |
| `pagesize/updated` | — | Attestation page size changes |
| `anchor/deactiv` | `AnchorDeactivated` | Health-failure threshold reached |
| `routing` | `RoutingDecisionEvent` | Routing strategy picks an anchor |
| `contract/paused`, `contract/unpaused` | `ContractPaused`, `ContractUnpaused` | Pause state changes |
| `migr/compl` | — | Migration completed |

There are no `("log", "entry")`, `("http", "request")`, or `("http", "response")` topics. Event payloads live in `src/events.rs` and in the contract-local structs at the top of `src/contract.rs`.

## Reading the record

```bash
# Stream audit events from a local network
soroban events --start-ledger 1000 --filter "audit"

# All attestation events
soroban events --start-ledger 1000 --filter "attest"

# Route events to jq for indexing
soroban events --start-ledger 1000 --filter "audit" | \
  jq -c 'select(.type == "contract")'
```

The CLI wraps the audit read path:

```bash
# Single entry
anchorkit audit get 42

# All entries for a session
anchorkit audit list --session 7 --format json --pretty

# Export for external ingestion
anchorkit export-audit --format csv --output audit.csv
```

## Sensitive data

Redaction is **not** a contract feature. The contract stores no log payloads, so there is nothing to redact on-chain. Anything resembling payload capture happens in off-chain consumers of these events.

The contract does store payload hashes (`payload_hash: Bytes`) on attestations, never payload bodies. Attestation payloads must never be submitted on-chain — only their hashes.

## Verification

```bash
# Tracing spans
cargo test tracing_span_tests

# Request ID generation and propagation
cargo test request_id_tests

# Audit log
cargo test audit_log_offset_tests

# Event emission
cargo test attestor_event_tests
```

`cargo test logging_tests` matches no tests in this crate — there is no logging test module. The modules above are the real coverage for observability.

## CLI verbosity

`anchorkit test --verbose` passes `--verbose` through to `cargo test`. This is test output verbosity, not contract logging, and there is no global `--debug` or `--verbose` flag on other subcommands.

## Future work

These are not implemented. Treat them as proposals, not features:

- Off-chain structured log sink with configurable redaction and size limits
- Log rotation and archival
- Integration with OpenTelemetry