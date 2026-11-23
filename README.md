# AnchorKit

AnchorKit is a Soroban-native toolkit for anchoring off-chain attestations to Stellar. It enables smart contracts to verify real-world events such as KYC approvals, payment confirmations, and signed claims in a trust-minimized way.

## Key Features

- **Attestation Management**: Register attestors, submit and retrieve attestations
- **Endpoint Configuration**: Manage attestor endpoints for off-chain integration
- **Session Management**: Group operations into logical sessions for traceability
- **Audit Trail**: Complete immutable record of all operations
- **Reproducibility**: Deterministic operation replay for verification
- **Replay Protection**: Multi-level protection against unauthorized replays
- **Mock Anchor**: Built-in simulator for testing without external APIs

## New: Session Traceability & Reproducibility

AnchorKit now includes comprehensive session management and operation tracing to ensure all anchor interactions are **reproducible** and **traceable**.

### What This Means

- **Every operation is logged** with complete context (who, what, when, result)
- **Sessions group related operations** for logical organization
- **Audit trail is immutable** for compliance and verification
- **Operations can be replayed** deterministically for reproducibility
- **Replay attacks are prevented** through nonce-based protection

### Quick Example

```rust
// Create a session
let session_id = contract.create_session(&user_address);

// Perform operations within the session
let attestation_id = contract.submit_attestation_with_session(
    &session_id,
    &issuer,
    &subject,
    &timestamp,
    &payload_hash,
    &signature
);

// Verify session completeness
let operation_count = contract.get_session_operation_count(&session_id);

// Retrieve audit logs
let audit_log = contract.get_audit_log(&0);
```

## Testing with Mock Anchor

```rust
use anchorkit::mock_anchor::MockAnchor;

// Create mock attestation data
let payload = Bytes::from_slice(&env, b"KYC approved");
let payload_hash = MockAnchor::hash_payload(&env, &payload);
let signature = MockAnchor::sign(&env, &issuer, &subject, timestamp, &payload_hash);

// Submit to contract
let id = contract.submit_attestation(&issuer, &subject, &timestamp, &payload_hash, &signature);
```

See [MOCK_ANCHOR.md](./MOCK_ANCHOR.md) for complete testing guide.

## Documentation

### Getting Started
- **[QUICK_START.md](./QUICK_START.md)** - Quick reference guide with examples
- **[MOCK_ANCHOR.md](./MOCK_ANCHOR.md)** - Mock anchor for testing without external APIs

### Feature Documentation
- **[SESSION_TRACEABILITY.md](./SESSION_TRACEABILITY.md)** - Complete feature guide with usage patterns
- **[API_SPEC.md](./API_SPEC.md)** - API specification and error codes

### Technical Documentation
- **[IMPLEMENTATION_GUIDE.md](./IMPLEMENTATION_GUIDE.md)** - Technical implementation details
- **[IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md)** - Implementation overview
- **[VERIFICATION_CHECKLIST.md](./VERIFICATION_CHECKLIST.md)** - Verification and quality assurance

## New API Methods

### Session Management
- `create_session(initiator)` - Create new session
- `get_session(session_id)` - Get session details
- `get_session_operation_count(session_id)` - Get operation count
- `get_audit_log(log_id)` - Get audit log entry

### Session-Aware Operations
- `submit_attestation_with_session(...)` - Submit attestation with logging
- `register_attestor_with_session(...)` - Register attestor with logging
- `revoke_attestor_with_session(...)` - Revoke attestor with logging

## New Data Structures

- `InteractionSession` - Represents a session with metadata
- `OperationContext` - Captures operation details
- `AuditLog` - Complete audit entry

## New Events

- `SessionCreated` - Emitted when session is created
- `OperationLogged` - Emitted when operation is logged

## Building

```bash
cargo build --release
```

## Release provenance

Published releases include attestations for the Soroban contract WASM and any
CLI binaries. The UI npm package is published with npm provenance when
`ui/package.json` is present. See [PROVENANCE.md](PROVENANCE.md) for verification
commands.

## Testing

The contract includes comprehensive tests for all functionality:

```bash
cargo test
```

## Backward Compatibility

All existing methods remain unchanged. Session features are opt-in, allowing gradual adoption.

## Use Cases

### Compliance & Audit
- Complete audit trail for regulatory compliance
- Immutable operation records
- Actor tracking for accountability

### Reproducibility
- Deterministic operation replay
- Session-based operation grouping
- Complete context preservation

### Security
- Replay attack prevention
- Multi-level protection
- Nonce-based verification

## Architecture

AnchorKit consists of:

- **Core Contract** (`src/lib.rs`) - Main contract logic
- **Storage Layer** (`src/storage.rs`) - Persistent data management
- **Event System** (`src/events.rs`) - Event definitions and publishing
- **Type System** (`src/types.rs`) - Data structures
- **Error Handling** (`src/errors.rs`) - Error codes and definitions

## Security

- Stable error codes (100-120) for API compatibility
- Replay protection at multiple levels
- Immutable audit logs
- Authorization checks on all operations
- Complete operation context for verification

## Performance

- Efficient storage with TTL management
- Minimal event data
- Sequential IDs (no hash lookups)
- Optimized for Soroban constraints

## License

[Add your license here]

## Support

For questions or issues:
1. Check the documentation files
2. Review the API specification
3. Examine the test cases in `src/lib.rs`
