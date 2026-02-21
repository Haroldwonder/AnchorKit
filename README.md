# AnchorKit

AnchorKit is a Soroban-native toolkit for anchoring off-chain attestations to Stellar. It enables smart contracts to verify real-world events such as KYC approvals, payment confirmations, and signed claims in a trust-minimized way.


## Features

- Attestation management with replay attack protection
- Attestor registration and revocation
- Endpoint configuration for attestors
- Service capability discovery (deposits, withdrawals, quotes, KYC)
- **Health monitoring** (latency, failures, availability)
- Event emission for all state changes
- Comprehensive error handling with stable error codes

## Supported Services

Anchors can configure which services they support:

- **Deposits**: Accept incoming deposits from users
- **Withdrawals**: Process withdrawal requests
- **Quotes**: Provide exchange rate quotes
- **KYC**: Perform Know Your Customer verification

## Usage Example

```rust
// Initialize the contract
contract.initialize(&admin);

// Register an attestor/anchor
contract.register_attestor(&anchor);

// Configure supported services for the anchor
let mut services = Vec::new(&env);
services.push_back(ServiceType::Deposits);
services.push_back(ServiceType::Withdrawals);
services.push_back(ServiceType::KYC);
contract.configure_services(&anchor, &services);

// Query supported services
let supported = contract.get_supported_services(&anchor);

// Check if a specific service is supported
if contract.supports_service(&anchor, &ServiceType::Deposits) {
    // Process deposit
}
```

## Key Features

- **Attestation Management**: Register attestors, submit and retrieve attestations
- **Endpoint Configuration**: Manage attestor endpoints for off-chain integration
- **Session Management**: Group operations into logical sessions for traceability
- **Audit Trail**: Complete immutable record of all operations
- **Reproducibility**: Deterministic operation replay for verification
- **Replay Protection**: Multi-level protection against unauthorized replays
- **Secure Credential Management**: Runtime credential injection with automatic rotation

## New: Session Traceability & Reproducibility

AnchorKit now includes comprehensive session management and operation tracing to ensure all anchor interactions are **reproducible** and **traceable**.

### What This Means

- **Every operation is logged** with complete context (who, what, when, result)
- **Sessions group related operations** for logical organization
- **Audit trail is immutable** for compliance and verification
- **Operations can be replayed** deterministically for reproducibility
- **Replay attacks are prevented** through nonce-based protection

### Quick Example

```javascript
// Create a session
const sessionId = await contract.create_session(userAddress);

// Perform operations within the session
const attestationId = await contract.submit_attestation_with_session(
    sessionId,
    issuer,
    subject,
    timestamp,
    payloadHash,
    signature
);

// Verify session completeness
const operationCount = await contract.get_session_operation_count(sessionId);

// Retrieve audit logs
const auditLog = await contract.get_audit_log(0);
```

## Documentation

### Getting Started
- **[QUICK_START.md](./QUICK_START.md)** - Quick reference guide with examples

### Feature Documentation
- **[SESSION_TRACEABILITY.md](./SESSION_TRACEABILITY.md)** - Complete feature guide with usage patterns
- **[SECURE_CREDENTIALS.md](./SECURE_CREDENTIALS.md)** - Secure credential injection and management
- **[HEALTH_MONITORING.md](./HEALTH_MONITORING.md)** - Anchor health monitoring interface
- **[API_SPEC.md](./API_SPEC.md)** - API specification and error codes

### Technical Documentation
- **[IMPLEMENTATION_GUIDE.md](./IMPLEMENTATION_GUIDE.md)** - Technical implementation details
- **[IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md)** - Implementation overview
- **[DEPLOYMENT_WITH_CREDENTIALS.md](./DEPLOYMENT_WITH_CREDENTIALS.md)** - Deployment guide with secure credentials
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

